# Performance Analysis — Plan

**Date:** 2026-09-06. **Scope:** a first pass at CPU/memory performance across the `oca`
workspace (`avbridge`/`core`/`ui`), producing a real, runnable measurement harness plus a batch of
"obvious" Rust-idiom fixes with measured (not assumed) impact.

## Constraints this plan works around

This sandbox cannot link anything that depends on the `core` crate as a whole: `core`'s `ort`
dependency needs `libonnxruntime` at final-link time (not just compile time), and fetching it
needs network access this environment doesn't have (`ORT_SKIP_DOWNLOAD=1` only defers the failure
past `cargo check`, per `CLAUDE.md`). `ui` links `core`, so it inherits the same gap. Concretely:
`cargo test -p core`/`cargo bench -p core` cannot produce a runnable binary here at all, regardless
of which function is under test — confirmed directly this session (a plain `cargo test -p core
--lib preview_effects` run hangs/times out at the link step). This is a pre-existing, documented
environment gap (`CLAUDE.md`), not something this pass can fix.

**Consequence for methodology:** any benchmark that needs to actually *run* (not just type-check)
has to live in a crate with zero dependency on `core`/`ui`. `perf/` is that crate — see its own
`Cargo.toml` doc comment. It duplicates the specific functions under test rather than importing
them, which is an accepted, explicitly-documented tradeoff (see `REPORT.md`'s "Limitations"), not
a silent one — this repository's own `CLAUDE.md` already establishes the same "copy a
zero-heavy-dependency module into a throwaway scratch crate to get real execution" technique for
verification; `perf/` formalizes it as a persistent, reusable harness instead of a one-off.

## Method

1. **Find candidates two ways, not one guess:**
   - Read the codebase's own prior performance analysis first (`spec/architecture/
     performance-and-caching.md`, `.claude/engineering/TIMELINE_PERFORMANCE.md`,
     `spec/matrix/performance.md`) — this project already has real, specific, previously-confirmed
     hot-path findings (e.g. "no track/clip-level viewport culling" in the timeline panel,
     `preview.rs` cloning a full `MediaAsset`/`ClipInstance` per overlay track). Re-deriving these
     from scratch would waste effort already spent; extending them is cheaper and more reliable.
   - Run `cargo clippy --workspace --all-targets -- -W clippy::perf` (via the documented temporary
     `filters.c`/`text_overlay.c` shim — see `CLAUDE.md` — since `avbridge`'s build script needs
     it to get far enough to reach `core`/`ui` at all) for a mechanical, second, independent source
     of "obvious per Rust's own idiom" findings — exactly the standard this task asked for, not a
     hand-rolled opinion about what "looks slow."
2. **Triage clippy's findings by real hot-path relevance**, not lint count. A `chunks_exact` used
   in a `#[cfg(test)]` helper that runs once per test isn't worth fixing for performance (though
   harmless to fix); the same lint firing inside a function called once per decoded preview frame
   (`preview_effects.rs`, `preview.rs`'s compositor) is a different matter — cross-reference every
   finding's file against what's actually on a per-frame or per-inference call path before
   deciding it's worth a benchmark.
3. **For each fix applied to genuinely hot code, measure it — don't assume the lint's own premise
   holds.** A `_before`/`_after` pair, benchmarked with Criterion against realistic input (a real
   Full HD frame size, not a toy 4x4 buffer), on the *same* machine, back to back. See
   `REPORT.md` for why this mattered here — one of the three fixes measured did **not** show the
   improvement the lint's rationale implies, in this specific noisy sandbox.
4. **Apply the real fix to `crates/core`/`crates/ui`, not just to the perf harness's copy.** The
   harness proves the algorithm; the actual shipped code is what needs to change. Verify via
   `cargo check --workspace --all-targets` + `cargo clippy --workspace --all-targets` (clean) —
   `core`/`ui`'s own test suites still can't *run* here (the same linking gap), so this is the same
   "type-checked, not run" caveat every other `core`/`ui`-side change in this repository's history
   carries when made from this sandbox.
5. **Document what was found but not fixed**, and why — a viewport-culling rewrite of the timeline
   panel, for instance, is real, worth doing, and explicitly out of scope for this pass (a
   materially larger, riskier change than a mechanical lint-driven fix — see `REPORT.md`'s
   "Recommended, not attempted" section).

## What "obvious" means here

Per the task's own framing ("de acordo com o padrão do Rust" — matching Rust's own idiom): a fix
counts as "obvious" for this pass only if **all** of the following hold —

- It is flagged by `clippy::perf` (or an immediately adjacent default-warn lint like
  `manual_flatten`/`manual_is_multiple_of` that removes real per-iteration branching), not a
  hand-identified "this looks slow" guess.
- It changes *how* the same result is computed, never *what* is computed — confirmed per fix via
  either a `_before`/`_after` equality test (the three `perf/` functions) or, where duplicating
  isn't practical, a direct read confirming the transformation is representation-only (e.g.
  `&data.to_vec()` → `data`, `contains()` vs `iter().any()`).
- It's a small, local, single-function diff — never a restructuring (no fix in this pass touches
  more than one function's body).

Real, larger, structural opportunities this project's own docs already flag (viewport culling,
per-track cloning in `preview.rs`) are listed as **recommendations**, not silently folded into
"obvious fixes" scope — see `REPORT.md`.

## What this plan explicitly does not attempt

- Anything needing a real GStreamer pipeline/decoded frame, a GPU, or a large real project file to
  profile against realistically — this sandbox has none of the three (same standing constraint
  `CLAUDE.md`/this session's whole history already documents for other features).
- Wall-clock profiling of the actual `ui` binary (`perf`/`flamegraph`/Instruments) — `ui` can't
  even link here, so it can't run at all, let alone be profiled.
- Restructuring the timeline panel's rendering loop for viewport culling, or any other
  multi-function architectural change — flagged as a recommendation for a follow-up pass with a
  real dev machine (this sandbox's own `preview_effects.rs` benchmark results already show *why*
  a change like that needs to be validated on real hardware, not assumed from a lint).

---

See `REPORT.md` for the actual findings, fixes, and measured numbers.
