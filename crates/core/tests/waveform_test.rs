use std::path::{Path, PathBuf};

use avcore::waveform::{generate_waveform, WAVEFORM_BUCKET_COUNT};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn computes_a_full_resolution_peak_table_for_a_real_audio_file() {
    let peaks = generate_waveform(&fixture("audio.m4a")).unwrap();

    assert_eq!(peaks.len(), WAVEFORM_BUCKET_COUNT);
    assert!(peaks.iter().any(|(min, max)| *min < 0.0 || *max > 0.0));
}

#[test]
fn errors_on_a_missing_source() {
    let result = generate_waveform(&fixture("does_not_exist.mp4"));
    assert!(result.is_err());
}
