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

//! Builds the `geq` avfilter node description for one [`crate::timeline::ShapeClip`] — the
//! Rust-side half of the "Rust builds the filter string, C just chains it" split
//! `avbridge::apply_shape_overlays` uses (see that C function's doc comment), same shape as
//! `ClipInstance::video_filter_chain` for the main export path.
//!
//! **Why `geq` and not a native avfilter primitive:** `drawbox` draws an axis-aligned
//! rectangle but has no rotation option, and no native avfilter draws an arbitrary polygon or
//! an ellipse at all. Every shape is instead tested per-pixel: rotate the pixel into the
//! shape's own local (unrotated) frame, then run an in/out test there — a quadratic test for
//! [`ShapeKind::Ellipse`], or a ray-casting point-in-polygon test (handles concave outlines
//! like the arrow preset, not just convex ones) for [`ShapeKind::Polygon`].
//!
//! **Verification:** the generated `geq` expression was rendered for real — `ffmpeg -f lavfi
//! -i color=... -vf "<generated filter>" -frames:v 1 out.png` — and visually confirmed to
//! produce a correctly positioned, rotated, and colored shape, including the arrow preset's
//! concave notches (which a convex-only in/out test would get wrong). That check also caught
//! two real bugs before they shipped: (1) an early version set `lum`/`cb`/`cr` directly to
//! `r`/`g`/`b`, which isn't how YCbCr works — fixed by [`rgb_to_ycbcr`]; (2) running the geq
//! chain directly against a `yuv420p` frame corrupted color in a way that looked like a
//! geometry bug (a stray block in the frame corner) but was actually `cb`/`cr` being
//! chroma-subsampled to half the luma plane's resolution, so the same `(X,Y)` pixel offsets
//! landed in the wrong place on those planes — fixed by wrapping the chain in
//! `format=yuv444p,...,format=yuv420p` in `avbridge::apply_shape_overlays`. This dev machine's
//! FFmpeg build can't open an *encoder* right now (see `CLAUDE.md`'s GPU encode note), so the
//! full decode-filter-**encode** pipeline inside `apply_shape_overlays` itself is still
//! encode-unverified here — same caveat class as the rest of this codebase's export path — but
//! the filter graph itself (parsing, configuring, and evaluating per-pixel against real frames)
//! is confirmed correct, not just assumed. The underlying geometry (point-in-polygon, ellipse
//! test) is additionally unit tested as pure Rust math, which the generated expression string
//! is built to mirror term-for-term.

use crate::timeline::ShapeKind;

/// Everything [`build_shape_filter_desc`] needs about one shape instance, decoupled from
/// [`crate::timeline::ShapeClip`] so this module doesn't need to depend on the whole timeline
/// type graph (and stays easy to unit test with hand-built fixtures).
pub struct ShapeRenderInput<'a> {
    pub shape_kind: &'a ShapeKind,
    /// Center, as a `0.0..=1.0` fraction of canvas width/height.
    pub center_x: f32,
    pub center_y: f32,
    /// Size, as a `0.0..=1.0` fraction of canvas width/height.
    pub width: f32,
    pub height: f32,
    /// Clockwise rotation around the shape's own center, in degrees.
    pub rotation_deg: f32,
    pub color_rgba: [u8; 4],
    /// `0.0` = filled; `> 0.0` = outline only, that many pixels thick.
    pub stroke_thickness_px: f32,
    pub start_secs: f64,
    pub duration_secs: f64,
    pub canvas_width: u32,
    pub canvas_height: u32,
}

/// Small epsilon added to a ray-casting edge's `y2 - y1` denominator so an exactly-horizontal
/// edge (common in every straight-edged preset — a rectangle's top/bottom, a trapezoid's
/// parallel sides) never divides by exactly zero. The edge's crossing term is already forced to
/// zero by the `y1`/`y2`-straddle test in that case regardless of this term's value, so the
/// epsilon doesn't perturb which pixels count as inside — it only keeps the expression from
/// evaluating a `0/0`.
const RAY_EPSILON: f64 = 1e-6;

