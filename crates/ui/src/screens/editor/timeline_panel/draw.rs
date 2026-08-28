// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

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
pub(super) struct ThumbnailDrawWork<'a> {
    pub(super) requests: &'a mut Vec<(u64, i64)>,
    pub(super) touches: &'a mut Vec<(u64, i64)>,
}

pub(super) fn draw_filmstrip(
    thumbnail_textures: &HashMap<(u64, i64), egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
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
        let key = (asset.id, frame_index);
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
    thumbnail_textures: &HashMap<(u64, i64), egui::TextureHandle>,
    painter: &egui::Painter,
    clip_rect: egui::Rect,
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
    let key = (asset.id, frame_index);

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

    painter.text(
        clip_rect.left_top() + egui::vec2(3.0, 2.0),
        egui::Align2::LEFT_TOP,
        "❄",
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

/// Small diamond marker for every distinct `time_fraction` used across `clip`'s four keyframe
/// lists (position/scale/rotation/opacity — see [`avcore::keyframe`]), along the clip block's
/// bottom edge. Read-only: this pass covers visibility only, not on-timeline add/drag/delete —
/// editing keyframes still goes through the properties panel's list editor. A no-op if the clip
/// has no keyframes on any property (the common case).
pub(super) fn draw_keyframe_markers(
    painter: &egui::Painter,
    clip_rect: egui::Rect,
    clip: &avcore::timeline::ClipInstance,
) {
    if !clip.has_position_keyframes()
        && !clip.has_scale_keyframes()
        && !clip.has_rotation_keyframes()
        && !clip.has_opacity_keyframes()
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

/// Draws the playhead as a vertical line inside `rect`, if it falls within `rect`'s horizontal
/// span — used both on the ruler strip and on every track row so it reads as one continuous
/// line down the timeline despite each row being drawn separately.
pub(super) fn draw_playhead(
    ui: &egui::Ui,
    rect: egui::Rect,
    playhead_secs: f64,
    px_per_sec: f32,
    stroke_width: f32,
) {
    let x = rect.left() + playhead_secs as f32 * px_per_sec;
    if x >= rect.left() && x <= rect.right() {
        ui.painter().vline(
            x,
            rect.y_range(),
            egui::Stroke::new(stroke_width, theme::ACCENT),
        );
    }
}

/// A short glyph labeling a shape clip's block on the timeline strip, standing in for the
/// full-fidelity render (which only happens on export, same preview gap as [`avcore::timeline::
/// TextClip`]'s).
pub(super) fn shape_kind_glyph(kind: &avcore::timeline::ShapeKind) -> &'static str {
    match kind {
        avcore::timeline::ShapeKind::Ellipse => "●",
        avcore::timeline::ShapeKind::Polygon(vertices) => match vertices.len() {
            3 => "▲",
            4 => "■",
            _ => "⬠",
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
