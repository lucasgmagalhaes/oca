// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::collections::HashMap;

use avcore::MediaAsset;
use eframe::egui;

use crate::app::thumbnail_frame_index;
use crate::theme;

pub(super) fn visible_tile_range(
    clip_left: f32,
    clip_right: f32,
    visible_left: f32,
    visible_right: f32,
    tile_width: f32,
) -> std::ops::Range<usize> {
    if tile_width <= 0.0 || clip_right <= clip_left || visible_right <= visible_left {
        return 0..0;
    }
    let first = ((visible_left - clip_left) / tile_width).floor().max(0.0) as usize;
    let end = ((visible_right.min(clip_right) - clip_left) / tile_width)
        .ceil()
        .max(first as f32) as usize;
    first..end
}

pub(super) fn filmstrip_frame_index_for_tile(
    source_in_secs: f64,
    tile_center_offset_px: f32,
    px_per_sec: f32,
    fps: Option<f32>,
) -> i64 {
    let time_in_source = source_in_secs + (tile_center_offset_px / px_per_sec) as f64;
    thumbnail_frame_index(time_in_source, fps)
}

/// Draws aspect-ratio-sized tiles only where `clip_rect` intersects the painter's visible clip
/// rect. Every tile samples the source timestamp under its center, quantized to the asset's
/// frame rate by [`thumbnail_frame_index`]: increasing timeline zoom shrinks the source-time
/// distance between adjacent tiles and therefore reveals denser, distinct frames; zooming out
/// spaces them farther apart. Tile columns remain anchored to the full clip rect so vertical
/// scrolling/repaint clipping never changes which frame a given column represents.
/// `(project_id, asset_id, frame_index)` — matches `crate::app::ThumbnailKey`. Spelled out here
/// rather than importing that alias since `project_id` disambiguates asset ids that are only
/// unique *within* a project (`MediaAsset::id` is assigned per-project, starting from 1 in each
/// one), not globally — without it, two different projects' assets sharing an id would collide
/// in the shared `thumbnail_textures` cache.
type ThumbnailKey = (u64, u64, i64);

pub(super) struct ThumbnailDrawWork<'a> {
    pub(super) requests: &'a mut Vec<ThumbnailKey>,
    pub(super) touches: &'a mut Vec<ThumbnailKey>,
}

