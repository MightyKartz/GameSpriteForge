//! Deterministic validation and extraction for Provider-generated animation sheets.
//!
//! `topdown-spritesheet@3.0.0` deliberately keeps the Provider contract small:
//! one square 2x2 image contains four ordered phases for exactly one action.
//! Forge, rather than the image model, owns cell order, slicing and downstream
//! animation semantics.

use std::fs;
use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::character_direction::DirectionViewV1;
use crate::frames::bbox_from_image;
use crate::video::{slice_sprite_sheet_grid, SliceSpriteSheetParams};

pub const ANIMATION_SHEET_PROFILE_V1: &str = "animation-sheet@1.0.0";
pub const DIRECTION_SHEET_PROFILE_V1: &str = "direction-sheet@1.0.0";
pub const ONION_SKIN_ALIGNMENT_PROFILE_V1: &str = "onion-skin@1.0.0";

#[derive(Debug, Error)]
pub enum AnimationSheetError {
    #[error("animation sheet I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("animation sheet image is invalid: {0}")]
    Image(#[from] image::ImageError),
    #[error("animation sheet contract failed: {0}")]
    Contract(String),
    #[error("animation sheet extraction failed: {0}")]
    Extraction(String),
    #[cfg(feature = "pixel-delivery-v2")]
    #[error("animation sheet pixel delivery failed: {0}")]
    PixelGrid(#[from] crate::pixel_grid::PixelGridError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSheetCellV1 {
    pub frame_index: u32,
    pub row: u32,
    pub column: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub foreground_ratio: f32,
    pub touches_cell_boundary: bool,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationSheetReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub attempt: u32,
    pub sheet_path: PathBuf,
    pub sheet_sha256: String,
    pub width: u32,
    pub height: u32,
    pub columns: u32,
    pub rows: u32,
    pub cell_width: u32,
    pub cell_height: u32,
    pub cells: Vec<AnimationSheetCellV1>,
    pub verdict: AnimationSheetVerdictV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationSheetVerdictV1 {
    GameReady,
    Regenerate,
    Blocked,
}

impl AnimationSheetReportV1 {
    pub fn is_game_ready(&self) -> bool {
        self.verdict == AnimationSheetVerdictV1::GameReady
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSheetExtractionV1 {
    pub report: AnimationSheetReportV1,
    pub frames: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirectionSheetExtractionV1 {
    pub report: AnimationSheetReportV1,
    pub directions: Vec<(DirectionViewV1, PathBuf)>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnionSkinFrameAlignmentV1 {
    pub frame_index: u32,
    pub source_body_height: f32,
    pub source_scale_ratio: f32,
    pub shared_scale: f32,
    pub offset_x: i32,
    pub offset_y: i32,
    pub normalized_center_drift_px: f32,
    pub normalized_foot_drift_px: f32,
    pub content_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnionSkinAlignmentReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub canvas_size: u32,
    pub target_body_height: f32,
    pub shared_scale: f32,
    pub max_source_scale_drift: f32,
    pub max_normalized_center_drift_px: f32,
    pub max_normalized_foot_drift_px: f32,
    pub frames: Vec<OnionSkinFrameAlignmentV1>,
    pub verdict: AnimationSheetVerdictV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedSheetAlignmentV1 {
    pub report: OnionSkinAlignmentReportV1,
    pub paths: Vec<PathBuf>,
    pub images: Vec<RgbaImage>,
    pub onion_skin_paths: Vec<PathBuf>,
}

/// Convert one repaired cell to the same body-height and foot-anchor contract
/// used by the four-frame sheet. This is reserved for targeted frame retries;
/// the initial sheet always uses a single scale computed across all cells.
pub fn normalize_repaired_frame_to_contract(
    frame: &RgbaImage,
    canvas_size: u32,
    _report_path: Option<&Path>,
) -> Result<RgbaImage, AnimationSheetError> {
    let bbox = bbox_from_image(frame, 12);
    if !bbox.has_foreground() {
        return Err(AnimationSheetError::Contract(
            "targeted frame retry contains no foreground".into(),
        ));
    }
    let target_body_height = canvas_size as f32 * 0.72;
    let scale = target_body_height / bbox.height.max(1.0);
    let scaled_width = ((frame.width() as f32 * scale).round() as u32).max(1);
    let scaled_height = ((frame.height() as f32 * scale).round() as u32).max(1);
    // A targeted retry is inserted into three immutable source frames. Keep
    // its resampling contract identical to the original shared sheet even
    // when pixel-delivery-v2 is compiled in; applying a new binary-alpha /
    // palette transform to only one cell creates an artificial temporal
    // silhouette discontinuity. Full newly generated deliveries still use
    // the versioned pixel-delivery path at their normal export boundary.
    let scaled = image::imageops::resize(
        frame,
        scaled_width,
        scaled_height,
        image::imageops::FilterType::Lanczos3,
    );
    let scaled_bbox = bbox_from_image(&scaled, 12);
    let offset_x = (canvas_size as f32 * 0.5 - scaled_bbox.center_x).round() as i32;
    let offset_y = (canvas_size as f32 * 0.90 - scaled_bbox.bottom_y).round() as i32;
    let mut output = RgbaImage::from_pixel(canvas_size, canvas_size, Rgba([0, 0, 0, 0]));
    for (x, y, pixel) in scaled.enumerate_pixels() {
        let tx = x as i32 + offset_x;
        let ty = y as i32 + offset_y;
        if tx >= 0 && ty >= 0 && tx < canvas_size as i32 && ty < canvas_size as i32 {
            output.put_pixel(tx as u32, ty as u32, *pixel);
        } else if pixel[3] > 12 {
            return Err(AnimationSheetError::Contract(
                "targeted frame retry would be cropped by the shared canvas".into(),
            ));
        }
    }
    Ok(output)
}

/// Inspect a fixed 2x2 sheet and extract its four cells in row-major order.
///
/// The layout gate is intentionally limited to structural failures that a
/// person cannot safely waive: malformed dimensions, empty cells and content
/// crossing a cell boundary. Identity, equipment and motion are assessed by
/// the existing Character quality pipeline after matting and alignment.
pub fn inspect_and_extract_animation_sheet(
    sheet_path: &Path,
    output_directory: &Path,
    animation: &str,
    attempt: u32,
) -> Result<AnimationSheetExtractionV1, AnimationSheetError> {
    let sheet = image::open(sheet_path)?.to_rgba8();
    let width = sheet.width();
    let height = sheet.height();
    if width < 128 || height < 128 {
        return Err(AnimationSheetError::Contract(
            "sheet must be at least 128x128 pixels".into(),
        ));
    }
    if width != height || width % 2 != 0 || height % 2 != 0 {
        return Err(AnimationSheetError::Contract(
            "sheet must be square with even dimensions for a 2x2 grid".into(),
        ));
    }

    let cell_width = width / 2;
    let cell_height = height / 2;
    let mut cells = Vec::with_capacity(4);
    let mut reasons = Vec::new();
    for row in 0..2 {
        for column in 0..2 {
            let frame_index = row * 2 + column;
            let x = column * cell_width;
            let y = row * cell_height;
            let cell = image::imageops::crop_imm(&sheet, x, y, cell_width, cell_height).to_image();
            let metrics = inspect_cell(&cell);
            if metrics.foreground_ratio < 0.01 {
                reasons.push(format!("frame {frame_index} contains no usable foreground"));
            }
            if metrics.touches_cell_boundary {
                reasons.push(format!(
                    "frame {frame_index} foreground crosses or touches its cell boundary"
                ));
            }
            cells.push(AnimationSheetCellV1 {
                frame_index,
                row,
                column,
                x,
                y,
                width: cell_width,
                height: cell_height,
                foreground_ratio: metrics.foreground_ratio,
                touches_cell_boundary: metrics.touches_cell_boundary,
                content_sha256: hash_rgba(&cell),
            });
        }
    }
    let verdict = if reasons.is_empty() {
        AnimationSheetVerdictV1::GameReady
    } else {
        AnimationSheetVerdictV1::Regenerate
    };
    let report = AnimationSheetReportV1 {
        schema_version: "1".into(),
        profile: ANIMATION_SHEET_PROFILE_V1.into(),
        animation: animation.into(),
        attempt,
        sheet_path: sheet_path.to_path_buf(),
        sheet_sha256: hash_file(sheet_path)?,
        width,
        height,
        columns: 2,
        rows: 2,
        cell_width,
        cell_height,
        cells,
        verdict,
        reasons,
    };

    fs::create_dir_all(output_directory)?;
    let extracted = slice_sprite_sheet_grid(&SliceSpriteSheetParams {
        sheet_path: sheet_path.to_path_buf(),
        output_directory: output_directory.to_path_buf(),
        frame_width: cell_width,
        frame_height: cell_height,
        columns: 2,
        rows: 2,
    })
    .map_err(|error| AnimationSheetError::Extraction(error.to_string()))?;
    Ok(AnimationSheetExtractionV1 {
        report,
        frames: extracted.frames,
    })
}

/// Extract the canonical four-view sheet in a fixed, model-independent order:
/// front, rear, right and left. The directions are data owned by Forge; the
/// Provider never gets to choose or reorder them.
pub fn inspect_and_extract_direction_sheet(
    sheet_path: &Path,
    output_directory: &Path,
    attempt: u32,
) -> Result<DirectionSheetExtractionV1, AnimationSheetError> {
    let extraction = inspect_and_extract_animation_sheet(
        sheet_path,
        output_directory,
        "direction_lock",
        attempt,
    )?;
    let directions = [
        DirectionViewV1::Front,
        DirectionViewV1::Rear,
        DirectionViewV1::Right,
        DirectionViewV1::Left,
    ]
    .into_iter()
    .zip(extraction.frames)
    .collect();
    Ok(DirectionSheetExtractionV1 {
        report: extraction.report,
        directions,
    })
}

/// Normalize all sheet cells with one shared scale, then apply translation-only
/// foot-anchor alignment. This preserves real size variation for quality gates
/// instead of independently resizing every frame until drift becomes invisible.
pub fn align_sheet_frames_with_onion_skin(
    frames: &[RgbaImage],
    canvas_size: u32,
    output_directory: &Path,
) -> Result<SharedSheetAlignmentV1, AnimationSheetError> {
    if frames.len() != 4 {
        return Err(AnimationSheetError::Contract(
            "shared sheet alignment requires exactly four frames".into(),
        ));
    }
    if canvas_size < 64 {
        return Err(AnimationSheetError::Contract(
            "shared sheet alignment canvas must be at least 64 pixels".into(),
        ));
    }
    let source_bboxes = frames
        .iter()
        .map(|frame| bbox_from_image(frame, 12))
        .collect::<Vec<_>>();
    if source_bboxes.iter().any(|bbox| !bbox.has_foreground()) {
        return Err(AnimationSheetError::Contract(
            "shared sheet alignment found an empty foreground".into(),
        ));
    }
    let mut heights = source_bboxes
        .iter()
        .map(|bbox| bbox.height)
        .collect::<Vec<_>>();
    heights.sort_by(f32::total_cmp);
    let median_height = (heights[1] + heights[2]) * 0.5;
    let target_body_height = canvas_size as f32 * 0.72;
    let shared_scale = target_body_height / median_height.max(1.0);
    let target_center_x = canvas_size as f32 * 0.5;
    let target_foot_y = canvas_size as f32 * 0.90;
    let max_source_scale_drift = source_bboxes
        .iter()
        .map(|bbox| (bbox.height / median_height.max(1.0) - 1.0).abs())
        .fold(0.0_f32, f32::max);

    fs::create_dir_all(output_directory)?;
    let onion_directory = output_directory.join("onion-skin");
    fs::create_dir_all(&onion_directory)?;
    let mut normalized = Vec::with_capacity(4);
    let mut paths = Vec::with_capacity(4);
    let mut frame_reports = Vec::with_capacity(4);
    let mut reasons = Vec::new();
    for (frame_index, (frame, source_bbox)) in frames.iter().zip(source_bboxes.iter()).enumerate() {
        let scaled_width = ((frame.width() as f32 * shared_scale).round() as u32).max(1);
        let scaled_height = ((frame.height() as f32 * shared_scale).round() as u32).max(1);
        let scaled = image::imageops::resize(
            frame,
            scaled_width,
            scaled_height,
            image::imageops::FilterType::Lanczos3,
        );
        let scaled_bbox = bbox_from_image(&scaled, 12);
        let offset_x = (target_center_x - scaled_bbox.center_x).round() as i32;
        let offset_y = (target_foot_y - scaled_bbox.bottom_y).round() as i32;
        let mut canvas = RgbaImage::from_pixel(canvas_size, canvas_size, Rgba([0, 0, 0, 0]));
        let mut cropped = false;
        for (x, y, pixel) in scaled.enumerate_pixels() {
            let tx = x as i32 + offset_x;
            let ty = y as i32 + offset_y;
            if tx >= 0 && ty >= 0 && tx < canvas_size as i32 && ty < canvas_size as i32 {
                canvas.put_pixel(tx as u32, ty as u32, *pixel);
            } else if pixel[3] > 12 {
                cropped = true;
            }
        }
        if cropped {
            reasons.push(format!(
                "frame {frame_index} would be cropped by the shared canvas"
            ));
        }
        let bbox = bbox_from_image(&canvas, 12);
        let center_drift = (bbox.center_x - target_center_x).abs();
        let foot_drift = (bbox.bottom_y - target_foot_y).abs();
        let path = output_directory.join(format!("frame-{frame_index:02}.png"));
        canvas.save(&path)?;
        frame_reports.push(OnionSkinFrameAlignmentV1 {
            frame_index: frame_index as u32,
            source_body_height: source_bbox.height,
            source_scale_ratio: source_bbox.height / median_height.max(1.0),
            shared_scale,
            offset_x,
            offset_y,
            normalized_center_drift_px: center_drift,
            normalized_foot_drift_px: foot_drift,
            content_sha256: hash_rgba(&canvas),
        });
        paths.push(path);
        normalized.push(canvas);
    }
    if max_source_scale_drift > 0.20 {
        reasons.push(format!(
            "source body scale drift {:.3} exceeds 0.200",
            max_source_scale_drift
        ));
    }
    let max_center_drift = frame_reports
        .iter()
        .map(|frame| frame.normalized_center_drift_px)
        .fold(0.0_f32, f32::max);
    let max_foot_drift = frame_reports
        .iter()
        .map(|frame| frame.normalized_foot_drift_px)
        .fold(0.0_f32, f32::max);
    if max_center_drift > 2.0 {
        reasons.push(format!(
            "normalized center drift {:.3}px exceeds 2px",
            max_center_drift
        ));
    }
    if max_foot_drift > 2.0 {
        reasons.push(format!(
            "normalized foot drift {:.3}px exceeds 2px",
            max_foot_drift
        ));
    }
    let mut onion_skin_paths = Vec::with_capacity(4);
    for index in 0..normalized.len() {
        let previous = &normalized[(index + normalized.len() - 1) % normalized.len()];
        let current = &normalized[index];
        let overlay = onion_overlay(previous, current);
        let path = onion_directory.join(format!("frame-{index:02}.png"));
        overlay.save(&path)?;
        onion_skin_paths.push(path);
    }
    let verdict = if reasons.is_empty() {
        AnimationSheetVerdictV1::GameReady
    } else {
        AnimationSheetVerdictV1::Regenerate
    };
    Ok(SharedSheetAlignmentV1 {
        report: OnionSkinAlignmentReportV1 {
            schema_version: "1".into(),
            profile: ONION_SKIN_ALIGNMENT_PROFILE_V1.into(),
            canvas_size,
            target_body_height,
            shared_scale,
            max_source_scale_drift,
            max_normalized_center_drift_px: max_center_drift,
            max_normalized_foot_drift_px: max_foot_drift,
            frames: frame_reports,
            verdict,
            reasons,
        },
        paths,
        images: normalized,
        onion_skin_paths,
    })
}

fn onion_overlay(previous: &RgbaImage, current: &RgbaImage) -> RgbaImage {
    let mut output = RgbaImage::from_pixel(current.width(), current.height(), Rgba([0, 0, 0, 0]));
    for y in 0..current.height() {
        for x in 0..current.width() {
            let before = previous.get_pixel(x, y).0;
            let now = current.get_pixel(x, y).0;
            let before_alpha = before[3] as f32 / 255.0 * 0.35;
            let now_alpha = now[3] as f32 / 255.0 * 0.75;
            let alpha = (before_alpha + now_alpha - before_alpha * now_alpha).clamp(0.0, 1.0);
            if alpha <= f32::EPSILON {
                continue;
            }
            let red = (now[0] as f32 * now_alpha + 40.0 * before_alpha * (1.0 - now_alpha)) / alpha;
            let green =
                (now[1] as f32 * now_alpha + 210.0 * before_alpha * (1.0 - now_alpha)) / alpha;
            let blue =
                (now[2] as f32 * now_alpha + 235.0 * before_alpha * (1.0 - now_alpha)) / alpha;
            output.put_pixel(
                x,
                y,
                Rgba([
                    red.clamp(0.0, 255.0) as u8,
                    green.clamp(0.0, 255.0) as u8,
                    blue.clamp(0.0, 255.0) as u8,
                    (alpha * 255.0) as u8,
                ]),
            );
        }
    }
    output
}

struct CellMetrics {
    foreground_ratio: f32,
    touches_cell_boundary: bool,
}

fn inspect_cell(cell: &RgbaImage) -> CellMetrics {
    let mut foreground = 0_u64;
    let mut boundary = false;
    let margin = ((cell.width().min(cell.height()) as f32 * 0.015).ceil() as u32).max(1);
    for (x, y, pixel) in cell.enumerate_pixels() {
        if is_foreground(pixel.0) {
            foreground += 1;
            if x < margin || y < margin || x + margin >= cell.width() || y + margin >= cell.height()
            {
                boundary = true;
            }
        }
    }
    CellMetrics {
        foreground_ratio: foreground as f32 / (cell.width() * cell.height()) as f32,
        touches_cell_boundary: boundary,
    }
}

// Provider fixtures and current xAI prompts use either native alpha or a flat
// chroma green background. This conservative predicate is only a layout gate;
// the real matting implementation remains the source of truth downstream.
fn is_foreground([r, g, b, a]: [u8; 4]) -> bool {
    if a <= 12 {
        return false;
    }
    let chroma_green =
        g > 96 && g as i16 - r as i16 > 28 && g as i16 - b as i16 > 20 && r < 150 && b < 150;
    !chroma_green
}

fn hash_rgba(image: &RgbaImage) -> String {
    let mut hasher = Sha256::new();
    hasher.update(image.width().to_le_bytes());
    hasher.update(image.height().to_le_bytes());
    hasher.update(image.as_raw());
    format!("{:x}", hasher.finalize())
}

fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();
    hasher.update(fs::read(path)?);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn valid_sheet(path: &Path) {
        let mut sheet = RgbaImage::from_pixel(192, 192, Rgba([0, 255, 0, 255]));
        for row in 0..2 {
            for column in 0..2 {
                let ox = column * 96;
                let oy = row * 96;
                for y in 20..84 {
                    for x in (24 + column * 2)..(72 + column * 2) {
                        sheet.put_pixel(ox + x, oy + y, Rgba([80, 50 + row as u8, 25, 255]));
                    }
                }
            }
        }
        sheet.save(path).unwrap();
    }

    #[test]
    fn extracts_fixed_two_by_two_order() {
        let temp = tempfile::tempdir().unwrap();
        let sheet = temp.path().join("sheet.png");
        valid_sheet(&sheet);
        let result = inspect_and_extract_animation_sheet(
            &sheet,
            &temp.path().join("extract"),
            "walk_right",
            1,
        )
        .unwrap();
        assert!(result.report.is_game_ready());
        assert_eq!(result.frames.len(), 4);
        assert_eq!(result.report.cells[3].row, 1);
        assert_eq!(result.report.cells[3].column, 1);
    }

    #[test]
    fn rejects_non_square_sheet() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("bad.png");
        RgbaImage::from_pixel(192, 128, Rgba([0, 255, 0, 255]))
            .save(&path)
            .unwrap();
        let error =
            inspect_and_extract_animation_sheet(&path, &temp.path().join("extract"), "idle", 1)
                .unwrap_err();
        assert!(error.to_string().contains("square"));
    }

    #[test]
    fn marks_cross_cell_bleed_for_regeneration() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("sheet.png");
        valid_sheet(&sheet_path);
        let mut sheet = image::open(&sheet_path).unwrap().to_rgba8();
        sheet.put_pixel(0, 30, Rgba([90, 50, 20, 255]));
        sheet.save(&sheet_path).unwrap();
        let result = inspect_and_extract_animation_sheet(
            &sheet_path,
            &temp.path().join("extract"),
            "walk_up",
            1,
        )
        .unwrap();
        assert_eq!(result.report.verdict, AnimationSheetVerdictV1::Regenerate);
        assert!(result.report.reasons[0].contains("boundary"));
    }
}
