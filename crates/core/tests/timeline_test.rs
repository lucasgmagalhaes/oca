use avcore::timeline::{
    ClipInstance, ColorFilter, MaskShape, Timeline, Track, TrackKind, TransitionType,
};
use avcore::ClipFormatting;

fn clip(id: u64, start_secs: f64, source_in_secs: f64, source_out_secs: f64) -> ClipInstance {
    ClipInstance {
        id,
        asset_id: 1,
        start_secs,
        source_in_secs,
        source_out_secs,
        composite_id: None,
        gain_db: 0.0,
        frozen: false,
        speed_factor: 1.0,
        crop_x: 0.0,
        crop_y: 0.0,
        crop_w: 1.0,
        crop_h: 1.0,
        mask_shape: MaskShape::None,
        mask_corner_radius: 0.0,
        flipped_h: false,
        color_filter: ColorFilter::None,
        vignette_intensity: 0.0,
        brightness: 0.0,
        contrast: 1.0,
        saturation: 1.0,
        sharpen: 0.0,
        chroma_key_enabled: false,
        chroma_key_color: [0, 255, 0],
        chroma_key_tolerance: 0.4,
        blur_intensity: 0.0,
        shake_intensity: 0.0,
        glitch_intensity: 0.0,
        pixelize_intensity: 0.0,
        transition_in: TransitionType::None,
        transition_duration_secs: 0.5,
        zoom_start: 1.0,
        zoom_end: 1.0,
    }
}

#[test]
fn clip_duration_is_out_minus_in() {
    assert_eq!(clip(1, 0.0, 10.0, 30.0).duration_secs(), 20.0);
}

#[test]
fn zero_gain_db_is_unity_linear_gain() {
    assert_eq!(clip(1, 0.0, 0.0, 10.0).gain_linear(), 1.0);
}

#[test]
fn positive_gain_db_scales_linear_gain_above_unity() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.gain_db = 6.0;
    assert!((c.gain_linear() - 1.9953).abs() < 0.001);
}

#[test]
fn negative_gain_db_scales_linear_gain_below_unity() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.gain_db = -6.0;
    assert!((c.gain_linear() - 0.5012).abs() < 0.001);
}

#[test]
fn video_filter_chain_is_empty_for_a_neutral_clip() {
    assert_eq!(clip(1, 0.0, 0.0, 10.0).video_filter_chain(), "");
}

#[test]
fn video_filter_chain_includes_crop_when_cropped() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.crop_x = 0.1;
    c.crop_y = 0.2;
    c.crop_w = 0.5;
    c.crop_h = 0.6;
    assert_eq!(c.video_filter_chain(), "crop=iw*0.5:ih*0.6:iw*0.1:ih*0.2");
}

#[test]
fn video_filter_chain_includes_eq_when_color_adjusted() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.brightness = 0.2;
    assert_eq!(
        c.video_filter_chain(),
        "eq=brightness=0.2:contrast=1:saturation=1"
    );
}

#[test]
fn video_filter_chain_maps_black_and_white_to_hue_desaturate() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.color_filter = ColorFilter::BlackAndWhite;
    assert_eq!(c.video_filter_chain(), "hue=s=0");
}

#[test]
fn video_filter_chain_maps_sepia_to_colorchannelmixer() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.color_filter = ColorFilter::Sepia;
    assert_eq!(
        c.video_filter_chain(),
        "colorchannelmixer=.393:.769:.189:0:.349:.686:.168:0:.272:.534:.131:0"
    );
}

#[test]
fn video_filter_chain_includes_colorkey_when_chroma_keyed() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.chroma_key_enabled = true;
    c.chroma_key_color = [0, 255, 0];
    c.chroma_key_tolerance = 0.4;
    assert_eq!(c.video_filter_chain(), "colorkey=0x00ff00:0.400:0.1");
}

