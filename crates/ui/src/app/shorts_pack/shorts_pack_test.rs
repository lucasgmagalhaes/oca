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
use avcore::timeline::{
    AudioRole, ClipInstance, ColorFilter, Marker, MaskShape, Track, TransitionType,
};

fn clip(id: u64, start_secs: f64, duration_secs: f64) -> ClipInstance {
    ClipInstance {
        id,
        asset_id: 1,
        start_secs,
        source_in_secs: 0.0,
        source_out_secs: duration_secs,
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
        mask_shape: MaskShape::None,
        mask_corner_radius: 0.0,
        flipped_h: false,
        color_filter: ColorFilter::None,
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
        transition_in: TransitionType::None,
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
    }
}

fn video_track(id: u64, clips: Vec<ClipInstance>) -> Track {
    Track {
        id,
        name: format!("V{id}"),
        kind: TrackKind::Video,
        clips,
        text_clips: vec![],
        shape_clips: vec![],
        visible: true,
        audio_role: AudioRole::Unspecified,
        locked: false,
        color_label: None,
    }
}

fn timeline_with(tracks: Vec<Track>) -> Timeline {
    Timeline {
        tracks,
        playhead_secs: 0.0,
        markers: vec![],
        multicam_groups: vec![],
    }
}

#[test]
fn clips_needing_reframe_includes_a_clip_overlapping_the_window() {
    let timeline = timeline_with(vec![video_track(1, vec![clip(1, 0.0, 20.0)])]);
    // Highlight at 10s -> window [5, 20) with the default lead-in/trail-out.
    let ids = clips_needing_reframe(&timeline, &[10.0], 100.0);
    assert_eq!(ids, vec![1]);
}

#[test]
fn clips_needing_reframe_excludes_a_clip_that_already_has_crop_keyframes() {
    let mut c = clip(1, 0.0, 20.0);
    c.crop_x_keyframes = vec![avcore::Keyframe {
        time_fraction: 0.0,
        value: 0.1,
    }];
    let timeline = timeline_with(vec![video_track(1, vec![c])]);
    let ids = clips_needing_reframe(&timeline, &[10.0], 100.0);
    assert!(ids.is_empty());
}

#[test]
fn clips_needing_reframe_excludes_a_clip_outside_every_window() {
    let timeline = timeline_with(vec![video_track(1, vec![clip(1, 0.0, 5.0)])]);
    // Highlight far past this clip's own [0, 5) span.
    let ids = clips_needing_reframe(&timeline, &[500.0], 1000.0);
    assert!(ids.is_empty());
}

#[test]
fn clips_needing_reframe_ignores_non_video_tracks() {
    let mut audio_track = video_track(1, vec![clip(1, 0.0, 20.0)]);
    audio_track.kind = avcore::timeline::TrackKind::Audio;
    let timeline = timeline_with(vec![audio_track]);
    let ids = clips_needing_reframe(&timeline, &[10.0], 100.0);
    assert!(ids.is_empty());
}

#[test]
fn clips_needing_reframe_deduplicates_a_clip_spanning_two_windows() {
    // One long clip [0, 100) overlaps both highlight windows -- must appear once, not twice.
    let timeline = timeline_with(vec![video_track(1, vec![clip(1, 0.0, 100.0)])]);
    let ids = clips_needing_reframe(&timeline, &[10.0, 50.0], 200.0);
    assert_eq!(ids, vec![1]);
}

#[test]
fn clips_needing_reframe_collects_every_distinct_clip_across_windows() {
    let timeline = timeline_with(vec![video_track(
        1,
        vec![clip(1, 0.0, 20.0), clip(2, 100.0, 20.0)],
    )]);
    let ids = clips_needing_reframe(&timeline, &[10.0, 110.0], 200.0);
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn sorted_highlight_positions_sorts_and_filters_to_highlight_markers_only() {
    let mut timeline = timeline_with(vec![]);
    timeline.markers = vec![
        Marker {
            id: 1,
            position_secs: 40.0,
            kind: avcore::timeline::MarkerKind::Highlight,
            label: String::new(),
            completed: false,
        },
        Marker {
            id: 2,
            position_secs: 10.0,
            kind: avcore::timeline::MarkerKind::Highlight,
            label: String::new(),
            completed: false,
        },
        Marker {
            id: 3,
            position_secs: 5.0,
            kind: avcore::timeline::MarkerKind::Chapter,
            label: String::new(),
            completed: false,
        },
    ];
    let positions = sorted_highlight_positions(&timeline);
    assert_eq!(positions, vec![10.0, 40.0]);
}
