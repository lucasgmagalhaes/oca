# Roadmap — Actionable Queue

**Start here to pick the next task.** Reconciled against actual current status (matrix files +
old `CLAUDE.md` Status section, now `matrix/changelog.md`) as of 2026-08-27 — `features/
request.md`'s original phased plan (Fase 1-8) is effectively done; this queue is what comes
after it. Status markers: `[x]` done, `[~]` partial, `[ ]` not started.

**Before picking up any `[ ]`/`[~]` item: read [RULES.md](RULES.md) first.** Short — mandatory
rules + definition of done.

---

## P0 — Editing Foundations

These undermine trust in *everything* built on top — an editor without undo is one mistake away
from losing work, and dragging without snap makes frame-precise cuts tedious for every future
feature that touches the timeline.

1. `[x]` **Undo/redo.** `core::undo::UndoStack` wired into `ui` — `Ctrl+Z`/`Ctrl+Y`, toolbar
   buttons, covers clip add/split/delete/cut/copy/paste, trim/move drags, track/text/shape-
   track add, composite merge, paste-formatting, text-color-modal confirm, and every effect-
   property slider/keyframe editor (gain/crop/mask/color/blur/etc., via a drag-coalescing
   `push_undo_snapshot_for_drag` so a held slider is one undo step, not one per frame);
   `undo_stack.clear()` on sequence/project switch. **Verified**: `cargo test -p core --test
   undo_test` (7/7) and `cargo test -p ui` (239/239, including the drag-coalescing tests) pass;
   the track-add/remove path is additionally covered by a live e2e smoke test against the real
   compiled binary (`e2e/test_undo_redo.py`). See
   [architecture/undo-redo.md](architecture/undo-redo.md) for the design.
2. `[x]` **Magnetic snap** while dragging — playhead scrubbing on the ruler, clip trim (either
   edge), and clip body move (single or composite-group) all snap to the nearest other clip's
   start/end edge or the playhead, within a fixed pixel threshold at the current zoom
   (`screens::editor::timeline_panel`'s `snap_to_nearest`/`snap_move_start`, `SNAP_THRESHOLD_PX`
   = 8px). Hold `Alt` to temporarily disable snapping, the common editor convention. Pure
   functions, unit tested (8 cases: within/outside threshold, no targets, leading-edge snap,
   trailing-edge snap, whichever edge needs less adjustment, out-of-range no-op) — not yet
   driven through a live/e2e build (this is UI-only interaction logic, no avbridge/GStreamer
   pipeline involved to verify against). Timeline markers aren't a snap target — no markers
   feature exists yet (P2 item 9); revisit when it lands. Unblocks D5 (beat-aligned snap, P3),
   which extends this with waveform low-energy points as an additional target.

## P1 — Performance Infrastructure

Read [architecture/performance-and-caching.md](architecture/performance-and-caching.md).

3. `[~]` Dirty-flag mutation classification for timeline edits (position vs. content vs.
   effect vs. track). Confirmed the premise didn't hold as stated — position/effect edits
   already skip a reopen (they just don't live-update the pipeline either, a separate gap);
   the one real hot-path violation found (text-clip property panel forcing a full pipeline
   reopen on every dragged-slider frame, bundling position/content with start/duration) is
   fixed via a cheap `appsrc` buffer refresh. See `matrix/performance.md` for the full
   findings and what's still open (effect-property live preview updates, non-text overlay
   kinds).
4. `[x]` Versioned cache for the timeline→avfilter-graph resolution
   (`resolve_timeline_segments_multi`). The confirmed hot spot was `screens::queue::show`
   recomputing it every UI frame the Fila screen is open, just for a size estimate — fixed via
   `App::resolved_active_sequence_export_preview`'s value-equality cache. See
   `matrix/performance.md` for what is/isn't covered.
5. `[x]` Extract a shared `FrameSampler` primitive — `avcore::FrameSampler`
   (`crates/core/src/frame_sampler.rs`) now backs auto-reframe, motion tracking,
   background-removal matte generation, and (found during the same pass) thumbnail extraction,
   which had the identical shape. Session-reuse for `background_removal::segment_person`'s ONNX
   session is a related but separate gap, left open — see `architecture/performance-and-
   caching.md` §5.

## P2 — High-Impact Parity

