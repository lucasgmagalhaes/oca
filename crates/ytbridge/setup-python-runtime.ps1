# One-time setup: fetches a self-contained Python runtime (python-build-standalone) and
# installs yt-dlp, its EJS scripts and Deno, so `cargo build -p ytbridge` ships every runtime
# needed for YouTube extraction. Run once after cloning, and again
# whenever bumping PYTHON_VERSION/RELEASE_TAG below. See CLAUDE.md's Fase 8 "Python runtime for
# ytbridge" entry for the full rationale.
#
# Windows twin of setup-python-runtime.sh; both produce the platform-specific layout consumed
# by ytbridge/build.rs.

$ErrorActionPreference = "Stop"

$PythonVersion = "3.10.21"
$ReleaseTag = "20260814"
$AssetName = "cpython-$PythonVersion+$ReleaseTag-x86_64-pc-windows-msvc-install_only.tar.gz"
$Url = "https://github.com/astral-sh/python-build-standalone/releases/download/$ReleaseTag/$AssetName"
$PythonSha256 = "9e2c77a2f2bfba3148b6e6c74679df571aa436d5b50113b01213fcd4cb803b70"
$YtDlpVersion = "2026.07.04"
$YtDlpAsset = "yt-dlp.tar.gz"
$YtDlpUrl = "https://github.com/yt-dlp/yt-dlp/releases/download/$YtDlpVersion/$YtDlpAsset"
$YtDlpSha256 = "31c32457d1a573a341bb0929386c624fe47339a5338829e6e9c9454bdfa7397a"
$EjsVersion = "0.8.0"
$EjsAsset = "yt_dlp_ejs-$EjsVersion-py3-none-any.whl"
$EjsUrl = "https://github.com/yt-dlp/ejs/releases/download/$EjsVersion/$EjsAsset"
$EjsSha256 = "79300e5fca7f937a1eeede11f0456862c1b41107ce1d726871e0207424f4bdb4"
$DenoVersion = "2.9.5"
$DenoAsset = "deno-x86_64-pc-windows-msvc.zip"
$DenoUrl = "https://github.com/denoland/deno/releases/download/v$DenoVersion/$DenoAsset"
$DenoSha256 = "171efab55ac6b9881fd53ee4c20f8bf3bb1340ffc618483746909014db12216a"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$VendorDir = Join-Path $RepoRoot "vendor"
$RuntimeDir = Join-Path $VendorDir "python-runtime"
$ArchivePath = Join-Path $VendorDir $AssetName
$YtDlpPath = Join-Path $VendorDir $YtDlpAsset
$EjsPath = Join-Path $VendorDir $EjsAsset
$DenoPath = Join-Path $VendorDir $DenoAsset
$MarkerPath = Join-Path $RuntimeDir ".oca-runtime-versions"
$ExpectedMarker = @"
python=$PythonVersion+$ReleaseTag
yt-dlp=$YtDlpVersion
yt-dlp-ejs=$EjsVersion
deno=$DenoVersion
"@.Trim()

if ((Test-Path $MarkerPath) -and ((Get-Content $MarkerPath -Raw).Trim() -eq $ExpectedMarker)) {
    Write-Host "vendor/python-runtime already contains every pinned dependency." -ForegroundColor Green
    exit 0
}

New-Item -ItemType Directory -Force -Path $VendorDir | Out-Null

if (-not (Test-Path $RuntimeDir)) {
    Write-Host "Downloading $AssetName..." -ForegroundColor Cyan
    Invoke-WebRequest -Uri $Url -OutFile $ArchivePath
    $ActualPythonSha256 = (Get-FileHash -Algorithm SHA256 $ArchivePath).Hash.ToLowerInvariant()
    if ($ActualPythonSha256 -ne $PythonSha256) {
        Remove-Item $ArchivePath -ErrorAction SilentlyContinue
        throw "Python runtime SHA-256 mismatch: expected $PythonSha256, got $ActualPythonSha256"
    }

    Write-Host "Extracting Python..." -ForegroundColor Cyan
    tar -xzf $ArchivePath -C $VendorDir
    Remove-Item $ArchivePath
    # python-build-standalone's tarball extracts to a top-level "python/" directory.
    Rename-Item -Path (Join-Path $VendorDir "python") -NewName "python-runtime"
} else {
    $ActualVersion = & (Join-Path $RuntimeDir "python.exe") --version 2>&1
    if ($ActualVersion -ne "Python $PythonVersion") {
        throw "vendor/python-runtime has a different Python version; delete it and run again."
    }
}

