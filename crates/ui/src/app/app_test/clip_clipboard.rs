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

//! Clip formatting and clipboard operation tests.

use super::support::*;
use super::*;

#[test]
fn copy_selected_clip_formatting_is_a_no_op_when_nothing_is_selected() {
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

    app.copy_selected_clip_formatting();

    assert!(!app.has_formatting_clipboard());
}

#[test]
fn paste_selected_clip_formatting_applies_gain_and_frozen_without_touching_position() {
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
    app.set_selected_clip_gain(6.0);
    app.set_selected_clip_frozen(true);
    app.set_selected_clip_speed(2.0);
    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);
    app.set_selected_clip_mask(avcore::timeline::MaskShape::Circle, 0.3);
    app.set_selected_clip_flip_h(true);
    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::Sepia);
    app.set_selected_clip_vignette(0.5);
    app.set_selected_clip_color_adjust(0.2, 1.5, 0.5);
    app.set_selected_clip_sharpen(0.6);
    app.set_selected_clip_chroma_key(true, [10, 200, 30], 0.7);
    app.set_selected_clip_blur(0.1);
    app.set_selected_clip_shake(0.2);
    app.set_selected_clip_glitch(0.3);
    app.set_selected_clip_pixelize(0.4);
    app.set_selected_clip_transition(avcore::timeline::TransitionType::Fade, 1.2);
    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.5,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 2.5,
        },
    ]);
    app.copy_selected_clip_formatting();

    app.selected_clip_id = Some(2);
    app.paste_selected_clip_formatting();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[1].gain_db, 6.0);
    assert!(clips[1].frozen);
    assert_eq!(clips[1].speed_factor, 2.0);
    assert_eq!(
        (
            clips[1].crop_x,
            clips[1].crop_y,
            clips[1].crop_w,
            clips[1].crop_h
        ),
        (0.1, 0.2, 0.5, 0.6)
    );
    assert_eq!(clips[1].mask_shape, avcore::timeline::MaskShape::Circle);
    assert_eq!(clips[1].mask_corner_radius, 0.3);
    assert!(clips[1].flipped_h);
    assert_eq!(clips[1].color_filter, avcore::timeline::ColorFilter::Sepia);
    assert_eq!(clips[1].vignette_intensity, 0.5);
    assert_eq!(
        (clips[1].brightness, clips[1].contrast, clips[1].saturation),
        (0.2, 1.5, 0.5)
    );
    assert_eq!(clips[1].sharpen, 0.6);
    assert!(clips[1].chroma_key_enabled);
    assert_eq!(clips[1].chroma_key_color, [10, 200, 30]);
    assert_eq!(clips[1].chroma_key_tolerance, 0.7);
    assert_eq!(
        (
            clips[1].blur_intensity,
            clips[1].shake_intensity,
            clips[1].glitch_intensity,
            clips[1].pixelize_intensity
        ),
        (0.1, 0.2, 0.3, 0.4)
    );
    assert_eq!(
        clips[1].transition_in,
        avcore::timeline::TransitionType::Fade
    );
    assert_eq!(clips[1].transition_duration_secs, 1.2);
    assert_eq!(
        clips[1].scale_keyframes,
        vec![
            avcore::Keyframe {
                time_fraction: 0.0,
                value: 1.5
            },
            avcore::Keyframe {
                time_fraction: 1.0,
                value: 2.5
            },
        ]
    );
    assert_eq!(clips[1].start_secs, 10.0); // position untouched
    assert_eq!(clips[1].source_out_secs, 20.0); // trim untouched
}

#[test]
fn paste_selected_clip_formatting_is_a_no_op_with_an_empty_clipboard() {
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

    app.paste_selected_clip_formatting();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].gain_db, 0.0);
    assert!(!clips[0].frozen);
}

#[test]
fn selected_clip_track_kind_reports_the_track_the_clip_lives_on() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                test_track(1, TrackKind::Video, vec![test_clip(1, 0.0, 0.0, 10.0)]),
                test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 10.0)]),
            ],
        )],
        Vec::new(),
    );

    app.selected_clip_id = Some(1);
    assert_eq!(app.selected_clip_track_kind(), Some(TrackKind::Video));

    app.selected_clip_id = Some(2);
    assert_eq!(app.selected_clip_track_kind(), Some(TrackKind::Audio));

    app.selected_clip_id = None;
    assert_eq!(app.selected_clip_track_kind(), None);
}

