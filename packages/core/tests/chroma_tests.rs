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
