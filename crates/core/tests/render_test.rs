use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use avcore::project::Sequence;
use avcore::render::{render_export, resolve_text_segments, RenderOutcome};
use avcore::timeline::{TextClip, Timeline, Track, TrackKind, WordTiming};
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

fn text_clip(words: Vec<WordTiming>, highlight_enabled: bool) -> TextClip {
    TextClip {
        id: 1,
        start_secs: 10.0,
        duration_secs: 2.0,
        text: "Hello World".to_string(),
        font_size: 48.0,
        color_rgba: [255, 255, 255, 255],
        pos_x: 0.1,
        pos_y: 0.85,
        words,
        highlight_enabled,
        highlight_color_rgba: [255, 220, 0, 255],
    }
}

fn sequence_with_text_track(clip: TextClip) -> Sequence {
    Sequence {
        id: 1,
        name: "Sequence 1".to_string(),
        timeline: Timeline {
            tracks: vec![Track {
                id: 1,
                name: "Legendas".to_string(),
                kind: TrackKind::Text,
                clips: vec![],
                text_clips: vec![clip],
                visible: true,
            }],
            playhead_secs: 0.0,
        },
    }
}

#[test]
fn resolve_text_segments_returns_one_segment_for_a_plain_text_clip() {
    let sequence = sequence_with_text_track(text_clip(Vec::new(), false));

    let segments = resolve_text_segments(&sequence, 1920);

    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].text, "Hello World");
    assert_eq!(segments[0].start_secs, 10.0);
    assert_eq!(segments[0].duration_secs, 2.0);
    assert_eq!(segments[0].pos_x, 0.1);
}

#[test]
fn resolve_text_segments_ignores_words_when_highlight_is_disabled() {
    let words = vec![
        WordTiming {
            text: "Hello".to_string(),
            start_secs: 0.0,
            end_secs: 0.5,
        },
        WordTiming {
            text: "World".to_string(),
            start_secs: 0.5,
            end_secs: 1.0,
        },
    ];
    let sequence = sequence_with_text_track(text_clip(words, false));

    let segments = resolve_text_segments(&sequence, 1920);

    assert_eq!(segments.len(), 1);
}

#[test]
fn resolve_text_segments_expands_a_highlighted_clip_into_a_base_plus_one_segment_per_word() {
    let words = vec![
        WordTiming {
            text: "Hello".to_string(),
            start_secs: 0.0,
            end_secs: 0.5,
        },
        WordTiming {
            text: "World".to_string(),
            start_secs: 0.5,
            end_secs: 1.0,
        },
    ];
    let sequence = sequence_with_text_track(text_clip(words, true));

    let segments = resolve_text_segments(&sequence, 1920);

    // 1 base (full sentence, whole clip duration) + 2 word overlays.
    assert_eq!(segments.len(), 3);

    let base = segments
        .iter()
        .find(|s| s.text == "Hello World")
        .expect("base segment missing");
    assert_eq!(base.start_secs, 10.0);
    assert_eq!(base.duration_secs, 2.0);
    assert_eq!(base.color_rgba, [255, 255, 255, 255]);

    let word_segments: Vec<_> = segments
        .iter()
        .filter(|s| s.text != "Hello World")
        .collect();
    assert_eq!(word_segments.len(), 2);
    for word in &word_segments {
        assert_eq!(word.color_rgba, [255, 220, 0, 255]);
        // Timeline-absolute: clip.start_secs (10.0) + the word's own relative offset.
        assert!(word.start_secs >= 10.0 && word.start_secs < 11.5);
    }

    // "World" (the second word) must be positioned strictly to the right of "Hello" (the
    // first) - proves text_metrics actually drove the x offset, not just a constant pos_x.
    let hello = segments.iter().find(|s| s.text == "Hello").unwrap();
    let world = segments.iter().find(|s| s.text == "World").unwrap();
    assert!(world.pos_x > hello.pos_x);
    // Both still anchored relative to the base clip's own pos_x (0.1), not drifted off-screen.
    assert!(hello.pos_x >= 0.1);
}

#[test]
fn resolve_text_segments_falls_back_to_one_segment_when_canvas_width_is_zero() {
    let words = vec![WordTiming {
        text: "Hello".to_string(),
        start_secs: 0.0,
        end_secs: 0.5,
    }];
    let sequence = sequence_with_text_track(text_clip(words, true));

    let segments = resolve_text_segments(&sequence, 0);

    assert_eq!(segments.len(), 1);
}
