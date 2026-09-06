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
use avcore::motion_template::{
    ColorBinding, GraphicTemplate, ParameterValue, TemplateElement, TemplateParameter,
    TemplateParameterKind, TemplateShapeElement, TemplateTextElement, TextBinding,
};
use avcore::timeline::{AudioRole, ClipInstance, ShapeKind, Track, TrackKind};
use avcore::{LoudnessMetrics, MediaAsset, MediaKind, Recency, Sequence, Timeline};
use eframe::egui;
use std::collections::HashMap;

mod application_state;
mod clip_basics;
mod clip_clipboard;
mod clip_visual_effects;
mod export_queue;
mod graphics;
mod media_import;
mod media_preview;
mod motion_background;
mod project_timeline;
mod support;
mod timeline_edit;
mod timeline_review;
mod youtube_download;

use support::*;

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

    assert!(app.projects.is_empty());
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

#[test]
fn set_track_audio_role_writes_the_role_on_the_targeted_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_audio_role(1, avcore::AudioRole::Mic);

    assert_eq!(
        app.active_project().timeline().tracks[0].audio_role,
        avcore::AudioRole::Mic
    );
}

#[test]
fn set_track_audio_role_is_a_no_op_for_an_unknown_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_audio_role(404, avcore::AudioRole::GameAudio);

    assert_eq!(
        app.active_project().timeline().tracks[0].audio_role,
        avcore::AudioRole::Unspecified
    );
}

#[test]
fn set_track_color_label_writes_the_label_on_the_targeted_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_color_label(1, Some([229, 83, 83]));

    assert_eq!(
        app.active_project().timeline().tracks[0].color_label,
        Some([229, 83, 83])
    );
}

#[test]
fn set_track_color_label_is_a_no_op_for_an_unknown_track() {
    let track = test_track(1, TrackKind::Audio, Vec::new());
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_track_color_label(404, Some([229, 83, 83]));

    assert_eq!(app.active_project().timeline().tracks[0].color_label, None);
}

#[test]
fn set_clip_color_label_writes_the_label_on_the_targeted_clip_regardless_of_selection() {
    let clip = test_clip(1, 0.0, 0.0, 10.0);
    let track = test_track(1, TrackKind::Video, vec![clip]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());
    app.selected_clip_id = None;

    app.set_clip_color_label(1, Some([86, 156, 214]));

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].color_label,
        Some([86, 156, 214])
    );
}

#[test]
fn detach_audio_mutes_the_video_clip_and_adds_a_synced_audio_clip() {
    let video_clip = test_clip(1, 5.0, 1.0, 4.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![video_track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.detach_audio_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks[0].clips[0].gain_db, *GAIN_DB_RANGE.start());
    let audio_track = timeline
        .tracks
        .iter()
        .find(|t| t.kind == TrackKind::Audio)
        .expect("an audio track should have been created");
    assert_eq!(audio_track.clips.len(), 1);
    let detached = &audio_track.clips[0];
    assert_eq!(detached.asset_id, 1);
    assert_eq!(detached.start_secs, 5.0);
    assert_eq!(detached.source_in_secs, 1.0);
    assert_eq!(detached.source_out_secs, 4.0);
    assert_eq!(detached.gain_db, 0.0);
}

#[test]
fn detach_audio_is_a_no_op_when_the_asset_has_no_audio() {
    let video_clip = test_clip(1, 5.0, 1.0, 4.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut silent_asset = test_asset(1);
    silent_asset.has_audio = false;
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![video_track],
            vec![silent_asset],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.detach_audio_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].clips[0].gain_db, 0.0);
}

#[test]
fn detach_audio_is_a_no_op_for_a_non_video_clip() {
    let audio_clip = test_clip(1, 5.0, 1.0, 4.0);
    let audio_track = test_track(1, TrackKind::Audio, vec![audio_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![audio_track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.detach_audio_from_selected_clip();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].clips[0].gain_db, 0.0);
}

#[test]
fn apply_speed_ramp_splits_into_contiguous_steps_with_interpolated_speed() {
    // A clip from 10s..20s (10s long at 1.0x, the default speed_factor test_clip already uses).
    let video_clip = test_clip(1, 10.0, 0.0, 10.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks(1, vec![video_track])],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.apply_speed_ramp_to_selected_clip(0.5, 2.0, 4);

    let mut clips = app.active_project().timeline().tracks[0].clips.clone();
    clips.sort_by(|a, b| a.start_secs.total_cmp(&b.start_secs));
    assert_eq!(clips.len(), 4);

    // Speeds interpolate linearly from 0.5 to 2.0 across the 4 pieces.
    let speeds: Vec<f32> = clips.iter().map(|c| c.speed_factor).collect();
    assert_eq!(speeds, vec![0.5, 1.0, 1.5, 2.0]);

    // The pieces stay contiguous (no gaps/overlaps) even though each one's duration_secs now
    // differs from the others, since speed_factor changed per piece.
    let mut cursor = 10.0;
    for clip in &clips {
        assert_eq!(clip.start_secs, cursor);
        cursor += clip.duration_secs();
    }

    // Splitting at equal ORIGINAL (unramped) 2.5s boundaries means each piece's own trimmed
    // source range is 2.5s wide, so its post-ramp duration is 2.5 / speed_factor.
    for (clip, speed) in clips.iter().zip(&speeds) {
        assert!((clip.duration_secs() - 2.5 / *speed as f64).abs() < 1e-9);
    }
}

#[test]
fn apply_speed_ramp_is_a_no_op_with_fewer_than_two_steps() {
    let video_clip = test_clip(1, 10.0, 0.0, 10.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks(1, vec![video_track])],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.apply_speed_ramp_to_selected_clip(0.5, 2.0, 1);

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}

#[test]
fn apply_speed_ramp_clamps_to_speed_factor_range() {
    let video_clip = test_clip(1, 10.0, 0.0, 10.0);
    let video_track = test_track(1, TrackKind::Video, vec![video_clip]);
    let mut app = test_app(
        vec![test_project_with_tracks(1, vec![video_track])],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    // 10.0x and 0.01x are both outside SPEED_FACTOR_RANGE (0.25..=4.0).
    app.apply_speed_ramp_to_selected_clip(10.0, 0.01, 2);

    let mut clips = app.active_project().timeline().tracks[0].clips.clone();
    clips.sort_by(|a, b| a.start_secs.total_cmp(&b.start_secs));
    assert_eq!(clips[0].speed_factor, *SPEED_FACTOR_RANGE.end());
    assert_eq!(clips[1].speed_factor, *SPEED_FACTOR_RANGE.start());
}

#[test]
fn set_clip_color_label_none_clears_an_existing_label() {
    let mut clip = test_clip(1, 0.0, 0.0, 10.0);
    clip.color_label = Some([86, 156, 214]);
    let track = test_track(1, TrackKind::Video, vec![clip]);
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.set_clip_color_label(1, None);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].color_label,
        None
    );
}

