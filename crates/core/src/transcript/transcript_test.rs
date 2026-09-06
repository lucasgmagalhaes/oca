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

use super::*;
use crate::transcribe::TranscribeWord;

fn segments() -> Vec<TranscribeSegment> {
    vec![
        TranscribeSegment {
            start_secs: 0.0,
            end_secs: 1.0,
            text: "hello world".to_string(),
            words: vec![
                TranscribeWord {
                    text: "hello".to_string(),
                    start_secs: 0.0,
                    end_secs: 0.4,
                    confidence: 0.95,
                },
                TranscribeWord {
                    text: "world".to_string(),
                    start_secs: 0.4,
                    end_secs: 1.0,
                    confidence: 0.9,
                },
            ],
        },
        TranscribeSegment {
            start_secs: 1.0,
            end_secs: 2.0,
            text: "again".to_string(),
            words: vec![TranscribeWord {
                text: "again".to_string(),
                start_secs: 1.0,
                end_secs: 2.0,
                confidence: 0.8,
            }],
        },
    ]
}

#[test]
fn from_transcribe_segments_flattens_words_with_sequential_ids() {
    let doc = TranscriptDocument::from_transcribe_segments(1, Some("en".to_string()), &segments());
    assert_eq!(doc.schema_version, TRANSCRIPT_SCHEMA_VERSION);
    assert_eq!(doc.asset_id, 1);
    assert_eq!(doc.language.as_deref(), Some("en"));
    assert_eq!(doc.words.len(), 3);
    assert_eq!(doc.words[0].id, 1);
    assert_eq!(doc.words[1].id, 2);
    assert_eq!(doc.words[2].id, 3);
    assert_eq!(doc.words[0].text, "hello");
    assert_eq!(doc.words[2].text, "again");
    assert!(doc.words.iter().all(|w| w.speaker.is_none()));
}

#[test]
fn from_transcribe_segments_is_empty_for_no_segments() {
    let doc = TranscriptDocument::from_transcribe_segments(1, None, &[]);
    assert!(doc.words.is_empty());
}

#[test]
fn validate_accepts_a_well_formed_document() {
    let doc = TranscriptDocument::from_transcribe_segments(1, None, &segments());
    assert!(doc.validate().is_ok());
}

#[test]
fn validate_rejects_a_newer_schema_version() {
    let mut doc = TranscriptDocument::from_transcribe_segments(1, None, &segments());
    doc.schema_version = TRANSCRIPT_SCHEMA_VERSION + 1;
    assert_eq!(
        doc.validate(),
        Err(TranscriptValidationError::UnsupportedSchemaVersion(
            TRANSCRIPT_SCHEMA_VERSION + 1
        ))
    );
}

#[test]
fn validate_rejects_a_duplicate_word_id() {
    let mut doc = TranscriptDocument::from_transcribe_segments(1, None, &segments());
    doc.words[1].id = doc.words[0].id;
    assert_eq!(
        doc.validate(),
        Err(TranscriptValidationError::DuplicateWordId(doc.words[0].id))
    );
}

#[test]
fn validate_rejects_a_negative_start() {
    let mut doc = TranscriptDocument::from_transcribe_segments(1, None, &segments());
    doc.words[0].start_secs = -1.0;
    assert!(matches!(
        doc.validate(),
        Err(TranscriptValidationError::InvalidWord { .. })
    ));
}

#[test]
fn validate_rejects_end_before_start() {
    let mut doc = TranscriptDocument::from_transcribe_segments(1, None, &segments());
    doc.words[0].end_secs = doc.words[0].start_secs;
    assert!(matches!(
        doc.validate(),
        Err(TranscriptValidationError::InvalidWord { .. })
    ));
}

#[test]
fn validate_rejects_out_of_range_confidence() {
    let mut doc = TranscriptDocument::from_transcribe_segments(1, None, &segments());
    doc.words[0].confidence = 1.5;
    assert!(matches!(
        doc.validate(),
        Err(TranscriptValidationError::InvalidWord { .. })
    ));
}

#[test]
fn validate_rejects_nan_confidence() {
    let mut doc = TranscriptDocument::from_transcribe_segments(1, None, &segments());
    doc.words[0].confidence = f32::NAN;
    assert!(matches!(
        doc.validate(),
        Err(TranscriptValidationError::InvalidWord { .. })
    ));
}

#[test]
fn save_and_load_round_trips_a_document() {
    let dir = std::env::temp_dir().join(format!(
        "oca_transcript_test_round_trip_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let doc = TranscriptDocument::from_transcribe_segments(42, Some("pt".to_string()), &segments());

    let path = save_transcript_document(&dir, &doc).unwrap();
    assert!(path.exists());

    let loaded = load_transcript_document(&dir, 42).unwrap().unwrap();
    assert_eq!(loaded, doc);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_returns_none_for_a_missing_asset() {
    let dir = std::env::temp_dir().join(format!(
        "oca_transcript_test_missing_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let loaded = load_transcript_document(&dir, 999).unwrap();
    assert!(loaded.is_none());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn load_rejects_a_corrupt_file_via_validate() {
    // A document that parses cleanly (correct framing/schema) but fails semantic validation
    // (start_secs < 0) must not silently load as if nothing were wrong.
    let dir = std::env::temp_dir().join(format!(
        "oca_transcript_test_invalid_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let mut doc = TranscriptDocument::from_transcribe_segments(7, None, &segments());
    doc.words[0].start_secs = -5.0;
    let bytes = crate::persistence::to_octr_bytes(&doc).unwrap();
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(transcript_path(&dir, 7), bytes).unwrap();

    let result = load_transcript_document(&dir, 7);
    assert!(matches!(
        result,
        Err(TranscriptStorageError::Validation(
            TranscriptValidationError::InvalidWord { .. }
        ))
    ));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_real_transcription_builds_a_valid_document() {
    // Real end-to-end: actual Whisper inference against a real fixture, through
    // from_transcribe_segments, through validate() -- not just synthetic segments(). Skips (not
    // fails) without a model, same convention transcribe_test.rs's own real-inference test uses.
    let Some(model) = std::env::var("WHISPER_MODEL_PATH")
        .ok()
        .map(std::path::PathBuf::from)
    else {
        eprintln!("skipping: WHISPER_MODEL_PATH not set");
        return;
    };
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join("audio.m4a");
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let outcome = crate::transcribe::transcribe(&fixture, &model, None, &cancel, |_| {}).unwrap();
    let crate::transcribe::TranscribeOutcome::Completed(transcribed_segments) = outcome else {
        panic!("expected Completed");
    };

    let doc = TranscriptDocument::from_transcribe_segments(1, None, &transcribed_segments);
    doc.validate()
        .expect("a real transcription should build a valid document");
    assert!(!doc.words.is_empty());
    // Stable, sequential, 1-based -- exactly from_transcribe_segments' own documented contract,
    // now checked against real inference output instead of a hand-built fixture.
    for (index, word) in doc.words.iter().enumerate() {
        assert_eq!(word.id, index as u64 + 1);
    }
}