Write-Host "Downloading and verifying yt-dlp $YtDlpVersion..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $YtDlpUrl -OutFile $YtDlpPath
$ActualYtDlpSha256 = (Get-FileHash -Algorithm SHA256 $YtDlpPath).Hash.ToLowerInvariant()
if ($ActualYtDlpSha256 -ne $YtDlpSha256) {
    Remove-Item $YtDlpPath -ErrorAction SilentlyContinue
    throw "yt-dlp SHA-256 mismatch: expected $YtDlpSha256, got $ActualYtDlpSha256"
}
$YtDlpExtractDir = Join-Path $VendorDir "yt-dlp-extracted"
if (Test-Path $YtDlpExtractDir) { Remove-Item -Recurse -Force $YtDlpExtractDir }
New-Item -ItemType Directory -Force -Path $YtDlpExtractDir | Out-Null
tar -xzf $YtDlpPath -C $YtDlpExtractDir
$SitePackages = Join-Path $RuntimeDir "Lib\site-packages"
New-Item -ItemType Directory -Force -Path $SitePackages | Out-Null
$YtDlpPackage = Join-Path $SitePackages "yt_dlp"
if (Test-Path $YtDlpPackage) { Remove-Item -Recurse -Force $YtDlpPackage }
Copy-Item (Join-Path $YtDlpExtractDir "yt-dlp\yt_dlp") $SitePackages -Recurse
Remove-Item -Recurse -Force $YtDlpExtractDir
Remove-Item $YtDlpPath

Write-Host "Downloading and verifying yt-dlp EJS scripts $EjsVersion..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $EjsUrl -OutFile $EjsPath
$ActualEjsSha256 = (Get-FileHash -Algorithm SHA256 $EjsPath).Hash.ToLowerInvariant()
if ($ActualEjsSha256 -ne $EjsSha256) {
    Remove-Item $EjsPath -ErrorAction SilentlyContinue
    throw "yt-dlp EJS SHA-256 mismatch: expected $EjsSha256, got $ActualEjsSha256"
}
$EjsPackage = Join-Path $SitePackages "yt_dlp_ejs"
$EjsMetadata = Join-Path $SitePackages "yt_dlp_ejs-$EjsVersion.dist-info"
if (Test-Path $EjsPackage) { Remove-Item -Recurse -Force $EjsPackage }
if (Test-Path $EjsMetadata) { Remove-Item -Recurse -Force $EjsMetadata }
& (Join-Path $RuntimeDir "python.exe") -m zipfile -e $EjsPath $SitePackages
Remove-Item $EjsPath

Write-Host "Downloading and verifying Deno $DenoVersion..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $DenoUrl -OutFile $DenoPath
$ActualDenoSha256 = (Get-FileHash -Algorithm SHA256 $DenoPath).Hash.ToLowerInvariant()
if ($ActualDenoSha256 -ne $DenoSha256) {
    Remove-Item $DenoPath -ErrorAction SilentlyContinue
    throw "Deno SHA-256 mismatch: expected $DenoSha256, got $ActualDenoSha256"
}
$ToolsDir = Join-Path $RuntimeDir "tools"
if (Test-Path $ToolsDir) { Remove-Item -Recurse -Force $ToolsDir }
New-Item -ItemType Directory -Force -Path $ToolsDir | Out-Null
Expand-Archive -Path $DenoPath -DestinationPath $ToolsDir
Remove-Item $DenoPath
Set-Content -Path $MarkerPath -Value $ExpectedMarker

Write-Host "Done. vendor/python-runtime includes CPython, yt-dlp, EJS scripts and Deno." -ForegroundColor Green
Write-Host "'cargo build -p ytbridge' will pick it up" -ForegroundColor Green
Write-Host "via .cargo/config.toml's PYO3_PYTHON and bundle it next to ytbridge.exe automatically." -ForegroundColor Green
