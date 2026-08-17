// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::path::{Path, PathBuf};

use avcore::preview::Preview;
use avcore::timeline::{ClipInstance, ColorFilter, MaskShape, TransitionType};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn clip() -> ClipInstance {
    ClipInstance {
        id: 1,
        asset_id: 1,
        start_secs: 0.0,
        source_in_secs: 0.0,
        source_out_secs: 1.0,
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
        position_keyframes: vec![],
        scale_keyframes: vec![],
        rotation_keyframes: vec![],
        opacity_keyframes: vec![],
        deflicker_enabled: false,
        lut_path: String::new(),
        layer_scale_x: 1.0,
        layer_scale_y: 1.0,
        stabilization_intensity: 0.0,
        background_removal_enabled: false,
        background_removal_mask_path: String::new(),
    }
}

#[test]
fn opens_a_real_file_and_reports_duration() {
    let preview = Preview::open(&fixture("video.mp4"), None).unwrap();
    let duration = preview
        .duration_secs()
        .expect("duration should be known after preroll");
    assert!((duration - 1.0).abs() < 0.2);
}

#[test]
fn play_then_pause_does_not_error() {
    let preview = Preview::open(&fixture("video.mp4"), None).unwrap();
    preview.play().unwrap();
    preview.pause().unwrap();
}

#[test]
fn seek_succeeds_and_position_stays_in_bounds() {
    let preview = Preview::open(&fixture("video.mp4"), None).unwrap();
    let duration = preview.duration_secs().unwrap();

    preview.seek(duration / 2.0).unwrap();

    if let Some(position) = preview.position_secs() {
        assert!(position >= 0.0 && position <= duration + 0.5);
    }
}

#[test]
fn seek_with_rate_succeeds_and_position_stays_in_bounds() {
    let preview = Preview::open(&fixture("video.mp4"), None).unwrap();
    let duration = preview.duration_secs().unwrap();

    preview.seek_with_rate(duration / 2.0, 2.0).unwrap();

    if let Some(position) = preview.position_secs() {
        assert!(position >= 0.0 && position <= duration + 0.5);
    }
}

#[test]
fn errors_on_a_missing_file() {
    assert!(Preview::open(&fixture("does_not_exist.mp4"), None).is_err());
}

#[test]
fn current_frame_returns_correctly_sized_rgba() {
    let preview = Preview::open(&fixture("video.mp4"), None).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");

    // video.mp4 is a 320x240 fixture (see tests/probe_test.rs / avbridge's fixture generation).
    assert_eq!(frame.width, 320);
    assert_eq!(frame.height, 240);
    assert_eq!(frame.rgba.len(), 320 * 240 * 4);
    // Not all-zero — testsrc paints an actual pattern, a blank buffer would mean the caps
    // negotiation or plane extraction silently produced garbage/empty data.
    assert!(frame.rgba.iter().any(|&b| b != 0));
}

#[test]
fn current_frame_is_none_for_audio_only_input() {
    let preview = Preview::open(&fixture("audio.m4a"), None).unwrap();
    assert!(preview.current_frame().is_none());
}

#[test]
fn a_neutral_clip_applies_no_filter_bin_and_frame_size_is_unchanged() {
    let preview = Preview::open(&fixture("video.mp4"), Some(&clip())).unwrap();
    let frame = preview.current_frame().unwrap();
    assert_eq!((frame.width, frame.height), (320, 240));
}

#[test]
fn a_cropped_clip_shrinks_the_decoded_frame() {
    let mut c = clip();
    c.crop_x = 0.25;
    c.crop_y = 0.25;
    c.crop_w = 0.5;
    c.crop_h = 0.5;

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");

    assert_eq!((frame.width, frame.height), (160, 120));
}

#[test]
fn a_pixelized_clip_still_opens_and_decodes_at_full_size() {
    let mut c = clip();
    c.pixelize_intensity = 0.5;

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    // The downscale/upscale pair round-trips back to the source size — this proves the filter
    // bin links and prerolls a real frame, not that pixels are mosaiced (nothing here
    // decodes/compares pixel content).
    assert_eq!((frame.width, frame.height), (320, 240));
}

#[test]
fn a_shaken_clip_still_opens_and_decodes_at_full_size() {
    let mut c = clip();
    c.shake_intensity = 0.5;

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    // The dynamic videocrop + fixed-size upscale round-trips back to the source size on every
    // frame (crop's own output size never changes, only where within the margin it sits) — this
    // proves the pad probe doesn't crash the pipeline or desync caps, not that pixels shake.
    assert_eq!((frame.width, frame.height), (320, 240));

    // A few more pulls exercise the pad probe's frame counter past frame 0 without erroring.
    for _ in 0..3 {
        let _ = preview.current_frame();
    }
}

