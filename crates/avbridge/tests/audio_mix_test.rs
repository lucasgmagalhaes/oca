// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use avbridge::{
    mix_audio_timeline, mux_video_audio, probe, AudioMixOutcome, AudioSegment, StreamKind,
};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn mixes_overlapping_audio_segments_and_muxes_them_without_reencoding_video() {
    let mixed = std::env::temp_dir().join("avbridge_audio_mix_test.m4a");
    let muxed = std::env::temp_dir().join("avbridge_audio_mux_test.mp4");
    let _ = std::fs::remove_file(&mixed);
    let _ = std::fs::remove_file(&muxed);
    let source = fixture("audio.m4a");
    let segments = vec![
        AudioSegment {
            source_path: source.clone(),
            source_in_secs: 0.0,
            source_out_secs: 0.8,
            timeline_start_secs: 0.0,
            gain_db: -3.0,
            speed_factor: 1.0,
        },
        AudioSegment {
            source_path: source,
            source_in_secs: 0.1,
            source_out_secs: 0.7,
            timeline_start_secs: 0.2,
            gain_db: -6.0,
            speed_factor: 1.25,
        },
    ];

    let outcome =
        mix_audio_timeline(&segments, 1.0, &mixed, -14.0, &AtomicBool::new(false)).unwrap();
    assert_eq!(outcome, AudioMixOutcome::Completed);
    let mixed_info = probe(&mixed).unwrap();
    assert_eq!(mixed_info.kind, StreamKind::Audio);
    assert!(mixed_info.has_audio);
    // The first branch ends at 0.8 s; the second ends at 0.2 + 0.6 / 1.25 = 0.68 s.
    // timeline_duration_secs is an upper bound, not a request to pad trailing silence.
    assert!((mixed_info.duration_secs - 0.8).abs() < 0.1);

    mux_video_audio(&fixture("video.mp4"), &mixed, &muxed).unwrap();
    let muxed_info = probe(&muxed).unwrap();
    assert_eq!(muxed_info.kind, StreamKind::Video);
    assert!(muxed_info.has_audio);
    assert_eq!(muxed_info.resolution, Some((320, 240)));
    assert_eq!(muxed_info.codec_name, "mpeg4");

    let _ = std::fs::remove_file(&mixed);
    let _ = std::fs::remove_file(&muxed);
}
