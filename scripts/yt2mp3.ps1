# Convert YouTube URL to MP3 via yt-dlp.
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Url,

    [Parameter(Position = 1)]
    [string]$OutDir = "."
)

$ErrorActionPreference = "Stop"

if (-not (Get-Command yt-dlp -ErrorAction SilentlyContinue)) {
    Write-Error "yt-dlp not found. Install: pip install yt-dlp"
    exit 1
}
if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Error "ffmpeg not found in PATH."
    exit 1
}

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

yt-dlp -x --audio-format mp3 --audio-quality 0 `
    -o "$OutDir/%(title)s.%(ext)s" `
    $Url
