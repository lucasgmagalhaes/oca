# oca workspace commands.
#
# On Windows without `make` on PATH, MinGW-w64 distributions (e.g. WinLibs) typically bundle
# it as `mingw32-make` instead — run `mingw32-make <target>` in that case. Every target below
# is a thin wrapper around a single cargo/rustup command, so running that command directly
# works too if you'd rather skip make entirely.

# ---------------------------------------------------------------------------
# Platform-specific library paths
#
# Set FFMPEG_DIR and (where needed) PKG_CONFIG_PATH in your environment, or
# let the platform block below provide sensible defaults based on common
# install locations.  Nothing here is user-specific — adjust your shell's
# environment instead of editing this file.
# ---------------------------------------------------------------------------

_OS := $(shell uname -s 2>/dev/null || echo Windows)

ifeq ($(_OS),Darwin)
  # macOS — Homebrew (Apple Silicon: /opt/homebrew, Intel: /usr/local)
  _BREW_PREFIX := $(shell brew --prefix 2>/dev/null || echo /opt/homebrew)
  FFMPEG_DIR    ?= $(_BREW_PREFIX)/opt/ffmpeg
  # GStreamer macOS SDK installs its pkg-config files under the framework.
  PKG_CONFIG_PATH ?= /Library/Frameworks/GStreamer.framework/Versions/1.0/lib/pkgconfig
  export FFMPEG_DIR
  export PKG_CONFIG_PATH
  export PATH := $(FFMPEG_DIR)/lib:$(_BREW_PREFIX)/bin:$(PATH)
else ifeq ($(_OS),Linux)
  # Linux — pkg-config handles discovery; FFMPEG_DIR falls back to /usr.
  FFMPEG_DIR ?= $(shell pkg-config --variable=prefix libavformat 2>/dev/null || echo /usr)
  export FFMPEG_DIR
  export PATH := $(FFMPEG_DIR)/bin:$(PATH)
else
  # Windows — set FFMPEG_DIR and PKG_CONFIG_PATH in your environment before
  # running make, or override them on the command line:
  #   make build FFMPEG_DIR=C:/path/to/ffmpeg PKG_CONFIG_PATH=C:/path/to/gst/pkgconfig
  ifndef FFMPEG_DIR
    $(error FFMPEG_DIR is not set. Point it at an FFmpeg dev build with include/ and lib/ subdirectories.)
  endif
  ifndef PKG_CONFIG_PATH
    $(warning PKG_CONFIG_PATH is not set; GStreamer headers may not be found.)
  endif
  export FFMPEG_DIR
  export PKG_CONFIG_PATH
  export PATH := $(FFMPEG_DIR)/bin;$(PATH)
endif

.PHONY: build release run debug test test-core test-app test-xtask test-e2e bench fmt fmt-check lint graph docs check clean

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

# End-to-end tests (e2e/, pytest + pywinauto) driving the real built ui.exe through Windows
# UI Automation. Needs a debug build first (this target doesn't build one for you, since e2e
# runs typically follow a `make build`/`make run` you already did) and Python deps installed
# once: `python -m pip install -r e2e/requirements.txt`.
test-e2e:
	cd e2e && python -m pytest

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
