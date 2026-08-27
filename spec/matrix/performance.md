# Performance

Detail: `matrix/changelog.md` Fase 7. Architecture-level caching/invalidation patterns (not
yet applied here — see the gaps below) live in `architecture/performance-and-caching.md`.

- [x] Editing proxy — user-selectable resolution (360p/480p/720p cap), baked into the proxy
      filename so a preference change can't collide with a stale-resolution proxy. No
      retroactive regeneration: an asset imported before a preference change keeps its old proxy.
- [x] Zoom-adaptive, bounded timeline filmstrip loading — samples visible tiles at the asset's
      own frame rate (not a fixed one-second bucket), ≤16 pending extraction pipelines, 512-entry
      LRU texture cache (~18 MiB worst case), independently bounded failed-key suppression.
- [x] Hardware-accelerated preview decode — see `matrix/preview-pipeline.md`.
- [x] Release build: `opt-level=3`, LTO, symbol stripping.
- [x] Runtime telemetry — import/export duration, preview frame time, CPU/RAM sampling
      (`sysinfo`-backed, 30s interval, own background thread), JSON-lines local file with
      size-based rotation (10 MiB), on-device only, toggleable in Preferences.
- [x] Shared `avcore::FrameSampler` primitive (`architecture/performance-and-caching.md` §5) —
      replaces four independent open/seek/poll-for-a-decoded-frame loops (`ui`'s auto-reframe,
      motion tracking, background-removal matte generation, and thumbnail extraction).

## Known gaps

- [ ] GPU usage telemetry — `sysinfo` has no cross-platform GPU reader; a vendor-specific one
      (NVML/etc.) is hardware-dependent, same "hard wall" class as the GPU encoder ladder.
- [x] Versioned filter-graph cache (`architecture/performance-and-caching.md` §2) for
      `resolve_timeline_segments_multi`/`resolve_audio_segments`. **Confirmed** (2026-08-27):
      the only per-frame caller was `screens::queue::show` — the Fila (export queue) screen's
      header recomputed both, from scratch, every single UI frame the screen was open, just to
      show a file-size estimate (linear filter-chain string building + a linear media-library
      scan per clip). `render.rs`'s own internal caller (`render_export_job`) only runs once per
      queued job, not a hot path. Fixed with `App::resolved_active_sequence_export_preview`
      (`ui/src/app/export.rs`) — a value-equality cache keyed on the active sequence's `id` +
      `timeline.tracks` + the project's `media_library` (not the whole `Sequence`, so
      `export_settings`/`name` edits don't spuriously invalidate it; not `playhead_secs`, so
      scrubbing/playback — continuous per-frame mutation — doesn't either). No global version
      counter threaded through every timeline mutator: `Track`/`ClipInstance`/`MediaAsset` all
      already derive `PartialEq`, so a cache miss can never silently serve stale data the way a
      forgotten `mark_dirty()` call site could — correctness by construction, at the cost of an
      O(clips) equality check per frame instead of an O(1) version-counter compare (still far
      cheaper than the string-building recompute it guards). Covered by 3 new `ui` unit tests
      (`app_test.rs`) verifying a real clip edit is picked up (no stale-cache bug) and a
      playhead-only change is not treated as a cache miss.
- [~] Dirty-flag mutation classification (`architecture/performance-and-caching.md` §1, §6).
      **Confirmed** (2026-08-27): the premise that "every edit forces a full preview-pipeline
      reopen" doesn't hold across the board. `App::move_clip`/`trim_clip_start`/`trim_clip_end`
      and every `set_selected_clip_*` effect setter (`ui/src/app/clip_props.rs` — gain, crop,
      color adjust, blur, chroma key, mask, transitions, keyframes, ...) never call
      `App::invalidate_preview_rendering` at all — `App::ensure_preview_loaded`'s per-frame
      id-diffing already skips a reopen for a position-only drag, matching this pattern's intent.
      The flip side: those effect setters currently have **no live-preview update path either**
      — a `ClipInstance` property baked into `build_video_filter_bin` at pipeline-build time
      (brightness/contrast/crop/blur/etc.) only reflects a new value once something else
      happens to reopen the pipeline (playhead leaving and re-entering the clip, undo/redo,
      sequence switch) — a real gap, but a live-element-property-update mechanism (mirroring
      the keyframe pad-probe technique `build_video_filter_bin` already uses for
      scale/rotation/opacity) is a materially bigger lift than the confirm step here scoped for,
      and is left as a follow-up, not silently claimed fixed.
      The one confirmed *actual* hot-path violation — `properties_panel::text_clip_properties`
      bundling every field (text/font/color/background/position *and* start/duration) behind one
      `changed` flag that called a full `invalidate_preview_rendering()` on every dragged-slider
      frame — is fixed: `start_secs`/`duration_secs` (can change which clip covers the playhead)
      still force a full reopen, everything else now goes through a new
      `Preview::refresh_text_overlay`/`App::refresh_preview_text_content` path that pushes a
      freshly rasterized buffer into the clip's already-open `appsrc` branch instead, reusing
      the existing `update_text_overlays`/`imagefreeze(allow-replace=true)` primitive rather than
      tearing down and rebuilding the whole compositor pipeline. The text color modal's confirm
      step (`app/color.rs`) now goes through the same cheap path. Verified via a new
      `core` integration test (`refresh_text_overlay_redraws_a_content_only_edit_without_
      reopening_the_pipeline`) against the real GStreamer pipeline — **not run in this session**
      (this sandbox's rustc 1.94.1 can't build any workspace crate at all right now — several
      transitive deps in `Cargo.lock` require rustc ≥1.95/1.96 — a pre-existing environment gap,
      not a regression from this change; same "implemented, unverified in this environment"
      category as the FFmpeg/GPU-encoder notes in `CLAUDE.md`).
      **Follow-up (2026-08-27)**: the live-element-property-update mechanism flagged above as a
      follow-up is now partially done, scoped to color balance specifically —
      `Preview::set_live_balance` pushes brightness/contrast/saturation to the already-built
      `videobalance` element by name (`gst::Bin::by_name`, recursive, so it works for both the
      single-clip and composited-overlay pipelines) instead of waiting for an incidental reopen,
      wired from `App`'s shared `with_selected_clip_mut` dispatch path. Deliberately not
      extended to every other effect property: most (blur, crop, pixelize, shake, chroma key,
      mask, ...) only get their GStreamer element built at all once their own intensity/toggle
      first goes non-neutral, so a live update for those needs restructuring the running filter
      graph mid-playback, not just a property push — color balance was the one case where all
      three properties (brightness/contrast/saturation) share a single element that, once built,
      stays present across any further change among the three, making a plain `set_property`
      push safe. Verified against a real GStreamer pipeline in a scratch crate depending on real
      `avbridge` (this sandbox's `core` test binary itself can't link — the ONNX Runtime gap —
      but `avbridge` alone can): the two no-op paths (no element was ever built; a mismatched
      clip id) are confirmed correct for real. The positive path — finding and updating a real
      element, and that doing so changes decoded output — isn't verified: this sandbox's
      `playbin` never actually constructs a working video output branch at all
      (`current_frame()` returns `None` for every clip here, confirmed against an unmodified
      copy of `preview_test.rs`'s own pre-existing `opens_with_hardware_decoding_allowed` test),
      the same class of GUI/hardware-dependent-verification gap as the note directly above.

---

[← back to spec/INDEX.md](../INDEX.md)
