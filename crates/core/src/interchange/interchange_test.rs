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
use crate::media::{MediaAsset, MediaKind};
use crate::project::{Project, Recency, SequenceExportSettings};
use crate::timeline::{
    AudioRole, ClipInstance, ColorFilter, Marker, MaskShape, Track, TrackKind, TransitionType,
};

fn clip(id: u64, asset_id: u64, start_secs: f64, duration_secs: f64) -> ClipInstance {
    ClipInstance {
        id,
        asset_id,
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

fn track(id: u64, kind: TrackKind, clips: Vec<ClipInstance>) -> Track {
    Track {
        id,
        name: format!("T{id}"),
        kind,
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

fn sequence(name: &str, timeline: Timeline) -> Sequence {
    Sequence {
        id: 1,
        name: name.to_string(),
        timeline,
        export_settings: SequenceExportSettings::default(),
    }
}

fn asset(id: u64, file_name: &str) -> MediaAsset {
    MediaAsset {
        id,
        file_name: file_name.to_string(),
        source_path: file_name.into(),
        kind: MediaKind::Video,
        has_audio: true,
        duration_secs: 60.0,
        codec: "h264".to_string(),
        source_bitrate_mbps: 8.0,
        resolution: Some((1920, 1080)),
        fps: Some(30.0),
        sample_rate_khz: None,
        loudness: None,
        proxy_path: None,
        waveform_peaks: None,
    }
}

fn project(media_library: Vec<MediaAsset>, sequences: Vec<Sequence>) -> Project {
    Project {
        id: 1,
        name: "Test".to_string(),
        last_edited: Recency::HoursAgo(1),
        summary: String::new(),
        media_library,
        sequences,
        active_sequence: 0,
        file_path: None,
        panel_layout: None,
        smart_bins: vec![],
    }
}

#[test]
fn rational_time_round_trips_through_seconds() {
    let rt = RationalTime::from_seconds(12.5, INTERCHANGE_TIME_RATE);
    assert!((rt.to_seconds() - 12.5).abs() < 1e-9);
}

#[test]
fn sequence_to_interchange_maps_track_order_and_clip_source_range() {
    let clips = vec![clip(1, 10, 0.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    assert_eq!(result.name, "Main");
    assert_eq!(result.tracks.len(), 1);
    assert_eq!(result.tracks[0].kind, InterchangeTrackKind::Video);
    assert_eq!(result.tracks[0].items.len(), 1);
    let InterchangeTrackItem::Clip(ic) = &result.tracks[0].items[0] else {
        panic!("expected a clip item");
    };
    assert_eq!(ic.source_id, 1);
    assert!((ic.source_range.start_time.to_seconds() - 0.0).abs() < 1e-9);
    assert!((ic.source_range.duration.to_seconds() - 5.0).abs() < 1e-9);
}

#[test]
fn sequence_to_interchange_resolves_media_reference_by_asset_id() {
    let clips = vec![clip(1, 10, 0.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    let InterchangeTrackItem::Clip(ic) = &result.tracks[0].items[0] else {
        panic!("expected a clip item");
    };
    assert_eq!(
        ic.media_reference,
        MediaReference::External {
            target_url: "gameplay.mp4".to_string()
        }
    );
}

#[test]
fn sequence_to_interchange_reports_missing_media_for_a_deleted_asset() {
    let clips = vec![clip(1, 999, 0.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    let InterchangeTrackItem::Clip(ic) = &result.tracks[0].items[0] else {
        panic!("expected a clip item");
    };
    assert_eq!(ic.media_reference, MediaReference::Missing);
}

#[test]
fn sequence_to_interchange_inserts_a_leading_gap_before_the_first_clip() {
    let clips = vec![clip(1, 10, 5.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    assert_eq!(result.tracks[0].items.len(), 2);
    let InterchangeTrackItem::Gap(gap) = &result.tracks[0].items[0] else {
        panic!("expected a leading gap");
    };
    assert!((gap.start_time.to_seconds() - 0.0).abs() < 1e-9);
    assert!((gap.duration.to_seconds() - 5.0).abs() < 1e-9);
    assert!(matches!(
        result.tracks[0].items[1],
        InterchangeTrackItem::Clip(_)
    ));
}

#[test]
fn sequence_to_interchange_inserts_a_gap_between_two_non_adjacent_clips() {
    let clips = vec![clip(1, 10, 0.0, 5.0), clip(2, 10, 8.0, 2.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    // clip(0..5), gap(5..8), clip(8..10)
    assert_eq!(result.tracks[0].items.len(), 3);
    let InterchangeTrackItem::Gap(gap) = &result.tracks[0].items[1] else {
        panic!("expected a middle gap");
    };
    assert!((gap.start_time.to_seconds() - 5.0).abs() < 1e-9);
    assert!((gap.duration.to_seconds() - 3.0).abs() < 1e-9);
}

#[test]
fn sequence_to_interchange_emits_no_gap_between_back_to_back_clips() {
    let clips = vec![clip(1, 10, 0.0, 5.0), clip(2, 10, 5.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    assert_eq!(result.tracks[0].items.len(), 2);
    assert!(result.tracks[0]
        .items
        .iter()
        .all(|item| matches!(item, InterchangeTrackItem::Clip(_))));
}

#[test]
fn sequence_to_interchange_sorts_out_of_order_clips_by_start_time() {
    let clips = vec![clip(2, 10, 5.0, 5.0), clip(1, 10, 0.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    let ids: Vec<u64> = result.tracks[0]
        .items
        .iter()
        .filter_map(|item| match item {
            InterchangeTrackItem::Clip(c) => Some(c.source_id),
            InterchangeTrackItem::Gap(_) => None,
        })
        .collect();
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn sequence_to_interchange_omits_text_and_shape_tracks() {
    let tracks = vec![
        track(1, TrackKind::Video, vec![]),
        track(2, TrackKind::Text, vec![]),
        track(3, TrackKind::Shape, vec![]),
        track(4, TrackKind::Audio, vec![]),
    ];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    assert_eq!(result.tracks.len(), 2);
    assert_eq!(result.tracks[0].kind, InterchangeTrackKind::Video);
    assert_eq!(result.tracks[1].kind, InterchangeTrackKind::Audio);
}

#[test]
fn sequence_to_interchange_carries_speed_factor_and_transition_kind() {
    let mut c = clip(1, 10, 0.0, 5.0);
    c.speed_factor = 1.5;
    c.transition_in = TransitionType::Fade;
    let tracks = vec![track(1, TrackKind::Video, vec![c])];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    let InterchangeTrackItem::Clip(ic) = &result.tracks[0].items[0] else {
        panic!("expected a clip item");
    };
    assert_eq!(ic.speed_factor, 1.5);
    assert_eq!(ic.transition_in, Some(InterchangeTransitionKind::Fade));
}

#[test]
fn sequence_to_interchange_carries_markers_as_point_in_time_ranges() {
    let mut timeline = timeline_with(vec![]);
    timeline.markers = vec![Marker {
        id: 1,
        position_secs: 12.0,
        label: "Boss fight".to_string(),
        kind: MarkerKind::Highlight,
        completed: false,
    }];
    let seq = sequence("Main", timeline);
    let proj = project(vec![], vec![]);

    let result = sequence_to_interchange(&seq, &proj);
    assert_eq!(result.markers.len(), 1);
    assert_eq!(result.markers[0].name, "Boss fight");
    assert_eq!(result.markers[0].kind, MarkerKind::Highlight);
    assert!((result.markers[0].marked_range.start_time.to_seconds() - 12.0).abs() < 1e-9);
    assert!((result.markers[0].marked_range.duration.to_seconds() - 0.0).abs() < 1e-9);
}

#[test]
fn compatibility_report_marks_plain_clips_as_supported_only() {
    let clips = vec![clip(1, 10, 0.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));

    let report = interchange_compatibility_report(&seq);
    assert_eq!(report.supported.len(), 1);
    assert!(report.approximated.is_empty());
    assert!(report.omitted.is_empty());
}

#[test]
fn compatibility_report_flags_a_transition_as_approximated() {
    let mut c = clip(1, 10, 0.0, 5.0);
    c.transition_in = TransitionType::Slide;
    let tracks = vec![track(1, TrackKind::Video, vec![c])];
    let seq = sequence("Main", timeline_with(tracks));

    let report = interchange_compatibility_report(&seq);
    assert_eq!(report.approximated.len(), 1);
    assert_eq!(report.approximated[0].clip_id, Some(1));
}

#[test]
fn compatibility_report_flags_color_grading_and_crop_as_omitted() {
    let mut c = clip(1, 10, 0.0, 5.0);
    c.brightness = 0.2;
    c.crop_w = 0.5;
    let tracks = vec![track(1, TrackKind::Video, vec![c])];
    let seq = sequence("Main", timeline_with(tracks));

    let report = interchange_compatibility_report(&seq);
    let descriptions: Vec<&str> = report
        .omitted
        .iter()
        .map(|e| e.description.as_str())
        .collect();
    assert!(descriptions.contains(&"color grading"));
    assert!(descriptions.contains(&"crop/pan"));
}

#[test]
fn compatibility_report_flags_a_nested_sequence_as_omitted() {
    let mut c = clip(1, 10, 0.0, 5.0);
    c.nested_sequence_id = Some(42);
    let tracks = vec![track(1, TrackKind::Video, vec![c])];
    let seq = sequence("Main", timeline_with(tracks));

    let report = interchange_compatibility_report(&seq);
    assert!(report
        .omitted
        .iter()
        .any(|e| e.description == "compound clip (nested sequence)"));
}

#[test]
fn compatibility_report_flags_overlay_tracks_as_omitted_with_track_context() {
    let tracks = vec![track(1, TrackKind::Text, vec![])];
    let seq = sequence("Main", timeline_with(tracks));

    let report = interchange_compatibility_report(&seq);
    assert_eq!(report.omitted.len(), 1);
    assert_eq!(report.omitted[0].track_name, "T1");
    assert_eq!(report.omitted[0].clip_id, None);
}

#[test]
fn compatibility_report_reports_marker_count() {
    let mut timeline = timeline_with(vec![]);
    timeline.markers = vec![Marker {
        id: 1,
        position_secs: 1.0,
        label: String::new(),
        kind: MarkerKind::Standard,
        completed: false,
    }];
    let seq = sequence("Main", timeline);

    let report = interchange_compatibility_report(&seq);
    assert!(report
        .supported
        .iter()
        .any(|e| e.description == "1 marker(s)"));
}

#[test]
fn interchange_to_timeline_round_trips_a_clip_with_no_timing_drift() {
    let clips = vec![clip(1, 10, 3.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let ic = sequence_to_interchange(&seq, &proj);
    let mut next_id = 100;
    let result = interchange_to_timeline(&ic, &proj, &mut next_id);

    assert!(result.warnings.is_empty());
    assert_eq!(result.timeline.tracks.len(), 1);
    assert_eq!(result.timeline.tracks[0].clips.len(), 1);
    let reconstructed = &result.timeline.tracks[0].clips[0];
    let original = &seq.timeline.tracks[0].clips[0];
    assert!((reconstructed.start_secs - original.start_secs).abs() < 1e-6);
    assert!((reconstructed.source_in_secs - original.source_in_secs).abs() < 1e-6);
    assert!((reconstructed.source_out_secs - original.source_out_secs).abs() < 1e-6);
    assert_eq!(reconstructed.speed_factor, original.speed_factor);
    assert_eq!(reconstructed.asset_id, original.asset_id);
}

#[test]
fn interchange_to_timeline_round_trips_a_long_sequence_without_accumulating_drift() {
    let mut clips = Vec::new();
    let mut cursor = 0.0;
    for i in 0..500u64 {
        let duration = 1.7;
        clips.push(clip(i + 1, 10, cursor, duration));
        cursor += duration + 0.3; // leave a gap between every clip
    }
    let expected_starts: Vec<f64> = clips.iter().map(|c| c.start_secs).collect();
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let ic = sequence_to_interchange(&seq, &proj);
    let mut next_id = 10_000;
    let result = interchange_to_timeline(&ic, &proj, &mut next_id);

    assert!(result.warnings.is_empty());
    let reconstructed_starts: Vec<f64> = result.timeline.tracks[0]
        .clips
        .iter()
        .map(|c| c.start_secs)
        .collect();
    assert_eq!(reconstructed_starts.len(), expected_starts.len());
    for (expected, actual) in expected_starts.iter().zip(reconstructed_starts.iter()) {
        assert!(
            (expected - actual).abs() < 1e-6,
            "expected {expected}, got {actual}"
        );
    }
}

#[test]
fn interchange_to_timeline_carries_speed_factor_and_transition_kind() {
    let mut c = clip(1, 10, 0.0, 5.0);
    c.speed_factor = 2.0;
    c.transition_in = TransitionType::Zoom;
    let tracks = vec![track(1, TrackKind::Video, vec![c])];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let ic = sequence_to_interchange(&seq, &proj);
    let mut next_id = 100;
    let result = interchange_to_timeline(&ic, &proj, &mut next_id);

    let reconstructed = &result.timeline.tracks[0].clips[0];
    assert_eq!(reconstructed.speed_factor, 2.0);
    assert_eq!(reconstructed.transition_in, TransitionType::Zoom);
}

#[test]
fn interchange_to_timeline_reports_a_warning_and_skips_a_clip_with_unresolvable_media() {
    let clips = vec![clip(1, 999, 0.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![], vec![]);

    let ic = sequence_to_interchange(&seq, &proj);
    let mut next_id = 100;
    let result = interchange_to_timeline(&ic, &proj, &mut next_id);

    assert!(result.timeline.tracks[0].clips.is_empty());
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].track_name, "T1");
}

#[test]
fn interchange_to_timeline_allocates_strictly_increasing_ids_and_advances_next_id() {
    let clips = vec![clip(1, 10, 0.0, 5.0), clip(2, 10, 5.0, 5.0)];
    let tracks = vec![track(1, TrackKind::Video, clips)];
    let seq = sequence("Main", timeline_with(tracks));
    let proj = project(vec![asset(10, "gameplay.mp4")], vec![]);

    let ic = sequence_to_interchange(&seq, &proj);
    let mut next_id = 50;
    let result = interchange_to_timeline(&ic, &proj, &mut next_id);

    let mut ids = vec![result.timeline.tracks[0].id];
    ids.extend(result.timeline.tracks[0].clips.iter().map(|c| c.id));
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(ids.len(), sorted.len(), "every allocated id must be unique");
    assert!(next_id > 50, "next_id must advance past its starting value");
}

#[test]
fn interchange_to_timeline_round_trips_markers() {
    let mut timeline = timeline_with(vec![]);
    timeline.markers = vec![Marker {
        id: 1,
        position_secs: 42.0,
        label: "Boss fight".to_string(),
        kind: MarkerKind::Highlight,
        completed: false,
    }];
    let seq = sequence("Main", timeline);
    let proj = project(vec![], vec![]);

    let ic = sequence_to_interchange(&seq, &proj);
    let mut next_id = 100;
    let result = interchange_to_timeline(&ic, &proj, &mut next_id);

    assert_eq!(result.timeline.markers.len(), 1);
    assert_eq!(result.timeline.markers[0].label, "Boss fight");
    assert_eq!(result.timeline.markers[0].kind, MarkerKind::Highlight);
    assert!((result.timeline.markers[0].position_secs - 42.0).abs() < 1e-6);
}
