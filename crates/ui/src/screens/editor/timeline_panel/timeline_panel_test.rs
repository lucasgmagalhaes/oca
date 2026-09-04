// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use super::draw::{filmstrip_frame_index_for_tile, visible_tile_range};
use super::snap::{snap_move_start, snap_to_nearest, waveform_snap_points_for_clip};

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

// px_per_sec = 10.0 below -> SNAP_THRESHOLD_PX (8.0) is 0.8s, a round number to reason about.

#[test]
fn snap_to_nearest_snaps_within_the_pixel_threshold() {
    assert_eq!(snap_to_nearest(5.0, &[5.5, 10.0], 10.0), 5.5);
}

#[test]
fn snap_to_nearest_leaves_a_candidate_outside_the_threshold_untouched() {
    assert_eq!(snap_to_nearest(5.0, &[7.0], 10.0), 5.0);
}

#[test]
fn snap_to_nearest_is_a_no_op_with_no_targets() {
    assert_eq!(snap_to_nearest(5.0, &[], 10.0), 5.0);
}

#[test]
fn snap_move_start_snaps_the_clips_leading_edge() {
    // candidate spans [5.0, 7.0); 4.7 is within range of the start edge only.
    assert_eq!(snap_move_start(5.0, 2.0, &[4.7], 10.0), 4.7);
}

#[test]
fn snap_move_start_snaps_the_clips_trailing_edge() {
    // candidate spans [5.0, 7.0); 7.3 is within range of the end edge only — the clip's start
    // shifts by the same amount so the end edge lands exactly on the target.
    assert_eq!(snap_move_start(5.0, 2.0, &[7.3], 10.0), 5.3);
}

#[test]
fn snap_move_start_prefers_whichever_edge_needs_the_smaller_adjustment() {
    // candidate spans [5.0, 7.0); 5.2 is 0.2 from the start edge, 7.5 is 0.5 from the end edge
    // — both are in range, so the smaller (start) adjustment wins.
    assert_eq!(snap_move_start(5.0, 2.0, &[5.2, 7.5], 10.0), 5.2);
}

#[test]
fn snap_move_start_is_a_no_op_when_neither_edge_is_in_range() {
    assert_eq!(snap_move_start(5.0, 2.0, &[100.0], 10.0), 5.0);
}

fn test_asset_with_peaks(peaks: Vec<(f32, f32)>, duration_secs: f64) -> avcore::MediaAsset {
    avcore::MediaAsset {
        id: 1,
        file_name: "clip.mp4".to_string(),
        source_path: std::path::PathBuf::from("clip.mp4"),
        kind: avcore::MediaKind::Video,
        has_audio: true,
        duration_secs,
        codec: "H.264".to_string(),
        source_bitrate_mbps: 8.0,
        resolution: Some((1920, 1080)),
        fps: Some(30.0),
        sample_rate_khz: None,
        loudness: None,
        proxy_path: None,
        waveform_peaks: Some(peaks),
        favorited: false,
    }
}

fn test_clip(
    start_secs: f64,
    source_in_secs: f64,
    source_out_secs: f64,
) -> avcore::timeline::ClipInstance {
    avcore::timeline::ClipInstance {
        id: 1,
        asset_id: 1,
        start_secs,
        source_in_secs,
        source_out_secs,
        composite_id: None,
        color_label: None,
        gain_db: 0.0,
        frozen: false,
        speed_factor: 1.0,
        speed_ramp_end_factor: None,
        nested_sequence_id: None,
        crop_x: 0.0,
        crop_y: 0.0,
        crop_w: 1.0,
        crop_h: 1.0,
        mask_shape: avcore::timeline::MaskShape::None,
        mask_corner_radius: 0.0,
        flipped_h: false,
        color_filter: avcore::timeline::ColorFilter::None,
        vignette_intensity: 0.0,
        brightness: 0.0,
        contrast: 1.0,
        saturation: 1.0,
        sharpen: 0.0,
        chroma_key_enabled: false,
        chroma_key_color: [0, 255, 0],
        chroma_key_tolerance: 0.4,
        blur_intensity: 0.0,
        shake_intensity: 0.0,
        glitch_intensity: 0.0,
        pixelize_intensity: 0.0,
        transition_in: avcore::timeline::TransitionType::None,
        transition_duration_secs: 0.5,
        position_keyframes: vec![],
        scale_keyframes: vec![],
        rotation_keyframes: vec![],
        opacity_keyframes: vec![],
        gain_keyframes: vec![],
        voice_cleanup_enabled: false,
        voice_cleanup_noise_floor_db: -30.0,
        voice_cleanup_compressor_threshold_db: -18.0,
        voice_cleanup_compressor_ratio: 3.0,
        voice_cleanup_ceiling_linear: 0.95,
        brightness_keyframes: vec![],
        contrast_keyframes: vec![],
        saturation_keyframes: vec![],
        crop_x_keyframes: vec![],
        crop_y_keyframes: vec![],
        crop_w_keyframes: vec![],
        crop_h_keyframes: vec![],
        deflicker_enabled: false,
        lut_path: String::new(),
        layer_scale_x: 1.0,
        layer_scale_y: 1.0,
        stabilization_intensity: 0.0,
        background_removal_enabled: false,
        background_removal_mask_path: String::new(),
        blend_mode: avcore::timeline::BlendMode::Normal,
        anchor_x: 0.5,
        anchor_y: 0.5,
    }
}

#[test]
fn waveform_snap_points_for_clip_returns_a_midpoint_per_detected_gap() {
    // 20 one-second buckets, loud except a silent run [8, 12).
    let mut peaks = vec![(-0.8, 0.8); 20];
    for p in &mut peaks[8..12] {
        *p = (0.0, 0.0);
    }
    let asset = test_asset_with_peaks(peaks, 20.0);
    // Clip shows the full source, starting at timeline 0 -- gap [8,12) maps unchanged.
    let clip = test_clip(0.0, 0.0, 20.0);

    let points = waveform_snap_points_for_clip(&asset, &clip);

    assert_eq!(points, vec![10.0], "midpoint of the [8, 12) gap");
}

#[test]
fn waveform_snap_points_for_clip_catches_a_gap_shorter_than_d1s_own_cuttable_threshold() {
    // 100 buckets over 10s -> 0.1s per bucket. A single silent bucket is a 0.1s gap: shorter
    // than D1's own 0.5s cuttable-gap minimum (so D1 itself would never suggest cutting it),
    // but still well above D5's own much smaller minimum (0.05s).
    let mut peaks = vec![(-0.8, 0.8); 100];
    peaks[50] = (0.0, 0.0);
    let asset = test_asset_with_peaks(peaks, 10.0);
    let clip = test_clip(0.0, 0.0, 10.0);

    let points = waveform_snap_points_for_clip(&asset, &clip);

    assert_eq!(points.len(), 1);
    assert!((points[0] - 5.05).abs() < 1e-9);
}

#[test]
fn waveform_snap_points_for_clip_is_empty_without_a_cached_waveform() {
    let asset = avcore::MediaAsset {
        waveform_peaks: None,
        favorited: false,
        ..test_asset_with_peaks(vec![], 10.0)
    };
    let clip = test_clip(0.0, 0.0, 10.0);

    assert!(waveform_snap_points_for_clip(&asset, &clip).is_empty());
}
