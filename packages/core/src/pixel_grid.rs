use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use image::{ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::asset_project::StyleLockV1;

pub const PALETTE_LOCK_PROFILE: &str = "palette-lock@1.0.0";
pub const PIXEL_DELIVERY_PROFILE: &str = "pixel-delivery@2.0.0";
pub const PALETTE_LOCK_ALGORITHM: &str = "deterministic-weighted-rgb@1.0.0";

#[derive(Debug, Error)]
pub enum PixelGridError {
    #[error("invalid pixel delivery input: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaletteSourceV1 {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    pub path: PathBuf,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaletteLockColorV1 {
    pub rgb: [u8; 3],
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaletteLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub revision: String,
    pub source: PaletteSourceV1,
    pub max_colors: u16,
    pub algorithm: String,
    pub colors: Vec<PaletteLockColorV1>,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PixelGridV1 {
    pub cell_width: u32,
    pub cell_height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    pub score: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PixelDeliveryReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub grid_detection: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grid: Option<PixelGridV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub alpha_binary: bool,
    pub palette_locked: bool,
    pub palette_lock_revision: String,
    pub palette_lock_sha256: String,
    pub color_count: usize,
    pub max_colors: u16,
    pub input_sha256: String,
    pub output_sha256: String,
}

impl PaletteLockV1 {
    pub fn derive(
        image: &RgbaImage,
        source: PaletteSourceV1,
        max_colors: u16,
    ) -> Result<Self, PixelGridError> {
        if !(2..=256).contains(&max_colors) {
            return Err(PixelGridError::Invalid(
                "maxColors must be between 2 and 256".into(),
            ));
        }
        let mut counts = BTreeMap::<[u8; 3], u64>::new();
        let mut total = 0_u64;
        for pixel in image.pixels() {
            if pixel[3] < 128 {
                continue;
            }
            let key = [pixel[0] / 16, pixel[1] / 16, pixel[2] / 16];
            *counts.entry(key).or_default() += 1;
            total += 1;
        }
        if total == 0 {
            return Err(PixelGridError::Invalid(
                "palette lock requires at least one opaque pixel".into(),
            ));
        }
        let mut entries = counts.into_iter().collect::<Vec<_>>();
        entries.sort_by(|(left_key, left_count), (right_key, right_count)| {
            right_count
                .cmp(left_count)
                .then_with(|| left_key.cmp(right_key))
        });
        let colors = entries
            .into_iter()
            .take(max_colors as usize)
            .map(|(key, count)| PaletteLockColorV1 {
                rgb: [
                    (u32::from(key[0]) * 16 + 8).min(255) as u8,
                    (u32::from(key[1]) * 16 + 8).min(255) as u8,
                    (u32::from(key[2]) * 16 + 8).min(255) as u8,
                ],
                weight: count as f32 / total as f32,
            })
            .collect::<Vec<_>>();
        let revision_payload = serde_json::json!({
            "algorithm": PALETTE_LOCK_ALGORITHM,
            "source": source,
            "maxColors": max_colors,
            "colors": colors,
        });
        let revision = sha256_hex(&serde_json::to_vec(&revision_payload)?)[..16].to_string();
        let mut lock = Self {
            schema_version: "1".into(),
            profile: PALETTE_LOCK_PROFILE.into(),
            revision,
            source,
            max_colors,
            algorithm: PALETTE_LOCK_ALGORITHM.into(),
            colors,
            sha256: "0".repeat(64),
        };
        lock.sha256 = sha256_hex(&serde_json::to_vec(&lock)?);
        Ok(lock)
    }

    pub fn nearest(&self, rgb: [u8; 3]) -> [u8; 3] {
        self.colors
            .iter()
            .map(|color| color.rgb)
            .min_by_key(|candidate| rgb_distance_squared(rgb, *candidate))
            .unwrap_or(rgb)
    }
}

pub fn write_style_palette_lock(
    project_root: &Path,
    style: &StyleLockV1,
    max_colors: u16,
) -> Result<PathBuf, PixelGridError> {
    let board = image::open(&style.board_path)?.to_rgba8();
    let source = PaletteSourceV1 {
        kind: "style_board".into(),
        revision: Some(style.revision.clone()),
        path: style.board_path.clone(),
        sha256: style.board_sha256.clone(),
    };
    let lock = PaletteLockV1::derive(&board, source, max_colors)?;
    let directory = project_root.join(".forge/palette").join(&lock.revision);
    fs::create_dir_all(&directory)?;
    let path = directory.join("palette-lock.json");
    write_json_atomic(&path, &lock)?;
    Ok(path)
}

pub fn detect_pixel_grid(image: &RgbaImage) -> Option<PixelGridV1> {
    if image.width() < 16 || image.height() < 16 {
        return None;
    }
    let mut best: Option<PixelGridV1> = None;
    for cell in 2_u32..=16 {
        for offset_x in 0..cell {
            for offset_y in 0..cell {
                let score = grid_score(image, cell, cell, offset_x, offset_y);
                if score <= 0.0 {
                    continue;
                }
                let candidate = PixelGridV1 {
                    cell_width: cell,
                    cell_height: cell,
                    offset_x,
                    offset_y,
                    score,
                };
                if best
                    .as_ref()
                    .is_none_or(|current| candidate.score > current.score)
                {
                    best = Some(candidate);
                }
            }
        }
    }
    best.filter(|grid| grid.score >= 3.0)
}

pub fn resize_for_delivery(
    source: &RgbaImage,
    width: u32,
    height: u32,
    palette: &PaletteLockV1,
) -> Result<(RgbaImage, PixelDeliveryReportV1), PixelGridError> {
    if width == 0 || height == 0 || source.width() == 0 || source.height() == 0 {
        return Err(PixelGridError::Invalid(
            "pixel delivery resize requires non-zero dimensions".into(),
        ));
    }
    let input_sha256 = rgba_sha256(source);
    let grid = detect_pixel_grid(source);
    let mut output = ImageBuffer::from_pixel(width, height, Rgba([0, 0, 0, 0]));
    if let Some(grid) = grid {
        for (x, y, pixel) in output.enumerate_pixels_mut() {
            let left = x as u64 * source.width() as u64 / width as u64;
            let right = ((x + 1) as u64 * source.width() as u64 / width as u64).max(left + 1);
            let top = y as u64 * source.height() as u64 / height as u64;
            let bottom = ((y + 1) as u64 * source.height() as u64 / height as u64).max(top + 1);
            *pixel = mode_pixel(source, left as u32, top as u32, right as u32, bottom as u32);
        }
        let _ = grid;
    } else {
        for (x, y, pixel) in output.enumerate_pixels_mut() {
            let source_x = ((x as u64 * source.width() as u64 + width as u64 / 2) / width as u64)
                .min(source.width() as u64 - 1) as u32;
            let source_y = ((y as u64 * source.height() as u64 + height as u64 / 2) / height as u64)
                .min(source.height() as u64 - 1) as u32;
            *pixel = *source.get_pixel(source_x, source_y);
        }
    }
    let mut colors = BTreeMap::<[u8; 3], u64>::new();
    for pixel in output.pixels_mut() {
        let alpha = if pixel[3] >= 128 { 255 } else { 0 };
        let rgb = palette.nearest([pixel[0], pixel[1], pixel[2]]);
        *pixel = Rgba([rgb[0], rgb[1], rgb[2], alpha]);
        if alpha > 0 {
            *colors.entry(rgb).or_default() += 1_u64;
        }
    }
    let output_sha256 = rgba_sha256(&output);
    let report = PixelDeliveryReportV1 {
        schema_version: "1".into(),
        profile: PIXEL_DELIVERY_PROFILE.into(),
        grid_detection: if grid.is_some() {
            "detected".into()
        } else {
            "fallback".into()
        },
        grid,
        fallback_reason: grid
            .is_none()
            .then(|| "no stable integer pixel grid detected".into()),
        alpha_binary: output
            .pixels()
            .all(|pixel| pixel[3] == 0 || pixel[3] == 255),
        palette_locked: true,
        palette_lock_revision: palette.revision.clone(),
        palette_lock_sha256: palette.sha256.clone(),
        color_count: colors.len(),
        max_colors: palette.max_colors,
        input_sha256,
        output_sha256,
    };
    Ok((output, report))
}

pub fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), PixelGridError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn grid_score(
    image: &RgbaImage,
    cell_width: u32,
    cell_height: u32,
    offset_x: u32,
    offset_y: u32,
) -> f32 {
    let width = image.width();
    let height = image.height();
    let mut means = Vec::new();
    let mut variances = Vec::new();
    let mut y = offset_y;
    while y < height {
        let mut row = Vec::new();
        let mut x = offset_x;
        let next_y = (y + cell_height).min(height);
        while x < width {
            let end_x = (x + cell_width).min(width);
            let mut counts = BTreeMap::<[u8; 3], u64>::new();
            let mut total = 0_u64;
            for py in y..next_y {
                for px in x..end_x {
                    let pixel = image.get_pixel(px, py);
                    if pixel[3] >= 128 {
                        *counts
                            .entry([pixel[0] / 32, pixel[1] / 32, pixel[2] / 32])
                            .or_default() += 1;
                        total += 1;
                    }
                }
            }
            if total == 0 {
                row.push([u8::MAX; 3]);
                variances.push(0.0);
            } else {
                let (mean, dominant) = counts
                    .into_iter()
                    .max_by(|(left_key, left_count), (right_key, right_count)| {
                        left_count
                            .cmp(right_count)
                            .then_with(|| right_key.cmp(left_key))
                    })
                    .expect("non-empty block");
                row.push(mean);
                variances.push(1.0 - dominant as f32 / total as f32);
            }
            x = end_x;
        }
        means.push(row);
        y = next_y;
    }
    if means.len() < 2 || means.iter().map(Vec::len).max().unwrap_or_default() < 2 {
        return 0.0;
    }
    let unique_means = means
        .iter()
        .flat_map(|row| row.iter().copied())
        .filter(|mean| *mean != [u8::MAX; 3])
        .collect::<std::collections::BTreeSet<_>>();
    if unique_means.len() > 32 {
        return 0.0;
    }
    let mut boundary = 0.0;
    let mut boundary_count = 0_u64;
    for row in 0..means.len() {
        for col in 0..means[row].len() {
            if means[row][col] == [u8::MAX; 3] {
                continue;
            }
            if col + 1 < means[row].len() && means[row][col + 1] != [u8::MAX; 3] {
                boundary += quantized_distance(means[row][col], means[row][col + 1]);
                boundary_count += 1;
            }
            if row + 1 < means.len()
                && col < means[row + 1].len()
                && means[row + 1][col] != [u8::MAX; 3]
            {
                boundary += quantized_distance(means[row][col], means[row + 1][col]);
                boundary_count += 1;
            }
        }
    }
    if boundary_count == 0 {
        return 0.0;
    }
    let boundary_mean = boundary / boundary_count as f32;
    let variance_mean = variances.iter().sum::<f32>() / variances.len().max(1) as f32;
    if variance_mean > 0.25 {
        return 0.0;
    }
    boundary_mean / (variance_mean + 0.05)
}

fn quantized_distance(left: [u8; 3], right: [u8; 3]) -> f32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left.abs_diff(right) as f32)
        .sum::<f32>()
}

