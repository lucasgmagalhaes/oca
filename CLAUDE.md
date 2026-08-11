# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

oca — a native Rust video editor (`egui`/`eframe`) for cutting gameplay footage for the
PacoPaçoca YouTube channel. Headline features: automatic loudness normalization and
export-that-matches-the-source-bitrate. Full phased plan: [`features/request.md`](features/request.md)
(canonical — `docs/plano.md` is a stale near-duplicate).

## Status

- **Fase 1-3 (done):** probing/export/loudness/proxy all via `avbridge` FFI (no ffmpeg
  subprocess). GStreamer preview pipeline. Full timeline editing — select, split, trim,
  drag-move, copy/cut/paste, context menu, composite blocks, multi-sequence tabs, resizable
  panels, per-position filmstrip thumbnails, waveforms. JSON project save/load. Background
  import (probe first, loudness/proxy/waveform enrich in place after) and background export
  queue.
- **Fase 4 (in progress):** per-block volume gain (`ClipInstance::gain_db`) and freeze frame
  (`ClipInstance::frozen`) — both live in the properties panel and affect the timeline display
  (waveform scaling / poster-frame draw) but not preview playback or export yet, since export
  still passthrough-renders one source file per job rather than mixing the actual timeline.
  Speed (`ClipInstance::speed_factor`) follows the same pattern — a properties-panel slider and
  a "2.00x"-style badge on the timeline block, but doesn't resample audio or change the block's
  on-timeline length yet (that needs the timeline to support a block whose displayed duration
  differs from its trimmed source range, which nothing does yet). Crop
  (`ClipInstance::crop_x/y/w/h`, a normalized `0.0..=1.0` sub-rectangle of the frame, properties
  panel drag values gated to video clips) is the same shape again — a "⛶" badge on the timeline
  block, no preview/export effect yet. Copy formatting (`Ctrl+Shift+C`/`Ctrl+Shift+V`,
  `OcaApp::copy_selected_clip_formatting`/`paste_selected_clip_formatting`) copies gain/freeze/
  speed/crop/mask/flip/color-filter between blocks without duplicating the clip. Layer masks
  (`ClipInstance::mask_shape`: none/circle/rounded-rect, `mask_corner_radius` for the latter) are
  the same shape too — a shape picker plus corner-radius slider in the properties panel, a "●"/
  "▢" badge on the timeline block. Horizontal flip (`ClipInstance::flipped_h`, video-only
  checkbox) rounds out the timeline block's badge corners (freeze top-left, speed top-right,
  crop bottom-right, mask bottom-left, flip "⇄" top-center). Color filter
  (`ClipInstance::color_filter`: none/black-and-white/sepia, a bounded subset of the eventual
  "Filtros de cor e LUTs" library) is shown as a translucent tint over the timeline block instead
  of a badge, since the corners are taken. Vignette (`ClipInstance::vignette_intensity`,
  `0.0..=1.0`) is a darkened border stroke scaled by intensity, same idea. Brightness/contrast/
  saturation (`ClipInstance::brightness/contrast/saturation`) and sharpen
  (`ClipInstance::sharpen`) round out that "Efeitos visuais" group — four properties-panel
  sliders with no visible effect anywhere yet, not even a timeline cue. Chroma key
  (`ClipInstance::chroma_key_enabled`/`chroma_key_color`/`chroma_key_tolerance`) is a checkbox +
  color picker + tolerance slider, with a "🟩" badge (bottom-center) on the timeline block. Same
  preview/export gap as the rest for all of these. Check `features/request.md` for what's still
  unbuilt before assuming a feature is live — when in doubt, `graphify query`.

## Commands

Requires Rust (stable) via rustup, either the MSVC or GNU target.

`avbridge` needs `FFMPEG_DIR` set to an FFmpeg dev build (`include/`+`lib/`, e.g. a BtbN shared
build with `avfilter`/`swscale`/`libopenh264`) to compile at all; its DLLs (`FFMPEG_DIR/bin`)
must be on `PATH` at runtime. `core`'s `preview` module needs GStreamer's dev build discoverable
via `PKG_CONFIG_PATH=<gstreamer_root>/lib/pkgconfig`, with `<gstreamer_root>/bin` on `PATH` too.

**Do not remove `avbridge/build.rs`'s import-lib-renaming step.** GStreamer's SDK bundles its
own FFmpeg build (`gst-libav`) whose import libs share filenames with (but are incompatible
versions of) the ones under `FFMPEG_DIR`. Left as a bare `-l<name>`, linking can silently
resolve to GStreamer's copy instead — compiles fine, but corrupts `avbridge`'s FFmpeg calls at
runtime with no build-time error. `build.rs` copies its import libs into its own `OUT_DIR` under
unique names specifically to prevent this — full diagnosis in `features/fase1/commit_plan.md`
(chore-002).

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

