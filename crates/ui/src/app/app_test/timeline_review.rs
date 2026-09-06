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

//! Timeline index, marker, silence-review, and transcript-review tests.

use super::support::*;
use super::*;

#[test]
fn toggle_timeline_index_flips_the_open_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    assert!(!app.timeline_index_open);

    app.toggle_timeline_index();
    assert!(app.timeline_index_open);

    app.toggle_timeline_index();
    assert!(!app.timeline_index_open);
}

#[test]
fn add_marker_at_playhead_places_it_at_the_current_playhead() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 12.5;

    let id = app.add_marker_at_playhead(avcore::MarkerKind::Chapter);

    let timeline = app.active_project().timeline();
    let marker = timeline.markers.iter().find(|m| m.id == id).unwrap();
    assert_eq!(marker.position_secs, 12.5);
    assert_eq!(marker.kind, avcore::MarkerKind::Chapter);
}

#[test]
fn save_preview_snapshot_writes_the_last_decoded_frame_as_a_png() {
    // The PNG bytes themselves (does the pixel data round-trip correctly) are covered for real
    // by `core`'s own `video_frame_save_png_writes_a_real_decodable_png` test -- `image` isn't
    // a `ui` dependency, so this only confirms `save_preview_snapshot` actually forwards to
    // `VideoFrame::save_png` and lands a real file, via the fixed 8-byte signature every PNG
    // decoder checks first.
    let dir = std::env::temp_dir().join("oca_app_save_preview_snapshot_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let output_path = dir.join("snapshot.png");

    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.preview_state.last_frame = Some(avcore::preview::VideoFrame {
        width: 4,
        height: 2,
        rgba: vec![200u8; 4 * 2 * 4],
    });

    app.save_preview_snapshot(output_path.clone());

    let written = std::fs::read(&output_path).unwrap();
    assert_eq!(&written[..8], b"\x89PNG\r\n\x1a\n");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_preview_snapshot_toasts_instead_of_writing_when_no_frame_decoded_yet() {
    let dir = std::env::temp_dir().join("oca_app_save_preview_snapshot_test_empty");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let output_path = dir.join("snapshot.png");

    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.save_preview_snapshot(output_path.clone());

    assert!(!output_path.exists());
    assert_eq!(app.toasts.len(), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn remove_marker_deletes_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::Standard);

    app.remove_marker(id);

    assert!(app.active_project().timeline().markers.is_empty());
}

#[test]
fn set_marker_label_updates_the_right_marker() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::Standard);

    app.set_marker_label(id, "Needs a re-take".to_string());

    let timeline = app.active_project().timeline();
    assert_eq!(
        timeline.markers.iter().find(|m| m.id == id).unwrap().label,
        "Needs a re-take"
    );
}

#[test]
fn set_marker_kind_updates_the_right_marker() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::Standard);

    app.set_marker_kind(id, avcore::MarkerKind::ToDo);

    let timeline = app.active_project().timeline();
    assert_eq!(
        timeline.markers.iter().find(|m| m.id == id).unwrap().kind,
        avcore::MarkerKind::ToDo
    );
}

#[test]
fn toggle_marker_completed_flips_the_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let id = app.add_marker_at_playhead(avcore::MarkerKind::ToDo);

    app.toggle_marker_completed(id);
    assert!(
        app.active_project()
            .timeline()
            .markers
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .completed
    );

    app.toggle_marker_completed(id);
    assert!(
        !app.active_project()
            .timeline()
            .markers
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .completed
    );
}

#[test]
fn marker_mutations_are_no_ops_for_an_unknown_id() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.remove_marker(404);
    app.set_marker_label(404, "x".to_string());
    app.set_marker_kind(404, avcore::MarkerKind::Chapter);
    app.toggle_marker_completed(404);

    assert!(app.active_project().timeline().markers.is_empty());
}

fn silence_review_asset() -> MediaAsset {
    // 20 one-second buckets, loud except a silent run [8, 12).
    let mut peaks = vec![(-0.8, 0.8); 20];
    for p in &mut peaks[8..12] {
        *p = (0.0, 0.0);
    }
    MediaAsset {
        duration_secs: 20.0,
        waveform_peaks: Some(peaks),
        favorited: false,
        ..test_asset(1)
    }
}

