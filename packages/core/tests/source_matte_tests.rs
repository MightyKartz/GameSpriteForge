use forge_core::{
    matting::{ChromaKeyMode, ChromaParameters},
    source_matte::{matte_png, SourceMatteRequest},
};
use image::{Rgb, RgbImage, Rgba, RgbaImage};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn request(root: &Path) -> SourceMatteRequest {
    SourceMatteRequest {
        schema_version: "1".into(),
        input: root.join("source.png"),
        output: root.join("new/matte.png"),
        parameters: ChromaParameters {
            key_mode: ChromaKeyMode::Manual,
            manual_key_color: "#FFFFFF".into(),
            threshold: 10,
            softness: 100,
            despill_strength: 0.0,
            halo_pixels: 0,
        },
    }
}

#[test]
fn rgb_matte_preserves_rectangle_coordinates_and_soft_alpha_with_hash_evidence() {
    let root = tempfile::tempdir().unwrap();
    let request = request(root.path());
    let mut source = RgbImage::from_pixel(23, 11, Rgb([255, 255, 255]));
    source.put_pixel(3, 7, Rgb([255, 255, 200]));
    source.put_pixel(19, 2, Rgb([70, 80, 90]));
    source.save(&request.input).unwrap();
    let before = fs::read(&request.input).unwrap();
    let report = matte_png(&request).unwrap();
    let output = image::open(&request.output).unwrap().to_rgba8();
    assert_eq!(output.dimensions(), (23, 11));
    assert_eq!(output.get_pixel(19, 2), &Rgba([70, 80, 90, 255]));
    let soft = output.get_pixel(3, 7);
    assert_eq!(&soft.0[..3], &[255, 255, 200]);
    assert!(soft[3] > 0 && soft[3] < 255);
    assert!(output
        .pixels()
        .filter(|p| p[3] == 0)
        .all(|p| p.0 == [0, 0, 0, 0]));
    assert_eq!(fs::read(&request.input).unwrap(), before);
    assert_eq!(
        report.source_sha256,
        format!("{:x}", Sha256::digest(&before))
    );
    assert_eq!(
        report.output_sha256,
        format!("{:x}", Sha256::digest(fs::read(&request.output).unwrap()))
    );
    assert_eq!(report.parameters, request.parameters);
    assert_eq!(report.resolved_key_color, "#FFFFFF");
    assert_eq!(report.partial_alpha_pixels, 1);
    assert_eq!(report.provider_request_count, 0);
    assert!(
        report.canvas_preserved && report.transparent_rgb_cleared && report.visual_review_required
    );
}

#[test]
fn rgba_auto_corners_retains_subject_alpha_and_clears_transparent_rgb() {
    let root = tempfile::tempdir().unwrap();
    let mut request = request(root.path());
    request.parameters = ChromaParameters::default();
    let mut source = RgbaImage::from_pixel(17, 9, Rgba([0, 255, 0, 255]));
    source.put_pixel(4, 3, Rgba([200, 20, 20, 89]));
    source.put_pixel(11, 6, Rgba([100, 120, 180, 0]));
    source.save(&request.input).unwrap();
    let report = matte_png(&request).unwrap();
    let output = image::open(&request.output).unwrap().to_rgba8();
    assert_eq!(output.dimensions(), (17, 9));
    assert_eq!(output.get_pixel(4, 3)[3], 89);
    assert_eq!(output.get_pixel(11, 6).0, [0, 0, 0, 0]);
    assert_eq!(report.resolved_key_color, "#00FF00");
    assert_eq!(report.source_color_type, "rgba8");
}

#[test]
fn rejects_in_place_existing_outputs_invalid_parameters_and_empty_foreground_without_writes() {
    let root = tempfile::tempdir().unwrap();
    let mut request = request(root.path());
    RgbImage::from_pixel(7, 3, Rgb([255, 255, 255]))
        .save(&request.input)
        .unwrap();
    let before = fs::read(&request.input).unwrap();
    request.output = request.input.clone();
    assert!(matte_png(&request).unwrap_err().contains("already exists"));
    assert_eq!(fs::read(&request.input).unwrap(), before);
    request.output = root.path().join("new/out.png");
    assert!(matte_png(&request)
        .unwrap_err()
        .contains("all visible pixels"));
    assert!(!request.output.exists());
    assert!(!root.path().join("new").exists());
    request.parameters.despill_strength = f32::NAN;
    assert!(matte_png(&request).is_err());
    request.parameters.despill_strength = 3.0;
    assert!(matte_png(&request).is_err());
    request.parameters.despill_strength = 0.0;
    request.parameters.halo_pixels = 5;
    assert!(matte_png(&request).is_err());
}

#[test]
fn existing_output_is_never_replaced() {
    let root = tempfile::tempdir().unwrap();
    let mut request = request(root.path());
    request.output = root.path().join("existing.png");
    fs::write(&request.output, b"retained output").unwrap();
    RgbImage::from_pixel(7, 3, Rgb([80, 90, 100]))
        .save(&request.input)
        .unwrap();
    assert!(matte_png(&request).is_err());
    assert_eq!(fs::read(&request.output).unwrap(), b"retained output");
}
