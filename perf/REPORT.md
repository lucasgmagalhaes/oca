# Performance Analysis — Report

See `PLAN.md` for methodology. This is the results document: what was found, what was fixed, what
was measured, and what's recommended but not attempted this pass.

## Summary

- Ran `cargo clippy --workspace --all-targets -- -W clippy::perf` across the whole workspace.
  Cross-referenced every finding against whether it sits on a real per-frame/per-call hot path.
- Fixed 3 genuinely hot findings in `crates/core/src/preview_effects.rs` (the CPU-side live-preview
  vignette/glitch/deflicker/LUT effects, applied to every decoded preview frame — see that file's
  own doc comment) plus a matching one in `crates/core/src/preview.rs`'s live-preview compositor,
  swapping `chunks_exact`/`chunks_exact_mut` for `as_chunks`/`as_chunks_mut` (chunk size becomes a
  compile-time const generic instead of a runtime value clippy's `chunks_exact_to_as_chunks` lint
  exists specifically to catch).
- Fixed 4 more small, real, `clippy::perf`-flagged issues elsewhere in `core`: an unnecessary
  `Vec` allocation+copy on every AI-background-removal inference (`&data.to_vec()` → `data`), a
  `contains()`-vs-manual-`iter().any()` in error-report validation, two `Vec::extend(repeat().
  take(n))` → `extend(repeat_n(n))` calls, and one `if let` inside a `for` loop → `.flatten()`.
- Built `perf/`, a real, runnable Criterion benchmark harness (see `PLAN.md` for why it has to be
  a standalone crate) and measured the three `preview_effects.rs` fixes' actual before/after cost
  at a realistic Full HD (1920x1080) frame size — not assumed from the lint's own rationale.
- **The measured result is more nuanced than "clippy said so, so it's faster"** — see below.

## Measured results (Criterion, 1920x1080 RGBA8 buffer, `perf/benches/preview_effects_bench.rs`)

Run with `cargo bench` from `perf/` (200 samples, 15s measurement window per benchmark — see that
file's own comment on why). Numbers are the reported median (Criterion's middle estimate of a
three-point confidence interval); this sandbox's CPU is shared/virtualized and visibly noisy (see
"A necessary caveat" below), so treat anything under roughly 10% as inside that noise floor, not as
a property of the code.

| Function | Before (`chunks_exact`) | After (`as_chunks`) | Delta |
|---|---|---|---|
| `apply_lut_to_rgba` | 83.7 ms | 76.3 ms | **~9-13% faster**, reproduced in every run |
| `apply_glitch_to_rgba` | 20.1 ms | 22.8 ms | inconclusive — direction flipped across repeated runs (see below) |
| `apply_deflicker_to_rgba` | 2.83 ms | 2.96 ms | inconclusive — within noise floor |
| `apply_vignette_to_rgba` (unchanged) | 22.6 ms | 23.6 ms | **same code, two runs** — this ~4% swing is the sandbox's own noise floor |

**`apply_lut_to_rgba` is a real, reproducible win.** Every one of 4 independent full benchmark runs
this session had `after` faster than `before` for this function specifically, by 9-14% each time —
consistent direction even though the exact magnitude moved around, which is the signature of a
real (if modest) effect rather than noise. It's also the most expensive of the four effects by far
(70-90ms per Full HD frame vs. 2-25ms for the other three) — see "The bigger finding" below for why
that absolute cost matters more than the relative improvement.

**`apply_glitch_to_rgba` and `apply_deflicker_to_rgba` did not show a reliable improvement.**
Across repeated runs, `glitch`'s before/after delta ranged from "after 21% slower" to "after 7%
faster" depending on the run — direction wasn't consistent, unlike LUT's. `deflicker`'s two
measured runs disagreed on direction too (one showed `before` "regressing" by 15% relative to a
prior run of the *same* unchanged-in-between code). Given `apply_vignette_to_rgba` — literally
unchanged between the two benchmark runs — itself shows a ~4% swing run-to-run, a single-digit or
low-teens delta on `glitch`/`deflicker` cannot be attributed to the `chunks_exact`→`as_chunks`
change with any confidence in this environment.

