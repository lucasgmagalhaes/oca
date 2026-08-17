# One-time setup: fetches a self-contained Python runtime (python-build-standalone) and
# installs yt-dlp into it, so `cargo build -p ytbridge` links against and ships a Python that
# needs nothing pre-installed on the machine it runs on. Run once after cloning, and again
# whenever bumping PYTHON_VERSION/RELEASE_TAG below. See CLAUDE.md's Fase 8 "Python runtime for
# ytbridge" entry for the full rationale.
#
# Windows-only for now — see ytbridge/build.rs's doc comment for why.

$ErrorActionPreference = "Stop"

$PythonVersion = "3.10.21"
$ReleaseTag = "20260814"
$AssetName = "cpython-$PythonVersion+$ReleaseTag-x86_64-pc-windows-msvc-install_only.tar.gz"
$Url = "https://github.com/astral-sh/python-build-standalone/releases/download/$ReleaseTag/$AssetName"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$VendorDir = Join-Path $RepoRoot "vendor"
$RuntimeDir = Join-Path $VendorDir "python-runtime"
$ArchivePath = Join-Path $VendorDir $AssetName

if (Test-Path $RuntimeDir) {
    Write-Host "vendor/python-runtime already exists — delete it first to re-fetch." -ForegroundColor Yellow
    exit 0
}

New-Item -ItemType Directory -Force -Path $VendorDir | Out-Null

Write-Host "Downloading $AssetName..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $Url -OutFile $ArchivePath

Write-Host "Extracting..." -ForegroundColor Cyan
tar -xzf $ArchivePath -C $VendorDir
Remove-Item $ArchivePath
# python-build-standalone's tarball extracts to a top-level "python/" directory.
Rename-Item -Path (Join-Path $VendorDir "python") -NewName "python-runtime"

Write-Host "Installing yt-dlp into the vendored runtime..." -ForegroundColor Cyan
& (Join-Path $RuntimeDir "python.exe") -m pip install --no-warn-script-location yt-dlp

Write-Host "Done. vendor/python-runtime is ready — 'cargo build -p ytbridge' will pick it up" -ForegroundColor Green
Write-Host "via .cargo/config.toml's PYO3_PYTHON and bundle it next to ytbridge.exe automatically." -ForegroundColor Green
