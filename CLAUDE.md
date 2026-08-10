# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

oca (formerly NivelaEditor) — a native Rust video editor (`egui`/`eframe`) for cutting gameplay
footage for the PacoPaçoca YouTube channel. Its two headline features are automatic loudness
normalization and export-that-matches-the-source-bitrate. Full phased execution plan:
[`features/request.md`](features/request.md) (a near-duplicate lives at `docs/plano.md` —
not yet reconciled, treat `features/request.md` as canonical).

Current status: the GUI shell (all five screens, navigable) and JSON project save/load are
wired end-to-end from the UI (`home.rs` open dialog, `editor.rs` save). Probing
(`nivela_core::probe`) and export rendering (`nivela_core::render`) both go through
`oca-avbridge`, a native FFI bridge over libavformat/libavcodec/libavfilter — no subprocess, no
ffprobe/ffmpeg on PATH required for either. `loudness` and `proxy` still spawn `ffmpeg` as a
subprocess and are next in line to move to the same FFI bridge (see Fase 1 in the plan doc).
Not yet implemented: the GStreamer/MLT decode-and-preview pipeline, real timeline editing
(cut/split/trim), and the background export queue worker. Check the plan doc for which phase a
task belongs to before assuming a feature is live.

## Commands

Requires Rust (stable) via rustup. Both the MSVC target (`stable-x86_64-pc-windows-msvc`, needs
Visual Studio Build Tools with the "Desktop development with C++" workload) and the GNU target
(`x86_64-pc-windows-gnu` + a MinGW-w64 toolchain e.g. WinLibs, `bin` dir on `PATH`) work — this
workspace currently defaults to MSVC, but nothing in it requires MSVC specifically.

`oca-avbridge` links against libavformat/libavcodec/libavfilter/libavutil and needs `FFMPEG_DIR`
set to an FFmpeg dev build with `include/` and `lib/` subdirectories (e.g. a BtbN shared build,
with `avfilter` among the linked libs) to compile at all — the workspace won't build without
it. At runtime the FFmpeg DLLs (`FFMPEG_DIR/bin`) need to be next to the built binary or on
`PATH`. `loudness`/`proxy` still spawn `ffmpeg` as a subprocess (not yet moved to
`oca-avbridge`) — the app falls back to mock data for those without `ffmpeg` on `PATH`.

`nivela-core` also depends on the `gstreamer` crate (prep for the preview pipeline — not built
yet, see Architecture below), needing `pkg-config` plus GStreamer's dev build (MSVC or mingw,
matching whichever Rust target you're using) discoverable via
`PKG_CONFIG_PATH=<gstreamer_root>/lib/pkgconfig`, with `<gstreamer_root>/bin` on `PATH` too
(build-time lookup and runtime DLL resolution both need it).

**Do not remove `oca-avbridge/build.rs`'s import-lib-renaming step.** GStreamer's SDK bundles
its own FFmpeg build (`gst-libav`) whose import libs share filenames with (but are a different,
incompatible version from) the ones under `FFMPEG_DIR` — confirmed on both MSVC and GNU/mingw,
and confirmed to be a filename collision, not a toolchain-ABI issue (ruled out by testing both).
Left as bare `-l<name>` + `-L<path>`, a bare-name search can silently resolve to GStreamer's
bundled copy instead, compiling fine but corrupting `oca-avbridge`'s FFmpeg calls at runtime
(`avformat_open_input` starts failing on every call, no error at build time). `build.rs` copies
its FFmpeg import libs into its own `OUT_DIR` under unique names before linking specifically to
make that impossible — full diagnosis in `features/fase1/commit_plan.md` (chore-002).

```bash
make build     # debug build, whole workspace
make run       # run the GUI app (debug)
make test      # run every crate's test suite
make test-core # cargo test -p nivela-core only
make bench     # criterion benchmarks (nivela-core parsing/serialization) -> target/criterion/report/index.html
make fmt       # cargo fmt --all
make lint      # cargo clippy --workspace --all-targets
make check     # cargo check --workspace --all-targets (fast compile-only loop)
```

