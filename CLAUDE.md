# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

oca — a native Rust video editor (`egui`/`eframe`) for cutting gameplay footage for the
PacoPaçoca YouTube channel. Headline features: automatic loudness normalization and
export-that-matches-the-source-bitrate. Full phased plan: [`features/request.md`](features/request.md).

## Status

- **Fase 1-3 — done.** probe/export/loudness/proxy via avbridge FFI; GStreamer preview
  pipeline; full timeline editing (select/split/trim/drag/copy-paste/context menu/composite
  blocks/multi-sequence tabs/resizable panels/filmstrip/waveforms); `.ocproj` save/load;
  background import/export queue.

- **Fase 4 — in progress.** Export and preview are timeline-aware.
  `ClipInstance::video_filter_chain()` builds per-clip avfilter chains for export;
  `build_video_filter_bin` covers a narrower subset for GStreamer preview.

  **Wired to export:** gain_db, crop, flip, color_filter, vignette, brightness/contrast/
  saturation, sharpen, chroma_key, mask_shape, blur, pixelize, shake, glitch, zoom,
  speed_factor, freeze_frame, deflicker, transitions (fade/slide/zoom via `ClipSegment::
  transition_in`), 3D LUTs (`ClipInstance::lut_path`, no preview element exists for this),
  video stabilization (`stabilization_intensity` -> `deshake`, single-pass only — this
  build has `--disable-libvidstab`), a general keyframe system (position/scale/rotation/
  opacity via `Keyframe<T>` lists — see below), layer transform (position drag + resize),
  layer templates, auto-reframe, motion tracking.

  **Wired to preview:** scale/rotation/opacity keyframes, pixelize/shake/zoom/freeze_frame.
  **Not wired to preview:** transitions, vignette, chroma_key, mask_shape, gain_db, speed,
  glitch, deflicker, LUTs, stabilization, position keyframes (needs multi-track preview
  compositing that doesn't exist yet — single-clip `playbin` pipeline has no `overlay` stage).

  **Alpha/overlay-track caveat (applies to chroma_key, mask_shape, position/opacity
  keyframes, layer resize):** these only have a visible effect on a clip placed on an
  **overlay track** (`avbridge_encode_timeline_export_multi`'s track 1+) — a single/background
  track's final `format=yuv420p` conform drops the alpha plane they need, and has no
  pad/fit step for a resized footprint either.

  **Keyframe system:** `crate::keyframe` — position/scale/rotation/opacity each animate via
  a `Vec<Keyframe<T>>` on `ClipInstance`, piecewise-linearly interpolated
  (`evaluate_keyframes`). Since `ClipSegment` crosses the Rust→C FFI boundary as `#[repr(C)]`,
  each property's piecewise avfilter expression is built in Rust and passed across as a
  plain string (`keyframe_video_filter_chain`, `position_overlay_xy_expr`) — no new C
  functions. Splitting a keyframed clip rescales and inserts a continuity-preserving
  boundary point (`split_keyframes_at`). Visual markers (read-only diamonds) show on the
  timeline clip block; editing goes through the properties panel's list editor.
  **Verification caveat:** the filter-graph correctness of rotation/position's `t`/`if`/
  `between` expressions has not been empirically confirmed against a real render on any
  machine — this dev machine's FFmpeg build can't open any encoder at all right now (see
  GPU encode note below), so every encode-path test fails here regardless.

  **Auto-reframe (static crop only, not animated):** `avcore::auto_reframe` — reuses
  `ClipInstance`'s static `crop_x/y/w/h` (no new keyframe track), matching `request.md`'s own
  wording: a one-shot AI recenter on aspect-ratio change, then the existing crop controls are
  the "manual adjustment on top". `compute_reframe_crop()` is pure geometry (fully unit
  tested, no ONNX). `detect_faces()` runs UltraFace `version-RFB-320`
  (`Linzaer/Ultra-Light-Fast-Generic-Face-Detector-1MB`, MIT) via the `ort` crate
  (`download-binaries` feature — **statically links** `onnxruntime` into the binary, not a
  runtime `.dll`). Model isn't bundled with the installer yet —
  `avcore::model_download::download_reframe_model` fetches it on demand into
  `<prefs dir>/models/`, same shape as the Whisper model download (both share
  `model_download.rs`'s `download_file()` helper). Falls back to a centered crop (with a
  toast) when no face is found. **Confirmed working end-to-end** on this dev machine: a real
  inference pass against the downloaded model succeeded on the `core` test fixture (0 faces
  found, as expected for a fixture with no real face in it).

  **Motion tracking (single centered region, no ROI picker yet):**
  `avcore::motion_tracking` — plain fixed-template block matching (full search within a
  radius each sampled frame, SAD against the *first* frame's template so it doesn't drift),
  no ML model. Reuses the keyframe system: `tracked_positions_to_keyframes()` converts the
  tracked path into a `position_keyframes` delta from the first tracked frame, added onto
  whatever fixed position was already set, so the layer rides the motion while keeping its
  initial placement. Fully unit tested against synthetic frames. `ui`'s "Rastrear movimento"
  button always tracks a region centered on the frame — no way to pick a different region yet
  (a real gap for an off-center subject).

  **AI background removal (ONNX inference done, export/preview wiring not started):**
  `avcore::background_removal::segment_person` runs MODNet (`ZHKKKe/MODNet`, ONNX export by
  `yakhyo/modnet`, Apache-2.0) against one decoded frame, returning a per-pixel alpha matte.
  Model isn't bundled yet — `avcore::model_download::download_background_removal_model`
  fetches it on demand, same shape as the Whisper/reframe model downloads. **Confirmed working
  end-to-end** on this dev machine: real inference against the downloaded model succeeded on a
  synthetic frame. `ClipInstance::background_removal_enabled`/`background_removal_mask_path`
  exist and have a properties-panel checkbox, but unlike auto-reframe/motion-tracking there's
  no background job wired up yet to actually generate a matte file, and no `alphamerge`-based
  compositing stage in the export pipeline to consume one — same "field is real, only a UI
  toggle for now" gap `gain_db`/`blur_intensity` shipped with before their own wiring landed.
  Next step: an avbridge function that encodes Rust-supplied raw frames into a plain
  grayscale-as-luma H.264 video (reusing the already-working `libopenh264` encoder, no new
  alpha-codec requirement), then a per-clip second filtergraph input + `alphamerge` in
  `timeline_export_multi.c` (which already builds two-input filtergraphs for overlay-track
  compositing — `src0`/`src1` in that file — so this reuses an existing pattern rather than
  inventing one).

  **Text-to-speech (done, real end-to-end):** `avcore::text_to_speech` — `espeak-rs`
  (statically-linked espeak-ng, no runtime DLL; `core/build.rs` copies its `espeak-ng-data`
  next to the workspace's build output, since `espeak-rs` looks for that directory name next
  to the running executable) phonemizes typed text, then a Piper VITS ONNX model
  (`rhasspy/piper-voices`' pt_BR `faber` voice, MIT) synthesizes it — Piper's phoneme-ID scheme
  (BOS/EOS/interspersed-PAD) and ONNX I/O contract (`input`/`input_lengths`/`scales` ->
  `output`) confirmed against a real downloaded voice, not guessed. **Confirmed working
  end-to-end** on this dev machine: real synthesis of a Portuguese sentence produced a WAV with
  real (non-silent, non-garbage) audio content. Model isn't bundled yet —
  `avcore::download_tts_voice` fetches both the `.onnx` and its `.onnx.json` sidecar on demand.
  UI: Mídia screen's "Texto-pra-fala" button opens a text modal; "Gerar" synthesizes on a
  background thread and imports the resulting WAV through the exact same pipeline as a
  drag-and-drop import ([`App::spawn_import`]) — no separate asset-creation path needed.

- **Fase 5/6 — partially done.** Aspect ratio selection; prefs (`prefs.oc` — same
  gzip-compressed MessagePack framing `.ocproj` uses, via `avcore::to_ocproj_bytes`/
  `from_ocproj_bytes` generalized to any `Serialize`/`DeserializeOwned` value) + export queue
  (`queue.json`, still plain JSON) persisted to the platform config dir; recent project list;
  debounced autosave + restore modal; crash detection + panic hook; prefs modal; home screen
  context menu; file size estimate; structured logging; copy formatting (`Ctrl+Shift+C`/`V`);
  configurable key bindings; export job reordering; output-folder overwrite/rename/cancel
  prompt; word-highlight subtitles (single-line only — a caption that wraps in `drawtext`
  gets every highlight positioned as if still on one line); Whisper subtitles (model fetched
  on demand via `avcore::model_download`, not bundled); music/SFX library scanning (no
  bundled content, points at a user-configured folder).

  **GPU encode — hardware success unverified.** `gpu_encoder.c`'s `open_video_encoder()`
  tries NVENC/Quick Sync/AMF per `Prefs.gpu_encoder`, falling back to CPU (libopenh264) on
  failure; `h264_qsv` correctly requests NV12 (not the yuv420p every other encoder uses) via
  `pix_fmt_for_encoder_name()`. No machine this was developed on has NVENC/AMF hardware, and
  this dev machine's FFmpeg build has neither `libopenh264` nor `h264_qsv` compiled in at
  all — every test that reaches `avcodec_open2`/`avcodec_send_frame` fails with
  `EncodeError::Encoder` here specifically, a pre-existing build gap, not a regression.

Check `features/request.md` for what's still unbuilt before assuming a feature is live —
when in doubt, `graphify query`.

## Commands

Requires Rust (stable) via rustup. `avbridge` needs `FFMPEG_DIR` set to an FFmpeg dev
build (`include/`+`lib/`); its DLLs must be on `PATH` at runtime. `core`'s `preview`
module needs GStreamer discoverable via `PKG_CONFIG_PATH`. `core`'s `whisper-rs` dependency
needs `LIBCLANG_PATH` pointing at a libclang install (bindgen) and a build directory that
isn't under `%TEMP%` on Windows (MSVC's FileTracker fails there — `FTK1011`).

**Do not remove `avbridge/build.rs`'s import-lib-renaming step.** GStreamer bundles its
own FFmpeg (gst-libav) with identically named import libs — `build.rs` copies them into
`OUT_DIR` under unique names to prevent silent ABI-mismatch linking at runtime.

```bash
make build     # debug build, whole workspace
make run       # run the GUI app (debug)
make test      # run every crate's test suite
make test-core # cargo test -p core only
make test-e2e  # pytest + pywinauto against a built target/debug/ui.exe
make bench     # criterion benchmarks -> target/criterion/report/index.html
make fmt       # cargo fmt --all
make lint      # cargo clippy --workspace --all-targets
make check     # cargo check --workspace --all-targets (fast compile-only loop)
```

Single test: `cargo test -p core --test probe_test measure_loudness` or
`cargo test -p ui i18n::tests`.

**End-to-end tests** (`e2e/`) drive the real `ui.exe` via Windows UI Automation
(`pytest` + `pywinauto`, `backend="uia"`). One-time setup:
`python -m pip install -r e2e/requirements.txt`. **Use a real Python, not a Windows Store
alias** — a Store-packaged `python.exe` carries MSIX package identity that breaks DLL
loading for spawned child processes (`ui.exe` crashes with `STATUS_DLL_NOT_FOUND` even with
`PATH` set correctly), while the identical binary launched directly from PowerShell works
fine. Needs a debug build first and FFmpeg/GStreamer DLL dirs on `PATH` (`make test-e2e`
exports both). The `oca_window` fixture calls `set_focus()` before yielding — without it,
synthetic clicks land on whatever window has focus.

## Architecture

Three-crate split, enforced by dependency direction: `avbridge` → `core` → `ui`.

- **`avbridge`** — thin C bridge (`csrc/bridge.c`) over libavformat/libavcodec/libavfilter/
  libavutil, called via FFI. Exposes probing, timeline export rendering, loudness
  measurement, proxy generation, and waveform extraction.
- **`core`** — project/timeline/media data model plus wrappers (`probe`, `render`,
  `loudness`, `proxy`, `waveform`), GStreamer playback pipeline (`preview`), and `.ocproj`
  save/load (`persistence`) — gzip-compressed MessagePack (struct-map mode, so
  `#[serde(default)]` still lets an older-saved project load after a new field is added).
  Locale-neutral — stores enums, never pre-formatted strings. No mock/sample data anywhere.
- **`ui`** — eframe/egui GUI (glow/OpenGL): `app.rs` holds `App` and mutation methods;
  `screens/` has one module per screen; `theme.rs` is the dark/teal palette; **all UI
  strings live in `i18n.rs`** (pt-BR and English) — never hardcode display text.

Test placement: `core` and `avbridge` have `[lib]` targets → integration tests in
`crates/<crate>/tests/`. `ui` is bin-only → unit tests in `src/<module>/<module>_test.rs`.

## Approach

- Read existing files before writing. Don't re-read unless changed.
- Thorough in reasoning, concise in output.
- Skip files over 100KB unless required.
- No sycophantic openers or closing fluff. No emojis or em-dashes.
- Do not guess APIs, versions, flags, or package names. Verify by reading code or docs.
- Write all code and commit messages in English.
- Commit using Conventional Commits format (`feat:`, `fix:`, `refactor:`, etc.).

## graphify

This project has a knowledge graph at `graphify-out/` with god nodes, community structure,
and cross-file relationships.

- For codebase questions, first run `graphify query "<question>"` when `graphify-out/graph.json`
  exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"`
  for focused concepts.
- If `graphify-out/wiki/index.md` exists, use it for broad navigation.
- Read `graphify-out/GRAPH_REPORT.md` only for broad architecture review or when
  query/path/explain don't surface enough context.
- After modifying code, run `graphify update .` to keep the graph current.
