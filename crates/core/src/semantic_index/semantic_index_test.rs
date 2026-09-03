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

fn fingerprint(size: u64) -> MediaFingerprint {
    MediaFingerprint {
        file_size_bytes: size,
        modified_unix: 1000,
        duration_secs: 60.0,
    }
}

fn chunk(start: f64, end: f64, source: MatchSource, embedding: Vec<f32>) -> IndexedChunk {
    IndexedChunk {
        start_secs: start,
        end_secs: end,
        source,
        embedding,
        text: None,
    }
}

fn entry(
    media_id: u64,
    fp: MediaFingerprint,
    model_version: &str,
    chunks: Vec<IndexedChunk>,
) -> MediaIndexEntry {
    MediaIndexEntry {
        media_id,
        fingerprint: fp,
        model_version: model_version.to_string(),
        chunks,
    }
}

#[test]
fn needs_reindex_is_true_when_no_entry_exists() {
    let index = SemanticIndex::new();
    assert!(index.needs_reindex(1, fingerprint(100), "v1"));
}

#[test]
fn needs_reindex_is_false_when_fingerprint_and_model_version_match() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(1, fingerprint(100), "v1", vec![]));
    assert!(!index.needs_reindex(1, fingerprint(100), "v1"));
}

#[test]
fn needs_reindex_is_true_when_fingerprint_changed() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(1, fingerprint(100), "v1", vec![]));
    assert!(index.needs_reindex(1, fingerprint(200), "v1"));
}

#[test]
fn needs_reindex_is_true_when_model_version_changed() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(1, fingerprint(100), "v1", vec![]));
    assert!(index.needs_reindex(1, fingerprint(100), "v2"));
}

#[test]
fn upsert_entry_replaces_an_existing_entry_for_the_same_media_id() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![chunk(0.0, 1.0, MatchSource::Transcript, vec![1.0])],
    ));
    index.upsert_entry(entry(1, fingerprint(200), "v2", vec![]));
    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].fingerprint, fingerprint(200));
    assert_eq!(index.entries[0].model_version, "v2");
}

#[test]
fn remove_entry_reports_whether_it_actually_removed_something() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(1, fingerprint(100), "v1", vec![]));
    assert!(index.remove_entry(1));
    assert!(!index.remove_entry(1));
    assert!(index.entries.is_empty());
}

#[test]
fn total_chunks_sums_across_every_entry() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![
            chunk(0.0, 1.0, MatchSource::Transcript, vec![1.0]),
            chunk(1.0, 2.0, MatchSource::Transcript, vec![1.0]),
        ],
    ));
    index.upsert_entry(entry(
        2,
        fingerprint(200),
        "v1",
        vec![chunk(0.0, 1.0, MatchSource::Visual, vec![1.0])],
    ));
    assert_eq!(index.total_chunks(), 3);
}

#[test]
fn enforce_chunk_budget_evicts_the_oldest_whole_entry_first() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![
            chunk(0.0, 1.0, MatchSource::Transcript, vec![1.0]),
            chunk(1.0, 2.0, MatchSource::Transcript, vec![1.0]),
        ],
    ));
    index.upsert_entry(entry(
        2,
        fingerprint(200),
        "v1",
        vec![chunk(0.0, 1.0, MatchSource::Visual, vec![1.0])],
    ));

    index.enforce_chunk_budget(2);

    assert_eq!(index.entries.len(), 1);
    assert_eq!(index.entries[0].media_id, 2);
}

#[test]
fn enforce_chunk_budget_is_a_no_op_within_budget() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![chunk(0.0, 1.0, MatchSource::Transcript, vec![1.0])],
    ));
    index.enforce_chunk_budget(100);
    assert_eq!(index.entries.len(), 1);
}

#[test]
fn search_ranks_by_cosine_similarity_highest_first() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![
            chunk(0.0, 1.0, MatchSource::Transcript, vec![1.0, 0.0]),
            chunk(1.0, 2.0, MatchSource::Transcript, vec![0.0, 1.0]),
        ],
    ));

    let results = index.search(&[1.0, 0.0], "v1", 10);

    assert_eq!(results.len(), 2);
    assert!((results[0].score - 1.0).abs() < 1e-6);
    assert!(results[0].start_secs == 0.0);
    assert!(results[1].score < results[0].score);
}

#[test]
fn search_excludes_chunks_from_a_different_model_version() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![chunk(0.0, 1.0, MatchSource::Transcript, vec![1.0, 0.0])],
    ));

    let results = index.search(&[1.0, 0.0], "v2", 10);
    assert!(results.is_empty());
}

#[test]
fn search_respects_top_k() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![
            chunk(0.0, 1.0, MatchSource::Transcript, vec![1.0, 0.0]),
            chunk(1.0, 2.0, MatchSource::Transcript, vec![0.9, 0.1]),
            chunk(2.0, 3.0, MatchSource::Transcript, vec![0.0, 1.0]),
        ],
    ));

    let results = index.search(&[1.0, 0.0], "v1", 1);
    assert_eq!(results.len(), 1);
}

