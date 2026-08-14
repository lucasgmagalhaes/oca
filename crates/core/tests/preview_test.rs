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
