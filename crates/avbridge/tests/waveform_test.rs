use std::path::Path;

use avbridge::{generate_waveform, WaveformError};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn computes_peaks_for_a_real_audio_file() {
    let peaks = generate_waveform(&fixture("audio.m4a"), 64).unwrap();

    assert_eq!(peaks.len(), 64);
    for (min, max) in &peaks {
        assert!((-1.0..=1.0).contains(min), "min {min} out of range");
        assert!((-1.0..=1.0).contains(max), "max {max} out of range");
        assert!(min <= max, "bucket min {min} > max {max}");
    }
    // Real audio, not silence - at least one bucket must show actual amplitude.
    assert!(peaks.iter().any(|(min, max)| *min < 0.0 || *max > 0.0));
}

#[test]
fn fails_on_missing_input() {
    let err = generate_waveform(&fixture("does_not_exist.mp4"), 64).unwrap_err();
    assert!(matches!(err, WaveformError::OpenInput));
}

#[test]
fn fails_on_zero_bucket_count() {
    let err = generate_waveform(&fixture("audio.m4a"), 0).unwrap_err();
    assert!(matches!(err, WaveformError::Pipeline));
}
