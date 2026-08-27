# Differentiator Features (beyond `features/request.md`)

`features/request.md`'s original differentiators (automatic loudness normalization,
source-matched bitrate export) are done — see `matrix/changelog.md`. These are the *next* set,
proposed 2026-08-27 after a market-angle review. Each reuses existing infra (avbridge/core
primitives already built) rather than needing a new engine — that's the selection filter, not
just an idea list. Full original write-up/rationale: the published artifact from that session
(ask the user for the link if you need the narrative version — this file is the spec-shaped
one for implementation).

Priority order matches `ROADMAP.md` P3. Effort tags are rough, not estimates to commit to.

---

## D1 — Automatic silence/dead-air cut

**Effort: low.** Detects silent stretches in the commentary track (breath, pause, "hmm") and
suggests or applies ripple-cut. Biggest single time-saver for gameplay-commentary editing —
same feature that made Descript's editing model popular.

- Reuses: per-clip loudness measurement (`avcore::loudness`) — same measurement, add a
  threshold + minimum-duration gate.
- New: ripple-cut across a detected-silence list; a review UI (accept/reject per detected gap
  before applying, not a silent auto-apply).
- Blocked on: nothing — buildable now. Blocks nothing else.

## D2 — Highlight detection from audio spikes

**Effort: high.** Scans a long VOD for simultaneous game-audio + mic spikes (scream, death,
chat reaction) and marks highlight candidates on the timeline. Direct fit for the
"50 chefes do Cuphead" batch-cut use case `request.md` Fase 5 already names.

- Reuses: waveform extraction (`avcore::waveform`), loudness measurement, background export
  queue (run the scan as a queued job, not blocking the UI).
- New: a scoring/threshold pass across two audio streams at once; a candidate-review panel.
- Blocked on: nothing directly, but D6 (batch shorts pack) depends on this.

## D3 — Series-level loudness consistency

**Effort: low.** Per-clip LUFS normalization exists; this adds a series/batch level — analyze
every job in an export queue batch and equalize *perceived* loudness across episodes, so
episode 1 and episode 5 of a series don't sound mismatched back to back.

- Reuses: LUFS analysis (Fase 2), export queue (Fase 5) — this is orchestration over both, not
  new audio DSP.
- New: a batch-scope LUFS target resolver that runs before the per-job normalization pass.

## D4 — Automatic chapter markers from scene cuts

**Effort: medium.** Detects hard scene cuts (loading screen, death screen, menu transition) via
frame-difference during decode, and suggests YouTube-description-ready chapter timestamps.

- Reuses: the decode pass import/proxy generation already does — scene-cut detection piggybacks
  on frames already being decoded, not a new decode pass.
- New: frame-diff scoring + a chapter-marker list export (plain text, timestamp + label).

## D5 — Beat-aligned cut snapping

**Effort: medium.** Dragging a cut point near a low-energy moment in the game audio waveform
snaps to it — avoids a hard cut landing mid-sound-effect or mid-word.

- Reuses: waveform already rendered on the timeline (Fase 3).
- Depends on: the general snap system (`ROADMAP.md` P0 item — snapping doesn't exist at all
  yet, see `matrix/timeline-and-editing.md`) — this is an *extension* of that, not standalone.

## D6 — One-click shorts pack

**Effort: high.** From a long timeline, auto-selects N highlight windows (via D2) and batch-
exports vertical 9:16 cuts with auto-reframe + word-highlight subtitles already applied —
raw VOD to a reviewable batch of shorts in one action.

- Reuses: auto-reframe, word-highlight subtitle rendering, multi-job export queue — three
  already-shipped features tied into one workflow. No new rendering primitive needed.
- Depends on: D2 (highlight detection) for candidate selection.

## D7 — Lightweight collaboration package

**Effort: low.** Generates a portable bundle (project file + edit-resolution proxies, not full
source media) to hand off an edit to a collaborator without moving multi-GB 4K files.

- Reuses: editing proxy (Fase 7) and the project format, which already stores references, never
  copies media in — this is packaging what already exists into one redistributable archive.

---

## Explicitly out of scope (for now)

Flagged during the review as real but too large for this pass — revisit only if a differentiator
above proves the audience wants more:

- Voice-clone TTS (beyond the current single bundled Piper voice) — needs a voice-cloning model
  pipeline, a materially bigger lift than the current TTS feature.
- Distributed/render-farm export — multi-machine coordination, no current need signal.
