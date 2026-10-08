use forge_core::matting::chroma::{
    apply_chroma_key, process_chroma_batch, ChromaBackgroundScope, ChromaKeyMode, ChromaParameters,
};
use image::{Rgba, RgbaImage};
use std::path::PathBuf;

fn base_params() -> ChromaParameters {
    ChromaParameters {
        key_mode: ChromaKeyMode::AutoCorners,
        manual_key_color: "#00FF00".to_string(),
        threshold: 24,
        softness: 0,
        despill_strength: 0.0,
        halo_pixels: 0,
        background_scope: ChromaBackgroundScope::Auto,
        edge_color_recovery: false,
    }
}

#[test]
fn green_background_alpha_becomes_zero_and_white_foreground_stays_opaque() {
    let mut image = RgbaImage::from_pixel(4, 4, Rgba([0, 255, 0, 255]));
    image.put_pixel(1, 1, Rgba([255, 255, 255, 255]));

    let processed = apply_chroma_key(&image, &base_params()).unwrap();

    assert_eq!(processed.get_pixel(0, 0)[3], 0);
    assert_eq!(processed.get_pixel(3, 3)[3], 0);
    assert_eq!(processed.get_pixel(1, 1)[3], 255);
}

#[test]
fn border_connected_gradient_green_is_removed_without_erasing_enclosed_green_details() {
    let mut image = RgbaImage::from_fn(96, 96, |x, y| {
        let green = 120 + ((x + y) % 80) as u8;
        Rgba([8, green, 18, 255])
    });
    for y in 24..72 {
        for x in 28..68 {
            let outline = x == 28 || x == 67 || y == 24 || y == 71;
            image.put_pixel(
                x,
                y,
                if outline {
                    Rgba([18, 14, 26, 255])
                } else {
                    Rgba([20, 90, 110, 255])
                },
            );
        }
    }
    image.put_pixel(48, 48, Rgba([0, 200, 60, 255]));

    let processed = apply_chroma_key(&image, &base_params()).unwrap();

    assert_eq!(processed.get_pixel(2, 2)[3], 0);
    assert_eq!(processed.get_pixel(90, 80)[3], 0);
    assert_eq!(processed.get_pixel(40, 40)[3], 255);
    assert_eq!(processed.get_pixel(48, 48)[3], 255);
}

#[test]
fn manual_key_color_overrides_corner_sampling() {
    let mut image = RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255]));
    image.put_pixel(1, 1, Rgba([0, 255, 0, 255]));

    let mut params = base_params();
    params.key_mode = ChromaKeyMode::Manual;
    params.manual_key_color = "#00FF00".to_string();

    let processed = apply_chroma_key(&image, &params).unwrap();

    assert_eq!(processed.get_pixel(1, 1)[3], 0);
    assert_eq!(processed.get_pixel(0, 0)[3], 255);
}

#[test]
fn batch_processed_frame_dimensions_match_raw_before_normalization() {
    let temp = tempfile::tempdir().unwrap();
    let raw_dir = temp.path().join("raw");
    let processed_dir = temp.path().join("processed");
    std::fs::create_dir_all(&raw_dir).unwrap();

    let raw_path = raw_dir.join("frame_00001.png");
    let mut image = RgbaImage::from_pixel(7, 5, Rgba([0, 255, 0, 255]));
    image.put_pixel(3, 2, Rgba([255, 255, 255, 255]));
    image.save(&raw_path).unwrap();

    let result =
        process_chroma_batch(&[PathBuf::from(&raw_path)], &processed_dir, &base_params()).unwrap();
    let processed = image::open(processed_dir.join("frame_00001.png"))
        .unwrap()
        .to_rgba8();

    assert_eq!((processed.width(), processed.height()), (7, 5));
    assert_eq!(result.frames[0].width, 7);
    assert_eq!(result.frames[0].height, 5);
    assert!(processed_dir.join("bboxes.json").exists());
}

#[test]
fn manual_border_connected_scope_preserves_enclosed_key_colored_detail() {
    let mut image = RgbaImage::from_pixel(32, 32, Rgba([255, 0, 255, 255]));
    for y in 10..22 {
        for x in 10..22 {
            let barrier = x == 10 || x == 21 || y == 10 || y == 21;
            image.put_pixel(
                x,
                y,
                if barrier {
                    Rgba([20, 25, 30, 255])
                } else {
                    Rgba([255, 0, 255, 255])
                },
            );
        }
    }

    let mut params = base_params();
    params.key_mode = ChromaKeyMode::Manual;
    params.manual_key_color = "#FF00FF".into();
    params.background_scope = ChromaBackgroundScope::BorderConnected;

    let processed = apply_chroma_key(&image, &params).unwrap();

    assert_eq!(processed.get_pixel(0, 0)[3], 0);
    assert_eq!(processed.get_pixel(16, 16)[3], 255);
    assert_eq!(&processed.get_pixel(16, 16).0[..3], &[255, 0, 255]);
}

