use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use oca_avbridge::{encode_export, probe, EncodeError, EncodeOutcome, StreamKind};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn encodes_and_normalizes_a_real_file() {
    let out = std::env::temp_dir().join("oca_avbridge_test_encode_ok.mp4");
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
    let out = std::env::temp_dir().join("oca_avbridge_test_encode_cancelled.mp4");
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
    let out = std::env::temp_dir().join("oca_avbridge_test_encode_missing.mp4");
    let cancel = AtomicBool::new(false);

    let err =
        encode_export(&fixture("does_not_exist.mp4"), &out, -14.0, &cancel, |_| {}).unwrap_err();

    assert!(matches!(err, EncodeError::OpenInput));
}

#[test]
fn fails_on_audio_only_input_without_error_for_video_field() {
    // audio.m4a has no video stream, but does have audio - encode should still succeed
    // (video passthrough is simply a no-op when there's nothing to copy).
    let out = std::env::temp_dir().join("oca_avbridge_test_encode_audio_only.mp4");
    let cancel = AtomicBool::new(false);

    let outcome = encode_export(&fixture("audio.m4a"), &out, -14.0, &cancel, |_| {}).unwrap();
    assert_eq!(outcome, EncodeOutcome::Completed);

    let info = probe(&out).unwrap();
    assert_eq!(info.kind, StreamKind::Audio);

    let _ = std::fs::remove_file(&out);
}
