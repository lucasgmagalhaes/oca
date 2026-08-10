# oca workspace commands.
#
# On Windows without `make` on PATH, MinGW-w64 distributions (e.g. WinLibs) typically bundle
# it as `mingw32-make` instead — run `mingw32-make <target>` in that case. Every target below
# is a thin wrapper around a single cargo/rustup command, so running that command directly
# works too if you'd rather skip make entirely.

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