No `make` on PATH → run the underlying `cargo`/`rustup` command directly (Makefile targets are
one-liners, see [`Makefile`](Makefile)).

Single test: `cargo test -p core --test probe_test measure_loudness` (core's integration tests
live one `<module>_test.rs` file per module under `crates/core/tests/`) or
`cargo test -p ui i18n::tests` for `ui`'s `src/<module>/<module>_test.rs` unit tests.

**End-to-end tests** (`e2e/`) drive the real built `ui.exe` via Windows UI Automation
(`pytest` + `pywinauto`, `backend="uia"`) — this is what actually proves a change works in the
running app, not just that it compiles. Works via `eframe`'s `accesskit` feature, which gives
egui's own widgets an accessible name for free; hand-painted controls need
`response.widget_info(...)` added explicitly to be reachable (done for the nav rail; timeline
clips and media library items aren't yet, so they need coordinate-based calls for now).
One-time setup: `python -m pip install -r e2e/requirements.txt`. Needs a debug build
(`make build`) first and the FFmpeg/GStreamer runtime DLL dirs on `PATH` for the launched
process — `make test-e2e` exports both automatically. The `oca_window` fixture
(`e2e/conftest.py`) calls `set_focus()` before yielding; without it, synthetic clicks land on
whatever window has focus instead (silently does nothing, no exception).

## Architecture

Three-crate split, enforced by dependency direction: `avbridge` has no dependents besides
`core`; `core` is UI-agnostic (no `egui` dependency) and `ui` is its only consumer.

- **`avbridge`** — thin, hand-written C bridge (`csrc/bridge.c`) over
  libavformat/libavcodec/libavfilter/libavutil, called via FFI (`src/lib.rs`). Exposes probing,
  export rendering (video passthrough-copied, audio loudnorm+limiter → AAC re-encoded),
  loudness-only measurement, proxy generation (libswscale downscale → `libopenh264`), and
  waveform peak extraction.
- **`core`** — project/timeline/media data model plus the wrappers that populate it (`probe`,
  `render`, `loudness`, `proxy`, `waveform`, all via `avbridge`), a `playbin`-based GStreamer
  playback pipeline (`preview`), JSON save/load (`persistence`), and mock sample data
  (`sample`). Locale-neutral: stores data like `Recency` (an enum), never pre-formatted
  strings — formatting is `ui`'s job.
- **`ui`** — the eframe/egui GUI (glow/OpenGL backend): `app.rs` holds top-level state
  (`OcaApp`) and mutation methods; `screens/` has one module per screen (home, library, editor,
  queue, prefs) plus shared `widgets`; `theme.rs` is the dark/teal palette; **all UI strings
  live in `i18n.rs`** (pt-BR and English) — never hardcode display text, add a `Text` variant.

Test placement follows what's reachable: `core` and `avbridge` have `[lib]` targets, so their
tests are real integration tests in `crates/<crate>/tests/` (one `<module>_test.rs` per module),
plus `crates/core/benches/` for criterion benchmarks. `ui` is bin-only, so all its tests are
`src/<module>/<module>_test.rs` unit tests. A handful of `core` tests that need a private helper
unreachable from `tests/` stay as `src/<module>/<module>_test.rs` unit tests too.

## Approach

- Read existing files before writing. Don't re-read unless changed.
- Thorough in reasoning, concise in output.
- Skip files over 100KB unless required.
- No sycophantic openers or closing fluff.
- No emojis or em-dashes.
- Do not guess APIs, versions, flags, commit SHAs, or package names. Verify by reading code or docs before asserting.
- Write all code and commit messages in English.
- Commit using Conventional Commits format (`feat:`, `fix:`, `refactor:`, `docs:`, `test:`, `chore:`, etc.), always in English.

## graphify

This project has a knowledge graph at `graphify-out/` with god nodes, community structure, and
cross-file relationships.

- For codebase questions, first run `graphify query "<question>"` when `graphify-out/graph.json`
  exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"`
  for focused concepts — these return a scoped subgraph, usually much smaller than
  `GRAPH_REPORT.md` or raw grep output.
- If `graphify-out/wiki/index.md` exists, use it for broad navigation instead of raw source
  browsing.
- Read `graphify-out/GRAPH_REPORT.md` only for broad architecture review or when
  query/path/explain don't surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API
  cost).
