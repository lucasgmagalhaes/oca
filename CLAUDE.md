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

  **Motion tracking (region now user-editable):**
  `avcore::motion_tracking` — plain fixed-template block matching (full search within a
  radius each sampled frame, SAD against the *first* frame's template so it doesn't drift),
  no ML model. Reuses the keyframe system: `tracked_positions_to_keyframes()` converts the
  tracked path into a `position_keyframes` delta from the first tracked frame, added onto
  whatever fixed position was already set, so the layer rides the motion while keeping its
  initial placement. Fully unit tested against synthetic frames, including an off-center
  starting region (`tracks_a_block_starting_from_an_off_center_region`) so a future regression
  that silently re-hardcodes the center back to `(0.5, 0.5)` gets caught. `ui`'s "Rastrear
  movimento" button no longer always tracks a region centered on the frame — the properties
  panel now has region controls (center X/Y drag values, size/search-radius sliders, a
  "Centralizar região" reset button) right above it, defaulting to the old centered-region
  values but user-editable before clicking the button. `App::motion_track_center_x`/`_y`/
  `_size`/`_search_radius` are plain, non-persisted session state (a one-shot tracking job's
  input, not a durable clip property like `crop_x/y` — only the job's *output*
  `position_keyframes` is saved), read once at click time by
  `App::spawn_motion_track_selected_clip` and threaded through to `avcore::track_region`. No
  drag-on-preview picker yet (numeric entry only) — `avcore::track_region` still only supports
  a square region (`template_size_frac` is one scalar), not an independent width/height.

  **AI background removal (inference, matte generation, and export wiring all done —
  preview still not):** `avcore::background_removal::segment_person` runs MODNet
  (`ZHKKKe/MODNet`, ONNX export by `yakhyo/modnet`, Apache-2.0) against one decoded frame,
  returning a per-pixel alpha matte. Model isn't bundled yet —
  `avcore::model_download::download_background_removal_model` fetches it on demand, same shape
  as the Whisper/reframe model downloads. **Confirmed working end-to-end** on this dev machine:
  real inference against the downloaded model succeeded on a synthetic frame.

  **Matte generation (`ui`):** `App::spawn_generate_matte_for_selected_clip`
  (`crates/ui/src/app/background_removal.rs`) — the properties panel's "Gerar máscara" button,
  next to the "Remoção de fundo (IA)" checkbox. Samples frames across the clip's own trimmed
  source range via `avcore::preview::Preview` (same seek+poll pattern motion-tracking/auto-
  reframe use, coarser — `SAMPLES_PER_SEC = 2.0`, `MAX_SAMPLES = 40` — since each sample re-runs
  a whole ONNX session, see `segment_person`'s doc comment on the lack of session reuse across
  calls), runs `segment_person` on each, and encodes the resulting alpha values (rounded to
  `0..=255`) into a small H.264 video via `avcore::encode_matte_video` — saved to
  `avcore::background_removal::mask_cache_dir_for_project` (a hidden sibling folder next to the
  project file, same convention as `crate::proxy`'s editing-proxy cache), keyed by clip id via
  `mask_path_for_clip` (per-clip, not per-asset, since the matte covers this clip's own
  trim range). The matte's own declared fps is however many frames actually got sampled per
  second of the clip's real duration — a "stepped" alpha update rate slower than the clip's own
  frame rate, not frame-perfect, but the export-side compositor already tolerates a
  shorter/coarser overlay input by holding the last known frame (the same mechanism a track-1
  clip whose own duration doesn't match track 0's already relies on).

  **Encoding (`avbridge`):** `avbridge_encode_matte_video` (new file `csrc/matte_encode.c`) —
  encodes a flat `luma_frames` buffer (frames concatenated, `width*height` bytes each) into
  YUV420P H.264 via a forced `libopenh264` open (no GPU-preference ladder — this is a small
  internal artifact, not user-facing output), luma plane = the supplied alpha bytes, both
  chroma planes filled with neutral 128 ("grayscale-as-luma", not a true single-plane GRAY8
  stream — sidesteps unverified GRAY8-support-in-libopenh264 risk). Closely mirrors
  `proxy.c`'s existing encoder-open/frame-write/flush pattern, since that already does almost
  everything needed here minus the decode/scale side (frames arrive pre-decoded from Rust,
  no demuxer/decoder needed at all — the first avbridge function shaped this way). Rust wrapper
  `avbridge::encode_matte_video`/`avcore::encode_matte_video`.

  **Export compositing (`avbridge`):** `ClipSegment` gained a `mask_video_path` field (C
  struct in `bridge.h`, the `#[repr(C)]` `RawClipSegment` mirror, and the public
  `avbridge::ClipSegment` — all three kept in lockstep, plus both `CString`-marshalling call
  sites in `avbridge/src/lib.rs`) — empty string = no matte, same "always non-null, empty
  means unset" convention every other optional string field on this struct already uses.
  `core::render::resolve_timeline_segments_multi` populates it from
  `ClipInstance::background_removal_mask_path`, gated the same way `resolve_clip_filters`
  already gates `layer_scale`'s resize stage: **only on an overlay track** (`is_overlay`) —
  a single/background track's clips never reach a compositing stage, so their alpha (from
  this or any other source, e.g. chroma_key/mask_shape) is always discarded by the final
  `format=yuv420p` conform regardless, same documented caveat those two already carry.
  `timeline_export_multi.c`'s `init_overlay_graph` (previously a fixed 2-input
  `[in0]<f0>[v0];[in1]<f1>[v1];[v0][v1]overlay=...` graph) now optionally takes a third
  `vdec2`/`f2` pair, producing `[in0]<f0>[v0];[in1]<f1>[v1];[in2]<f2>,format=gray[m2];
  [v1][m2]alphamerge[v1a];[v0][v1a]overlay=...` when a matte is present — a second
  `OverlayDecoder` (`ov2`) opened/advanced exactly like the existing overlay-track decoder
  (`ov1`), against a synthetic `ClipSegment` pointing at the matte file with its own 0-based
  `source_in_secs`/`source_out_secs` (the matte's internal timeline, distinct from the
  original clip's timeline/source coordinates — see `mask_video_path`'s doc comment in
  `bridge.h` for the exact mapping). Falls back to compositing without the matte (not a hard
  export failure) if the matte file can't be opened or the 3-input graph fails to build —
  matches this codebase's general "an optional post-effect degrades gracefully rather than
  aborting a multi-minute render" posture. `alphamerge` **replaces**, not combines with, any
  alpha `f1`'s own chain already produced (e.g. simultaneous chroma_key/mask_shape on the same
  clip) — combining multiple alpha sources on one clip isn't supported.

  **Verification caveat:** this specific C work (the `init_overlay_graph` 3-input extension
  and `matte_encode.c`) was written and reviewed carefully but could not be exercised against
  a real render on any machine during development — same "this dev machine's FFmpeg build
  can't open any encoder" gap noted below applies. It *was*, however, syntax/type-checked for
  real: `gcc -fsyntax-only -I <ffmpeg include dir>` against each modified/new `.c` file
  individually (works even when the full crate can't build, since it only needs the headers
  the *specific* file includes — `filters.c`'s separate FFmpeg-7.1-only API usage doesn't
  block syntax-checking files that don't call those functions) came back clean, and the
  before/after brace/paren-count delta across the whole file matched exactly, both useful
  fallback techniques when `cargo check`/`build` itself is blocked in a sandbox missing a
  new-enough FFmpeg. Not a substitute for an actual render — treat the export-compositing path
  here as unverified beyond static analysis until it's run for real once.

  **Not yet done:** preview compositing (no `alphamerge` stage in the GStreamer preview
  pipeline — same gap chroma_key/mask_shape/position keyframes already have there).

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

  **Geometric shapes (data model, export rendering, and UI all wired now):**
  `TrackKind::Shape`/`ShapeClip` — rectangle/square, ellipse/circle, triangle, trapezoid,
  arrow presets (`ShapeKind::rectangle()` etc.), plus a `Polygon(Vec<(f32,f32)>)` variant that
  also covers `request.md`'s "forma personalizada" (custom shape) ask — vertex-editing UI for a
  hand-drawn custom polygon still doesn't exist (a real, separate follow-up), but every fixed
  preset is placeable and editable now. `avcore::shape_render` builds a `geq` avfilter node per
  shape (rotation via a per-pixel coordinate rotation, ellipse via a quadratic test, every
  straight-edged shape via ray-casting point-in-polygon — correct for the arrow's concave
  notches), applied as a post-processing pass (`avbridge::apply_shape_overlays`) the same way
  text overlays already are. **Confirmed working for real**, not just parse-checked: rendered
  the generated filter against a synthetic frame and visually verified correct position/
  rotation/color, including catching two real bugs (RGB not converted to YCbCr; yuv420p
  chroma-subsampled coordinates not matching luma's) before they shipped.
  UI (`App::add_shape_track`/`add_shape_clip`, `timeline_ops.rs`): toolbar "S+ Forma"/"+
  Adicionar forma" buttons (`screens/editor/mod.rs`'s `toolbar()`); `add_shape_clip` auto-
  creates a shape track if none exists yet (unlike `add_text_clip`, which stays a no-op without
  one — there's no drag-and-drop path onto a shape track the way there is for media assets, so
  the button has to be able to place the first clip itself). Timeline strip renders each shape
  clip as a colored block with a preset glyph (`shape_kind_glyph` in `timeline_panel.rs`),
  click-to-select and a context-menu delete, same interaction shape as text clips. Properties
  panel (`shape_clip_properties` in `properties_panel.rs`) edits preset/color/center position/
  size/rotation/outline thickness/timing via the same clone-mutate-writeback pattern
  `text_clip_properties` uses. **Preview still not implemented** — same gap `TextClip` has
  (`ShapeClip`'s own doc comment flags it); the timeline block is a color/glyph stand-in only,
  the real render only happens on export.

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

**Ubuntu 24.04's packaged FFmpeg (6.1.1, via `apt install libavformat-dev` etc.) is too old
to build `avbridge`.** `csrc/filters.c` calls `avcodec_get_supported_config`/
`AV_CODEC_CONFIG_SAMPLE_FORMAT`/`AV_CODEC_CONFIG_SAMPLE_RATE`, added in FFmpeg 7.1 — a plain
`apt`-installed FFmpeg dev build on noble fails with "undeclared identifier" on that file
specifically, before reaching any of this project's own logic. A sandboxed session without
network access to fetch a newer FFmpeg build (or without access to `cdn.pyke.io` for `ort`'s
prebuilt ONNX Runtime binaries — set `ORT_SKIP_DOWNLOAD=1` to defer *that* failure past `cargo
check`, same idea as the GPU-encoder note below) can't get a full `cargo check`/`build`/`test`
to pass — same category of pre-existing, machine-specific build gap as the GPU encoder note
below, not a code regression. `cargo fmt --check` still works (parses each file independently,
no dependency build needed) and is a reasonable sanity check when a full build isn't possible.
For `avbridge/csrc/*.c` changes specifically, `gcc -fsyntax-only -I <ffmpeg-include-dir>
-Wall -Wextra <file>.c` run per-file from `crates/avbridge/csrc/` is a real (if partial)
compiler check that still works even when the whole crate can't build — it only needs the
headers *that file* includes, so it stays clean for every file except `filters.c` itself even
on Ubuntu's too-old packaged FFmpeg. It won't catch link-time or runtime/filtergraph-semantic
issues, but it does catch real syntax/type errors no amount of manual review guarantees.

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
