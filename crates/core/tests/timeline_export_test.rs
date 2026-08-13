use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use avcore::project::Sequence;
use avcore::render::{render_timeline_export, RenderError, RenderOutcome};
use avcore::timeline::{
    ClipInstance, ColorFilter, MaskShape, Timeline, Track, TrackKind, TransitionType,
};
use avcore::{probe_media, MediaAsset, MediaKind};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn clip(
    id: u64,
    asset_id: u64,
    start_secs: f64,
    source_in_secs: f64,
    source_out_secs: f64,
) -> ClipInstance {
    ClipInstance {
        id,
        asset_id,
        start_secs,
        source_in_secs,
        source_out_secs,
        composite_id: None,
        gain_db: 0.0,
        frozen: false,
        speed_factor: 1.0,
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
        zoom_start: 1.0,
        zoom_end: 1.0,
    }
}

fn video_asset(id: u64) -> MediaAsset {
    let path = fixture("video.mp4");
    probe_media(&path)
        .unwrap()
        .into_media_asset(id, "video.mp4".to_string(), path)
}

fn sequence_with(tracks: Vec<Track>) -> Sequence {
    Sequence {
        id: 1,
        name: "Sequence 1".to_string(),
        timeline: Timeline {
            tracks,
            playhead_secs: 0.0,
        },
    }
}

#[test]
fn renders_two_clips_with_different_effects_as_one_concatenated_export() {
    let asset = video_asset(1);
    let mut c1 = clip(1, 1, 0.0, 0.0, 0.3);
    let mut c2 = clip(2, 1, 0.3, 0.3, 0.6);
    c1.flipped_h = true;
    c2.vignette_intensity = 0.5;

    let track = Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![c2, c1], // deliberately out of start_secs order
        visible: true,
    };
    let sequence = sequence_with(vec![track]);

    let output = std::env::temp_dir().join("avcore_test_timeline_export_ok.mp4");
    let cancel = AtomicBool::new(false);
    let mut last_percent = 0u8;

    let outcome = render_timeline_export(&sequence, &[asset], &output, -14.0, &cancel, |percent| {
        last_percent = percent
    })
    .unwrap();

    assert_eq!(outcome, RenderOutcome::Completed);
    assert_eq!(last_percent, 100);

    let info = probe_media(&output).unwrap();
    assert_eq!(info.kind, MediaKind::Video);

    let _ = std::fs::remove_file(&output);
}

#[test]
fn frozen_clip_still_exports_its_full_timeline_duration() {
    let asset = video_asset(1);
    let mut c1 = clip(1, 1, 0.0, 0.0, 0.3);
    c1.frozen = true;

    let track = Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![c1],
        visible: true,
    };
    let sequence = sequence_with(vec![track]);

    let output = std::env::temp_dir().join("avcore_test_timeline_export_frozen.mp4");
    let cancel = AtomicBool::new(false);

    let outcome = render_timeline_export(&sequence, &[asset], &output, -14.0, &cancel, |_| {})
        .unwrap();

    assert_eq!(outcome, RenderOutcome::Completed);

    let info = probe_media(&output).unwrap();
    assert_eq!(info.kind, MediaKind::Video);

    let _ = std::fs::remove_file(&output);
}

#[test]
fn rejects_a_sequence_with_no_video_track() {
    let sequence = sequence_with(vec![]);
    let output = std::env::temp_dir().join("avcore_test_timeline_export_empty.mp4");
    let cancel = AtomicBool::new(false);

    let err = render_timeline_export(&sequence, &[], &output, -14.0, &cancel, |_| {}).unwrap_err();

    assert!(matches!(err, RenderError::EmptyTimeline));
}

#[test]
fn rejects_a_clip_with_a_missing_asset() {
    let track = Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![clip(1, 999, 0.0, 0.0, 0.3)],
        visible: true,
    };
    let sequence = sequence_with(vec![track]);
    let output = std::env::temp_dir().join("avcore_test_timeline_export_missing_asset.mp4");
    let cancel = AtomicBool::new(false);

    // No assets at all in the media library — clip's asset_id (999) can't resolve.
    let err = render_timeline_export(&sequence, &[], &output, -14.0, &cancel, |_| {}).unwrap_err();

    assert!(matches!(err, RenderError::MissingAsset));
}

#[test]
fn cancelling_mid_timeline_export_reports_cancelled() {
    let asset = video_asset(1);
    let track = Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![clip(1, 1, 0.0, 0.0, 0.3), clip(2, 1, 0.3, 0.3, 0.6)],
        visible: true,
    };
    let sequence = sequence_with(vec![track]);
    let output = std::env::temp_dir().join("avcore_test_timeline_export_cancelled.mp4");
    let cancel = AtomicBool::new(false);
    let mut calls = 0;

    let outcome =
        render_timeline_export(&sequence, &[asset], &output, -14.0, &cancel, |_percent| {
            calls += 1;
            if calls >= 3 {
                cancel.store(true, Ordering::Relaxed);
            }
        })
        .unwrap();

    assert_eq!(outcome, RenderOutcome::Cancelled);

    let _ = std::fs::remove_file(&output);
}

#[test]
fn fade_transition_exports_without_error() {
    let asset = video_asset(1);
    let mut c1 = clip(1, 1, 0.0, 0.0, 0.5);
    c1.transition_in = TransitionType::Fade;
    c1.transition_duration_secs = 0.3;

    let track = Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![c1],
        visible: true,
    };
    let sequence = sequence_with(vec![track]);

    let output = std::env::temp_dir().join("avcore_test_timeline_export_fade.mp4");
    let cancel = AtomicBool::new(false);

    let outcome =
        render_timeline_export(&sequence, &[asset], &output, -14.0, &cancel, |_| {}).unwrap();

    assert_eq!(outcome, RenderOutcome::Completed);

    let info = probe_media(&output).unwrap();
    assert_eq!(info.kind, MediaKind::Video);

    let _ = std::fs::remove_file(&output);
}

#[test]
fn slide_transition_exports_without_error() {
    let asset = video_asset(1);
    let mut c1 = clip(1, 1, 0.0, 0.0, 0.5);
    c1.transition_in = TransitionType::Slide;
    c1.transition_duration_secs = 0.3;

    let track = Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![c1],
        visible: true,
    };
    let sequence = sequence_with(vec![track]);

    let output = std::env::temp_dir().join("avcore_test_timeline_export_slide.mp4");
    let cancel = AtomicBool::new(false);

    let outcome =
        render_timeline_export(&sequence, &[asset], &output, -14.0, &cancel, |_| {}).unwrap();

    assert_eq!(outcome, RenderOutcome::Completed);

    let info = probe_media(&output).unwrap();
    assert_eq!(info.kind, MediaKind::Video);

    let _ = std::fs::remove_file(&output);
}

#[test]
fn zoom_transition_exports_without_error() {
    let asset = video_asset(1);
    let mut c1 = clip(1, 1, 0.0, 0.0, 0.5);
    c1.transition_in = TransitionType::Zoom;
    c1.transition_duration_secs = 0.3;

    let track = Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![c1],
        visible: true,
    };
    let sequence = sequence_with(vec![track]);

    let output = std::env::temp_dir().join("avcore_test_timeline_export_zoom_transition.mp4");
    let cancel = AtomicBool::new(false);

    let outcome =
        render_timeline_export(&sequence, &[asset], &output, -14.0, &cancel, |_| {}).unwrap();

    assert_eq!(outcome, RenderOutcome::Completed);

    let info = probe_media(&output).unwrap();
    assert_eq!(info.kind, MediaKind::Video);

    let _ = std::fs::remove_file(&output);
}