#[test]
fn begin_silence_review_maps_a_detected_gap_into_timeline_coordinates() {
    // Clip shows source 5..15 starting at timeline 100 -- same setup as
    // avcore::silence_detection's own clip_silence_gaps test, exercised here end-to-end
    // through the App wrapper.
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.begin_silence_review();

    let review = app.silence_review.as_ref().expect("review should open");
    assert_eq!(review.track_id, 1);
    assert_eq!(review.gaps.len(), 1);
    assert_eq!(review.gaps[0].clip_id, 1);
    assert_eq!(review.gaps[0].gap.start_secs, 103.0);
    assert_eq!(review.gaps[0].gap.end_secs, 107.0);
    assert!(review.gaps[0].accepted, "gaps default to accepted");
}

#[test]
fn begin_silence_review_without_a_selected_clip_toasts_instead_of_opening() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.begin_silence_review();

    assert!(app.silence_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn begin_silence_review_skips_clips_whose_asset_has_no_cached_waveform() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.begin_silence_review();

    assert!(app.silence_review.as_ref().unwrap().gaps.is_empty());
}

#[test]
fn toggle_silence_gap_accepted_flips_only_the_targeted_entry() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.begin_silence_review();

    app.toggle_silence_gap_accepted(0);

    assert!(!app.silence_review.as_ref().unwrap().gaps[0].accepted);
}

#[test]
fn apply_silence_review_ripple_deletes_only_accepted_gaps_and_closes_the_modal() {
    // Two clips, each with its own silent run, on the same track.
    let track = test_track(
        1,
        TrackKind::Audio,
        vec![test_clip(1, 0.0, 0.0, 20.0), test_clip(2, 20.0, 0.0, 20.0)],
    );
    let mut asset2 = silence_review_asset();
    asset2.id = 2;
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset(), asset2],
        )],
        Vec::new(),
    );
    // clip 1 uses asset 1, clip 2 uses asset 2 -- fix up asset_id on the second clip.
    app.active_project_mut().timeline_mut().tracks[0].clips[1].asset_id = 2;
    app.selected_clip_id = Some(1);
    app.begin_silence_review();
    assert_eq!(app.silence_review.as_ref().unwrap().gaps.len(), 2);

    // Reject the first clip's gap; only the second clip's 4s gap should actually be cut.
    app.toggle_silence_gap_accepted(0);
    app.apply_silence_review();

    assert!(app.silence_review.is_none(), "modal closes after apply");
    let track = &app.active_project().timeline().tracks[0];
    assert_eq!(
        track.clips.len(),
        3,
        "clip 1 kept whole, clip 2 split in two"
    );
    let total_duration: f64 = track.clips.iter().map(|c| c.duration_secs()).sum();
    assert_eq!(
        total_duration, 36.0,
        "only the second clip's 4s silent run was removed (20 + 20 - 4)"
    );
}

#[test]
fn apply_silence_review_with_nothing_accepted_is_a_no_op() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.begin_silence_review();
    app.toggle_silence_gap_accepted(0);

    app.apply_silence_review();

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].duration_secs(),
        10.0,
        "nothing was accepted -- the clip is untouched"
    );
}

// --- CF-01 slices 4-5: speech-edit proposal review/apply ---

/// One transcript word at the given media-relative times, with a stable id.
fn transcript_word(id: u64, text: &str, start_secs: f64, end_secs: f64) -> avcore::TranscriptWord {
    avcore::TranscriptWord {
        id,
        text: text.to_string(),
        start_secs,
        end_secs,
        confidence: 0.9,
        speaker: None,
    }
}

/// A transcript that yields exactly two non-overlapping proposals: a dead-air cut in `[0.5,2.0)`
/// and a retake (the second "the") in `[3.5,4.0)`.
fn two_proposal_document() -> avcore::TranscriptDocument {
    avcore::TranscriptDocument {
        schema_version: 1,
        asset_id: 1,
        language: Some("en".to_string()),
        words: vec![
            transcript_word(1, "hello", 0.0, 0.5),
            transcript_word(2, "world", 2.0, 2.5),
            transcript_word(3, "the", 3.0, 3.5),
            transcript_word(4, "the", 3.5, 4.0),
        ],
    }
}

/// Builds an App whose active project holds `track` + `assets` and whose asset 1's transcript
/// sidecar is already written on disk (under `project.file_path`'s cache dir). Returns the
/// `TempDir` alongside so the sidecar survives the test body.
fn transcript_proposals_app_with(
    track: Track,
    assets: Vec<MediaAsset>,
    doc: &avcore::TranscriptDocument,
) -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(1, vec![track], assets);
    project.file_path = Some(dir.path().join("proj.ocproj"));
    let cache_dir = avcore::transcript_cache_dir_for_project(&project);
    avcore::save_transcript_document(&cache_dir, doc).unwrap();
    (test_app(vec![project], Vec::new()), dir)
}

