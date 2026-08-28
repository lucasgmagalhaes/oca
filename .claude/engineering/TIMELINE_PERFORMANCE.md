# Timeline Performance

Ground truth against `crates/ui/src/screens/editor/timeline_panel/{mod.rs,draw.rs}` and
`crates/ui/src/app/preview.rs`. This file previously described aspirational, generic performance
rules; it now records what's actually implemented, what's already good, and the one confirmed
real gap — so effort goes toward the real gap instead of re-solving already-solved problems.

## Known gap: no track/clip-level viewport culling

`timeline_panel/mod.rs` iterates every track's every clip (video, text, and shape) and issues a
`painter.rect_filled` (plus label/icon text calls) for each, every frame, regardless of whether
it's scrolled into view. There is no horizontal-visible-time-range or vertical-visible-track-range
filter before these loops. For a long timeline with many clips this is real O(all clips) per-frame
cost — a genuine risk for large projects, not a hypothetical one. **Any change that adds
per-clip work to these loops multiplies this existing cost** — factor that into the estimate, and
consider whether the change is the moment to add culling rather than making the ungated case
worse.

Tile-level virtualization **does** already exist, just scoped inside one clip's filmstrip:
`draw.rs`'s `visible_tile_range` intersects `painter.clip_rect()` to skip off-screen thumbnail
tiles within a single clip. That's a real, working pattern — the missing piece is applying the
same idea one level up, at the track/clip scope, not inventing virtualization from scratch.

## Level of detail

No zoom-dependent LOD simplification exists (clips render the same regardless of `px_per_sec`).
Not currently a confirmed bottleneck — flag as a candidate only if profiling at low zoom on a
large project shows it matters; don't preemptively add LOD complexity without that evidence.

## Caching — already consistent, follow the convention

`ExportPreviewCache` (`ui/src/app/export.rs`) and `NestedSequenceCache`
(`core/src/nested_sequence.rs`) invalidate via `#[derive(PartialEq)]` value-equality against the
relevant domain data, not a dirty flag or version counter — see `engineering/ARCHITECTURE.md`.
Waveform peaks are precomputed once and stored directly on `MediaAsset.waveform_peaks`
(persisted), read straight off the asset by `draw_waveform` — no separate runtime cache needed
there. Thumbnails are cached in `App.thumbnail_state.thumbnail_textures: HashMap<(asset_id,
frame_index), egui::TextureHandle>`, bounded by `THUMBNAIL_CACHE_CAPACITY` with LRU-style
eviction (`evict_thumbnail_textures`) — a real, bounded cache, not unbounded growth.

## Hot path discipline — confirmed good, one exception worth checking

- `pump_preview_frame`'s hot path already avoids cloning the full `ClipInstance`+`MediaAsset`
  (which would include every keyframe `Vec`) via `current_preview_clip_lut_and_vignette()`,
  cloning only a `String` — a deliberate prior optimization. Match this discipline: prefer
  cloning the smallest field actually needed over the whole struct.
- `current_preview_overlay_clips`/`current_preview_audio_clips` (`preview.rs`) still clone a full
  `ClipInstance`+`MediaAsset` per matching overlay/audio track — cost scales with track count.
  Not confirmed as a problem, but worth checking call frequency (every frame during playback vs.
  only on scrub/seek) before adding more per-track work here.
- `preview.rs`'s `current_preview_clip()` (not the `_lut_and_vignette` fast path) clones the
  entire `media_library: Vec<MediaAsset>` at one call site — confirm this only runs on
  media-library mutation / non-hot-path calls before treating it as safe to build on; don't assume
  it's cheap without checking the actual call site first.

## Interaction

Dragging, trimming, and (for markers) ruler clicks already use the lightweight
"collect-during-the-draw-loop, apply-after" pattern — transient `ClipDrag`/`trim_requests`
buffers during the per-frame immutable-borrow loop, committed (with a single
`push_undo_snapshot()`) after it ends. This is the real mechanism behind "commit durable edits at
appropriate boundaries" — extend it for new drag/trim-like interactions rather than mutating
`Project` inside the draw loop (which the borrow checker would reject anyway, since the loop holds
an immutable borrow of the timeline while iterating).

## When adding a new timeline feature

1. Check whether it needs to run inside the existing per-clip/per-track loop (cost multiplies by
   clip/track count, subject to the culling gap above) or can be computed once per frame
   (cheap) or cached (cheapest).
2. If it needs new derived visual data (a new kind of cached texture, a new precomputed array),
   follow the value-equality caching convention, not a dirty flag.
3. If it needs background computation, use the `std::thread::spawn` + mpsc + `pump_*` pattern
   (`engineering/ARCHITECTURE.md`) — never block inside the draw loop.
