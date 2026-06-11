use forge_core::frames::{
    default_foot_anchor, manual_anchor, normalize_frames, CanvasMode, NormalizeOptions,
};
use image::{Rgba, RgbaImage};

fn frame_with_rect(
    width: u32,
    height: u32,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) -> RgbaImage {
    let mut image = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    for y in top..bottom {
        for x in left..right {
            image.put_pixel(x, y, Rgba([255, 255, 255, 255]));
        }
    }
    image
}

#[test]
fn manual_anchor_locks_user_coordinates() {
    let anchor = manual_anchor(17.0, 29.0);

    assert_eq!(anchor.x, 17.0);
    assert_eq!(anchor.y, 29.0);
    assert!(anchor.locked_by_user);
}

#[test]
fn default_foot_anchor_uses_canvas_center_and_bottom_margin() {
    let anchor = default_foot_anchor(64, 48, 6);

    assert_eq!(anchor.x, 32.0);
    assert_eq!(anchor.y, 42.0);
    assert!(!anchor.locked_by_user);
}

#[test]
fn normalized_frames_share_identical_dimensions() {
    let frames = vec![
        frame_with_rect(20, 16, 4, 4, 12, 14),
        frame_with_rect(16, 12, 3, 2, 10, 11),
    ];

    let normalized = normalize_frames(
        &frames,
        NormalizeOptions {
            mode: CanvasMode::SquareCenter,
            margin: 4,
            margin_bottom: 0,
            alpha_threshold: 0,
            manual_anchor: None,
        },
    );

    assert_eq!(normalized.len(), 2);
    assert_eq!(normalized[0].size, normalized[1].size);
    assert_eq!(
        normalized[0].image.dimensions(),
        normalized[1].image.dimensions()
    );
}

#[test]
fn square_bottom_keeps_bbox_bottom_near_anchor_y() {
    let frames = vec![
        frame_with_rect(24, 24, 6, 8, 14, 20),
        frame_with_rect(24, 24, 7, 7, 15, 18),
    ];

    let normalized = normalize_frames(
        &frames,
        NormalizeOptions {
            mode: CanvasMode::SquareBottom,
            margin: 4,
            margin_bottom: 3,
            alpha_threshold: 0,
            manual_anchor: None,
        },
    );

    for frame in normalized {
        assert!((frame.bbox.bottom_y - frame.anchor.y).abs() <= 0.5);
    }
}

#[test]
fn square_bottom_can_use_manual_anchor() {
    let frames = vec![frame_with_rect(24, 24, 6, 8, 14, 20)];

    let normalized = normalize_frames(
        &frames,
        NormalizeOptions {
            mode: CanvasMode::SquareBottom,
            margin: 4,
            margin_bottom: 3,
            alpha_threshold: 0,
            manual_anchor: Some(manual_anchor(12.0, 18.0)),
        },
    );

    assert_eq!(normalized.len(), 1);
    assert_eq!(normalized[0].anchor.x, 12.0);
    assert_eq!(normalized[0].anchor.y, 18.0);
    assert!(normalized[0].anchor.locked_by_user);
    assert!((normalized[0].bbox.bottom_y - 18.0).abs() <= 0.5);
}
