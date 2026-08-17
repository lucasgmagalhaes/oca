# Watches a folder and automatically processes new gameplay videos: clearer voice, less noise, no
# clipping on loud moments. Serves a live pipeline UI at http://localhost:<Port>/ instead of a
# console-only view.
param(
    [string]$WatchPath = "E:\records\teste",
    [string]$OutputFolderName = "processed",
    [int]$StableSeconds = 6,
    [int]$PollIntervalSeconds = 2,
    [int]$Port = 8787
)

$ErrorActionPreference = "Stop"

$videoExtensions = @(".mp4", ".mkv", ".mov", ".avi", ".flv", ".ts", ".m4v")

# Audio filter chain:
# highpass    -> cuts unwanted low-end rumble from the mic
# afftdn      -> reduces background noise (fan, keyboard, hiss)
# acompressor -> evens out the voice level, already softening scream peaks
# loudnorm    -> normalizes overall loudness so videos sound consistent
# alimiter    -> final safety ceiling, guarantees nothing clips
$audioFilter = "highpass=f=80,afftdn=nf=-30,acompressor=threshold=-18dB:ratio=3:attack=10:release=250:makeup=1.5,loudnorm=I=-16:TP=-1.5:LRA=11,alimiter=limit=0.95:attack=5:release=50"
$measureFilter = "loudnorm=I=-16:TP=-1.5:LRA=11:print_format=json"

if (-not (Test-Path $WatchPath)) {
    Write-Host "Folder not found: $WatchPath" -ForegroundColor Red
    exit 1
}

foreach ($tool in @("ffmpeg", "ffprobe")) {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        Write-Host "$tool not found in PATH." -ForegroundColor Red
        exit 1
    }
}

$uiHtmlPath = Join-Path $PSScriptRoot "ui.html"
if (-not (Test-Path $uiHtmlPath)) {
    Write-Host "ui.html not found next to the script ($uiHtmlPath)." -ForegroundColor Red
    exit 1
}
$uiHtml = Get-Content -Path $uiHtmlPath -Raw -Encoding UTF8

$outputPath = Join-Path $WatchPath $OutputFolderName
if (-not (Test-Path $outputPath)) {
    New-Item -ItemType Directory -Path $outputPath | Out-Null
}

function Format-Arg {
    param([string]$Value)
    if ($Value -match '[\s"]') {
        return '"' + ($Value -replace '"', '\"') + '"'
    }
    return $Value
}

function Get-VideoDuration {
    param([string]$Path)
    $out = & ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 $Path
    $seconds = 0.0
    if ([double]::TryParse($out, [ref]$seconds)) {
        return $seconds
    }
    return 0.0
}

function Get-AudioAnalysis {
    param([string]$Path)

    $ffmpegArgs = @(
        "-y", "-nostdin", "-hide_banner", "-loglevel", "info",
        "-i", $Path,
        "-af", $measureFilter,
        "-f", "null", "-"
    )

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "ffmpeg"
    $psi.Arguments = ($ffmpegArgs | ForEach-Object { Format-Arg $_ }) -join " "
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $p = New-Object System.Diagnostics.Process
    $p.StartInfo = $psi
    $p.Start() | Out-Null
    $p.StandardInput.Close()
    $stderr = $p.StandardError.ReadToEnd()
    $p.WaitForExit()

    if ($stderr -match '(?s)(\{[^{}]*\})') {
        try {
            $json = $matches[1] | ConvertFrom-Json
            return @{
                Lufs     = [double]$json.input_i
                TruePeak = [double]$json.input_tp
                Lra      = [double]$json.input_lra
            }
        } catch {
            return $null
        }
    }
    return $null
}

function Invoke-FfmpegWithProgress {
    param(
        [string]$InputPath,
        [string]$OutputPath,
        [double]$DurationSeconds,
        [hashtable]$Record
    )

    $ffmpegArgs = @(
        "-y", "-nostdin", "-hide_banner", "-loglevel", "warning",
        "-i", $InputPath,
        "-af", $audioFilter,
        "-c:v", "copy",
        "-c:a", "aac", "-b:a", "192k",
        "-progress", "pipe:1",
        $OutputPath
    )

    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "ffmpeg"
    $psi.Arguments = ($ffmpegArgs | ForEach-Object { Format-Arg $_ }) -join " "
    # StandardInput is redirected (and never written to) as an extra guard against ffmpeg
    # falling back to its interactive stdin command prompt, on top of -nostdin.
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true

    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $psi
    $process.Start() | Out-Null
    $process.StandardInput.Close()

    while (-not $process.StandardOutput.EndOfStream) {
        $line = $process.StandardOutput.ReadLine()

        if ($line -match '^out_time_us=(\d+)') {
            $currentSeconds = [double]$matches[1] / 1000000
            if ($DurationSeconds -gt 0) {
                $percent = [math]::Min(100, [math]::Round(($currentSeconds / $DurationSeconds) * 100))
                $Record.Percent = $percent
            }
        }
    }

    $process.WaitForExit()
    return $process.ExitCode
}

# ---------------------------------------------------------------------------
# Shared state between the file-watching loop (this thread) and the HTTP
# server (background runspace). $Files holds one record per tracked file;
# $FileOrder preserves detection order so the UI can list newest-first.
# ---------------------------------------------------------------------------
$Files = [hashtable]::Synchronized(@{})
$FileOrder = [System.Collections.ArrayList]::Synchronized((New-Object System.Collections.ArrayList))