#[test]
fn video_filter_chain_includes_a_circle_mask_when_masked() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.mask_shape = MaskShape::Circle;
    assert_eq!(
        c.video_filter_chain(),
        "format=yuva420p,geq=lum='p(X,Y)':cb='cb(X,Y)':cr='cr(X,Y)':a='alpha(X,Y)*lte(pow(X-W/\
         2,2)+pow(Y-H/2,2),pow(min(W,H)/2,2))'"
    );
}

#[test]
fn video_filter_chain_includes_a_rounded_rect_mask_when_masked() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.mask_shape = MaskShape::RoundedRect;
    c.mask_corner_radius = 0.3;
    assert_eq!(
        c.video_filter_chain(),
        "format=yuva420p,geq=lum='p(X,Y)':cb='cb(X,Y)':cr='cr(X,Y)':a='alpha(X,Y)*lte(sqrt(pow(\
         max(abs(X-W/2)-(W/2-min(0.3000*min(W,H),min(W,H)/2)),0),2)+pow(max(abs(Y-H/2)-(H/2-min(\
         0.3000*min(W,H),min(W,H)/2)),0),2))-min(0.3000*min(W,H),min(W,H)/2),0)'"
    );
}

#[test]
fn video_filter_chain_orders_mask_after_chroma_key_before_blur() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.chroma_key_enabled = true;
    c.chroma_key_color = [0, 255, 0];
    c.chroma_key_tolerance = 0.4;
    c.mask_shape = MaskShape::Circle;
    c.blur_intensity = 0.5;
    let chain = c.video_filter_chain();
    let colorkey_pos = chain.find("colorkey").unwrap();
    let mask_pos = chain.find("geq").unwrap();
    let blur_pos = chain.find("boxblur").unwrap();
    assert!(colorkey_pos < mask_pos && mask_pos < blur_pos);
}

#[test]
fn video_filter_chain_includes_boxblur_when_blurred() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.blur_intensity = 0.5;
    assert_eq!(c.video_filter_chain(), "boxblur=5.00");
}

#[test]
fn video_filter_chain_includes_unsharp_when_sharpened() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.sharpen = 0.5;
    assert_eq!(c.video_filter_chain(), "unsharp=5:5:1.50:5:5:0.0");
}

#[test]
fn video_filter_chain_includes_vignette_when_vignetted() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.vignette_intensity = 0.8;
    assert_eq!(c.video_filter_chain(), "vignette=PI/4*0.800");
}

#[test]
fn video_filter_chain_includes_hflip_last_when_flipped() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.flipped_h = true;
    c.vignette_intensity = 0.5;
    assert_eq!(c.video_filter_chain(), "vignette=PI/4*0.500,hflip");
}

#[test]
fn video_filter_chain_orders_crop_before_color_before_flip() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.crop_w = 0.5;
    c.color_filter = ColorFilter::Sepia;
    c.flipped_h = true;
    assert_eq!(
        c.video_filter_chain(),
        "crop=iw*0.5:ih*1:iw*0:ih*0,colorchannelmixer=.393:.769:.189:0:.349:.686:.168:0:.272:.\
         534:.131:0,hflip"
    );
}

#[test]
fn empty_timeline_has_zero_duration() {
    let timeline = Timeline {
        tracks: vec![],
        playhead_secs: 0.0,
    };
    assert_eq!(timeline.duration_secs(), 0.0);
}

#[test]
fn timeline_duration_is_the_furthest_clip_end_across_all_tracks() {
    let timeline = Timeline {
        tracks: vec![
            Track {
                id: 1,
                name: "V1".to_string(),
                kind: TrackKind::Video,
                clips: vec![clip(1, 0.0, 0.0, 30.0), clip(2, 30.0, 0.0, 44.0)],

                text_clips: vec![],

                visible: true,

            },
            Track {
                id: 2,
                name: "A2".to_string(),
                kind: TrackKind::Audio,
                // Shorter overall, so it must not win over the V1 track's later end.
                clips: vec![clip(3, 0.0, 0.0, 10.0)],

                text_clips: vec![],

                visible: true,

            },
        ],
        playhead_secs: 0.0,
    };
    // Track V1's second clip ends at 30 + (44 - 0) = 74.
    assert_eq!(timeline.duration_secs(), 74.0);
}