pub(super) fn draw_filmstrip(
    thumbnail_textures: &HashMap<ThumbnailKey, egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    project_id: u64,
    asset: &MediaAsset,
    source_in_secs: f64,
    px_per_sec: f32,
    thumbnail_work: &mut ThumbnailDrawWork<'_>,
) {
    let Some((res_w, res_h)) = asset.resolution else {
        return;
    };
    if res_h == 0 {
        return;
    }
    let tile_height = clip_rect.height();
    let tile_width = (tile_height * res_w as f32 / res_h as f32).max(1.0);
    let visible_rect = clip_rect.intersect(painter.clip_rect());
    if !visible_rect.is_positive() || px_per_sec <= 0.0 {
        return;
    }

    let tile_range = visible_tile_range(
        clip_rect.left(),
        clip_rect.right(),
        visible_rect.left(),
        visible_rect.right(),
        tile_width,
    );
    for tile_index in tile_range {
        let x = clip_rect.left() + tile_index as f32 * tile_width;
        let w = tile_width.min(clip_rect.right() - x);
        let tile_rect =
            egui::Rect::from_min_size(egui::pos2(x, clip_rect.top()), egui::vec2(w, tile_height));
        let tile_center_offset_px = x + w / 2.0 - clip_rect.left();
        let frame_index = filmstrip_frame_index_for_tile(
            source_in_secs,
            tile_center_offset_px,
            px_per_sec,
            asset.fps,
        );
        let key = (project_id, asset.id, frame_index);
        match thumbnail_textures.get(&key) {
            Some(texture) => {
                thumbnail_work.touches.push(key);
                painter.image(
                    texture.id(),
                    tile_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            None => thumbnail_work.requests.push(key),
        }
    }
}

/// Draws a single poster frame — the one at `source_in_secs`, the frame a frozen block
/// ([`avcore::timeline::ClipInstance::frozen`]) holds per `request.md`'s Fase 4 "Congelar"
/// spec — tiled across all of `clip_rect`, plus a small "❄" badge marking the block as frozen
/// even at a zoom level too tight to tell a still poster from a real filmstrip. Reuses
/// `draw_filmstrip`'s texture cache/request plumbing, just pinned to the exact held source frame
/// instead of sampling one frame per column. Only visible columns are iterated.
pub(super) fn draw_frozen_poster(
    thumbnail_textures: &HashMap<ThumbnailKey, egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    project_id: u64,
    asset: &MediaAsset,
    source_in_secs: f64,
    thumbnail_work: &mut ThumbnailDrawWork<'_>,
) {
    let Some((res_w, res_h)) = asset.resolution else {
        return;
    };
    if res_h == 0 {
        return;
    }
    let tile_height = clip_rect.height();
    let tile_width = (tile_height * res_w as f32 / res_h as f32).max(1.0);
    let visible_rect = clip_rect.intersect(painter.clip_rect());
    if !visible_rect.is_positive() {
        return;
    }
    let frame_index = thumbnail_frame_index(source_in_secs, asset.fps);
    let key = (project_id, asset.id, frame_index);

    match thumbnail_textures.get(&key) {
        Some(texture) => {
            thumbnail_work.touches.push(key);
            let tile_range = visible_tile_range(
                clip_rect.left(),
                clip_rect.right(),
                visible_rect.left(),
                visible_rect.right(),
                tile_width,
            );
            for tile_index in tile_range {
                let x = clip_rect.left() + tile_index as f32 * tile_width;
                let w = tile_width.min(clip_rect.right() - x);
                let tile_rect = egui::Rect::from_min_size(
                    egui::pos2(x, clip_rect.top()),
                    egui::vec2(w, tile_height),
                );
                painter.image(
                    texture.id(),
                    tile_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
        }
        None => thumbnail_work.requests.push(key),
    }

    // "F" (ASCII), not "❄" -- same tofu class as this file's other single-letter clip badges
    // (C/O/#/H for cropped/masked/flipped/etc.).
    painter.text(
        clip_rect.left_top() + egui::vec2(3.0, 2.0),
        egui::Align2::LEFT_TOP,
        "F",
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );
}

/// Draws one vertical min/max bar per horizontal pixel of `clip_rect`, resampling `peaks`
/// (the asset's full-duration waveform, see [`avcore::waveform`]) down to whatever's visible
/// between `source_in_secs` and `source_out_secs` — the same "fixed-resolution source data
/// resampled to the current on-screen width" idea as `draw_filmstrip`'s zoom handling, just
/// per-column instead of per-tile. A no-op if `peaks` is empty or `asset_duration_secs`
/// is non-positive (shouldn't happen for a real decoded asset, but guards div-by-zero).
pub(super) fn draw_waveform(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    peaks: &[(f32, f32)],
    asset_duration_secs: f64,
    source_range_secs: std::ops::Range<f64>,
    gain_linear: f32,
    color: egui::Color32,
) {
    if peaks.is_empty() || asset_duration_secs <= 0.0 {
        return;
    }
    let bucket_count = peaks.len() as f64;
    let start_bucket = (source_range_secs.start / asset_duration_secs * bucket_count)
        .clamp(0.0, bucket_count - 1.0);
    let end_bucket =
        (source_range_secs.end / asset_duration_secs * bucket_count).clamp(0.0, bucket_count);
    let bucket_span = (end_bucket - start_bucket).max(1.0);

    let mid_y = clip_rect.center().y;
    let half_h = clip_rect.height() / 2.0 - 1.0;
    let width_px = clip_rect.width().max(1.0) as usize;

    for x in 0..width_px {
        let t = x as f64 / width_px as f64;
        let bucket = ((start_bucket + t * bucket_span) as usize).min(peaks.len() - 1);
        let (min, max) = peaks[bucket];
        // Live gain preview (Fase 4 "ganho de volume por bloco") — bars scale with the block's
        // gain, clamped so extreme gain doesn't paint outside the clip's own row.
        let min = (min * gain_linear).clamp(-1.0, 1.0);
        let max = (max * gain_linear).clamp(-1.0, 1.0);
        let px = clip_rect.left() + x as f32;
        painter.line_segment(
            [
                egui::pos2(px, mid_y - max * half_h),
                egui::pos2(px, mid_y - min * half_h),
            ],
            egui::Stroke::new(1.0, color),
        );
    }
}

/// Small diamond marker for every distinct `time_fraction` used across `clip`'s five keyframe
/// lists (position/scale/rotation/opacity/gain — see [`avcore::keyframe`]), along the clip
/// block's bottom edge. `gain_keyframes` is the one of these that's ever populated on an audio
/// clip (the other four are video-transform fields) — drawing it here, on top of
/// [`draw_waveform`]'s volume-envelope line, is what the OCA mockup's "white diamond keyframe
/// markers on a volume envelope line" region maps to (`spec/architecture/
/// editor-ui-visual-redesign.md`'s Timeline section) — the data already existed, this is the
/// first place it's drawn directly on the timeline strip rather than only in the properties
/// panel's keyframe editor. Read-only: this pass covers visibility only, not on-timeline add/
/// drag/delete — editing keyframes still goes through the properties panel's list editor. A
/// no-op if the clip has no keyframes on any of the five properties (the common case).
pub(super) fn draw_keyframe_markers(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    clip: &avcore::timeline::ClipInstance,
) {
    if !clip.has_position_keyframes()
        && !clip.has_scale_keyframes()
        && !clip.has_rotation_keyframes()
        && !clip.has_opacity_keyframes()
        && !clip.has_gain_keyframes()
    {
        return;
    }
    let mut fractions: Vec<f32> = clip
        .position_keyframes
        .iter()
        .map(|k| k.time_fraction)
        .chain(clip.scale_keyframes.iter().map(|k| k.time_fraction))
        .chain(clip.rotation_keyframes.iter().map(|k| k.time_fraction))
        .chain(clip.opacity_keyframes.iter().map(|k| k.time_fraction))
        .chain(clip.gain_keyframes.iter().map(|k| k.time_fraction))
        .collect();
    fractions.sort_by(|a, b| a.total_cmp(b));
    fractions.dedup_by(|a, b| (*a - *b).abs() < 1e-3);

    let y = clip_rect.bottom() - 5.0;
    let r = 3.0;
    for frac in fractions {
        let x = clip_rect.left() + frac.clamp(0.0, 1.0) * clip_rect.width();
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(x, y - r),
                egui::pos2(x + r, y),
                egui::pos2(x, y + r),
                egui::pos2(x - r, y),
            ],
            theme::ACCENT_2,
            egui::Stroke::new(1.0, theme::TEXT_PRIMARY.gamma_multiply(0.6)),
        ));
    }
}

/// Draws a filled wedge over `clip_rect`'s incoming (left) edge when
/// [`avcore::timeline::ClipInstance::has_transition`] is true — the OCA mockup's "purple
/// transition wedges between adjacent clips" (`spec/architecture/editor-ui-visual-redesign.md`'s
/// Timeline section). `ClipInstance::transition_in`/`transition_duration_secs` already apply at
/// render/preview time (see `matrix/effects-and-color.md`); this is the first place the timeline
/// strip itself paints anything for it, purely visual (no new interaction). No pixel-sampled
/// color exists for this region in the source mockup (unlike `theme::ACCENT`/`AUDIO_TINT`), so
/// this reuses `theme::ACCENT_2` (already this codebase's keyframe-diamond/composite-border
/// color) rather than fabricating a new token for an unsampled color. A no-op for a clip whose
/// transition duration doesn't cover any on-screen width (`px_per_sec` too small) or that has no
/// transition configured.
pub(super) fn draw_transition_wedge(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    clip: &avcore::timeline::ClipInstance,
    px_per_sec: f32,
) {
    if !clip.has_transition() {
        return;
    }
    let wedge_w = (clip.transition_duration_secs as f32 * px_per_sec)
        .min(clip_rect.width())
        .max(1.0);
    // Full clip height at the incoming edge, tapering to a point `wedge_w` in — the classic
    // NLE "bowtie" transition wedge shape (each side of a cut draws its own half; this is the
    // incoming clip's half).
    painter.add(egui::Shape::convex_polygon(
        vec![
            clip_rect.left_top(),
            clip_rect.left_bottom(),
            egui::pos2(clip_rect.left() + wedge_w, clip_rect.center().y),
        ],
        theme::ACCENT_2.gamma_multiply(0.55),
        egui::Stroke::NONE,
    ));
}

/// Draws the playhead as a vertical line inside `rect`, if it falls within `rect`'s horizontal
/// span — used both on the ruler strip and on every track row so it reads as one continuous
/// line down the timeline despite each row being drawn separately.
/// A short, locale-neutral color for one [`avcore::MarkerKind`] — [`draw_marker_ticks`]'s own
/// ruler tick. Distinct from `App::modals`'s `marker_kind_icon` (a text glyph for the Timeline
/// Index panel's list rows, a different context) — this needs a flat color a filled triangle can
/// use, not a glyph.
fn marker_kind_color(kind: avcore::MarkerKind) -> egui::Color32 {
    match kind {
        avcore::MarkerKind::Standard => theme::ACCENT_2,
        avcore::MarkerKind::ToDo => theme::ERROR,
        avcore::MarkerKind::Chapter => theme::ACCENT,
        avcore::MarkerKind::Highlight => egui::Color32::from_rgb(0xe6, 0xc8, 0x38),
    }
}

/// Renders every marker in `markers` as a small filled triangle at the top of the ruler `rect`,
/// colored by [`avcore::MarkerKind`] ([`marker_kind_color`]) — per
/// `spec/architecture/competitive-feature-plan.md`'s "render timeline markers on the ruler"
/// quick win (P2 item 9's own doc comment left this as a known gap: markers existed with no
/// ruler tick and weren't a snap target — the snap-target half is a separate change in the
/// caller, this covers the visual half). Markers outside `rect`'s visible x-range are skipped
/// rather than drawn off-screen. Clicking a tick seeks the playhead there, mirroring the
/// Timeline Index panel's own click-to-seek.
pub(super) fn draw_marker_ticks(
    ui: &egui::Ui,
    rect: egui::Rect,
    markers: &[avcore::timeline::Marker],
    px_per_sec: f32,
) -> Option<f64> {
    let mut seek_to = None;
    for marker in markers {
        let x = rect.left() + marker.position_secs as f32 * px_per_sec;
        if x < rect.left() - 4.0 || x > rect.right() + 4.0 {
            continue;
        }
        let tick_rect =
            egui::Rect::from_center_size(egui::pos2(x, rect.top() + 4.0), egui::vec2(8.0, 8.0));
        let color = marker_kind_color(marker.kind);
        ui.painter().add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(x - 4.0, rect.top()),
                egui::pos2(x + 4.0, rect.top()),
                egui::pos2(x, rect.top() + 8.0),
            ],
            color,
            egui::Stroke::NONE,
        ));
        let response = ui.interact(
            tick_rect,
            ui.id().with(("timeline_marker_tick", marker.id)),
            egui::Sense::click(),
        );
        if response.clicked() {
            seek_to = Some(marker.position_secs);
        }
        response.on_hover_text(marker.label.clone());
    }
    seek_to
}

