//! Real end-to-end Whisper transcription tests. Needs an actual GGML model file this repo
//! can't bundle (see `crates/core/src/transcribe.rs`'s module docs) — gated on the
//! `WHISPER_MODEL_PATH` env var, same "documented setup step, not a default CI dependency"
//! pattern as `e2e/`'s build-first requirement. Skips (not fails) when unset.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use avcore::transcribe::{transcribe, TranscribeError, TranscribeOutcome};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn model_path() -> Option<PathBuf> {
    std::env::var("WHISPER_MODEL_PATH").ok().map(PathBuf::from)
}

#[test]
fn transcribes_real_audio_and_returns_at_least_one_segment() {
    // Also the regression test for the whisper-rs set_abort_callback_safe bug transcribe.rs
    // works around (see its doc comment): cancel is false throughout, so a correctly-behaving
    // abort_trampoline must return false every time it's polled and let this run to
    // completion. The original buggy version made whisper.cpp abort unconditionally on the
    // very first check regardless of what the closure returned, which turned this exact case
    // into a Err(Inference) instead of Ok(Completed(_)) - this test would have caught it.
    let Some(model) = model_path() else {
        eprintln!("skipping: WHISPER_MODEL_PATH not set");
        return;
    };
    let cancel = Arc::new(AtomicBool::new(false));

    let outcome = transcribe(&fixture("audio.m4a"), &model, None, &cancel, |_| {}).unwrap();

    let TranscribeOutcome::Completed(segments) = outcome else {
        panic!("expected Completed, got {outcome:?}");
    };
    for seg in &segments {
        assert!(seg.start_secs >= 0.0);
        assert!(seg.end_secs >= seg.start_secs);
        assert!(!seg.text.is_empty());
        // Word-highlight subtitles depend on every segment having usable per-word timing.
        assert!(!seg.words.is_empty(), "segment {seg:?} has no words");
        for word in &seg.words {
            assert!(!word.text.trim().is_empty());
            assert!(word.start_secs >= 0.0);
            assert!(word.end_secs >= word.start_secs);
        }
    }
}

#[test]
fn cancelling_before_the_call_skips_inference_entirely() {
    // No model needed - cancellation is checked before the model would even be loaded.
    let cancel = Arc::new(AtomicBool::new(true)); // already cancelled before the call starts

    let outcome = transcribe(
        &fixture("audio.m4a"),
        &fixture("does_not_exist.bin"),
        None,
        &cancel,
        |_| {},
    )
    .unwrap();

    assert_eq!(outcome, TranscribeOutcome::Cancelled(Vec::new()));
}

#[test]
fn fails_on_a_source_with_no_audio_stream() {
    let Some(model) = model_path() else {
        eprintln!("skipping: WHISPER_MODEL_PATH not set");
        return;
    };
    let cancel = Arc::new(AtomicBool::new(false));

    let err = transcribe(
        &fixture("does_not_exist.mp4"),
        &model,
        None,
        &cancel,
        |_| {},
    )
    .unwrap_err();

    assert!(matches!(err, TranscribeError::Audio(_)));
}

#[test]
fn fails_on_a_missing_model_file() {
    let cancel = Arc::new(AtomicBool::new(false));
    let bogus_model = fixture("does_not_exist.bin");

    let err = transcribe(&fixture("audio.m4a"), &bogus_model, None, &cancel, |_| {}).unwrap_err();

    assert!(matches!(err, TranscribeError::ModelLoad(_)));
}
