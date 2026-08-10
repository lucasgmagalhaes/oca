# oca workspace commands.
#
# On Windows without `make` on PATH, MinGW-w64 distributions (e.g. WinLibs) typically bundle
# it as `mingw32-make` instead — run `mingw32-make <target>` in that case. Every target below
# is a thin wrapper around a single cargo/rustup command, so running that command directly
# works too if you'd rather skip make entirely.

# FFMPEG_DIR/PKG_CONFIG_PATH aren't set as persistent Windows env vars on this machine, so
# every fresh shell needs them before cargo can even locate FFmpeg/GStreamer's dev files —
# pinned here to this machine's actual install locations (MSVC target, the workspace default)
# so every target below works out of the box. Also extends PATH with both libraries' runtime
# DLL dirs, needed by anything that links and runs a binary (run/debug/test/bench), not just
# compiles one. If you already export real FFMPEG_DIR/PKG_CONFIG_PATH yourself, these override
# them for recipes run through this Makefile — adjust the three paths below if FFmpeg/
# GStreamer live somewhere else on your machine.
export FFMPEG_DIR := C:/Users/lucas/AppData/Local/Microsoft/WinGet/Packages/BtbN.FFmpeg.LGPL.Shared.8.1_Microsoft.Winget.Source_8wekyb3d8bbwe/ffmpeg-n8.1.2-34-g9b6c8969e0-win64-lgpl-shared-8.1
export PKG_CONFIG_PATH := C:/Users/lucas/gstreamer-msvc/1.0/msvc_x86_64/lib/pkgconfig
export PATH := $(FFMPEG_DIR)/bin;C:/Users/lucas/gstreamer-msvc/1.0/msvc_x86_64/bin;$(PATH)

.PHONY: build release run debug test test-core test-app test-xtask bench fmt fmt-check lint graph docs check clean

# Debug build of the whole workspace (core, ui, xtask).
build:
	cargo build --workspace

# Optimized build. Binary ends up at target/release/ui(.exe).
release:
	cargo build --workspace --release

# Run the GUI app (debug build).
run:
	cargo run -p ui

# Run the GUI app with a backtrace on panic — use this over `run` when chasing a crash.
# (Set as a target-specific variable, not `VAR=val cargo run`, so it doesn't depend on make
# invoking a POSIX shell — this works the same under sh.exe and cmd.exe on Windows.)
debug: export RUST_BACKTRACE = 1
debug:
	cargo run -p ui

# Run every crate's test suite (core's ffprobe/loudness parsers, ui's i18n
# catalog, xtask's codegraph scanner).
test:
	cargo test --workspace

test-core:
	cargo test -p core

test-app:
	cargo test -p ui

test-xtask:
	cargo test -p xtask

# Runs the criterion benchmarks (parsing ffprobe/loudnorm output, project (de)serialization,
# timeline duration math) and writes an HTML report to target/criterion/report/index.html.
# Compare successive runs to catch performance regressions as the engine grows.
bench:
	cargo bench -p core

fmt:
	cargo fmt --all

# Check formatting without rewriting files — use in CI.
fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets

# Regenerate docs/CODE_GRAPH.md (module dependency graph + public API index). Run after
# adding/removing modules or public items.
graph:
	cargo run -p xtask -- graph

# Build and open the full rustdoc API reference for the workspace.
docs:
	cargo doc --workspace --no-deps --open

# Fast compile-only check (no codegen) — quicker feedback loop while editing.
check:
	cargo check --workspace --all-targets

clean:
	cargo clean