fn track_with(clips: Vec<ClipInstance>) -> Track {
    Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips,

        text_clips: vec![],

        visible: true,

    }
}

#[test]
fn track_duration_is_zero_for_an_empty_track() {
    assert_eq!(track_with(Vec::new()).duration_secs(), 0.0);
}

#[test]
fn track_duration_is_the_single_clips_end() {
    assert_eq!(
        track_with(vec![clip(1, 5.0, 0.0, 20.0)]).duration_secs(),
        25.0
    );
}

#[test]
fn track_duration_is_the_furthest_clip_end_on_this_track_only() {
    let track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 50.0, 0.0, 5.0)]);
    // Second clip ends at 50 + 5 = 55, which is furthest even though it's shorter.
    assert_eq!(track.duration_secs(), 55.0);
}

#[test]
fn clip_at_finds_the_clip_covering_a_position() {
    let track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 5.0)]);
    assert_eq!(track.clip_at(3.0).unwrap().id, 1);
    assert_eq!(track.clip_at(12.0).unwrap().id, 2);
}

#[test]
fn clip_at_is_inclusive_of_a_clips_start() {
    let track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 5.0)]);
    assert_eq!(track.clip_at(10.0).unwrap().id, 2);
}

#[test]
fn clip_at_is_exclusive_of_a_clips_end() {
    let track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);
    assert!(track.clip_at(10.0).is_none());
}

#[test]
fn clip_at_is_none_for_a_gap_between_clips() {
    let track = track_with(vec![clip(1, 0.0, 0.0, 5.0), clip(2, 20.0, 0.0, 5.0)]);
    assert!(track.clip_at(10.0).is_none());
}

#[test]
fn clip_at_is_none_for_an_empty_track() {
    assert!(track_with(Vec::new()).clip_at(0.0).is_none());
}

#[test]
fn split_clip_at_divides_the_covering_clip_into_two() {
    let mut track = track_with(vec![clip(1, 10.0, 0.0, 20.0)]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    assert_eq!(
        track.clips,
        vec![clip(1, 10.0, 0.0, 10.0), clip(99, 20.0, 10.0, 20.0)]
    );
}

#[test]
fn split_clip_at_keeps_both_halves_in_the_same_composite_group() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.composite_id = Some(7);
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    assert_eq!(track.clips[0].composite_id, Some(7));
    assert_eq!(track.clips[1].composite_id, Some(7));
}

#[test]
fn split_clip_at_keeps_gain_db_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.gain_db = 3.0;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    assert_eq!(track.clips[0].gain_db, 3.0);
    assert_eq!(track.clips[1].gain_db, 3.0);
}

#[test]
fn split_clip_at_keeps_frozen_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.frozen = true;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    assert!(track.clips[0].frozen);
    assert!(track.clips[1].frozen);
}

#[test]
fn new_clip_defaults_to_normal_speed() {
    assert_eq!(clip(1, 0.0, 0.0, 10.0).speed_factor, 1.0);
}

#[test]
fn split_clip_at_keeps_speed_factor_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.speed_factor = 2.0;
    let mut track = track_with(vec![clip]);

    // With speed=2.0 the clip's timeline duration is (20-0)/2=10s, spanning 10..20.
    // Split at 15.0 (midpoint) — splitting at the former end boundary (20.0) would now
    // be outside the clip and return false.
    let split = track.split_clip_at(15.0, 99);

    assert!(split);
    assert_eq!(track.clips[0].speed_factor, 2.0);
    assert_eq!(track.clips[1].speed_factor, 2.0);
}

#[test]
fn new_clip_defaults_to_an_uncropped_full_frame() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert!(!c.is_cropped());
    assert_eq!(
        (c.crop_x, c.crop_y, c.crop_w, c.crop_h),
        (0.0, 0.0, 1.0, 1.0)
    );
}

#[test]
fn is_cropped_is_true_when_any_crop_field_differs_from_the_full_frame() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.crop_w = 0.5;
    assert!(c.is_cropped());
}