Read [matrix/effects-and-color.md](matrix/effects-and-color.md),
[matrix/robustness.md](matrix/robustness.md), [matrix/competitor-parity.md](matrix/competitor-parity.md).

6. `[ ]` Audio ducking (auto-lower music under speech) — `audio_mix.c`'s multi-branch mixing
   already provides the infra this builds on. Confirmed standard in CapCut/Premiere/DaVinci.
   **Skipped over (2026-08-27), picked up item 8 first**: this needs new `avfilter` wiring
   (`sidechaincompress` or equivalent) in `avbridge/csrc/audio_mix.c`, C code this sandbox
   cannot even syntax-check right now — no FFmpeg dev headers present at all (worse than
   `CLAUDE.md`'s documented "too-old packaged FFmpeg" gap; `pkg-config --cflags libavfilter`
   finds nothing here). Picking this up blind, with zero compiler feedback on C changes, isn't
   a reasonable risk to take — do this from an environment with FFmpeg dev headers available.
7. `[x]` Color scopes (waveform/vectorscope) for calibrated grading. `avcore::scopes`
   (pure pixel analysis, no new avfilter/GStreamer element) + an opt-in "📊" toggle on the
   Editor preview panel. Grayscale-intensity simplification, not a calibrated-graticule
   broadcast scope — see `matrix/effects-and-color.md` for the exact scope (pun intended) of
   what shipped.
