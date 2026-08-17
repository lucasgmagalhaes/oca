# Auto-update release contract

oca checks the latest published GitHub Release at startup. A newer release can be installed in
place on Windows and Linux when that release contains the exact archive for the running Rust
target triple.

## Asset names

| Target | Release asset | Executable inside archive |
| --- | --- | --- |
| `x86_64-pc-windows-msvc` | `oca-x86_64-pc-windows-msvc.zip` | `ui.exe` |
| `x86_64-unknown-linux-gnu` | `oca-x86_64-unknown-linux-gnu.tar.gz` | `ui` |
| `x86_64-unknown-linux-gnu` (AppImage) | `oca-x86_64-unknown-linux-gnu-appimage.tar.gz` | `oca.AppImage` |

Additional architectures use the same `oca-<target>.<extension>` convention, with the
`-appimage` suffix for AppImage packages. The AppImage archive must preserve its executable bit.
A checksum or signature file does not count as the package: the archive name must match exactly.
Draft and prerelease releases are not returned by GitHub's `releases/latest` endpoint.

The archive contains only the executable replaced by `self_update`. Native runtime libraries,
models, GStreamer plugins, and other installer-owned files must keep backward compatibility with
that executable until the full Fase 8 installer/bundled-engine release process is implemented.

## Application flow

1. Startup fetches the latest release metadata and verifies that its asset list contains the
   exact archive for the current target.
2. The About modal offers **Download and install** only when the platform and asset are both
   supported. Otherwise it keeps the manual GitHub Releases link.
3. The worker fetches the selected tag again, validates the asset selected by `self_update`,
   downloads and extracts it into a temporary directory, then atomically replaces the installed
   executable. Under AppImage, `$APPIMAGE` selects and replaces the outer AppImage file rather
   than the read-only executable inside its mounted filesystem.
4. The old process remains open. The user explicitly chooses **Restart now**; oca launches the
   replaced executable and exits through eframe's normal shutdown path.

An installation can fail when the executable's directory is read-only, antivirus software locks
the file, the release archive is malformed, or the network is unavailable. The modal then offers
retry and keeps the manual release link.

## Publishing checklist

1. Update the workspace version and tag the commit as `v<version>`.
2. Build the release binary for every supported target.
3. Put `ui`, `ui.exe`, or `oca.AppImage` at the root of its archive and use the exact asset names
   above.
4. Publish a non-draft, non-prerelease GitHub Release for the tag.
5. Install the previous version on each platform and verify check, install, and restart against
   the published release before announcing it.
