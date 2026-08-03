use forge_core::matting::chroma::{
    apply_chroma_key, process_chroma_batch, ChromaKeyMode, ChromaParameters,
};
use forge_core::preview::{preview_chroma_frame, TargetCanvasMode};
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
fn single_frame_preview_writes_source_processed_and_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let raw_path = temp.path().join("raw.png");
    let previews_dir = temp.path().join("previews");
    let image = RgbaImage::from_pixel(3, 2, Rgba([0, 255, 0, 255]));
    image.save(&raw_path).unwrap();

    let result = preview_chroma_frame(
        &raw_path,
        &base_params(),
        TargetCanvasMode::Original,
        &previews_dir,
    )
    .unwrap();

    assert!(previews_dir.join("source.png").exists());
    assert!(previews_dir.join("processed.png").exists());
    assert!(previews_dir.join("preview.json").exists());
    assert_eq!((result.processed_width, result.processed_height), (3, 2));
}