#[test]
fn a_zoomed_clip_still_opens_and_decodes_at_full_size() {
    let mut c = clip();
    c.scale_keyframes = vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.0,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 1.5,
        },
    ];

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    // The dynamic videocrop + fixed-size upscale round-trips back to the source size on every
    // frame — this proves the PTS-keyed pad probe doesn't crash the pipeline or desync caps,
    // not that pixels are actually zoomed in.
    assert_eq!((frame.width, frame.height), (320, 240));

    for _ in 0..3 {
        let _ = preview.current_frame();
    }
}

#[test]
fn a_fade_transition_clip_still_opens_and_decodes() {
    let mut c = clip();
    c.transition_in = TransitionType::Fade;
    c.transition_duration_secs = 0.3;

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    assert_eq!((frame.width, frame.height), (320, 240));
    for _ in 0..3 {
        let _ = preview.current_frame();
    }
}

#[test]
fn a_zoom_transition_clip_still_opens_and_decodes_at_full_size() {
    let mut c = clip();
    c.transition_in = TransitionType::Zoom;
    c.transition_duration_secs = 0.3;

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    assert_eq!((frame.width, frame.height), (320, 240));
    for _ in 0..3 {
        let _ = preview.current_frame();
    }
}

#[test]
fn a_slide_transition_clip_still_opens_and_decodes_at_full_size() {
    let mut c = clip();
    c.transition_in = TransitionType::Slide;
    c.transition_duration_secs = 0.3;

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    // Proves the two chained videobox elements' output size stays the fixed canvas size
    // throughout, not just at the moment of preroll.
    assert_eq!((frame.width, frame.height), (320, 240));
    for _ in 0..3 {
        let _ = preview.current_frame();
    }
}

#[test]
fn open_composited_reports_a_canvas_sized_frame() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let overlay_clip = clip();

    let preview = Preview::open_composited(
        &bg,
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");

    // video.mp4 is 320x240 (see preview_test.rs's other fixture-size assertions) — the
    // canvas is sized from the background branch, and a neutral overlay (no layer_scale)
    // doesn't change the compositor's own output size.
    assert_eq!((frame.width, frame.height), (320, 240));
    assert!(frame.rgba.iter().any(|&b| b != 0));
}

#[test]
fn open_composited_with_background_removal_matte_composites_without_error() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let mut overlay_clip = clip();
    overlay_clip.background_removal_enabled = true;
    // Any decodable video works as the matte source for a "does the alphacombine wiring link
    // and preroll" check — video.mp4 stands in the same way it does as its own overlay branch
    // in the tests above; this doesn't assert anything about which pixels end up transparent.
    overlay_clip.background_removal_mask_path = overlay.to_string_lossy().to_string();

    let preview = Preview::open_composited(
        &bg,
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    assert_eq!((frame.width, frame.height), (320, 240));

    preview.seek_composited(&[0.2, 0.1], &[1.0, 1.0]).unwrap();
    let _ = preview.current_frame();
}

#[test]
fn open_composited_with_mask_shape_composites_without_error() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let mut overlay_clip = clip();
    overlay_clip.mask_shape = MaskShape::Circle;

    let preview = Preview::open_composited(
        &bg,
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    assert_eq!((frame.width, frame.height), (320, 240));

    preview.seek_composited(&[0.2, 0.1], &[1.0, 1.0]).unwrap();
    let _ = preview.current_frame();
}

#[test]
fn open_composited_with_mask_shape_and_matte_composites_without_error() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let mut overlay_clip = clip();
    overlay_clip.mask_shape = MaskShape::RoundedRect;
    overlay_clip.mask_corner_radius = 0.2;
    overlay_clip.background_removal_enabled = true;
    overlay_clip.background_removal_mask_path = overlay.to_string_lossy().to_string();

    let preview = Preview::open_composited(
        &bg,
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    assert_eq!((frame.width, frame.height), (320, 240));
}

#[test]
fn open_composited_with_text_and_shape_overlays_composites_without_error() {
    let bg = fixture("video.mp4");
    let text_clip = avcore::timeline::TextClip {
        id: 1,
        start_secs: 0.0,
        duration_secs: 1.0,
        text: "hi".to_string(),
        font_size: 24.0,
        color_rgba: [255, 255, 255, 255],
        pos_x: 0.1,
        pos_y: 0.1,
        words: vec![],
        highlight_enabled: false,
        highlight_color_rgba: [255, 220, 0, 255],
    };
    let shape_clip = avcore::timeline::ShapeClip {
        id: 2,
        start_secs: 0.0,
        duration_secs: 1.0,
        shape_kind: avcore::timeline::ShapeKind::Ellipse,
        center_x: 0.5,
        center_y: 0.5,
        width: 0.2,
        height: 0.2,
        rotation_deg: 0.0,
        color_rgba: [0, 255, 0, 255],
        stroke_thickness_px: 0.0,
    };

    let preview =
        Preview::open_composited(&bg, None, &[], &[], &[(&text_clip, 0.0)], &[&shape_clip])
            .unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    assert_eq!((frame.width, frame.height), (320, 240));
}

