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
- **Fase 4 (in progress):** export is now timeline-aware — "Add Export" renders the active
  sequence's video track (in `start_secs` order) instead of one raw asset
  (`core::render::render_timeline_export`/`render_export_job`, `avbridge_encode_timeline_export`
  in `bridge.c`, a real video `AVFilterGraph` decode→filter→libopenh264-encode pipeline, unlike
  the older passthrough-copy `render_export`/`avbridge_encode_export` which still exists and is
  otherwise unused). `ClipInstance::video_filter_chain()` builds each clip's own avfilter chain
  from the subset of effect fields expressible as a static per-clip filter — **gain_db** (as a
  runtime-adjustable `volume` stage ahead of the shared loudnorm/limiter graph), **crop_x/y/w/h**,
  **flipped_h**, **color_filter** (black-and-white/sepia), **vignette_intensity**,
  **brightness/contrast/saturation**, **sharpen**, **chroma_key_enabled/color/tolerance**, and
  **blur_intensity** — all now audible/visible in the exported file, not just the properties
  panel and timeline badges. **Chroma key is the one exception**: `colorkey` marks matching
  pixels transparent, but the final `format=yuv420p` conform stage drops that alpha plane with
  nothing composited underneath (no multi-track layering yet), so the keyed color is still
  there in the rendered file — the checkbox/picker have no visible export effect despite being
  in `video_filter_chain()`'s output, same as if it weren't wired at all.

  **freeze frame** (`ClipInstance::frozen`) is also now wired into export, but through a
  different mechanism than the filter chain above: `resolve_timeline_segments` passes it
  straight through to `avbridge::ClipSegment::frozen`, and the timeline decode loop in
  `avbridge_encode_timeline_export` (`bridge.c`) special-cases it — instead of decoding the
  clip's whole trimmed range, it decodes just the first frame at/after `source_in_secs` and
  synthesizes duplicate pushes of it (spaced at `canvas_fps`) through the same per-clip filter
  chain to fill the block's full on-timeline duration. Audio is unaffected — still decoded
  across the clip's whole trimmed range regardless of `frozen`.

  Preview is now timeline-aware too, not just export: `avcore::preview::Preview` follows the
  active sequence's timeline playhead (`Track::clip_at`) instead of the media-library
  selection, reopening its `playbin` pipeline whenever the playhead crosses onto a different
  clip (`OcaApp::ensure_preview_loaded`/`pump_preview_frame`/`seek_preview` in `ui/src/app.rs`)
  and applying that clip's effects via a `gst::Bin` on `playbin`'s `video-filter` property
  (`build_video_filter_bin` in `preview.rs`). Covers **crop**, **flipped_h**,
  **brightness/contrast/saturation**, **color_filter** (black-and-white via forced saturation,
  sepia via `coloreffects`), and **blur_intensity**/**sharpen** (one `gaussianblur` element,
  signed sigma) — a narrower subset than export's, live-verified against this machine's
  `gstreamer-msvc` install via `gst-inspect-1.0`. **Not covered by preview**: **vignette** (no
  matching GStreamer element found), **chroma key** (same compositing gap as export, see
  above), **gain_db** (preview has no audio route at all — `audio-sink` is `fakesink`,
  deliberately, a separate pre-existing gap), and now **freeze frame** too — export's hold
  mechanism lives in `avbridge_encode_timeline_export`'s decode loop, which preview's
  `playbin`-based pipeline never goes through, so it needs its own separate mechanism (e.g.
  seeking-and-pausing at the held timestamp) that hasn't been built. Crossing a clip boundary during playback tears
  down and reopens the pipeline (a brief hitch at every cut) rather than gapless — a real
  compositor pipeline would be needed to avoid that, out of scope here. There is no more
  "preview a raw asset before placing it on the timeline" mode — selecting a media-library
  asset (`selected_asset_id`) no longer affects preview at all, only what covers the playhead
  does.

  Multiple video tracks / layered compositing, audio-only tracks, and an exact source-bitrate
  export match (the export canvas's bitrate is now a duration-weighted average of the
  timeline's own clips' source bitrates, not an exact copy, since video is re-encoded rather
  than stream-copied) are out of scope for both passes above.
  **Not yet covered by export or preview at all**: speed
  (`ClipInstance::speed_factor`, needs resampling + the timeline supporting a displayed
  duration different from the trimmed source range, which nothing does yet), layer masks
  (`ClipInstance::mask_shape`: none/circle/
  rounded-rect, needs alpha-geometry compositing), shake/glitch/pixelize
  (`ClipInstance::shake_intensity/glitch_intensity/pixelize_intensity`), transitions
  (`ClipInstance::transition_in`: none/fade/hard cut/slide/zoom — also models only a block's
  incoming edge, not a real two-clip cross-blend, which would need a relationship between
  adjacent clips instead of a single-clip field), and zoom (`ClipInstance::zoom_start`/
  `zoom_end`, needs keyframed crop-over-time). Copy formatting (`Ctrl+Shift+C`/`Ctrl+Shift+V`,
  `OcaApp::copy_selected_clip_formatting`/`paste_selected_clip_formatting`) copies all of the
  above between blocks without duplicating the clip, regardless of which ones render yet. Every
  field still has its properties-panel control and timeline badge/tint/stroke (freeze top-left,
  speed top-right, crop bottom-right, mask bottom-left, flip "⇄" top-center, color filter as a
  translucent tint, vignette as a darkened border stroke, chroma key as a "🟩" badge
  bottom-center) regardless of render status. Check `features/request.md` for what's still
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
  playback pipeline (`preview`), and JSON save/load (`persistence`). Locale-neutral: stores data
  like `Recency` (an enum), never pre-formatted strings — formatting is `ui`'s job. The app
  starts with an empty project list; every project, asset, and export job comes from the user
  ("Novo projeto"/"Abrir projeto" and real imports) — there is no mock/sample data anywhere in
  the codebase.
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
