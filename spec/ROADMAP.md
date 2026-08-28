# Roadmap — Actionable Queue

**Start here to pick the next task.** Reconciled against actual current status (matrix files +
old `CLAUDE.md` Status section, now `matrix/changelog.md`) as of 2026-08-28 — `features/
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
   fixed via a cheap `appsrc` buffer refresh. **Partial follow-up**: color-balance sliders
   (brightness/contrast/saturation — the confirmed hot-path violation's own motivating
   example) now update live too, via `Preview::set_live_balance` pushing directly to the
   already-built `videobalance` element (`gst::Bin::by_name`, named `oca_balance_{clip_id}`)
   instead of waiting for an incidental reopen — deliberately scoped to just this one
   property group, not the full "every effect setter" scope, which stays a materially
   bigger lift (most effects' elements are only conditionally present at all — e.g. no
   `gaussianblur` exists in the pipeline until `blur_intensity` first goes non-zero — so a
   live update for those needs restructuring the running filter graph, not just a property
   push; color balance was tractable specifically because brightness/contrast/saturation all
   share one element that, once built, stays present across any further change among the
   three). Verified against a real GStreamer pipeline in a scratch crate (this sandbox's
   `core` test binary can't link — the ONNX Runtime gap — but `avbridge` alone can, so a
   scratch crate depending on real `avbridge` + `gstreamer` proved the negative-path logic
   for real): `set_live_balance` correctly declines when no element was built, and correctly
   declines for a mismatched clip id. **Not verified**: that it finds/updates the element
   once one exists, or that a push actually changes decoded output — this sandbox's `playbin`
   never constructs a working video output branch at all (`current_frame()` returns `None`
   for every clip, confirmed against an unmodified copy of `preview_test.rs`'s own
   pre-existing test), the same GUI/hardware-dependent-verification limitation this codebase's
   test suite already carries elsewhere.

   **Second follow-up**: blur/sharpen sliders also update live now, via `Preview::set_live_blur`
   pushing to the already-built `gaussianblur` element (named `oca_blur_{clip_id}`, same naming
   convention `set_live_balance`'s `videobalance` element uses) — the one other case where a
   single named element, once built, stays present across any further change to the property
   that built it: `gaussianblur`'s single signed `sigma` covers both `blur_intensity` (positive)
   and `sharpen` (negative) via one derived `net_sigma`, so there is no "which of several
   properties" ambiguity color balance's three-property case has to resolve either. Wired
   through the same `App::with_selected_clip_mut` shared dispatch path color balance's own push
   already goes through, so every effect setter gets the cheap no-op push for free without its
   own call site. Verified via `cargo check`/`clippy` for the whole workspace (clean) and two
   negative-path tests (`set_live_blur_returns_false_when_no_element_was_built`/
   `_for_a_mismatched_clip_id`, mirroring `set_live_balance`'s own two exactly) — same "type-
   checked but not run" status as those, since `core`'s own test binary still can't link in this
   sandbox (the ONNX Runtime gap) and reproducing `preview.rs`'s full `Preview`/`ClipInstance`
   surface in a throwaway scratch crate (unlike `set_live_balance`'s own verification, which
   only needed `avbridge` + `gstreamer`, not the two-thousand-line `preview.rs` module itself)
   wasn't attempted this round.

   **Third follow-up**: chroma-key color/tolerance also update live now, via
   `Preview::set_live_chroma_key` pushing to the already-built `alpha` element (in
   `method=custom` mode, named `oca_chromakey_{clip_id}`) — a third case fitting the same
   "single element, once built, stays present across further edits to the property that built
   it" shape. Unlike balance/blur, this element is only ever built for a composited-overlay
   branch gated on `chroma_key_enabled`, a plain on/off toggle rather than a gradually-
   approached intensity — toggling chroma key on for the first time still needs the existing
   incidental-reopen fallback to build the element at all; only color/tolerance edits made
   *after* that go live. Wired through the same shared dispatch path. Two negative-path tests
   mirror the blur ones exactly, one confirming a plain (non-composited) pipeline never builds
   this element at all, the other a mismatched clip id against a real chroma-key-enabled
   overlay clip — same "type-checked, not run in this sandbox" status.

   **Fourth follow-up**: crop, pixelize, shake, and (layer) mask all update live now too,
   closing out this item's own "genuinely a bigger lift" note above — each needed a materially
   different mechanism, not a copy-paste of `set_live_balance`'s plain property push:
   `Preview::set_live_crop` names the static-crop `videocrop` element (`oca_crop_{clip_id}`) and
   redoes the fraction-to-pixel math using a new `Preview::clip_resolutions` cache (populated at
   build time) so `ui`'s `App` never needs to track decoded frame dimensions itself;
   `Preview::set_live_pixelize` renegotiates the downscale `capsfilter`'s caps live (block-size
   caps changes propagate as a real renegotiation, unlike shake/crop's window shape) rather than
   rebuilding the two-`videoscale` chain; `Preview::set_live_shake` writes into a shared
   `ShakeMarginHandle`'s atomics the buffer probe already reads every frame — no property push
   or renegotiation needed at all, just new numbers; `Preview::set_live_mask` reuses the
   `refresh_text_overlay` pattern (`appsrc ! imagefreeze(allow-replace=true)`) to push a freshly
   rasterized `GRAY8` buffer. Deflicker/stabilization remain the genuinely open gap — both need
   *temporal* state across multiple frames, a different problem shape entirely. Shapes (non-text
   overlays) still have no live-preview path. Wired through the same `with_selected_clip_mut`
   shared dispatch path as balance/blur/chroma-key. See `matrix/performance.md` for the full
   findings.
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

6. `[x]` Audio ducking (auto-lower music under speech). Reuses the existing `AudioRole`
   (`Mic`/`Music`/`GameAudio`/`Unspecified`) rather than new per-track metadata:
   `AudioRole::to_duck_role_code()` (core) maps it to a raw `u8` crossing the FFI boundary as
   `AudioSegment::duck_role`, and `build_mix_graph` in `avbridge/csrc/audio_mix.c` routes every
   `Music`-tagged branch through a `sidechaincompress` keyed by the mixed `Mic`-tagged branches
   whenever both roles are present on the timeline — opt-in and additive, byte-identical to the
   old flat `amix` otherwise. A trigger (mic) branch needs its audio in two places at once (the
   sidechain control input *and* still audible in the final mix) but a filter output pad can
   only be consumed once, so each trigger branch gets its own `asplit` feeding both consumers —
   the bug that produced `AVERROR(EINVAL)` on the first attempt, found by real
   `avfilter_graph_config` runs, not by re-reading the C. Verified for real, not just
   syntax-checked: `crates/avbridge` has zero heavy dependencies (no ONNX/whisper/GStreamer), so
   `cargo test -p avbridge --test audio_mix_test` fully links and runs in this sandbox against
   real fixture media — three new integration tests cover single-branch-per-role, multiple
   branches per role (the `music_mix`/`trigger_mix` sub-`amix` paths), and a target with no
   trigger falling back to the original flat mix. `resolve_audio_segments` wiring covered in
   `core`'s own test suite. The prior skip note (no FFmpeg dev headers, `pkg-config` found
   nothing) no longer applies in this sandbox — `pkg-config --cflags/--modversion libavfilter`
   both resolve now.
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
10. `[x]` **Multicam editing** — sync footage from multiple sources (game capture, webcam, mic),
    switch angles dynamically. Scoped to the smallest end-to-end vertical slice that's honestly
    "shipped," not half-wired: angles are ordinary `Video` tracks (no new `TrackKind`), grouped
    by a new `avcore::timeline::MulticamGroup` sidecar record (member tracks, program/active
    track, per-track sync offsets) — same non-invasive shape `Marker` uses.
    - **Sync**: audio-waveform only (not timecode — no code in this repo reads embedded
      timecode/creation-time metadata, and waveform sync alone satisfies "sync by audio").
      `avcore::multicam_sync` cross-correlates each track's first clip's audio (via the
      already-existing `avbridge::extract_pcm_16k_mono`, previously whisper.cpp-only) against a
      reference track — a coarse RMS envelope + bounded lag search, not full-resolution
      correlation (too slow over a multi-minute recording). Pure Rust, no new `avbridge`/C code.
    - **Switching**: `Timeline::switch_multicam_angle` splits the program track's clip at the
      playhead (`Track::split_clip_at`, already used by D1/D6) and retargets the new piece's
      `asset_id`/`source_in_secs`/`source_out_secs` to the target angle's footage at the
      sync-offset-adjusted equivalent time — reuses existing editing primitives end to end,
      no new avfilter/C wiring. `App::create_multicam_group_from_video_tracks` (Editor toolbar's
      "Sync Multicam" button) builds the group; number keys 1-9 at the playhead switch angles.
    - **Export correctness falls out for free**: once a switch is materialized as an ordinary
      same-track clip split, `resolve_timeline_segments_multi`/`resolve_audio_segments` already
      produce correct segments — no `render.rs` changes needed.
    - **Explicitly not done** (documented, not silently missing): live multi-feed preview during
      scrub/playback — still plays one decoded source, same "no live preview effect yet" gap
      every other per-clip effect has; a real multicam monitor needs GStreamer multi-branch
      preview-pipeline work, a separate and larger task. Verified via a real-execution scratch-
      crate (this sandbox's `core` test binary can't *link* — the pre-existing ONNX Runtime gap
      — but `timeline.rs`/`keyframe.rs`/`multicam_sync.rs` have zero heavy deps, so copying them
      into a throwaway crate gives genuine `cargo test` runs, not just type-checking) — 15 new
      tests across sync-offset correlation and group/switch semantics, all passing for real.
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
17. `[x]` **D2 — highlight detection from audio spikes** (`architecture/differentiators.md`).
    Resolved the design fork this item's own investigation note (2026-08-27, preserved below)
    found — per-track role metadata, not the scoped-down independent-scoring fallback, since it
    matches the doc's actual "simultaneous" premise and doubles as groundwork for the
    already-queued Multicam item (P2 item 10).
    - `avcore::timeline::AudioRole` (Unspecified/GameAudio/Mic/Music) on `Track`, user-set via a
      small icon `ComboBox` on the timeline track header (`App::set_track_audio_role`, not
      undo-tracked — metadata, not content).
    - `avcore::highlight_detection::{clip_amplitude_samples, detect_highlight_candidates}`:
      `clip_amplitude_samples` maps one clip's asset waveform into timeline-relative amplitude
      samples (same source-to-timeline mapping `clip_silence_gaps`/D1 uses, but every bucket's
      peak, not just below-threshold runs — a highlight cares about loud moments).
      `detect_highlight_candidates` correlates two independently-sampled, irregularly-spaced
      series onto a common coarse time grid and flags windows where both channels spike at once.
    - `MarkerKind::Highlight` — auto-detected candidates become non-destructive markers, same
      "Timeline Index panel's rename/delete is the review step" shape D4's Chapter markers
      already established, not a separate accept/reject modal.
    - `ui`: `App::detect_highlights` runs synchronously (no background thread — only reads
      `MediaAsset::waveform_peaks`, already cached in memory, same reasoning D1's
      `begin_silence_review` relies on) from the Editor toolbar's "Detect Highlights" button.
    - Verified via `cargo check --workspace --all-targets` (temporary local FFmpeg shim);
      `core`'s own math was independently confirmed by extracting `highlight_detection.rs` into
      a throwaway no-dependency scratch crate and running its tests for real (this sandbox's
      `core` crate itself only type-checks — the ONNX Runtime link gap blocks `cargo test`
      outright, see `CLAUDE.md`), plus new `App`-level unit tests for the full detect-and-mark
      flow.
    <details><summary>Original investigation note (2026-08-27), preserved for context</summary>

    Not started as of that note: the doc's premise is "simultaneous game-audio + mic spikes,"
    but `Project`/`Timeline` had no structural game-audio-vs-mic distinction —
    `create_new_project` starts with zero tracks, and Video/Audio track *kind* alone doesn't say
    which audio track is the mic and which is a video asset's embedded game audio (the "V1"/
    "A1"/"A2" names seen in test fixtures were just convention, never an enforced or even
    UI-surfaced role). Scoring "two streams at once" needed *some* answer to "which track is
    which" before any DSP could be written — either new per-track metadata (a "role" tag the
    user sets) or a scoped-down v1 that scores every audio-bearing track independently and OR's
    the results (loses the "simultaneous" cross-correlation the doc specifically calls out). Left
    open rather than guessed at; resolved above once asked.
    </details>
18. `[x]` **D6 — one-click shorts pack** (`architecture/differentiators.md`). Confirmed three
    scope decisions with the user before writing this — the biggest single item in P3, several
    real design forks, not a blind-implementable reuse:
    - **Highlight window size**: D2 only stores a single marker position per highlight, not a
      range, so D6 needed its own heuristic — a fixed 5s lead-in + 10s reaction (15s total),
      clamped to the sequence's own bounds, over a user-configurable setting (no new UI surface
      needed).
    - **Auto-reframe**: reuses whatever `position_keyframes` the windowed clip already carries
      rather than running a fresh face-detection pass per short — a short whose source clip was
      never auto-reframed just exports centered, a real documented gap, not a silently-forced
      extra pipeline run.
    - **Subtitles**: reuses existing transcribed `TextClip`s that fall inside the window rather
      than triggering a fresh Whisper run — a never-transcribed source just exports without
      subtitles, same reasoning as auto-reframe above.

    `avcore::extract_timeline_window` (new `core` primitive, `timeline_window.rs`) turns one
    `[start, end)` slice of a `Timeline` into its own standalone, rebased-to-zero `Timeline`:
    Video/Audio clips straddling a boundary are split precisely via the existing
    `Track::split_clip_at`; Text/Shape overlays skip splitting entirely (word-highlight timing is
    relative to a `TextClip`'s own `start_secs`, so approximating a mid-clip split risks
    desyncing captions from audio) — only overlay clips *entirely* inside the window are kept.
    `App::spawn_shorts_pack` loops every `MarkerKind::Highlight` marker, extracts its window,
    resolves it exactly the way "Adicionar exportação" resolves the whole sequence (forced to
    `ExportAspectRatio::Portrait`), and queues it — a window that resolves to zero clips (e.g. it
    landed entirely in a gap) is skipped, not treated as a hard error, and the toolbar's "Shorts
    Pack" button reports a queued-vs-skipped summary toast.

    Verified via `cargo check --workspace --all-targets` (temporary local FFmpeg shim);
    `extract_timeline_window`'s split/boundary math was independently confirmed for real by
    extracting `timeline.rs`/`keyframe.rs`/`timeline_window.rs` into a throwaway no-dependency
    scratch crate and running its tests (this sandbox's `core` crate itself only type-checks —
    the ONNX Runtime link gap blocks `cargo test` outright, see `CLAUDE.md`), plus new `App`-level
    unit tests for the full extract-resolve-queue flow. **Not run against a live GUI session** —
    the actual exported `.mp4` files (correct framing/timing/audio for a real windowed short)
    haven't been visually verified, same caveat every UI-only change in this sandboxed
    environment already carries.

## P4 — Hardware-Dependent / Confirmed Hard Walls / Lower-Priority Parity

Not blocked on design decisions — blocked on hardware/tooling this dev environment doesn't
have, or genuinely lower value for a small/single-editor channel. Pick up opportunistically,
not by default priority.

19. `[ ]` GPU encode real-hardware verification (NVENC/Quick Sync/AMF/VAAPI) —
    `matrix/engine.md`. Code path exists, never run against real hardware.
20. `[x]` GPU usage telemetry — `matrix/performance.md`. No cross-platform reader exists, so
    this went vendor-specific: `avcore::GpuSampler` via `nvml-wrapper` (NVML), which dynamically
    loads `libnvidia-ml.so`/`nvml.dll` at runtime rather than link-time linking against it — a
    machine with no NVIDIA GPU/driver at all (confirmed against this sandbox's own dev machine)
    just gets `None` back from `GpuSampler::new()`, never a build-time requirement. AMD/Intel
    stay out of scope, same single-vendor call the GPU encoder ladder (item 19) already made.
    Positive-path (a real non-`None` sample against actual NVIDIA hardware) stays unverified in
    this sandbox — same category as item 19's own hardware-verification gap.
21. `[~]` Preview support for vignette/glitch/deflicker/3D-LUT/stabilization —
    `matrix/effects-and-color.md`. Confirmed no matching GStreamer element on the dev machine for
    any of the five; a custom-coded element was never attempted (no way to visually verify a
    GStreamer plugin in this sandbox). Instead, **partial**: `avcore::preview_effects` covers
    3D LUT and vignette as CPU-side post-processing of the already-decoded preview frame — same
    pattern `avcore::scopes` established for the waveform/vectorscope overlays. `Lut3D::parse`/
    `load` read the standard `.cube` format with real trilinear interpolation (precise,
    unit-tested, no visual-verification risk); the vignette is a simple radial-falloff
    *approximation*, explicitly not FFmpeg's own cosine-based formula (reproducing that exactly
    from `libavfilter` C source without being able to A/B it visually against export wasn't a
    risk worth taking). `App::pump_preview_frame` applies both to the live preview texture only
    — export is untouched, still the real `lut3d`/`vignette` `avfilter`s.

    **Follow-up**: glitch is covered too now. The "no single well-specified algorithm" objection
    dissolved once actually checked — export's own `glitch_intensity` already resolves to a
    concrete, specific avfilter (`noise=c0s=...:c0f=t:c1s=...:c1f=t:c2s=...:c2f=t`, temporal
    luma/chroma noise, not an RGB-shift/block-displacement "datamosh" look), so this isn't a
    from-scratch judgment call, just mirroring what this codebase's own glitch effect already
    *is*. `avcore::apply_glitch_to_rgba` adds per-pixel, per-channel additive noise (RGB, not a
    luma/chroma split — no YUV conversion available on an already-decoded RGBA frame) via a
    xorshift64* PRNG seeded per pixel from a caller-supplied `seed`; `App::pump_preview_frame`
    passes elapsed wall-clock time (bit-cast to `u64`) as that seed, so the pattern actually
    looks like temporal corruption frame to frame rather than a static grain overlay baked onto
    the image. Still a **preview approximation**, not a reproduction of
    `libavfilter/vf_noise.c`'s own PRNG — same "not bit-exact" caveat vignette already carries.
    **Explicitly still not done**: deflicker and stabilization (both need *temporal* state across
    multiple frames, a materially larger, stateful piece of work with its own seek/scrub edge
    cases — not a natural extension of this per-frame-only module). Verified via a real-execution
    scratch crate (`preview_effects.rs` has zero heavy deps) — 19 tests (12 original + 7 new for
    glitch: no-op at zero intensity, empty-buffer no-op, alpha untouched, byte-bounds safety at
    full intensity, determinism for a fixed seed, divergence across seeds, and an actual-
    perturbation sanity check), one of which caught a real bug in a *test's own* expected value
    (a coarse 2-point LUT interpolates rather than reproducing the exact original channel value)
    before it could pass silently.
22. `[x]` Smart bins (rule-based media-pool auto-organization) — real in DaVinci Resolve, but
    lower priority for a small/single-editor workflow than for a studio pipeline. The one P4 item
    tractable in this sandbox without special hardware or a missing GStreamer element (unlike 19-
    21) — pure filtering over `Project::media_library`, no `avbridge`/GStreamer/GPU dependency.
    `avcore::SmartBin` (`kind_filter`/`name_contains`/`requires_audio`, every set criterion
    ANDed) + `Project::add_smart_bin`/`remove_smart_bin`/`smart_bin_mut`. Membership isn't
    stored — `SmartBin::matches` is evaluated fresh against the live `media_library` every time,
    same "recompute, don't cache" shape `Marker`/`MulticamGroup` already follow. Editor Library
    panel: a filter-chip row above the asset list ("All" + one chip per bin, click-to-select/
    double-click-to-edit) plus "+ New Bin", backed by a create/edit modal (name, kind Any/Video/
    Audio, file-name-contains text, has-audio Either/Yes/No, Save/Cancel/Delete). Verified via a
    real-execution scratch crate (same ONNX-link-gap workaround as multicam) — 5 passing tests on
    `SmartBin::matches`.
27. `[x]` Clip/track color labels — `matrix/competitor-parity.md`'s 2026-08-27 update. Present
    in Premiere (clip), DaVinci Resolve (clip *and* track), FCP (clip). The cheapest gap in
    that update: pure data (`ClipInstance::color_label`/`Track::color_label`, `Option<[u8;3]>`)
    + timeline-widget rendering, no `avbridge`/GStreamer work — same cost tier as `Marker`/
    `SmartBin`, both already shipped. A fixed 6-swatch palette (matching Premiere/DaVinci/FCP's
    own fixed-palette convention, not a free color picker) offered via a right-click context
    menu on a timeline clip or the track-header name; clears via a "Limpar rótulo" entry.
    Overrides the clip/track's usual kind-based fill color when set. Carried across
    `Track::split_clip_at` (both halves keep the label, same as `transition_in`). Deliberately
    excluded from `ClipFormatting` — an organizational tag, not a rendering style, same
    reasoning `background_removal_mask_path` is excluded for a different reason.
28. `[x]` Detach/unlink audio from a clip (the mechanical precondition for J-cuts/L-cuts) —
    `matrix/competitor-parity.md`. `App::detach_audio_from_selected_clip` mutes the video
    clip's own audio (`gain_db` set to `GAIN_DB_RANGE`'s floor — no separate "muted" flag exists,
    so muting reuses the existing gain primitive as scoped) and places a new clip on an Audio
    track pointing at the same asset, with the same trim range and timeline placement, at unity
    gain — both then independently trimmable. Reused a factored-out `default_clip_instance`
    helper (previously duplicated between `add_asset_to_timeline`/`add_asset_to_timeline_at`)
    rather than adding a third copy. No new render/preview pipeline work — per-track independent
    clips already mix correctly. Triggered via a "Destacar áudio" entry in the timeline clip's
    context menu, enabled only for Video-track clips.
29. `[x]` Speed ramping — `App::apply_speed_ramp_to_selected_clip` ships a **stepped**
    approximation, not the smooth continuous curve CapCut/Premiere/DaVinci/FCP all have. A
    deliberate scope decision (raised to and confirmed by the user, 2026-08-27): the smooth
    version needs the export-side `setpts` filter's output PTS to be the *integral* of
    `1/speed` over time, which for a piecewise-linear speed curve has no simple closed form
    (needs a `log()` term per segment) — a real, easy-to-get-subtly-wrong derivation with no
    way to render/verify it in this sandbox (no decode capability), unlike every other
    `Keyframe<T>`-reusing item in this list. Shipped instead: splits the selected clip into N
    equal-timeline-duration pieces (reusing the already-correct, already-tested
    `Track::split_clip_at`, unchanged) and assigns each piece a constant `speed_factor`
    linearly interpolated between a start/end speed (reusing the existing field, unchanged) —
    a real, visible "staircase" speed ramp built entirely from primitives that were already
    correct before this item, with zero new avfilter/geq/setpts math to get wrong. Since
    `duration_secs()` depends on `speed_factor`, each piece's `start_secs` is reflowed left to
    right after the speed assignment so the pieces stay contiguous. Triggered via a "Rampa de
    velocidade" submenu in the timeline clip's context menu, with two fixed presets (0.5x→2x
    slow-to-fast, 2x→0.5x fast-to-slow, 4 steps each) rather than a custom-curve dialog — also
    deliberately out of scope for this pass.

    **Follow-up**: a custom start/end speed and step count dialog also ships now, via a
    "Personalizada…" entry in the same context-menu submenu (`App::speed_ramp_dialog` staged
    state, `App::show_speed_ramp_modal`, same `egui::Modal` pattern as `renaming_project`'s own
    dialog). UI-only — `App::apply_speed_ramp_to_selected_clip` itself already accepted
    `start_speed`/`end_speed`/`steps` as parameters before this, so no new avfilter/geq/setpts
    math was needed, none of the sandbox-verification risk the smooth-curve version has.

    **Second follow-up: the smooth continuous-curve version now ships too**, and — unlike when
    this item was first scoped — this development environment turned out to have a working,
    fully-linked FFmpeg build capable of actually rendering and verifying the derivation, not
    just type-checking it. `ClipInstance::speed_ramp_end_factor: Option<f32>` (`None` = plain
    constant `speed_factor`, unchanged) carries the ramp's end speed directly on the clip — no
    splitting needed at all, unlike the stepped mode.
    `keyframe::smooth_speed_ramp_duration_secs`/`smooth_speed_ramp_source_secs_at` solve the
    closed-form integral (`∫ 1/speed(t) dt` for a linear `speed(t)` — a natural-log term) and its
    inverse; `ClipInstance::duration_secs()` and `Track::split_clip_at` (splitting a ramped clip
    now continues the ramp correctly across both halves, sharing the exact speed-at-the-cut
    boundary) both use them. Crosses the FFI boundary as
    `avbridge::ClipSegment::smooth_speed_ramp_end_factor` (`<=0.0` = no ramp, same sentinel
    convention `speed_factor` itself already uses elsewhere); `timeline_export.c`/
    `timeline_export_multi.c` each gained a `build_setpts_str` helper emitting
    `setpts=(K/TB)*log((V0+B*T)/V0)` instead of the old constant `setpts=PTS/speed`, expressed
    entirely in the filter's own runtime `TB`/`T` variables so it's valid regardless of the
    actual stream timebase. Audio has no continuous per-sample tempo-ramp primitive in this
    FFmpeg build (`atempo` takes one fixed parameter, not a `t`-keyed expression) — both files'
    audio tempo command uses the ramp's *average* speed instead, a deliberate, documented
    approximation (exact av-sync isn't achievable here; staying audible and reasonably close
    throughout beats jumping between discrete steps). `timeline_export_multi.c`'s per-frame
    track-0-to-timeline time mapping (`ftl`, used to decide which overlay track is active "right
    now," and progress-callback reporting) is also ramp-aware now
    (`smooth_speed_ramp_timeline_elapsed`); an *overlay*-track clip's own ramp still uses the
    old linear approximation for its end-time/seek math — a real, narrower, documented remaining
    gap (only matters when a ramped clip sits on an overlay track specifically, not the common
    single/background-track case). UI: the existing custom speed-ramp dialog gained a "smooth,
    continuous curve" checkbox — checked calls the new
    `App::apply_smooth_speed_ramp_to_selected_clip` (which just sets the two fields directly, no
    splitting) instead of the stepped path, and hides the now-irrelevant step-count field.
    **Verified for real, not just type-checked**: `keyframe::smooth_speed_ramp_duration_secs`
    cross-checked against brute-force numerical integration (200,000 steps) to a `1e-4` tolerance,
    plus an exact round-trip test through its own inverse function — both run for real in this
    session (`cargo test -p core`, 230+ passing, this machine's FFmpeg/GStreamer toolchain
    actually links here, unlike the more constrained sandbox earlier `ROADMAP.md` entries
    describe); a new `Track::split_clip_at` test confirms ramp continuity across a real split;
    and — the strongest evidence — a new `avbridge` integration test
    (`smooth_speed_ramp_produces_a_real_export_with_the_predicted_duration`) actually encodes a
    ramped segment through the real linked FFmpeg build and asserts the *exported file's own
    probed duration* matches the closed-form prediction to within a loose bound — proof the
    whole Rust-math-to-C-setpts-expression-to-real-decoder pipeline is correct end to end, not
    just that the strings happen to parse.
30. `[x]` Real-time audio level meter (VU/peak) during playback — `matrix/competitor-parity.md`.
    Present in Premiere (VU meters) and DaVinci (Fairlight LUFS/peak meter). A pad probe on the
    preview audio path (same pattern as the existing keyframe pad-probes, reading instead of
    writing): `Preview::build_metering_audio_sink` wraps the real audio-sink element (both the
    single-clip `playbin` path and the manually-built compositor audio-mix path) in a small
    `audioconvert!capsfilter(F32LE)!sink` bin, forcing a known sample format so a buffer probe
    on the capsfilter's src pad can parse raw f32 bytes directly and compute peak/RMS combined
    across every channel (a flat sequence, not per-channel — matches this item's "small meter
    widget" scope, not a full per-channel Fairlight-style meter). Stored in a shared
    `Arc<Mutex<AudioLevel>>` updated from GStreamer's own streaming thread, read from the UI
    thread via `Preview::current_audio_level()`. UI: a small peak/RMS bar in the Editor preview
    panel's transport row, next to the scopes toggle, with the peak marker turning red above
    0.98 amplitude to flag near-clipping. Verified via a scratch-crate real-execution check
    (this sandbox's `core` test binary can't link — missing `libonnxruntime`): the metering
    probe logic, run against a real audio fixture through a real GStreamer `playbin`, observed
    real nonzero peak/RMS from actual decoded samples — audio decode/preroll works in this
    sandbox, unlike video decode (confirmed separately: the same harness against a video
    fixture failed with a missing-decoder-plugin error, the known pre-existing video-decode gap,
    not a metering bug).
31. `[x]` Audio gain keyframes (volume fade/ramp within one clip, not just a constant
    `gain_db`) — found while surveying what else the existing `Keyframe<T>` infrastructure
    could drive. Same shape as position/scale/rotation/opacity: `gain_keyframes: Vec<Keyframe<
    f32>>` on `ClipInstance`, overriding the constant `gain_db` when non-empty. Export-side,
    FFmpeg's `volume` filter's `eval=frame` expression mode evaluates to a *linear* multiplier
    (not dB), so the dB-space keyframe curve needs `pow(10, X/20)` wrapping before being handed
    to `volume=<expr>:eval=frame` — verified against FFmpeg's own filter docs, not assumed.
    Crosses the `avbridge` FFI boundary (a new expression-string field on `AudioSegment`/
    `RawAudioSegment`, and an `audio_mix.c` branch alongside the existing literal-`%.6fdB` path).
    Verified: `gain_filter_db_expr`'s unit tests run for real in a scratch crate; a real
    `avbridge` integration test exercises the new `av_asprintf`-built `volume=<expr>:eval=frame`
    path end-to-end against a real FFmpeg filter graph (`avfilter_graph_config` succeeding is
    proof the expression syntax is valid, not just that the C compiles).
32. `[x]` Color grading keyframes (brightness/contrast/saturation ramping over a clip, not a
    constant value) — `ClipInstance::brightness_keyframes`/`contrast_keyframes`/
    `saturation_keyframes`, each independently overriding its own constant field when non-empty
    (same relationship `gain_keyframes` has with `gain_db`). `keyframe::color_balance_filter_expr`
    builds the combined `eq=brightness=...:contrast=...:saturation=...[:eval=frame]` stage —
    `eval=frame` only appended when at least one axis is actually animated, each un-animated axis
    still using its own plain constant. Spliced into `keyframe_video_filter_chain` (alongside
    scale/rotation/opacity) rather than `video_filter_chain`'s own static `eq` stage, which is
    now suppressed whenever any color-grading keyframe list is non-empty (`has_color_keyframes`)
    to avoid double-emitting. **Known caveat, documented in code**: this moves the animated `eq`
    stage to the *front* of the per-clip filter chain instead of its usual post-crop/deflicker/
    stabilization spot — a clip combining color-grading keyframes with crop/deflicker/
    stabilization sees color grading applied to the pre-crop/pre-deflicker/pre-stabilization
    frame. Preview-side piggybacking on `Preview::set_live_balance` (item 3's follow-up) is not
    done — export only, same "export first" shape every other keyframe field started with.
    Verified: `color_balance_filter_expr`'s 3 new unit tests run for real in the same
    `keyframe.rs` scratch crate as `gain_filter_db_expr`'s (32/32 passing); new `video_filter_chain`/
    `split_clip_at` tests in `timeline_test.rs`.
33. `[x]` Crop/pan keyframes (`crop_x`/`crop_y`/`crop_w`/`crop_h` animated over a clip, e.g. a
    slow pan/reveal independent of the existing `scale_keyframes` symmetric zoom) —
    `ClipInstance::crop_x_keyframes`/`crop_y_keyframes`/`crop_w_keyframes`/`crop_h_keyframes`,
    each independently overriding its own constant field when non-empty. `keyframe::
    crop_filter_expr` generalizes `scale_filter_expr`'s single-axis `geq`-based per-pixel inverse
    sample (chosen over `crop`+`eval=frame` for the same real heap-corruption reason, see
    CLAUDE.md) to four independent axes — the sampled window's x/y/width/height — so a moving/
    resizing crop rectangle animates without the frame's own resolution changing frame-to-frame.
    Spliced into `keyframe_video_filter_chain` *first* (before scale/rotation/opacity/color-
    balance — crop reframes the source before those geometric/color stages operate on it, mirroring
    the static `crop` stage's own traditional first-in-chain position) rather than `video_filter_
    chain`'s own static `crop` stage, which `has_crop_keyframes()` now suppresses to avoid
    double-emitting. Wired into `Track::split_clip_at`. Not yet wired into live preview.
    Verified: `crop_filter_expr`'s 3 new unit tests run for real in the same `keyframe.rs`
    scratch crate as the other keyframe expression builders' (35/35 passing); new
    `video_filter_chain`/`split_clip_at` tests in `timeline_test.rs`.
34. `[x]` Text/shape clip animation keyframes — `TextClip`/`ShapeClip` had *zero* keyframe fields
    (position/scale/rotation/opacity keyframes only existed on `ClipInstance` before this), so
    this was a structural gap, not a one-field addition. **Partial, now covers all of
    `ShapeClip`**: `center_x_keyframes`/`center_y_keyframes` (position) and, in a follow-up
    pass, `width_keyframes`/`height_keyframes`/`rotation_keyframes` (size/rotation) all ship —
    `ShapeClip`'s export path (a self-contained `geq` filter expression built entirely in Rust,
    `crate::shape_render`) turned out to already support a `T`-keyed per-pixel expression with
    zero FFI/C changes (`geq` natively exposes `T`, elapsed seconds, per pixel — confirmed
    against FFmpeg's own filter docs, not assumed). `keyframe::shape_axis_expr` offsets `T` by
    the shape's own `start_secs` (the overlay is composited onto the already-exported full video
    in a post-pass, not inside a per-clip filter chain with its own PTS reset, so `T` is
    timeline-*absolute*, unlike every other `*_filter_expr` builder in this module — a real,
    documented difference). The size/rotation follow-up reworked `shape_render::inside_expr`'s
    ellipse/polygon geometry math to accept `geq`-expression-language sub-expressions for the
    half-extents instead of literal `f64`s — the multiply-through-avoid-division trick the
    static case always used generalizes verbatim, so an unkeyframed width/height still
    degenerates to the same plain numeric literal as before. Rotation swaps the old
    Rust-precomputed `sin_a`/`cos_a` literals for `geq`'s own `sin()`/`cos()`/`PI`
    expression-language functions (confirmed present via FFmpeg's `eval.c`-backed docs, not
    assumed), evaluated per pixel instead of once; a scratch-crate numeric check confirmed the
    new per-pixel formula produces the same sin/cos values the old precomputed literals did, at
    several angles, so this is not a behavior change in the unkeyframed (still the common) case.
    Verified: `shape_axis_expr`'s unit tests and `build_shape_filter_desc`/`inside_expr`
    animation tests (position, then size/rotation) all run for real in a scratch crate
    (`keyframe.rs`+`timeline.rs`+`shape_render.rs` have zero heavy deps) — 58/58 passing,
    including every pre-existing `shape_render` test (confirms the already-visually-verified
    static geometry math is unchanged).

    **`TextClip` opacity keyframes also ship**, in a further follow-up: `TextClip`'s export path
    pre-rasterizes a full-canvas PNG per segment with position baked in at generation time, not a
    moving overlay — so position/scale animation stays out of scope (would mean restructuring
    that pipeline toward a small sprite + `overlay=x=<expr>:y=<expr>`, a materially bigger lift).
    Opacity is different: the raster already carries a real alpha channel, so a fade is just an
    alpha *multiplier* applied to the existing pixels, no rasterization change needed.
    `keyframe::text_opacity_alpha_expr` builds the same `T`-keyed-offset-by-`start_secs`
    expression `shape_axis_expr` does; `avbridge_apply_text_overlays` splices a `geq` alpha
    stage between the movie source and the overlay node when a segment's
    `opacity_keyframe_expr` is non-empty. **Real, non-obvious finding from testing this against
    this project's actual linked FFmpeg build** (`avbridge/tests/text_overlay_test.rs`, not just
    reasoning from docs): `geq`'s alpha read-back function is spelled `alpha(X,Y)`, not `a(X,Y)`
    as FFmpeg's own docs otherwise imply — `a(X,Y)` parses as "Unknown function" against the
    real library. A `colorchannelmixer=aa=<expr>:eval=frame` alternative (no per-pixel read-back
    needed at all) was tried first and ruled out the same way: that filter has no `eval` option
    in this build. The full encode still fails in this sandbox with a `Pipeline` error — but
    that reproduces identically on the *unmodified* (no-fade) code path too, confirming it's the
    same pre-existing encoder-availability gap `CLAUDE.md` already documents elsewhere in this
    codebase, not something this change introduced.

    **`TextClip` position keyframes also ship**, in a further follow-up — without the sprite-
    cropping restructuring described above turning out to be necessary. Instead of shrinking the
    raster to a tight sprite, the raster still bakes one constant anchor (the first keyframe's
    value when `pos_x_keyframes`/`pos_y_keyframes` are present, `pos_x`/`pos_y` otherwise — same
    "keyframes win when present" convention `ShapeClip`'s own fields use) exactly as before, and
    `keyframe::text_position_offset_expr` builds the *delta* from that anchor, in canvas pixels,
    as an `overlay=x=<expr>:y=<expr>` fragment — `overlay`'s own expression evaluator uses
    lowercase `t` where `geq`'s (used by the opacity fade above) uses uppercase `T`, same
    timeline-absolute clock, different filter's own variable name. Each axis independently
    overrides its own constant, same as `ShapeClip::center_x_keyframes`/`center_y_keyframes` (two
    separate `f32` lists, not one `Position`-typed list, since this is an absolute `0.0..=1.0`
    anchor fraction, not `ClipInstance::position_keyframes`' `-1.0..=1.0` pan-offset convention).
    Verified: `text_position_offset_expr`'s unit tests run for real in a scratch crate
    (`keyframe.rs` alone has zero heavy deps) — 47/47 passing, including every pre-existing
    `keyframe` test. `avbridge/tests/text_overlay_test.rs` gained a real-FFmpeg-build test for the
    new `overlay=x='...':y='...'` syntax too — same `Err(Pipeline)` encoder-availability gap as
    the opacity test in this sandbox (confirmed by temporarily no-op-stubbing the unrelated
    `av_opt_set_array` symbol this file also can't link against here, per `CLAUDE.md`'s filters.c
    note, discarded before commit), but critically *not* `Err(FilterGraph)` — confirming the new
    overlay x/y expression syntax itself parses against the real linked `avfilter_graph_parse_ptr`,
    identically to the pre-existing opacity/no-fade tests it was verified alongside.

    **`TextClip` scale keyframes also ship**, in a further follow-up. Unlike position, this needed
    no raster-baking change at all: `keyframe::text_scale_sample_exprs` builds a `geq`
    inverse-sample remap (output pixel `(X,Y)` reads the raster's own pixels back from
    `anchor + (X-anchor)/scale(T)` instead of `(X,Y)`) around the same raster-baked position
    anchor position keyframes already use — an inverse *zoom*, not a literal `scale` filter with
    `eval=frame`: letting `scale` renegotiate its own output size every frame is the exact thing
    that reliably corrupted the heap in a real export elsewhere in this codebase (`CLAUDE.md`,
    `timeline_export.c`) — the same reasoning `ClipInstance::scale_keyframes`' own `scale_filter_
    expr` already applies, now reused for `TextClip`. Because the geq remap keeps the raster's own
    canvas-sized frame unchanged throughout, position and scale keyframes compose independently
    with no interaction term needed. `avbridge`'s `text_overlay.c` filter-chain builder was
    restructured from a 2-branch (fade/no-fade) split into an incremental N-stage chain (movie
    source → optional scale-remap geq → optional opacity-fade geq → overlay), extensible for a
    future rotation stage. Verified: `text_scale_sample_exprs`' 6 new unit tests run for real in
    the same `keyframe.rs` scratch crate as the other keyframe expression builders' (53/53
    passing). `avbridge/tests/text_overlay_test.rs` gained a real-FFmpeg-build test for the new
    `geq=r='r(...)':g='g(...)':b='b(...)':a='alpha(...)'` remap syntax; linking that specific test
    binary hits a separate pre-existing sandbox gap (`av_opt_set_array`, a symbol newer than this
    sandbox's packaged FFmpeg — same category as, but distinct from, `filters.c`'s own gap) so it
    could not be executed here, though `cargo check`/`clippy` for the whole workspace stayed clean.

    **`TextClip` rotation keyframes also ship**, completing this item's `TextClip` scope
    (opacity/position/scale/rotation all now animate). `keyframe::text_rotation_sample_exprs`
    builds a `geq` inverse-sample remap around the same raster-baked anchor scale uses, via
    FFmpeg's `sin()`/`cos()` expression-language functions — the same inverse-rotation math
    `shape_render`'s own `ShapeClip` rotation already relies on, just re-added onto the anchor
    instead of compared against half-extents for an inside test. Composes independently of the
    scale remap around that shared anchor (an isotropic scale and a rotation around the same
    center commute algebraically), so the two geq stages' relative order in the filter chain
    doesn't affect the result. Verified: `text_rotation_sample_exprs`' 6 new unit tests run for
    real in the same `keyframe.rs` scratch crate as the other keyframe expression builders'
    (59/59 passing). `avbridge/tests/text_overlay_test.rs` gained a real-FFmpeg-build test for
    the new rotation `geq` remap syntax; same pre-existing `av_opt_set_array` link-time sandbox
    gap as the scale test above kept it from actually running here, though `cargo check`/
    `clippy` for the whole workspace stayed clean.

    The sprite-cropping restructuring floated as a future possibility above (shrinking the
    raster to a tight sprite instead of a full-canvas-sized one) remains genuinely not needed —
    every axis (opacity, position, scale, rotation) shipped without it, purely via geq inverse-
    sampling and overlay-expression deltas — and stays a possible future performance
    optimization rather than a functional gap.

35. `[~]` **Nested sequences / compound clips** (Premiere/DaVinci/FCP) — the scoping pass
    `matrix/competitor-parity.md`'s own note called for. The key insight that made this tractable:
    oca already has independently-editable `Sequence`s (the Editor's own tabs) — a compound clip
    just points a `ClipInstance` at another `Sequence` in the same `Project`
    (`ClipInstance::nested_sequence_id: Option<u64>`) instead of inventing a second sub-timeline
    concept.
    - **Real recursion, not a bolt-on pointer** (the concern this item's original scoping note
      raised): `avcore::nested_sequence::materialize_nested_sequences` actually renders each
      nested sequence to a cached temp file (recursively — a nested sequence's own clips may
      themselves be nested, with cycle detection via a `visiting` set), then hands back a
      synthetic `MediaAsset` per nested clip pointing at that file. Every existing resolution
      function (`resolve_timeline_segments_multi` and friends) then treats a compound clip
      exactly like an ordinary asset-backed one — zero changes to their own logic, and every
      per-clip effect/keyframe still applies on top of the rendered nested content for free,
      since it's the same `ClipInstance`.
    - **Caching**: keyed on the nested sequence's own `Timeline` content (`PartialEq`, no
      hashing) via `ui`'s `App::nested_sequence_render_cache` — mirrors `ExportPreviewCache`'s
      own "value-equality, not a version counter" pattern. `ExportPreviewCache` itself now also
      compares every *other* sequence (not just the active one), since a compound clip's
      rendered content depends on a sequence `tracks`/`media_library` alone can't see edits to.
    - **UI**: "📦 Criar clipe composto" (timeline clip context menu, Video-track clips only,
      single-clip only — multi-selection/composite-group compounding isn't supported yet, a
      real scope cut) moves the selected clip into a fresh `Sequence`'s own new V1 track
      (rebased to start at `0.0`) and replaces it in place with a plain nested-sequence clip.
      Double-click (or "📦 Abrir clipe composto") switches the Editor's active tab into the
      nested sequence — the common "enter the compound clip" affordance every NLE with this
      feature has. The timeline block itself shows the nested sequence's own name (📦 badge) in
      place of a filmstrip/waveform, since a compound clip has no `asset_id`/media-library entry
      to draw one from.
    - **Known, deliberately-not-hidden cost**: unlike an imported asset, a nested sequence's
      rendered file is produced by a real encode, not instant — `ui`'s callers (queueing an
      export, the Fila screen's size-estimate preview, and now `ensure_preview_loaded` too) call
      this synchronously and block until it's done, no background-thread/progress-reporting path
      yet. Paid once per edit to that nested sequence (the cache), but the first hit after an
      edit is a real, currently un-signposted UI hitch for a long nested sequence — the honest
      reason this item is `[~]` not `[x]`.
    - **Follow-up: live preview now works too.** `App::current_preview_clip`/
      `current_preview_overlay_clips`/`current_preview_audio_clips` now take an explicit
      `media_library` slice instead of reading `Project::media_library` directly, so
      `App::ensure_preview_loaded` can merge in
      `materialize_nested_sequences_for_active_sequence`'s synthetic assets before resolving —
      same cache-backed pattern the export path already uses, so scrubbing/playback across a
      compound clip only pays the render cost once per edit to it. Fixed a real bug found while
      wiring this in: `ensure_preview_loaded`'s own cheap per-frame "did anything change" fast
      path (`current_preview_clip_id`/`current_preview_overlay_clip_ids`) still required an
      `asset_id` to resolve in the *plain* `media_library` even for a nested clip (which has no
      real `asset_id` at all) — would have made the fast path see "still unresolved, nothing
      changed" forever and never actually attempt to open a compound clip's pipeline. Two call
      sites that only ever needed clip-level fields (`frozen`, `speed_factor`, `id`), never the
      asset — `toggle_preview_playback`, `seek_preview` — were switched to a new asset-free
      `current_preview_video_clip` instead of threading `media_library` through them for no
      reason.
    - **Explicitly not done**: dragging an *existing* sequence tab onto another timeline as a
      nested clip (only the "create compound from selection" direction ships); deleting a
      `Sequence` still referenced by a compound clip elsewhere leaves a dangling
      `nested_sequence_id` — `RenderError::MissingNestedSequence` degrades gracefully (logged,
      clip skipped) rather than crashing, but there's no warning at delete time.
    - **Verified for real**: `avcore::nested_sequence`'s own test module — cycle detection (both
      direct self-nesting and an indirect A→B→A cycle), a missing-sequence error, and, the
      strongest evidence, an actual end-to-end test that builds a two-sequence project, calls
      `materialize_nested_sequences`, and confirms the rendered output file really exists and
      probes as valid video (a real recursive FFmpeg encode, not just type-checked) — plus a
      cache-reuse test confirming an unchanged nested timeline returns the same cached path
      rather than re-rendering. All run for real in this session (`cargo test -p core`, this
      machine's FFmpeg/GStreamer toolchain actually links here).

## P5 — Competitive Product Growth

The consolidated 2026-08-28 survey found that oca's largest remaining gaps are workflow and
interchange gaps, not basic timeline tools. Read
[architecture/competitive-feature-plan.md](architecture/competitive-feature-plan.md) for the
competitive evidence, implementation slices, acceptance criteria, security requirements, and
deliberate non-goals. Implement in this order unless an active production problem justifies moving
an item earlier:

- `[ ]` **CF-01: transcript-based editing and speech cleanup.** Reuse Whisper word timings to
  search, seek, propose filler-word/retake removals, and apply reviewed cuts as one undo action.
- `[ ]` **CF-02: gameplay event ingestion and watched-folder import.** Import versioned event
  sidecars/bookmarks and combine them with audio-spike scoring before building a full recorder.
- `[ ]` **CF-03: integrated gameplay-voice cleanup.** Move the proven watched-folder FFmpeg chain
  into a non-destructive `Mic`-role effect with A/B preview and measured output.
- `[ ]` **CF-04: dynamic auto-reframe.** Track a face/selected subject and generate reviewed,
  smoothed crop/position keyframes for vertical exports and Shorts Pack.
- `[ ]` **CF-05: OpenTimelineIO interchange.** Round-trip the supported editorial subset and
  emit an explicit compatibility report for unsupported effects.
- `[ ]` **CF-06: live multicam monitor.** Show synchronized proxy-backed feeds and materialize
  angle decisions through the existing ordinary clip-split representation.
- `[ ]` **CF-07: parameterized motion-graphics templates.** Add a declarative, script-free,
  versioned asset format for reusable channel graphics and aspect-ratio variants.
- `[ ]` **CF-08: semantic transcript and visual search.** Build a bounded, versioned local index
  after exact transcript search ships in CF-01.
- `[ ]` **CF-09: arbitrary-object mask and tracking.** Start with a user-seeded local model and
  privacy blur, reusing the existing matte/model/tracker infrastructure.
- `[ ]` **CF-10: direct publishing and review collaboration.** Start with a secure YouTube upload
  flow; keep OAuth credentials in the OS vault and cloud review separate from offline bundles.

Quick wins that may be completed alongside CF-01:

- `[x]` **Marker ruler rendering/snap.** Every marker now draws as a small color-coded (by
  `MarkerKind`) triangle at the top of the timeline ruler (`timeline_panel::draw::
  draw_marker_ticks`), click-to-seek like the Timeline Index panel's own rows, and is a
  magnetic-snap target for both the ruler's own playhead drag and every per-clip trim/move drag
  — closing the gap P0 item 2's own doc comment flagged ("Timeline markers aren't a snap
  target — no markers feature exists yet... revisit when it lands"), now that P2 item 9 shipped
  markers. Pure UI wiring reusing already-tested `snap_to_nearest`/`snap_move_start` — no new
  pure-function surface needed a unit test of its own.
- `[ ]` Stabilization and deflicker preview parity.
- `[ ]` Real-hardware GPU encoder validation.
- `[ ]` Integrating the standalone watched-folder utility into the app.

## P6 — Explicitly Deferred

Real ideas, deliberately not queued — revisit only if a P3 differentiator proves the audience
wants more in this direction. See `architecture/differentiators.md`'s "out of scope" section
and `matrix/competitor-parity.md`'s "deliberately not adopted" section.

23. `[ ]` Voice-clone TTS beyond the single bundled Piper voice — still deferred until CF-01 and
    CF-03 ship, and must require explicit consent plus deletion of derived voice artifacts.
24. `[ ]` Distributed/render-farm export.

Motion-graphics templates and real-time AI object masking moved from this section to CF-07 and
CF-09 respectively. The competitive refresh found concrete gameplay/channel use cases for both,
but they remain behind the higher-impact CF-01-CF-06 workflow items.

---

[← back to spec/INDEX.md](INDEX.md)