// 20 half-second buckets over a 10s asset -- matches DEFAULT_HIGHLIGHT_GRID_SECS (0.5s) exactly
// so each spiking bucket lands in its own grid cell instead of several buckets sharing one.
fn spiky_peaks(spike_range: std::ops::Range<usize>) -> Vec<(f32, f32)> {
    let mut peaks = vec![(-0.1, 0.1); 20];
    for p in &mut peaks[spike_range] {
        *p = (-0.9, 0.9);
    }
    peaks
}

fn highlight_test_project() -> Project {
    let game_asset = MediaAsset {
        waveform_peaks: Some(spiky_peaks(6..12)),
        favorited: false,
        duration_secs: 10.0,
        ..test_asset(1)
    };
    let mic_asset = MediaAsset {
        id: 2,
        waveform_peaks: Some(spiky_peaks(6..12)),
        favorited: false,
        duration_secs: 10.0,
        ..test_asset(2)
    };
    let mut game_track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    game_track.audio_role = avcore::AudioRole::GameAudio;

    let mic_clip = ClipInstance {
        asset_id: 2,
        ..test_clip(2, 0.0, 0.0, 10.0)
    };
    let mut mic_track = test_track(2, TrackKind::Audio, vec![mic_clip]);
    mic_track.audio_role = avcore::AudioRole::Mic;

    test_project_with_tracks_and_assets(1, vec![game_track, mic_track], vec![game_asset, mic_asset])
}

#[test]
fn detect_highlights_adds_a_marker_at_the_simultaneous_spike() {
    let mut app = test_app(vec![highlight_test_project()], Vec::new());

    app.detect_highlights();

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].kind, avcore::MarkerKind::Highlight);
    assert_eq!(markers[0].position_secs, 3.0);
}

#[test]
fn detect_highlights_toasts_when_a_role_is_missing() {
    // Only a game-audio track, no mic track tagged.
    let track = {
        let mut t = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
        t.audio_role = avcore::AudioRole::GameAudio;
        t
    };
    let asset = MediaAsset {
        waveform_peaks: Some(spiky_peaks(3..6)),
        favorited: false,
        duration_secs: 10.0,
        ..test_asset(1)
    };
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );

    app.detect_highlights();

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn detect_highlights_toasts_when_nothing_spikes_together() {
    let project = {
        let mut p = highlight_test_project();
        // Mic never spikes.
        p.media_library[1].waveform_peaks = Some(vec![(-0.1, 0.1); 20]);
        p
    };
    let mut app = test_app(vec![project], Vec::new());

    app.detect_highlights();

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

fn sidecar_json(source_media_filename: &str, event_source_timestamp_secs: f64) -> String {
    format!(
        r#"{{"schema_version":1,"source_media_filename":"{source_media_filename}","events":[{{"kind":"kill","source_timestamp_secs":{event_source_timestamp_secs},"confidence":1.0}}]}}"#
    )
}

fn write_sidecar(dir: &std::path::Path, name: &str, json: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, json).unwrap();
    path
}

#[test]
fn import_gameplay_events_adds_a_marker_for_a_matching_clip() {
    let asset = test_asset(1); // file_name: "asset-1.mp4"
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", &sidecar_json("asset-1.mp4", 4.0));

    app.import_gameplay_events(path);

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0].kind, avcore::MarkerKind::Highlight);
    assert_eq!(markers[0].position_secs, 4.0);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_is_idempotent() {
    let asset = test_asset(1);
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", &sidecar_json("asset-1.mp4", 4.0));

    app.import_gameplay_events(path.clone());
    app.import_gameplay_events(path);

    let markers = app.active_project().timeline().markers_sorted();
    assert_eq!(markers.len(), 1, "re-importing must not duplicate markers");
}

