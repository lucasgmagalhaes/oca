# Competitor Parity Survey

Real feature research (2026-08-27) across CapCut Desktop, DaVinci Resolve, Premiere Pro, Final
Cut Pro — not assumed from memory. Cross-referenced against `spec/matrix/*.md`; only genuinely
new findings listed below (items already tracked — undo/redo, snap, ducking, color scopes,
export presets — aren't repeated here, see `ROADMAP.md` P0/P2).

Sources: [CapCut Desktop Review 2026](https://bigvu.tv/blog/capcut-online-desktop-editor-review/),
[CapCut AI Features Guide](https://freeacademy.ai/blog/capcut-ai-features-complete-guide-review-2026),
[DaVinci Resolve 21 Features Guide](https://documents.blackmagicdesign.com/SupportNotes/DaVinci_Resolve_21_New_Features_Guide.pdf),
[DaVinci Resolve 21 — PetaPixel](https://petapixel.com/2026/06/03/davinci-resolve-21-officially-released-with-new-photo-editing-ai-tools-and-much-more/),
[Premiere Pro 26.0 — Phantom Editor](https://phantomeditor.video/blog/whats-new-premiere-pro-26-2026),
[Premiere Pro Multicam Workflow](https://pixflow.net/blog/premiere-pro-multicam-editing-workflow/),
[Final Cut Pro Magnetic Timeline — Apple](https://support.apple.com/guide/final-cut-pro/intro-to-the-magnetic-timeline-verb8fcfc133/mac),
[Final Cut Pro Multicam — FCPX Full Access](https://fcpxfullaccess.com/blogs/blog/top-10-must-know-final-cut-pro-features-to-supercharge-your-editing),
[Ripple/Roll/Slip/Slide — Noble Desktop](https://www.nobledesktop.com/learn/premiere-pro/perfecting-your-edits-in-adobe-premiere-pro-the-ripple,-roll,-slip,-and-slide-tools).

## New gaps found (not previously tracked anywhere in `spec/`)

- [ ] **Multicam editing.** Sync footage from multiple sources (timecode or audio waveform),
      switch dynamically between angles on one timeline track. Present in all four editors
      surveyed. Directly relevant to oca's actual recording setup (game capture + webcam + mic
      as separate sources) — not a generic nice-to-have for this channel specifically.
      → `ROADMAP.md` P2.
- [x] **Named trim modes: Ripple / Roll / Slip / Slide.** Standard, distinctly-named tools in
      Premiere/DaVinci/FCP, not just generic drag-to-trim:
      - *Ripple* — trim without leaving a gap, later clips shift to fill it.
      - *Roll* — move the cut point between two adjacent clips, total duration unchanged.
      - *Slip* — change which part of the source media shows, without moving the clip on the
        timeline or changing its duration.
      - *Slide* — move a clip along the timeline, adjacent clips' in/out points adjust to
        absorb it, nothing else shifts.
      Confirmed oca's pre-existing trim covered none of these. All four implemented as their
      own `EditorTool` modes — see `matrix/timeline-and-editing.md` for the exact scope.
      → `ROADMAP.md` P2 item 11.
- [ ] **Smart bins.** Rule-based media-pool folders that auto-populate by file type, flag,
      metadata field — DaVinci Resolve. Lower priority for a single-editor/small-team channel
      than for a studio pipeline, but real. → `ROADMAP.md` P4.
- [ ] **Real-time AI object masking.** Premiere Pro 2026 — arbitrary-object segmentation +
      tracking, not fixed-template block matching. A materially bigger lift than oca's current
      motion tracking (`matrix/ai-features.md`, SAD-based, no ML) — would need a real
      segmentation model, similar tier of effort to the background-removal MODNet integration
      already shipped. Noted as an *enhancement path* for motion tracking, not a new roadmap
      item on its own yet.
- [ ] **Motion graphics templates (MOGRT-style reusable animated assets).** Adobe's Graphics
      Templates panel — an animated graphic with editable text/image fields, importable/
      exportable/shareable as a standalone asset. Distinct from oca's existing "layer
      templates" (`matrix/timeline-and-editing.md`), which save a *position/effect
      configuration* for a layer group, not a portable animated-graphic asset with its own
      editable parameter set. Real gap if template sharing between projects/editors ever
      matters; low priority otherwise. → `ROADMAP.md` P5 (deferred, same tier as voice-clone
      TTS — needs its own asset format).

## Validates existing plans (found independently, matches what's already queued)

- CapCut's "Long Video to Shorts" AI clip-suggestion feature is functionally the same idea as
  oca's own **D2 (highlight detection) + D6 (shorts pack)**
  (`architecture/differentiators.md`) — external confirmation this is a real, competitively
  expected feature, not a novel guess.
- Premiere Pro's **Auto Ducking** matches `ROADMAP.md` P2 item 6 exactly — confirms priority.

## Deliberately not adopted

- **Final Cut Pro's trackless "Magnetic Timeline."** A fundamental structural redesign (no
  fixed tracks, connected-clip model), not a bolt-on feature — would mean rearchitecting
  `core/src/timeline.rs`'s track model, not adding a checklist item. Not proposed here; revisit
  only if track-based editing itself becomes a proven pain point for this channel's workflow.
- **DaVinci's AI IntelliScript (script → full timeline).** Same class of "generative AI
  auto-edit" `features/request.md` already explicitly excluded (CapCut's AI Auto-Edit) as a
  separate, much larger generative-AI project outside this editor's scope.

---

[← back to spec/INDEX.md](../INDEX.md)
