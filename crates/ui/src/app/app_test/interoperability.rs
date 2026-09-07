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

//! Collaboration bundle, OTIO, scene-cut, and chapter export tests.

use super::support::*;
use super::*;

fn collab_bundle_scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oca_app_collab_bundle_test_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn export_collab_bundle_writes_a_zip_and_toasts_success() {
    let dir = collab_bundle_scratch_dir("export_ok");
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let output_path = dir.join("handoff.zip");

    app.export_collab_bundle(output_path.clone());

    assert!(output_path.exists());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn export_collab_bundle_toasts_on_failure_instead_of_panicking() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    // A parent directory that doesn't exist -- File::create fails.
    let output_path = PathBuf::from("/nonexistent-oca-test-dir/handoff.zip");

    app.export_collab_bundle(output_path);

    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_collab_bundle_opens_the_project_with_its_new_file_path() {
    let dir = collab_bundle_scratch_dir("import_ok");
    let bundle_path = dir.join("handoff.zip");
    let mut sender = test_app(vec![test_project(1, Vec::new())], Vec::new());
    sender.export_collab_bundle(bundle_path.clone());

    let mut recipient = test_app(Vec::new(), Vec::new());
    let dest_project_path = dir.join("recipient/project.ocproj");

    recipient.import_collab_bundle(bundle_path, dest_project_path.clone());

    assert_eq!(recipient.projects.len(), 1);
    assert_eq!(
        recipient.projects[0].file_path,
        Some(dest_project_path.clone())
    );
    assert_eq!(
        recipient.screen,
        Screen::Editor,
        "opens the imported project"
    );
    assert!(dest_project_path.exists());
}

#[test]
fn import_collab_bundle_toasts_on_failure_instead_of_panicking() {
    let mut app = test_app(Vec::new(), Vec::new());
    let missing_zip = PathBuf::from("/nonexistent-oca-test-dir/handoff.zip");
    let dest_project_path =
        std::env::temp_dir().join("oca_app_collab_bundle_test_never_written.ocproj");

    app.import_collab_bundle(missing_zip, dest_project_path);

    assert!(app.open_projects.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

fn otio_fixture(target_url: &str, source_secs: f64, duration_secs: f64, rate: f64) -> String {
    serde_json::json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": "Imported Sequence",
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "children": [{
                "OTIO_SCHEMA": "Track.1",
                "kind": "Video",
                "name": "V1",
                "children": [{
                    "OTIO_SCHEMA": "Clip.1",
                    "name": "Clip-1",
                    "source_range": {
                        "OTIO_SCHEMA": "TimeRange.1",
                        "start_time": { "OTIO_SCHEMA": "RationalTime.1", "value": source_secs * rate, "rate": rate },
                        "duration": { "OTIO_SCHEMA": "RationalTime.1", "value": duration_secs * rate, "rate": rate },
                    },
                    "media_reference": {
                        "OTIO_SCHEMA": "ExternalReference.1",
                        "target_url": target_url,
                    },
                }],
            }],
            "markers": [],
        },
    })
    .to_string()
}

