# Competitive Feature Implementation Plan

This document turns the 2026-08-28 competitive survey into an implementation plan. It is the
source of truth for the next product-growth work after the original Fase 1-8 plan and the P0-P4
roadmap work. Status remains in [ROADMAP.md](../ROADMAP.md); this file defines scope, sequencing,
acceptance criteria, and security constraints.

The goal is not to reproduce every feature in Premiere, Resolve, Final Cut Pro, CapCut, Descript,
Medal, or Outplayed. Oca should remain a focused gameplay editor whose strongest workflow is:

```text
long recording -> find useful moments -> tighten speech/gameplay -> create chapters and shorts
               -> normalize audio -> export or hand off to another editor
```

## Current product baseline

The implementation already goes substantially beyond the stale summary in the repository root
README. The current baseline, reconciled from the code, ROADMAP, and `matrix/*.md`, is:

| Area | Shipped capability |
|---|---|
| Timeline | Multiple sequences, undo/redo, magnetic and waveform snap, split/trim/move, Ripple/Roll/Slip/Slide, copy/paste, color labels, markers, and Smart Bins |
| Composition | Multiple video/audio layers, PiP, text, shapes, masks, chroma key, crop, blur/sharpen, shake, pixelization, transitions, effects, and broad keyframe coverage |
| Multicam | Waveform synchronization, multicam groups, and angle switching with number keys |
| Speed | Constant speed, stepped ramps, and smooth continuous ramps |
| Color | Balance controls, LUTs, vignette/glitch, waveform, and vectorscope |
| Audio | Loudness measurement/normalization, true peak, batch loudness matching, roles, ducking, gain keyframes, and live level metering |
| AI and automation | MODNet background removal, static face auto-reframe, block-match motion tracking, Whisper subtitles, Piper TTS, silence removal, scene chapters, audio-spike highlights, and Shorts packs |
| Export and reliability | Background export queue, cancellation and persistence, platform presets, source-bitrate matching, proxies, autosave/recovery, telemetry, packaging, and auto-update |
| Collaboration and utilities | Proxy collaboration bundle, YouTube download, and a standalone watched-folder audio-processing script |

The remaining known technical gaps from the existing plan still apply: real-hardware validation
of GPU encoders and preview parity for temporal effects such as stabilization and deflicker.

## Competitive findings

Only official vendor documentation is used for the consolidated refresh:

