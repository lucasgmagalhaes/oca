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
//! full-canvas RGBA buffer — the preview-side counterpart to the export path's `drawtext`/`geq`
//! avfilter passes ([`crate::render::text_clip_to_segments`], [`crate::shape_render`]), which
//! only ever run against an already-encoded file and have no live GStreamer element equivalent
//! ([`crate::preview`]'s doc comment previously listed this as a known gap — see
//! `Preview::open_composited`, which now feeds each buffer this module builds into the
//! compositor as its own static overlay branch).
//!
//! Both clip kinds are static for the clip's whole visible span (no keyframes on either type),
//! so unlike every other preview overlay effect, one RGBA buffer rendered once is enough — no
//! per-buffer pad probe recomputing anything. `imagefreeze` repeats that single buffer for as
//! long as the branch stays open.
//!
//! Text uses `fontdue`'s own layout engine (already a dependency, see [`crate::text_metrics`])
//! rather than the manual baseline math its `Metrics` type alone would require — `Layout` hands
//! back each glyph's top-left pixel position directly under [`fontdue::layout::CoordinateSystem::
//! PositiveYDown`], matching this buffer's row-major top-down layout. Word-highlight timing
//! ([`crate::timeline::TextClip::words`]) isn't rendered here — the base text only, same
//! "approximate, not pixel-perfect" tolerance the rest of `preview` already documents for
//! effects it only partially covers.
//!
//! Shapes mirror [`crate::shape_render::build_shape_filter_desc`]'s per-pixel math term-for-term
//! (rotate into the shape's local frame, then an ellipse quadratic or
//! [`crate::shape_render::point_in_polygon`] ray-cast), just evaluated directly against a pixel
//! buffer in Rust instead of compiled into a `geq` expression string.

use fontdue::layout::{CoordinateSystem, GlyphRasterConfig, Layout, LayoutSettings, TextStyle};

use crate::shape_render::point_in_polygon;
use crate::timeline::{ShapeClip, ShapeKind, TextClip};

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

/// Renders `clip`'s text into a fully transparent `canvas_width`×`canvas_height` RGBA buffer,
/// positioned the same way export's `drawtext` anchors it (`x=w*pos_x:y=h*pos_y`, top-left of
/// the text block) — see `avbridge/csrc/text_overlay.c`. Falls back to an all-transparent buffer
/// if the platform default font can't be loaded, same "degrade rather than fail" shape
/// [`crate::text_metrics::text_width_px`] already has for the same missing-font case.
pub fn render_text_clip_rgba(clip: &TextClip, canvas_width: u32, canvas_height: u32) -> Vec<u8> {
    let mut buf = vec![0u8; canvas_width as usize * canvas_height as usize * 4];
    let Some(font) = crate::text_metrics::default_font() else {
        return buf;
    };

    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    layout.reset(&LayoutSettings {
        x: clip.pos_x * canvas_width as f32,
        y: clip.pos_y * canvas_height as f32,
        ..LayoutSettings::default()
    });
    layout.append(&[font], &TextStyle::new(&clip.text, clip.font_size, 0));

    let [r, g, b, a] = clip.color_rgba;
    for glyph in layout.glyphs() {
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
                put_pixel(
                    &mut buf,
                    canvas_width,
                    canvas_height,
                    glyph.x as i64 + col as i64,
                    glyph.y as i64 + row as i64,
                    [r, g, b, pixel_alpha],
                );
            }
        }
    }
    buf
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

#[cfg(test)]
#[path = "overlay_render/overlay_render_test.rs"]
mod tests;
