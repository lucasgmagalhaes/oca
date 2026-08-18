# FFmpeg Update Guide

This document explains how to update the pinned FFmpeg builds used in CI workflows.

## Security Context

The CI workflow downloads FFmpeg native libraries from BtbN's FFmpeg-Builds repository. To prevent supply-chain attacks where compromised or tampered libraries could execute arbitrary code during the build process, we:

1. **Pin to specific release tags** instead of using mutable `latest` URLs
2. **Verify SHA256 checksums** before extraction
3. **Fail the build** if checksums don't match

## Update Process

When you need to update FFmpeg (e.g., for bug fixes, new features, or security patches):

### 1. Choose a Release

Visit https://github.com/BtbN/FFmpeg-Builds/releases and select an immutable `autobuild-*` tag.
Use matching versioned `linux64-lgpl-shared` and `win64-lgpl-shared` assets, never the mutable
`latest` release or a GPL/nonfree variant.

### 2. Download Assets and Compute Checksums

Download both platform-specific assets and compute their SHA256 checksums:

#### Windows (PowerShell)
```powershell
$releaseTag = "autobuild-YYYY-MM-DD-HH-MM"
$asset = "ffmpeg-<build-id>-win64-lgpl-shared-<major>.zip"
$uri = "https://github.com/BtbN/FFmpeg-Builds/releases/download/$releaseTag/$asset"
Invoke-WebRequest -Uri $uri -OutFile ffmpeg-win64.zip
(Get-FileHash -Algorithm SHA256 ffmpeg-win64.zip).Hash
```

#### Linux (bash)
```bash
release_tag="autobuild-YYYY-MM-DD-HH-MM"
asset="ffmpeg-<build-id>-linux64-lgpl-shared-<major>.tar.xz"
uri="https://github.com/BtbN/FFmpeg-Builds/releases/download/${release_tag}/${asset}"
curl -fsSL -o ffmpeg-linux64.tar.xz "$uri"
sha256sum ffmpeg-linux64.tar.xz
```

### 3. Update Workflow Environment Variables

Edit `.github/workflows/ci.yml` and update the `env` section at the top:

```yaml
env:
  CARGO_TERM_COLOR: always
  FFMPEG_RELEASE_TAG: "autobuild-YYYY-MM-DD-HH-MM"
  FFMPEG_BUILD_ID: "n<version>-<revision>-g<commit>"
  FFMPEG_WIN64_SHA256: "abc123..."         # SHA256 from step 2 (Windows)
  FFMPEG_LINUX64_SHA256: "def456..."       # SHA256 from step 2 (Linux)
```

Update the FFmpeg entry in `packaging/bundle-manifest.json` with the same version, immutable
URLs and checksums so regular CI and release packaging cannot drift.

**Important:** All three values must be updated together. The checksums must match the specific release tag.

### 4. Update Cache Keys

The cache keys in the workflow automatically include the release tag and checksums, so they will invalidate automatically when you update the environment variables. No manual cache clearing is needed.

### 5. Test the Changes

1. Commit your changes to a branch
2. Open a pull request
3. Verify that both Windows and Linux CI jobs pass
4. Check the job logs to confirm checksum verification succeeded

## Troubleshooting

### Checksum Mismatch Error

If you see an error like:
```
FFmpeg checksum mismatch! Expected: abc123..., Got: def456...
This may indicate a compromised download or an outdated checksum.
```

This means either:
- The downloaded file was corrupted or tampered with (security issue)
- The checksum in the workflow doesn't match the release tag (configuration error)

**Do not ignore this error.** Verify that:
1. The `FFMPEG_RELEASE_TAG` matches the release you downloaded
2. You computed the checksum for the correct file
3. You copied the full checksum without truncation

### Download Failures

The workflow includes retry logic with exponential backoff. If downloads consistently fail:
- Check if the release tag exists at https://github.com/BtbN/FFmpeg-Builds/releases
- Verify the asset filenames haven't changed
- Check GitHub's status page for API issues

## Why This Matters

Without checksum verification, an attacker who compromises:
- The BtbN repository
- The GitHub Releases CDN
- A network path between GitHub Actions and the download source

...could inject malicious native libraries that execute arbitrary code when the build links against them. SHA256 verification ensures that only the exact, reviewed FFmpeg build is used in CI.
