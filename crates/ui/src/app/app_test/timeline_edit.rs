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

//! Timeline edits, trim modes, selection, and composite-clip tests.

use super::support::*;
use super::*;

#[test]
fn split_at_playhead_splits_the_covering_clip_on_every_track_that_has_one() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 20.0)]),
                test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 20.0)]),
            ],
        )],
        Vec::new(),
    );
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;

    app.split_at_playhead();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks[0].clips.len(), 2);
    assert_eq!(tracks[1].clips.len(), 2);
    assert_eq!(tracks[0].clips[1].start_secs, 10.0);
    assert_eq!(tracks[1].clips[1].start_secs, 10.0);
    // Each track's new half got its own fresh id — no collision between them.
    assert_ne!(tracks[0].clips[1].id, tracks[1].clips[1].id);
}

#[test]
fn split_at_playhead_only_splits_tracks_the_playhead_actually_covers() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 20.0)]),
                test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 5.0)]),
            ],
        )],
        Vec::new(),
    );
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;

    app.split_at_playhead();

    let tracks = &app.active_project().timeline().tracks;
    assert_eq!(tracks[0].clips.len(), 2);
    assert_eq!(tracks[1].clips.len(), 1);
}

#[test]
fn split_at_playhead_is_a_no_op_when_nothing_covers_the_playhead() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.active_project_mut().timeline_mut().playhead_secs = 50.0;

    app.split_at_playhead();

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}

#[test]
fn a_continuous_effect_property_drag_pushes_only_one_undo_step() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    assert!(!app.can_undo());

    // Simulates egui re-firing the slider's setter every frame while the pointer stays down
    // (see App::push_undo_snapshot_for_drag's doc comment) — three "frames" of the same drag.
    app.set_selected_clip_gain(1.0);
    app.set_selected_clip_gain(2.0);
    app.set_selected_clip_gain(3.0);
    assert!(app.can_undo());
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        3.0
    );

    app.undo();
    // One undo step undoes the whole drag, back to the value before it started, not just the
    // last frame's increment.
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        0.0
    );
    assert!(!app.can_undo());
}

#[test]
fn releasing_the_pointer_starts_a_fresh_undo_step_for_the_next_drag() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.set_selected_clip_gain(1.0);
    app.end_undo_drag_tracking_if_pointer_released(false);
    app.set_selected_clip_gain(2.0);

    assert!(app.can_undo());
    app.undo();
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        1.0
    );
    assert!(app.can_undo());
    app.undo();
    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].gain_db,
        0.0
    );
    assert!(!app.can_undo());
}

#[test]
fn undo_after_split_at_playhead_restores_the_unsplit_clip() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;
    assert!(!app.can_undo());

    app.split_at_playhead();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 2);
    assert!(app.can_undo());
    assert!(!app.can_redo());

    app.undo();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
    assert!(!app.can_undo());
    assert!(app.can_redo());

    app.redo();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 2);
    assert!(app.can_undo());
    assert!(!app.can_redo());
}

#[test]
fn undo_is_a_no_op_with_empty_history() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.undo();
    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}

#[test]
fn selecting_a_sequence_clears_undo_history_from_the_previous_one() {
    let mut project = test_project_with_tracks(
        1,
        vec![test_track(
            1,
            TrackKind::Video,
            vec![test_clip(1, 0.0, 0.0, 20.0)],
        )],
    );
    project.new_sequence("Second".to_string());
    project.active_sequence = 0;
    let mut app = test_app(vec![project], Vec::new());
    app.active_project_mut().timeline_mut().playhead_secs = 10.0;

    app.split_at_playhead();
    assert!(app.can_undo());

    app.select_sequence(1);
    assert!(!app.can_undo());
}

#[test]
fn trim_clip_start_moves_the_left_edge_and_keeps_the_end_fixed() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 10.0, 5.0, 30.0)],
            )],
        )],
        Vec::new(),
    );

    app.trim_clip_start(1, 15.0);

    let clip = &app.active_project().timeline().tracks[0].clips[0];
    assert_eq!(clip.start_secs, 15.0);
    assert_eq!(clip.source_in_secs, 10.0);
    assert_eq!(clip.source_out_secs, 30.0);
}