- Adobe Premiere provides text-based editing, transcript and media search, nested sequences,
  OpenTimelineIO interchange, Frame.io review, caption translation, and AI object masking with
  tracking. Sources: [Premiere Help](https://helpx.adobe.com/premiere/desktop.html),
  [Object Mask](https://helpx.adobe.com/premiere/desktop/add-video-effects/work-with-masks/object-masking.html),
  and [release notes](https://helpx.adobe.com/premiere/desktop/whats-new/release-notes.html).
- DaVinci Resolve 21 provides content/dialogue/face search, advanced speech generation from a
  voice sample, optical-flow retiming, object tracking, Lottie graphics, deep Fairlight audio,
  and Blackmagic Cloud collaboration. Source:
  [Blackmagic Design](https://www.blackmagicdesign.com/products/davinciresolve/whatsnew/).
- Final Cut Pro 12 provides Transcript Search, Visual Search, Auto Mask, compound clips, object
  tracking, and Voice Isolation. Sources: [release notes](https://support.apple.com/en-euro/102825),
  [media search](https://support.apple.com/guide/final-cut-pro/find-clips-and-projects-ver65764b45/mac),
  and [Final Cut Pro](https://www.apple.com/final-cut-pro/).
- CapCut Desktop provides Smart Search, dynamic auto-reframe, filler-word and repetition removal,
  camera tracking, relighting, social templates, and direct sharing. Sources:
  [CapCut Desktop AI](https://www.capcut.com/tools/desktop-ai-power) and
  [Filler Word Remover](https://www.capcut.com/tools/filler-words).
- Descript provides document-style transcript editing, filler-word/retake removal, Studio Sound,
  voice cloning, live collaboration, and direct publishing. Source:
  [Descript Video Editor](https://www.descript.com/tools/video-editor).
- Medal and Outplayed combine recording/replay buffers with real game-event detection, bookmarks,
  separate game/microphone audio, per-game profiles, and automatic storage management. Sources:
  [Medal Auto Clipping](https://support.medal.tv/support/solutions/articles/48001167701) and
  [Medal Windows setup](https://support.medal.tv/support/solutions/articles/48000959661-getting-started-with-medal).

## Product strategy

Oca should specialize before broadening. The preferred strategy is:

1. Reuse the local analysis primitives already shipped: Whisper words, waveform extraction,
   silence detection, face detection, keyframes, proxies, export jobs, and undo snapshots.
2. Import event metadata from recorders and games before owning a complete capture stack.
3. Offer OpenTimelineIO handoff before trying to match every professional finishing tool.
4. Keep analysis local by default. Cloud features must be explicit, optional, and isolated.
5. Every automatic edit must produce a reviewable proposal before it mutates the timeline.

## Ordered implementation backlog

### CF-01: Transcript-based editing and speech cleanup

**Outcome:** use the existing Whisper result as an editing surface, not only as subtitle input.

**Reuse:** `avcore::transcribe::{TranscribeSegment, TranscribeWord}`, `timeline::WordTiming`,
`App::apply_transcription`, the timeline split/delete operations, silence-removal review, and the
undo stack.

**Implementation slices:**

1. Persist a media-relative transcript document with stable word IDs, start/end timestamps,
   confidence, speaker when available, and an explicit schema version.
2. Add a transcript panel that seeks on word selection and highlights the current playback word.
3. Add exact text search across the active asset and then across the project media library.
4. Build a proposed-edit list for deleted phrases, filler words, repeated phrases, and retakes.
5. Apply accepted proposals through existing split/ripple-delete primitives as one undoable action.
6. Keep subtitle text and editorial transcript separate: editing captions must not silently cut
   media, and cutting media must explicitly rebase affected word timings.

**Acceptance criteria:**

- Selecting a word seeks within one frame of its timestamp.
- Deleting a reviewed phrase creates the expected cuts without desynchronizing detached audio.
- Rejected proposals leave the timeline byte-for-byte unchanged.
- One undo restores the full automatic edit.
- Search and review remain responsive on a two-hour transcript.

### CF-02: Gameplay event ingestion and watched-folder import

**Outcome:** improve highlight precision by combining recorder/game events with the existing audio
spike score.

**Reuse:** `scripts/Watch-Gameplay.ps1`, media import/probe, marker types, highlight detection,
chapter generation, and the Shorts pack.

**Implementation slices:**

1. Move watched-folder behavior into a cross-platform core service with configurable stability
   time, duplicate detection, cancellation, and UI status.
2. Define a versioned JSON sidecar containing source identity, event type, source timestamp,
   confidence, and optional pre/post-roll hints.
3. Implement adapters for generic OBS bookmarks first, then documented Medal/Outplayed exports or
   user-provided sidecars. Do not scrape private recorder state.
4. Convert accepted events to typed timeline markers and feed them into highlight scoring.
5. Expose per-game event allowlists and pre/post-roll settings.

**Acceptance criteria:**

- A growing recording is never imported before the writer releases it.
- Importing the same media/sidecar twice is idempotent.
- Unknown schema versions, fields, event kinds, or invalid timestamps are rejected with an
  actionable error.
- Event-only, audio-only, and combined highlight scoring are independently testable.
- No absolute recorder path is required to reopen a project on another machine.

### CF-03: Integrated gameplay-voice cleanup

**Outcome:** make microphone audio usable without leaving the editor.

**Reuse:** the FFmpeg chain proven in `scripts/Watch-Gameplay.ps1`, audio roles, loudness analysis,
ducking, export filters, background jobs, and effect-property undo.

**Implementation slices:**

1. Add a non-destructive `Gameplay Voice` preset for high-pass, denoise, compressor, loudness,
   and limiter stages.
2. Expose a small advanced panel for noise floor, compressor threshold/ratio, and output ceiling.
3. Add an A/B preview and measured before/after loudness and peak values.
4. Keep a future ML voice-isolation backend behind the same effect contract, optional and local by
   default.

**Acceptance criteria:**

- The preset is applied only to `Mic` roles unless the user explicitly selects another role.
- Preview and export use equivalent parameters.
- Bypass is lossless and does not rebuild unrelated preview branches.
- Loudness and peak limits are verified with generated noisy/speech fixtures.

### CF-04: Dynamic auto-reframe for vertical outputs

**Outcome:** follow a face or selected subject throughout a clip and generate smooth crop/position
keyframes for Shorts.

**Reuse:** `FrameSampler`, `detect_faces`, `main_subject_center`, `compute_reframe_crop`, crop and
position keyframes, motion tracking, and platform export presets.

**Implementation slices:**

1. Sample the clip at a bounded adaptive cadence and track subject candidates between samples.
2. Select the primary subject using continuity, size, confidence, and an optional user seed.
3. Smooth the trajectory and create sparse keyframes within frame bounds.
4. Show the proposed path and allow correction before applying it.
5. Make Shorts Pack run dynamic reframe when requested instead of only reusing existing keyframes.

**Acceptance criteria:**

- The crop never leaves the decoded frame or produces a non-positive region.
- Short detection gaps do not cause center jumps.
- Long subject loss falls back predictably to last-known or centered framing.
- Applying the proposal is one undoable operation.
- Sampling has an explicit frame/time budget for long clips.

### CF-05: OpenTimelineIO interchange

**Outcome:** let Oca specialize in gameplay automation while Premiere, Resolve, or other tools can
perform final finishing.

**Implementation slices:**

1. Create an `avcore::interchange` boundary independent of UI and render code.
2. Export sequences, rational time, source ranges, track order, markers, transitions, speed, and
   media references to a supported OTIO schema version.
3. Import the same supported subset into a new sequence; preserve unsupported fields as warnings,
   never silently approximate them.
4. Add a compatibility report listing exported, approximated, and omitted features.

**Acceptance criteria:**

- Export/import round trips the supported subset without timing drift over long sequences.
- Missing media produces offline references rather than dropping clips.
- Unsupported effects are reported with clip/track context.
- Imported paths are normalized and cannot escape an explicitly selected media root.

### CF-06: Live multicam monitor

**Outcome:** display synchronized gameplay, webcam, and secondary feeds while selecting angles in
real time.

**Reuse:** `MulticamGroup`, sync offsets, number-key switching, `Preview`, and proxy media.

**Implementation slices:**

1. Build bounded GStreamer branches for group members, preferring proxies and a configurable
   maximum number of live feeds.
2. Synchronize branches to the program clock and visually distinguish the preview and program
   angles.
3. Record angle decisions as the same ordinary clip splits already used by multicam switching.
4. Degrade gracefully to thumbnails when decode capacity is exceeded.

**Acceptance criteria:**

- Switching remains frame-consistent with existing sync offsets.
- Closing the monitor releases all pipelines and file handles.
- One slow or corrupt angle cannot stall the program preview.
- CPU/GPU and memory limits are observable through existing telemetry.

### CF-07: Parameterized motion-graphics templates

**Outcome:** reusable channel assets such as lower thirds, scoreboards, subscribe prompts, webcam
frames, and 16:9/9:16 variants.

**Reuse:** text/shape clips, layer templates, keyframes, overlays, assets, and persistence.

**Implementation slices:**

1. Define a versioned declarative JSON format with allowlisted primitives and editable parameters.
2. Support text, color, image, timing, safe-area anchors, and aspect-ratio variants.
3. Package templates as data plus validated media assets; no scripts or executable expressions.
4. Add preview, import/export, missing-font fallback, and migration tests.

**Acceptance criteria:**

- Templates render deterministically in preview and export.
- Unknown primitives/parameters are rejected rather than executed or ignored.
- A template cannot reference files outside its extracted asset directory.
- Loading an untrusted template never executes code or network requests.

### CF-08: Semantic transcript and visual media search

**Outcome:** locate moments using queries such as "boss fight", "victory screen", or a phrase
spoken by the presenter.

**Reuse:** transcript storage from CF-01, `FrameSampler`, Smart Bins, media IDs, proxies, and the
background-job/cache patterns.

**Implementation slices:**

1. Ship exact transcript search in CF-01 before adding embeddings.
2. Add a versioned local index keyed by media content fingerprint and model version.
3. Index bounded representative frames and transcript chunks incrementally.
4. Return timestamped results with the source of the match: transcript, visual, metadata, or a
   combined score.

**Acceptance criteria:**

- Search results seek to the matched moment, not only the containing asset.
- Index invalidation is deterministic when media or model versions change.
- Indexing is cancellable, resumable, and bounded in CPU, memory, and disk usage.
- Local search works without a network connection; any cloud provider is explicit opt-in.

### CF-09: Arbitrary-object mask and tracking

**Outcome:** select an arbitrary object/person, generate a segmentation mask, and track it through
the shot for blur, color, or other selective effects.

**Reuse:** MODNet model loading, motion tracking, matte export, manual masks, `FrameSampler`, and
the model-resource preference system.

**Implementation slices:**

1. Start with user-seeded rectangle/lasso selection and a local segmentation model.
2. Propagate masks between sampled frames and offer manual correction at failure points.
3. Store generated matte data outside the main project JSON with versioned references and cache
   invalidation.
4. Support privacy blur as the first end-to-end effect before general selective effects.

**Acceptance criteria:**

- Generated masks can be invalidated/rebuilt without corrupting the project.
- Missing model or matte files produce an offline/rebuild state, not a crash.
- Model input resolution, sample count, memory, and execution time are bounded.
- Manual correction always overrides model output at the corrected keyframe.

### CF-10: Direct publishing and review collaboration

**Outcome:** publish approved exports with metadata and, later, collect timestamped review
comments without manual file transfer.

This is intentionally after OTIO and the local workflow items. Start with YouTube upload plus
title, description, privacy, thumbnail, and chapter metadata. Add other providers only behind a
provider interface after the first integration proves useful. Keep cloud review separate from the
existing offline proxy bundle.

**Acceptance criteria:**

- Authentication uses the system browser and OAuth authorization code with PKCE where supported.
- Access/refresh tokens are stored in the operating-system credential vault, never in `.ocproj`,
  preferences, bundles, command lines, telemetry, or logs.
- Upload retries are resumable and idempotent where the provider supports it.
- The user sees the exact account, visibility, metadata, and destination before publishing.
- Revoking an account removes local credentials without deleting exported media.

## Security requirements shared by the backlog

The following are definition-of-done requirements, not optional hardening:

- Treat media, sidecars, OTIO files, template packages, collaboration bundles, captions, and model
  files as untrusted input. Validate schema, size, type, magic bytes where applicable, numeric
  ranges, and unknown fields before processing.
- Canonicalize filesystem paths and enforce an explicit allowed root before reading referenced or
  extracted files. Never trust names inside archives or imported documents.
- Use structured JSON for interchange. Do not deserialize native/binary objects from untrusted
  sources and do not execute scripts or expressions supplied by templates.
- Keep OAuth and service credentials outside source, project files, bundles, URLs, and logs. Use
  encrypted transport, explicit timeouts, no automatic redirects for generic outbound requests,
  and provider destination allowlists.
- Treat voice samples, face embeddings, transcripts, private media, review comments, and generated
  voice models as sensitive data. Default to local processing, require explicit consent for voice
  cloning, and provide deletion controls for derived artifacts.
- Bound decode duration, frame count, archive expansion, model memory, index size, and concurrent
  jobs so malformed inputs cannot cause unbounded resource consumption.

Relevant weakness classes: CWE-20 (input validation), CWE-22 (path traversal), CWE-434
(unrestricted file ingestion), CWE-502 (unsafe deserialization), CWE-312/CWE-319 (sensitive data
at rest/in transit), CWE-798 (hardcoded credentials), CWE-532 (sensitive logs), and CWE-359
(private personal information exposure).

## Quick wins before or alongside CF-01

- Render timeline markers on the ruler and add them as magnetic-snap targets.
- Finish preview parity for stabilization and deflicker.
- Validate NVENC, Quick Sync, AMF, and VAAPI on real hardware and record the support matrix.
- Integrate the standalone watched-folder workflow instead of leaving it as a PowerShell-only
  utility.
- Keep the repository README and the status matrices reconciled with ROADMAP after each feature.

## Deliberate non-goals

Do not prioritize these until usage proves they are necessary:

- A complete 3D/Fusion-style compositor or advanced HDR finishing page.
- Distributed render-farm export.
- Cloud generative-video features comparable to Generative Extend.
- A complete gameplay recorder before event/bookmark integration has been validated.
- Real-time cloud co-editing for a single-editor workflow.
- Voice cloning before transcript editing and microphone cleanup are complete.
- Replacing the track-based timeline with a trackless magnetic model.

## Delivery order

ER-01's client error-reporting foundation precedes CF-01 for broad beta distribution; see
[client-error-reporting.md](client-error-reporting.md). The competitive feature order remains
CF-01, CF-02, CF-03, CF-04, CF-05, CF-06, CF-07, CF-08, CF-09, and CF-10. A feature may move earlier
only when it unlocks an active PacoPaçoca production problem or provides a prerequisite for the
next item. Each feature must still satisfy [RULES.md](../RULES.md) and update both ROADMAP and the
relevant matrix when shipped.

[<- back to spec/INDEX.md](../INDEX.md)
