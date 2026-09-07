# Project Instructions

`oca` — a native Rust desktop video editor (egui/eframe) for cutting gameplay footage. Headline
features: automatic loudness normalization and export-that-matches-the-source-bitrate. This file
adapts the generic `.claude/` package to this repository's actual architecture — see
`E:\github\oca\CLAUDE.md` (repo root) for the authoritative build/test/env instructions; this
file supplements it with UI-specific workflow guidance.

## Real workspace structure

4-crate workspace, strict one-way dependency `avbridge -> core -> ui`, plus an isolated
`ytbridge`:

- **`avbridge`** — thin hand-written FFI wrapper (`src/lib.rs`, ~1700 lines) over
  `csrc/*.c` (`export.c`, `filters.c`, `probe.c`, `proxy.c`, `timeline_export.c`,
  `timeline_export_multi.c`, `audio_mix.c`, `loudness.c`, `waveform.c`, `text_overlay.c`,
  `shape_overlay.c`, `matte_encode.c`, `gpu_encoder.c`), which call libavformat/libavcodec/
  libavfilter/libavutil directly. `build.rs` locates FFmpeg via `FFMPEG_DIR` and renames
  Windows import libs to avoid a link collision with GStreamer's bundled `gst-libav`.
- **`core`** — package name `core`, **`[lib] name = "avcore"`** (Rust code imports it as
  `avcore`, never `core`, since that would shadow libcore). ~35 top-level modules: domain
  (`project.rs`, `timeline.rs`, `undo.rs`), persistence (`persistence.rs` — `.ocproj`/`.ocqueue`/
  `.octr` framing), media (`probe.rs`, `media.rs`), GStreamer real-time preview (`preview.rs`,
  `preview_effects.rs`), FFmpeg export rendering (`render.rs`, `export.rs`, delegating to
  `avbridge`), plus one module per AI/analysis feature (`transcribe.rs`/`transcript.rs`,
  `scene_detection.rs`, `background_removal.rs`, `motion_tracking.rs`, `auto_reframe.rs`,
  `highlight_detection.rs`, `silence_detection.rs`, `nested_sequence.rs`, etc). Depends on
  `gstreamer`/`gstreamer-app`/`gstreamer-video`, `whisper-rs` (CPU), `ort` (ONNX Runtime),
  `espeak-rs` (TTS), `rmp-serde`+`flate2` (MessagePack+gzip persistence).
- **`ui`** — `eframe` 0.36 (glow backend, `default-features=false` — wgpu/dx12 avoided due to a
  windows-rs conflict), `egui` 0.36, `tokio` with **only the `sync` feature** (channel
  primitives, not a task runtime — background work is plain `std::thread::spawn`, see
  `engineering/ARCHITECTURE.md`).
- **`ytbridge`** — standalone binary, deliberately isolated from `core`/`ui`'s dependency graph;
  vendors its own Python runtime via `pyo3` for yt-dlp. Not part of the UI/editor dependency
  chain — do not couple it to `core`/`ui` internals.

## Real build/test/quality commands

