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

//! Media import, export path conflict, and transcription tests.

use super::support::*;
use super::*;

#[test]
fn pump_import_queue_adds_the_asset_to_its_target_project_with_a_fresh_id() {
    let mut app = test_app(
        vec![
            test_project(1, vec![test_asset(5)]),
            test_project(2, Vec::new()),
        ],
        Vec::new(),
    );
    app.import_state.pending_imports = 1;
    let mut ready_asset = test_asset(0);
    ready_asset.file_name = "clip.mp4".to_string();
    app.import_state
        .import_tx
        .send(ImportEvent::AssetReady {
            project_id: 1,
            import_token: 0,
            asset: ready_asset,
        })
        .unwrap();

    app.pump_import_queue();

    let project = app.projects.iter().find(|p| p.id == 1).unwrap();
    assert_eq!(project.media_library.len(), 2);
    let imported = project
        .media_library
        .iter()
        .find(|a| a.file_name == "clip.mp4")
        .unwrap();
    assert_eq!(imported.id, 6);
    assert_eq!(app.import_state.pending_imports, 0);
}

#[test]
fn pump_import_queue_targets_the_project_by_id_not_the_active_index() {
    let mut app = test_app(
        vec![test_project(1, Vec::new()), test_project(2, Vec::new())],
        Vec::new(),
    );
    app.active_project = 1; // Simulates the user switching projects mid-import.
    app.import_state
        .import_tx
        .send(ImportEvent::AssetReady {
            project_id: 1,
            import_token: 0,
            asset: test_asset(0),
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(
        app.projects
            .iter()
            .find(|p| p.id == 1)
            .unwrap()
            .media_library
            .len(),
        1
    );
    assert!(app
        .projects
        .iter()
        .find(|p| p.id == 2)
        .unwrap()
        .media_library
        .is_empty());
}

#[test]
fn pump_import_queue_applies_enrichment_to_the_asset_it_was_assigned() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(5)])], Vec::new());
    let mut ready_asset = test_asset(0);
    ready_asset.file_name = "clip.mp4".to_string();
    app.import_state
        .import_tx
        .send(ImportEvent::AssetReady {
            project_id: 1,
            import_token: 7,
            asset: ready_asset,
        })
        .unwrap();
    app.pump_import_queue();
    let loudness = LoudnessMetrics {
        integrated_lufs: -14.0,
        true_peak_dbtp: -1.0,
        loudness_range_lu: 6.0,
    };

    app.import_state
        .import_tx
        .send(ImportEvent::Enriched {
            project_id: 1,
            import_token: 7,
            loudness: Some(loudness),
            proxy_path: Some(PathBuf::from("proxy.mp4")),
            waveform_peaks: Some(vec![(-0.5, 0.5)]),
            duration_ms: 250,
        })
        .unwrap();
    app.pump_import_queue();

    let imported = app
        .active_project()
        .media_library
        .iter()
        .find(|a| a.file_name == "clip.mp4")
        .unwrap();
    assert_eq!(imported.loudness, Some(loudness));
    assert_eq!(imported.proxy_path, Some(PathBuf::from("proxy.mp4")));
    assert_eq!(imported.waveform_peaks, Some(vec![(-0.5, 0.5)]));
}