#[test]
fn trim_clip_start_ignores_an_unknown_clip_id() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 10.0, 5.0, 30.0)],
            )],
        )],
        Vec::new(),
    );

    app.trim_clip_start(99, 15.0);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].start_secs,
        10.0
    );
}

#[test]
fn trim_clip_end_moves_the_right_edge_and_keeps_the_start_fixed() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.trim_clip_end(1, 5.0);

    let clip = &app.active_project().timeline().tracks[0].clips[0];
    assert_eq!(clip.start_secs, 0.0);
    assert_eq!(clip.source_out_secs, 5.0);
}

#[test]
fn trim_clip_end_is_bounded_by_the_source_assets_own_duration() {
    // test_asset's duration_secs is a fixed 10.0.
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.trim_clip_end(1, 50.0);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].source_out_secs,
        8.0
    );
}

#[test]
fn move_text_clip_repositions_it_on_the_timeline() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.active_project().timeline().tracks[0].text_clips[0].id;

    app.move_text_clip(clip_id, 12.5);

    let tc = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(tc.start_secs, 12.5);
}

#[test]
fn move_text_clip_clamps_a_negative_start_to_zero() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.active_project().timeline().tracks[0].text_clips[0].id;

    app.move_text_clip(clip_id, -5.0);

    let tc = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(tc.start_secs, 0.0);
}

#[test]
fn trim_text_clip_start_moves_the_left_edge_and_keeps_the_end_fixed() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.active_project().timeline().tracks[0].text_clips[0].id;
    // add_text_clip's own default: starts at the playhead (0.0), lasts 3.0 seconds.
    let end_secs = 3.0;

    app.trim_text_clip_start(clip_id, 1.0);

    let tc = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(tc.start_secs, 1.0);
    assert_eq!(tc.start_secs + tc.duration_secs, end_secs);
}

#[test]
fn trim_text_clip_start_never_shrinks_past_the_minimum_duration() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.active_project().timeline().tracks[0].text_clips[0].id;

    // Dragging the left edge all the way to (and past) the right edge should stop at the
    // minimum trim duration instead of collapsing/inverting the clip.
    app.trim_text_clip_start(clip_id, 100.0);

    let tc = &app.active_project().timeline().tracks[0].text_clips[0];
    assert!(tc.duration_secs > 0.0);
    assert!(tc.start_secs < 3.0);
}

#[test]
fn trim_text_clip_end_moves_the_right_edge_and_keeps_the_start_fixed() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.active_project().timeline().tracks[0].text_clips[0].id;

    app.trim_text_clip_end(clip_id, 10.0);

    let tc = &app.active_project().timeline().tracks[0].text_clips[0];
    assert_eq!(tc.start_secs, 0.0);
    assert_eq!(tc.duration_secs, 10.0);
}

#[test]
fn trim_text_clip_end_never_shrinks_past_the_minimum_duration() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_text_clip();
    let clip_id = app.active_project().timeline().tracks[0].text_clips[0].id;

    app.trim_text_clip_end(clip_id, -100.0);

    let tc = &app.active_project().timeline().tracks[0].text_clips[0];
    assert!(tc.duration_secs > 0.0);
}

#[test]
fn move_shape_clip_repositions_it_on_the_timeline() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_shape_clip();
    let clip_id = app.active_project().timeline().tracks[0].shape_clips[0].id;

    app.move_shape_clip(clip_id, 7.0);

    let sc = &app.active_project().timeline().tracks[0].shape_clips[0];
    assert_eq!(sc.start_secs, 7.0);
}

#[test]
fn trim_shape_clip_start_moves_the_left_edge_and_keeps_the_end_fixed() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_shape_clip();
    let clip_id = app.active_project().timeline().tracks[0].shape_clips[0].id;
    let sc_before = app.active_project().timeline().tracks[0].shape_clips[0].clone();
    let end_secs = sc_before.start_secs + sc_before.duration_secs;

    app.trim_shape_clip_start(clip_id, sc_before.start_secs + 0.5);

    let sc = &app.active_project().timeline().tracks[0].shape_clips[0];
    assert_eq!(sc.start_secs, sc_before.start_secs + 0.5);
    assert_eq!(sc.start_secs + sc.duration_secs, end_secs);
}