#[test]
fn split_clip_at_keeps_crop_rect_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.crop_x = 0.1;
    clip.crop_y = 0.2;
    clip.crop_w = 0.5;
    clip.crop_h = 0.6;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(
            (half.crop_x, half.crop_y, half.crop_w, half.crop_h),
            (0.1, 0.2, 0.5, 0.6)
        );
    }
}

#[test]
fn new_clip_defaults_to_unmasked() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert!(!c.is_masked());
    assert_eq!(c.mask_shape, MaskShape::None);
}

#[test]
fn is_masked_is_true_for_any_shape_but_none() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.mask_shape = MaskShape::Circle;
    assert!(c.is_masked());
}

#[test]
fn split_clip_at_keeps_mask_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.mask_shape = MaskShape::RoundedRect;
    clip.mask_corner_radius = 0.3;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(half.mask_shape, MaskShape::RoundedRect);
        assert_eq!(half.mask_corner_radius, 0.3);
    }
}

#[test]
fn new_clip_defaults_to_unflipped() {
    assert!(!clip(1, 0.0, 0.0, 10.0).flipped_h);
}

#[test]
fn split_clip_at_keeps_flip_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.flipped_h = true;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert!(half.flipped_h);
    }
}

#[test]
fn new_clip_defaults_to_unfiltered() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert!(!c.is_color_filtered());
    assert_eq!(c.color_filter, ColorFilter::None);
}

#[test]
fn is_color_filtered_is_true_for_any_filter_but_none() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.color_filter = ColorFilter::Sepia;
    assert!(c.is_color_filtered());
}

#[test]
fn split_clip_at_keeps_color_filter_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.color_filter = ColorFilter::BlackAndWhite;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(half.color_filter, ColorFilter::BlackAndWhite);
    }
}

#[test]
fn new_clip_defaults_to_no_vignette() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert!(!c.has_vignette());
    assert_eq!(c.vignette_intensity, 0.0);
}

#[test]
fn has_vignette_is_true_above_zero() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.vignette_intensity = 0.4;
    assert!(c.has_vignette());
}

#[test]
fn split_clip_at_keeps_vignette_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.vignette_intensity = 0.6;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(half.vignette_intensity, 0.6);
    }
}

#[test]
fn new_clip_defaults_to_unchanged_color_adjustment() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert_eq!((c.brightness, c.contrast, c.saturation), (0.0, 1.0, 1.0));
}

#[test]
fn split_clip_at_keeps_color_adjustment_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.brightness = 0.2;
    clip.contrast = 1.5;
    clip.saturation = 0.5;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(
            (half.brightness, half.contrast, half.saturation),
            (0.2, 1.5, 0.5)
        );
    }
}

#[test]
fn new_clip_defaults_to_unsharpened() {
    assert_eq!(clip(1, 0.0, 0.0, 10.0).sharpen, 0.0);
}

#[test]
fn split_clip_at_keeps_sharpen_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.sharpen = 0.4;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(half.sharpen, 0.4);
    }
}

#[test]
fn new_clip_defaults_to_chroma_key_disabled() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert!(!c.is_chroma_keyed());
    assert_eq!(c.chroma_key_color, [0, 255, 0]);
}

#[test]
fn split_clip_at_keeps_chroma_key_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.chroma_key_enabled = true;
    clip.chroma_key_color = [10, 200, 30];
    clip.chroma_key_tolerance = 0.7;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert!(half.chroma_key_enabled);
        assert_eq!(half.chroma_key_color, [10, 200, 30]);
        assert_eq!(half.chroma_key_tolerance, 0.7);
    }
}

#[test]
fn new_clip_defaults_to_no_other_effects() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert_eq!(
        (
            c.blur_intensity,
            c.shake_intensity,
            c.glitch_intensity,
            c.pixelize_intensity
        ),
        (0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn split_clip_at_keeps_other_effects_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.blur_intensity = 0.1;
    clip.shake_intensity = 0.2;
    clip.glitch_intensity = 0.3;
    clip.pixelize_intensity = 0.4;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(
            (
                half.blur_intensity,
                half.shake_intensity,
                half.glitch_intensity,
                half.pixelize_intensity
            ),
            (0.1, 0.2, 0.3, 0.4)
        );
    }
}

