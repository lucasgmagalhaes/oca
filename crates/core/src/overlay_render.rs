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

//! Rasterizes a [`crate::timeline::TextClip`] or [`crate::timeline::ShapeClip`] into a single
//! full-canvas RGBA buffer — text export now writes this exact buffer to a temporary PNG while
//! shape export still uses a `geq` avfilter pass ([`crate::render`], [`crate::shape_render`]),
//! only ever run against an already-encoded file and have no live GStreamer element equivalent
//! ([`crate::preview`]'s doc comment previously listed this as a known gap — see
//! `Preview::open_composited`, which now feeds each buffer this module builds into the
//! compositor as its own static overlay branch).
//!
//! Both clip kinds have static geometry for their whole visible span (no keyframes on either
//! type). Shape preview therefore needs one RGBA buffer only. Text preview also starts with one,
//! then replaces it only when playback enters another timed word; `imagefreeze` repeats the
//! latest buffer between boundaries without per-video-frame rasterization.
//!
//! Text uses `fontdue`'s own layout engine (already a dependency, see [`crate::text_metrics`])
//! rather than the manual baseline math its `Metrics` type alone would require — `Layout` hands
//! back each glyph's top-left pixel position directly under [`fontdue::layout::CoordinateSystem::
//! PositiveYDown`], matching this buffer's row-major top-down layout, and wraps at word
//! boundaries once a line would run past the canvas's right edge (`LayoutSettings::max_width`).
//! Word-highlight timing ([`crate::timeline::TextClip::words`]) is rendered through
//! [`render_text_clip_rgba`]'s `local_time_secs` parameter. Preview calls it on word-boundary
//! changes; export emits one precisely timed PNG overlay per word.
//!
//! Shapes mirror [`crate::shape_render::build_shape_filter_desc`]'s per-pixel math term-for-term
//! (rotate into the shape's local frame, then an ellipse quadratic or
//! [`crate::shape_render::point_in_polygon`] ray-cast), just evaluated directly against a pixel
//! buffer in Rust instead of compiled into a `geq` expression string.

use fontdue::layout::{CoordinateSystem, GlyphRasterConfig, Layout, LayoutSettings, TextStyle};

use crate::render::TextSegment;
use crate::shape_render::point_in_polygon;
use crate::timeline::{MaskShape, ShapeClip, ShapeKind, TextClip};

/// Writes `[r, g, b, a]` at `(x, y)` into a `width`×`height` RGBA buffer, `a` already the final
/// (straight, not premultiplied) alpha to store — glyphs/shape pixels don't overlap in practice
/// (distinct characters, one shape per buffer), so a plain overwrite is enough; no need to blend
/// against whatever's already there.
fn put_pixel(buf: &mut [u8], width: u32, height: u32, x: i64, y: i64, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x as u32 >= width || y as u32 >= height {
        return;
    }
    let idx = (y as u32 * width + x as u32) as usize * 4;
    buf[idx..idx + 4].copy_from_slice(&rgba);
}

