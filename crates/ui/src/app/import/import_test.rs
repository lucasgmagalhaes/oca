use super::*;

fn frame(width: u32, height: u32, pixel: [u8; 4]) -> avcore::preview::VideoFrame {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        rgba.extend_from_slice(&pixel);
    }
    avcore::preview::VideoFrame {
        width,
        height,
        rgba,
    }
}

#[test]
fn keeps_the_same_size_when_already_within_max_dim() {
    let f = frame(50, 50, [1, 2, 3, 4]);
    let (w, h, out) = downscale_rgba(&f, 96);
    assert_eq!((w, h), (50, 50));
    assert_eq!(out, f.rgba);
}

#[test]
fn scales_the_longer_side_down_to_max_dim() {
    let f = frame(1920, 1080, [9, 9, 9, 255]);
    let (w, h, out) = downscale_rgba(&f, 96);
    assert_eq!(w, 96);
    assert_eq!(h, 54);
    assert_eq!(out.len(), (w * h * 4) as usize);
}

#[test]
fn scales_a_portrait_frame_by_its_longer_side_too() {
    let f = frame(1080, 1920, [1, 1, 1, 255]);
    let (w, h, _) = downscale_rgba(&f, 96);
    assert_eq!(h, 96);
    assert_eq!(w, 54);
}

#[test]
fn never_scales_a_dimension_down_to_zero() {
    let f = frame(4000, 4, [1, 1, 1, 255]);
    let (w, h, out) = downscale_rgba(&f, 96);
    assert!(w >= 1);
    assert!(h >= 1);
    assert_eq!(out.len(), (w * h * 4) as usize);
}

#[test]
fn samples_the_correct_source_pixel_for_a_two_color_frame() {
    let mut rgba = Vec::new();
    rgba.extend_from_slice(&[255, 0, 0, 255]);
    rgba.extend_from_slice(&[0, 0, 255, 255]);
    let f = avcore::preview::VideoFrame {
        width: 2,
        height: 1,
        rgba,
    };
    let (w, h, out) = downscale_rgba(&f, 1);
    assert_eq!((w, h), (1, 1));
    assert_eq!(&out[0..4], &[255, 0, 0, 255]);
}