#[test]
fn new_clip_defaults_to_no_transition() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert!(!c.has_transition());
    assert_eq!(c.transition_in, TransitionType::None);
    assert_eq!(c.transition_duration_secs, 0.5);
}

#[test]
fn split_clip_at_keeps_transition_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.transition_in = TransitionType::Fade;
    clip.transition_duration_secs = 1.2;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!(half.transition_in, TransitionType::Fade);
        assert_eq!(half.transition_duration_secs, 1.2);
    }
}

#[test]
fn new_clip_defaults_to_unity_zoom() {
    let c = clip(1, 0.0, 0.0, 10.0);
    assert_eq!((c.zoom_start, c.zoom_end), (1.0, 1.0));
    assert!(!c.is_zoomed());
}

#[test]
fn split_clip_at_keeps_zoom_on_both_halves() {
    let mut clip = clip(1, 10.0, 0.0, 20.0);
    clip.zoom_start = 1.0;
    clip.zoom_end = 2.5;
    let mut track = track_with(vec![clip]);

    let split = track.split_clip_at(20.0, 99);

    assert!(split);
    for half in &track.clips {
        assert_eq!((half.zoom_start, half.zoom_end), (1.0, 2.5));
    }
}

#[test]
fn split_clip_at_only_splits_the_clip_that_covers_the_position() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]);

    let split = track.split_clip_at(15.0, 99);

    assert!(split);
    assert_eq!(track.clips[0], clip(1, 0.0, 0.0, 10.0));
    assert_eq!(
        track.clips[1..],
        [clip(2, 10.0, 0.0, 5.0), clip(99, 15.0, 5.0, 20.0)]
    );
}

#[test]
fn split_clip_at_is_a_no_op_when_nothing_covers_the_position() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    let split = track.split_clip_at(50.0, 99);

    assert!(!split);
    assert_eq!(track.clips, vec![clip(1, 0.0, 0.0, 10.0)]);
}

#[test]
fn split_clip_at_is_a_no_op_exactly_on_a_clip_boundary() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]);

    let split = track.split_clip_at(10.0, 99);

    assert!(!split);
    assert_eq!(
        track.clips,
        vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]
    );
}

#[test]
fn trim_start_shifts_start_and_source_in_by_the_same_delta() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    let trimmed = c.trim_start(15.0, 1.0);

    assert!(trimmed);
    assert_eq!(c, clip(1, 15.0, 10.0, 30.0));
}

#[test]
fn trim_start_is_a_no_op_when_it_would_go_negative() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    // Would push start_secs to -1.0.
    let trimmed = c.trim_start(-1.0, 1.0);

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn trim_start_is_a_no_op_when_it_would_shrink_below_the_minimum_duration() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    // delta = 24.5 -> source_in becomes 29.5 -> duration becomes 0.5, under the 1.0 minimum.
    let trimmed = c.trim_start(34.5, 1.0);

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn trim_end_extends_source_out_and_leaves_start_untouched() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    let trimmed = c.trim_end(25.0, 1.0, None);

    assert!(trimmed);
    // start=10, source_in=5, new duration = 25-10 = 15, so source_out = 5+15 = 20.
    assert_eq!(c, clip(1, 10.0, 5.0, 20.0));
}

#[test]
fn trim_end_is_a_no_op_past_the_source_medias_own_duration() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    // Would need source_out_secs = 35.0, past the 32.0 source duration.
    let trimmed = c.trim_end(40.0, 1.0, Some(32.0));

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn trim_end_is_a_no_op_when_it_would_shrink_below_the_minimum_duration() {
    let mut c = clip(1, 10.0, 5.0, 30.0);

    let trimmed = c.trim_end(10.5, 1.0, None);

    assert!(!trimmed);
    assert_eq!(c, clip(1, 10.0, 5.0, 30.0));
}