#[test]
fn pump_import_queue_ignores_enrichment_for_an_unknown_token() {
    let mut app = test_app(vec![test_project(1, vec![test_asset(5)])], Vec::new());
    app.import_state
        .import_tx
        .send(ImportEvent::Enriched {
            project_id: 1,
            import_token: 999,
            loudness: None,
            proxy_path: None,
            waveform_peaks: None,
            duration_ms: 0,
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(app.active_project().media_library.len(), 1);
}

#[test]
fn pump_import_queue_drops_a_failed_import_without_panicking() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.import_state.pending_imports = 1;
    app.import_state
        .import_tx
        .send(ImportEvent::Failed {
            path: PathBuf::from("missing.mp4"),
            message: "not found".to_string(),
        })
        .unwrap();

    app.pump_import_queue();

    assert_eq!(app.import_state.pending_imports, 0);
    assert!(app.active_project().media_library.is_empty());
}

#[test]
fn next_available_path_picks_the_first_free_numeric_suffix() {
    let dir = std::env::temp_dir().join("oca_test_next_available_path");
    let _ = std::fs::create_dir_all(&dir);
    let base = dir.join("clip.mp4");
    let taken2 = dir.join("clip (2).mp4");
    std::fs::write(&base, b"").unwrap();
    std::fs::write(&taken2, b"").unwrap();

    let result = super::export::next_available_path(&base);

    assert_eq!(result, dir.join("clip (3).mp4"));

    let _ = std::fs::remove_file(&base);
    let _ = std::fs::remove_file(&taken2);
}

#[test]
fn next_available_path_uses_suffix_two_when_only_the_base_name_exists() {
    let dir = std::env::temp_dir().join("oca_test_next_available_path_2");
    let _ = std::fs::create_dir_all(&dir);
    let base = dir.join("clip.mp4");
    std::fs::write(&base, b"").unwrap();

    let result = super::export::next_available_path(&base);

    assert_eq!(result, dir.join("clip (2).mp4"));

    let _ = std::fs::remove_file(&base);
}

#[test]
fn pending_export_conflict_overwrite_queues_with_the_original_path() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let dir = std::env::temp_dir().join("oca_test_export_conflict_overwrite");
    let _ = std::fs::create_dir_all(&dir);
    let output = dir.join("out.mp4");
    std::fs::write(&output, b"").unwrap();

    app.pending_export_conflict = Some(super::export::PendingExportConflict {
        title: "Export".to_string(),
        track_segments: Vec::new(),
        audio_segments: vec![],
        text_segments: vec![],
        shape_segments: vec![],
        privacy_blur_segments: vec![],
        canvas: test_canvas(),
        target_lufs: -14.0,
        output_path: output.clone(),
    });

    // Simulates the modal's Overwrite branch directly — queues with output_path unchanged.
    let pending = app.pending_export_conflict.take().unwrap();
    app.queue_export(
        pending.title,
        pending.track_segments,
        pending.audio_segments,
        pending.text_segments,
        pending.shape_segments,
        pending.privacy_blur_segments,
        pending.canvas,
        pending.target_lufs,
        pending.output_path.display().to_string(),
    );

    assert_eq!(app.export_jobs.len(), 1);
    assert_eq!(app.export_jobs[0].output_path, output.display().to_string());
    assert!(app.pending_export_conflict.is_none());

    let _ = std::fs::remove_file(&output);
}

#[test]
fn pump_transcribe_creates_a_text_track_with_rebased_word_timings() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 5.0;

    let segments = vec![avcore::transcribe::TranscribeSegment {
        start_secs: 100.0,
        end_secs: 101.0,
        text: "Hello World".to_string(),
        words: vec![
            avcore::transcribe::TranscribeWord {
                text: "Hello".to_string(),
                start_secs: 100.0,
                end_secs: 100.5,
                confidence: 0.9,
            },
            avcore::transcribe::TranscribeWord {
                text: "World".to_string(),
                start_secs: 100.5,
                end_secs: 101.0,
                confidence: 0.9,
            },
        ],
    }];
    app.transcribe_state
        .transcribe_tx
        .send(TranscribeEvent::Done {
            asset_id: 1,
            segments,
        })
        .unwrap();

    app.pump_transcribe();

    let text_track = app
        .active_project()
        .timeline()
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Text)
        .expect("no text track created");
    assert_eq!(text_track.text_clips.len(), 1);
    let tc = &text_track.text_clips[0];
    // Anchored at the playhead (5.0) plus the segment's own start_secs (100.0) - apply_
    // transcription preserves each segment's offset from the playhead rather than resetting
    // every segment to start exactly at it (see its doc comment).
    assert_eq!(tc.start_secs, 105.0);
    assert!(tc.highlight_enabled);
    assert_eq!(tc.words.len(), 2);
    // Rebased relative to the clip's own start, not the segment's absolute 100.0.
    assert_eq!(tc.words[0].start_secs, 0.0);
    assert_eq!(tc.words[0].end_secs, 0.5);
    assert_eq!(tc.words[1].start_secs, 0.5);
    assert_eq!(tc.words[1].end_secs, 1.0);
}
