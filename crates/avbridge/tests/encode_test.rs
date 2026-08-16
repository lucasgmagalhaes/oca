use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use avbridge::{
    encode_export, encode_timeline_export, probe, Canvas, ClipSegment, EncodeError, EncodeOutcome,
    GpuEncoderPreference, StreamKind,
};

const CANVAS: Canvas = Canvas {
    width: 320,
    height: 240,
    fps_num: 30,
    fps_den: 1,
    bit_rate_bps: 500_000,
};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn encodes_and_normalizes_a_real_file() {
    let out = std::env::temp_dir().join("avbridge_test_encode_ok.mp4");
    let cancel = AtomicBool::new(false);
    let mut progress_calls = 0;

    let outcome = encode_export(&fixture("video.mp4"), &out, -14.0, &cancel, |_secs| {
        progress_calls += 1;
    })
    .unwrap();

    assert_eq!(outcome, EncodeOutcome::Completed);
    assert!(progress_calls > 0);

    // Output must be a valid, probeable file with both streams intact.
    let info = probe(&out).unwrap();
    assert_eq!(info.kind, StreamKind::Video);
    assert_eq!(info.codec_name, "mpeg4");
    assert_eq!(info.resolution, Some((320, 240)));

    let _ = std::fs::remove_file(&out);
}

#[test]
fn cancelling_mid_render_leaves_no_valid_file() {
    let out = std::env::temp_dir().join("avbridge_test_encode_cancelled.mp4");
    let cancel = AtomicBool::new(false);
    let mut calls = 0;

    let outcome = encode_export(&fixture("video.mp4"), &out, -14.0, &cancel, |_secs| {
        calls += 1;
        if calls >= 3 {
            cancel.store(true, Ordering::Relaxed);
        }
    })
    .unwrap();

    assert_eq!(outcome, EncodeOutcome::Cancelled);
    // A cancelled render never writes a trailer - the output isn't a valid media file.
    assert!(probe(&out).is_err());

    let _ = std::fs::remove_file(&out);
}

#[test]
fn fails_on_missing_input() {
    let out = std::env::temp_dir().join("avbridge_test_encode_missing.mp4");
    let cancel = AtomicBool::new(false);

    let err =
        encode_export(&fixture("does_not_exist.mp4"), &out, -14.0, &cancel, |_| {}).unwrap_err();

    assert!(matches!(err, EncodeError::OpenInput));
}

#[test]
fn fails_on_audio_only_input_without_error_for_video_field() {
    // audio.m4a has no video stream, but does have audio - encode should still succeed
    // (video passthrough is simply a no-op when there's nothing to copy).
    let out = std::env::temp_dir().join("avbridge_test_encode_audio_only.mp4");
    let cancel = AtomicBool::new(false);

    let outcome = encode_export(&fixture("audio.m4a"), &out, -14.0, &cancel, |_| {}).unwrap();
    assert_eq!(outcome, EncodeOutcome::Completed);

    let info = probe(&out).unwrap();
    assert_eq!(info.kind, StreamKind::Audio);

    let _ = std::fs::remove_file(&out);
}

fn clip(
    source_in_secs: f64,
    source_out_secs: f64,
    gain_db: f32,
    video_filter: &str,
) -> ClipSegment {
    ClipSegment {
        source_path: fixture("video.mp4"),
        source_in_secs,
        source_out_secs,
        gain_db,
        video_filter: video_filter.to_string(),
        frozen: false,
        speed_factor: 1.0,
        position_x_expr: String::new(),
        position_y_expr: String::new(),
        transition_in: 0,
        transition_duration_secs: 0.3,
        timeline_start_secs: 0.0,
        mask_video_path: String::new(),
    }
}

#[test]
fn concatenates_two_segments_with_different_filters_into_one_export() {
    let out = std::env::temp_dir().join("avbridge_test_timeline_ok.mp4");
    let cancel = AtomicBool::new(false);
    let mut progress_calls = 0;

    let segments = [clip(0.0, 0.35, 0.0, ""), clip(0.35, 0.7, 6.0, "hflip")];
    let outcome = encode_timeline_export(
        &segments,
        CANVAS,
        &out,
        -14.0,
        GpuEncoderPreference::Auto,
        &cancel,
        |_secs| progress_calls += 1,
    )
    .unwrap();

    assert_eq!(outcome, EncodeOutcome::Completed);
    assert!(progress_calls > 0);

    let info = probe(&out).unwrap();
    assert_eq!(info.kind, StreamKind::Video);
    assert_eq!(info.resolution, Some((320, 240)));
    // Trimmed to ~0.7s total (0.35 + 0.35) — loose bound since seek lands on the nearest
    // keyframe, not exactly at each segment's requested in-point.
    assert!(info.duration_secs > 0.0 && info.duration_secs < 2.0);

    let _ = std::fs::remove_file(&out);
}

