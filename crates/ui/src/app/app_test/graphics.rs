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

//! Layer template, text overlay, and shape overlay tests.

use super::support::*;
use super::*;

#[test]
fn begin_save_layer_template_is_a_no_op_when_nothing_is_multi_selected() {
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

    app.begin_save_layer_template();

    assert!(app.saving_layer_template.is_none());
}

#[test]
fn begin_save_layer_template_snapshots_the_multi_selection_ordered_by_track_then_start() {
    let mut c1 = test_clip(1, 5.0, 0.0, 10.0);
    c1.gain_db = 3.0;
    let mut c2 = test_clip(2, 0.0, 0.0, 10.0);
    c2.gain_db = -3.0;
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![c1, c2]),
                test_track(2, TrackKind::Audio, vec![test_clip(3, 0.0, 0.0, 5.0)]),
            ],
        )],
        Vec::new(),
    );
    app.multi_selected_clip_ids.insert(1);
    app.multi_selected_clip_ids.insert(2);

    app.begin_save_layer_template();

    let (layers, name) = app.saving_layer_template.expect("should be staged");
    assert_eq!(name, "");
    assert_eq!(layers.len(), 2);
    // clip 2 (start_secs 0.0) sorts before clip 1 (start_secs 5.0) on the same track.
    assert_eq!(layers[0].0, TrackKind::Video);
    assert_eq!(layers[0].1.gain_db, -3.0);
    assert_eq!(layers[1].1.gain_db, 3.0);
}

#[test]
fn commit_save_layer_template_is_a_no_op_when_the_name_is_blank() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.saving_layer_template = Some((
        vec![(TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting())],
        "   ".to_string(),
    ));

    app.commit_save_layer_template();

    assert!(app.saving_layer_template.is_some());
    assert!(app.prefs.saved_layer_templates.is_empty());
}

#[test]
fn commit_save_layer_template_pushes_a_named_template_and_clears_the_pending_state() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    let layers = vec![(TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting())];
    app.saving_layer_template = Some((layers, "Webcam corner".to_string()));

    app.commit_save_layer_template();

    assert!(app.saving_layer_template.is_none());
    assert_eq!(app.prefs.saved_layer_templates.len(), 1);
    assert_eq!(app.prefs.saved_layer_templates[0].name, "Webcam corner");
}

#[test]
fn delete_layer_template_removes_the_entry_at_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.saved_layer_templates = vec![
        avcore::LayerTemplate {
            name: "A".to_string(),
            layers: vec![],
        },
        avcore::LayerTemplate {
            name: "B".to_string(),
            layers: vec![],
        },
    ];

    app.delete_layer_template(0);

    assert_eq!(app.prefs.saved_layer_templates.len(), 1);
    assert_eq!(app.prefs.saved_layer_templates[0].name, "B");
}

#[test]
fn delete_layer_template_is_a_no_op_for_an_out_of_range_index() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "A".to_string(),
        layers: vec![],
    }];

    app.delete_layer_template(5);

    assert_eq!(app.prefs.saved_layer_templates.len(), 1);
}

#[test]
fn begin_apply_layer_template_stages_one_empty_slot_per_layer() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Group".to_string(),
        layers: vec![
            (TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting()),
            (TrackKind::Audio, test_clip(2, 0.0, 0.0, 1.0).formatting()),
        ],
    }];
    app.layer_templates_menu_open = true;

    app.begin_apply_layer_template(0);

    let (index, slots) = app.applying_layer_template.expect("should be staged");
    assert_eq!(index, 0);
    assert_eq!(slots, vec![None, None]);
    assert!(!app.layer_templates_menu_open);
}

#[test]
fn confirm_apply_layer_template_creates_one_clip_per_filled_layer_with_its_formatting() {
    let mut styled = test_clip(99, 0.0, 0.0, 1.0);
    styled.gain_db = 6.0;
    styled.flipped_h = true;
    let mut project = test_project(1, vec![test_asset(1), test_asset(2)]);
    project.timeline_mut().tracks = vec![];
    project.timeline_mut().playhead_secs = 2.5;
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Group".to_string(),
        layers: vec![(TrackKind::Video, styled.formatting())],
    }];
    app.applying_layer_template = Some((0, vec![Some(1)]));

    app.confirm_apply_layer_template();

    assert!(app.applying_layer_template.is_none());
    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    let track = &timeline.tracks[0];
    assert_eq!(track.kind, TrackKind::Video);
    assert_eq!(track.clips.len(), 1);
    let clip = &track.clips[0];
    assert_eq!(clip.asset_id, 1);
    assert_eq!(clip.start_secs, 2.5);
    assert_eq!(clip.gain_db, 6.0);
    assert!(clip.flipped_h);
}

