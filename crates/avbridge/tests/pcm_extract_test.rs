// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
