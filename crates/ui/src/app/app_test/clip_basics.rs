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

//! Basic clip audio, speed, crop, and automatic reframing tests.

use super::support::*;
use super::*;

#[test]
fn set_selected_clip_gain_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_gain(6.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, 0.0);
    assert_eq!(clips[1].gain_db, 6.0);
}

#[test]
fn set_selected_clip_gain_clamps_to_gain_db_range() {
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

    app.set_selected_clip_gain(999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, *crate::app::GAIN_DB_RANGE.end());
}

#[test]
fn set_selected_clip_gain_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_gain(6.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, 0.0);
}

#[test]
fn set_selected_clip_deflicker_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_deflicker(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].deflicker_enabled);
    assert!(clips[1].deflicker_enabled);
}

#[test]
fn set_selected_clip_deflicker_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_deflicker(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].deflicker_enabled);
}

#[test]
fn set_selected_clip_frozen_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_frozen(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].frozen);
    assert!(clips[1].frozen);
}

#[test]
fn set_selected_clip_frozen_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_frozen(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].frozen);
}

#[test]
fn set_selected_clip_speed_updates_the_selected_clip() {
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
    app.selected_clip_id = Some(2);

    app.set_selected_clip_speed(2.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].speed_factor, 1.0);
    assert_eq!(clips[1].speed_factor, 2.0);
}

#[test]
fn preview_seek_parameters_keep_composited_branches_in_order() {
    let mut background = test_clip(1, 2.0, 1.0, 9.0);
    background.speed_factor = 0.5;
    let mut overlay = test_clip(2, 3.0, 0.25, 9.0);
    overlay.speed_factor = 2.0;
    let mut audio = test_clip(3, 1.0, 4.0, 9.0);
    audio.frozen = true;
    audio.speed_factor = 1.5;

    let (offsets, rates) = App::preview_seek_parameters([&background, &overlay, &audio], 5.0);

    assert_eq!(offsets, vec![2.5, 4.25, 4.0]);
    assert_eq!(rates, vec![0.5, 2.0, 1.5]);
}

#[test]
fn set_selected_clip_speed_clamps_to_speed_factor_range() {
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

    app.set_selected_clip_speed(999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].speed_factor, *crate::app::SPEED_FACTOR_RANGE.end());
}

#[test]
fn set_selected_clip_speed_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_speed(2.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].speed_factor, 1.0);
}

#[test]
fn set_selected_clip_crop_updates_the_selected_clip() {
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

    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (
            clips[0].crop_x,
            clips[0].crop_y,
            clips[0].crop_w,
            clips[0].crop_h
        ),
        (0.1, 0.2, 0.5, 0.6)
    );
}

#[test]
fn set_selected_clip_crop_clamps_each_field_independently() {
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

    app.set_selected_clip_crop(-1.0, 2.0, 0.0, 999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].crop_x, 0.0);
    assert_eq!(clips[0].crop_y, 1.0);
    assert_eq!(clips[0].crop_w, crate::app::CROP_MIN_SIZE);
    assert_eq!(clips[0].crop_h, 1.0);
}

#[test]
fn set_selected_clip_crop_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (
            clips[0].crop_x,
            clips[0].crop_y,
            clips[0].crop_w,
            clips[0].crop_h
        ),
        (0.0, 0.0, 1.0, 1.0)
    );
}

#[test]
fn set_selected_clip_reframe_seed_point_updates_the_selected_clip() {
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

    app.set_selected_clip_reframe_seed_point(Some((0.3, 0.7)));

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].reframe_seed_point, Some((0.3, 0.7)));
}

#[test]
fn set_selected_clip_reframe_seed_point_clamps_each_axis() {
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

    app.set_selected_clip_reframe_seed_point(Some((-1.0, 2.0)));

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].reframe_seed_point, Some((0.0, 1.0)));
}

#[test]
fn set_selected_clip_reframe_seed_point_clears_back_to_none() {
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
    app.set_selected_clip_reframe_seed_point(Some((0.3, 0.7)));

    app.set_selected_clip_reframe_seed_point(None);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].reframe_seed_point, None);
}

#[test]
fn set_selected_clip_reframe_seed_point_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_reframe_seed_point(Some((0.3, 0.7)));

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].reframe_seed_point, None);
}

#[test]
fn spawn_auto_reframe_selected_clip_skips_the_no_model_toast_when_a_seed_point_is_set() {
    let mut clip = test_clip(1, 0.0, 0.0, 10.0);
    clip.reframe_seed_point = Some((0.3, 0.7));
    let mut asset = test_asset(1);
    asset.resolution = Some((1920, 1080));
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![test_track(1, TrackKind::Video, vec![clip])],
            vec![asset],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.prefs.reframe_model_path.clear();

    app.spawn_auto_reframe_selected_clip();

    assert!(
        app.toasts.is_empty(),
        "a clip with a manual seed point must not require a configured model"
    );
}

#[test]
fn spawn_auto_reframe_selected_clip_still_requires_a_model_without_a_seed_point() {
    let mut asset = test_asset(1);
    asset.resolution = Some((1920, 1080));
    let mut app = test_app(
        vec![test_project_with_tracks_and_assets(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 10.0)],
            )],
            vec![asset],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.prefs.reframe_model_path.clear();

    app.spawn_auto_reframe_selected_clip();

    assert_eq!(app.toasts.len(), 1);
}

#[test]
fn spawn_motion_track_selected_clip_is_a_no_op_when_nothing_is_selected() {
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
    app.selected_clip_id = None;

    app.spawn_motion_track_selected_clip();

    // No background job started — the busy flag stays clear rather than getting stuck "in
    // progress" forever with nothing to ever complete it.
    assert_eq!(app.motion_tracking_state.motion_tracking_clip_id, None);
}

#[test]
fn spawn_motion_track_selected_clip_is_a_no_op_while_a_run_is_already_in_flight() {
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
    app.motion_tracking_state.motion_tracking_clip_id = Some(99);

    app.spawn_motion_track_selected_clip();

    // Stays pinned to the already-running clip's id, not overwritten by this second call.
    assert_eq!(app.motion_tracking_state.motion_tracking_clip_id, Some(99));
}