#[test]
fn import_otio_into_new_sequence_creates_a_new_sequence_with_the_imported_content() {
    let dir = tempfile::tempdir().unwrap();
    let otio_path = dir.path().join("imported.otio");
    std::fs::write(&otio_path, otio_fixture("clip.mp4", 2.0, 5.0, 24.0)).unwrap();

    let mut asset = test_asset(7);
    asset.source_path = PathBuf::from("clip.mp4");
    let mut app = test_app(vec![test_project(1, vec![asset])], Vec::new());
    assert_eq!(app.active_project().sequences.len(), 1);

    app.import_otio_into_new_sequence(otio_path, None);

    let project = app.active_project();
    assert_eq!(project.sequences.len(), 2, "appended, not replaced");
    assert_eq!(project.active_sequence, 1, "switched to the new sequence");
    assert_eq!(project.active_sequence().name, "Imported Sequence");
    let timeline = &project.active_sequence().timeline;
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].kind, TrackKind::Video);
    assert_eq!(timeline.tracks[0].clips.len(), 1);
    let clip = &timeline.tracks[0].clips[0];
    assert_eq!(clip.asset_id, 7);
    assert!((clip.source_in_secs - 2.0).abs() < 1e-6);
    assert!((clip.source_out_secs - 7.0).abs() < 1e-6);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_otio_into_new_sequence_toasts_on_malformed_json_without_adding_a_sequence() {
    let dir = tempfile::tempdir().unwrap();
    let otio_path = dir.path().join("garbage.otio");
    std::fs::write(&otio_path, "not json at all").unwrap();
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.import_otio_into_new_sequence(otio_path, None);

    assert_eq!(app.active_project().sequences.len(), 1);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_otio_into_new_sequence_counts_a_warning_for_an_unresolvable_media_reference() {
    let dir = tempfile::tempdir().unwrap();
    let otio_path = dir.path().join("offline.otio");
    // No asset in the project has this target_url -- the clip should be skipped and counted
    // as a warning, not silently linked to the wrong media.
    std::fs::write(&otio_path, otio_fixture("nonexistent.mp4", 0.0, 3.0, 24.0)).unwrap();
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.import_otio_into_new_sequence(otio_path, None);

    let project = app.active_project();
    assert_eq!(project.sequences.len(), 2);
    assert!(
        project.active_sequence().timeline.tracks[0]
            .clips
            .is_empty(),
        "the unresolvable clip must be skipped, not placed with a wrong asset"
    );
    assert_eq!(app.toasts.len(), 1);
    assert!(
        app.toasts[0].0.contains('1'),
        "toast should mention the one warning: {}",
        app.toasts[0].0
    );
}

#[test]
fn apply_detected_scene_cuts_adds_numbered_chapter_markers_at_timeline_coordinates() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 100.0, 5.0, 15.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.locale = Locale::En;

    app.apply_detected_scene_cuts(
        1,
        vec![
            avcore::SceneCut {
                at_secs: 8.0,
                score: 0.5,
            },
            avcore::SceneCut {
                at_secs: 12.0,
                score: 0.6,
            },
        ],
    );

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 2);
    assert_eq!(markers[0].position_secs, 103.0);
    assert_eq!(markers[0].label, "Chapter 1");
    assert_eq!(markers[0].kind, avcore::MarkerKind::Chapter);
    assert_eq!(markers[1].position_secs, 107.0);
    assert_eq!(markers[1].label, "Chapter 2");
}

#[test]
fn apply_detected_scene_cuts_numbering_continues_from_existing_chapters() {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.locale = Locale::En;
    app.active_project_mut()
        .timeline_mut()
        .add_marker(1.0, avcore::MarkerKind::Chapter);

    app.apply_detected_scene_cuts(
        1,
        vec![avcore::SceneCut {
            at_secs: 5.0,
            score: 0.5,
        }],
    );

    let markers = app.active_project().timeline().markers_sorted();
    let new_marker = markers.iter().find(|m| m.position_secs == 5.0).unwrap();
    assert_eq!(new_marker.label, "Chapter 2");
}

#[test]
fn apply_detected_scene_cuts_is_a_no_op_for_an_unknown_clip() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.apply_detected_scene_cuts(
        404,
        vec![avcore::SceneCut {
            at_secs: 1.0,
            score: 0.5,
        }],
    );

    assert!(app.active_project().timeline().markers.is_empty());
}

#[test]
fn export_chapters_txt_writes_sorted_timecode_lines() {
    let dir = std::env::temp_dir().join("oca_app_export_chapters_test");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let output_path = dir.join("chapters.txt");

    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let timeline = app.active_project_mut().timeline_mut();
    let later_id = timeline.add_marker(65.0, avcore::MarkerKind::Chapter);
    timeline.marker_mut(later_id).unwrap().label = "Boss fight".to_string();
    let earlier_id = timeline.add_marker(0.0, avcore::MarkerKind::Chapter);
    timeline.marker_mut(earlier_id).unwrap().label = "Intro".to_string();
    // A non-Chapter marker should never show up in the export.
    timeline.add_marker(30.0, avcore::MarkerKind::Standard);

    app.export_chapters_txt(output_path.clone());

    let contents = std::fs::read_to_string(&output_path).unwrap();
    assert_eq!(contents, "00:00 Intro\n01:05 Boss fight\n");
}

#[test]
fn export_chapters_txt_toasts_instead_of_writing_when_no_chapters_exist() {
    let dir = std::env::temp_dir().join("oca_app_export_chapters_test_empty");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let output_path = dir.join("chapters.txt");

    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.export_chapters_txt(output_path.clone());

    assert!(!output_path.exists());
    assert_eq!(app.toasts.len(), 1);
}
