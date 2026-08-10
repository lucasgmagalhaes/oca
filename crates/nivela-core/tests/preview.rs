use std::path::{Path, PathBuf};

use nivela_core::preview::Preview;

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