#[test]
fn confirm_apply_layer_template_puts_each_layer_on_its_own_new_track() {
    let mut project = test_project(1, vec![test_asset(1), test_asset(1)]);
    project.timeline_mut().tracks = vec![];
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Two video layers".to_string(),
        layers: vec![
            (TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting()),
            (TrackKind::Video, test_clip(2, 0.0, 0.0, 1.0).formatting()),
        ],
    }];
    app.applying_layer_template = Some((0, vec![Some(1), Some(1)]));

    app.confirm_apply_layer_template();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 2);
    assert_eq!(timeline.tracks[0].clips.len(), 1);
    assert_eq!(timeline.tracks[1].clips.len(), 1);
}

#[test]
fn confirm_apply_layer_template_skips_layers_left_without_an_asset() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![];
    let mut app = test_app(vec![project], Vec::new());
    app.prefs.saved_layer_templates = vec![avcore::LayerTemplate {
        name: "Group".to_string(),
        layers: vec![
            (TrackKind::Video, test_clip(1, 0.0, 0.0, 1.0).formatting()),
            (TrackKind::Video, test_clip(2, 0.0, 0.0, 1.0).formatting()),
        ],
    }];
    app.applying_layer_template = Some((0, vec![Some(1), None]));

    app.confirm_apply_layer_template();

    let timeline = app.active_project().timeline();
    assert_eq!(timeline.tracks.len(), 1);
    assert_eq!(timeline.tracks[0].clips.len(), 1);
}

#[test]
fn add_shape_track_appends_an_empty_shape_track() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_shape_track();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Shape);
    assert!(tracks[0].shape_clips.is_empty());
}

#[test]
fn add_text_clip_auto_creates_a_text_track_at_the_playhead_and_selects_it() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 12.5;
    app.selected_clip_id = Some(90);
    app.selected_shape_clip_id = Some(91);

    app.add_text_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Text);
    assert_eq!(tracks[0].text_clips.len(), 1);
    let clip = &tracks[0].text_clips[0];
    assert_eq!(clip.start_secs, 12.5);
    assert_eq!(clip.duration_secs, 3.0);
    assert_eq!(clip.text, "Texto");
    assert_eq!(app.selected_text_clip_id, Some(clip.id));
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_shape_clip_id, None);
}

#[test]
fn add_text_clip_reuses_the_existing_text_track() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_text_clip();
    app.add_text_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].text_clips.len(), 2);
    assert_ne!(tracks[0].text_clips[0].id, tracks[0].text_clips[1].id);
}

#[test]
fn text_color_edit_commits_each_supported_target_only_on_confirmation() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.selected_text_clip_id.unwrap();

    app.begin_text_color_edit(clip_id, TextColorTarget::Foreground, [255, 255, 255, 255]);
    app.text_color_edit.as_mut().unwrap().rgba = [1, 2, 3, 4];
    assert_eq!(
        app.active_project().timeline().tracks[0].text_clips[0].color_rgba,
        [255, 255, 255, 255]
    );
    assert!(app.confirm_text_color_edit());

    for (target, rgba) in [
        (TextColorTarget::Background, [5, 6, 7, 8]),
        (TextColorTarget::Highlight, [9, 10, 11, 12]),
    ] {
        app.begin_text_color_edit(clip_id, target, [255, 255, 255, 255]);
        app.text_color_edit.as_mut().unwrap().rgba = rgba;
        assert!(app.confirm_text_color_edit());
    }

    let clip = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(clip.color_rgba, [1, 2, 3, 4]);
    assert_eq!(clip.background_rgba, [5, 6, 7, 8]);
    assert_eq!(clip.highlight_color_rgba, [9, 10, 11, 12]);
}

#[test]
fn confirming_a_text_color_edit_pushes_an_undo_step() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.selected_text_clip_id.unwrap();
    app.undo_stack.clear(); // discard the snapshot add_text_clip() itself pushed.

    app.begin_text_color_edit(clip_id, TextColorTarget::Foreground, [255, 255, 255, 255]);
    app.text_color_edit.as_mut().unwrap().rgba = [1, 2, 3, 4];
    assert!(app.confirm_text_color_edit());

    assert!(app.can_undo());
    app.undo();
    assert_eq!(
        app.active_project().timeline().tracks[0].text_clips[0].color_rgba,
        [255, 255, 255, 255]
    );
}

#[test]
fn switching_sequences_discards_an_open_text_color_edit() {
    let mut project = test_project(1, Vec::new());
    project.new_sequence("Sequence 2".to_string());
    project.active_sequence = 0;
    let mut app = test_app(vec![project], Vec::new());
    app.add_text_clip();
    let clip_id = app.selected_text_clip_id.unwrap();
    app.begin_text_color_edit(clip_id, TextColorTarget::Foreground, [255, 255, 255, 255]);

    app.select_sequence(1);

    assert_eq!(app.text_color_edit, None);
}

#[test]
fn add_shape_clip_auto_creates_a_shape_track_and_selects_the_new_clip() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_shape_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Shape);
    assert_eq!(tracks[0].shape_clips.len(), 1);
    let clip_id = tracks[0].shape_clips[0].id;
    assert_eq!(app.selected_shape_clip_id, Some(clip_id));
    assert_eq!(app.selected_clip_id, None);
    assert_eq!(app.selected_text_clip_id, None);
}