fn mode_pixel(source: &RgbaImage, left: u32, top: u32, right: u32, bottom: u32) -> Rgba<u8> {
    let mut colors = BTreeMap::<[u8; 3], u64>::new();
    let mut opaque = 0_u64;
    let mut total = 0_u64;
    for y in top..bottom.min(source.height()) {
        for x in left..right.min(source.width()) {
            let pixel = source.get_pixel(x, y);
            total += 1;
            if pixel[3] >= 128 {
                opaque += 1;
                *colors
                    .entry([pixel[0] / 16, pixel[1] / 16, pixel[2] / 16])
                    .or_default() += 1;
            }
        }
    }
    if opaque * 2 < total {
        return Rgba([0, 0, 0, 0]);
    }
    let rgb = colors
        .into_iter()
        .max_by(|(left_key, left_count), (right_key, right_count)| {
            left_count
                .cmp(right_count)
                .then_with(|| right_key.cmp(left_key))
        })
        .map(|(key, _)| {
            [
                (u32::from(key[0]) * 16 + 8).min(255) as u8,
                (u32::from(key[1]) * 16 + 8).min(255) as u8,
                (u32::from(key[2]) * 16 + 8).min(255) as u8,
            ]
        })
        .unwrap_or([0, 0, 0]);
    Rgba([rgb[0], rgb[1], rgb[2], 255])
}

