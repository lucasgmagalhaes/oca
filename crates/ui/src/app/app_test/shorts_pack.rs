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

//! Highlight-driven short-form export tests.

use super::support::*;
use super::*;

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
