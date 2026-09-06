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

//! Motion-tracking region and background-removal tests.

use super::support::*;
use super::*;

#[test]
fn motion_track_region_defaults_to_a_centered_region() {
    let app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    assert_eq!(app.motion_track_region.motion_track_center_x, 0.5);
    assert_eq!(app.motion_track_region.motion_track_center_y, 0.5);
}

#[test]
fn start_picking_motion_track_region_is_a_no_op_without_a_loaded_preview() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.preview_state.preview_texture = None;

    app.start_picking_motion_track_region();

    assert!(!app.motion_track_region.picking_motion_track_region);
}

#[test]
fn start_picking_motion_track_region_activates_with_a_loaded_preview() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let ctx = egui::Context::default();
    let image = egui::ColorImage::new([1, 1], vec![egui::Color32::BLACK]);
    app.preview_state.preview_texture =
        Some(ctx.load_texture("test", image, egui::TextureOptions::default()));

    app.start_picking_motion_track_region();

    assert!(app.motion_track_region.picking_motion_track_region);
}

#[test]
fn stop_picking_motion_track_region_clears_the_flag() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.motion_track_region.picking_motion_track_region = true;

    app.stop_picking_motion_track_region();

    assert!(!app.motion_track_region.picking_motion_track_region);
}

#[test]
fn spawn_generate_matte_for_selected_clip_is_a_no_op_when_no_model_is_configured() {
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
    assert!(app.prefs.background_removal_model_path.trim().is_empty());

    app.spawn_generate_matte_for_selected_clip();

    // No background job started, and the user is told why.
    assert_eq!(app.matte_generation_state.matte_generating_clip_id, None);
    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn spawn_generate_matte_for_selected_clip_is_a_no_op_while_a_run_is_already_in_flight() {
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
    app.prefs.background_removal_model_path = "/models/modnet.onnx".to_string();
    app.selected_clip_id = Some(1);
    app.matte_generation_state.matte_generating_clip_id = Some(99);

    app.spawn_generate_matte_for_selected_clip();

    // Stays pinned to the already-running clip's id, not overwritten by this second call.
    assert_eq!(
        app.matte_generation_state.matte_generating_clip_id,
        Some(99)
    );
}

#[test]
fn set_selected_clip_background_removal_mask_path_updates_the_selected_clip() {
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

    app.set_selected_clip_background_removal_mask_path("/cache/clip_1_matte.mp4".to_string());

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].background_removal_mask_path,
        "/cache/clip_1_matte.mp4"
    );
}