#[test]
fn clip_mut_finds_a_clip_by_id() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0), clip(2, 10.0, 0.0, 20.0)]);

    let found = track.clip_mut(2).unwrap();

    assert_eq!(found.start_secs, 10.0);
}

#[test]
fn clip_mut_returns_none_for_an_unknown_id() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    assert!(track.clip_mut(99).is_none());
}

#[test]
fn move_clip_repositions_start_secs_and_leaves_the_source_range_untouched() {
    let mut track = track_with(vec![clip(1, 0.0, 5.0, 15.0)]);

    let moved = track.move_clip(1, 40.0);

    assert!(moved);
    assert_eq!(track.clips[0], clip(1, 40.0, 5.0, 15.0));
}

#[test]
fn move_clip_is_a_no_op_for_a_negative_position() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    let moved = track.move_clip(1, -5.0);

    assert!(!moved);
    assert_eq!(track.clips[0], clip(1, 0.0, 0.0, 10.0));
}

fn timeline_with(tracks: Vec<Track>) -> Timeline {
    Timeline {
        tracks,
        playhead_secs: 0.0,
    }
}

#[test]
fn move_clip_to_track_relocates_the_clip_to_a_same_kind_track() {
    let mut timeline = timeline_with(vec![
        Track {
            id: 1,
            name: "V1".to_string(),
            kind: TrackKind::Video,
            clips: vec![clip(1, 0.0, 0.0, 10.0)],

            text_clips: vec![],

            visible: true,

        },
        Track {
            id: 2,
            name: "V2".to_string(),
            kind: TrackKind::Video,
            clips: vec![],

            text_clips: vec![],

            visible: true,

        },
    ]);

    let moved = timeline.move_clip_to_track(1, 2, 5.0);

    assert!(moved);
    assert!(timeline.tracks[0].clips.is_empty());
    assert_eq!(timeline.tracks[1].clips, vec![clip(1, 5.0, 0.0, 10.0)]);
}

#[test]
fn move_clip_to_track_is_a_no_op_across_mismatched_kinds() {
    let mut timeline = timeline_with(vec![
        Track {
            id: 1,
            name: "V1".to_string(),
            kind: TrackKind::Video,
            clips: vec![clip(1, 0.0, 0.0, 10.0)],

            text_clips: vec![],

            visible: true,

        },
        Track {
            id: 2,
            name: "A1".to_string(),
            kind: TrackKind::Audio,
            clips: vec![],

            text_clips: vec![],

            visible: true,

        },
    ]);

    let moved = timeline.move_clip_to_track(1, 2, 5.0);

    assert!(!moved);
    assert_eq!(timeline.tracks[0].clips, vec![clip(1, 0.0, 0.0, 10.0)]);
    assert!(timeline.tracks[1].clips.is_empty());
}

#[test]
fn move_clip_to_track_is_a_no_op_for_an_unknown_target_track() {
    let mut timeline = timeline_with(vec![Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![clip(1, 0.0, 0.0, 10.0)],

        text_clips: vec![],

        visible: true,

    }]);

    let moved = timeline.move_clip_to_track(1, 99, 5.0);

    assert!(!moved);
    assert_eq!(timeline.tracks[0].clips, vec![clip(1, 0.0, 0.0, 10.0)]);
}

#[test]
fn move_clip_to_track_is_a_no_op_for_a_negative_position() {
    let mut timeline = timeline_with(vec![
        Track {
            id: 1,
            name: "V1".to_string(),
            kind: TrackKind::Video,
            clips: vec![clip(1, 0.0, 0.0, 10.0)],

            text_clips: vec![],

            visible: true,

        },
        Track {
            id: 2,
            name: "V2".to_string(),
            kind: TrackKind::Video,
            clips: vec![],

            text_clips: vec![],

            visible: true,

        },
    ]);

    let moved = timeline.move_clip_to_track(1, 2, -5.0);

    assert!(!moved);
    assert_eq!(timeline.tracks[0].clips, vec![clip(1, 0.0, 0.0, 10.0)]);
    assert!(timeline.tracks[1].clips.is_empty());
}