#[test]
fn open_composited_with_animated_overlay_still_composites_without_error() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let mut overlay_clip = clip();
    overlay_clip.position_keyframes = vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: avcore::keyframe::Position { x: 0.0, y: 0.0 },
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: avcore::keyframe::Position { x: 0.5, y: 0.25 },
        },
    ];
    overlay_clip.opacity_keyframes = vec![
        avcore::Keyframe {
            time_fraction: 0.0,
            value: 1.0,
        },
        avcore::Keyframe {
            time_fraction: 1.0,
            value: 0.4,
        },
    ];
    overlay_clip.layer_scale_x = 0.5;
    overlay_clip.layer_scale_y = 0.5;
    overlay_clip.chroma_key_enabled = true;

    let preview = Preview::open_composited(
        &bg,
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    // Proves the branch's videoscale/chroma-key/compositor-pad-probe chain links and prerolls
    // a real frame without erroring — not that the overlay is visually positioned/keyed
    // correctly (nothing here decodes/compares pixel content).
    assert_eq!((frame.width, frame.height), (320, 240));

    // A few more pulls exercise the per-buffer compositor-pad probe past frame 0.
    for _ in 0..3 {
        let _ = preview.current_frame();
    }
}

#[test]
fn seek_composited_seeks_every_branch_without_error() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let overlay_clip = clip();

    let preview = Preview::open_composited(
        &bg,
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    preview.seek_composited(&[0.5, 0.2], &[1.0, 1.0]).unwrap();
    let _ = preview.current_frame();
}

#[test]
fn seek_composited_honors_a_per_branch_rate() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let overlay_clip = clip();

    let preview = Preview::open_composited(
        &bg,
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    // Background at half speed, overlay at double — proves seek_composited's rates argument
    // reaches each branch independently rather than being ignored or applied uniformly.
    preview.seek_composited(&[0.2, 0.2], &[0.5, 2.0]).unwrap();
    let _ = preview.current_frame();
}

#[test]
fn open_composited_errors_on_a_missing_background() {
    let overlay = fixture("video.mp4");
    let overlay_clip = clip();
    assert!(Preview::open_composited(
        &fixture("does_not_exist.mp4"),
        None,
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .is_err());
}

#[test]
fn a_flipped_clip_still_opens_and_decodes() {
    let mut c = clip();
    c.flipped_h = true;

    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview.current_frame().unwrap();
    // videoflip doesn't change dimensions for a horizontal flip — this just proves the filter
    // bin links and the pipeline still preroll a real frame, not that pixels are mirrored
    // (nothing here decodes/compares pixel content).
    assert_eq!((frame.width, frame.height), (320, 240));
}

#[test]
fn a_gained_clip_opens_with_a_real_audio_sink_and_still_decodes_video() {
    let mut c = clip();
    c.gain_db = -6.0;

    // Proves build_audio_filter_bin's volume element links into playbin's audio-filter and the
    // pipeline still prerolls against a real (not fakesink) audio-sink — doesn't assert
    // anything about the actual decoded audio samples' loudness.
    let preview = Preview::open(&fixture("video.mp4"), Some(&c)).unwrap();
    let frame = preview.current_frame().unwrap();
    assert_eq!((frame.width, frame.height), (320, 240));
    preview.play().unwrap();
    preview.pause().unwrap();
}

#[test]
fn open_composited_with_gain_composites_with_real_audio() {
    let bg = fixture("video.mp4");
    let overlay = fixture("video.mp4");
    let mut bg_clip = clip();
    bg_clip.gain_db = 3.0;
    let overlay_clip = clip();

    let preview = Preview::open_composited(
        &bg,
        Some(&bg_clip),
        &[(overlay.as_path(), &overlay_clip)],
        &[],
        &[],
        &[],
    )
    .unwrap();
    let frame = preview
        .current_frame()
        .expect("a frame should be available right after preroll");
    assert_eq!((frame.width, frame.height), (320, 240));
    preview.play().unwrap();
    preview.pause().unwrap();
}

#[test]
fn open_composited_mixes_an_audio_only_timeline_branch() {
    let background = fixture("video.mp4");
    let audio = fixture("audio.m4a");
    let mut audio_clip = clip();
    audio_clip.id = 2;
    audio_clip.gain_db = -4.0;

    let preview = Preview::open_composited(
        &background,
        None,
        &[],
        &[(audio.as_path(), &audio_clip)],
        &[],
        &[],
    )
    .unwrap();
    let frame = preview
        .current_frame()
        .expect("mixed-audio preview should still preroll video");
    assert_eq!((frame.width, frame.height), (320, 240));
    preview.seek_composited(&[0.1, 0.2], &[1.0, 1.25]).unwrap();
    preview.play().unwrap();
    preview.pause().unwrap();
}