/// Alpha-composites one straight-alpha RGBA pixel over the existing buffer pixel.
fn blend_pixel(buf: &mut [u8], width: u32, height: u32, x: i64, y: i64, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x as u32 >= width || y as u32 >= height || rgba[3] == 0 {
        return;
    }
    let idx = (y as u32 * width + x as u32) as usize * 4;
    let src_a = rgba[3] as f32 / 255.0;
    let dst_a = buf[idx + 3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    if out_a <= f32::EPSILON {
        return;
    }
    for channel in 0..3 {
        let src = rgba[channel] as f32 / 255.0;
        let dst = buf[idx + channel] as f32 / 255.0;
        let out = (src * src_a + dst * dst_a * (1.0 - src_a)) / out_a;
        buf[idx + channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    buf[idx + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn text_layout(
    font: &fontdue::Font,
    text: &str,
    font_size: f32,
    x: f32,
    y: f32,
    max_width: f32,
) -> Layout<()> {
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings {
        x,
        y,
        max_width: Some(max_width),
        ..LayoutSettings::default()
    });
    layout.append(&[font], &TextStyle::new(text, font_size, 0));
    layout
}

fn draw_laid_out_text(
    buf: &mut [u8],
    font: &fontdue::Font,
    layout: &Layout<()>,
    rgba: [u8; 4],
    glyph_byte_range: Option<[u32; 2]>,
    canvas_width: u32,
    canvas_height: u32,
) {
    let [r, g, b, a] = rgba;
    for glyph in layout.glyphs() {
        if glyph_byte_range.is_some_and(|[start, end]| {
            let offset = glyph.byte_offset as u64;
            offset < start as u64 || offset >= end as u64
        }) {
            continue;
        }
        let (_metrics, coverage) = font.rasterize_config(GlyphRasterConfig {
            glyph_index: glyph.key.glyph_index,
            px: glyph.key.px,
            font_hash: glyph.key.font_hash,
        });
        for row in 0..glyph.height {
            for col in 0..glyph.width {
                let cov = coverage[row * glyph.width + col];
                if cov == 0 {
                    continue;
                }
                let pixel_alpha = (cov as u32 * a as u32 / 255) as u8;
                if pixel_alpha == 0 {
                    continue;
                }
                blend_pixel(
                    buf,
                    canvas_width,
                    canvas_height,
                    glyph.x as i64 + col as i64,
                    glyph.y as i64 + row as i64,
                    [r, g, b, pixel_alpha],
                );
            }
        }
    }
}

fn draw_rounded_background(
    buf: &mut [u8],
    layout: &Layout<()>,
    segment: &TextSegment,
    canvas_width: u32,
    canvas_height: u32,
) {
    let rgba = segment.background_rgba;
    let glyph_byte_range = segment.glyph_byte_range;
    let glyphs: Vec<_> = layout
        .glyphs()
        .iter()
        .filter(|glyph| {
            !glyph_byte_range.is_some_and(|[start, end]| {
                let offset = glyph.byte_offset as u64;
                offset < start as u64 || offset >= end as u64
            })
        })
        .collect();
    if rgba[3] == 0 || glyphs.is_empty() {
        return;
    }
    let min_x = glyphs
        .iter()
        .map(|glyph| glyph.x)
        .fold(f32::INFINITY, f32::min);
    let min_y = glyphs
        .iter()
        .map(|glyph| glyph.y)
        .fold(f32::INFINITY, f32::min);
    let max_x = glyphs
        .iter()
        .map(|glyph| glyph.x + glyph.width as f32)
        .fold(f32::NEG_INFINITY, f32::max);
    let max_y = glyphs
        .iter()
        .map(|glyph| glyph.y + glyph.height as f32)
        .fold(f32::NEG_INFINITY, f32::max);
    let padding = segment.background_padding.max(0.0);
    let left = min_x - padding;
    let top = min_y - padding;
    let right = max_x + padding;
    let bottom = max_y + padding;
    let radius = segment
        .background_corner_radius
        .max(0.0)
        .min((right - left) / 2.0)
        .min((bottom - top) / 2.0);

    let x_start = left.floor().max(0.0) as i64;
    let x_end = right.ceil().min(canvas_width as f32) as i64;
    let y_start = top.floor().max(0.0) as i64;
    let y_end = bottom.ceil().min(canvas_height as f32) as i64;
    for y in y_start..y_end {
        for x in x_start..x_end {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let nearest_x = px.clamp(left + radius, right - radius);
            let nearest_y = py.clamp(top + radius, bottom - radius);
            let dx = px - nearest_x;
            let dy = py - nearest_y;
            if dx * dx + dy * dy <= radius * radius {
                blend_pixel(buf, canvas_width, canvas_height, x, y, rgba);
            }
        }
    }
}

fn draw_text_segment_onto(
    buf: &mut [u8],
    segment: &TextSegment,
    canvas_width: u32,
    canvas_height: u32,
) {
    let Some(font) = crate::text_metrics::bundled_font(segment.font_family, segment.font_style)
    else {
        return;
    };
    let x = segment.pos_x * canvas_width as f32;
    let y = segment.pos_y * canvas_height as f32;
    let max_width = (canvas_width as f32 - x).max(1.0);
    let layout = text_layout(font, &segment.text, segment.font_size, x, y, max_width);
    draw_rounded_background(buf, &layout, segment, canvas_width, canvas_height);
    draw_laid_out_text(
        buf,
        font,
        &layout,
        segment.color_rgba,
        segment.glyph_byte_range,
        canvas_width,
        canvas_height,
    );
}

/// Rasterizes one semantic export segment into a full-canvas transparent RGBA image. Export
/// writes this buffer to a temporary PNG and the native post-pass overlays it, ensuring the
/// exact same font/background implementation as preview.
pub fn render_text_segment_rgba(
    segment: &TextSegment,
    canvas_width: u32,
    canvas_height: u32,
) -> Vec<u8> {
    let mut buf = vec![0u8; canvas_width as usize * canvas_height as usize * 4];
    draw_text_segment_onto(&mut buf, segment, canvas_width, canvas_height);
    buf
}

/// Renders `clip`'s text into a fully transparent `canvas_width`×`canvas_height` RGBA buffer,
/// positioned at `x=w*pos_x:y=h*pos_y` (top-left of the text block). Export writes the same
/// rasterized buffer to PNG before its native overlay pass. Falls back to an all-transparent
/// buffer if a bundled font is unexpectedly corrupt.
///
/// Wraps at word boundaries once a line would run past the canvas's right edge. `max_width` is
/// the space between the text's own left anchor
/// and the canvas's right edge (`fontdue::layout::LayoutSettings`'s `x`/`max_width` are
/// independent — `max_width` alone doesn't already account for a nonzero `x`), floored at `1.0`
/// so a clip anchored at or past the right edge still lays out instead of getting a degenerate
/// zero/negative wrap width.
///
/// `local_time_secs` is elapsed time since `clip.start_secs` (not a timeline position) — when
/// [`TextClip::highlight_enabled`] and it falls within some [`crate::timeline::WordTiming`]'s
/// `[start_secs, end_secs)`, that word's glyph range is redrawn on top in
/// `highlight_color_rgba`. The range is filtered from the complete caption's layout, so explicit
/// newlines and automatic wrapping match the base text and export exactly.
pub fn render_text_clip_rgba(
    clip: &TextClip,
    canvas_width: u32,
    canvas_height: u32,
    local_time_secs: f64,
) -> Vec<u8> {
    let base = TextSegment {
        start_secs: clip.start_secs,
        duration_secs: clip.duration_secs,
        text: clip.text.clone(),
        font_size: clip.font_size,
        font_family: clip.font_family,
        font_style: clip.font_style,
        color_rgba: clip.color_rgba,
        background_rgba: clip.background_rgba,
        background_padding: clip.background_padding,
        background_corner_radius: clip.background_corner_radius,
        glyph_byte_range: None,
        pos_x: clip.pos_x,
        pos_y: clip.pos_y,
        opacity_keyframe_expr: String::new(),
        position_keyframe_expr_x: String::new(),
        position_keyframe_expr_y: String::new(),
        scale_keyframe_expr_x: String::new(),
        scale_keyframe_expr_y: String::new(),
        rotation_keyframe_expr_x: String::new(),
        rotation_keyframe_expr_y: String::new(),
    };
    let mut buf = render_text_segment_rgba(&base, canvas_width, canvas_height);

    if let Some(index) = active_highlight_word_index(clip, local_time_secs) {
        let words: Vec<&str> = clip.words.iter().map(|w| w.text.as_str()).collect();
        let byte_range = crate::text_metrics::word_byte_ranges(&clip.text, &words)
            .get(index)
            .copied()
            .flatten()
            .and_then(|[start, end]| Some([u32::try_from(start).ok()?, u32::try_from(end).ok()?]));
        let Some(glyph_byte_range) = byte_range else {
            return buf;
        };
        let highlight = TextSegment {
            color_rgba: clip.highlight_color_rgba,
            background_rgba: [0, 0, 0, 0],
            background_padding: 0.0,
            background_corner_radius: 0.0,
            glyph_byte_range: Some(glyph_byte_range),
            ..base
        };
        draw_text_segment_onto(&mut buf, &highlight, canvas_width, canvas_height);
    }

    buf
}

/// Index of the word whose half-open timing window covers `local_time_secs`, or `None` when
/// highlighting is disabled/between words. Shared by the rasterizer and live preview branch so
/// an unchanged word does not trigger another full-canvas RGBA upload every UI frame.
pub fn active_highlight_word_index(clip: &TextClip, local_time_secs: f64) -> Option<usize> {
    clip.highlight_enabled.then(|| {
        clip.words
            .iter()
            .position(|word| local_time_secs >= word.start_secs && local_time_secs < word.end_secs)
    })?
}

/// `(rx/hw)^2 + (ry/hh)^2 <= 1` — the pure-pixel twin of
/// [`crate::shape_render::ellipse_inside_expr`]'s `geq` expression, `rx`/`ry` already in the
/// shape's own unrotated local frame (pixels relative to center).
fn ellipse_inside(rx: f64, ry: f64, w_px: f64, h_px: f64) -> bool {
    let hw = w_px / 2.0;
    let hh = h_px / 2.0;
    if hw <= 0.0 || hh <= 0.0 {
        return false;
    }
    (rx / hw).powi(2) + (ry / hh).powi(2) <= 1.0
}

fn shape_inside(kind: &ShapeKind, rx: f64, ry: f64, w_px: f64, h_px: f64) -> bool {
    match kind {
        ShapeKind::Ellipse => ellipse_inside(rx, ry, w_px, h_px),
        ShapeKind::Polygon(vertices) => {
            let scaled: Vec<(f64, f64)> = vertices
                .iter()
                .map(|&(x, y)| (x as f64 * w_px, y as f64 * h_px))
                .collect();
            point_in_polygon((rx, ry), &scaled)
        }
    }
}

/// Renders `clip`'s shape into a fully transparent `canvas_width`×`canvas_height` RGBA buffer —
/// the pixel-buffer twin of [`crate::shape_render::build_shape_filter_desc`], evaluated directly
/// instead of compiled into a `geq` expression, and iterated only over the shape's own bounding
/// box (padded for rotation) rather than the whole canvas.
pub fn render_shape_clip_rgba(clip: &ShapeClip, canvas_width: u32, canvas_height: u32) -> Vec<u8> {
    let mut buf = vec![0u8; canvas_width as usize * canvas_height as usize * 4];

    let cx = clip.center_x as f64 * canvas_width as f64;
    let cy = clip.center_y as f64 * canvas_height as f64;
    let w_px = clip.width as f64 * canvas_width as f64;
    let h_px = clip.height as f64 * canvas_height as f64;
    let angle = (clip.rotation_deg as f64).to_radians();
    let (sin_a, cos_a) = angle.sin_cos();

    let inner_w = (w_px - 2.0 * clip.stroke_thickness_px as f64).max(0.0);
    let inner_h = (h_px - 2.0 * clip.stroke_thickness_px as f64).max(0.0);
    let has_stroke = clip.stroke_thickness_px > 0.0;

    // Bounding box padded to the diagonal, since a rotated shape's axis-aligned extent can
    // exceed its own unrotated width/height.
    let half_diag = ((w_px / 2.0).powi(2) + (h_px / 2.0).powi(2)).sqrt().ceil();
    let x0 = (cx - half_diag).floor().max(0.0) as u32;
    let x1 = (cx + half_diag).ceil().min(canvas_width as f64) as u32;
    let y0 = (cy - half_diag).floor().max(0.0) as u32;
    let y1 = (cy + half_diag).ceil().min(canvas_height as f64) as u32;

    let [r, g, b, a] = clip.color_rgba;
    for y in y0..y1 {
        for x in x0..x1 {
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - cy;
            let rx = dx * cos_a + dy * sin_a;
            let ry = -dx * sin_a + dy * cos_a;

            let outer = shape_inside(&clip.shape_kind, rx, ry, w_px, h_px);
            let inside = if has_stroke {
                outer && !shape_inside(&clip.shape_kind, rx, ry, inner_w, inner_h)
            } else {
                outer
            };
            if inside {
                put_pixel(
                    &mut buf,
                    canvas_width,
                    canvas_height,
                    x as i64,
                    y as i64,
                    [r, g, b, a],
                );
            }
        }
    }
    buf
}

/// Renders a [`crate::timeline::ClipInstance::mask_shape`] into a `canvas_width`×`canvas_height`
/// single-channel `GRAY8` buffer — `255` inside the shape (visible), `0` outside (masked out) —
/// for [`crate::preview::build_composite_branch`]'s `alphacombine` mask stage, the preview-side
/// counterpart to export's `geq`-based alpha-clipping stage
/// (`ClipInstance::video_filter_chain`'s `mask_shape` block). Mirrors that block's math
/// term-for-term: [`MaskShape::Circle`] is a plain circle inscribed in `min(width, height)`;
/// [`MaskShape::RoundedRect`] is a rounded-rect signed-distance test, `corner_radius` a
/// `0.0..=1.0` fraction of `min(width, height)`. Static — no keyframes on `mask_shape`, so one
/// buffer rendered once (like [`render_text_clip_rgba`]/[`render_shape_clip_rgba`]) covers the
/// clip's whole visible span. Returns an all-zero (fully masked) buffer for
/// [`MaskShape::None`] — callers gate on [`crate::timeline::ClipInstance::is_masked`]
/// before calling this at all, so that case shouldn't be reached in practice.
pub fn render_mask_shape_gray8(
    mask_shape: MaskShape,
    corner_radius: f32,
    canvas_width: u32,
    canvas_height: u32,
) -> Vec<u8> {
    let mut buf = vec![0u8; canvas_width as usize * canvas_height as usize];
    if mask_shape == MaskShape::None {
        return buf;
    }

    let w = canvas_width as f64;
    let h = canvas_height as f64;
    let cx = w / 2.0;
    let cy = h / 2.0;
    let minwh = w.min(h);

    for y in 0..canvas_height {
        for x in 0..canvas_width {
            let dx = x as f64 + 0.5 - cx;
            let dy = y as f64 + 0.5 - cy;
            let inside = match mask_shape {
                MaskShape::None => false,
                MaskShape::Circle => dx * dx + dy * dy <= (minwh / 2.0).powi(2),
                MaskShape::RoundedRect => {
                    let radius = (corner_radius.clamp(0.0, 1.0) as f64 * minwh).min(minwh / 2.0);
                    let outer_x = (dx.abs() - (w / 2.0 - radius)).max(0.0);
                    let outer_y = (dy.abs() - (h / 2.0 - radius)).max(0.0);
                    (outer_x * outer_x + outer_y * outer_y).sqrt() - radius <= 0.0
                }
            };
            if inside {
                let idx = (y * canvas_width + x) as usize;
                buf[idx] = 255;
            }
        }
    }
    buf
}

#[cfg(test)]
#[path = "overlay_render/overlay_render_test.rs"]
mod tests;
