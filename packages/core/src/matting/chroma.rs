use image::{ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub type MattingResult<T> = Result<T, MattingError>;

#[derive(Debug, Error)]
pub enum MattingError {
    #[error("invalid manual key color `{0}`; expected #RRGGBB")]
    InvalidManualKeyColor(String),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChromaKeyMode {
    AutoCorners,
    Manual,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromaParameters {
    pub key_mode: ChromaKeyMode,
    pub manual_key_color: String,
    pub threshold: u8,
    pub softness: u8,
    pub despill_strength: f32,
    pub halo_pixels: u8,
}

impl Default for ChromaParameters {
    fn default() -> Self {
        Self {
            key_mode: ChromaKeyMode::AutoCorners,
            manual_key_color: "#00FF00".to_string(),
            threshold: 48,
            softness: 18,
            despill_strength: 0.5,
            halo_pixels: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlphaBBox {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
    pub width: u32,
    pub height: u32,
    pub center_x: f32,
    pub center_y: f32,
    pub bottom_y: u32,
    pub alpha_coverage: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameBBox {
    pub frame: String,
    pub width: u32,
    pub height: u32,
    pub bbox: Option<AlphaBBox>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchMattingResult {
    pub processed_dir: PathBuf,
    pub frames: Vec<FrameBBox>,
}

pub fn apply_chroma_key(
    image: &RgbaImage,
    parameters: &ChromaParameters,
) -> MattingResult<RgbaImage> {
    let params = sanitized_parameters(parameters);
    let key = key_color(image, &params)?;
    let keyed = ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
        let pixel = image.get_pixel(x, y);
        matte_pixel(*pixel, key, &params)
    });

    if params.halo_pixels == 0 {
        Ok(keyed)
    } else {
        Ok(apply_halo_cleanup(&keyed, params.halo_pixels.min(4)))
    }
}

pub fn process_chroma_frame(
    raw_frame_path: impl AsRef<Path>,
    processed_frame_path: impl AsRef<Path>,
    parameters: &ChromaParameters,
) -> MattingResult<FrameBBox> {
    let raw_frame_path = raw_frame_path.as_ref();
    let processed_frame_path = processed_frame_path.as_ref();
    let raw = image::open(raw_frame_path)?.to_rgba8();
    let processed = apply_chroma_key(&raw, parameters)?;

    if let Some(parent) = processed_frame_path.parent() {
        fs::create_dir_all(parent)?;
    }
    processed.save(processed_frame_path)?;

    Ok(FrameBBox {
        frame: processed_frame_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("frame.png")
            .to_string(),
        width: processed.width(),
        height: processed.height(),
        bbox: alpha_bbox(&processed),
    })
}

pub fn process_chroma_batch(
    raw_frame_paths: &[PathBuf],
    processed_dir: impl AsRef<Path>,
    parameters: &ChromaParameters,
) -> MattingResult<BatchMattingResult> {
    let processed_dir = processed_dir.as_ref();
    fs::create_dir_all(processed_dir)?;

    let mut frames = Vec::with_capacity(raw_frame_paths.len());
    for (index, raw_frame_path) in raw_frame_paths.iter().enumerate() {
        let file_name = format!("frame_{:05}.png", index + 1);
        let processed_frame_path = processed_dir.join(&file_name);
        frames.push(process_chroma_frame(
            raw_frame_path,
            &processed_frame_path,
            parameters,
        )?);
    }

    let bboxes_path = processed_dir.join("bboxes.json");
    fs::write(&bboxes_path, serde_json::to_vec_pretty(&frames)?)?;

    Ok(BatchMattingResult {
        processed_dir: processed_dir.to_path_buf(),
        frames,
    })
}

pub fn alpha_bbox(image: &RgbaImage) -> Option<AlphaBBox> {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut opaque_pixels = 0u64;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 0 {
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
            opaque_pixels += 1;
        }
    }

    if opaque_pixels == 0 {
        return None;
    }

    let width = right - left + 1;
    let height = bottom - top + 1;
    let total_pixels = (image.width() as u64).saturating_mul(image.height() as u64);
    let alpha_coverage = if total_pixels == 0 {
        0.0
    } else {
        opaque_pixels as f32 / total_pixels as f32
    };

    Some(AlphaBBox {
        left,
        top,
        right,
        bottom,
        width,
        height,
        center_x: left as f32 + width as f32 / 2.0,
        center_y: top as f32 + height as f32 / 2.0,
        bottom_y: bottom,
        alpha_coverage,
    })
}

fn sanitized_parameters(parameters: &ChromaParameters) -> ChromaParameters {
    ChromaParameters {
        key_mode: parameters.key_mode.clone(),
        manual_key_color: parameters.manual_key_color.clone(),
        threshold: parameters.threshold,
        softness: parameters.softness,
        despill_strength: parameters.despill_strength.clamp(0.0, 2.0),
        halo_pixels: parameters.halo_pixels.min(4),
    }
}

fn key_color(image: &RgbaImage, parameters: &ChromaParameters) -> MattingResult<[u8; 3]> {
    match &parameters.key_mode {
        ChromaKeyMode::Manual => parse_hex_color(&parameters.manual_key_color),
        ChromaKeyMode::AutoCorners => Ok(sample_corner_key(image)),
    }
}

fn parse_hex_color(value: &str) -> MattingResult<[u8; 3]> {
    let hex = value.strip_prefix('#').unwrap_or(value);
    if hex.len() != 6 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(MattingError::InvalidManualKeyColor(value.to_string()));
    }

    let red = u8::from_str_radix(&hex[0..2], 16)
        .map_err(|_| MattingError::InvalidManualKeyColor(value.to_string()))?;
    let green = u8::from_str_radix(&hex[2..4], 16)
        .map_err(|_| MattingError::InvalidManualKeyColor(value.to_string()))?;
    let blue = u8::from_str_radix(&hex[4..6], 16)
        .map_err(|_| MattingError::InvalidManualKeyColor(value.to_string()))?;

    Ok([red, green, blue])
}

fn sample_corner_key(image: &RgbaImage) -> [u8; 3] {
    if image.width() == 0 || image.height() == 0 {
        return [0, 255, 0];
    }

    let max_x = image.width() - 1;
    let max_y = image.height() - 1;
    let corners = [
        image.get_pixel(0, 0),
        image.get_pixel(max_x, 0),
        image.get_pixel(0, max_y),
        image.get_pixel(max_x, max_y),
    ];

    let [red, green, blue] = corners.iter().fold([0u32; 3], |mut sum, pixel| {
        sum[0] += pixel[0] as u32;
        sum[1] += pixel[1] as u32;
        sum[2] += pixel[2] as u32;
        sum
    });

    [(red / 4) as u8, (green / 4) as u8, (blue / 4) as u8]
}

fn matte_pixel(pixel: Rgba<u8>, key: [u8; 3], parameters: &ChromaParameters) -> Rgba<u8> {
    let distance = color_distance([pixel[0], pixel[1], pixel[2]], key);
    let threshold = parameters.threshold as f32;
    let softness = parameters.softness as f32;
    let alpha_factor = if distance <= threshold {
        0.0
    } else if softness <= 0.0 {
        1.0
    } else {
        ((distance - threshold) / softness).clamp(0.0, 1.0)
    };

    let alpha = (pixel[3] as f32 * alpha_factor).round().clamp(0.0, 255.0) as u8;
    let mut output = Rgba([pixel[0], pixel[1], pixel[2], alpha]);

    if alpha > 0 && parameters.despill_strength > 0.0 {
        despill(&mut output, key, parameters.despill_strength, alpha_factor);
    }

    output
}

fn color_distance(color: [u8; 3], key: [u8; 3]) -> f32 {
    let red = color[0] as f32 - key[0] as f32;
    let green = color[1] as f32 - key[1] as f32;
    let blue = color[2] as f32 - key[2] as f32;
    (red * red + green * green + blue * blue).sqrt()
}

fn despill(pixel: &mut Rgba<u8>, key: [u8; 3], strength: f32, alpha_factor: f32) {
    let dominant_channel = if key[1] >= key[0] && key[1] >= key[2] {
        1
    } else if key[0] >= key[2] {
        0
    } else {
        2
    };

    let spill = pixel[dominant_channel] as f32;
    let other_max = (0..3)
        .filter(|channel| *channel != dominant_channel)
        .map(|channel| pixel[channel] as f32)
        .fold(0.0, f32::max);
    let reduction = (spill - other_max).max(0.0) * strength * (1.0 - alpha_factor * 0.5);
    pixel[dominant_channel] = (spill - reduction).round().clamp(0.0, 255.0) as u8;
}

fn apply_halo_cleanup(image: &RgbaImage, radius: u8) -> RgbaImage {
    let radius = radius as i32;
    ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
        let mut pixel = *image.get_pixel(x, y);
        if pixel[3] == 0 {
            return pixel;
        }

        let mut min_alpha = pixel[3];
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= image.width() as i32 || ny >= image.height() as i32 {
                    continue;
                }
                min_alpha = min_alpha.min(image.get_pixel(nx as u32, ny as u32)[3]);
            }
        }
        pixel[3] = min_alpha;
        pixel
    })
}
