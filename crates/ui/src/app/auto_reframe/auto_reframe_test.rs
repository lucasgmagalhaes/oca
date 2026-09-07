use super::*;

fn solid_rgba(width: u32, height: u32, pixel: [u8; 4]) -> Vec<u8> {
    let mut buf = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..(width * height) {
        buf.extend_from_slice(&pixel);
    }
    buf
}

#[test]
fn is_a_no_op_when_already_within_max_dim() {
    let rgba = solid_rgba(100, 50, [1, 2, 3, 4]);
    let (w, h, out) = downscale_frame_rgba(100, 50, rgba.clone(), 100);
    assert_eq!((w, h), (100, 50));
    assert_eq!(out, rgba);
}

#[test]
fn is_a_no_op_when_smaller_than_max_dim() {
    let rgba = solid_rgba(10, 10, [1, 2, 3, 4]);
    let (w, h, _) = downscale_frame_rgba(10, 10, rgba, 960);
    assert_eq!((w, h), (10, 10));
}

#[test]
fn scales_the_longer_side_down_to_max_dim() {
    let rgba = solid_rgba(1920, 1080, [9, 9, 9, 255]);
    let (w, h, out) = downscale_frame_rgba(1920, 1080, rgba, 960);
    assert_eq!(w, 960);
    assert_eq!(h, 540);
    assert_eq!(out.len(), (w * h * 4) as usize);
}

#[test]
fn scales_a_portrait_frame_by_its_longer_side_too() {
    let rgba = solid_rgba(1080, 1920, [1, 1, 1, 255]);
    let (w, h, _) = downscale_frame_rgba(1080, 1920, rgba, 960);
    assert_eq!(h, 960);
    assert_eq!(w, 540);
}

#[test]
fn never_scales_a_dimension_down_to_zero() {
    // An extreme aspect ratio must still keep both dimensions at least 1 pixel.
    let rgba = solid_rgba(4000, 4, [1, 1, 1, 255]);
    let (w, h, out) = downscale_frame_rgba(4000, 4, rgba, 960);
    assert!(w >= 1);
    assert!(h >= 1);
    assert_eq!(out.len(), (w * h * 4) as usize);
}

#[test]
fn samples_the_correct_source_pixel_for_a_two_color_frame() {
    // A 2x1 frame, left pixel red and right pixel blue, downscaled to fit within 1px wide.
    let mut rgba = Vec::new();
    rgba.extend_from_slice(&[255, 0, 0, 255]);
    rgba.extend_from_slice(&[0, 0, 255, 255]);
    let (w, h, out) = downscale_frame_rgba(2, 1, rgba, 1);
    assert_eq!((w, h), (1, 1));
    // Nearest-neighbor at x=0 of a 1-wide destination maps back to source x=0 -- the red pixel.
    assert_eq!(&out[0..4], &[255, 0, 0, 255]);
}