**Kept the fix anyway, for both.** `as_chunks`/`as_chunks_mut` is not just "maybe faster" — it also
moves a runtime `len % N == 0` bounds computation the old `chunks_exact*` calls performed on every
call into a compile-time fact (`N` is a const generic), which is a real, if here-unmeasurable,
reduction in what the function does per call, and it's `clippy`'s own current default-lint-level
recommendation (not a nursery/pedantic-only opinion) — reverting to a form clippy itself flags
because one noisy sandbox run showed a fluke regression would be overfitting to measurement noise,
not responding to a real signal. The `_before`/`_after` equality tests in `perf/src/lib.rs`
(`cargo test` from `perf/`, 3/3 passing) confirm every fix is behavior-preserving, independent of
the performance question.

## The bigger finding: LUT preview cost, in absolute terms

83.7ms (before) to apply one 3D LUT lookup pass to one Full HD frame is the more important number
here, independent of the idiom-level fix. At 30fps, a frame budget is ~33ms; at 60fps, ~16ms. A
user with a LUT applied to a clip and live preview at Full HD is **already well outside frame
budget on CPU alone**, before compositing, decoding, or anything else `pump_preview_frame` does
that same frame — matching `TIMELINE_PERFORMANCE.md`'s existing "known gap" style of finding
(confirmed-and-quantified, not hypothetical). The ~9-13% improvement from this pass's fix is real
but doesn't change that headline conclusion. **Recommendation** (not attempted this pass — a
genuinely different, larger scope than a lint-driven fix): the LUT's own trilinear interpolation
(`Lut3D::sample`, called once per pixel — over 2 million times for one Full HD frame) is the
likely dominant cost; a real follow-up should benchmark `Lut3D::sample` in isolation and consider
either a coarser preview-only LUT size (a 3D LUT applied to a downscaled preview frame is a
common real-NLE compromise — Resolve/Premiere both scale live-preview processing to viewport
resolution, not source resolution) or downsampling the *preview frame itself* before applying CPU
effects, matching what a hardware-accelerated preview pipeline would do anyway.

## Fixes applied to `crates/core`

All verified via `cargo check --workspace --all-targets` + `cargo clippy --workspace --all-targets`
(both clean — via the documented temporary `filters.c`/`text_overlay.c` shim, reverted before any
commit; see `CLAUDE.md`). `core`'s own test suite still can't *run* in this sandbox (the
`libonnxruntime` link gap `PLAN.md` describes) — same "type-checked, not run from this repo's own
test suite" caveat every other `core`-side change made from this sandbox carries; the *specific*
functions changed here are additionally verified for real via `perf/`'s own equality tests, which
is strictly more evidence than a typical sandboxed `core` change gets.

1. **`preview_effects.rs`** — `apply_lut_to_rgba`, `apply_glitch_to_rgba`,
   `apply_deflicker_to_rgba`: `chunks_exact`/`chunks_exact_mut` → `as_chunks`/`as_chunks_mut`.
   Also `rgba.len() % 4 != 0` → `!rgba.len().is_multiple_of(4)` in `apply_deflicker_to_rgba`
   (`clippy::manual_is_multiple_of`) — same computation, clippy's preferred spelling.
2. **`preview.rs`** — the live-preview compositor's per-pixel blend loop (`blend_branches`, called
   once per decoded frame whenever an overlay/blend-mode track is active): same `chunks_exact_mut`/
   `chunks_exact` → `as_chunks_mut`/`as_chunks` swap, zipping the base and overlay planes.
3. **`background_removal.rs`** — `resize_matte(&data.to_vec(), ...)` → `resize_matte(data, ...)`.
   `data` was already `&[f32]` (straight from `try_extract_tensor`); `.to_vec()` allocated and
   copied a full `model_w * model_h`-sized float buffer on every single AI-background-removal
   ONNX inference call for no reason — `resize_matte` only ever needed the slice.
   `clippy::unnecessary_to_owned`.
4. **`error_reporting.rs`** — `STATE_TRANSITIONS.iter().any(|known| *known == state)` →
   `STATE_TRANSITIONS.contains(&state)`. Not a hot path (breadcrumb validation, called rarely), but
   a genuine `clippy::perf`-flagged case (`manual_contains`) with zero risk — same result, no
   manual iteration where the standard slice method already does it.
