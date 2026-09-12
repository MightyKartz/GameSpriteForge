//! Measured local PNG facts. These diagnostics do not approve artwork.
use std::fs;
use std::path::Path;

use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn inspect_png(
    path: &Path,
    grid: Option<(u32, u32)>,
    preview_dir: Option<&Path>,
) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() < 29 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err("source inspect requires a valid PNG".into());
    }
    let decoded = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let rgba = decoded.to_rgba8();
    let (width, height) = rgba.dimensions();
    let thresholds = [1, 8, 32, 128, 240];
    let bounds = thresholds
        .into_iter()
        .map(|threshold| json!({"alphaThreshold":threshold,"bounds":bounds(&rgba,threshold)}))
        .collect::<Vec<_>>();
    let transparent = rgba.pixels().filter(|p| p[3] == 0).count();
    let partial = rgba.pixels().filter(|p| p[3] > 0 && p[3] < 255).count();
    let hidden_rgb = rgba
        .pixels()
        .filter(|p| p[3] == 0 && p.0[..3].iter().any(|v| *v != 0))
        .count();
    let grid_report = if let Some((fw, fh)) = grid {
        if fw == 0 || fh == 0 {
            return Err("frame dimensions must be positive".into());
        }
        let divisible = width % fw == 0 && height % fh == 0;
        let mut cells = Vec::new();
        if divisible {
            for row in 0..height / fh {
                for column in 0..width / fw {
                    let touches = |threshold| {
                        (0..fw).any(|x| {
                            rgba.get_pixel(column * fw + x, row * fh)[3] >= threshold
                                || rgba.get_pixel(column * fw + x, row * fh + fh - 1)[3]
                                    >= threshold
                        }) || (0..fh).any(|y| {
                            rgba.get_pixel(column * fw, row * fh + y)[3] >= threshold
                                || rgba.get_pixel(column * fw + fw - 1, row * fh + y)[3]
                                    >= threshold
                        })
                    };
                    cells.push(json!({"column":column,"row":row,"touchesEdgeAlpha1":touches(1),"touchesEdgeAlpha32":touches(32)}));
                }
            }
        }
        json!({"frameWidth":fw,"frameHeight":fh,"divisible":divisible,"remainderX":width%fw,"remainderY":height%fh,"cells":cells})
    } else {
        Value::Null
    };
    let mut previews = Vec::new();
    if let Some(dir) = preview_dir {
        // Exclusive creation keeps a diagnostic run from replacing prior work.
        fs::create_dir(dir).map_err(|e| format!("preview directory must be new: {e}"))?;
        for (name, background) in [
            ("on-light.png", [238u8, 238, 238]),
            ("on-dark.png", [24u8, 24, 24]),
        ] {
            let mut composite = rgba.clone();
            for pixel in composite.pixels_mut() {
                let alpha = u32::from(pixel[3]);
                for channel in 0..3 {
                    pixel[channel] = ((u32::from(pixel[channel]) * alpha
                        + u32::from(background[channel]) * (255 - alpha)
                        + 127)
                        / 255) as u8;
                }
                pixel[3] = 255;
            }
            let output = dir.join(name);
            composite.save(&output).map_err(|e| e.to_string())?;
            previews.push(output);
        }
    }
    Ok(
        json!({"schemaVersion":"1","path":path,"sha256":format!("{:x}",Sha256::digest(&bytes)),
        "width":width,"height":height,"pngBitDepth":bytes[24],"pngColorType":bytes[25],
        "decodedColor":format!("{:?}",decoded.color()),"hasAlphaChannel":decoded.color().has_alpha(),
        "analysisColor":"Rgba8","alphaThresholdOperator":">=",
        "alpha":{"transparentPixels":transparent,"partialPixels":partial,"opaquePixels":(u64::from(width)*u64::from(height))-(transparent+partial) as u64,"transparentPixelsWithRgb":hidden_rgb},
        "bounds":bounds,"grid":grid_report,"previews":previews,"visualReview":"required",
        "notes":["Measurements are decoded pixels, not prompt dimensions.","Alpha counts, bounds and previews use decoded 8-bit RGBA; 16-bit sources are reduced for this diagnostic.","RGB under zero alpha can be legitimate; it is not evidence of a visible halo.","Grid edge contact is a review cue, not proof of a split body. Baked checkerboards and artistic quality need visual review."]}),
    )
}

fn bounds(image: &RgbaImage, threshold: u8) -> Value {
    let (mut left, mut top, mut right, mut bottom) = (image.width(), image.height(), 0, 0);
    let mut found = false;
    for (x, y, Rgba(pixel)) in image.enumerate_pixels() {
        if pixel[3] >= threshold {
            found = true;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    if found {
        json!({"x":left,"y":top,"width":right-left+1,"height":bottom-top+1})
    } else {
        Value::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn measures_alpha_grid_and_exclusive_previews_without_changing_source() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("source.png");
        let mut image = RgbaImage::from_pixel(7, 4, Rgba([99, 12, 1, 0]));
        image.put_pixel(3, 2, Rgba([255, 100, 0, 8]));
        image.save(&path).unwrap();
        let before = fs::read(&path).unwrap();
        let preview = root.path().join("preview");
        let report = inspect_png(&path, Some((4, 4)), Some(&preview)).unwrap();
        assert_eq!(report["width"], 7);
        assert_eq!(report["grid"]["divisible"], false);
        assert_eq!(report["alpha"]["transparentPixelsWithRgb"], 27);
        assert_eq!(report["bounds"][0]["bounds"]["x"], 3);
        assert!(report["bounds"][2]["bounds"].is_null());
        assert!(inspect_png(&path, None, Some(&preview)).is_err());
        assert_eq!(before, fs::read(path).unwrap());
    }
}