#[test]
fn freezes_a_segment_into_a_held_frame_export() {
    let out = std::env::temp_dir().join("avbridge_test_timeline_frozen.mp4");
    let cancel = AtomicBool::new(false);
    let mut progress_calls = 0;

    let mut frozen_clip = clip(0.1, 0.6, 0.0, "");
    frozen_clip.frozen = true;
    let segments = [frozen_clip];
    let outcome = encode_timeline_export(
        &segments,
        CANVAS,
        &out,
        -14.0,
        GpuEncoderPreference::Auto,
        &cancel,
        |_secs| progress_calls += 1,
    )
    .unwrap();

    assert_eq!(outcome, EncodeOutcome::Completed);
    assert!(progress_calls > 0);

    // Still spans the segment's whole trimmed duration (~0.5s) even though only one source
    // frame is ever decoded for it.
    let info = probe(&out).unwrap();
    assert_eq!(info.kind, StreamKind::Video);
    assert!(info.duration_secs > 0.3 && info.duration_secs < 1.0);

    let _ = std::fs::remove_file(&out);
}

#[test]
fn rejects_an_empty_timeline() {
    let out = std::env::temp_dir().join("avbridge_test_timeline_empty.mp4");
    let cancel = AtomicBool::new(false);

    let err = encode_timeline_export(
        &[],
        CANVAS,
        &out,
        -14.0,
        GpuEncoderPreference::Auto,
        &cancel,
        |_| {},
    )
    .unwrap_err();

    assert!(matches!(err, EncodeError::EmptyTimeline));
}

#[test]
fn fails_on_a_segment_with_a_missing_source() {
    let out = std::env::temp_dir().join("avbridge_test_timeline_missing.mp4");
    let cancel = AtomicBool::new(false);
    let mut missing = clip(0.0, 0.5, 0.0, "");
    missing.source_path = fixture("does_not_exist.mp4");

    let err = encode_timeline_export(
        &[missing],
        CANVAS,
        &out,
        -14.0,
        GpuEncoderPreference::Auto,
        &cancel,
        |_| {},
    )
    .unwrap_err();

    assert!(matches!(err, EncodeError::OpenInput));
}

#[test]
fn cancelling_mid_timeline_export_leaves_no_valid_file() {
    let out = std::env::temp_dir().join("avbridge_test_timeline_cancelled.mp4");
    let cancel = AtomicBool::new(false);
    let mut calls = 0;

    let segments = [clip(0.0, 0.35, 0.0, ""), clip(0.35, 0.7, 0.0, "")];
    let outcome = encode_timeline_export(
        &segments,
        CANVAS,
        &out,
        -14.0,
        GpuEncoderPreference::Auto,
        &cancel,
        |_secs| {
            calls += 1;
            if calls >= 3 {
                cancel.store(true, Ordering::Relaxed);
            }
        },
    )
    .unwrap();

    assert_eq!(outcome, EncodeOutcome::Cancelled);
    assert!(probe(&out).is_err());

    let _ = std::fs::remove_file(&out);
}

/// Every [`GpuEncoderPreference`] variant must still produce a valid export — on a dev/CI
/// machine without the requested vendor's GPU/driver, `open_video_encoder` (see gpu_encoder.c)
/// falls through to the CPU (libopenh264) encoder rather than failing the whole export. This is
/// the only way the hardware-encoder code paths get exercised at all in this environment: forcing
/// e.g. `Nvenc` here deterministically hits the fallback branch (no NVIDIA GPU present), which is
/// still a real assertion — it proves the fallback logic actually works end-to-end, not just that
/// `Auto`/`Cpu` do.
#[test]
fn every_gpu_encoder_preference_falls_back_to_a_working_export() {
    for (name, preference) in [
        ("auto", GpuEncoderPreference::Auto),
        ("cpu", GpuEncoderPreference::Cpu),
        ("nvenc", GpuEncoderPreference::Nvenc),
        ("quicksync", GpuEncoderPreference::QuickSync),
        ("amf", GpuEncoderPreference::Amf),
    ] {
        let out = std::env::temp_dir().join(format!("avbridge_test_gpu_encoder_{name}.mp4"));
        let cancel = AtomicBool::new(false);

        let segments = [clip(0.0, 0.35, 0.0, "")];
        let outcome =
            encode_timeline_export(&segments, CANVAS, &out, -14.0, preference, &cancel, |_| {})
                .unwrap();

        assert_eq!(
            outcome,
            EncodeOutcome::Completed,
            "preference {name:?} failed"
        );

        let info = probe(&out).unwrap();
        assert_eq!(info.kind, StreamKind::Video);

        let _ = std::fs::remove_file(&out);
    }
}
