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
fn computes_peaks_for_a_video_with_an_audio_track() {
    let peaks = generate_waveform(&fixture("video.mp4")).unwrap();

    assert_eq!(peaks.len(), WAVEFORM_BUCKET_COUNT);
    // The fixture has a synthesized 1000Hz tone — at least one bucket must show amplitude.
    assert!(peaks.iter().any(|(min, max)| *min < 0.0 || *max > 0.0));
}

#[test]
fn all_peaks_are_in_the_normalized_range() {
    let peaks = generate_waveform(&fixture("audio.m4a")).unwrap();

    for (min, max) in &peaks {
        assert!((-1.0..=1.0).contains(min), "min {min} out of [-1,1]");
        assert!((-1.0..=1.0).contains(max), "max {max} out of [-1,1]");
        assert!(min <= max, "bucket min {min} > max {max}");
    }
}

#[test]
fn errors_on_a_missing_source() {
    let result = generate_waveform(&fixture("does_not_exist.mp4"));
    assert!(result.is_err());
}