#[test]
fn trim_shape_clip_end_moves_the_right_edge_and_keeps_the_start_fixed() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());
    app.add_shape_clip();
    let clip_id = app.active_project().timeline().tracks[0].shape_clips[0].id;
    let start_secs = app.active_project().timeline().tracks[0].shape_clips[0].start_secs;

    app.trim_shape_clip_end(clip_id, start_secs + 20.0);

    let sc = &app.active_project().timeline().tracks[0].shape_clips[0];
    assert_eq!(sc.start_secs, start_secs);
    assert_eq!(sc.duration_secs, 20.0);
}

#[test]
fn ripple_trim_clip_start_shifts_only_later_clips() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 0.0, 5.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.ripple_trim_clip_start(2, 12.0);

    let tracks = &app.active_project().timeline().tracks[0];
    assert_eq!(tracks.clips[0].start_secs, 0.0);
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    assert_eq!(clip2.start_secs, 12.0);
    assert_eq!(clip2.source_in_secs, 2.0);
}

#[test]
fn ripple_trim_clip_end_is_bounded_by_the_source_assets_own_duration() {
    // test_asset's duration_secs is a fixed 10.0, so trimming clip 1's end past it must
    // refuse -- same bound App::trim_clip_end already enforces.
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 8.0), test_clip(2, 8.0, 0.0, 5.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.ripple_trim_clip_end(1, 50.0);

    let tracks = &app.active_project().timeline().tracks[0];
    assert_eq!(tracks.clips[0].source_out_secs, 8.0);
    assert_eq!(
        tracks.clips[1].start_secs, 8.0,
        "a refused trim must not shift the later clip either"
    );
}

#[test]
fn roll_edit_clip_moves_the_shared_boundary() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 2.0, 7.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.roll_edit_clip(1, 8.0);

    let tracks = &app.active_project().timeline().tracks[0];
    let clip1 = tracks.clips.iter().find(|c| c.id == 1).unwrap();
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    assert_eq!(clip1.start_secs + clip1.duration_secs(), 8.0);
    assert_eq!(clip2.start_secs, 8.0);
}

#[test]
fn roll_edit_from_start_edge_resolves_the_previous_neighbor() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 2.0, 7.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    // Dragging clip 2's own start edge should produce the same roll as dragging clip 1's end.
    app.roll_edit_from_start_edge(2, 8.0);

    let tracks = &app.active_project().timeline().tracks[0];
    let clip1 = tracks.clips.iter().find(|c| c.id == 1).unwrap();
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    assert_eq!(clip1.start_secs + clip1.duration_secs(), 8.0);
    assert_eq!(clip2.start_secs, 8.0);
}

#[test]
fn slip_clip_shifts_source_range_without_moving_on_the_timeline() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(1, 5.0, 1.0, 6.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.slip_clip(1, 1.0);

    let clip = &app.active_project().timeline().tracks[0].clips[0];
    assert_eq!(clip.start_secs, 5.0);
    assert_eq!(clip.source_in_secs, 2.0);
    assert_eq!(clip.source_out_secs, 7.0);
}

#[test]
fn slide_clip_absorbs_the_move_into_both_neighbors() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![
            test_clip(1, 0.0, 0.0, 6.0),
            test_clip(2, 6.0, 0.0, 9.0),
            test_clip(3, 15.0, 0.0, 5.0),
        ],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.slide_clip(2, 8.0);

    let tracks = &app.active_project().timeline().tracks[0];
    let clip1 = tracks.clips.iter().find(|c| c.id == 1).unwrap();
    let clip2 = tracks.clips.iter().find(|c| c.id == 2).unwrap();
    let clip3 = tracks.clips.iter().find(|c| c.id == 3).unwrap();
    assert_eq!(clip2.start_secs, 8.0);
    assert_eq!(clip1.start_secs + clip1.duration_secs(), 8.0);
    assert_eq!(clip3.start_secs, 17.0);
}

#[test]
fn move_clip_repositions_it_on_its_own_track() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 5.0, 15.0)],
            )],
        )],
        Vec::new(),
    );

    app.move_clip(1, 40.0);

    let clip = &app.active_project().timeline().tracks[0].clips[0];
    assert_eq!(clip.start_secs, 40.0);
    assert_eq!(clip.source_in_secs, 5.0);
    assert_eq!(clip.source_out_secs, 15.0);
}