/// Periodic timecode labels along the ruler ("00:00, 00:30, 01:00, ..."), confirmed missing
/// against the Timeline mockup via a real screenshot — the ruler previously painted nothing but
/// its background fill, marker ticks, and the playhead, no actual time grid at all. Picks a
/// "nice" interval from a fixed candidate list so labels land at least `MIN_LABEL_SPACING_PX`
/// apart regardless of zoom, rather than a fixed-seconds interval that's either unreadably dense
/// zoomed out or wastefully sparse zoomed in. `rect` is the ruler's full (un-scrolled) content
/// rect — `rect.left()` is timeline `t=0` — so tick x-positions need no separate pan offset;
/// egui's own `ScrollArea` clipping already limits what actually paints to the visible viewport.
pub(super) fn draw_ruler_ticks(painter: &egui::Painter, rect: egui::Rect, px_per_sec: f32) {
    const MIN_LABEL_SPACING_PX: f32 = 70.0;
    const CANDIDATE_INTERVALS_SECS: &[f64] = &[
        1.0, 2.0, 5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1800.0, 3600.0,
    ];
    if px_per_sec <= 0.0 {
        return;
    }
    let interval = CANDIDATE_INTERVALS_SECS
        .iter()
        .copied()
        .find(|secs| *secs as f32 * px_per_sec >= MIN_LABEL_SPACING_PX)
        .unwrap_or(3600.0);
    let last_tick = ((rect.width() / px_per_sec) as f64 / interval).ceil() as i64 + 1;
    for i in 0..=last_tick {
        let secs = i as f64 * interval;
        let x = rect.left() + (secs * px_per_sec as f64) as f32;
        if x > rect.right() + 40.0 {
            break;
        }
        painter.vline(
            x,
            rect.bottom() - 4.0..=rect.bottom(),
            egui::Stroke::new(1.0, theme::BORDER),
        );
        painter.text(
            egui::pos2(x + 3.0, rect.top()),
            egui::Align2::LEFT_TOP,
            avcore::media::format_timecode(secs),
            egui::FontId::monospace(9.0),
            theme::TEXT_MUTED,
        );
    }
}