Use `make build`/`make run`/`make test`/`make test-core`/`make lint`/`make fmt`/`make check`
exactly as documented in the repo-root `CLAUDE.md` — do not invent alternative cargo invocations.
Requires `FFMPEG_DIR`, `PKG_CONFIG_PATH` (GStreamer pkg-config dir), and for `whisper-rs`
`LIBCLANG_PATH` set per-shell (not persistent on most dev machines — see repo `CLAUDE.md`'s
"Commands" section and, if present, this machine's own build-env memory notes).

## Before changing UI

Read the relevant files — now pointing at real content, not placeholders:

- Product/domain changes: `domain/VIDEO_EDITOR.md`
- Timeline changes: `domain/TIMELINE.md` and `engineering/TIMELINE_PERFORMANCE.md`
- Workflow changes: `ux/WORKFLOWS.md`
- UI changes: `design/DESIGN_SYSTEM.md`, `design/PROFESSIONAL_UI.md`, `engineering/EGUI.md`
- Architecture changes: `engineering/ARCHITECTURE.md`
- Quality gates before calling a change done: `engineering/QUALITY.md`

Do not implement a feature from a vague request without identifying:
- the user goal;
- the active object (a `ClipInstance`? a `TextClip`/`ShapeClip`? a `Marker`? the whole
  `Sequence`?);
- the current context (`App::screen`, `App::tool`);
- the primary interaction;
- the expected state transition;
- undo/redo semantics — remember undo here is **full-`Sequence`-snapshot**, not per-field
  commands (see `engineering/ARCHITECTURE.md`);
- performance implications, especially inside `timeline_panel/` or anything called from
  `pump_preview_frame`.

## Design before code

For non-trivial UI changes, first produce a short internal plan covering:
1. user workflow;
2. affected objects and state (which `App` sub-struct, e.g. `PreviewState`/`ImportState`, and
   whether anything needs a new field on `Project`/`Timeline`/`ClipInstance`);
3. layout and interaction — does it belong in `screens/editor/timeline_panel/` (custom-painted),
   a `screens/` module (standard egui layout), or a new `app/<feature>.rs` (state + `impl App`
   business logic, separate from drawing);
4. architecture — reuse existing primitives (`Track::ripple_delete_range`,
   `push_undo_snapshot`/`push_undo_snapshot_for_drag`, the `pump_*`/channel pattern) before
   inventing new ones;
5. performance risks — see `engineering/TIMELINE_PERFORMANCE.md`'s "known gap" section before
   assuming per-frame cost is fine;
6. validation — which tests, which quality gates.

Then implement.

## Non-negotiable UI principles

- Do not design a generic SaaS dashboard.
- Do not overuse cards, rounded containers, shadows, borders, or decorative gradients.
- The editing workspace is more important than application chrome.
- Prefer direct manipulation over modal workflows — though this codebase does use `egui::Modal`
  for batch-review flows (silence review, transcript-proposal review) where a reviewable list of
  automatic suggestions genuinely needs one; that is a deliberate, narrow exception, not a
  default pattern to reach for.
- Use progressive disclosure.
- Keep contextual tools contextual — the properties panel (`screens/editor/properties_panel/`)
  is already context-aware on the selected clip type; extend that, don't bypass it.
- Preserve muscle memory and keyboard efficiency — most shortcuts are handled inline in
  `screens/editor/mod.rs::show` before delegating to submodules; check there first.
- Do not invent a new visual pattern when a project component already exists
  (`components/{frame,section,property,combo,tag}.rs`, `theme.rs` tokens).
- Do not expose every feature permanently.

## Rust principles

- Avoid unnecessary clones and allocations in hot paths — `pump_preview_frame`'s call chain
  already has one deliberate optimization (`current_preview_clip_lut_and_vignette` cloning only a
  `String` instead of the full `ClipInstance`+`MediaAsset`); match that discipline in new
  per-frame code rather than reaching for `.clone()` by default.
- Do not block the UI thread — dispatch anything FFmpeg/GStreamer/Whisper/network-bound through
  the established `std::thread::spawn` + `tokio::sync::mpsc::UnboundedSender/Receiver` +
  per-frame `pump_<feature>()` pattern (see `engineering/ARCHITECTURE.md`), not `tokio::spawn`
  (the runtime isn't present — only `tokio`'s `sync` feature is enabled).
- Separate persistent project state (`Project`/`Sequence`/`Timeline`/`ClipInstance` in `avcore`)
  from transient UI state (`App` and its sub-structs in `ui`) — selection
  (`selected_clip_id`/`multi_selected_clip_ids`) is a confirmed real example: it lives on `App`,
  never serialized.
- Use `App::push_undo_snapshot`/`push_undo_snapshot_for_drag` for undoable edits — one call per
  user-visible commit, coalesced for continuous drags, never per-frame.
- Keep rendering, interaction, and domain logic separate — the timeline panel's own convention
  (draw functions in `draw.rs` taking `(painter, rect, data, px_per_sec)` and returning an
  optional interaction result, called from `mod.rs`'s per-frame loop) is the pattern to extend,
  not a bespoke one.
- Prefer small cohesive modules over giant files — `app/` already has ~30 feature-scoped files;
  add a new one for a new feature rather than growing `app/mod.rs` further (it's already 2100+
  lines and its own doc comment records a prior "122 flat fields" cleanup as a cautionary note).
- Run formatting, linting and relevant tests after changes (`make fmt`, `make lint`,
  `make test-core`/`cargo test -p ui`).

## Done criteria

A feature is not done until:
- behavior is correct;
- UI states are handled;
- keyboard/mouse interaction is coherent;
- undo/redo is considered where applicable (remember: full-sequence snapshot, not per-field);
- performance impact is assessed, especially anything touching `timeline_panel/` — horizontal
  viewport culling now skips off-screen clips (see `engineering/TIMELINE_PERFORMANCE.md`), but
  new per-clip work added *before* that culling check, or work that runs for every clip
  regardless of the cull (e.g. building data the culled clips still need), still pays full
  per-clip cost every frame;
- tests are added or updated where appropriate (`ui`'s unit tests live in
  `src/<module>/<module>_test.rs`, `core`'s integration tests in `crates/core/tests/`);
- no obvious visual regressions remain.