#[test]
fn selected_clip_track_audio_role_reports_the_track_the_clip_lives_on() {
    let mut mic_track = test_track(1, TrackKind::Audio, vec![test_clip(1, 0.0, 0.0, 10.0)]);
    mic_track.audio_role = AudioRole::Mic;
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![
                mic_track,
                test_track(2, TrackKind::Audio, vec![test_clip(2, 0.0, 0.0, 10.0)]),
            ],
        )],
        Vec::new(),
    );

    app.selected_clip_id = Some(1);
    assert_eq!(app.selected_clip_track_audio_role(), Some(AudioRole::Mic));

    app.selected_clip_id = Some(2);
    assert_eq!(
        app.selected_clip_track_audio_role(),
        Some(AudioRole::Unspecified)
    );

    app.selected_clip_id = None;
    assert_eq!(app.selected_clip_track_audio_role(), None);
}

#[test]
fn select_timeline_clip_synchronizes_its_backing_asset() {
    let mut project = test_project(1, vec![test_asset(1)]);
    project.timeline_mut().tracks = vec![test_track(
        1,
        TrackKind::Video,
        vec![test_clip(7, 0.0, 0.0, 10.0)],
    )];
    let mut app = test_app(vec![project], Vec::new());

    app.select_timeline_clip(7);

    assert_eq!(app.selected_clip_id, Some(7));
    assert_eq!(app.selected_asset_id, Some(1));
}

#[test]
fn copy_selected_clip_is_a_no_op_when_nothing_is_selected() {
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

    app.copy_selected_clip();

    assert!(!app.has_clipboard_clip());
}

#[test]
fn paste_clip_at_playhead_is_a_no_op_with_an_empty_clipboard() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(1, TrackKind::Video, vec![])],
        )],
        Vec::new(),
    );

    app.paste_clip_at_playhead();

    assert!(app.active_project().timeline().tracks[0].clips.is_empty());
}

#[test]
fn paste_clip_at_playhead_appends_a_fresh_clip_with_the_copied_trim_range() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(5, 0.0, 2.0, 12.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(5);
    app.set_selected_clip_gain(4.0);
    app.set_selected_clip_frozen(true);
    app.set_selected_clip_speed(2.0);
    app.set_selected_clip_crop(0.1, 0.2, 0.5, 0.6);
    app.set_selected_clip_mask(avcore::timeline::MaskShape::RoundedRect, 0.4);
    app.set_selected_clip_flip_h(true);
    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::BlackAndWhite);
    app.set_selected_clip_vignette(0.7);
    app.set_selected_clip_color_adjust(0.3, 1.2, 0.8);
    app.set_selected_clip_sharpen(0.9);
    app.set_selected_clip_chroma_key(true, [5, 180, 5], 0.55);
    app.set_selected_clip_blur(0.1);
    app.set_selected_clip_shake(0.2);
    app.set_selected_clip_glitch(0.3);
    app.set_selected_clip_pixelize(0.4);
    app.set_selected_clip_transition(avcore::timeline::TransitionType::Slide, 0.9);
    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.5,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 2.5,
        },
    ]);
    app.copy_selected_clip();
    app.active_project_mut().timeline_mut().playhead_secs = 30.0;

    app.paste_clip_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 2);
    let pasted = &clips[1];
    assert_ne!(pasted.id, 5); // fresh id, not a duplicate of the copied clip's.
    assert_eq!(pasted.start_secs, 30.0);
    assert_eq!(pasted.source_in_secs, 2.0);
    assert_eq!(pasted.source_out_secs, 12.0);
    assert_eq!(pasted.gain_db, 4.0); // gain travels with the clip through copy/paste.
    assert!(pasted.frozen); // so does frozen.
    assert_eq!(pasted.speed_factor, 2.0); // so does speed.
    assert_eq!(
        (pasted.crop_x, pasted.crop_y, pasted.crop_w, pasted.crop_h),
        (0.1, 0.2, 0.5, 0.6)
    ); // so does crop.
    assert_eq!(pasted.mask_shape, avcore::timeline::MaskShape::RoundedRect); // so does mask.
    assert_eq!(pasted.mask_corner_radius, 0.4);
    assert!(pasted.flipped_h); // so does flip.
    assert_eq!(
        pasted.color_filter,
        avcore::timeline::ColorFilter::BlackAndWhite
    ); // so does the color filter.
    assert_eq!(pasted.vignette_intensity, 0.7); // so does vignette.
    assert_eq!(
        (pasted.brightness, pasted.contrast, pasted.saturation),
        (0.3, 1.2, 0.8)
    ); // so does the color adjustment.
    assert_eq!(pasted.sharpen, 0.9); // so does sharpen.
    assert!(pasted.chroma_key_enabled); // so does chroma key.
    assert_eq!(pasted.chroma_key_color, [5, 180, 5]);
    assert_eq!(pasted.chroma_key_tolerance, 0.55);
    assert_eq!(
        (
            pasted.blur_intensity,
            pasted.shake_intensity,
            pasted.glitch_intensity,
            pasted.pixelize_intensity
        ),
        (0.1, 0.2, 0.3, 0.4)
    ); // so do blur/shake/glitch/pixelize.
    assert_eq!(
        pasted.transition_in,
        avcore::timeline::TransitionType::Slide
    ); // so does the transition.
    assert_eq!(pasted.transition_duration_secs, 0.9);
    assert_eq!(
        pasted.scale_keyframes,
        vec![
            avcore::Keyframe {
                time_fraction: 0.0,
                value: 1.5
            },
            avcore::Keyframe {
                time_fraction: 1.0,
                value: 2.5
            },
        ]
    ); // so do scale keyframes.
}

