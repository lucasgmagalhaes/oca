# NivelaEditor

A native video editor for cutting gameplay footage (PacoPaçoca channel), with automatic
loudness normalization and export-that-matches-the-source-bitrate as the two headline
features. See [`plano-editor-pacopacoca_1.md`](plano-editor-pacopacoca_1.md) for the full
execution plan and phase breakdown this workspace is being built against.

## Workspace layout

```
crates/
  nivela-core/   UI-agnostic engine: project/timeline/media data model, ffprobe/ffmpeg
                 wrappers, mock sample data. No egui dependency.
  nivela-app/    Native GUI (eframe/egui, glow backend): the five screens, theme, i18n.
  xtask/         `cargo run -p xtask -- graph` — regenerates docs/CODE_GRAPH.md.
docs/
  CODE_GRAPH.md  Auto-generated module graph + public-API index. Regenerate with `make graph`
                 after adding/removing modules or public items — don't hand-edit it.
```

`nivela-core`'s tests live in `crates/nivela-core/tests/` (one file per module, e.g.
`tests/probe.rs`) as real integration tests against its public API, plus `crates/nivela-core/
benches/` for criterion benchmarks (`make bench`). A handful of tests that exercise a private
helper (not reachable from `tests/`, by design — see the Rust Book's chapter on test
organization) stay as `src/<module>/tests.rs` unit tests instead. `nivela-app` and `xtask` are
bin-only crates (no `[lib]` target), so they have no public API for a `tests/` directory to
link against — their tests are `src/<module>/tests.rs` unit tests throughout.

`nivela-app` is the only consumer of `nivela-core`; nothing in `nivela-core` knows about
`egui`. All display text lives in `nivela-app::i18n` (pt-BR and English today) — `nivela-core`
only stores locale-neutral data (e.g. `Recency` instead of a pre-formatted "2 hours ago"
string).

## Prerequisites

- **Rust** (stable), via [rustup](https://rustup.rs).
- On Windows without the MSVC Build Tools installed, use the `x86_64-pc-windows-gnu` target
  with a MinGW-w64 toolchain (e.g. [WinLibs](https://winlibs.com)) — `rustup target add
  x86_64-pc-windows-gnu && rustup default stable-x86_64-pc-windows-gnu`, with the toolchain's
  `bin` directory on `PATH`.
- **`ffprobe`/`ffmpeg`** on `PATH`, only needed to actually probe/measure real media files
  (`nivela_core::probe`, `nivela_core::loudness`). The app runs fine without them — it just
  falls back to the bundled mock data (`nivela_core::sample`) instead of a loaded project.
- **GNU Make** to use the `Makefile` — on Windows without `make` on `PATH`, MinGW-w64
  distributions typically bundle it as `mingw32-make` instead; run that or call the
  underlying `cargo`/`rustup` commands directly (see the Makefile for what each target runs).

## Common commands

```bash
make build     # debug build, whole workspace
make release   # optimized build (target/release/nivela-app)
make run       # run the GUI app (debug)
make test      # run every crate's test suite
make bench     # run the criterion benchmarks (nivela-core parsing/serialization)
make fmt       # cargo fmt --all
make lint      # cargo clippy --workspace --all-targets
make graph     # regenerate docs/CODE_GRAPH.md
make docs      # cargo doc --workspace --no-deps --open
make clean     # cargo clean
```

Without `make`, the equivalent `cargo`/`rustup` commands are in the `Makefile` itself — every
target is a one-liner.

## Status

The GUI shell (all five screens, navigable, running on mock data) and the `ffprobe`/`ffmpeg`
wrappers (`nivela_core::probe`, `nivela_core::loudness`) are implemented and unit-tested.
Not wired together yet, and not yet implemented: loading/saving a project as JSON, the
GStreamer/MLT decode-and-preview pipeline, real timeline editing (cut/split/trim), and the
background export queue (`tokio::mpsc` worker). See the execution plan for the phase these
land in.
