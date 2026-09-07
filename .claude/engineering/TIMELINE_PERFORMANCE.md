# Timeline Performance

Ground truth against `crates/ui/src/screens/editor/timeline_panel/{mod.rs,draw.rs}` and
`crates/ui/src/app/preview.rs`/`preview_selection.rs`. This file previously described
aspirational, generic performance rules; it now records what's actually implemented, what's
already good, and any confirmed real gaps — so effort goes toward a real gap instead of
re-solving an already-solved problem or chasing an unconfirmed one.

## Track/clip-level viewport culling — now implemented

**2026-09-07 correction**: this section previously described an open gap ("no track/clip-level
viewport culling... O(all clips) per-frame cost"). Confirmed by reading the current source
(`timeline_panel/mod.rs`, `shape_overlays.rs`, `text_overlays.rs`) that this has since been
closed — a one-sided horizontal cull (`if x > track_rect.right() && !clip_being_dragged {
continue; }`, right after each clip's left edge `x` is computed) now runs in all three per-track
loops (video clips in `mod.rs`, shape clips in `shape_overlays.rs`, text clips in
`text_overlays.rs`), skipping a clip's paint/interaction/thumbnail cost entirely once it's
scrolled past the visible right edge. One-sided is deliberate and correct, not a partial fix:
the timeline's own horizontal scroll model has no negative offset (only zoom changes what's
visible, per the loop's own comment), so nothing before `x=0` is ever off-screen to begin with —
a two-sided check would just be dead code. A clip currently mid-drag/trim is never culled even
past the visible edge, checked against all three of its own interactive widget ids (body move,
start trim, end trim) — the same loop iteration is what keeps `ui.interact`'s response alive for
an in-progress gesture, so culling it mid-drag would silently abandon the drag rather than just
skip a paint.

Tile-level virtualization inside one clip's own filmstrip (`draw.rs`'s `visible_tile_range`,
intersecting `painter.clip_rect()`) is unrelated and still the right pattern for that narrower
scope — this section previously called it out as the model to extend one level up, which is now
done.

**Still open**: no *vertical* (track-row) culling — every visible track's clips are still culled
horizontally, but a track scrolled out of the panel's own vertical viewport (many tracks in a
tall project) isn't skipped before its row is laid out. Not confirmed as a real cost (egui's own
`ScrollArea` typically skips laying out off-screen rows itself for a plain list, but each track
here is its own nested horizontal `ScrollArea`, which may not get that for free) — a candidate to
check with a real large multi-track project on a real display, not a confirmed gap.

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
- `current_preview_overlay_clips`/`current_preview_audio_clips` (`preview_selection.rs`) still
  clone a full `ClipInstance`+`MediaAsset` per matching overlay/audio track. **2026-09-07: call
  frequency confirmed, not a hot path.** Both are only reached from `App::ensure_preview_loaded`'s
  `if current.is_some()` branch, which itself is gated behind a cheap id-only comparison
  (`current_preview_clip_id()`/`current_preview_*_clip_ids()` against the already-open pipeline's
  ids) that returns early on the common no-op case — "nothing at the playhead changed this frame,"
  true for every frame of uninterrupted playback. The expensive clone path only actually runs when
  the playhead crosses into a different clip set (a cut boundary, a seek, an edit) — inherently
  rare relative to frame rate, not per-frame. Lower priority than this note previously implied;
  still worth the same "clone only the field actually read" treatment eventually
  (`current_preview_clip_lut_and_vignette`'s own precedent), just not urgent.
- `preview_selection.rs`'s `current_preview_clip()` (not the `_lut_and_vignette` fast path) reads
  a caller-supplied `media_library: &[MediaAsset]` slice, not an owned clone — `ensure_preview_
  loaded` builds that slice once per its own (already-rare, see above) invocation by cloning
  `active_project().media_library` and extending it with nested-sequence synthetic assets, not
  once per matching track. Confirmed not a repeated-per-track cost.

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
