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

//! Motion tracking — per `request.md`'s Fase 4 "Motion tracking": follow a subject's on-screen
//! movement and use it to drive [`crate::timeline::ClipInstance::position_keyframes`], so a
//! layer (e.g. a small webcam corner, or a graphic pinned to something in the footage) rides
//! along with the tracked point instead of staying at one fixed spot for the whole clip.
//!
//! Plain template tracking (fixed-template block matching, full search within a radius each
//! frame) — no ML model, no new dependency, deliberately simple. [`track_region`] is pure and
//! frame-source-agnostic (fed pre-decoded [`GrayFrame`]s), so it's fully unit testable against
//! synthetic frames; the caller (`ui`'s `App::spawn_motion_track_selected_clip`) is what
//! actually decodes real video frames via `crate::preview::Preview`.
//!
//! **Output convention:** [`tracked_positions_to_keyframes`] converts each frame's tracked
//! center into a *delta* from the first frame's tracked center, added to a fixed
//! `base_position` — the produced [`crate::keyframe::Keyframe<crate::keyframe::Position>`] list
//! makes the layer ride along with the tracked motion while keeping its initial on-canvas
//! placement, rather than snapping to wherever the tracked point happens to sit in the source
//! frame (a meaningless canvas position, since the tracked region's coordinates are in *source*
//! frame space, not canvas space).

use crate::keyframe::{Keyframe, Position};

/// A single-channel (luma) decoded frame, at whatever resolution it was extracted at — cheap to
/// track against since block matching only needs intensity, not full RGBA.
#[derive(Debug, Clone)]
pub struct GrayFrame {
    pub width: u32,
    pub height: u32,
    /// Row-major, one byte per pixel.
    pub data: Vec<u8>,
}

/// Converts a decoded RGBA frame (e.g. [`crate::preview::VideoFrame::rgba`]) to a [`GrayFrame`]
/// via the standard luma weights.
pub fn rgba_to_gray(rgba: &[u8], width: u32, height: u32) -> GrayFrame {
    let mut data = vec![0u8; (width * height) as usize];
    for (i, px) in data.iter_mut().enumerate() {
        let base = i * 4;
        let r = rgba[base] as u32;
        let g = rgba[base + 1] as u32;
        let b = rgba[base + 2] as u32;
        *px = ((r * 299 + g * 587 + b * 114) / 1000) as u8;
    }
    GrayFrame {
        width,
        height,
        data,
    }
}

/// The tracked region's center, as a fraction (`0.0..=1.0`) of the frame it was found in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackedPosition {
    pub center_x_frac: f32,
    pub center_y_frac: f32,
}

fn extract_patch(frame: &GrayFrame, x0: i32, y0: i32, width: i32, height: i32) -> Vec<u8> {
    let mut patch = vec![0u8; (width * height) as usize];
    for row in 0..height {
        let src_row = ((y0 + row) as u32 * frame.width) as usize;
        let src_start = src_row + x0 as usize;
        let dst_start = (row * width) as usize;
        patch[dst_start..dst_start + width as usize]
            .copy_from_slice(&frame.data[src_start..src_start + width as usize]);
    }
    patch
}

/// Sum of absolute differences between `template` and the `width`x`height` block of `frame`
/// starting at `(x0, y0)` — lower is a better match. Caller guarantees the block fits inside
/// `frame` (checked by [`track_region`] before calling this).
fn sad(template: &[u8], frame: &GrayFrame, x0: i32, y0: i32, width: i32, height: i32) -> i64 {
    let mut total: i64 = 0;
    for row in 0..height {
        let src_row = ((y0 + row) as u32 * frame.width) as usize;
        let src_start = src_row + x0 as usize;
        let tpl_start = (row * width) as usize;
        for col in 0..width as usize {
            total += (template[tpl_start + col] as i64 - frame.data[src_start + col] as i64).abs();
        }
    }
    total
}