/// Builds the complete `geq=lum=...:cb=...:cr=...:enable=...` filter node description for one
/// shape instance — ready to hand across the FFI boundary as [`avbridge::ShapeSegment`]'s
/// `filter_desc`.
pub fn build_shape_filter_desc(input: &ShapeRenderInput) -> String {
    let cx = input.center_x as f64 * input.canvas_width as f64;
    let cy = input.center_y as f64 * input.canvas_height as f64;
    let w_px = input.width as f64 * input.canvas_width as f64;
    let h_px = input.height as f64 * input.canvas_height as f64;
    let angle = (input.rotation_deg as f64).to_radians();
    let (sin_a, cos_a) = angle.sin_cos();

    // Pixel coordinate (X,Y), rotated by -angle around the shape's center — so RX/RY are the
    // pixel's position in the shape's own unrotated local frame, in pixels relative to center.
    let rx = format!("((X-{cx:.4})*{cos_a:.6}+(Y-{cy:.4})*{sin_a:.6})");
    let ry = format!("(-(X-{cx:.4})*{sin_a:.6}+(Y-{cy:.4})*{cos_a:.6})");

    let outer = inside_expr(input.shape_kind, &rx, &ry, w_px, h_px);
    let inside = if input.stroke_thickness_px > 0.0 {
        // Approximate an outline by shrinking the shape toward its own center by the stroke
        // thickness on each axis — exact for the ellipse's quadratic test, an approximation for
        // polygons (a true inward offset of a concave outline, e.g. the arrow, is a real
        // computational-geometry operation this doesn't attempt).
        let inner_w = (w_px - 2.0 * input.stroke_thickness_px as f64).max(0.0);
        let inner_h = (h_px - 2.0 * input.stroke_thickness_px as f64).max(0.0);
        let inner = inside_expr(input.shape_kind, &rx, &ry, inner_w, inner_h);
        format!("(({outer})*(1-({inner})))")
    } else {
        format!("({outer})")
    };

    let alpha = input.color_rgba[3] as f64 / 255.0;
    let end_secs = input.start_secs + input.duration_secs;
    let start = input.start_secs;
    let (y, cb, cr) = rgb_to_ycbcr(input.color_rgba);

    format!(
        "geq=\
         lum='lum(X\\,Y)*(1-{inside}*{alpha:.4})+{y:.2}*{inside}*{alpha:.4}':\
         cb='cb(X\\,Y)*(1-{inside}*{alpha:.4})+{cb:.2}*{inside}*{alpha:.4}':\
         cr='cr(X\\,Y)*(1-{inside}*{alpha:.4})+{cr:.2}*{inside}*{alpha:.4}':\
         enable='between(t\\,{start:.4}\\,{end_secs:.4})'"
    )
}

/// Converts an `[r, g, b, a]` (`0..=255` each) color to full-range BT.601 `(Y, Cb, Cr)`
/// (`Y` `0.0..=255.0`, `Cb`/`Cr` `0.0..=255.0` centered on `128.0`) — the geq filter's
/// `lum`/`cb`/`cr` channels are luma/chroma, not RGB, so a fill color has to convert before it
/// can be baked into the expression (setting `lum`/`cb`/`cr` directly to `r`/`g`/`b`, which an
/// earlier version of this function did, silently renders the wrong color — confirmed visually:
/// a `[255,0,0]` red rendered as near-white).
fn rgb_to_ycbcr([r, g, b, _]: [u8; 4]) -> (f64, f64, f64) {
    let (r, g, b) = (r as f64, g as f64, b as f64);
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    let cb = -0.168736 * r - 0.331264 * g + 0.5 * b + 128.0;
    let cr = 0.5 * r - 0.418688 * g - 0.081312 * b + 128.0;
    (y, cb, cr)
}