/// Trim Tool's live readout (`CINECUT_PRODUCT_DECISIONS_v1.0.md` section 5: "while trimming,
/// show ... source timecode; sequence timecode; trim duration"). Drawn floating above-right of
/// the drag pointer so it never sits under the finger/cursor doing the dragging.
pub(super) fn draw_trim_info(
    painter: &egui::Painter,
    anchor: egui::Pos2,
    locale: crate::i18n::Locale,
    source_secs: f64,
    sequence_secs: f64,
    trim_duration_secs: f64,
) {
    use crate::i18n::Text;
    let lines = [
        format!(
            "{}: {}",
            Text::TrimInfoSource.tr(locale),
            avcore::media::format_timecode(source_secs.max(0.0))
        ),
        format!(
            "{}: {}",
            Text::TrimInfoSequence.tr(locale),
            avcore::media::format_timecode(sequence_secs.max(0.0))
        ),
        format!(
            "{}: {}",
            Text::TrimInfoDuration.tr(locale),
            avcore::media::format_timecode(trim_duration_secs.max(0.0))
        ),
    ];
    let font = egui::FontId::monospace(10.0);
    const LINE_HEIGHT: f32 = 13.0;
    const PADDING: f32 = 6.0;
    let width = lines.iter().map(|l| l.len()).max().unwrap_or(0) as f32 * 5.5 + PADDING * 2.0;
    let height = lines.len() as f32 * LINE_HEIGHT + PADDING * 2.0;
    let pos = anchor + egui::vec2(12.0, -height - 12.0);
    let rect = egui::Rect::from_min_size(pos, egui::vec2(width, height));
    painter.rect_filled(
        rect,
        egui::CornerRadius::same(theme::RADIUS_SM),
        theme::SURFACE_2,
    );
    painter.rect_stroke(
        rect,
        egui::CornerRadius::same(theme::RADIUS_SM),
        egui::Stroke::new(1.0, theme::BORDER),
        egui::StrokeKind::Outside,
    );
    for (i, line) in lines.iter().enumerate() {
        painter.text(
            rect.min + egui::vec2(PADDING, PADDING + i as f32 * LINE_HEIGHT),
            egui::Align2::LEFT_TOP,
            line,
            font.clone(),
            theme::TEXT_PRIMARY,
        );
    }
}

