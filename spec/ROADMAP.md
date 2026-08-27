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

13. `[ ]` **D1 — automatic silence/dead-air cut.** Low effort, no dependencies. Highest
    time-saved-per-effort of the set.
14. `[ ]` **D7 — lightweight collaboration package.** Low effort, no dependencies.
15. `[ ]` **D4 — automatic chapter markers from scene cuts.** Medium effort.
16. `[ ]` **D5 — beat-aligned cut snapping.** Medium effort. Depends on P0 item 2 (general
    snap mechanism).
17. `[ ]` **D2 — highlight detection from audio spikes.** High effort. Unblocks D6.
18. `[ ]` **D6 — one-click shorts pack.** High effort. Depends on D2.

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
