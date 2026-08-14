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
  glitch, zoom, speed_factor, freeze_frame, deflicker (`ClipInstance::deflicker_enabled` ->
  `deflicker=mode=am:size=5`, applied right after crop before any color/style stage), transitions
  (fade/slide/zoom entry effects via `ClipSegment::transition_in` + `bridge.c` avfilter
  expressions; HardCut/None are no-ops).
  **Not wired to preview:** transitions (see TODO in `build_video_filter_bin`), vignette,
  chroma_key, mask_shape, gain_db, speed, glitch, deflicker. pixelize/shake/zoom/freeze_frame now covered
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

  **General keyframe system (done, export only):** `crate::keyframe` (new module) — position,
  scale, rotation, and opacity can each be animated by a list of `Keyframe<T>` points
  (`ClipInstance::position_keyframes`/`scale_keyframes`/`rotation_keyframes`/
  `opacity_keyframes`), piecewise-linearly interpolated between the two points surrounding a
  given `time_fraction` (`evaluate_keyframes`) — per `request.md`'s Fase 4 "Keyframes" spec.
  Replaces `zoom_start`/`zoom_end` (a 2-keyframe `scale_keyframes` list reproduces the same
  Ken-Burns behavior as a degenerate case; splitting a keyframed clip now properly rescales and
  inserts a continuity-preserving boundary point via `split_keyframes_at`, fixing a real gap the
  old zoom split behavior had — it used to just copy the same two values onto both halves
  unscaled). **Architecture:** since `avbridge::ClipSegment` crosses the Rust→C FFI boundary as
  a `#[repr(C)]` struct, a `Vec<Keyframe<T>>` can't be a field on it directly — instead, the
  piecewise avfilter expression for each property is built in Rust
  (`ClipInstance::keyframe_video_filter_chain` for scale/rotation/opacity, folded onto the front
  of `video_filter`; `keyframe::position_overlay_xy_expr` for position, threaded into
  `init_overlay_graph`'s `overlay=x:y` as two new `ClipSegment` string fields) and passed across
  FFI as a plain string, the same way `video_filter` already was — no new C functions,
  `build_kenburns_zoom` and its 3 call sites are deleted outright. Scale/opacity use `N`
  (frame count, matching the existing `geq`-based zoom/mask convention); rotation/position use
  `t` (seconds) since `rotate`/`overlay` are evaluated through FFmpeg's general per-option
  expression framework, not `geq`'s per-pixel one — confirmed via `ffmpeg -h filter=rotate`/
  `-h filter=overlay` on the pinned build, not guessed. Opacity reuses `mask_shape`'s exact
  existing alpha-composition pattern (`format=yuva420p,geq=...:a='alpha(X,Y)*<ramp>'`, so it
  composes correctly with an existing mask/chroma-key alpha instead of clobbering it) and
  therefore has the same overlay-track-only caveat mask_shape/chroma_key already have; position
  has the same caveat for the same reason (no compositing stage on a single/background track).
  **Not yet done:** live GStreamer preview — only scale keyframes are wired into preview
  (`preview.rs`'s existing Ken-Burns pad-probe now reads `scale_keyframes` via
  `evaluate_keyframes` instead of the old two fixed endpoints, so it already supports arbitrary
  keyframe counts for free); rotation/position/opacity have no preview element yet. Also not
  done: visual keyframe markers on the timeline clip block itself (properties-panel list editing
  only this pass — add/edit/delete rows, no on-timeline handles). **Verification caveat:**
  rotation/position's `t`/`if`/`between` usage is new territory for this codebase (only `geq`'s
  per-pixel language had been exercised here before) and, like the GPU encoder fix above, this
  dev machine's FFmpeg build can't actually open any encoder right now — so the new
  `animated_*_keyframes_*_without_error` tests (`timeline_export_test.rs`,
  `timeline_export_multi_test.rs`) fail with `EncodeError::Encoder` here the same as every other
  encode-path test, and the filter-graph correctness of rotation/position specifically has not
  been empirically confirmed against a real render on any machine yet.

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

  **GPU encode (done, including the Quick Sync pixel-format fix; hardware success still
  unverified):** `avbridge_encode_timeline_export`/`_multi` take a `gpu_encoder_preference` int
  (`GpuEncoderPreference` in `bridge_internal.h`: AUTO/CPU/NVENC/QUICKSYNC/AMF), threaded from
  a new `Prefs.gpu_encoder` setting (`screens/prefs.rs`'s "Encode por GPU" row) through
  `core::render`'s `render_export_job`/`render_export_job_multi`/`render_timeline_export`.
  `gpu_encoder.c`'s `open_video_encoder()` tries the requested hardware encoder(s) (AUTO tries
  NVENC, then Quick Sync, then AMF) and falls back to the CPU (libopenh264) encoder if
  `avcodec_open2` fails for any reason. **Previously a known gap, now fixed:** every attempt
  used to hardcode `AV_PIX_FMT_YUV420P` regardless of encoder — but `h264_qsv` explicitly
  rejects that format (`"Specified pixel format yuv420p is not supported by the h264_qsv
  encoder"`, wants nv12/qsv instead), which the fallback logic couldn't distinguish from "no
  GPU/driver present", so `GPU_ENCODER_QUICKSYNC` silently fell back to CPU every time even on
  real Quick Sync hardware. `gpu_encoder.c`'s new `pix_fmt_for_encoder_name()` now requests
  `AV_PIX_FMT_NV12` specifically for `h264_qsv` (every other encoder keeps yuv420p, unchanged);
  `open_video_encoder()` reports back via a new `out_pix_fmt` out-param which format the opened
  encoder actually wants, and both `timeline_export.c` and `timeline_export_multi.c` thread that
  into their filter graphs' final `format=...` stage (previously a hardcoded `format=yuv420p`
  literal in three places) so the frames reaching the encoder already match. NVENC/AMF are
  unchanged (still yuv420p) since neither has been observed to reach the pix_fmt check on any
  dev machine so far (no NVIDIA GPU; no `amfrt64.dll`) — whether they'd accept yuv420p directly
  on real hardware remains unverified either way, same as before.
  **Still unverified:** fixing the format mismatch doesn't by itself prove `h264_qsv`
  successfully encodes on real Quick Sync hardware — no machine with that hardware has run this
  code yet. Worse, the current dev machine's Homebrew FFmpeg build doesn't even have
  `libopenh264` or `h264_qsv` compiled in at all (`ffmpeg -encoders` lists neither — only
  `libx264`/`libx264rgb`/`h264_videotoolbox`), so `open_video_encoder()` can't open *any*
  encoder here right now, including the CPU fallback — every test that reaches
  `avcodec_open2`/`avcodec_send_frame` (`encode_test.rs`'s
  `every_gpu_encoder_preference_falls_back_to_a_working_export` and most of the rest of that
  file, plus `timeline_export_multi_test.rs`'s encode-path tests, plus `core`'s
  `generates_a_real_downscaled_proxy`) currently fails with `EncodeError::Encoder` on this
  machine specifically, confirmed identical on `main` before this fix — a pre-existing FFmpeg
  build gap, not something introduced by this change or a regression to fix as part of it.

  Export job reordering is done (move up/down buttons on Queued jobs, `screens/queue.rs`);
  pausing an in-flight render is a deliberate non-goal, not a gap — `ffmpeg` has no notion of
  pausing mid-render (see `pump_export_queue`'s doc comment), so a Rendering job only offers
  Cancel. **Not yet done:** the rest of Fase 4's larger CapCut-parity items (layer templates,
  layer transform, video stabilization, AI background removal, auto-reframe, LUTs,
  text-to-speech, motion tracking, music/SFX library) — keyframes are done, see above.

  **Word-highlight subtitles (done):** true in-place highlighting — the full sentence stays on
  screen, the currently-spoken word lights up in `highlight_color_rgba` exactly where it sits in
  the line — per `request.md`'s "Legenda com destaque de palavra (estilo shorts)". Three new
  pieces: (1) `transcribe()` requests `set_token_timestamps(true)` and `collect_words()` groups
  whisper.cpp's per-token timestamps into per-word ones (a word can be several tokens, e.g.
  "run"+"ning"; whisper.cpp marks a new word by prefixing its first token's text with a space).
  (2) `core::text_metrics` (new module, pure-Rust `fontdue`, no C toolchain) measures each
  word's pixel advance width at the same font `avbridge_apply_text_overlays`'s `drawtext`
  renders with, to position highlight overlays precisely on top of the plain word underneath —
  verified against a real `drawtext` render before trusting it (fontdue's advance width and
  FreeType's ink bounding box differed by exactly the expected right-side-bearing amount, not
  approximately). (3) `resolve_text_segments` (now takes `canvas_width` to convert pixel
  offsets to the `0.0..=1.0` fraction `TextSegment::pos_x` expects) expands a highlighted
  `TextClip` into a base segment (full sentence, full duration) plus one short segment per word
  (just that word, in the highlight color, only for its own speaking window) — reuses the
  existing multi-`drawtext` overlay pipeline unchanged, no C changes needed. Known limitation:
  single-line only — a caption long enough to wrap in `drawtext` gets every highlight positioned
  as if still on one line, landing wrong past the wrap point.

  **Whisper subtitles (done, one known limitation):** `avcore::transcribe::transcribe()`
  decodes a source's audio to 16kHz mono float PCM via a new `avbridge` function
  (`extract_pcm_16k_mono` / `pcm_extract.c`, same no-subprocess FFI approach as every other
  avbridge call) and runs it through a local Whisper model (`whisper-rs`, CPU-only default
  features) to produce timestamped segments. The Mídia screen's "Transcrever" button runs it in
  the background (`ui/src/app/transcribe.rs`) and drops the result as `TextClip`s onto the
  active sequence's Text track, anchored at the playhead. The model itself isn't bundled with
  the installer yet (Fase 8's "Modelo do Whisper... incluídos no instalador" is unbuilt) — but
  Preferences can fetch one on demand now: `avcore::model_download` (new, `ureq` with rustls,
  no C toolchain) streams a tiny/base/small GGML file from huggingface.co/ggerganov/whisper.cpp
  straight to `<prefs dir>/models/`, `ui/src/app/model_download.rs` runs it on a background
  thread (progress bar + cancel in Preferences, same spawn/channel/pump shape as
  `spawn_transcribe`), and `prefs.whisper_model_path` is set automatically on completion — no
  separate "now go find the file" step. Manual path entry (browse to an already-downloaded
  model) still works too, for an offline setup or a custom model.
  **Fixed a real upstream bug rather than working around it:** `whisper-rs` 0.16.0's
  `set_abort_callback_safe` has broken trampoline codegen (casts its double-boxed
  `Box<dyn FnMut() -> bool>` back to the *original* closure type instead of the box type,
  unlike `set_progress_callback_safe`'s correct version) — confirmed by empirical reproduction
  (any capturing closure, even a plain `bool`, made whisper.cpp abort on the first internal
  check regardless of what it returned). `transcribe()` bypasses only that one broken method,
  using `FullParams`'s raw `unsafe fn set_abort_callback`/`set_abort_callback_user_data` with
  its own correctly-typed trampoline (`transcribe.rs`'s `abort_trampoline`, doc comment has the
  full trace) — not a fork of the crate, just the same double-box construction paired with a
  matching generic. Mid-run cancellation works end-to-end (`TranscribeOutcome::Cancelled`
  carries whatever segments were already committed before the abort); whisper.cpp's return
  code doesn't distinguish "aborted via callback" from a genuine encode/decode failure, so
  `transcribe()` checks `cancel` itself after `state.full()` returns rather than trusting
  which `Result` variant came back once cancellation was requested.

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
