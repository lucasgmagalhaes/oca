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

- [x] **Multicam editing.** Sync footage from multiple sources (audio waveform, not timecode —
      see the roadmap item's own scoping note), switch dynamically between angles. Present in
      all four editors surveyed. Directly relevant to oca's actual recording setup (game capture
      + webcam + mic as separate sources) — not a generic nice-to-have for this channel
      specifically. → `ROADMAP.md` P2 item 10.
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
- [x] **Smart bins.** Rule-based media-pool folders that auto-populate by kind/file-name/has-
      audio — DaVinci Resolve. Lower priority for a single-editor/small-team channel than for a
      studio pipeline, but real, and (unlike the rest of P4) pure filtering over data this
      codebase already has, no special hardware or GStreamer element needed.
      → `ROADMAP.md` P4 item 22.
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

## New gaps found (2026-08-27 update — lower cost than the P5 tier)

A second pass, prompted by "what's left that's cheaper than voice-clone TTS/render-farm/MOGRT-
templates/real-time-AI-masking." Sources: [DaVinci Resolve free-tier feature rundown](https://electronics.alibaba.com/question/davinci-resolve-free-what-you-can-(and-can%E2%80%99t)-do-in-2026),
[nested sequences vs. compound clips across Premiere/Resolve/FCP](https://www.steakunderwater.com/VFXPedia/__man/Resolve18-6/DaVinciResolve18_Manual_files/part1331.htm),
[CapCut's curve-based speed ramp](https://www.capcut.com/tools/speed-ramp),
[J-cut/L-cut split edits across Premiere/Resolve/FCP](https://www.miracamp.com/learn/video-editing/j-cuts-and-l-cuts),
[detach/unlink audio across Resolve/FCP/Premiere](https://www.hollyland.com/blog/tips/unlink-audio-and-video-in-davinci-resolve),
[clip/track color labels across Premiere/Resolve/FCP](https://www.tella.com/definition/clip-color-coding),
[Premiere's audio VU meters](https://www.premiumbeat.com/blog/audio-meters-premiere-pro/),
[Resolve's Fairlight loudness meter](https://blog.prosoundeffects.com/advanced-audio-editing-in-davinci-resolve).

- [x] **Clip/track color labels.** Assign a color to a clip or track for at-a-glance
      organization — Premiere (clip labels), DaVinci Resolve (both clip *and* track color,
      called out by users as something Premiere still lacks for tracks), Final Cut Pro (clip
      labels via right-click). The cheapest gap found: no new algorithm, no `avbridge`/
      GStreamer work — a `color_label: Option<[u8; 3]>` field on `Track`/`ClipInstance` plus a
      colored tag/strip in the timeline widget. Same cost tier as `Marker`/`SmartBin`, both
      already shipped this way. → `ROADMAP.md` P4 item 27 (done).
- [x] **Detach/unlink audio from a clip** (the mechanical precondition for J-cuts/L-cuts —
      split edits where audio and video change at different points, present in Premiere/
      Resolve/FCP). oca already supports independent audio-only clips on separate `Audio`
      tracks with their own trim range (used for mic/music/multicam), so the missing piece is
      specifically the one-click action: given a video clip's own embedded audio, mute the
      original (`gain_db` already supports this) and place a synced audio-only clip on an
      `Audio` track pointing at the same asset — both then independently trimmable, same as
      every surveyed editor's version of this. Reuses existing track/clip-creation and muting
      primitives; no new render/preview pipeline work, since per-track independent clips
      already mix correctly (`resolve_audio_segments`). → `ROADMAP.md` P4 item 28 (done).
- [~] **Speed ramping (keyframed speed, not just a constant per clip).** `ClipInstance::
      speed_factor` is a single `f32` — CapCut (curve-based speed editor), Premiere, DaVinci,
      and FCP all have a *smooth* speed curve. Shipped as a **stepped** approximation instead
      (splits the clip into N pieces via `Track::split_clip_at`, each a constant `speed_factor`
      linearly interpolated between a start/end speed) — the smooth version needs the export
      `setpts` filter's output PTS to be the integral of `1/speed` over time, unverifiable in
      this sandbox (no decode capability); see `ROADMAP.md` P4 item 29 for the full reasoning
      and what's still not done.
- [x] **Real-time audio level meter (VU/peak) during playback.** Live level display while
      scrubbing/playing, not just the after-the-fact `LoudnessMetrics` this codebase already
      computes at import/export time — Premiere's classic VU meters and DaVinci's Fairlight
      LUFS/peak meter both do this live. A pad probe on the preview audio path sampling RMS/peak
      per buffer (the same pattern already used for keyframe pad-probes, reading instead of
      writing) plus a small meter widget in the Editor's preview panel. Combined across channels
      (flat sequence), not per-channel — matches this item's own "small meter widget" scope, not
      a full Fairlight-style per-channel meter. → `ROADMAP.md` P4 item 30 (done).

**Found but not included here** — bigger than the four above, closer to Multicam's own tier of
effort than to Smart Bins': **nested sequences / compound clips** (Premiere, DaVinci, and FCP
via its own compound-clip model all let a group of clips be edited as one sub-timeline, then
dropped onto a parent timeline as a single clip — DaVinci's own manual describes this as
functionally the same feature across all three, just named differently). oca's existing
"composite blocks" (`matrix/timeline-and-editing.md`) group clips that move/trim/delete
together but don't give the group its own independently-editable internal timeline or let it be
dropped in as one clip elsewhere — a real gap, but closing it means the render/preview pipeline
recursing into a sub-timeline resolved as if it were one clip, not a bolt-on field. Worth its
own scoping pass if it's ever prioritized, same as Multicam was — not proposed as a small item
here.

## New gaps found (2026-08-27 update — reusing the existing `Keyframe<T>` system)

Prompted by "what else can the existing keyframe infrastructure (position/scale/rotation/
opacity, piecewise-linear, `crate::keyframe::evaluate_keyframes`) drive that it doesn't yet."
Not competitor-survey-sourced like the sections above — an internal capability audit, confirmed
via grep that no other `ClipInstance` property (`gain_db`, brightness/contrast/saturation, crop,
etc.) has a keyframe variant, and that `TextClip`/`ShapeClip` have no keyframe fields at all.

- [ ] **Audio gain keyframes.** `gain_db` is a single constant per clip today — no fade/ramp
      within one clip. Every other surveyed editor supports audio volume automation/keyframes.
      → `ROADMAP.md` P4 item 31 (done).
- [x] **Color grading keyframes.** Brightness/contrast/saturation ramping over a clip (e.g. a
      slow color shift), not just a constant. → `ROADMAP.md` P4 item 32 (done).
- [x] **Crop/pan keyframes.** `crop_x`/`crop_y`/`crop_w`/`crop_h` animated over a clip (e.g. a
      slow reveal/pan independent of `scale_keyframes`' zoom). → `ROADMAP.md` P4 item 33 (done).
- [~] **Text/shape clip animation keyframes.** `TextClip`/`ShapeClip` had zero keyframe fields
      (a structural gap, not a missing effect) — every surveyed editor supports animating
      text/graphic position/scale/opacity over time. → `ROADMAP.md` P4 item 34 (partial —
      `ShapeClip` position keyframes ship; `ShapeClip` scale/rotation and all of `TextClip`
      animation still not done).

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
