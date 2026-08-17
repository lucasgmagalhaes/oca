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

use super::*;
use crate::timeline::{MaskShape, ShapeClip, ShapeKind, TextClip, WordTiming};

fn sample_shape(kind: ShapeKind) -> ShapeClip {
    ShapeClip {
        id: 1,
        start_secs: 0.0,
        duration_secs: 1.0,
        shape_kind: kind,
        center_x: 0.5,
        center_y: 0.5,
        width: 0.4,
        height: 0.4,
        rotation_deg: 0.0,
        color_rgba: [255, 0, 0, 255],
        stroke_thickness_px: 0.0,
    }
}

fn pixel(buf: &[u8], width: u32, x: u32, y: u32) -> [u8; 4] {
    let idx = (y * width + x) as usize * 4;
    [buf[idx], buf[idx + 1], buf[idx + 2], buf[idx + 3]]
}

#[test]
fn ellipse_center_pixel_is_opaque_and_corners_are_transparent() {
    let clip = sample_shape(ShapeKind::Ellipse);
    let buf = render_shape_clip_rgba(&clip, 200, 100);
    assert_eq!(pixel(&buf, 200, 100, 50), [255, 0, 0, 255]);
    assert_eq!(pixel(&buf, 200, 0, 0), [0, 0, 0, 0]);
    assert_eq!(pixel(&buf, 200, 199, 99), [0, 0, 0, 0]);
}

#[test]
fn rectangle_fills_its_whole_local_square_not_just_the_center() {
    let clip = sample_shape(ShapeKind::rectangle());
    let buf = render_shape_clip_rgba(&clip, 200, 100);
    // Center (100, 50) is inside; a corner just outside the 80x40px rect (width/height 0.4 of
    // 200x100) at (100+45, 50) should be outside.
    assert_eq!(pixel(&buf, 200, 100, 50), [255, 0, 0, 255]);
    assert_eq!(pixel(&buf, 200, 100 + 45, 50), [0, 0, 0, 0]);
}

#[test]
fn stroke_only_shape_leaves_its_own_center_transparent() {
    let mut clip = sample_shape(ShapeKind::Ellipse);
    clip.stroke_thickness_px = 5.0;
    let buf = render_shape_clip_rgba(&clip, 200, 100);
    assert_eq!(pixel(&buf, 200, 100, 50), [0, 0, 0, 0]);
}

fn sample_text(text: &str) -> TextClip {
    TextClip {
        id: 1,
        start_secs: 0.0,
        duration_secs: 1.0,
        text: text.to_string(),
        font_size: 32.0,
        color_rgba: [255, 255, 255, 255],
        pos_x: 0.1,
        pos_y: 0.1,
        words: Vec::<WordTiming>::new(),
        highlight_enabled: false,
        highlight_color_rgba: [255, 220, 0, 255],
    }
}

#[test]
fn empty_text_produces_a_fully_transparent_buffer() {
    let clip = sample_text("");
    let buf = render_text_clip_rgba(&clip, 200, 100);
    assert!(buf.chunks_exact(4).all(|p| p[3] == 0));
}

#[test]
fn non_empty_text_draws_at_least_one_opaque_pixel() {
    let clip = sample_text("A");
    let buf = render_text_clip_rgba(&clip, 200, 100);
    assert!(buf.chunks_exact(4).any(|p| p[3] > 0));
}

fn gray_pixel(buf: &[u8], width: u32, x: u32, y: u32) -> u8 {
    buf[(y * width + x) as usize]
}

#[test]
fn none_mask_shape_is_fully_masked_out() {
    let buf = render_mask_shape_gray8(MaskShape::None, 0.0, 100, 100);
    assert!(buf.iter().all(|&b| b == 0));
}

#[test]
fn circle_mask_center_is_visible_and_corners_are_masked() {
    let buf = render_mask_shape_gray8(MaskShape::Circle, 0.0, 100, 100);
    assert_eq!(gray_pixel(&buf, 100, 50, 50), 255);
    assert_eq!(gray_pixel(&buf, 100, 0, 0), 0);
}

#[test]
fn rounded_rect_mask_with_zero_radius_covers_almost_the_whole_frame() {
    let buf = render_mask_shape_gray8(MaskShape::RoundedRect, 0.0, 100, 100);
    assert_eq!(gray_pixel(&buf, 100, 50, 50), 255);
    assert_eq!(gray_pixel(&buf, 100, 1, 1), 255);
}

#[test]
fn rounded_rect_mask_with_full_radius_masks_out_the_corner() {
    let buf = render_mask_shape_gray8(MaskShape::RoundedRect, 1.0, 100, 100);
    assert_eq!(gray_pixel(&buf, 100, 50, 50), 255);
    assert_eq!(gray_pixel(&buf, 100, 0, 0), 0);
}