#[test]
fn edge_color_recovery_reconstructs_a_blended_foreground_edge() {
    let mut image = RgbaImage::from_pixel(24, 24, Rgba([255, 0, 255, 255]));
    image.put_pixel(6, 12, Rgba([128, 128, 255, 255]));

    let mut params = base_params();
    params.key_mode = ChromaKeyMode::Manual;
    params.manual_key_color = "#FF00FF".into();
    params.threshold = 52;
    params.softness = 255;
    params.background_scope = ChromaBackgroundScope::BorderConnected;
    params.edge_color_recovery = true;

    let processed = apply_chroma_key(&image, &params).unwrap();
    let edge = processed.get_pixel(6, 12);

    assert!(edge[3] > 120 && edge[3] < 135, "{edge:?}");
    assert!(edge[0] < 8, "{edge:?}");
    assert!(edge[1] > 247, "{edge:?}");
    assert_eq!(edge[2], 255);
}

#[test]
fn recovery_and_despill_compose_without_changing_alpha() {
    let image = RgbaImage::from_fn(4, 1, |x, _| {
        Rgba(match x {
            0 => [0, 255, 0, 255],   // Removed background.
            1 => [50, 190, 50, 255], // Soft edge, still green after recovery.
            2 => [24, 80, 20, 255],  // Opaque spill: recovery is a no-op.
            _ => [200, 80, 40, 128], // Non-key foreground with source alpha.
        })
    });
    let mut params = base_params();
    params.key_mode = ChromaKeyMode::Manual;
    params.background_scope = ChromaBackgroundScope::Global;
    params.threshold = 40;
    params.softness = 100;
    params.edge_color_recovery = true;
    let recovery_only = apply_chroma_key(&image, &params).unwrap();

    params.despill_strength = 1.0;
    let combined = apply_chroma_key(&image, &params).unwrap();
    for (recovered, corrected) in recovery_only.pixels().zip(combined.pixels()) {
        assert_eq!(recovered[3], corrected[3]);
        assert_eq!(recovered[0], corrected[0]);
        assert_eq!(recovered[2], corrected[2]);
    }
    let soft_edge = combined.get_pixel(1, 0);
    assert!(soft_edge[3] > 0 && soft_edge[3] < 255);
    assert!(soft_edge[1] < recovery_only.get_pixel(1, 0)[1]);
    // Despill acts on recovered RGB, rather than replacing the recovered result.
    assert_eq!(&soft_edge.0[..3], &[89, 103, 89]);
    assert_eq!(combined.get_pixel(2, 0), &Rgba([24, 52, 20, 255]));
    assert_eq!(combined.get_pixel(3, 0), image.get_pixel(3, 0));
    assert_eq!(combined.get_pixel(0, 0)[3], 0);

    params.edge_color_recovery = false;
    let despill_only = apply_chroma_key(&image, &params).unwrap();
    assert_eq!(combined.get_pixel(2, 0), despill_only.get_pixel(2, 0));
    assert_ne!(combined.get_pixel(1, 0), despill_only.get_pixel(1, 0));
}

#[test]
fn combined_recovery_and_despill_preserve_border_scope_exclusions() {
    let mut image = RgbaImage::from_pixel(5, 5, Rgba([0, 255, 0, 255]));
    for y in 1..4 {
        for x in 1..4 {
            image.put_pixel(x, y, Rgba([24, 80, 20, 255]));
        }
    }
    image.put_pixel(2, 2, Rgba([0, 255, 0, 255]));
    let mut params = base_params();
    params.key_mode = ChromaKeyMode::Manual;
    params.background_scope = ChromaBackgroundScope::BorderConnected;
    params.edge_color_recovery = true;
    params.despill_strength = 1.0;
    let processed = apply_chroma_key(&image, &params).unwrap();
    assert_eq!(processed.get_pixel(0, 0)[3], 0);
    assert_eq!(processed.get_pixel(1, 1), image.get_pixel(1, 1));
    assert_eq!(processed.get_pixel(2, 2), image.get_pixel(2, 2));
}
