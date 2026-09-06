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

//! Visual clip formatting, keyframes, transitions, and effects tests.

use super::support::*;
use super::*;

#[test]
fn set_selected_clip_mask_updates_the_selected_clip() {
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

    app.set_selected_clip_mask(avcore::timeline::MaskShape::Circle, 0.3);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].mask_shape, avcore::timeline::MaskShape::None);
    assert_eq!(clips[1].mask_shape, avcore::timeline::MaskShape::Circle);
    assert_eq!(clips[1].mask_corner_radius, 0.3);
}

#[test]
fn set_selected_clip_mask_clamps_corner_radius_to_range() {
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

    app.set_selected_clip_mask(avcore::timeline::MaskShape::RoundedRect, 999.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].mask_corner_radius,
        *crate::app::MASK_CORNER_RADIUS_RANGE.end()
    );
}

#[test]
fn set_selected_clip_mask_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_mask(avcore::timeline::MaskShape::Circle, 0.3);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].mask_shape, avcore::timeline::MaskShape::None);
}

#[test]
fn set_selected_clip_flip_h_updates_the_selected_clip() {
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

    app.set_selected_clip_flip_h(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].flipped_h);
    assert!(clips[1].flipped_h);
}

#[test]
fn set_selected_clip_flip_h_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_flip_h(true);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].flipped_h);
}

#[test]
fn set_selected_clip_color_filter_updates_the_selected_clip() {
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

    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::Sepia);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].color_filter, avcore::timeline::ColorFilter::None);
    assert_eq!(clips[1].color_filter, avcore::timeline::ColorFilter::Sepia);
}

#[test]
fn set_selected_clip_color_filter_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_color_filter(avcore::timeline::ColorFilter::Sepia);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].color_filter, avcore::timeline::ColorFilter::None);
}

#[test]
fn set_selected_clip_vignette_updates_the_selected_clip() {
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

    app.set_selected_clip_vignette(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].vignette_intensity, 0.0);
    assert_eq!(clips[1].vignette_intensity, 0.5);
}

#[test]
fn set_selected_clip_vignette_clamps_to_vignette_intensity_range() {
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

    app.set_selected_clip_vignette(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].vignette_intensity,
        *crate::app::VIGNETTE_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_vignette_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_vignette(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].vignette_intensity, 0.0);
}

#[test]
fn set_selected_clip_color_adjust_updates_the_selected_clip() {
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

    app.set_selected_clip_color_adjust(0.2, 1.5, 0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (clips[0].brightness, clips[0].contrast, clips[0].saturation),
        (0.0, 1.0, 1.0)
    );
    assert_eq!(
        (clips[1].brightness, clips[1].contrast, clips[1].saturation),
        (0.2, 1.5, 0.5)
    );
}

#[test]
fn set_selected_clip_color_adjust_clamps_each_field_independently() {
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

    app.set_selected_clip_color_adjust(-5.0, 5.0, -5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (clips[0].brightness, clips[0].contrast, clips[0].saturation),
        (
            *crate::app::BRIGHTNESS_RANGE.start(),
            *crate::app::CONTRAST_RANGE.end(),
            *crate::app::SATURATION_RANGE.start()
        )
    );
}

#[test]
fn set_selected_clip_color_adjust_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_color_adjust(0.2, 1.5, 0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        (clips[0].brightness, clips[0].contrast, clips[0].saturation),
        (0.0, 1.0, 1.0)
    );
}

#[test]
fn set_selected_clip_sharpen_updates_the_selected_clip() {
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

    app.set_selected_clip_sharpen(0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].sharpen, 0.0);
    assert_eq!(clips[1].sharpen, 0.6);
}

#[test]
fn set_selected_clip_sharpen_clamps_to_sharpen_range() {
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

    app.set_selected_clip_sharpen(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].sharpen, *crate::app::SHARPEN_RANGE.end());
}

#[test]
fn set_selected_clip_sharpen_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_sharpen(0.6);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].sharpen, 0.0);
}

#[test]
fn set_selected_clip_transition_updates_the_selected_clip() {
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

    app.set_selected_clip_transition(avcore::timeline::TransitionType::Fade, 1.2);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].transition_in,
        avcore::timeline::TransitionType::None
    );
    assert_eq!(clips[0].transition_duration_secs, 0.5);
    assert_eq!(
        clips[1].transition_in,
        avcore::timeline::TransitionType::Fade
    );
    assert_eq!(clips[1].transition_duration_secs, 1.2);
}

#[test]
fn set_selected_clip_transition_clamps_duration_to_range() {
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

    app.set_selected_clip_transition(avcore::timeline::TransitionType::Slide, 10.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].transition_duration_secs,
        *crate::app::TRANSITION_DURATION_RANGE.end()
    );
}

#[test]
fn set_selected_clip_transition_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_transition(avcore::timeline::TransitionType::Fade, 1.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].transition_in,
        avcore::timeline::TransitionType::None
    );
    assert_eq!(clips[0].transition_duration_secs, 0.5);
}