/// Tracks a rectangular region across `frames`, starting centered at
/// `(initial_center_x_frac, initial_center_y_frac)` (a fraction of the first frame's size) in
/// `frames[0]`. `template_width_frac`/`template_height_frac` set the tracked block's width/
/// height independently (both as a fraction of the frame's shorter dimension, so a `1.0` value
/// on either axis still means "as big as the shorter side" — matching the old single-scalar
/// `template_size_frac`'s convention exactly when both are equal, e.g. `track_region(...,
/// 0.2, 0.2, ...)` behaves identically to the old `track_region(..., 0.2, ...)`);
/// `search_radius_frac` bounds how far (as the same fraction) the block is allowed to move
/// between consecutive frames — a full search within that radius, by sum-of-absolute-
/// differences against the *original* frame-0 template (not re-templated each step, so
/// tracking doesn't drift from accumulating small per-step errors, at the cost of losing the
/// subject if its appearance changes too much).
///
/// Returns one [`TrackedPosition`] per input frame, `frames[0]`'s being exactly the requested
/// initial center (clamped so the template fits inside the frame). Empty input returns empty
/// output. Assumes every frame is the same size as `frames[0]` — a source video's dimensions
/// don't change mid-clip, so callers extracting frames from one clip always satisfy this.
pub fn track_region(
    frames: &[GrayFrame],
    initial_center_x_frac: f32,
    initial_center_y_frac: f32,
    template_width_frac: f32,
    template_height_frac: f32,
    search_radius_frac: f32,
) -> Vec<TrackedPosition> {
    let Some(first) = frames.first() else {
        return Vec::new();
    };
    let (w, h) = (first.width as i32, first.height as i32);
    let short_side = w.min(h);
    let width = ((template_width_frac * short_side as f32).round() as i32).clamp(4, w);
    let height = ((template_height_frac * short_side as f32).round() as i32).clamp(4, h);
    let half_w = width / 2;
    let half_h = height / 2;

    let mut cx = (initial_center_x_frac * w as f32).round() as i32;
    let mut cy = (initial_center_y_frac * h as f32).round() as i32;
    cx = cx.clamp(half_w, w - width + half_w);
    cy = cy.clamp(half_h, h - height + half_h);

    let template = extract_patch(first, cx - half_w, cy - half_h, width, height);
    let search_radius = ((search_radius_frac * short_side as f32).round() as i32).max(1);

    let mut results = Vec::with_capacity(frames.len());
    results.push(TrackedPosition {
        center_x_frac: cx as f32 / w as f32,
        center_y_frac: cy as f32 / h as f32,
    });

    for frame in &frames[1..] {
        let (fw, fh) = (frame.width as i32, frame.height as i32);
        let mut best_score = i64::MAX;
        let mut best_x = cx;
        let mut best_y = cy;
        for dy in -search_radius..=search_radius {
            for dx in -search_radius..=search_radius {
                let px = cx + dx;
                let py = cy + dy;
                let (x0, y0) = (px - half_w, py - half_h);
                if x0 < 0 || y0 < 0 || x0 + width > fw || y0 + height > fh {
                    continue;
                }
                let score = sad(&template, frame, x0, y0, width, height);
                if score < best_score {
                    best_score = score;
                    best_x = px;
                    best_y = py;
                }
            }
        }
        cx = best_x;
        cy = best_y;
        results.push(TrackedPosition {
            center_x_frac: cx as f32 / fw as f32,
            center_y_frac: cy as f32 / fh as f32,
        });
    }

    results
}

/// Converts tracked positions into a [`Keyframe<Position>`] list: each keyframe's value is
/// `base_position` plus the tracked point's delta from its own first-frame position — see this
/// module's doc comment for why a delta, not the tracked point's raw (source-frame-space)
/// coordinates. `sample_time_fractions` must be the same length as `tracked` (one entry per
/// sampled frame, `0.0..=1.0` within the clip's own duration); mismatched lengths zip to the
/// shorter of the two, same as any other `Iterator::zip`.
pub fn tracked_positions_to_keyframes(
    tracked: &[TrackedPosition],
    sample_time_fractions: &[f32],
    base_position: Position,
) -> Vec<Keyframe<Position>> {
    let Some(first) = tracked.first() else {
        return Vec::new();
    };
    tracked
        .iter()
        .zip(sample_time_fractions)
        .map(|(t, &time_fraction)| Keyframe {
            time_fraction,
            value: Position {
                x: base_position.x + (t.center_x_frac - first.center_x_frac),
                y: base_position.y + (t.center_y_frac - first.center_y_frac),
            },
        })
        .collect()
}

#[cfg(test)]
#[path = "motion_tracking/motion_tracking_test.rs"]
mod tests;