#[test]
fn move_clip_ignores_a_negative_position() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 10.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );

    app.move_clip(1, -5.0);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].start_secs,
        10.0
    );
}

#[test]
fn toggle_multi_select_adds_then_removes() {
    let mut app = test_app(vec![test_project(1, Vec::new())], Vec::new());

    app.toggle_multi_select(1);
    assert!(app.multi_selected_clip_ids.contains(&1));

    app.toggle_multi_select(1);
    assert!(!app.multi_selected_clip_ids.contains(&1));
}

#[test]
fn merge_into_composite_is_a_no_op_with_fewer_than_two_selected() {
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
    app.toggle_multi_select(1);

    app.merge_into_composite();

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].composite_id,
        None
    );
    assert!(app.multi_selected_clip_ids.contains(&1)); // left untouched, not cleared.
}

#[test]
fn merge_into_composite_is_a_no_op_when_ids_span_different_tracks() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Video, vec![test_clip(2, 10.0, 0.0, 10.0)]),
            ],
        )],
        Vec::new(),
    );
    app.toggle_multi_select(1);
    app.toggle_multi_select(2);

    app.merge_into_composite();

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].composite_id,
        None
    );
    assert_eq!(
        app.active_project().timeline().tracks[1].clips[0].composite_id,
        None
    );
}

#[test]
fn merge_into_composite_assigns_a_shared_id_and_clears_the_multi_selection() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![
                    test_clip(1, 0.0, 0.0, 10.0),
                    test_clip(2, 10.0, 0.0, 10.0),
                    test_clip(3, 20.0, 0.0, 10.0),
                ],
            )],
        )],
        Vec::new(),
    );
    app.toggle_multi_select(1);
    app.toggle_multi_select(2);

    app.merge_into_composite();

    let clips = &app.active_project().timeline().tracks[0].clips;
    let group = clips[0].composite_id;
    assert!(group.is_some());
    assert_eq!(clips[1].composite_id, group);
    assert_eq!(clips[2].composite_id, None); // not selected, left standalone.
    assert!(app.multi_selected_clip_ids.is_empty());
}

#[test]
fn delete_selected_clip_removes_every_composite_member() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![
                    test_composite_clip(1, 0.0, 9),
                    test_composite_clip(2, 10.0, 9),
                    test_clip(3, 20.0, 0.0, 10.0),
                ],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.delete_selected_clip();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, 3);
}

#[test]
fn move_clip_with_group_moves_every_member_by_the_same_delta() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![
                    test_composite_clip(1, 0.0, 9),
                    test_composite_clip(2, 10.0, 9),
                    test_clip(3, 100.0, 0.0, 10.0),
                ],
            )],
        )],
        Vec::new(),
    );

    app.move_clip_with_group(1, 50.0); // +50s delta.

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].start_secs, 50.0);
    assert_eq!(clips[1].start_secs, 60.0);
    assert_eq!(clips[2].start_secs, 100.0); // standalone clip, untouched.
}

#[test]
fn move_clip_with_group_behaves_like_a_single_move_when_not_composite() {
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

    app.move_clip_with_group(1, 30.0);

    assert_eq!(
        app.active_project().timeline().tracks[0].clips[0].start_secs,
        30.0
    );
}

#[test]
fn move_clip_to_track_relocates_a_clip_to_a_same_kind_track() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Video, Vec::new()),
            ],
        )],
        Vec::new(),
    );

    app.move_clip_to_track(1, 2, 5.0);

    assert!(app.active_project().timeline().tracks[0].clips.is_empty());
    let moved = &app.active_project().timeline().tracks[1].clips[0];
    assert_eq!(moved.id, 1);
    assert_eq!(moved.start_secs, 5.0);
}

#[test]
fn move_clip_to_track_ignores_a_mismatched_kind() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Audio, Vec::new()),
            ],
        )],
        Vec::new(),
    );

    app.move_clip_to_track(1, 2, 5.0);

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
    assert!(app.active_project().timeline().tracks[1].clips.is_empty());
}

#[test]
fn delete_selected_clip_removes_it_from_its_track_and_clears_the_selection() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0), test_clip(2, 10.0, 0.0, 20.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);

    app.delete_selected_clip();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 1);
    assert_eq!(clips[0].id, 2);
    assert_eq!(app.selected_clip_id, None);
}
