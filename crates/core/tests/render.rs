use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use avcore::render::{render_export, RenderOutcome};
use avcore::{measure_loudness, probe_media};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn renders_and_normalizes_loudness_toward_target() {
    let source = fixture("video.mp4");
    let output = std::env::temp_dir().join("avcore_test_render_ok.mp4");
    let duration_secs = probe_media(&source).unwrap().duration_secs;
    let cancel = AtomicBool::new(false);
    let mut last_percent = 0u8;

    let outcome = render_export(&source, &output, -14.0, duration_secs, &cancel, |percent| {
        last_percent = percent;
    })
    .unwrap();

    assert_eq!(outcome, RenderOutcome::Completed);
    assert_eq!(last_percent, 100);

    // Independent cross-check: measure_loudness is a *separate* code path (still spawns
    // ffmpeg as a subprocess) from render_export's FFI encode, so this genuinely verifies
    // the normalization happened rather than re-testing the same code against itself.
    let source_loudness = measure_loudness(&source).unwrap();
    let output_loudness = measure_loudness(&output).unwrap();
    assert!(output_loudness.integrated_lufs > source_loudness.integrated_lufs);
    assert!((output_loudness.integrated_lufs - (-14.0)).abs() < 2.0);

    let _ = std::fs::remove_file(&output);
}

#[test]
fn cancelling_mid_render_reports_cancelled() {
    let source = fixture("video.mp4");
    let output = std::env::temp_dir().join("avcore_test_render_cancelled.mp4");
    let duration_secs = probe_media(&source).unwrap().duration_secs;
    let cancel = AtomicBool::new(false);
    let mut calls = 0;

    let outcome = render_export(
        &source,
        &output,
        -14.0,
        duration_secs,
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

#[test]
fn rejects_a_missing_source() {
    let output = std::env::temp_dir().join("avcore_test_render_missing.mp4");
    let cancel = AtomicBool::new(false);

    let result = render_export(
        &fixture("does_not_exist.mp4"),
        &output,
        -14.0,
        1.0,
        &cancel,
        |_| {},
    );

    assert!(result.is_err());
}