#[test]
fn import_gameplay_events_toasts_when_no_clip_uses_the_recording() {
    let asset = test_asset(1); // file_name: "asset-1.mp4"
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", &sidecar_json("other.mp4", 4.0));

    app.import_gameplay_events(path);

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_toasts_when_the_event_falls_outside_the_trimmed_clip_range() {
    let asset = test_asset(1);
    // Clip only covers source seconds [2.0, 5.0).
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 2.0, 5.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![asset],
        )],
        Vec::new(),
    );
    let dir = tempfile::tempdir().unwrap();
    // Event at source second 20 — well outside the clip's trimmed range.
    let path = write_sidecar(
        dir.path(),
        "events.json",
        &sidecar_json("asset-1.mp4", 20.0),
    );

    app.import_gameplay_events(path);

    assert!(app.active_project().timeline().markers.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_toasts_on_invalid_json() {
    let mut app = test_app(Vec::new(), Vec::new());
    let dir = tempfile::tempdir().unwrap();
    let path = write_sidecar(dir.path(), "events.json", "not valid json");

    app.import_gameplay_events(path);

    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn import_gameplay_events_toasts_when_the_file_cannot_be_read() {
    let mut app = test_app(Vec::new(), Vec::new());

    app.import_gameplay_events(PathBuf::from("/does/not/exist.json"));

    assert_eq!(app.toasts.len(), 1);
}

fn shorts_pack_scratch_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oca_app_shorts_pack_test_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn shorts_pack_test_project() -> Project {
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 60.0)]);
    let mut project = test_project_with_tracks_and_assets(1, vec![track], vec![test_asset(1)]);
    project
        .timeline_mut()
        .add_marker(10.0, avcore::MarkerKind::Highlight);
    project
        .timeline_mut()
        .add_marker(40.0, avcore::MarkerKind::Highlight);
    project
}

#[test]
fn spawn_shorts_pack_queues_one_job_per_highlight_marker() {
    let dir = shorts_pack_scratch_dir("basic");
    let mut app = test_app(vec![shorts_pack_test_project()], Vec::new());

    app.spawn_shorts_pack(dir.clone());

    assert_eq!(app.export_jobs.len(), 2);
    assert!(app.export_jobs[0].title.contains("short_1"));
    assert!(app.export_jobs[1].title.contains("short_2"));
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn spawn_shorts_pack_uses_portrait_aspect_ratio() {
    let dir = shorts_pack_scratch_dir("portrait");
    let mut app = test_app(vec![shorts_pack_test_project()], Vec::new());

    app.spawn_shorts_pack(dir);

    assert_eq!(app.export_jobs[0].canvas.width, 1080);
    assert_eq!(app.export_jobs[0].canvas.height, 1920);
}

#[test]
fn spawn_shorts_pack_toasts_when_there_are_no_highlight_markers() {
    let dir = shorts_pack_scratch_dir("none");
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 60.0)]);
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![track],
            vec![test_asset(1)],
        )],
        Vec::new(),
    );

    app.spawn_shorts_pack(dir);

    assert!(app.export_jobs.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn spawn_shorts_pack_skips_a_window_landing_entirely_in_a_gap() {
    let dir = shorts_pack_scratch_dir("gap");
    // A single short clip [0, 5); a highlight far past it has no clip content in its window.
    let track = test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 5.0)]);
    let mut project = test_project_with_tracks_and_assets(1, vec![track], vec![test_asset(1)]);
    project
        .timeline_mut()
        .add_marker(500.0, avcore::MarkerKind::Highlight);
    let mut app = test_app(vec![project], Vec::new());

    app.spawn_shorts_pack(dir);

    assert!(
        app.export_jobs.is_empty(),
        "the only candidate's window had no clip content"
    );
}

#[test]
fn spawn_shorts_pack_defers_export_and_starts_reframing_an_un_reframed_clip() {
    // CF-04's Shorts Pack integration slice: with a reframe model configured, an un-reframed
    // clip a highlight window touches gets queued for dynamic reframe *before* any export --
    // nothing should be queued yet, and the reframe queue/in-flight state should reflect it.
    let dir = shorts_pack_scratch_dir("reframe_pending");
    let mut app = test_app(vec![shorts_pack_test_project()], Vec::new());
    app.prefs.reframe_model_path = "model.onnx".to_string();

    app.spawn_shorts_pack(dir);

    assert!(
        app.export_jobs.is_empty(),
        "nothing should be queued until the reframe pre-pass finishes"
    );
    assert!(app.shorts_pack_reframe_state.is_some());
    assert_eq!(app.dynamic_reframe_state.dynamic_reframing_clip_id, Some(1));
}

#[test]
fn spawn_shorts_pack_skips_the_reframe_pre_pass_when_the_clip_already_has_crop_keyframes() {
    let dir = shorts_pack_scratch_dir("reframe_skip");
    let mut project = shorts_pack_test_project();
    project.timeline_mut().tracks[0].clips[0].crop_x_keyframes = vec![avcore::Keyframe {
        time_fraction: 0.0,
        value: 0.2,
    }];
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.reframe_model_path = "model.onnx".to_string();

    app.spawn_shorts_pack(dir);

    assert!(
        app.shorts_pack_reframe_state.is_none(),
        "the only clip already has crop keyframes -- nothing to reframe"
    );
    assert_eq!(app.export_jobs.len(), 2);
}

