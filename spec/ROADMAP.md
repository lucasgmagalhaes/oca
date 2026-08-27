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

1. `[~]` **Undo/redo.** Core primitive done — `core::undo::UndoStack`, snapshot-based on
   `Sequence` (real, unit tested, see [architecture/undo-redo.md](architecture/undo-redo.md)
   for the design and exactly what's left). Not yet wired to `ui` — no call site pushes a
   snapshot yet, no key binding, no toolbar button. **Not verified against a real build**
   (no GStreamer in this dev sandbox) — run `cargo test -p core --test undo_test` on a machine
   that can build `core` before trusting it beyond code review.
2. `[ ]` **Magnetic snap** while dragging (playhead, other clip edges, markers). Blocks D5
   (beat-aligned snap, P3) — build the general mechanism first, D5 extends it.

## P1 — Performance Infrastructure

Read [architecture/performance-and-caching.md](architecture/performance-and-caching.md).

3. `[ ]` Dirty-flag mutation classification for timeline edits (position vs. content vs.
   effect vs. track) — today likely every edit forces the same full-pipeline-reopen path in
   preview; confirm, then fix.
4. `[ ]` Versioned cache for the timeline→avfilter-graph resolution
   (`resolve_timeline_segments_multi`) — rebuilds from scratch on every call today.
5. `[ ]` Extract a shared `FrameSampler` primitive — auto-reframe, motion tracking, and
   background-removal matte generation each reimplement their own seek-and-poll sampling loop.

## P2 — High-Impact Parity

Read [matrix/effects-and-color.md](matrix/effects-and-color.md),
[matrix/robustness.md](matrix/robustness.md), [matrix/competitor-parity.md](matrix/competitor-parity.md).

6. `[ ]` Audio ducking (auto-lower music under speech) — `audio_mix.c`'s multi-branch mixing
   already provides the infra this builds on. Confirmed standard in CapCut/Premiere/DaVinci.
7. `[ ]` Color scopes (waveform/vectorscope) for calibrated grading.
8. `[ ]` Export presets per platform (YouTube Shorts / Instagram Reels / TikTok — resolution +
   aspect + LUFS target bundled under one name).
9. `[ ]` Review/comment markers on the timeline — consider Final Cut Pro's typed-marker +
   searchable Timeline Index model, not just a plain note.
10. `[ ]` **Multicam editing** — sync footage from multiple sources (game capture, webcam, mic)
    by timecode or audio waveform, switch angles dynamically on one track. In all four editors
    surveyed (`matrix/competitor-parity.md`); directly matches this channel's actual multi-
    source recording setup.
11. `[ ]` **Named trim modes: Ripple / Roll / Slip / Slide** — confirm which of the four oca's
    current trim tool actually covers, fill the rest. See `matrix/competitor-parity.md` for
    the exact definition of each.
12. `[ ]` **D3 — series-level loudness consistency** across an export-queue batch
    (`architecture/differentiators.md`). Low effort, pure orchestration over LUFS analysis +
    export queue, both already built.

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