#[test]
fn paste_clip_at_playhead_survives_a_sequence_switch() {
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
    app.copy_selected_clip();

    app.add_sequence(); // switches to a fresh, empty tab.
    app.paste_clip_at_playhead();

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}

#[test]
fn cut_selected_clip_copies_then_removes_the_clip() {
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

    app.cut_selected_clip();

    assert!(app.active_project().timeline().tracks[0].clips.is_empty());
    assert!(app.has_clipboard_clip());
}

#[test]
fn copy_selected_clip_captures_every_member_of_a_composite_group() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 5.0), test_clip(2, 5.0, 0.0, 5.0)],
            )],
        )],
        Vec::new(),
    );
    app.multi_selected_clip_ids = HashSet::from([1, 2]);
    app.merge_into_composite();
    app.selected_clip_id = Some(1);

    app.copy_selected_clip();

    let (copied, _kind) = app.clipboard_clip.as_ref().unwrap();
    assert_eq!(copied.len(), 2);
    let mut copied_ids: Vec<u64> = copied.iter().map(|c| c.id).collect();
    copied_ids.sort();
    assert_eq!(copied_ids, vec![1, 2]);
}

#[test]
fn paste_clip_at_playhead_pastes_a_composite_group_as_one_block() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 5.0), test_clip(2, 5.0, 0.0, 5.0)],
            )],
        )],
        Vec::new(),
    );
    app.multi_selected_clip_ids = HashSet::from([1, 2]);
    app.merge_into_composite();
    app.selected_clip_id = Some(1);
    app.copy_selected_clip();
    app.active_project_mut().timeline_mut().playhead_secs = 100.0;

    app.paste_clip_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips.len(), 4);
    let pasted: Vec<_> = clips.iter().filter(|c| c.id != 1 && c.id != 2).collect();
    assert_eq!(pasted.len(), 2);
    // The two originals started at 0.0 and 5.0 (a 5s relative offset) - that offset survives
    // the paste, anchored at the new playhead instead of at 0.0.
    let mut pasted_starts: Vec<f64> = pasted.iter().map(|c| c.start_secs).collect();
    pasted_starts.sort_by(f64::total_cmp);
    assert_eq!(pasted_starts, vec![100.0, 105.0]);
    // Fresh ids, distinct from both the originals and each other.
    assert_ne!(pasted[0].id, pasted[1].id);
    assert!(pasted[0].id != 1 && pasted[0].id != 2);
    // Both pasted clips share one new composite_id, distinct from the original group's.
    let original_group = app.active_project().timeline().tracks[0]
        .clips
        .iter()
        .find(|c| c.id == 1)
        .unwrap()
        .composite_id;
    assert!(pasted[0].composite_id.is_some());
    assert_eq!(pasted[0].composite_id, pasted[1].composite_id);
    assert_ne!(pasted[0].composite_id, original_group);
}

#[test]
fn paste_clip_at_playhead_keeps_a_lone_copied_clip_standalone() {
    let mut app = test_app(
        vec![test_project_with_tracks(
            1,
            vec![test_track(
                1,
                TrackKind::Video,
                vec![test_clip(1, 0.0, 0.0, 5.0)],
            )],
        )],
        Vec::new(),
    );
    app.selected_clip_id = Some(1);
    app.copy_selected_clip();
    app.active_project_mut().timeline_mut().playhead_secs = 20.0;

    app.paste_clip_at_playhead();

    let pasted = &app.active_project().timeline().tracks[0].clips[1];
    assert_eq!(pasted.composite_id, None);
}

#[test]
fn delete_selected_clip_is_a_no_op_when_nothing_is_selected() {
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

    app.delete_selected_clip();

    assert_eq!(app.active_project().timeline().tracks[0].clips.len(), 1);
}
