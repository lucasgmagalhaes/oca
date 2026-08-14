# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

oca — a native Rust video editor (`egui`/`eframe`) for cutting gameplay footage for the
PacoPaçoca YouTube channel. Headline features: automatic loudness normalization and
export-that-matches-the-source-bitrate. Full phased plan: [`features/request.md`](features/request.md).

## Status

- **Fase 1-3 (done):** probe/export/loudness/proxy via avbridge FFI; GStreamer preview
  pipeline; full timeline editing (select/split/trim/drag/copy-paste/context menu/composite
  blocks/multi-sequence tabs/resizable panels/filmstrip/waveforms); JSON save/load;
  background import/export queue.

- **Fase 4 (in progress):** Export and preview are timeline-aware.
  `ClipInstance::video_filter_chain()` builds per-clip avfilter chains for export;
  `build_video_filter_bin` covers a narrower subset for GStreamer preview.
  Effects wired to export: gain_db, crop, flip, color_filter, vignette,
  brightness/contrast/saturation, sharpen, chroma_key, mask_shape, blur, pixelize, shake,
  glitch, zoom, speed_factor, freeze_frame, transitions (fade/slide/zoom entry effects via
  `ClipSegment::transition_in` + `bridge.c` avfilter expressions; HardCut/None are no-ops).
  **Not wired to preview:** transitions (see TODO in `build_video_filter_bin`), vignette,
  chroma_key, mask_shape, gain_db, speed, glitch. pixelize/shake/zoom/freeze_frame now covered
  (pixelize: static scale-down/up via `videoscale`; shake/zoom: `videocrop` driven per-frame by
  a pad probe, zoom keyed off buffer PTS since preview has no fixed canvas fps; freeze_frame:
  `ui`'s `App` keeps the pipeline `Paused` at `source_in_secs` and advances the playhead by
  wall-clock time instead of pipeline position — see `preview_frozen_since`/`frozen_playhead`
  in `crates/ui/src/app/preview.rs`).
  **Chroma key caveat:** `colorkey` marks pixels transparent but the final `yuv420p`
  conform (on a single/background track) drops the alpha plane — keyed color still appears in
  output there despite being in `video_filter_chain()`. Same caveat applies to `mask_shape`'s
  `geq` alpha stage. Both only have a visible effect on a clip placed on an **overlay** track
  (`avbridge_encode_timeline_export_multi`'s track 1+), since `build_overlay_vfilter` has
  no per-track `format=yuv420p` conform before the `overlay` filter that composites them —
  track 0 (background) and single-track exports still drop the alpha.
  **Fixed:** Slide/Zoom transitions used to fail against the pinned FFmpeg build — two
  independent bugs, not one. (1) `drawbox`'s `x`/`y` don't expose a frame-count variable in
  this build at all (`n` is an undefined constant there, regardless of operator syntax) — Slide
  is now a `geq` per-pixel expression instead (`N` works fine in `geq`). (2) Letting `scale`
  actually renegotiate its output size every frame (`eval=frame`) reliably corrupted the heap
  once run through a real export (`STATUS_HEAP_CORRUPTION`, not just a parse error) — Zoom is
  now a `geq` inverse-sample too (output size never changes; only what each output pixel
  samples does), the same technique the mask stages above use. Both are implemented in
  `timeline_export.c`'s per-segment transition block and duplicated in
  `timeline_export_multi.c`'s `build_vfilter_descr`.

  **Also fixed, found while fixing the above:** the *other* zoom — `ClipInstance::zoom_start`/
  `zoom_end` (Ken-Burns), unrelated to `transition_in`'s Zoom entry effect — was silently
  broken for every non-degenerate case (`zoom_start != zoom_end`): its `crop=iw/(A+B*n):...`
  never set `eval=frame` either, and this FFmpeg build rejects a frame variable outright in a
  filter's default "init" eval mode, so the export failed with `ENCODE_ERR_FILTER_GRAPH`
  every time, not just failed to animate. No test caught it because every zoom-bearing fixture
  in the test suite happened to use `zoom_start == zoom_end` (the safe, static-crop branch).
  Fixed the same way, factored into a shared `build_kenburns_zoom()` (declared in
  `bridge_internal.h`, defined in `timeline_export_multi.c`) since the same fix was needed in
  three places: `timeline_export.c`'s per-segment block, and both of
  `timeline_export_multi.c`'s filter-string builders (`build_vfilter_descr`,
  `build_overlay_vfilter`). **Now covered end-to-end** by
  `crates/core/tests/timeline_export_multi_test.rs` — exercises the real two-track composite
  path (`resolve_timeline_segments_multi` + `render_export_job_multi`), including Slide/Zoom
  transitions, Ken-Burns zoom, and mask_shape all on the overlay track specifically, not just
  the C string builders in isolation.

  Buffers along all of these paths were bumped generously (up to 20480 bytes at the widest —
  `init_overlay_graph`'s `fstr`) since a `geq` expression here, or a RoundedRect mask, can
  each independently run past a thousand bytes, and several of these strings nest more than one
  of them.

- **Fase 5/6 (partially done):** Aspect ratio selection; prefs + export queue persisted
  to platform JSON (`~/Library/Application Support/oca/` on macOS); recent project list;
  debounced autosave + restore modal; crash detection + panic hook; prefs modal; home
  screen right-click context menu; file size estimate; structured logging; copy formatting
  (`Ctrl+Shift+C`/`V`); configurable key bindings (`KeyBindings`/`BindableAction` in
  `crates/ui/src/app/mod.rs`, capture UI in `screens/prefs.rs`, matched in
  `screens/editor/mod.rs` — covers play/pause, split, copy/paste formatting).
  Text overlays, layer masks, transitions, and multi-track compositing (Fase 4 items) are
  also done — see the Fase 4 section above. Output-folder overwrite/rename/cancel prompt is
  done (`PendingExportConflict`/`show_export_conflict_modal` in `crates/ui/src/app/`).

  **GPU encode (done, with a known gap):** `avbridge_encode_timeline_export`/`_multi` take a
  `gpu_encoder_preference` int (`GpuEncoderPreference` in `bridge_internal.h`: AUTO/CPU/NVENC/
  QUICKSYNC/AMF), threaded from a new `Prefs.gpu_encoder` setting (`screens/prefs.rs`'s
  "Encode por GPU" row) through `core::render`'s `render_export_job`/`render_export_job_multi`/
  `render_timeline_export`. `gpu_encoder.c`'s `open_video_encoder()` tries the requested
  hardware encoder(s) (AUTO tries NVENC, then Quick Sync, then AMF) and falls back to the CPU
  (libopenh264) encoder if `avcodec_open2` fails for any reason — no compatible GPU/driver, or
  the encoder rejects the pixel format handed to it. **Known gap, found by empirically running
  this on a GPU-less dev machine:** every attempt uses `AV_PIX_FMT_YUV420P` (matching what the
  filter chains already conform to, so no filter-graph changes are needed) — but `h264_qsv`
  explicitly rejected that format at `avcodec_open2` time (`"Specified pixel format yuv420p is
  not supported by the h264_qsv encoder"`, wants nv12/qsv instead), which the fallback logic
  correctly treats as "unavailable" and falls through to CPU. This means on a machine with real
  Intel Quick Sync hardware, `GPU_ENCODER_QUICKSYNC` would currently still silently fall back to
  CPU every time rather than actually using the hardware — converting to nv12 (an extra
  swscale/format-filter stage before `avcodec_send_frame`) isn't implemented. NVENC/AMF didn't
  reach the pix_fmt check at all on this dev machine (no NVIDIA GPU; no `amfrt64.dll`), so
  whether they accept yuv420p directly on real hardware is unverified either way. The CPU
  fallback itself is exercised end-to-end for every preference value
  (`encode_test.rs`'s `every_gpu_encoder_preference_falls_back_to_a_working_export`) — what's
  NOT verified anywhere in this codebase is a hardware encoder actually succeeding, since doing
  so needs real GPU hardware this environment doesn't have.

  **Not yet done:** export job reordering/pausing.

  **Whisper subtitles (done, one known limitation):** `avcore::transcribe::transcribe()`
  decodes a source's audio to 16kHz mono float PCM via a new `avbridge` function
  (`extract_pcm_16k_mono` / `pcm_extract.c`, same no-subprocess FFI approach as every other
  avbridge call) and runs it through a local Whisper model (`whisper-rs`, CPU-only default
  features) to produce timestamped segments. The Mídia screen's "Transcrever" button runs it in
  the background (`ui/src/app/transcribe.rs`) and drops the result as `TextClip`s onto the
  active sequence's Text track, anchored at the playhead. The model itself isn't bundled —
  Preferences has a "Modelo Whisper" field pointing at a local GGML file (e.g. `ggml-base.bin`
  from huggingface.co/ggerganov/whisper.cpp); empty means the feature is unconfigured.
  **Known limitation, a real upstream bug, not a design choice:** `whisper-rs` 0.16.0's
  `set_abort_callback_safe` has broken trampoline codegen (casts its double-boxed
  `Box<dyn FnMut() -> bool>` back to the *original* closure type instead of the box type,
  unlike `set_progress_callback_safe`'s correct version) — confirmed by empirical reproduction
  (any capturing closure, even a plain `bool`, makes whisper.cpp abort on the first internal
  check). `transcribe()` doesn't use it; cancellation only works before inference starts, not
  mid-run — see `transcribe.rs`'s doc comment for the full trace.

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
`python -m pip install -r e2e/requirements.txt`. Needs a debug build first and FFmpeg/GStreamer
DLL dirs on `PATH` (`make test-e2e` exports both). The `oca_window` fixture calls
`set_focus()` before yielding — without it, synthetic clicks land on whatever window has
focus.

## Architecture

Three-crate split, enforced by dependency direction: `avbridge` → `core` → `ui`.

- **`avbridge`** — thin C bridge (`csrc/bridge.c`) over libavformat/libavcodec/libavfilter/
  libavutil, called via FFI. Exposes probing, timeline export rendering, loudness
  measurement, proxy generation, and waveform extraction.
- **`core`** — project/timeline/media data model plus wrappers (`probe`, `render`,
  `loudness`, `proxy`, `waveform`), GStreamer playback pipeline (`preview`), and JSON
  save/load (`persistence`). Locale-neutral — stores enums, never pre-formatted strings.
  No mock/sample data anywhere.
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