fn rgb_distance_squared(left: [u8; 3], right: [u8; 3]) -> u32 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left.abs_diff(right) as u32)
        .map(|delta| delta * delta)
        .sum()
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn rgba_sha256(image: &RgbaImage) -> String {
    let mut hasher = Sha256::new();
    hasher.update(image.width().to_le_bytes());
    hasher.update(image.height().to_le_bytes());
    hasher.update(image.as_raw());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette(colors: &[[u8; 3]]) -> PaletteLockV1 {
        PaletteLockV1 {
            schema_version: "1".into(),
            profile: PALETTE_LOCK_PROFILE.into(),
            revision: "0123456789abcdef".into(),
            source: PaletteSourceV1 {
                kind: "canonical_idle".into(),
                revision: None,
                path: PathBuf::from("source.png"),
                sha256: "a".repeat(64),
            },
            max_colors: colors.len() as u16,
            algorithm: PALETTE_LOCK_ALGORITHM.into(),
            colors: colors
                .iter()
                .enumerate()
                .map(|(index, rgb)| PaletteLockColorV1 {
                    rgb: *rgb,
                    weight: 1.0 / (index + 2) as f32,
                })
                .collect(),
            sha256: "b".repeat(64),
        }
    }

    #[test]
    fn palette_derivation_is_deterministic_and_weight_ordered() {
        let mut image = RgbaImage::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                let pixel = if x < 3 {
                    Rgba([10, 20, 30, 255])
                } else {
                    Rgba([200, 210, 220, 255])
                };
                image.put_pixel(x, y, pixel);
            }
        }
        let source = PaletteSourceV1 {
            kind: "canonical_idle".into(),
            revision: None,
            path: PathBuf::from("canonical.png"),
            sha256: "c".repeat(64),
        };
        let first = PaletteLockV1::derive(&image, source.clone(), 4).unwrap();
        let second = PaletteLockV1::derive(&image, source, 4).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.colors.len(), 2);
        assert_eq!(first.colors[0].rgb, [8, 24, 24]);
        assert!(first.colors[0].weight > first.colors[1].weight);
    }

    #[test]
    fn delivery_output_has_binary_alpha_and_palette_bound_colors() {
        let mut source = RgbaImage::new(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                let color = if x < 4 { [7, 9, 11] } else { [240, 241, 242] };
                source.put_pixel(x, y, Rgba([color[0], color[1], color[2], 200]));
            }
        }
        let lock = palette(&[[0, 0, 0], [255, 255, 255]]);
        let (output, report) = resize_for_delivery(&source, 4, 4, &lock).unwrap();
        assert!(output
            .pixels()
            .all(|pixel| pixel[3] == 0 || pixel[3] == 255));
        assert!(report.alpha_binary);
        assert!(report.color_count <= 2);
        assert!(output.pixels().all(|pixel| {
            let rgb = [pixel[0], pixel[1], pixel[2]];
            rgb == [0, 0, 0] || rgb == [255, 255, 255]
        }));
    }

    #[test]
    fn missing_grid_falls_back_with_an_explicit_report_reason() {
        let mut source = RgbaImage::new(32, 32);
        for (x, y, pixel) in source.enumerate_pixels_mut() {
            *pixel = Rgba([
                (x as u8).wrapping_mul(17),
                (y as u8).wrapping_mul(29),
                ((x + y) as u8).wrapping_mul(11),
                255,
            ]);
        }
        let lock = palette(&[[0, 0, 0], [128, 128, 128], [255, 255, 255]]);
        let (_output, report) = resize_for_delivery(&source, 16, 16, &lock).unwrap();
        assert_eq!(report.grid_detection, "fallback");
        assert!(report.grid.is_none());
        assert!(report
            .fallback_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("no stable integer pixel grid")));
    }

    #[test]
    fn delivery_is_byte_deterministic_for_the_same_input() {
        let mut source = RgbaImage::new(16, 16);
        for y in 0..16 {
            for x in 0..16 {
                let color = if (x / 4 + y / 4) % 2 == 0 {
                    [30, 90, 40]
                } else {
                    [220, 180, 70]
                };
                source.put_pixel(x, y, Rgba([color[0], color[1], color[2], 255]));
            }
        }
        let source_lock = PaletteLockV1::derive(
            &source,
            PaletteSourceV1 {
                kind: "canonical_idle".into(),
                revision: None,
                path: PathBuf::from("canonical.png"),
                sha256: "d".repeat(64),
            },
            8,
        )
        .unwrap();
        let (first, first_report) = resize_for_delivery(&source, 8, 8, &source_lock).unwrap();
        let (second, second_report) = resize_for_delivery(&source, 8, 8, &source_lock).unwrap();
        assert_eq!(first, second);
        assert_eq!(first_report.output_sha256, second_report.output_sha256);
    }

    #[test]
    fn style_palette_lock_writes_an_immutable_sidecar() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("project");
        let board_path = temp.path().join("style-board.png");
        let mut board = RgbaImage::new(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                board.put_pixel(x, y, Rgba([20 + x as u8, 30 + y as u8, 40, 255]));
            }
        }
        board.save(&board_path).unwrap();
        let style = StyleLockV1 {
            schema_version: "1".into(),
            revision: "style-test-000001".into(),
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            image_model: Some("fixture-image".into()),
            prompt: "style".into(),
            perspective: "topdown".into(),
            lighting: "soft".into(),
            outline: "clean".into(),
            background: "transparent".into(),
            sampling: crate::asset_project::SamplingMode::Nearest,
            character_canvas_size: 256,
            icon_canvas_size: 128,
            prop_canvas_size: 256,
            board_path: board_path.clone(),
            board_sha256: "a".repeat(64),
            reference_sha256: Vec::new(),
            baseline_profile: "style-baseline@test".into(),
            migrated_from_revision: None,
            baseline: crate::asset_project::StyleBaseline {
                palette: Vec::new(),
                edge_density: 0.0,
                foreground_scale: 0.0,
                perceptual_hash: "0".into(),
            },
        };
        let path = write_style_palette_lock(&project, &style, 8).unwrap();
        assert!(path.starts_with(project.join(".forge/palette")));
        let lock: PaletteLockV1 = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(lock.source.kind, "style_board");
        assert_eq!(lock.source.revision.as_deref(), Some("style-test-000001"));
        assert_eq!(lock.source.sha256, "a".repeat(64));
        assert!(!lock.colors.is_empty());
    }
}
