# Runtime dependency bundle

oca release artifacts are self-contained. The application never downloads models, media
engines, Python packages, fonts or codec plugins while it is running. Network access during
normal use is limited to features whose purpose is network access: checking GitHub Releases,
applying an update, and downloading media from a URL supplied by the user.

`packaging/bundle-manifest.json` is the machine-readable source of truth for versions, pinned
download URLs, SHA-256 checksums, licenses and bundle placement. `packaging/fetch_models.py`
downloads model files only while assembling a release. `packaging/validate_bundle.py` blocks
publication when any required runtime component is missing.

## Included components

| Component | Included form | Used by |
|---|---|---|
| FFmpeg 8.1 LGPL build | `ffmpeg` CLI, libav shared libraries | Native bridge, yt-dlp merge/extraction |
| GStreamer (1.26.11 Windows; Ubuntu 24.04 runtime on Linux) | Core libraries, plugin scanner and complete installed plugin set | Preview, decoding and scrubbing |
| ONNX Runtime 1.28 | Statically linked by `ort-sys` | Auto-reframe and background removal |
| whisper.cpp | Statically linked by `whisper-rs` | Automatic subtitles |
| Whisper Base | `resources/models/ggml-base.bin` | Automatic subtitles |
| UltraFace | `resources/models/version-RFB-320_simplified.onnx` | Auto-reframe |
| MODNet | `resources/models/modnet_photographic.onnx` | Background removal |
| Piper Faber pt-BR | ONNX model plus JSON sidecar | Text-to-speech |
| eSpeak NG | Statically linked engine plus `espeak-ng-data/` | Piper phonemization |
| CPython 3.10.21 | Private runtime beside `ytbridge` | YouTube downloader helper |
| yt-dlp 2026.07.04 | Verified release package copied into private Python `site-packages` | YouTube downloader helper |
| yt-dlp EJS 0.8.0 | Verified wheel extracted into private Python `site-packages` | YouTube signature extraction |
| Deno 2.9.5 | Private executable beside `ytbridge` | JavaScript runtime required by current yt-dlp YouTube support |
| Application fonts | Compiled into the Rust binary with `include_bytes!` | Titles, subtitles and text metrics |
| Inno Setup 6.7.1 | Build-time compiler, not an installed runtime | Configurable Windows installer |

Each package also carries `resources/DEPENDENCIES.json`, generated directly from `Cargo.lock`
and the bundle manifest. It inventories every Rust package with its exact version, registry
source and checksum as well as every native/model dependency. It also records the path, size
and SHA-256 of every actual file in the assembled payload, including shared libraries,
GStreamer plugins and private Python files; validation rejects missing, additional or changed
files. Font OFL notices and the oca
license are included under `resources/licenses/`. `resources/RUNTIME_VERSIONS.txt` records the
actual FFmpeg, GStreamer and Python versions reported by the assembled payload.
It also records yt-dlp, yt-dlp EJS and Deno versions. Bundle validation recalculates every
model's hash and rejects a runtime-version mismatch, so an accidental script/manifest drift
cannot be published.

The Windows ZIP contains the DLLs beside `ui.exe`, where the Windows loader can resolve them.
The Inno Setup installer contains that same validated tree and adds a configurable installation
directory, Start menu/optional desktop shortcuts and uninstallation; setup performs no network
downloads. Inno Setup itself is needed only on the release runner that compiles the installer
and is not installed on end-user machines. The installer is currently unsigned; release signing
requires configuring a Windows code-signing certificate in CI.
The Linux portable tree uses a launcher that sets private library, plugin and tool paths before
starting `ui-bin`; the AppImage wraps that exact validated tree. `OCA_RESOURCE_DIR` can override
the default `resources/`-beside-the-executable lookup for development and diagnostics.

## Deliberate system contract

Operating-system kernels, the Windows Universal C Runtime, Linux glibc, graphics/display stacks
and GPU vendor drivers are platform contracts rather than application payloads. Hardware
acceleration remains optional and falls back to CPU paths when a compatible vendor driver is not
available. Explorer on Windows and `xdg-open` on Linux are optional desktop integrations used
only by "open folder" actions; failure to find them does not affect editing or export. The exact
minimum contracts are recorded in the manifest.

Music and SFX are user media, not application dependencies, and are therefore not shipped.

## Release process

Pushing a `v*` tag runs `.github/workflows/release.yml`. It builds Windows and Linux from the
same source revision, verifies every downloaded archive, assembles complete portable trees,
validates them, creates the Windows portable ZIP, Windows Inno Setup executable, Linux tarball
and Linux AppImage, then publishes those four assets to the matching GitHub Release. A manually dispatched workflow builds and retains
the artifacts without publishing a release.

To update a dependency, update its URL, version and SHA-256 in the manifest together. For
FFmpeg, mirror the same pin in `.github/workflows/ci.yml`; see `.github/FFMPEG_UPDATE.md`.
