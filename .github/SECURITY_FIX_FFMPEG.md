# Security Fix: FFmpeg Download Verification

## Summary

This patch mitigates the risk of arbitrary code execution in GitHub Actions CI by implementing SHA256 checksum verification for FFmpeg downloads and pinning to specific release versions instead of using mutable `latest` URLs.

## Changes Made

### 1. `.github/workflows/ci.yml`

#### Environment Variables (lines 9-16)
- Added `FFMPEG_RELEASE_TAG` to pin to a specific BtbN release instead of `latest`
- Added `FFMPEG_WIN64_SHA256` for Windows FFmpeg archive checksum
- Added `FFMPEG_LINUX64_SHA256` for Linux FFmpeg archive checksum
- Added documentation comments explaining the security purpose

#### Windows Job (lines 49-101)
- Updated cache key to include release tag and checksum for proper invalidation
- Modified download step to use pinned release tag instead of `latest`
- Added placeholder validation check that fails fast with clear error message
- Added SHA256 checksum verification after download, before extraction
- Added error messages explaining potential security implications of checksum mismatches

#### Linux Job (lines 151-202)
- Updated cache key to include release tag and checksum for proper invalidation
- Modified download step to use pinned release tag instead of `latest`
- Added placeholder validation check that fails fast with clear error message
- Added SHA256 checksum verification after download, before extraction
- Added error messages explaining potential security implications of checksum mismatches

### 2. `.github/FFMPEG_UPDATE.md` (new file)

Created comprehensive documentation explaining:
- The security context and threat model
- Step-by-step process for updating FFmpeg versions
- How to compute checksums on both Windows and Linux
- Troubleshooting guidance for common issues
- Why this security measure matters

## Security Impact

### Before
- FFmpeg was downloaded from a mutable `releases/latest` URL
- No integrity verification was performed
- A compromised upstream release or man-in-the-middle attack could inject malicious shared libraries
- Malicious `.so`/`.dll` files would be linked and loaded during build/test, achieving code execution

### After
- FFmpeg is downloaded from a pinned, immutable release tag
- SHA256 checksums are verified before extraction
- Any tampering or corruption is detected and fails the build
- Clear error messages guide maintainers to investigate checksum mismatches
- Cache keys include checksums, preventing cached compromised artifacts

## Attack Surface Reduction

This fix eliminates the following attack vectors:
1. **Upstream compromise**: Even if the BtbN repository is compromised, attackers cannot inject malicious code without also updating the checksums in our workflow (which requires a PR review)
2. **Release asset tampering**: If a release asset is modified after initial publication, the checksum mismatch will be detected
3. **Man-in-the-middle attacks**: Network-level tampering during download will be caught by checksum verification
4. **Supply chain confusion**: Pinning to specific releases prevents accidental use of untested/unreviewed FFmpeg versions

## Maintenance Requirements

Maintainers must:
1. Follow the documented process in `.github/FFMPEG_UPDATE.md` when updating FFmpeg
2. Compute and verify checksums for both Windows and Linux assets
3. Update all three environment variables (`FFMPEG_RELEASE_TAG`, `FFMPEG_WIN64_SHA256`, `FFMPEG_LINUX64_SHA256`) together
4. Test the updated workflow in a PR before merging

## Implementation Notes

- Placeholder checksums are used initially and must be replaced with actual values
- The workflow includes validation to fail fast if placeholders are detected
- Retry logic with exponential backoff is preserved for transient download failures
- Cache invalidation is automatic when checksums change
- Error messages clearly distinguish between security issues and configuration errors

## Testing

Before the workflow can run successfully, the placeholder checksums must be replaced with actual SHA256 values computed from the specified release tag. The workflow will fail with a clear error message if placeholders are still present.

To test after updating checksums:
1. Push changes to a branch
2. Open a pull request
3. Verify both Windows and Linux jobs pass
4. Check job logs to confirm "FFmpeg checksum verified" messages appear

## References

- Original pentest finding: "Unverified FFmpeg CI download allows upstream-tampered native libraries to execute code in GitHub Actions"
- BtbN FFmpeg Builds: https://github.com/BtbN/FFmpeg-Builds
- Update documentation: `.github/FFMPEG_UPDATE.md`
