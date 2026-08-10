use std::path::{Path, PathBuf};

use avcore::preview::Preview;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn opens_a_real_file_and_reports_duration() {
    let preview = Preview::open(&fixture("video.mp4")).unwrap();
    let duration = preview
        .duration_secs()
        .expect("duration should be known after preroll");
    assert!((duration - 1.0).abs() < 0.2);
}

#[test]
fn play_then_pause_does_not_error() {
    let preview = Preview::open(&fixture("video.mp4")).unwrap();
    preview.play().unwrap();
    preview.pause().unwrap();
}

#[test]
fn seek_succeeds_and_position_stays_in_bounds() {
    let preview = Preview::open(&fixture("video.mp4")).unwrap();
    let duration = preview.duration_secs().unwrap();

    preview.seek(duration / 2.0).unwrap();

    if let Some(position) = preview.position_secs() {
        assert!(position >= 0.0 && position <= duration + 0.5);
    }
}

#[test]
fn errors_on_a_missing_file() {
    assert!(Preview::open(&fixture("does_not_exist.mp4")).is_err());
}

#[test]
fn current_frame_returns_correctly_sized_rgba() {
    let preview = Preview::open(&fixture("video.mp4")).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");

    // video.mp4 is a 320x240 fixture (see tests/probe_test.rs / avbridge's fixture generation).
    assert_eq!(frame.width, 320);
    assert_eq!(frame.height, 240);
    assert_eq!(frame.rgba.len(), 320 * 240 * 4);
    // Not all-zero — testsrc paints an actual pattern, a blank buffer would mean the caps
    // negotiation or plane extraction silently produced garbage/empty data.
    assert!(frame.rgba.iter().any(|&b| b != 0));
}

#[test]
fn current_frame_is_none_for_audio_only_input() {
    let preview = Preview::open(&fixture("audio.m4a")).unwrap();
    assert!(preview.current_frame().is_none());
}