#[test]
fn spawn_shorts_pack_skips_the_reframe_pre_pass_when_no_model_is_configured() {
    // Matches the feature's pre-existing behavior: no model means every un-reframed clip just
    // exports centered, exactly as before this slice.
    let dir = shorts_pack_scratch_dir("reframe_no_model");
    let mut app = test_app(vec![shorts_pack_test_project()], Vec::new());
    assert!(app.prefs.reframe_model_path.trim().is_empty());

    app.spawn_shorts_pack(dir);

    assert!(app.shorts_pack_reframe_state.is_none());
    assert_eq!(app.export_jobs.len(), 2);
}

#[test]
fn start_watching_folder_is_a_no_op_with_no_path_set() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.start_watching_folder();

    assert!(!app.watch_folder_state.running);
}

#[test]
fn start_watching_folder_is_a_no_op_while_already_running() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.watch_folder_state.watch_path = Some(std::path::PathBuf::from("E:/records"));
    app.watch_folder_state.running = true;
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: std::path::PathBuf::from("E:/records/a.mp4"),
            status: crate::app::WatchFolderFileStatus::Processing,
            percent: 40,
            error: None,
            before: None,
            after: None,
            added_to_project: false,
        });

    app.start_watching_folder();

    // The already-running session's file list isn't cleared by a second, ignored call.
    assert_eq!(app.watch_folder_state.files.len(), 1);
}

#[test]
fn stop_watching_folder_is_a_no_op_when_nothing_is_running() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    // Just needs to not panic without a live watch session.
    app.stop_watching_folder();

    assert!(!app.watch_folder_state.running);
}

#[test]
fn stop_watching_folder_clears_the_running_flag_and_signals_the_stop_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    app.watch_folder_state.running = true;
    app.watch_folder_state.stop = Some(std::sync::Arc::clone(&stop));

    app.stop_watching_folder();

    assert!(!app.watch_folder_state.running);
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn pump_watch_folder_inserts_a_newly_detected_file_at_the_front() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let path = std::path::PathBuf::from("E:/records/newest.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: std::path::PathBuf::from("E:/records/older.mp4"),
            status: crate::app::WatchFolderFileStatus::Done,
            percent: 100,
            error: None,
            before: None,
            after: None,
            added_to_project: false,
        });
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Detected(path.clone()));

    app.pump_watch_folder();

    assert_eq!(app.watch_folder_state.files.len(), 2);
    assert_eq!(app.watch_folder_state.files[0].path, path);
    assert_eq!(
        app.watch_folder_state.files[0].status,
        crate::app::WatchFolderFileStatus::Stabilizing
    );
}

#[test]
fn pump_watch_folder_applies_progress_and_completion_to_the_matching_row() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let path = std::path::PathBuf::from("E:/records/a.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: path.clone(),
            status: crate::app::WatchFolderFileStatus::Processing,
            percent: 10,
            error: None,
            before: None,
            after: None,
            added_to_project: false,
        });
    let before = avcore::LoudnessMetrics {
        integrated_lufs: -22.0,
        true_peak_dbtp: -3.0,
        loudness_range_lu: 8.0,
    };
    let after = avcore::LoudnessMetrics {
        integrated_lufs: -16.0,
        true_peak_dbtp: -1.0,
        loudness_range_lu: 6.0,
    };
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Progress(path.clone(), 55));
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Done {
            path: path.clone(),
            before,
            after,
        });

    app.pump_watch_folder();

    let row = &app.watch_folder_state.files[0];
    assert_eq!(row.status, crate::app::WatchFolderFileStatus::Done);
    assert_eq!(row.percent, 100);
    assert_eq!(row.before.unwrap().integrated_lufs, -22.0);
    assert_eq!(row.after.unwrap().integrated_lufs, -16.0);
}

#[test]
fn pump_watch_folder_applies_a_failure_to_the_matching_row() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let path = std::path::PathBuf::from("E:/records/a.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: path.clone(),
            status: crate::app::WatchFolderFileStatus::Processing,
            percent: 10,
            error: None,
            before: None,
            after: None,
            added_to_project: false,
        });
    let _ = app
        .watch_folder_state
        .tx
        .send(crate::app::WatchFolderEvent::Failed {
            path: path.clone(),
            message: "ffmpeg exited with code 1".to_string(),
        });

    app.pump_watch_folder();

    let row = &app.watch_folder_state.files[0];
    assert_eq!(row.status, crate::app::WatchFolderFileStatus::Error);
    assert_eq!(row.error.as_deref(), Some("ffmpeg exited with code 1"));
}

#[test]
fn add_watched_file_to_project_queues_the_cleaned_up_output_for_import() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let watch_path = std::path::PathBuf::from("E:/records");
    app.watch_folder_state.watch_path = Some(watch_path.clone());
    let source = watch_path.join("a.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: source.clone(),
            status: crate::app::WatchFolderFileStatus::Done,
            percent: 100,
            error: None,
            before: None,
            after: None,
            added_to_project: false,
        });

    app.add_watched_file_to_project(source.clone());

    assert!(app.watch_folder_state.files[0].added_to_project);
    assert_eq!(app.import_state.pending_imports, 1);
}

