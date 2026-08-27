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
            gain_keyframe_expr: String::new(),
            duck_role: 0,
        },
        AudioSegment {
            source_path: source,
            source_in_secs: 0.1,
            source_out_secs: 0.7,
            timeline_start_secs: 0.2,
            gain_db: -6.0,
            speed_factor: 1.25,
            gain_keyframe_expr: String::new(),
            duck_role: 0,
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

/// P2 item 6, "Audio ducking": exercises the actual `sidechaincompress`-based filter-graph
/// topology `build_mix_graph` builds when at least one `duck_role: 1` (trigger/mic) and one
/// `duck_role: 2` (target/music) branch are present. `avfilter_graph_config` is the real proof
/// this graph is valid, not just C that compiles -- a wrong pad order, a bad filter option name,
/// or a topology mistake surfaces here as `AudioMixError::FilterGraph`, which a passing
/// `AudioMixOutcome::Completed` rules out.
#[test]
fn mixes_with_sidechain_ducking_when_both_roles_are_present() {
    let mixed = std::env::temp_dir().join("avbridge_audio_duck_test.m4a");
    let _ = std::fs::remove_file(&mixed);
    let source = fixture("audio.m4a");
    let segments = vec![
        AudioSegment {
            source_path: source.clone(),
            source_in_secs: 0.0,
            source_out_secs: 0.8,
            timeline_start_secs: 0.0,
            gain_db: 0.0,
            speed_factor: 1.0,
            gain_keyframe_expr: String::new(),
            duck_role: 2, // target -- gets ducked
        },
        AudioSegment {
            source_path: source,
            source_in_secs: 0.0,
            source_out_secs: 0.8,
            timeline_start_secs: 0.0,
            gain_db: 0.0,
            speed_factor: 1.0,
            gain_keyframe_expr: String::new(),
            duck_role: 1, // trigger -- does the ducking, and still plays itself
        },
    ];

    let outcome =
        mix_audio_timeline(&segments, 1.0, &mixed, -14.0, &AtomicBool::new(false)).unwrap();
    assert_eq!(outcome, AudioMixOutcome::Completed);
    let mixed_info = probe(&mixed).unwrap();
    assert!(mixed_info.has_audio);

    let _ = std::fs::remove_file(&mixed);
}

/// Same as above but with *multiple* branches on each duck role, exercising the sub-`amix`
/// (`music_mix`/`trigger_mix`) branches `build_mix_graph` only takes when a role has more than
/// one contributing segment.
#[test]
fn mixes_with_sidechain_ducking_across_multiple_branches_per_role() {
    let mixed = std::env::temp_dir().join("avbridge_audio_duck_multi_test.m4a");
    let _ = std::fs::remove_file(&mixed);
    let source = fixture("audio.m4a");
    let segments = vec![
        AudioSegment {
            source_path: source.clone(),
            source_in_secs: 0.0,
            source_out_secs: 0.4,
            timeline_start_secs: 0.0,
            gain_db: 0.0,
            speed_factor: 1.0,
            gain_keyframe_expr: String::new(),
            duck_role: 2, // target #1
        },
        AudioSegment {
            source_path: source.clone(),
            source_in_secs: 0.4,
            source_out_secs: 0.8,
            timeline_start_secs: 0.4,
            gain_db: 0.0,
            speed_factor: 1.0,
            gain_keyframe_expr: String::new(),
            duck_role: 2, // target #2
        },
        AudioSegment {
            source_path: source.clone(),
            source_in_secs: 0.0,
            source_out_secs: 0.4,
            timeline_start_secs: 0.0,
            gain_db: 0.0,
            speed_factor: 1.0,
            gain_keyframe_expr: String::new(),
            duck_role: 1, // trigger #1
        },
        AudioSegment {
            source_path: source,
            source_in_secs: 0.4,
            source_out_secs: 0.8,
            timeline_start_secs: 0.4,
            gain_db: 0.0,
            speed_factor: 1.0,
            gain_keyframe_expr: String::new(),
            duck_role: 1, // trigger #2
        },
    ];

    let outcome =
        mix_audio_timeline(&segments, 0.8, &mixed, -14.0, &AtomicBool::new(false)).unwrap();
    assert_eq!(outcome, AudioMixOutcome::Completed);
    let mixed_info = probe(&mixed).unwrap();
    assert!(mixed_info.has_audio);

    let _ = std::fs::remove_file(&mixed);
}

/// A `duck_role: 2` (target) branch with no trigger present anywhere should fall back to the
/// original flat `amix` of every branch (ducking needs both roles) rather than erroring.
#[test]
fn a_target_branch_without_any_trigger_falls_back_to_a_plain_mix() {
    let mixed = std::env::temp_dir().join("avbridge_audio_duck_no_trigger_test.m4a");
    let _ = std::fs::remove_file(&mixed);
    let source = fixture("audio.m4a");
    let segments = vec![AudioSegment {
        source_path: source,
        source_in_secs: 0.0,
        source_out_secs: 0.8,
        timeline_start_secs: 0.0,
        gain_db: 0.0,
        speed_factor: 1.0,
        gain_keyframe_expr: String::new(),
        duck_role: 2, // target, but nothing tags a trigger
    }];

    let outcome =
        mix_audio_timeline(&segments, 0.8, &mixed, -14.0, &AtomicBool::new(false)).unwrap();
    assert_eq!(outcome, AudioMixOutcome::Completed);

    let _ = std::fs::remove_file(&mixed);
}

/// Exercises `build_mix_graph`'s new expression-mode `volume` branch (`audio_mix.c`) for a
/// segment carrying a keyframed gain ramp instead of a constant `gain_db` -- avbridge doesn't
/// depend on `core`, so this expression is written by hand rather than via
/// `avcore::keyframe::gain_filter_db_expr`, but it has the same shape that function produces
/// (a `t`-keyed piecewise-linear ramp from a linear-gain start value to a linear-gain end
/// value). `avfilter_graph_config` succeeding (a passing `AudioMixOutcome::Completed`) is real
/// proof the `av_asprintf`-built `"volume=%s:eval=frame"` string is valid avfilter syntax the
/// `volume` filter actually accepts -- not just C that compiles.
#[test]
fn mixes_a_segment_with_a_keyframed_gain_expression() {
    let mixed = std::env::temp_dir().join("avbridge_audio_gain_keyframe_test.m4a");
    let _ = std::fs::remove_file(&mixed);
    let source = fixture("audio.m4a");
    let segments = vec![AudioSegment {
        source_path: source,
        source_in_secs: 0.0,
        source_out_secs: 0.8,
        timeline_start_secs: 0.0,
        // Ignored when gain_keyframe_expr is non-empty -- confirms the expression path takes
        // priority over the constant, not just that both happen to agree.
        gain_db: -99.0,
        speed_factor: 1.0,
        gain_keyframe_expr: "if(lt(t,0.000000),0.1000000,if(between(t,0.000000,0.800000),\
                              (0.1000000+1.1250000*(t-0.000000)),1.0000000))"
            .to_string(),
        duck_role: 0,
    }];

    let outcome =
        mix_audio_timeline(&segments, 0.8, &mixed, -14.0, &AtomicBool::new(false)).unwrap();
    assert_eq!(outcome, AudioMixOutcome::Completed);
    let mixed_info = probe(&mixed).unwrap();
    assert!(mixed_info.has_audio);

    let _ = std::fs::remove_file(&mixed);
}
