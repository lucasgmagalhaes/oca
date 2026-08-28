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
      (`sysinfo`-backed, 30s interval, own background thread), GPU utilization/VRAM sampling
      (NVIDIA-only, `nvml-wrapper`-backed, own background thread, gracefully absent on a
      non-NVIDIA/no-driver machine — see "Known gaps" below), JSON-lines local file with
      size-based rotation (10 MiB), on-device only, toggleable in Preferences.
      This remains separate from proposed remote error collection: ER-01 does not upload this
      telemetry stream or reuse its free-form error messages; see
      `../architecture/client-error-reporting.md`.
- [x] Shared `avcore::FrameSampler` primitive (`architecture/performance-and-caching.md` §5) —
      replaces four independent open/seek/poll-for-a-decoded-frame loops (`ui`'s auto-reframe,
      motion tracking, background-removal matte generation, and thumbnail extraction).

## Known gaps

- [x] GPU usage telemetry — `sysinfo` has no cross-platform GPU reader, so this went
      vendor-specific: `avcore::GpuSampler` via `nvml-wrapper` (the NVIDIA Management Library).
      `Nvml::init()` dynamically loads `libnvidia-ml.so`/`nvml.dll` at *runtime* (confirmed in a
      throwaway scratch crate: builds and runs cleanly with no NVIDIA GPU/driver present at all,
      same as this sandbox's own dev machine — `Nvml::init()` just returns `Err`, no panic, no
      build-time requirement), so `GpuSampler::new()` returning `None` there is the expected,
      tested outcome, not an unverified path. AMD/Intel GPUs stay out of scope — same
      single-vendor scope decision the GPU encoder ladder already made. **Not verified**: an
      actual non-`None` sample against a real NVIDIA GPU (this sandbox has none) — the
      `GpuSampler::new`/`sample` no-panic contracts and the `TelemetryEvent::GpuUsage`
      serialization shape are the parts confirmed for real, same "implemented, [positive path]
      unverified in this environment" category the GPU encoder ladder (item 19) already carries.
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
      **Follow-up**: blur/sharpen sliders now update live too, via `Preview::set_live_blur` and
      the same `oca_blur_{clip_id}`-named `gaussianblur` element convention — the one other case
      that fits the "single element, once built, stays present across any further change to the
      property that built it" shape color balance already established: `gaussianblur`'s single
      signed `sigma` covers both `blur_intensity` and `sharpen` via one derived `net_sigma`
      (`build_video_filter_bin`'s own formula), so there's no multi-property ambiguity to resolve
      the way color balance's three independent properties had. Wired through the same
      `with_selected_clip_mut` dispatch path. Two negative-path tests mirror `set_live_balance`'s
      own exactly (`set_live_blur_returns_false_when_no_element_was_built`/
      `_for_a_mismatched_clip_id`) — same "type-checked (`cargo check`/`clippy` clean across the
      whole workspace), not run" status, since reproducing `preview.rs`'s full surface in a
      throwaway scratch crate (unlike `set_live_balance`'s own scratch-crate verification, which
      only needed `avbridge` + `gstreamer`, not the two-thousand-line `preview.rs` module and its
      `ClipInstance` struct) wasn't attempted this round.
      **Second follow-up**: chroma-key color/tolerance now update live too, via
      `Preview::set_live_chroma_key` and the `oca_chromakey_{clip_id}`-named `alpha` element
      (`method=custom`) — a third case fitting the same shape, though with one real difference:
      this element is only ever built for a composited-overlay branch gated on
      `chroma_key_enabled`, a plain on/off toggle rather than a gradually-approached intensity —
      the first enable still needs the existing incidental-reopen fallback; only color/tolerance
      edits made after that go live. Two negative-path tests mirror the blur ones exactly, same
      "type-checked, not run in this sandbox" status.
      **Third follow-up**: crop, pixelize, shake, and layer mask all update live now too —
      `Preview::set_live_crop`/`set_live_pixelize`/`set_live_shake`/`set_live_mask`, each using a
      different mechanism suited to what it actually needed (a plain property push for crop, a
      live `capsfilter` caps renegotiation for pixelize, shared atomics a buffer probe already
      reads for shake, an `appsrc`/`imagefreeze(allow-replace=true)` buffer push for mask, mirroring
      `refresh_text_overlay`'s own pattern). See `spec/ROADMAP.md` P1 item 3 for the full writeup.
      Deflicker/stabilization remain the genuinely open gap (temporal, not per-frame); shapes
      (non-text overlays) still have no live-preview path.

---

[← back to spec/INDEX.md](../INDEX.md)