#[test]
fn move_clip_is_a_no_op_for_an_unknown_clip_id() {
    let mut track = track_with(vec![clip(1, 0.0, 0.0, 10.0)]);

    let moved = track.move_clip(99, 5.0);

    assert!(!moved);
    assert_eq!(track.clips[0], clip(1, 0.0, 0.0, 10.0));
}

#[test]
fn formatting_roundtrip_preserves_all_fields() {
    let mut c = clip(1, 0.0, 0.0, 10.0);
    c.gain_db = 3.0;
    c.frozen = true;
    c.speed_factor = 1.5;
    c.crop_x = 0.1;
    c.crop_y = 0.2;
    c.crop_w = 0.7;
    c.crop_h = 0.8;
    c.mask_shape = MaskShape::Circle;
    c.mask_corner_radius = 0.25;
    c.flipped_h = true;
    c.color_filter = ColorFilter::Sepia;
    c.vignette_intensity = 0.6;
    c.brightness = 0.3;
    c.contrast = 1.2;
    c.saturation = 0.8;
    c.sharpen = 0.4;
    c.chroma_key_enabled = true;
    c.chroma_key_color = [10, 200, 30];
    c.chroma_key_tolerance = 0.35;
    c.blur_intensity = 0.2;
    c.shake_intensity = 0.1;
    c.glitch_intensity = 0.05;
    c.pixelize_intensity = 0.15;
    c.transition_in = TransitionType::Fade;
    c.transition_duration_secs = 1.0;
    c.zoom_start = 1.2;
    c.zoom_end = 1.8;

    let fmt = c.formatting();

    let mut dest = clip(2, 5.0, 0.0, 20.0);
    dest.apply_formatting(&fmt);

    assert_eq!(dest.formatting(), fmt);
    // Structural fields must not be touched by apply_formatting.
    assert_eq!(dest.id, 2);
    assert_eq!(dest.start_secs, 5.0);
    assert_eq!(dest.source_in_secs, 0.0);
    assert_eq!(dest.source_out_secs, 20.0);
}

#[test]
fn apply_formatting_overwrites_all_effect_fields() {
    let neutral: ClipFormatting = clip(1, 0.0, 0.0, 10.0).formatting();
    let mut c = clip(2, 0.0, 0.0, 5.0);
    c.gain_db = 6.0;
    c.color_filter = ColorFilter::BlackAndWhite;
    c.vignette_intensity = 0.9;

    c.apply_formatting(&neutral);

    assert_eq!(c.gain_db, 0.0);
    assert_eq!(c.color_filter, ColorFilter::None);
    assert_eq!(c.vignette_intensity, 0.0);
}

#[test]
fn timeline_clip_mut_finds_a_clip_across_tracks() {
    let mut timeline = timeline_with(vec![
        Track {
            id: 1,
            name: "V1".to_string(),
            kind: TrackKind::Video,
            clips: vec![clip(1, 0.0, 0.0, 10.0)],

            text_clips: vec![],

            visible: true,

        },
        Track {
            id: 2,
            name: "A1".to_string(),
            kind: TrackKind::Audio,
            clips: vec![clip(2, 0.0, 0.0, 10.0)],

            text_clips: vec![],

            visible: true,

        },
    ]);

    let found = timeline.clip_mut(2).unwrap();
    found.gain_db = 3.0;

    assert_eq!(timeline.tracks[1].clips[0].gain_db, 3.0);
    assert_eq!(timeline.tracks[0].clips[0].gain_db, 0.0);
}

#[test]
fn timeline_clip_mut_returns_none_for_an_unknown_id() {
    let mut timeline = timeline_with(vec![Track {
        id: 1,
        name: "V1".to_string(),
        kind: TrackKind::Video,
        clips: vec![clip(1, 0.0, 0.0, 10.0)],

        text_clips: vec![],

        visible: true,

    }]);

    assert!(timeline.clip_mut(99).is_none());
}
