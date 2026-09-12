use forge_core::{delivery::inventory, source_inspect::inspect_png};
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use std::fs;

#[test]
fn inspect_reports_actual_rgb_png_and_remains_read_only() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("rgb.png");
    RgbImage::from_pixel(8, 4, Rgb([20, 30, 40]))
        .save(&path)
        .unwrap();
    let before = inventory(root.path()).unwrap();
    let report = inspect_png(&path, Some((4, 4)), None).unwrap();
    assert_eq!(report["pngColorType"], 2);
    assert_eq!(report["pngBitDepth"], 8);
    assert_eq!(report["hasAlphaChannel"], false);
    assert_eq!(report["alpha"]["opaquePixels"], 32);
    assert_eq!(report["grid"]["cells"].as_array().unwrap().len(), 2);
    assert_eq!(report["grid"]["cells"][0]["touchesEdgeAlpha1"], true);
    assert_eq!(report["sha256"], before[0].sha256);
    assert_eq!(before, inventory(root.path()).unwrap());
}

#[test]
fn grid_edge_and_bounds_thresholds_include_exact_threshold_alpha() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("alpha.png");
    let mut image = RgbaImage::new(8, 4);
    image.put_pixel(3, 2, Rgba([0, 200, 0, 1]));
    image.put_pixel(4, 2, Rgba([0, 200, 0, 32]));
    image.save(&path).unwrap();
    let report = inspect_png(&path, Some((4, 4)), None).unwrap();
    assert_eq!(report["alphaThresholdOperator"], ">=");
    assert_eq!(report["bounds"][0]["bounds"]["width"], 2);
    assert_eq!(report["bounds"][2]["bounds"]["x"], 4);
    assert_eq!(report["bounds"][2]["bounds"]["width"], 1);
    assert_eq!(report["grid"]["cells"][0]["touchesEdgeAlpha1"], true);
    assert_eq!(report["grid"]["cells"][0]["touchesEdgeAlpha32"], false);
    assert_eq!(report["grid"]["cells"][1]["touchesEdgeAlpha32"], true);
}

#[test]
fn bit_depth_is_source_fact_and_8_bit_analysis_is_disclosed() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("16bit.png");
    image::ImageBuffer::from_pixel(4, 4, Rgba([50000u16, 1000, 2000, 64]))
        .save(&path)
        .unwrap();
    let report = inspect_png(&path, None, None).unwrap();
    assert_eq!(report["pngBitDepth"], 16);
    assert_eq!(report["decodedColor"], "Rgba16");
    assert_eq!(report["analysisColor"], "Rgba8");
    assert_eq!(report["alpha"]["transparentPixels"], 16);
}

#[test]
fn invalid_grid_or_format_cannot_create_preview_output() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("alpha.png");
    RgbaImage::new(8, 4).save(&path).unwrap();
    let previews = root.path().join("previews");
    assert!(inspect_png(&path, Some((0, 4)), Some(&previews)).is_err());
    assert!(!previews.exists());
    fs::write(&path, "not a PNG").unwrap();
    assert!(inspect_png(&path, None, Some(&previews)).is_err());
    assert!(!previews.exists());
}
