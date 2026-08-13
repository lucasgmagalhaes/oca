//! Multi-track compositing integration tests — `avbridge_encode_timeline_export_multi` had zero
//! Rust coverage before this file, even though its two filter-string builders duplicate the same
//! geq-based Slide/Zoom-transition and Ken-Burns zoom fixes validated end-to-end for the
//! single-track path in `timeline_export_test.rs`. These exercise the real two-track composite
//! path (`resolve_timeline_segments_multi` + `render_export_job_multi`), not just the C string
//! builders in isolation.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use avcore::project::Sequence;
use avcore::render::{render_export_job_multi, resolve_timeline_segments_multi, RenderOutcome};
use avcore::timeline::{
    ClipInstance, ColorFilter, MaskShape, Timeline, Track, TrackKind, TransitionType,
};
use avcore::{probe_media, MediaAsset};

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

fn track(id: u64, name: &str, clips: Vec<ClipInstance>) -> Track {
    Track {
        id,
        name: name.to_string(),
        kind: TrackKind::Video,
        clips,
        text_clips: vec![],
        visible: true,
    }
}

fn sequence_with(tracks: Vec<Track>) -> Sequence {
    Sequence {
        id: 1,
        name: "Sequence 1".to_string(),
        timeline: Timeline { tracks, playhead_secs: 0.0 },
    }
}

fn render_multi(sequence: &Sequence, assets: &[MediaAsset], output_name: &str) -> RenderOutcome {
    let (track_segments, canvas) = resolve_timeline_segments_multi(sequence, assets).unwrap();
    let output = std::env::temp_dir().join(output_name);
    let cancel = AtomicBool::new(false);

    let outcome = render_export_job_multi(
        &track_segments,
        canvas,
        &output,
        -14.0,
        &[],
        &cancel,
        |_| {},
    )
    .unwrap();

    let _ = std::fs::remove_file(&output);
    outcome
}

#[test]
fn two_video_tracks_composite_into_one_export() {
    let asset = video_asset(1);
    let background = track(1, "V1", vec![clip(1, 1, 0.0, 0.0, 0.5)]);
    let overlay = track(2, "V2", vec![clip(2, 1, 0.0, 0.0, 0.5)]);
    let sequence = sequence_with(vec![background, overlay]);

    let outcome = render_multi(
        &sequence,
        std::slice::from_ref(&asset),
        "avcore_test_multi_composite_ok.mp4",
    );

    assert_eq!(outcome, RenderOutcome::Completed);
}

#[test]
fn resolve_timeline_segments_multi_skips_hidden_tracks() {
    let asset = video_asset(1);
    let background = track(1, "V1", vec![clip(1, 1, 0.0, 0.0, 0.5)]);
    let mut hidden_overlay = track(2, "V2", vec![clip(2, 1, 0.0, 0.0, 0.5)]);
    hidden_overlay.visible = false;
    let sequence = sequence_with(vec![background, hidden_overlay]);

    let (track_segments, _canvas) =
        resolve_timeline_segments_multi(&sequence, &[asset]).unwrap();

    assert_eq!(track_segments.len(), 1);
}

#[test]
fn overlay_track_slide_transition_composites_without_error() {
    let asset = video_asset(1);
    let background = track(1, "V1", vec![clip(1, 1, 0.0, 0.0, 0.5)]);
    let mut c2 = clip(2, 1, 0.0, 0.0, 0.5);
    c2.transition_in = TransitionType::Slide;
    c2.transition_duration_secs = 0.3;
    let overlay = track(2, "V2", vec![c2]);
    let sequence = sequence_with(vec![background, overlay]);

    let outcome = render_multi(
        &sequence,
        std::slice::from_ref(&asset),
        "avcore_test_multi_overlay_slide.mp4",
    );

    assert_eq!(outcome, RenderOutcome::Completed);
}

#[test]
fn overlay_track_zoom_transition_composites_without_error() {
    let asset = video_asset(1);
    let background = track(1, "V1", vec![clip(1, 1, 0.0, 0.0, 0.5)]);
    let mut c2 = clip(2, 1, 0.0, 0.0, 0.5);
    c2.transition_in = TransitionType::Zoom;
    c2.transition_duration_secs = 0.3;
    let overlay = track(2, "V2", vec![c2]);
    let sequence = sequence_with(vec![background, overlay]);

    let outcome = render_multi(
        &sequence,
        std::slice::from_ref(&asset),
        "avcore_test_multi_overlay_zoom.mp4",
    );

    assert_eq!(outcome, RenderOutcome::Completed);
}

#[test]
fn overlay_track_animated_ken_burns_zoom_composites_without_error() {
    let asset = video_asset(1);
    let background = track(1, "V1", vec![clip(1, 1, 0.0, 0.0, 0.5)]);
    let mut c2 = clip(2, 1, 0.0, 0.0, 0.5);
    c2.zoom_start = 1.0;
    c2.zoom_end = 1.5;
    let overlay = track(2, "V2", vec![c2]);
    let sequence = sequence_with(vec![background, overlay]);

    let outcome = render_multi(
        &sequence,
        std::slice::from_ref(&asset),
        "avcore_test_multi_overlay_ken_burns.mp4",
    );

    assert_eq!(outcome, RenderOutcome::Completed);
}

#[test]
fn overlay_track_mask_shape_composites_without_error() {
    let asset = video_asset(1);
    let background = track(1, "V1", vec![clip(1, 1, 0.0, 0.0, 0.5)]);
    let mut c2 = clip(2, 1, 0.0, 0.0, 0.5);
    c2.mask_shape = MaskShape::Circle;
    let overlay = track(2, "V2", vec![c2]);
    let sequence = sequence_with(vec![background, overlay]);

    let outcome = render_multi(
        &sequence,
        std::slice::from_ref(&asset),
        "avcore_test_multi_overlay_mask.mp4",
    );

    assert_eq!(outcome, RenderOutcome::Completed);
}

#[test]
fn cancelling_mid_multi_track_export_reports_cancelled() {
    let asset = video_asset(1);
    let background = track(1, "V1", vec![clip(1, 1, 0.0, 0.0, 0.5), clip(3, 1, 0.5, 0.0, 1.0)]);
    let overlay = track(2, "V2", vec![clip(2, 1, 0.0, 0.0, 0.5), clip(4, 1, 0.5, 0.0, 1.0)]);
    let sequence = sequence_with(vec![background, overlay]);

    let (track_segments, canvas) =
        resolve_timeline_segments_multi(&sequence, &[asset]).unwrap();
    let output = std::env::temp_dir().join("avcore_test_multi_cancelled.mp4");
    let cancel = AtomicBool::new(false);
    let mut calls = 0;

    let outcome = render_export_job_multi(
        &track_segments,
        canvas,
        &output,
        -14.0,
        &[],
        &cancel,
        |_percent| {
            calls += 1;
            if calls >= 3 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    )
    .unwrap();

    assert_eq!(outcome, RenderOutcome::Cancelled);

    let _ = std::fs::remove_file(&output);
}
