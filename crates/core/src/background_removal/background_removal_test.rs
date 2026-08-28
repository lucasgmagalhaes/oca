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

use super::*;

fn approx(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "{a} != {b}");
}

#[test]
fn model_input_size_rounds_down_to_multiple_of_32() {
    let (w, h) = model_input_size(1920, 1080);
    assert_eq!(w % 32, 0);
    assert_eq!(h % 32, 0);
}

#[test]
fn model_input_size_scales_constrained_side_toward_target() {
    // 1920x1080 (landscape): height is the constrained side, should land near TARGET_SIZE.
    let (_, h) = model_input_size(1920, 1080);
    assert!(h <= TARGET_SIZE && h > TARGET_SIZE - 32);
}

#[test]
fn model_input_size_upscales_tiny_frames_toward_target() {
    // Both dims well under TARGET_SIZE — the reference implementation upscales rather than
    // leaving them tiny, same as a too-small source photo.
    let (w, h) = model_input_size(10, 10);
    assert_eq!(w % 32, 0);
    assert_eq!(h % 32, 0);
    assert!(w >= 480 && h >= 480);
}

#[test]
fn model_input_size_leaves_frames_already_spanning_target_alone_then_rounds() {
    // One dim already >= TARGET_SIZE and the other <= TARGET_SIZE: neither the "too small"
    // nor the "too large" branch fires, so only the multiple-of-32 rounding applies.
    let (w, h) = model_input_size(600, 400);
    assert_eq!(w, 600 - 600 % 32);
    assert_eq!(h, 400 - 400 % 32);
}

#[test]
fn preprocess_normalizes_black_pixel_to_negative_one() {
    let rgba = vec![0u8, 0, 0, 255]; // single black pixel
    let chw = preprocess(&rgba, 1, 1, 32, 32);
    assert_eq!(chw.len(), 3 * 32 * 32);
    approx(chw[0], -1.0);
}

#[test]
fn preprocess_normalizes_white_pixel_to_one() {
    let rgba = vec![255u8, 255, 255, 255];
    let chw = preprocess(&rgba, 1, 1, 32, 32);
    approx(chw[0], 1.0);
    approx(chw[32 * 32], 1.0);
    approx(chw[2 * 32 * 32], 1.0);
}

#[test]
fn resize_matte_upsamples_and_keeps_values() {
    // 2x2 matte with a distinct value per corner, upsampled to 4x4.
    let matte = vec![0.0, 1.0, 0.0, 1.0];
    let out = resize_matte(&matte, 2, 2, 4, 4);
    assert_eq!(out.len(), 16);
    // Top-left 2x2 block of the output should come from matte[0] = 0.0.
    approx(out[0], 0.0);
    // Top-right block should come from matte[1] = 1.0.
    approx(out[3], 1.0);
}

#[test]
fn resize_matte_downsamples_without_out_of_bounds() {
    let matte = vec![0.5; 64 * 64];
    let out = resize_matte(&matte, 64, 64, 8, 8);
    assert_eq!(out.len(), 64);
    for v in out {
        approx(v, 0.5);
    }
}