#[test]
fn add_watched_file_to_project_toasts_without_an_open_project() {
    let mut app = test_app(Vec::new(), Vec::new());
    let watch_path = std::path::PathBuf::from("E:/records");
    app.watch_folder_state.watch_path = Some(watch_path.clone());
    let source = watch_path.join("a.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: source.clone(),
            status: crate::app::WatchFolderFileStatus::Done,
            percent: 100,
            error: None,
            before: None,
            after: None,
            added_to_project: false,
        });

    app.add_watched_file_to_project(source);

    assert!(!app.watch_folder_state.files[0].added_to_project);
    assert_eq!(app.import_state.pending_imports, 0);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn add_watched_file_to_project_is_a_no_op_once_already_queued() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let watch_path = std::path::PathBuf::from("E:/records");
    app.watch_folder_state.watch_path = Some(watch_path.clone());
    let source = watch_path.join("a.mp4");
    app.watch_folder_state
        .files
        .push(crate::app::WatchedFileRow {
            path: source.clone(),
            status: crate::app::WatchFolderFileStatus::Done,
            percent: 100,
            error: None,
            before: None,
            after: None,
            added_to_project: true,
        });

    app.add_watched_file_to_project(source);

    assert_eq!(
        app.import_state.pending_imports, 0,
        "a row already marked added_to_project must not be re-queued"
    );
}

/// Captures everything [`App::report_error`] hands to its reporter, for asserting on the
/// delivered reports in tests.
#[derive(Default)]
struct CapturingReporter {
    reports: std::sync::Mutex<Vec<avcore::ErrorReport>>,
}

impl avcore::ErrorReporter for CapturingReporter {
    fn report(&self, report: avcore::ErrorReport) {
        self.reports.lock().unwrap().push(report);
    }
}

#[test]
fn report_error_is_a_noop_without_a_reporter() {
    let app = test_app(Vec::new(), Vec::new());
    assert!(!app.report_error(
        avcore::ErrorCode::Import,
        avcore::ErrorSeverity::Error,
        avcore::Operation::Import,
        avcore::RecoveryOutcome::RequiresUserAction,
        false,
    ));
}

#[test]
fn report_error_delivers_a_valid_sanitized_report() {
    let reporter = std::sync::Arc::new(CapturingReporter::default());
    let mut app = test_app(Vec::new(), Vec::new());
    app.error_reporter =
        Some(std::sync::Arc::clone(&reporter) as std::sync::Arc<dyn avcore::ErrorReporter>);

    assert!(app.report_error(
        avcore::ErrorCode::ExportEncode,
        avcore::ErrorSeverity::Error,
        avcore::Operation::Export,
        avcore::RecoveryOutcome::Aborted,
        false,
    ));
    assert!(app.report_error(
        avcore::ErrorCode::Import,
        avcore::ErrorSeverity::Error,
        avcore::Operation::Import,
        avcore::RecoveryOutcome::RequiresUserAction,
        true,
    ));

    let reports = reporter.reports.lock().unwrap();
    assert_eq!(reports.len(), 2);
    let encoded = &reports[0];
    let imported = &reports[1];
    assert_eq!(encoded.error_code, avcore::ErrorCode::ExportEncode);
    assert_eq!(encoded.severity, avcore::ErrorSeverity::Error);
    assert_eq!(encoded.operation, avcore::Operation::Export);
    assert_eq!(encoded.recovery_outcome, avcore::RecoveryOutcome::Aborted);
    assert!(!encoded.retried);
    assert_eq!(imported.error_code, avcore::ErrorCode::Import);
    assert!(imported.retried);
    assert_eq!(
        encoded.session_id, imported.session_id,
        "session id is stable across the launch"
    );
    assert_ne!(
        encoded.event_id, imported.event_id,
        "event id is unique per report"
    );
    assert!(avcore::validate_report(encoded).is_ok());
    for report in reports.iter() {
        let stack = report
            .sanitized_stack_trace
            .as_deref()
            .expect("a stack trace is captured for every report");
        assert!(!stack.is_empty());
        assert!(
            !avcore::contains_forbidden_content(stack),
            "the captured backtrace must not survive sanitization with forbidden content"
        );
    }
}

#[test]
fn seed_error_reporting_seeds_launch_identity_and_breadcrumb() {
    let reporter = crate::app::error_reporting::seed_error_reporting(
        crate::i18n::Locale::En,
        crate::app::error_reporting::ErrorReportingConsent::Disabled,
    );
    assert!(reporter.is_none(), "Disabled consent must hold no reporter");

    let capturer = std::sync::Arc::new(CapturingReporter::default());
    let mut app = test_app(Vec::new(), Vec::new());
    let capturer_for_app = std::sync::Arc::clone(&capturer);
    app.error_reporter = Some(capturer_for_app as std::sync::Arc<dyn avcore::ErrorReporter>);
    assert!(app.report_error(
        avcore::ErrorCode::ProjectSave,
        avcore::ErrorSeverity::Error,
        avcore::Operation::ProjectSave,
        avcore::RecoveryOutcome::RequiresUserAction,
        false,
    ));

    let report = capturer.reports.lock().unwrap().pop().unwrap();
    assert_eq!(report.locale, "en");
    assert!(
        report.breadcrumbs.iter().any(
            |b| matches!(b, avcore::Breadcrumb::StateTransition { state } if state == "app_started")
        ),
        "seed_error_reporting must record the launch breadcrumb"
    );
    assert!(report.release.starts_with("oca-"));
}

fn sample_pending_crash(timestamp: u64) -> crate::app::crash_review::PendingCrashReview {
    crate::app::crash_review::PendingCrashReview {
        timestamp,
        app_version: "1.4.2".to_owned(),
        location: "crates/ui/src/app/mod.rs:1:1".to_owned(),
        message: "index out of bounds".to_owned(),
        backtrace: "0: oca::main".to_owned(),
    }
}