No `make` on PATH → run the underlying `cargo`/`rustup` command directly; every Makefile target
is a one-liner (see [`Makefile`](Makefile)).

Single test: `cargo test -p nivela-core --test probe measure_loudness` (integration tests, one
file per module under `crates/nivela-core/tests/`) or `cargo test -p nivela-app i18n::tests` for
the `src/<module>/tests.rs` unit tests in `nivela-app`.

## Architecture

Three-crate split, enforced by dependency direction: `oca-avbridge` has no dependents within
the workspace besides `nivela-core`; `nivela-core` is UI-agnostic (no `egui` dependency at all)
and `nivela-app` is its only consumer.

- **`oca-avbridge`** — thin C bridge (`csrc/bridge.c`) over libavformat/libavcodec/libavfilter/
  libavutil, called from Rust via FFI (`src/lib.rs`). Written for this project, not an
  auto-generated binding of the full FFmpeg API — keeps the C surface small and auditable.
  `build.rs` locates FFmpeg via `FFMPEG_DIR` (see Commands above). Exposes probing
  (`oca_avbridge_probe`) and export rendering (`oca_avbridge_encode_export`: video
  passthrough-copied, audio decoded → loudnorm+limiter filter graph → AAC re-encoded, with
  progress callback and cooperative cancellation).
- **`nivela-core`** — project/timeline/media data model plus the media wrappers that populate
  it (`probe` and `render` via `oca-avbridge` FFI; `loudness`, `proxy` still via `ffmpeg`
  subprocess), JSON save/load (`persistence`), and mock sample data (`sample`) used to exercise
  the UI before real files are wired in. Locale-neutral by design: it stores data like
  `Recency` (an enum), never pre-formatted display strings — formatting is `nivela-app`'s job.
- **`nivela-app`** — the eframe/egui GUI (glow/OpenGL backend): `app.rs` holds all top-level
  state (`NivelaApp`, which screen is active, loaded projects, export jobs) and mutation methods
  (`open_project`, `queue_export`, etc.); `screens/` has one module per of the five screens
  (home, library, editor, queue, prefs) plus shared `widgets`; `theme.rs` is the dark/teal
  palette; **all UI strings live in `i18n.rs`** (pt-BR and English) — never hardcode display
  text in a screen module, add a `Text` variant instead.

`docs/CODE_GRAPH.md` (module dependency graph + public-API index) used to be regenerated by an
`xtask` crate (`cargo run -p xtask -- graph` / `make graph`); that crate was dropped and the
Makefile/docs referencing it are stale (`make graph`, `make test-xtask` — not fixed as part of
this pass). Use `graphify` instead — see the `## graphify` section below.

Test placement follows what's reachable: `nivela-core` has a `[lib]` target, so its tests are
real integration tests in `crates/nivela-core/tests/` (one file per module) exercising only the
public API, plus `crates/nivela-core/benches/` for criterion benchmarks. A handful of tests that
need a private helper unreachable from `tests/` stay as `src/<module>/tests.rs` unit tests
instead. `nivela-app` is a bin-only crate (no `[lib]`), so all its tests are
`src/<module>/tests.rs` unit tests. `oca-avbridge` has a `[lib]` target, so its FFI tests live
in `crates/oca-avbridge/tests/` against small checked-in media fixtures.

`nivela-core` depends on the `gstreamer` crate (dependency + link verified working alongside
`oca-avbridge`, see Commands above), but no preview pipeline is built on it yet.

Planned architecture (not yet implemented, see the plan doc for phases): decode/preview via
GStreamer or MLT running independent of the UI thread, and a background export queue where
rendering runs on a worker communicating over `tokio::mpsc` so the editing UI never blocks on an
in-progress export — each queued job snapshots its render config at enqueue time, so later edits
to the active project don't affect jobs already in the queue.

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