#[test]
fn set_selected_clip_scale_keyframes_updates_the_selected_clip() {
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

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].scale_keyframes.is_empty());
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
}

#[test]
fn set_selected_clip_scale_keyframes_clamps_each_value_independently() {
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

    app.set_selected_clip_scale_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 0.0,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 5.0,
        },
    ]);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].scale_keyframes,
        vec![
            avcore::Keyframe {
                time_fraction: 0.0,
                value: *crate::app::SCALE_RANGE.start(),
            },
            avcore::Keyframe {
                time_fraction: 1.0,
                value: *crate::app::SCALE_RANGE.end(),
            },
        ]
    );
}

#[test]
fn set_selected_clip_scale_keyframes_is_a_no_op_when_nothing_is_selected() {
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

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].scale_keyframes.is_empty());
}

#[test]
fn add_opacity_marker_at_playhead_is_a_no_op_when_nothing_is_selected() {
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
    app.active_project_mut().timeline_mut().playhead_secs = 4.0;

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].opacity_keyframes.is_empty());
}

#[test]
fn add_opacity_marker_at_playhead_is_a_no_op_outside_the_clips_own_span() {
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
    app.active_project_mut().timeline_mut().playhead_secs = 15.0; // past the clip's 10s span.

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(clips[0].opacity_keyframes.is_empty());
}

#[test]
fn add_opacity_marker_at_playhead_defaults_to_fully_opaque_with_no_existing_keyframes() {
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
    app.active_project_mut().timeline_mut().playhead_secs = 4.0; // 4s into a 10s clip -> 0.4.

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].opacity_keyframes.len(), 1);
    let added = clips[0].opacity_keyframes[0];
    assert!((added.time_fraction - 0.4).abs() < 1e-6);
    assert_eq!(added.value, 1.0);
}

#[test]
fn add_opacity_marker_at_playhead_preserves_the_currently_interpolated_value() {
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
    app.set_selected_clip_opacity_keyframes(vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 0.2,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 1.0,
        },
    ]);
    app.active_project_mut().timeline_mut().playhead_secs = 5.0; // halfway -> time_fraction 0.5.

    app.add_opacity_marker_at_playhead();

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].opacity_keyframes.len(), 3);
    let added = clips[0]
        .opacity_keyframes
        .iter()
        .find(|kf| (kf.time_fraction - 0.5).abs() < 1e-6)
        .expect("the new marker at time_fraction 0.5");
    // Halfway between 0.2 and 1.0 is 0.6 - adding the marker shouldn't itself change the
    // clip's current opacity.
    assert!((added.value - 0.6).abs() < 1e-6);
}

#[test]
fn set_selected_clip_chroma_key_updates_the_selected_clip() {
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

    app.set_selected_clip_chroma_key(true, [10, 200, 30], 0.7);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].chroma_key_enabled);
    assert!(clips[1].chroma_key_enabled);
    assert_eq!(clips[1].chroma_key_color, [10, 200, 30]);
    assert_eq!(clips[1].chroma_key_tolerance, 0.7);
}

#[test]
fn set_selected_clip_chroma_key_clamps_tolerance_to_range() {
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

    app.set_selected_clip_chroma_key(true, [0, 255, 0], 5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].chroma_key_tolerance,
        *crate::app::CHROMA_KEY_TOLERANCE_RANGE.end()
    );
}

#[test]
fn set_selected_clip_chroma_key_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_chroma_key(true, [10, 200, 30], 0.7);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert!(!clips[0].chroma_key_enabled);
}

#[test]
fn set_selected_clip_blur_updates_and_clamps() {
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

    app.set_selected_clip_blur(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].blur_intensity,
        *crate::app::BLUR_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_blur_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_blur(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].blur_intensity, 0.0);
}

#[test]
fn set_selected_clip_shake_updates_and_clamps() {
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

    app.set_selected_clip_shake(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].shake_intensity,
        *crate::app::SHAKE_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_shake_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_shake(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].shake_intensity, 0.0);
}

#[test]
fn set_selected_clip_glitch_updates_and_clamps() {
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

    app.set_selected_clip_glitch(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].glitch_intensity,
        *crate::app::GLITCH_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_glitch_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_glitch(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].glitch_intensity, 0.0);
}

#[test]
fn set_selected_clip_pixelize_updates_and_clamps() {
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

    app.set_selected_clip_pixelize(5.0);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(
        clips[0].pixelize_intensity,
        *crate::app::PIXELIZE_INTENSITY_RANGE.end()
    );
}

#[test]
fn set_selected_clip_pixelize_is_a_no_op_when_nothing_is_selected() {
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

    app.set_selected_clip_pixelize(0.5);

    let clips = &app.active_project().timeline().tracks[0].clips;
    assert_eq!(clips[0].pixelize_intensity, 0.0);
}