#[test]
fn begin_transcript_proposals_stages_detected_edits_for_the_previewed_clip() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;

    app.begin_transcript_proposals();

    let review = app.transcript_review.as_ref().expect("review should open");
    assert_eq!(review.track_id, 1);
    assert_eq!(review.clip_id, 1);
    assert_eq!(review.proposals.len(), 2);
    assert_eq!(
        review.proposals[0].proposal.kind,
        avcore::TranscriptEditKind::DeadAir,
        "staged earliest-first"
    );
    assert_eq!(
        review.proposals[1].proposal.kind,
        avcore::TranscriptEditKind::Retake
    );
    assert!(
        review.proposals.iter().all(|e| e.accepted),
        "proposals default to accepted"
    );
}

#[test]
fn begin_transcript_proposals_without_a_previewed_clip_toasts() {
    let track = test_track(1, TrackKind::Video, vec![]);
    let (mut app, _dir) =
        transcript_proposals_app_with(track, vec![test_asset(1)], &two_proposal_document());
    app.active_project_mut().timeline_mut().playhead_secs = 0.0;

    app.begin_transcript_proposals();

    assert!(app.transcript_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn begin_transcript_proposals_without_a_saved_transcript_toasts() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let dir = tempfile::tempdir().unwrap();
    let mut project = test_project_with_tracks_and_assets(1, vec![track], vec![test_asset(1)]);
    project.file_path = Some(dir.path().join("proj.ocproj"));
    let mut app = test_app(vec![project], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;

    app.begin_transcript_proposals();

    assert!(app.transcript_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn begin_transcript_proposals_with_nothing_detected_toasts_without_opening() {
    // Contiguous words, no fillers/repeats/gaps -- nothing to propose.
    let doc = avcore::TranscriptDocument {
        schema_version: 1,
        asset_id: 1,
        language: Some("en".to_string()),
        words: vec![
            transcript_word(1, "one", 0.0, 0.4),
            transcript_word(2, "two", 0.4, 0.8),
            transcript_word(3, "three", 0.8, 1.2),
        ],
    };
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;

    app.begin_transcript_proposals();

    assert!(app.transcript_review.is_none());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn toggle_transcript_proposal_flips_only_the_targeted_entry() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();

    app.toggle_transcript_proposal(0);

    assert!(!app.transcript_review.as_ref().unwrap().proposals[0].accepted);
    assert!(app.transcript_review.as_ref().unwrap().proposals[1].accepted);
}

#[test]
fn apply_transcript_proposals_ripple_deletes_only_accepted_and_closes() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();
    assert_eq!(app.transcript_review.as_ref().unwrap().proposals.len(), 2);

    // Reject the retake (index 1); only the 1.5s dead air should actually be cut.
    app.toggle_transcript_proposal(1);
    app.apply_transcript_proposals();

    assert!(app.transcript_review.is_none(), "modal closes after apply");
    let track = &app.active_project().timeline().tracks[0];
    assert_eq!(track.clips.len(), 2, "dead-air cut splits the clip in two");
    let total: f64 = track.clips.iter().map(|c| c.duration_secs()).sum();
    assert!(
        (total - 8.5).abs() < 1e-6,
        "only the 1.5s dead air was removed; got {total}"
    );
}

#[test]
fn apply_transcript_proposals_with_nothing_accepted_is_a_no_op() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();
    app.toggle_transcript_proposal(0);
    app.toggle_transcript_proposal(1);

    app.apply_transcript_proposals();

    let track = &app.active_project().timeline().tracks[0];
    assert_eq!(track.clips.len(), 1);
    assert_eq!(track.clips[0].duration_secs(), 10.0);
}

#[test]
fn apply_transcript_proposals_when_the_clip_was_deleted_is_a_no_op() {
    let doc = two_proposal_document();
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let (mut app, _dir) = transcript_proposals_app_with(track, vec![test_asset(1)], &doc);
    app.active_project_mut().timeline_mut().playhead_secs = 1.0;
    app.begin_transcript_proposals();
    assert!(app.transcript_review.is_some());
    app.active_project_mut().timeline_mut().tracks[0]
        .clips
        .clear();

    app.apply_transcript_proposals();

    assert!(
        app.transcript_review.is_none(),
        "apply still closes the modal even though nothing could be cut"
    );
}

#[test]
fn close_silence_review_discards_the_staged_review() {
    let track = test_track(1, TrackKind::Audio, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![silence_review_asset()],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.begin_silence_review();

    app.close_silence_review();

    assert!(app.silence_review.is_none());
}
