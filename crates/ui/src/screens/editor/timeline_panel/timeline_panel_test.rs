// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.

use super::{filmstrip_frame_index_for_tile, visible_tile_range};

#[test]
fn visible_tiles_stay_anchored_to_the_full_clip() {
    assert_eq!(visible_tile_range(10.0, 200.0, 10.0, 40.0, 30.0), 0..1);
    assert_eq!(visible_tile_range(10.0, 200.0, 99.0, 130.0, 30.0), 2..4);
    assert_eq!(visible_tile_range(10.0, 200.0, 100.0, 130.0, 30.0), 3..4);
}

#[test]
fn a_multi_hour_offscreen_clip_only_visits_viewport_tiles() {
    let tiles = visible_tile_range(0.0, 720_000.0, 0.0, 1_000.0, 40.0);

    assert_eq!(tiles, 0..25);
}

#[test]
fn zooming_in_samples_source_frames_more_densely() {
    let fps = Some(30.0);
    let first_zoomed_out = filmstrip_frame_index_for_tile(0.0, 20.0, 10.0, fps);
    let next_zoomed_out = filmstrip_frame_index_for_tile(0.0, 60.0, 10.0, fps);
    let first_zoomed_in = filmstrip_frame_index_for_tile(0.0, 20.0, 60.0, fps);
    let next_zoomed_in = filmstrip_frame_index_for_tile(0.0, 60.0, 60.0, fps);

    assert_eq!(first_zoomed_out, 60);
    assert_eq!(next_zoomed_out, 180);
    assert_eq!(first_zoomed_in, 10);
    assert_eq!(next_zoomed_in, 30);
    assert!(next_zoomed_in - first_zoomed_in < next_zoomed_out - first_zoomed_out);
}

#[test]
fn repeated_tile_centers_on_the_same_source_frame_share_a_cache_key() {
    let first = filmstrip_frame_index_for_tile(1.0, 1.0, 1_000.0, Some(30.0));
    let second = filmstrip_frame_index_for_tile(1.0, 10.0, 1_000.0, Some(30.0));

    assert_eq!(first, second);
}