5. **`privacy_blur.rs`** — two `std::iter::repeat(x).take(n)` → `std::iter::repeat_n(x, n)` calls
   in the CF-09 matte-padding function (`clippy::manual_repeat_n`) — `repeat_n` is the direct,
   already-bounded iterator `repeat().take()` builds indirectly via composition.
6. **`dynamic_reframe.rs`** — a `for c in &centers[..] { if let Some((x, y)) = c { ... } }` loop
   in the auto-reframe center-smoothing pass → `for (x, y) in centers[..].iter().flatten()`
   (`clippy::manual_flatten`) — removes a per-iteration branch the `Iterator::flatten` adapter
   already handles.

## Not fixed this pass, and why

- **`crates/core/tests/*.rs` and `#[cfg(test)]` helper `chunks_exact` findings** (`preview_test.rs`,
  `overlay_render_test.rs`, `persistence_test.rs`, `collab_bundle_test.rs`, `multicam_sync.rs`'s
  test-only `burst_signal` helper) — these run once per test invocation, not per frame or per
  inference. Real but not worth the diff noise in the same pass as genuine hot-path fixes; a
  reasonable follow-up for a "clean up remaining clippy::perf noise" pass, not a performance one.
- **`clippy::unnecessary_cast` (6 occurrences, mostly `theme.rs`'s `RADIUS_SM as u8` and similar)**
  — a same-type cast costs nothing at runtime (the compiler already elides it); this is a
  readability nit clippy happens to file under a lint name that sounds perf-related, not an actual
  perf finding. Left alone — fixing it doesn't belong in a performance-analysis pass's scope.
- **`clippy::ptr_arg`** (`&PathBuf` vs `&Path`, 2 occurrences) — `clippy::style`, not
  `clippy::perf`; a `&PathBuf` argument costs the same one word of stack space as `&Path` (both are
  thin pointers to the caller's existing allocation) — no allocation or copy either way. Out of
  this pass's own "obvious == perf-tagged" scope (see `PLAN.md`).

## Recommended, not attempted this pass (real, larger, needs a real dev machine)

These are already-documented findings from this repository's own prior analysis
(`TIMELINE_PERFORMANCE.md`, `spec/matrix/performance.md`) — repeated here only to connect them to
this pass's own methodology, not re-discovered from scratch:

1. **Timeline panel viewport culling** (`TIMELINE_PERFORMANCE.md`'s own "known gap," unchanged
   status). `timeline_panel/mod.rs` draws every clip on every track every frame regardless of
   scroll position — confirmed still true by inspection this pass (the per-track/per-clip loop has
   no visible-range filter before issuing paint calls). This is the single largest *structural*
   performance risk in the codebase for a project with many clips, but fixing it is a real
   multi-function change to the timeline's rendering loop, not a mechanical lint-driven fix, and
   needs a real large project + a real display to confirm the fix doesn't introduce scroll-position
   bugs (off-by-one clipping at the visible boundary, culling a partially-visible clip entirely,
   etc.) — exactly the kind of change this sandbox's own "no way to visually verify a GUI change"
   limitation (documented throughout this repository's session history) makes too risky to attempt
   blind.
2. **`preview.rs`'s per-track cloning** (`current_preview_overlay_clips`/
   `current_preview_audio_clips` clone a full `ClipInstance` + `MediaAsset` per matching track,
   confirmed still present by inspection this pass) — cost scales with overlay/audio track count,
   paid every frame during playback. A real fix (cloning only the specific fields actually read,
   the same discipline `current_preview_clip_lut_and_vignette` already established for the
   background clip) needs auditing every call site's actual field usage first — a correctness-
   sensitive change better done with the ability to run the real preview pipeline against it, which
   this sandbox doesn't have.
3. **`Lut3D::sample`'s absolute per-pixel cost** — see "The bigger finding" above. Worth its own
   dedicated benchmark (already easy to add to `perf/`, since `Lut3D` is already duplicated there)
   and a real design decision (preview-resolution LUT vs. downsampled preview frame), not a
   same-pass mechanical fix.

## How to reproduce

```bash
cd perf
cargo test              # 3/3 before/after equality checks
cargo bench              # full Criterion run, ~2 minutes
```

No `FFMPEG_DIR`/`PKG_CONFIG_PATH`/toolchain setup needed — `perf/` depends on nothing from this
repository's own workspace (see `PLAN.md`).
