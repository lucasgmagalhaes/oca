# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

oca — a native Rust video editor (`egui`/`eframe`) for cutting gameplay
footage for the PacoPaçoca YouTube channel. Its two headline features are automatic loudness
normalization and export-that-matches-the-source-bitrate. Full phased execution plan:
[`features/request.md`](features/request.md) (a near-duplicate lives at `docs/plano.md` —
not yet reconciled, treat `features/request.md` as canonical).

Current status: the GUI shell (all five screens, navigable) and JSON project save/load are
wired end-to-end from the UI (`home.rs` open dialog, `editor.rs` save). Probing
(`avcore::probe`), export rendering (`avcore::render`), loudness measurement
(`avcore::loudness`), and proxy generation (`avcore::proxy`) all go through
`avbridge`, a native FFI bridge over libavformat/libavcodec/libavfilter/libswscale — no
subprocess, no ffprobe/ffmpeg on PATH required for any of them (proxy uses `libopenh264` — BSD
— since this LGPL FFmpeg build has no `libx264`/GPL). The GStreamer preview pipeline
(`avcore::preview::Preview`) is wired into the Editor screen's preview panel (`ui/src/
app.rs::OcaApp::{select_asset,reload_preview,pump_preview_frame}` +
`ui/src/screens/editor.rs::preview_panel`): clicking an asset in the media library opens it,
decoded frames are uploaded to an egui texture every frame, and play/pause/seek (including a
click-to-seek position slider) drive the pipeline — playback of the selected clip only, not
yet full multi-clip timeline playback (the timeline itself is still static placeholder rects,
no real cut/split/trim). A background export queue worker already runs (`OcaApp::
pump_export_queue` dispatches `avcore::render_export` on a spawned thread, progress/done/
failed/cancelled reported back over `tokio::mpsc`) — the queue panel doesn't yet support
reordering/pausing jobs or persisting the queue across sessions. Not yet implemented: real
timeline editing (cut/split/trim), the custom timeline widget (thumbnails/waveform), and queue
reorder/pause/persistence. Check the plan doc for which phase a task belongs to before
assuming a feature is live.

## Commands

Requires Rust (stable) via rustup. Both the MSVC target (`stable-x86_64-pc-windows-msvc`, needs
Visual Studio Build Tools with the "Desktop development with C++" workload) and the GNU target
(`x86_64-pc-windows-gnu` + a MinGW-w64 toolchain e.g. WinLibs, `bin` dir on `PATH`) work — this
workspace currently defaults to MSVC, but nothing in it requires MSVC specifically.

`avbridge` links against libavformat/libavcodec/libavfilter/libswscale/libavutil and needs
`FFMPEG_DIR` set to an FFmpeg dev build with `include/` and `lib/` subdirectories (e.g. a BtbN
shared build, with `avfilter`/`swscale`/`libopenh264` among the linked libs/enabled encoders)
to compile at all — the workspace won't build without it. At runtime the FFmpeg DLLs
(`FFMPEG_DIR/bin`) need to be next to the built binary or on `PATH`.

`core` also depends on the `gstreamer` crate (`preview` module, see Architecture below),
needing `pkg-config` plus GStreamer's dev build (MSVC or mingw,
matching whichever Rust target you're using) discoverable via
`PKG_CONFIG_PATH=<gstreamer_root>/lib/pkgconfig`, with `<gstreamer_root>/bin` on `PATH` too
(build-time lookup and runtime DLL resolution both need it).

**Do not remove `avbridge/build.rs`'s import-lib-renaming step.** GStreamer's SDK bundles
its own FFmpeg build (`gst-libav`) whose import libs share filenames with (but are a different,
incompatible version from) the ones under `FFMPEG_DIR` — confirmed on both MSVC and GNU/mingw,
and confirmed to be a filename collision, not a toolchain-ABI issue (ruled out by testing both).
Left as bare `-l<name>` + `-L<path>`, a bare-name search can silently resolve to GStreamer's
bundled copy instead, compiling fine but corrupting `avbridge`'s FFmpeg calls at runtime
(`avformat_open_input` starts failing on every call, no error at build time). `build.rs` copies
its FFmpeg import libs into its own `OUT_DIR` under unique names before linking specifically to
make that impossible — full diagnosis in `features/fase1/commit_plan.md` (chore-002).

```bash
make build     # debug build, whole workspace
make run       # run the GUI app (debug)
make test      # run every crate's test suite
make test-core # cargo test -p core only
make bench     # criterion benchmarks (core parsing/serialization) -> target/criterion/report/index.html
make fmt       # cargo fmt --all
make lint      # cargo clippy --workspace --all-targets
make check     # cargo check --workspace --all-targets (fast compile-only loop)
```

No `make` on PATH → run the underlying `cargo`/`rustup` command directly; every Makefile target
is a one-liner (see [`Makefile`](Makefile)).

Single test: `cargo test -p core --test probe_test measure_loudness` (integration tests, one
`<module>_test.rs` file per module under `crates/core/tests/`) or `cargo test -p ui i18n::tests`
for the `src/<module>/<module>_test.rs` unit tests in `ui`.

## Architecture

Three-crate split, enforced by dependency direction: `avbridge` has no dependents within
the workspace besides `core`; `core` is UI-agnostic (no `egui` dependency at all)
and `ui` is its only consumer.

- **`avbridge`** — thin C bridge (`csrc/bridge.c`) over libavformat/libavcodec/libavfilter/
  libavutil, called from Rust via FFI (`src/lib.rs`). Written for this project, not an
  auto-generated binding of the full FFmpeg API — keeps the C surface small and auditable.
  `build.rs` locates FFmpeg via `FFMPEG_DIR` (see Commands above). Exposes probing
  (`avbridge_probe`), export rendering (`avbridge_encode_export`: video
  passthrough-copied, audio decoded → loudnorm+limiter filter graph → AAC re-encoded, with
  progress callback and cooperative cancellation), loudness-only measurement
  (`avbridge_measure_loudness`, same decode→loudnorm shape without the encoder — its
  report has no queryable struct API, only `av_log` output during filter-graph teardown, so
  this installs a process-global log callback for the call's duration; **not thread-safe**,
  documented on the function), and proxy generation (`avbridge_generate_proxy`: video
  decoded → libswscale downscale (aspect-preserving) → `libopenh264` re-encode, audio decoded
  → format-matched → AAC re-encode).
- **`core`** — project/timeline/media data model plus the media wrappers that populate
  it (`probe`, `render`, `loudness`, and `proxy` — all via `avbridge` FFI, no subprocess
  left in any of them), a `playbin`-based GStreamer playback pipeline (`preview`: open/play/pause/seek/
  query, `current_frame()` pulls packed RGBA via an appsink), JSON save/load (`persistence`),
  and mock sample data (`sample`) used to exercise the UI before real files
  are wired in. Locale-neutral by design: it stores data like `Recency` (an enum), never
  pre-formatted display strings — formatting is `ui`'s job.
- **`ui`** — the eframe/egui GUI (glow/OpenGL backend): `app.rs` holds all top-level
  state (`OcaApp`, which screen is active, loaded projects, export jobs) and mutation methods
  (`open_project`, `queue_export`, etc.); `screens/` has one module per of the five screens
  (home, library, editor, queue, prefs) plus shared `widgets`; `theme.rs` is the dark/teal
  palette; **all UI strings live in `i18n.rs`** (pt-BR and English) — never hardcode display
  text in a screen module, add a `Text` variant instead.

`docs/CODE_GRAPH.md` (module dependency graph + public-API index) used to be regenerated by an
`xtask` crate (`cargo run -p xtask -- graph` / `make graph`); that crate was dropped and the
Makefile/docs referencing it are stale (`make graph`, `make test-xtask` — not fixed as part of
this pass). Use `graphify` instead — see the `## graphify` section below.

Test placement follows what's reachable: `core` has a `[lib]` target, so its tests are
real integration tests in `crates/core/tests/` (one `<module>_test.rs` file per module)
exercising only the public API, plus `crates/core/benches/` for criterion benchmarks. A
handful of tests that need a private helper unreachable from `tests/` stay as
`src/<module>/<module>_test.rs` unit tests instead (wired in via `#[path = "..."] mod
tests;`, since the module is still named `tests`). `ui` is a bin-only crate (no `[lib]`), so
all its tests are `src/<module>/<module>_test.rs` unit tests. `avbridge` has a `[lib]`
target, so its FFI tests live in `crates/avbridge/tests/` (also `<module>_test.rs`) against
small checked-in media fixtures.

Planned architecture (not yet implemented, see the plan doc for phases): extracting `preview`'s
decoded frames into an egui texture and wiring that into the Editor screen's playhead, and a
background export queue where rendering runs on a worker communicating over `tokio::mpsc` so
the editing UI never blocks on an in-progress export — each queued job snapshots its render
config at enqueue time, so later edits to the active project don't affect jobs already in the
queue.

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

This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
