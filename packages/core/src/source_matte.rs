//! Explicit local chroma matting of one PNG, preserving its canvas and protecting the source.
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::matting::{alpha_bbox, apply_chroma_key, AlphaBBox, ChromaKeyMode, ChromaParameters};

const MAX_INPUT_BYTES: u64 = 128 * 1024 * 1024;
const MAX_PIXELS: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceMatteRequest {
    pub schema_version: String,
    pub input: PathBuf,
    pub output: PathBuf,
    #[serde(default)]
    pub parameters: ChromaParameters,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMatteReport {
    pub schema_version: &'static str,
    pub algorithm: &'static str,
    pub source_path: PathBuf,
    pub output_path: PathBuf,
    pub source_sha256: String,
    pub output_sha256: String,
    pub width: u32,
    pub height: u32,
    pub source_color_type: &'static str,
    pub output_color_type: &'static str,
    pub parameters: ChromaParameters,
    pub resolved_key_color: String,
    pub transparent_pixels: u64,
    pub partial_alpha_pixels: u64,
    pub visible_pixels: u64,
    pub foreground_bounds: Option<AlphaBBox>,
    pub canvas_preserved: bool,
    pub transparent_rgb_cleared: bool,
    pub provider_request_occurred: bool,
    pub provider_request_count: u32,
    pub visual_review_required: bool,
}

/// Matting changes RGB/alpha only; coordinates and dimensions are never transformed.
/// An existing output is always refused (including links to the input). Output publication
/// uses an atomic no-clobber operation after decoding and all validation succeed.
pub fn matte_png(request: &SourceMatteRequest) -> Result<SourceMatteReport, String> {
    if request.schema_version != "1" {
        return Err("source matte requires schemaVersion 1".into());
    }
    let params = &request.parameters;
    if !params.despill_strength.is_finite()
        || !(0.0..=2.0).contains(&params.despill_strength)
        || params.halo_pixels > 4
    {
        return Err("despillStrength must be finite in 0..=2 and haloPixels must be 0..=4".into());
    }
    if request
        .output
        .extension()
        .and_then(|value| value.to_str())
        .is_none_or(|value| !value.eq_ignore_ascii_case("png"))
    {
        return Err("output must name a new .png file".into());
    }
    if fs::symlink_metadata(&request.output).is_ok() {
        return Err("output already exists; select a new PNG path to protect source files".into());
    }
    let source_path =
        fs::canonicalize(&request.input).map_err(|e| format!("cannot open input: {e}"))?;
    let source = fs::File::open(&source_path).map_err(|e| e.to_string())?;
    if !source.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("input must be a regular PNG file".into());
    }
    let mut bytes = Vec::new();
    source
        .take(MAX_INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("input PNG exceeds the 128 MiB limit".into());
    }
    let mut reader = png::Decoder::new(Cursor::new(&bytes))
        .read_info()
        .map_err(|e| format!("invalid PNG input: {e}"))?;
    let info = reader.info();
    if info.width == 0
        || info.height == 0
        || u64::from(info.width) * u64::from(info.height) > MAX_PIXELS
    {
        return Err("input PNG exceeds the 33,554,432-pixel limit".into());
    }
    if info.bit_depth != png::BitDepth::Eight
        || !matches!(info.color_type, png::ColorType::Rgb | png::ColorType::Rgba)
    {
        return Err("source matte requires 8-bit RGB or RGBA PNG content".into());
    }
    if info.animation_control.is_some() {
        return Err("source matte accepts a single still PNG, not an APNG animation".into());
    }
    let source_color_type = if info.color_type == png::ColorType::Rgb {
        "rgb8"
    } else {
        "rgba8"
    };
    reader
        .finish()
        .map_err(|e| format!("invalid PNG container: {e}"))?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| format!("cannot decode PNG: {e}"))?
        .to_rgba8();
    let resolved_key_color = match params.key_mode {
        ChromaKeyMode::Manual => format!(
            "#{}",
            params
                .manual_key_color
                .trim_start_matches('#')
                .to_ascii_uppercase()
        ),
        ChromaKeyMode::AutoCorners => {
            let corners = [
                (0, 0),
                (image.width() - 1, 0),
                (0, image.height() - 1),
                (image.width() - 1, image.height() - 1),
            ];
            let mut sum = [0_u32; 3];
            for (x, y) in corners {
                for (channel, value) in sum.iter_mut().enumerate() {
                    *value += u32::from(image.get_pixel(x, y)[channel]);
                }
            }
            format!("#{:02X}{:02X}{:02X}", sum[0] / 4, sum[1] / 4, sum[2] / 4)
        }
    };
    let mut output = apply_chroma_key(&image, params).map_err(|e| e.to_string())?;
    let mut transparent_pixels = 0;
    let mut partial_alpha_pixels = 0;
    let mut visible_pixels = 0;
    for pixel in output.pixels_mut() {
        if pixel[3] == 0 {
            *pixel = image::Rgba([0, 0, 0, 0]);
            transparent_pixels += 1;
        } else {
            visible_pixels += 1;
            if pixel[3] < 255 {
                partial_alpha_pixels += 1;
            }
        }
    }
    if visible_pixels == 0 {
        return Err("chroma parameters remove all visible pixels; output was not written".into());
    }
    let foreground_bounds = alpha_bbox(&output);
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(output)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let encoded = encoded.into_inner();
    let parent = request
        .output
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    fs::create_dir_all(parent).map_err(|e| format!("cannot create output directory: {e}"))?;
    let parent = fs::canonicalize(parent).map_err(|e| e.to_string())?;
    let output_path = parent.join(
        request
            .output
            .file_name()
            .ok_or("output requires a filename")?,
    );
    let mut staged = tempfile::NamedTempFile::new_in(&parent).map_err(|e| e.to_string())?;
    staged.write_all(&encoded).map_err(|e| e.to_string())?;
    staged.as_file().sync_all().map_err(|e| e.to_string())?;
    staged
        .persist_noclobber(&output_path)
        .map_err(|e| format!("cannot publish new output PNG: {}", e.error))?;
    Ok(SourceMatteReport {
        schema_version: "1",
        algorithm: "local-chroma@1.0.0",
        source_path,
        output_path,
        source_sha256: format!("{:x}", Sha256::digest(&bytes)),
        output_sha256: format!("{:x}", Sha256::digest(&encoded)),
        width: image.width(),
        height: image.height(),
        source_color_type,
        output_color_type: "rgba8",
        parameters: params.clone(),
        resolved_key_color,
        transparent_pixels,
        partial_alpha_pixels,
        visible_pixels,
        foreground_bounds,
        canvas_preserved: true,
        transparent_rgb_cleared: true,
        provider_request_occurred: false,
        provider_request_count: 0,
        visual_review_required: true,
    })
}