#[test]
fn add_shape_clip_ids_stay_unique_past_an_existing_high_shape_clip_id() {
    let mut track = test_track(1, TrackKind::Shape, Vec::new());
    track.shape_clips.push(avcore::timeline::ShapeClip {
        id: 100,
        start_secs: 0.0,
        duration_secs: 1.0,
        shape_kind: avcore::timeline::ShapeKind::rectangle(),
        center_x: 0.5,
        center_y: 0.5,
        center_x_keyframes: vec![],
        center_y_keyframes: vec![],
        width_keyframes: vec![],
        height_keyframes: vec![],
        rotation_keyframes: vec![],
        width: 0.3,
        height: 0.3,
        rotation_deg: 0.0,
        color_rgba: [255, 255, 255, 255],
        stroke_thickness_px: 0.0,
    });
    let mut app = test_app(vec![test_project_with_tracks(1, vec![track])], Vec::new());

    app.add_shape_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks[0].shape_clips.len(), 2);
    assert_eq!(tracks[0].shape_clips[1].id, 101);
}

#[test]
fn add_shape_clip_reuses_the_existing_shape_track_on_a_second_call() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.add_shape_clip();
    app.add_shape_clip();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].shape_clips.len(), 2);
    // Ids are unique even across the two calls.
    assert_ne!(tracks[0].shape_clips[0].id, tracks[0].shape_clips[1].id);
}

#[test]
fn start_drawing_custom_shape_begins_an_empty_point_list() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.start_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, Some(Vec::new()));
}

#[test]
fn cancel_drawing_custom_shape_discards_the_in_progress_drawing() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    app.push_drawing_shape_point(0.2, 0.3);

    app.cancel_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, None);
}

#[test]
fn push_drawing_shape_point_is_a_no_op_outside_drawing_mode() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.push_drawing_shape_point(0.2, 0.3);

    assert_eq!(app.drawing_shape_points, None);
}

#[test]
fn push_drawing_shape_point_clamps_to_the_canvas() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();

    app.push_drawing_shape_point(-0.5, 1.5);

    assert_eq!(app.drawing_shape_points, Some(vec![(0.0, 1.0)]));
}

#[test]
fn finish_drawing_custom_shape_is_a_no_op_below_three_points() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    app.push_drawing_shape_point(0.2, 0.2);
    app.push_drawing_shape_point(0.4, 0.2);

    app.finish_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, Some(vec![(0.2, 0.2), (0.4, 0.2)]));
    assert!(app.active_project().timeline().tracks.is_empty());
}

#[test]
fn finish_drawing_custom_shape_creates_a_polygon_clip_and_clears_drawing_mode() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    app.push_drawing_shape_point(0.2, 0.2);
    app.push_drawing_shape_point(0.6, 0.2);
    app.push_drawing_shape_point(0.4, 0.6);

    app.finish_drawing_custom_shape();

    assert_eq!(app.drawing_shape_points, None);
    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].kind, TrackKind::Shape);
    assert_eq!(tracks[0].shape_clips.len(), 1);
    let clip = &tracks[0].shape_clips[0];
    assert_eq!(app.selected_shape_clip_id, Some(clip.id));
    // Bounding box of the three points above: x in [0.2, 0.6], y in [0.2, 0.6].
    assert!((clip.center_x - 0.4).abs() < 1e-6);
    assert!((clip.center_y - 0.4).abs() < 1e-6);
    assert!((clip.width - 0.4).abs() < 1e-6);
    assert!((clip.height - 0.4).abs() < 1e-6);
    assert_eq!(clip.rotation_deg, 0.0);
    let avcore::timeline::ShapeKind::Polygon(vertices) = &clip.shape_kind else {
        panic!("expected a Polygon shape kind");
    };
    assert_eq!(vertices.len(), 3);
    // First point (0.2, 0.2) is the box's top-left corner -> local (-0.5, -0.5).
    assert!((vertices[0].0 - (-0.5)).abs() < 1e-6);
    assert!((vertices[0].1 - (-0.5)).abs() < 1e-6);
}

#[test]
fn finish_drawing_custom_shape_floors_a_degenerate_bounding_box() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.start_drawing_custom_shape();
    // Three collinear points on the same vertical line - a zero-width bounding box.
    app.push_drawing_shape_point(0.5, 0.2);
    app.push_drawing_shape_point(0.5, 0.4);
    app.push_drawing_shape_point(0.5, 0.6);

    app.finish_drawing_custom_shape();

    let clip = &app.active_project().timeline().tracks[0].shape_clips[0];
    assert!(clip.width > 0.0);
    let avcore::timeline::ShapeKind::Polygon(vertices) = &clip.shape_kind else {
        panic!("expected a Polygon shape kind");
    };
    assert!(vertices.iter().all(|v| v.0.is_finite() && v.1.is_finite()));
}
