# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

NivelaEditor — a native Rust video editor (`egui`/`eframe`) for cutting gameplay footage for
the PacoPaçoca YouTube channel. Its two headline features are automatic loudness normalization
and export-that-matches-the-source-bitrate. Full phased execution plan:
[`plano-editor-pacopacoca_1.md`](plano-editor-pacopacoca_1.md).

Current status: the GUI shell (all five screens, navigable, running on mock data from
`nivela_core::sample`) and the `ffprobe`/`ffmpeg` wrappers (`probe`, `loudness`, `proxy`,
`render`) are implemented and unit-tested, but **not yet wired together**. Not yet implemented:
loading/saving a project as JSON end-to-end from the UI, the GStreamer/MLT decode-and-preview
pipeline, real timeline editing (cut/split/trim), and the background export queue worker. Check
the plan doc for which phase a task belongs to before assuming a feature is live.

## Commands

Requires Rust (stable) via rustup. On Windows without MSVC Build Tools installed, use the GNU
target instead: `rustup target add x86_64-pc-windows-gnu && rustup default
stable-x86_64-pc-windows-gnu`, with a MinGW-w64 toolchain (e.g. WinLibs) `bin` dir on `PATH`.
`ffprobe`/`ffmpeg` on `PATH` are only needed to probe/measure real media files — the app falls
back to mock data without them.

```bash
make build     # debug build, whole workspace
make run       # run the GUI app (debug)
make test      # run every crate's test suite
make test-core # cargo test -p nivela-core only
make bench     # criterion benchmarks (nivela-core parsing/serialization) -> target/criterion/report/index.html
make fmt       # cargo fmt --all
make lint      # cargo clippy --workspace --all-targets
make graph     # regenerate docs/CODE_GRAPH.md — run after adding/removing modules or public items
make check     # cargo check --workspace --all-targets (fast compile-only loop)
```

No `make` on PATH → run the underlying `cargo`/`rustup` command directly; every Makefile target
is a one-liner (see [`Makefile`](Makefile)).

Single test: `cargo test -p nivela-core --test probe measure_loudness` (integration tests, one
file per module under `crates/nivela-core/tests/`) or `cargo test -p nivela-app i18n::tests` for
the `src/<module>/tests.rs` unit tests in `nivela-app`/`xtask`.

## Architecture

Two-crate split, enforced by dependency direction: `nivela-core` is UI-agnostic (no `egui`
dependency at all) and `nivela-app` is its only consumer.

- **`nivela-core`** — project/timeline/media data model plus the `ffprobe`/`ffmpeg` wrappers
  that populate it (`probe`, `loudness`, `proxy`, `render`), JSON save/load (`persistence`), and
  mock sample data (`sample`) used to exercise the UI before real files are wired in. Locale-
  neutral by design: it stores data like `Recency` (an enum), never pre-formatted display
  strings — formatting is `nivela-app`'s job.
- **`nivela-app`** — the eframe/egui GUI (glow/OpenGL backend): `app.rs` holds all top-level
  state (`NivelaApp`, which screen is active, loaded projects, export jobs) and mutation methods
  (`open_project`, `queue_export`, etc.); `screens/` has one module per of the five screens
  (home, library, editor, queue, prefs) plus shared `widgets`; `theme.rs` is the dark/teal
  palette; **all UI strings live in `i18n.rs`** (pt-BR and English) — never hardcode display
  text in a screen module, add a `Text` variant instead.
- **`xtask`** — `cargo run -p xtask -- graph` regenerates [`docs/CODE_GRAPH.md`](docs/CODE_GRAPH.md),
  an auto-generated module dependency graph + public-API index. It's the fastest way to get
  oriented in this codebase without reading every file — check it before a broad exploration.
  Regenerate it (`make graph`) after adding/removing modules or public items; don't hand-edit it.

Test placement follows what's reachable: `nivela-core` has a `[lib]` target, so its tests are
real integration tests in `crates/nivela-core/tests/` (one file per module) exercising only the
public API, plus `crates/nivela-core/benches/` for criterion benchmarks. A handful of tests that
need a private helper unreachable from `tests/` stay as `src/<module>/tests.rs` unit tests
instead. `nivela-app` and `xtask` are bin-only crates (no `[lib]`), so all their tests are
`src/<module>/tests.rs` unit tests.

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