#[test]
fn crash_review_payload_preview_is_none_without_a_pending_crash() {
    let app = test_app(Vec::new(), Vec::new());
    assert!(app.crash_review_payload_preview().is_none());
}

#[test]
fn crash_review_payload_preview_shows_the_exact_post_sanitization_payload() {
    let mut app = test_app(Vec::new(), Vec::new());
    app.pending_crash_review = Some(sample_pending_crash(1_700_000_000));

    let preview = app
        .crash_review_payload_preview()
        .expect("a pending crash must produce a payload preview");
    let value: serde_json::Value = serde_json::from_str(&preview).unwrap();
    assert_eq!(value["tags"]["error_code"], "panic");
    assert_eq!(value["tags"]["operation"], "app");
    assert_eq!(value["release"], "oca-1.4.2");
    let stack = value["extra"]["stack_trace"].as_str().unwrap();
    assert!(stack.contains("index out of bounds"));
    assert!(stack.contains("crates/ui/src/app/mod.rs:1:1"));
}

#[test]
fn dismiss_pending_crash_review_clears_state_without_sending_anything() {
    let mut app = test_app(Vec::new(), Vec::new());
    let capturer = std::sync::Arc::new(CapturingReporter::default());
    app.error_reporter =
        Some(std::sync::Arc::clone(&capturer) as std::sync::Arc<dyn avcore::ErrorReporter>);
    app.pending_crash_review = Some(sample_pending_crash(1_700_000_042));

    app.dismiss_pending_crash_review();

    assert!(app.pending_crash_review.is_none());
    assert_eq!(app.prefs.last_reviewed_crash_unix, 1_700_000_042);
    assert!(
        capturer.reports.lock().unwrap().is_empty(),
        "Do not send must never report anything"
    );
    assert_eq!(
        app.prefs.error_reporting_consent,
        crate::app::error_reporting::ErrorReportingConsent::Disabled,
        "dismissing must not change the steady-state consent preference"
    );
}

#[test]
fn always_send_pending_crash_opts_in_and_reports_through_the_new_consent() {
    let mut app = test_app(Vec::new(), Vec::new());
    app.pending_crash_review = Some(sample_pending_crash(1_700_000_100));

    app.always_send_pending_crash();

    assert!(app.pending_crash_review.is_none());
    assert_eq!(app.prefs.last_reviewed_crash_unix, 1_700_000_100);
    assert_eq!(
        app.prefs.error_reporting_consent,
        crate::app::error_reporting::ErrorReportingConsent::AlwaysSend
    );
    assert!(
        app.error_reporter.is_some(),
        "opting in must leave a live reporter wired for future reports"
    );
}

#[test]
fn send_pending_crash_once_marks_reviewed_without_changing_steady_state_consent() {
    let mut app = test_app(Vec::new(), Vec::new());
    app.pending_crash_review = Some(sample_pending_crash(1_700_000_200));

    app.send_pending_crash_once();

    assert!(app.pending_crash_review.is_none());
    assert_eq!(app.prefs.last_reviewed_crash_unix, 1_700_000_200);
    assert_eq!(
        app.prefs.error_reporting_consent,
        crate::app::error_reporting::ErrorReportingConsent::Disabled,
        "Send once must stay a one-off action, never flipping the steady-state preference"
    );
}

#[test]
fn crash_review_actions_are_a_noop_without_a_pending_crash() {
    let mut app = test_app(Vec::new(), Vec::new());
    app.prefs.last_reviewed_crash_unix = 5;

    app.send_pending_crash_once();
    app.always_send_pending_crash();
    app.dismiss_pending_crash_review();

    assert!(app.pending_crash_review.is_none());
    assert_eq!(
        app.prefs.last_reviewed_crash_unix, 5,
        "with nothing pending, none of the three actions should touch the reviewed marker"
    );
}

fn text_template_element(id: &str, text: TextBinding) -> TemplateElement {
    TemplateElement::Text(TemplateTextElement {
        id: id.to_string(),
        text,
        color_rgba: ColorBinding::Fixed([255, 255, 255, 255]),
        font_family: Default::default(),
        font_style: Default::default(),
        font_size: 32.0,
        pos_x: 0.2,
        pos_y: 0.8,
        timing: Default::default(),
    })
}

fn shape_template_element(id: &str) -> TemplateElement {
    TemplateElement::Shape(TemplateShapeElement {
        id: id.to_string(),
        shape_kind: ShapeKind::rectangle(),
        color_rgba: ColorBinding::Fixed([0, 0, 0, 255]),
        center_x: 0.5,
        center_y: 0.5,
        width: 0.3,
        height: 0.1,
        rotation_deg: 0.0,
        stroke_thickness_px: 0.0,
    })
}

fn minimal_graphic_template(elements: Vec<TemplateElement>) -> GraphicTemplate {
    GraphicTemplate {
        schema_version: avcore::motion_template::TEMPLATE_SCHEMA_VERSION,
        name: "Test template".to_string(),
        canvas_width: 1920,
        canvas_height: 1080,
        safe_area_margin: 0.0,
        parameters: vec![TemplateParameter {
            id: "player_name".to_string(),
            label: "Player name".to_string(),
            kind: TemplateParameterKind::Text,
        }],
        elements,
    }
}