#[test]
fn search_seeks_to_the_matched_chunk_not_just_the_asset() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![
            chunk(0.0, 1.0, MatchSource::Transcript, vec![0.0, 1.0]),
            chunk(50.0, 51.0, MatchSource::Transcript, vec![1.0, 0.0]),
        ],
    ));

    let results = index.search(&[1.0, 0.0], "v1", 1);
    assert_eq!(results[0].start_secs, 50.0);
    assert_eq!(results[0].end_secs, 51.0);
}

#[test]
fn combine_search_results_merges_overlapping_hits_from_different_sources() {
    let results = vec![
        SearchResult {
            media_id: 1,
            start_secs: 10.0,
            end_secs: 11.0,
            source: MatchSource::Transcript,
            text: Some("boss fight".to_string()),
            score: 0.8,
        },
        SearchResult {
            media_id: 1,
            start_secs: 10.5,
            end_secs: 11.5,
            source: MatchSource::Visual,
            text: None,
            score: 0.6,
        },
    ];

    let combined = combine_search_results(results, DEFAULT_COMBINE_WINDOW_SECS);

    assert_eq!(combined.len(), 1);
    assert_eq!(combined[0].source, MatchSource::Combined);
    assert!((combined[0].score - 0.8).abs() < 1e-6); // (0.8+0.6)/2 + 0.1 = 0.8
    assert_eq!(combined[0].text, Some("boss fight".to_string()));
}

#[test]
fn combine_search_results_leaves_a_same_source_pair_unmerged() {
    let results = vec![
        SearchResult {
            media_id: 1,
            start_secs: 10.0,
            end_secs: 11.0,
            source: MatchSource::Transcript,
            text: None,
            score: 0.8,
        },
        SearchResult {
            media_id: 1,
            start_secs: 10.2,
            end_secs: 11.2,
            source: MatchSource::Transcript,
            text: None,
            score: 0.7,
        },
    ];

    let combined = combine_search_results(results, DEFAULT_COMBINE_WINDOW_SECS);
    assert_eq!(combined.len(), 2);
    assert!(combined.iter().all(|r| r.source == MatchSource::Transcript));
}

#[test]
fn combine_search_results_leaves_a_far_apart_pair_unmerged() {
    let results = vec![
        SearchResult {
            media_id: 1,
            start_secs: 0.0,
            end_secs: 1.0,
            source: MatchSource::Transcript,
            text: None,
            score: 0.8,
        },
        SearchResult {
            media_id: 1,
            start_secs: 100.0,
            end_secs: 101.0,
            source: MatchSource::Visual,
            text: None,
            score: 0.6,
        },
    ];

    let combined = combine_search_results(results, DEFAULT_COMBINE_WINDOW_SECS);
    assert_eq!(combined.len(), 2);
}

#[test]
fn combine_search_results_leaves_a_different_media_pair_unmerged() {
    let results = vec![
        SearchResult {
            media_id: 1,
            start_secs: 10.0,
            end_secs: 11.0,
            source: MatchSource::Transcript,
            text: None,
            score: 0.8,
        },
        SearchResult {
            media_id: 2,
            start_secs: 10.0,
            end_secs: 11.0,
            source: MatchSource::Visual,
            text: None,
            score: 0.6,
        },
    ];

    let combined = combine_search_results(results, DEFAULT_COMBINE_WINDOW_SECS);
    assert_eq!(combined.len(), 2);
}

#[test]
fn combine_search_results_clamps_the_bonus_to_1_0() {
    let results = vec![
        SearchResult {
            media_id: 1,
            start_secs: 10.0,
            end_secs: 11.0,
            source: MatchSource::Transcript,
            text: None,
            score: 0.99,
        },
        SearchResult {
            media_id: 1,
            start_secs: 10.2,
            end_secs: 11.2,
            source: MatchSource::Visual,
            text: None,
            score: 0.99,
        },
    ];

    let combined = combine_search_results(results, DEFAULT_COMBINE_WINDOW_SECS);
    assert_eq!(combined.len(), 1);
    assert!(combined[0].score <= 1.0);
}

#[test]
fn ocsi_framing_round_trips_a_real_index() {
    let mut index = SemanticIndex::new();
    index.upsert_entry(entry(
        1,
        fingerprint(100),
        "v1",
        vec![chunk(
            0.0,
            1.0,
            MatchSource::Transcript,
            vec![1.0, 2.0, 3.0],
        )],
    ));

    let bytes = crate::persistence::to_ocsi_bytes(&index).unwrap();
    assert_eq!(&bytes[..4], b"OCSI");
    let decoded: SemanticIndex = crate::persistence::from_ocsi_bytes(&bytes).unwrap();
    assert_eq!(decoded, index);
}

#[test]
fn ocsi_bytes_are_rejected_by_the_octr_decoder() {
    let index = SemanticIndex::new();
    let bytes = crate::persistence::to_ocsi_bytes(&index).unwrap();
    let result: Result<crate::transcript::TranscriptDocument, _> =
        crate::persistence::from_octr_bytes(&bytes);
    assert!(result.is_err());
}