$httpServerScript = {
    $listener = New-Object System.Net.HttpListener
    $listener.Prefixes.Add("http://localhost:$Port/")
    try {
        $listener.Start()
    } catch {
        return
    }

    while ($listener.IsListening) {
        try {
            $ctx = $listener.GetContext()
        } catch {
            break
        }

        $req = $ctx.Request
        $res = $ctx.Response

        try {
            if ($req.Url.AbsolutePath -eq "/api/status") {
                $order = @($FileOrder.ToArray())
                $list = New-Object System.Collections.ArrayList
                foreach ($p in $order) {
                    if ($Files.ContainsKey($p)) { [void]$list.Add($Files[$p]) }
                }
                $list.Reverse()
                $payload = @{ WatchPath = $WatchPath; OutputPath = $OutputPath; Files = $list }
                $json = $payload | ConvertTo-Json -Depth 6 -Compress
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($json)
                $res.ContentType = "application/json; charset=utf-8"
                $res.Headers.Add("Cache-Control", "no-store")
                $res.OutputStream.Write($bytes, 0, $bytes.Length)
            } else {
                $bytes = [System.Text.Encoding]::UTF8.GetBytes($UiHtml)
                $res.ContentType = "text/html; charset=utf-8"
                $res.OutputStream.Write($bytes, 0, $bytes.Length)
            }
        } catch {
            $res.StatusCode = 500
        } finally {
            $res.OutputStream.Close()
        }
    }
}

$runspace = [runspacefactory]::CreateRunspace()
$runspace.Open()
$runspace.SessionStateProxy.SetVariable("Files", $Files)
$runspace.SessionStateProxy.SetVariable("FileOrder", $FileOrder)
$runspace.SessionStateProxy.SetVariable("WatchPath", $WatchPath)
$runspace.SessionStateProxy.SetVariable("OutputPath", $outputPath)
$runspace.SessionStateProxy.SetVariable("UiHtml", $uiHtml)
$runspace.SessionStateProxy.SetVariable("Port", $Port)

$httpPS = [powershell]::Create()
$httpPS.Runspace = $runspace
[void]$httpPS.AddScript($httpServerScript)
$httpHandle = $httpPS.BeginInvoke()

$uiUrl = "http://localhost:$Port/"
Write-Host "UI running at $uiUrl" -ForegroundColor Cyan
Write-Host "Watching '$WatchPath' -> '$outputPath' (Ctrl+C to stop)" -ForegroundColor Cyan
Start-Process $uiUrl

function Test-FileReady {
    # A stable file size isn't proof a recorder is done with the file: writers can plateau
    # mid-write (buffering, GOP boundaries) and still hold the handle, and a finished MP4 often
    # only writes its moov atom at close time. Requiring an exclusive lock catches that case.
    param([string]$Path)
    try {
        $stream = [System.IO.File]::Open($Path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::None)
        $stream.Close()
        return $true
    } catch {
        return $false
    }
}

$tracked = @{}

while ($true) {
    $candidates = Get-ChildItem -Path $WatchPath -File | Where-Object { $videoExtensions -contains $_.Extension.ToLower() }

    foreach ($file in $candidates) {
        $outFile = Join-Path $outputPath $file.Name

        if (Test-Path $outFile) {
            continue
        }

        $size = $file.Length

        if (-not $tracked.ContainsKey($file.FullName)) {
            $tracked[$file.FullName] = @{ Size = $size; StableSince = Get-Date; Errored = $false }
        } else {
            $info = $tracked[$file.FullName]
            if ($size -ne $info.Size) {
                $info.Size = $size
                $info.StableSince = Get-Date
                $info.Errored = $false
            }
        }

        if (-not $Files.ContainsKey($file.FullName)) {
            $Files[$file.FullName] = @{
                Path    = $file.FullName
                Name    = $file.Name
                Status  = "detected"
                Percent = 0
                Error   = $null
                Metrics = @{ before = $null; after = $null }
            }
            [void]$FileOrder.Add($file.FullName)
        }

        $info = $tracked[$file.FullName]
        $record = $Files[$file.FullName]

        # A file that already failed is left alone until it changes again (someone re-saved it,
        # or it's still actively being written) — otherwise it would retry every poll forever.
        if ($info.Errored) {
            continue
        }

        $elapsed = (Get-Date) - $info.StableSince
        if ($elapsed.TotalSeconds -lt $StableSeconds -or -not (Test-FileReady -Path $file.FullName)) {
            $record.Status = "stabilizing"
            continue
        }

        try {
            $record.Status = "analyzing-in"
            $before = Get-AudioAnalysis -Path $file.FullName
            $record.Metrics.before = $before

            $record.Status = "processing"
            $record.Percent = 0
            $duration = Get-VideoDuration -Path $file.FullName
            $exitCode = Invoke-FfmpegWithProgress -InputPath $file.FullName -OutputPath $outFile -DurationSeconds $duration -Record $record

            if ($exitCode -ne 0) {
                throw "ffmpeg exited with code $exitCode"
            }

            $record.Status = "analyzing-out"
            $after = Get-AudioAnalysis -Path $outFile
            $record.Metrics.after = $after

            $record.Status = "done"
            $record.Percent = 100
            $tracked.Remove($file.FullName)
            Write-Host "[done] $($file.Name)" -ForegroundColor Green
        } catch {
            $record.Status = "error"
            $record.Error = $_.Exception.Message
            $info.Errored = $true
            Write-Host "[error] $($file.Name): $($_.Exception.Message)" -ForegroundColor Red
        }
    }

    Start-Sleep -Seconds $PollIntervalSeconds
}