#[test]
fn apply_graphic_template_places_a_text_element_at_the_playhead() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 5.0;
    let template = minimal_graphic_template(vec![text_template_element(
        "e1",
        TextBinding::Fixed("PacoPaçoca".to_string()),
    )]);

    app.apply_graphic_template(&template, &HashMap::new());

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Text);
    assert_eq!(tracks[0].text_clips.len(), 1);
    let clip = &tracks[0].text_clips[0];
    assert_eq!(clip.start_secs, 5.0);
    assert_eq!(clip.text, "PacoPaçoca");
    assert_eq!(clip.pos_x, 0.2);
    assert_eq!(clip.pos_y, 0.8);
}

#[test]
fn apply_graphic_template_places_a_shape_element_on_its_own_track() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![shape_template_element("s1")]);

    app.apply_graphic_template(&template, &HashMap::new());

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Shape);
    assert_eq!(tracks[0].shape_clips.len(), 1);
}

#[test]
fn apply_graphic_template_places_text_and_shape_elements_on_separate_tracks() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![
        text_template_element("e1", TextBinding::Fixed("Hi".to_string())),
        shape_template_element("s1"),
    ]);

    app.apply_graphic_template(&template, &HashMap::new());

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0].kind, TrackKind::Text);
    assert_eq!(tracks[1].kind, TrackKind::Shape);
}

#[test]
fn apply_graphic_template_resolves_a_parameter_bound_text_value() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![text_template_element(
        "e1",
        TextBinding::Parameter("player_name".to_string()),
    )]);
    let mut values = HashMap::new();
    values.insert(
        "player_name".to_string(),
        ParameterValue::Text("Zé".to_string()),
    );

    app.apply_graphic_template(&template, &values);

    let clip = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(clip.text, "Zé");
}

#[test]
fn apply_graphic_template_toasts_and_makes_no_change_on_a_missing_parameter_value() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![text_template_element(
        "e1",
        TextBinding::Parameter("player_name".to_string()),
    )]);

    app.apply_graphic_template(&template, &HashMap::new());

    assert!(app.active_project().timeline().tracks.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn apply_graphic_template_pushes_exactly_one_undo_snapshot_for_the_whole_batch() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.undo_stack.clear();
    let template = minimal_graphic_template(vec![
        text_template_element("e1", TextBinding::Fixed("Hi".to_string())),
        shape_template_element("s1"),
    ]);

    app.apply_graphic_template(&template, &HashMap::new());
    assert!(app.undo_stack.can_undo());

    let sequence = app.active_project().sequences[app.active_project().active_sequence].clone();
    let restored = app
        .undo_stack
        .undo(sequence)
        .expect("one snapshot was pushed");
    assert!(
        restored.timeline.tracks.is_empty(),
        "undoing the apply should restore the pre-apply (empty) timeline"
    );
    assert!(
        !app.undo_stack.can_undo(),
        "exactly one snapshot should have been pushed for the whole batch"
    );
}

#[test]
fn load_graphic_template_from_file_applies_a_parameterless_template_immediately() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![text_template_element(
        "e1",
        TextBinding::Fixed("Hi".to_string()),
    )]);
    // No parameters at all this time -- overrides the fixture's default one.
    let mut template = template;
    template.parameters = vec![];
    let json = serde_json::to_string(&template).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("template.json");
    std::fs::write(&path, json).unwrap();

    app.load_graphic_template_from_file(path);

    assert!(app.pending_graphic_template_apply.is_none());
    assert_eq!(
        app.active_project().timeline().tracks[0].text_clips.len(),
        1
    );
}

#[test]
fn load_graphic_template_from_file_stages_a_parameterized_template_instead_of_applying() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![text_template_element(
        "e1",
        TextBinding::Parameter("player_name".to_string()),
    )]);
    let json = serde_json::to_string(&template).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("template.json");
    std::fs::write(&path, json).unwrap();

    app.load_graphic_template_from_file(path);

    assert!(app.active_project().timeline().tracks.is_empty());
    let pending = app
        .pending_graphic_template_apply
        .as_ref()
        .expect("a parameterized template should be staged, not applied");
    assert!(pending.text_values.contains_key("player_name"));
}

#[test]
fn load_graphic_template_from_file_toasts_on_invalid_json() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("template.json");
    std::fs::write(&path, "not json").unwrap();

    app.load_graphic_template_from_file(path);

    assert!(app.pending_graphic_template_apply.is_none());
    assert!(app.active_project().timeline().tracks.is_empty());
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn confirm_apply_graphic_template_applies_the_staged_template_with_filled_values() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![text_template_element(
        "e1",
        TextBinding::Parameter("player_name".to_string()),
    )]);
    let mut text_values = HashMap::new();
    text_values.insert("player_name".to_string(), "Zé".to_string());
    app.pending_graphic_template_apply = Some(PendingGraphicTemplateApply {
        template,
        text_values,
        color_values: HashMap::new(),
    });

    app.confirm_apply_graphic_template();

    assert!(app.pending_graphic_template_apply.is_none());
    let clip = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(clip.text, "Zé");
}