8. `[x]` Export presets per platform (YouTube Shorts / Instagram Reels / TikTok — resolution +
   aspect + LUFS target bundled under one name). `avcore::PlatformExportPreset`
   (`crates/core/src/export.rs`) + `App::apply_platform_export_preset` + a one-click button row
   on the Fila screen, above the existing aspect-ratio/LUFS pickers (which stay live afterward
   for fine-tuning — a preset isn't a lock). All three presets currently resolve to the same
   numbers (1080x1920, -14 LUFS, matching this codebase's own existing "YouTube" LUFS profile)
   — a real current fact about these platforms' delivery specs, not a shortcut: each preset
   still carries its own independent mapping, ready to diverge without a shape change. Pure
   Rust/UI, no `avbridge` C changes — picked deliberately over item 6 for that reason.
9. `[x]` Review/comment markers on the timeline — `avcore::timeline::Marker`/`MarkerKind`
   (Standard/ToDo/Chapter, FCP's typed-marker model) + a searchable Timeline Index panel
   (text search, click-to-seek, inline edit). Not done: markers as a magnetic-snap target, and
   ruler tick-mark rendering — see `matrix/timeline-and-editing.md` for the exact scope.
10. `[ ]` **Multicam editing** — sync footage from multiple sources (game capture, webcam, mic)
    by timecode or audio waveform, switch angles dynamically on one track. In all four editors
    surveyed (`matrix/competitor-parity.md`); directly matches this channel's actual multi-
    source recording setup. **Skipped over (picked up item 11 first)**: a real implementation
    needs an audio-cross-correlation sync algorithm, a new "multicam group"/angle-switching
    data model, and export/preview wiring for switching sources mid-clip — a multi-part feature
    too large to responsibly finish end-to-end (not just half-wired) in one pass. Do this as its
    own dedicated task.
11. `[x]` **Named trim modes: Ripple / Roll / Slip / Slide.** Confirmed: oca's existing trim/move
    (`ClipInstance::trim_start`/`trim_end`, `Track::move_clip`) matched none of the four —
    trimming an edge never touched neighboring clips at all (no ripple, no roll), and there was
    no way to change source-in/out without moving the clip or changing its duration (no slip).
    All four now implemented as their own `EditorTool` toolbar modes — see
    `matrix/timeline-and-editing.md` for the exact scope and what's still unverified.
12. `[x]` **D3 — series-level loudness consistency** across an export-queue batch
    (`architecture/differentiators.md`). `App::match_loudness_across_queued_jobs` + a button row
    on the Fila screen (shown once ≥2 `Queued` jobs exist) sets one target LUFS across every
    not-yet-started job in the batch — each queued job otherwise keeps whatever `target_lufs`
    its own sequence/tab happened to have when it was queued, so episode 1 and episode 5 of a
    series could silently end up with mismatched targets. Pure orchestration, no new DSP —
    reuses the existing per-job `target_lufs` field and `LUFS_PROFILES` picker.

## P3 — Differentiators

Full spec: [architecture/differentiators.md](architecture/differentiators.md). Ordered by
effort and dependency, not by the doc's own numbering. CapCut's "Long Video to Shorts" and
Premiere's Auto Ducking independently confirm D2/D6 and P2 item 6 are competitively expected,
not novel guesses — see `matrix/competitor-parity.md`.

13. `[x]` **D1 — automatic silence/dead-air cut** (`architecture/differentiators.md`). Built on
    `avcore::waveform`'s per-bucket `(min, max)` peaks, not `avcore::loudness` as the doc
    originally assumed — `measure_loudness` is a single-pass whole-file aggregate (one
    LUFS/TruePeak/LRA report), not a time series, so it can't say *where* in a clip a silence
    falls; `generate_waveform`'s peaks already are windowed amplitude data, and most assets have
    them cached (`MediaAsset::waveform_peaks`) with no new decode pass needed.
    `avcore::silence_detection::{detect_silence_gaps, clip_silence_gaps}` are pure, fully unit-
    tested functions; `Track::ripple_delete_range` is the apply side (splits any clip straddling
    a gap's boundaries, drops what's fully inside, ripples the rest left). `ui`'s toolbar
    "Detect Silence" button + a review modal (`App::begin_silence_review`/
    `toggle_silence_gap_accepted`/`apply_silence_review`/`close_silence_review`) lists every
    detected gap with an accept/reject checkbox defaulting to accepted — nothing touches the
    timeline until "Apply selected cuts" is pressed, per the doc's explicit "not a silent
    auto-apply" requirement. Not yet run against a real GUI session (this environment can't
    launch the eframe app) — verified via `cargo check --workspace --all-targets` (temporary
    local FFmpeg-7.1 shim, see `CLAUDE.md`) and unit/integration tests covering the detection
    math, the ripple-delete edit, and every App-level review method.
14. `[x]` **D7 — lightweight collaboration package** (`architecture/differentiators.md`).
    `avcore::collab_bundle::{export_collab_bundle, import_collab_bundle}` package a project's
    `.ocproj` snapshot plus whatever editing proxies already exist in its proxy cache
    (`avcore::proxy`) into one portable `.zip` — never the multi-GB source media. On import, each
    asset's `proxy_path` is resolved by matching its `source_path`'s filename stem against the
    unpacked proxy dir across every `PreviewQuality`, so the recipient's preview works
    immediately even though the original source almost certainly isn't at that `source_path` on
    their machine at all. `zip` added as a `default-features = false` dependency —
    `CompressionMethod::Stored` is used explicitly since every entry (already-gzip-compressed
    project bytes, already-encoded proxy `.mp4`s) is incompressible in practice, so no optional
    codec needs linking. `ui`: Editor toolbar's "📦 Export Collaboration Bundle..." (save-file
    dialog) and Início's "📦 Import Collaboration Bundle..." (pick `.zip`, then a destination
    folder). **Not run against a live GUI session** — same verification ceiling as D1 above
    (`cargo check --workspace --all-targets` via the temporary local FFmpeg shim, plus
    `cargo check -p core --lib` actually linking and one `cargo test` attempt confirming the
    ONNX Runtime link gap is the only remaining blocker to real execution here); the round-trip
    (export → import, including a fake proxy file surviving intact and resolving to a *new* path
    under the recipient's own cache dir) is covered by `core`'s integration tests and mirrored at
    the `App` level in `ui`.
15. `[x]` **D4 — automatic chapter markers from scene cuts** (`architecture/differentiators.md`).
    `avcore::scene_detection::detect_scene_cuts` scores consecutive sampled-frame pairs by mean
    absolute luma difference (reuses `motion_tracking::rgba_to_gray` for grayscale conversion,
    `avcore::FrameSampler` for sampling — same primitives motion tracking's own background
    thread already uses, not a new decode pass). Detected cuts become non-destructive
    `MarkerKind::Chapter` markers (P2 item 9) rather than a separate accept/reject modal — the
    existing Timeline Index panel's rename/delete already is the review step, since a marker
    (unlike D1's ripple-delete) never mutates the timeline itself. `ui`: Editor toolbar's
    "Detect Chapters" button (background thread, mirrors `motion_tracking.rs`'s split exactly)
    and "Export chapters (.txt)" (plain-text `H:MM:SS Label` list, YouTube's own chapter
    format). Verified the same way as D1/D7 above (`cargo check --workspace --all-targets` via
    the temporary local FFmpeg shim); `core`'s frame-diff scoring and every `App`-level chapter
    method are unit tested, but the actual FrameSampler-driven background thread isn't run
    against real video in this sandbox.
16. `[x]` **D5 — beat-aligned cut snapping** (`architecture/differentiators.md`).
    `waveform_snap_points_for_clip` (`screens::editor::timeline_panel`) reuses
    `avcore::clip_silence_gaps` (built for D1) purely as a "quiet moment finder": each detected
    gap's midpoint is a candidate snap target, with a much shorter minimum gap (0.05s) than D1's
    own cuttable-gap threshold (0.5s) — D5 wants any brief natural pause, not just a length worth
    actually cutting. Wired into the two trim-edge (cut-point) drag handlers only, not the
    whole-clip body-move handler — moving a clip doesn't cut audio. Every trim-driven
    `EditorTool` (plain trim, Ripple, Roll) shares the same snapped value downstream, so one
    change benefits all of them. Verified via `cargo check --workspace --all-targets` (temporary
    local FFmpeg shim) plus new unit tests for the pure mapping function; the actual drag
    interaction isn't run against a live GUI session, same caveat every other timeline-panel
    interaction change in this file already carries.
17. `[ ]` **D2 — highlight detection from audio spikes.** High effort. Unblocks D6.
    **Investigated (2026-08-27), not started**: the doc's premise is "simultaneous game-audio +
    mic spikes," but `Project`/`Timeline` has no structural game-audio-vs-mic distinction —
    `create_new_project` starts with zero tracks, and Video/Audio track *kind* alone doesn't say
    which audio track is the mic and which is a video asset's embedded game audio (the "V1"/
    "A1"/"A2" names seen in test fixtures are just convention, not an enforced or even
    UI-surfaced role). Scoring "two streams at once" needs *some* answer to "which track is
    which" before any DSP gets written — either new per-track metadata (a "role" tag the user
    sets, a real UI addition beyond this feature's own scope) or a scoped-down v1 that scores
    every audio-bearing track independently and OR's the results (loses the "simultaneous"
    cross-correlation the doc specifically calls out, but ships on the existing data model with
    no new UI concept). This is a real design fork, not a blind-implementable reuse the way
    D1/D4/D5 were — left open rather than guessed at.
18. `[ ]` **D6 — one-click shorts pack.** High effort. Depends on D2, so blocked on the same
    open question above.

## P4 — Hardware-Dependent / Confirmed Hard Walls / Lower-Priority Parity

Not blocked on design decisions — blocked on hardware/tooling this dev environment doesn't
have, or genuinely lower value for a small/single-editor channel. Pick up opportunistically,
not by default priority.

19. `[ ]` GPU encode real-hardware verification (NVENC/Quick Sync/AMF/VAAPI) —
    `matrix/engine.md`. Code path exists, never run against real hardware.
20. `[ ]` GPU usage telemetry — `matrix/performance.md`. No cross-platform reader exists;
    needs a vendor-specific one (NVML/etc.).
21. `[ ]` Preview support for vignette/glitch/deflicker/3D-LUT/stabilization —
    `matrix/effects-and-color.md`. Confirmed no matching GStreamer element on the dev machine;
    needs a custom-coded element or CPU-side frame processing, a materially bigger lift than
    every other preview gap closed so far.
22. `[ ]` Smart bins (rule-based media-pool auto-organization) — real in DaVinci Resolve, but
    lower priority for a small/single-editor workflow than for a studio pipeline.

## P5 — Explicitly Deferred

Real ideas, deliberately not queued — revisit only if a P3 differentiator proves the audience
wants more in this direction. See `architecture/differentiators.md`'s "out of scope" section
and `matrix/competitor-parity.md`'s "deliberately not adopted" section.

23. `[ ]` Voice-clone TTS beyond the single bundled Piper voice.
24. `[ ]` Distributed/render-farm export.
25. `[ ]` Motion graphics templates (MOGRT-style portable animated assets, distinct from oca's
    existing layer templates) — needs its own asset format, same tier as voice-clone TTS.
26. `[ ]` Real-time AI object masking/segmentation (Premiere-style, beyond fixed-template
    motion tracking) — same effort tier as the background-removal model integration already
    shipped; revisit as an enhancement path for motion tracking, not standalone.

---

[← back to spec/INDEX.md](INDEX.md)