pub(super) fn draw_playhead(
    ui: &egui::Ui,
    rect: egui::Rect,
    playhead_secs: f64,
    px_per_sec: f32,
    stroke_width: f32,
) {
    let x = rect.left() + playhead_secs as f32 * px_per_sec;
    if x >= rect.left() && x <= rect.right() {
        // `theme::PLAYHEAD` (`state_playhead`), not `theme::ERROR`/`ACCENT` — a distinct red
        // dedicated to the current-position marker, per `CINECUT_Design_System_v1.0.md`'s own
        // separate `state_playhead`/`state_error` tokens (they happened to share a value in this
        // app's prior palette, but the doc treats them as different roles).
        ui.painter().vline(
            x,
            rect.y_range(),
            egui::Stroke::new(stroke_width, theme::PLAYHEAD),
        );
        // The ruler's playhead (the thicker of the two calls to this function — the per-track
        // row line stays a bare vline) gets a downward-pointing triangle head at its top,
        // matching oca-editor-mock.html's `.playhead::before`.
        if stroke_width > 1.5 {
            let half_w = 5.0;
            let tip = egui::pos2(x, rect.top() + 6.0);
            ui.painter().add(egui::Shape::convex_polygon(
                vec![
                    egui::pos2(x - half_w, rect.top()),
                    egui::pos2(x + half_w, rect.top()),
                    tip,
                ],
                theme::PLAYHEAD,
                egui::Stroke::NONE,
            ));
        }
    }
}

/// A short glyph labeling a shape clip's block on the timeline strip, standing in for the
/// full-fidelity render (which only happens on export, same preview gap as [`avcore::timeline::
/// TextClip`]'s).
pub(super) fn shape_kind_glyph(kind: &avcore::timeline::ShapeKind) -> &'static str {
    // ASCII stand-ins -- "●"/"▲"/"■"/"⬠" are the same tofu class as this module's other fixes.
    match kind {
        avcore::timeline::ShapeKind::Ellipse => "O",
        avcore::timeline::ShapeKind::Polygon(vertices) => match vertices.len() {
            3 => "^",
            4 => "#",
            _ => "P",
        },
    }
}

/// A translucent overlay color hinting at [`avcore::timeline::ClipInstance::color_filter`] on
/// the timeline block — `None` for [`avcore::timeline::ColorFilter::None`] (nothing drawn).
/// Just a preview-panel cue, not the real filtered pixels (see the field's doc comment for the
/// preview/export gap).
pub(super) fn color_filter_tint(filter: avcore::timeline::ColorFilter) -> Option<egui::Color32> {
    match filter {
        avcore::timeline::ColorFilter::None => None,
        avcore::timeline::ColorFilter::BlackAndWhite => {
            Some(egui::Color32::from_rgba_unmultiplied(128, 128, 128, 90))
        }
        avcore::timeline::ColorFilter::Sepia => {
            Some(egui::Color32::from_rgba_unmultiplied(180, 130, 60, 70))
        }
    }
}