#[test]
fn cancel_apply_graphic_template_discards_the_staged_template_without_applying() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let template = minimal_graphic_template(vec![text_template_element(
        "e1",
        TextBinding::Parameter("player_name".to_string()),
    )]);
    app.pending_graphic_template_apply = Some(PendingGraphicTemplateApply {
        template,
        text_values: HashMap::new(),
        color_values: HashMap::new(),
    });

    app.cancel_apply_graphic_template();

    assert!(app.pending_graphic_template_apply.is_none());
    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn apply_effect_preset_blur_sets_the_default_intensity() {
    use crate::app::effects_panel::{EffectPreset, EFFECT_DEFAULT_INTENSITY};

    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.apply_effect_preset(EffectPreset::Blur);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].blur_intensity, EFFECT_DEFAULT_INTENSITY);
}

#[test]
fn apply_effect_preset_black_and_white_sets_the_color_filter() {
    use crate::app::effects_panel::EffectPreset;
    use avcore::timeline::ColorFilter;

    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.apply_effect_preset(EffectPreset::BlackAndWhite);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].color_filter, ColorFilter::BlackAndWhite);
}

#[test]
fn apply_effect_preset_chroma_key_enables_it_without_touching_existing_color_or_tolerance() {
    use crate::app::effects_panel::EffectPreset;

    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.set_selected_clip_chroma_key(false, [10, 20, 30], 0.42);

    app.apply_effect_preset(EffectPreset::ChromaKey);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].chroma_key_enabled);
    assert_eq!(clips[0].chroma_key_color, [10, 20, 30]);
    assert_eq!(clips[0].chroma_key_tolerance, 0.42);
}

#[test]
fn apply_effect_preset_is_a_no_op_when_nothing_is_selected() {
    use crate::app::effects_panel::EffectPreset;

    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
        )],
        Vec::new(),
    );

    app.apply_effect_preset(EffectPreset::Blur);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].blur_intensity, 0.0);
}

#[test]
fn materialize_nested_sequences_for_active_sequence_is_a_no_op_without_any_nested_clips() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
        )],
        Vec::new(),
    );

    let assets = app.materialize_nested_sequences_for_active_sequence();

    assert!(assets.is_empty());
    assert!(app
        .nested_sequence_render_state
        .nested_sequence_rendering_ids
        .is_empty());
}

#[test]
fn materialize_nested_sequences_for_active_sequence_returns_cached_result_when_input_unchanged() {
    let mut clip = test_clip(1, 0.0, 0.0, 10.0);
    clip.nested_sequence_id = Some(99);
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(1, TrackKind::Video, vec![clip])],
        )],
        Vec::new(),
    );
    let sequences_input = app.active_project().sequences.clone();
    app.nested_sequence_render_state
        .nested_sequence_last_input
        .insert(1, sequences_input);
    app.nested_sequence_render_state
        .nested_sequence_last_result
        .insert(1, vec![]);

    let assets = app.materialize_nested_sequences_for_active_sequence();

    assert!(assets.is_empty());
    // Unchanged input -- must not have dispatched a fresh background render.
    assert!(app
        .nested_sequence_render_state
        .nested_sequence_rendering_ids
        .is_empty());
}

#[test]
fn materialize_nested_sequences_for_active_sequence_dispatches_a_background_render_for_a_nested_clip(
) {
    let mut clip = test_clip(1, 0.0, 0.0, 10.0);
    clip.nested_sequence_id = Some(99);
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(1, TrackKind::Video, vec![clip])],
        )],
        Vec::new(),
    );

    let assets = app.materialize_nested_sequences_for_active_sequence();

    // No prior result cached -- returns empty immediately while the background render runs.
    assert!(assets.is_empty());
    assert!(app
        .nested_sequence_render_state
        .nested_sequence_rendering_ids
        .contains(&1));
}

#[test]
fn pump_nested_sequence_renders_applies_a_ready_event() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.nested_sequence_render_state
        .nested_sequence_rendering_ids
        .insert(1);
    let asset = test_asset(1000);
    app.nested_sequence_render_state
        .nested_sequence_tx
        .send(crate::app::NestedSequenceEvent::Ready {
            sequence_id: 1,
            cache: std::collections::HashMap::new(),
            assets: vec![asset.clone()],
            sequences_input: app.active_project().sequences.clone(),
        })
        .unwrap();

    app.pump_nested_sequence_renders();

    assert_eq!(
        app.nested_sequence_render_state
            .nested_sequence_last_result
            .get(&1),
        Some(&vec![asset])
    );
    assert!(app
        .nested_sequence_render_state
        .nested_sequence_last_input
        .contains_key(&1));
    assert!(!app
        .nested_sequence_render_state
        .nested_sequence_rendering_ids
        .contains(&1));
}

#[test]
fn pump_nested_sequence_renders_applies_a_failed_event_and_latches_the_input() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.nested_sequence_render_state
        .nested_sequence_rendering_ids
        .insert(1);
    let sequences_input = app.active_project().sequences.clone();
    app.nested_sequence_render_state
        .nested_sequence_tx
        .send(crate::app::NestedSequenceEvent::Failed {
            sequence_id: 1,
            error: "missing nested sequence".to_string(),
            sequences_input: sequences_input.clone(),
        })
        .unwrap();

    app.pump_nested_sequence_renders();

    assert!(!app
        .nested_sequence_render_state
        .nested_sequence_last_result
        .contains_key(&1));
    assert_eq!(
        app.nested_sequence_render_state
            .nested_sequence_last_input
            .get(&1),
        Some(&sequences_input)
    );
    assert!(!app
        .nested_sequence_render_state
        .nested_sequence_rendering_ids
        .contains(&1));
}
