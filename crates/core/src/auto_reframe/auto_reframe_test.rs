use super::*;

fn approx(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "{a} != {b}");
}

#[test]
fn no_subject_falls_back_to_center_crop() {
    // 1920x1080 -> 9:16: narrower target, full height kept.
    let rect = compute_reframe_crop(1920, 1080, 9, 16, None);
    approx(rect.h, 1.0);
    approx(rect.w, (9.0 / 16.0) / (1920.0 / 1080.0));
    approx(rect.x, (1.0 - rect.w) / 2.0);
    approx(rect.y, 0.0);
}

#[test]
fn wider_target_keeps_full_width() {
    // 1080x1920 source -> 16:9 target: wider than source, full width kept.
    let rect = compute_reframe_crop(1080, 1920, 16, 9, None);
    approx(rect.w, 1.0);
    approx(rect.h, (1080.0 / 1920.0) / (16.0 / 9.0));
}

#[test]
fn subject_off_center_shifts_crop_toward_it() {
    let rect = compute_reframe_crop(1920, 1080, 9, 16, Some((0.2, 0.5)));
    // Crop is centered on x=0.2 but clamped so it doesn't run past the left edge.
    assert!(rect.x < (1.0 - rect.w) / 2.0);
    assert!(rect.x >= 0.0);
}

#[test]
fn subject_near_edge_clamps_without_overflow() {
    let rect = compute_reframe_crop(1920, 1080, 9, 16, Some((0.98, 0.98)));
    assert!(rect.x + rect.w <= 1.0 + 1e-5);
    assert!(rect.y + rect.h <= 1.0 + 1e-5);
    assert!(rect.x >= 0.0);
    assert!(rect.y >= 0.0);
}

#[test]
fn same_aspect_ratio_keeps_full_frame() {
    let rect = compute_reframe_crop(1920, 1080, 16, 9, Some((0.3, 0.7)));
    approx(rect.w, 1.0);
    approx(rect.h, 1.0);
    approx(rect.x, 0.0);
    approx(rect.y, 0.0);
}

#[test]
fn main_subject_picks_highest_score() {
    let faces = vec![
        FaceBox {
            x: 0.0,
            y: 0.0,
            w: 0.1,
            h: 0.1,
            score: 0.75,
        },
        FaceBox {
            x: 0.4,
            y: 0.4,
            w: 0.2,
            h: 0.2,
            score: 0.95,
        },
    ];
    let center = main_subject_center(&faces).unwrap();
    approx(center.0, 0.5);
    approx(center.1, 0.5);
}

#[test]
fn main_subject_none_when_no_faces() {
    assert!(main_subject_center(&[]).is_none());
}

#[test]
fn nms_collapses_overlapping_boxes() {
    let candidates = vec![
        FaceBox {
            x: 0.1,
            y: 0.1,
            w: 0.3,
            h: 0.3,
            score: 0.9,
        },
        FaceBox {
            x: 0.12,
            y: 0.11,
            w: 0.3,
            h: 0.3,
            score: 0.8,
        },
        FaceBox {
            x: 0.7,
            y: 0.7,
            w: 0.2,
            h: 0.2,
            score: 0.85,
        },
    ];
    let kept = non_max_suppress(candidates);
    assert_eq!(kept.len(), 2);
    approx(kept[0].score, 0.9);
    approx(kept[1].score, 0.85);
}
