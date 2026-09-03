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

fn sample(time_fraction: f32, subject_center: Option<(f32, f32)>) -> ReframeSample {
    ReframeSample {
        time_fraction,
        subject_center,
    }
}

#[test]
fn fill_reframe_gaps_holds_a_short_gap_at_the_last_known_center() {
    let samples = vec![
        sample(0.0, Some((0.1, 0.1))),
        sample(0.1, None),
        sample(0.2, None),
        sample(0.3, Some((0.5, 0.5))),
    ];
    let filled = fill_reframe_gaps(&samples);
    assert_eq!(
        filled,
        vec![
            Some((0.1, 0.1)),
            Some((0.1, 0.1)),
            Some((0.1, 0.1)),
            Some((0.5, 0.5)),
        ]
    );
}

#[test]
fn fill_reframe_gaps_falls_back_to_none_on_a_long_gap() {
    let samples = vec![
        sample(0.0, Some((0.1, 0.1))),
        sample(0.1, None),
        sample(0.2, None),
        sample(0.3, None),
        sample(0.4, None),
        sample(0.5, Some((0.9, 0.9))),
    ];
    let filled = fill_reframe_gaps(&samples);
    assert_eq!(
        filled,
        vec![Some((0.1, 0.1)), None, None, None, None, Some((0.9, 0.9)),]
    );
}

#[test]
fn fill_reframe_gaps_leaves_a_leading_gap_as_none() {
    // No subject has ever been seen yet -- nothing to hold.
    let samples = vec![
        sample(0.0, None),
        sample(0.1, None),
        sample(0.2, Some((0.5, 0.5))),
    ];
    let filled = fill_reframe_gaps(&samples);
    assert_eq!(filled, vec![None, None, Some((0.5, 0.5))]);
}

#[test]
fn fill_reframe_gaps_is_a_no_op_with_no_gaps() {
    let samples = vec![
        sample(0.0, Some((0.2, 0.2))),
        sample(0.5, Some((0.4, 0.4))),
        sample(1.0, Some((0.6, 0.6))),
    ];
    let filled = fill_reframe_gaps(&samples);
    assert_eq!(
        filled,
        vec![Some((0.2, 0.2)), Some((0.4, 0.4)), Some((0.6, 0.6))]
    );
}

#[test]
fn smooth_subject_centers_averages_within_the_window() {
    let centers = vec![Some((0.0, 0.0)), Some((0.2, 0.2)), Some((0.4, 0.4))];
    let smoothed = smooth_subject_centers(&centers, 3);
    assert_eq!(smoothed[0], Some((0.1, 0.1)));
    assert_eq!(smoothed[1], Some((0.2, 0.2)));
    assert_eq!(smoothed[2], Some((0.3, 0.3)));
}

#[test]
fn smooth_subject_centers_leaves_none_entries_untouched() {
    let centers = vec![Some((0.0, 0.0)), None, Some((1.0, 1.0))];
    let smoothed = smooth_subject_centers(&centers, 3);
    assert_eq!(smoothed[1], None);
    // Neighbors skip the None slot in their own average rather than treating it as zero.
    assert_eq!(smoothed[0], Some((0.0, 0.0)));
    assert_eq!(smoothed[2], Some((1.0, 1.0)));
}

fn crop(x: f32, y: f32, w: f32, h: f32) -> CropRect {
    CropRect { x, y, w, h }
}

#[test]
fn sparse_crop_keyframes_emits_first_and_last_even_when_perfectly_static() {
    let samples = vec![
        (0.0, crop(0.1, 0.1, 0.5, 0.5)),
        (0.5, crop(0.1, 0.1, 0.5, 0.5)),
        (1.0, crop(0.1, 0.1, 0.5, 0.5)),
    ];
    let (xs, ys, ws, hs) = sparse_crop_keyframes(&samples, DEFAULT_SPARSIFY_EPSILON);
    for axis in [&xs, &ys, &ws, &hs] {
        assert_eq!(axis.len(), 2);
        assert_eq!(axis[0].time_fraction, 0.0);
        assert_eq!(axis[1].time_fraction, 1.0);
    }
}

#[test]
fn sparse_crop_keyframes_is_empty_for_no_samples() {
    let (xs, ys, ws, hs) = sparse_crop_keyframes(&[], DEFAULT_SPARSIFY_EPSILON);
    assert!(xs.is_empty() && ys.is_empty() && ws.is_empty() && hs.is_empty());
}

#[test]
fn sparse_crop_keyframes_handles_a_single_sample_without_panicking() {
    let samples = vec![(0.5, crop(0.2, 0.2, 0.6, 0.6))];
    let (xs, ys, ws, hs) = sparse_crop_keyframes(&samples, DEFAULT_SPARSIFY_EPSILON);
    for axis in [&xs, &ys, &ws, &hs] {
        assert_eq!(axis.len(), 1);
        assert_eq!(axis[0].time_fraction, 0.5);
    }
}

#[test]
fn sparse_crop_keyframes_emits_a_point_where_a_value_crosses_the_threshold() {
    let samples = vec![
        (0.0, crop(0.1, 0.5, 0.5, 0.5)),
        (0.3, crop(0.1005, 0.5, 0.5, 0.5)), // within epsilon of the first x -- no new point
        (0.6, crop(0.5, 0.5, 0.5, 0.5)),    // crosses epsilon -- new point
        (1.0, crop(0.5, 0.5, 0.5, 0.5)),    // matches the last emitted value -- no new point
    ];
    let (xs, _ys, _ws, _hs) = sparse_crop_keyframes(&samples, 0.01);
    assert_eq!(xs.len(), 2);
    assert_eq!(xs[0].time_fraction, 0.0);
    assert_eq!(xs[0].value, 0.1);
    assert_eq!(xs[1].time_fraction, 0.6);
    assert_eq!(xs[1].value, 0.5);
}

#[test]
fn sparse_crop_keyframes_sparsifies_each_axis_independently() {
    // x moves a lot at the middle sample; y never moves at all -- y should stay at just its
    // first/last points while x also picks up the middle one.
    let samples = vec![
        (0.0, crop(0.1, 0.3, 0.5, 0.5)),
        (0.5, crop(0.6, 0.3, 0.5, 0.5)),
        (1.0, crop(0.6, 0.3, 0.5, 0.5)),
    ];
    let (xs, ys, _ws, _hs) = sparse_crop_keyframes(&samples, 0.01);
    assert_eq!(xs.len(), 2);
    assert_eq!(xs[1].time_fraction, 0.5);
    assert_eq!(ys.len(), 2);
    assert_eq!(ys[0].time_fraction, 0.0);
    assert_eq!(ys[1].time_fraction, 1.0);
}
