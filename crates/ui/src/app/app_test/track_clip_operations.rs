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

//! Track role, clip label, audio detach, and speed-ramp tests.

use super::support::*;
use super::*;

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
