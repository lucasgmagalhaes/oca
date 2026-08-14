use std::path::Path;

use avbridge::{extract_pcm_16k_mono, PcmError};

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn extracts_16k_mono_samples_from_a_real_audio_file() {
    let samples = extract_pcm_16k_mono(&fixture("audio.m4a")).unwrap();

    assert!(!samples.is_empty());
    for s in &samples {
        assert!(s.is_finite(), "sample must be finite, got {s}");
    }
    // Real audio, not silence - at least one sample must show actual amplitude.
    assert!(samples.iter().any(|s| s.abs() > 0.0));
}

#[test]
fn sample_count_matches_16khz_for_the_fixtures_known_duration() {
    // audio.m4a and video.mp4 are the same short fixture clip used throughout this crate's
    // tests (both under a couple seconds) - resampled to 16kHz, sample count should land in a
    // loose neighborhood of duration_secs * 16000, not some wildly different rate.
    let samples = extract_pcm_16k_mono(&fixture("audio.m4a")).unwrap();
    let implied_duration_secs = samples.len() as f64 / 16000.0;

    assert!(
        implied_duration_secs > 0.1 && implied_duration_secs < 30.0,
        "implied duration {implied_duration_secs}s looks wrong for a short fixture clip"
    );
}

#[test]
fn works_on_a_video_files_embedded_audio_track_too() {
    let samples = extract_pcm_16k_mono(&fixture("video.mp4")).unwrap();
    assert!(!samples.is_empty());
}

#[test]
fn fails_on_missing_input() {
    let err = extract_pcm_16k_mono(&fixture("does_not_exist.mp4")).unwrap_err();
    assert!(matches!(err, PcmError::OpenInput));
}