/// Builds the `geq`-expression-language 0/1 "is this pixel inside the shape" test, given
/// already-rotated local-frame pixel coordinate expressions `rx`/`ry` and the shape's own
/// (possibly shrunk, for an outline's inner boundary) pixel width/height.
fn inside_expr(kind: &ShapeKind, rx: &str, ry: &str, w_px: f64, h_px: f64) -> String {
    match kind {
        ShapeKind::Ellipse => ellipse_inside_expr(rx, ry, w_px, h_px),
        ShapeKind::Polygon(vertices) => polygon_inside_expr(rx, ry, vertices, w_px, h_px),
    }
}

/// `(rx/hw)^2 + (ry/hh)^2 <= 1`, multiplied through by `(hw*hh)^2` to avoid dividing by a
/// possibly-zero half-extent. `hw`/`hh` are half-width/half-height (`w_px`/`h_px` are the full
/// extents).
fn ellipse_inside_expr(rx: &str, ry: &str, w_px: f64, h_px: f64) -> String {
    let hw = w_px / 2.0;
    let hh = h_px / 2.0;
    format!(
        "lte({rx}*{rx}*{hh2:.4}+{ry}*{ry}*{hw2:.4}\\,{rhs:.4})",
        hh2 = hh * hh,
        hw2 = hw * hw,
        rhs = hw * hw * hh * hh,
    )
}

/// Ray-casting point-in-polygon test, expressed in `geq`'s expression language: for each edge,
/// a term that's `1` if a ray from `(rx,ry)` in the `+X` direction crosses that edge, `0`
/// otherwise; the point is inside iff the sum of crossing terms is odd (`mod(sum, 2)`). Mirrors
/// [`point_in_polygon`] term-for-term — that function is the unit-tested reference this
/// expression is built to match.
fn polygon_inside_expr(
    rx: &str,
    ry: &str,
    vertices: &[(f32, f32)],
    w_px: f64,
    h_px: f64,
) -> String {
    let px: Vec<(f64, f64)> = vertices
        .iter()
        .map(|&(x, y)| (x as f64 * w_px, y as f64 * h_px))
        .collect();

    let mut terms = Vec::with_capacity(px.len());
    for i in 0..px.len() {
        let (x1, y1) = px[i];
        let (x2, y2) = px[(i + 1) % px.len()];
        // straddle = 1 if the edge's endpoints are on opposite sides of the ray's y — gt()
        // returns 0/1, so their difference is -1/0/1 and abs() collapses that to 0/1.
        let straddle = format!("abs(gt({y1:.4}\\,{ry})-gt({y2:.4}\\,{ry}))");
        let x_intersect =
            format!("({x1:.4}+({ry}-{y1:.4})/({y2:.4}-{y1:.4}+{RAY_EPSILON})*({x2:.4}-{x1:.4}))");
        terms.push(format!("{straddle}*gt({x_intersect}\\,{rx})"));
    }
    format!("mod({sum}\\,2)", sum = terms.join("+"))
}

/// Pure-Rust reference implementation of the same ray-casting algorithm
/// [`polygon_inside_expr`] encodes as a `geq` expression — the thing that's actually unit
/// tested, since the generated expression string itself can only be checked by eye/by FFmpeg
/// (see this module's verification caveat).
pub fn point_in_polygon(point: (f64, f64), vertices: &[(f64, f64)]) -> bool {
    let (px, py) = point;
    let mut crossings = 0u32;
    for i in 0..vertices.len() {
        let (x1, y1) = vertices[i];
        let (x2, y2) = vertices[(i + 1) % vertices.len()];
        if (y1 > py) != (y2 > py) {
            let x_intersect = x1 + (py - y1) / (y2 - y1 + RAY_EPSILON) * (x2 - x1);
            if x_intersect > px {
                crossings += 1;
            }
        }
    }
    crossings % 2 == 1
}

#[cfg(test)]
#[path = "shape_render/shape_render_test.rs"]
mod tests;
