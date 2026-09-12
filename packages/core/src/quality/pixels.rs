//! Pixel evidence for animation review. These measurements do not approve artwork.
use std::path::PathBuf;

use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::frames::{bbox_from_image, FrameBbox, FrameSize};

use super::{
    compute_quality_report_for_animation, QualityRecommendationId, QualityReport, QualityVerdict,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityProfile {
    #[default]
    Character,
    Effect,
}

impl QualityProfile {
    pub fn is_character(&self) -> bool {
        *self == Self::Character
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelFrameEvidence {
    pub width: u32,
    pub height: u32,
    /// Pixel bounds use inclusive left/top and exclusive right/bottom; alpha > 0 counts.
    pub alpha_bounds: FrameBbox,
    pub alpha_mean: f64,
    pub fully_transparent: bool,
    /// Rec. 709 weights applied to alpha-premultiplied, encoded RGB (not linear light).
    pub premultiplied_brightness_mean: f64,
    pub nonzero_rgb_under_zero_alpha_pixels: u64,
    pub alpha_touches_edge: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelDifference {
    pub from_frame: usize,
    pub to_frame: usize,
    pub alpha_mean_absolute_difference: f64,
    pub premultiplied_rgb_mean_absolute_difference: f64,
    pub brightness_mean_absolute_difference: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PixelSequenceEvidence {
    pub frames: Vec<PixelFrameEvidence>,
    /// Missing entries indicate unequal dimensions; images are never resized to compare them.
    pub adjacent_differences: Vec<Option<PixelDifference>>,
    pub first_last_difference: Option<PixelDifference>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationPixelDiagnostics {
    pub schema_version: String,
    pub profile: QualityProfile,
    pub visual_approval: String,
    pub loop_check_applicable: bool,
    pub transparent_tail_allowed: bool,
    pub transparent_tail_start: Option<usize>,
    /// Frames after matting / selected loop extraction, before normalization.
    pub processed_source: PixelSequenceEvidence,
    pub normalized: PixelSequenceEvidence,
}

pub fn measure_pixel_sequence(images: &[RgbaImage]) -> PixelSequenceEvidence {
    let frames = images.iter().map(measure_frame).collect();
    let adjacent_differences = images
        .windows(2)
        .enumerate()
        .map(|(index, pair)| difference(&pair[0], &pair[1], index, index + 1))
        .collect();
    let first_last_difference = if images.len() >= 2 {
        difference(&images[0], images.last().unwrap(), 0, images.len() - 1)
    } else {
        None
    };
    PixelSequenceEvidence {
        frames,
        adjacent_differences,
        first_last_difference,
    }
}

fn measure_frame(image: &RgbaImage) -> PixelFrameEvidence {
    let count = u64::from(image.width()) * u64::from(image.height());
    let denominator = count.max(1) as f64;
    let mut alpha_sum = 0.0;
    let mut brightness_sum = 0.0;
    let mut hidden_rgb = 0;
    let mut touches_edge = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        let values = premultiplied(pixel.0);
        alpha_sum += values[3];
        brightness_sum += brightness(values);
        hidden_rgb += u64::from(pixel[3] == 0 && pixel.0[..3].iter().any(|value| *value != 0));
        touches_edge |=
            pixel[3] > 0 && (x == 0 || y == 0 || x + 1 == image.width() || y + 1 == image.height());
    }
    PixelFrameEvidence {
        width: image.width(),
        height: image.height(),
        alpha_bounds: bbox_from_image(image, 0),
        alpha_mean: alpha_sum / denominator,
        fully_transparent: alpha_sum == 0.0,
        premultiplied_brightness_mean: brightness_sum / denominator,
        nonzero_rgb_under_zero_alpha_pixels: hidden_rgb,
        alpha_touches_edge: touches_edge,
    }
}

fn premultiplied(rgba: [u8; 4]) -> [f64; 4] {
    let alpha = f64::from(rgba[3]) / 255.0;
    [
        f64::from(rgba[0]) / 255.0 * alpha,
        f64::from(rgba[1]) / 255.0 * alpha,
        f64::from(rgba[2]) / 255.0 * alpha,
        alpha,
    ]
}

fn brightness(values: [f64; 4]) -> f64 {
    values[0] * 0.2126 + values[1] * 0.7152 + values[2] * 0.0722
}

fn difference(
    first: &RgbaImage,
    last: &RgbaImage,
    from_frame: usize,
    to_frame: usize,
) -> Option<PixelDifference> {
    if first.dimensions() != last.dimensions() {
        return None;
    }
    let count = (u64::from(first.width()) * u64::from(first.height())).max(1) as f64;
    let mut alpha = 0.0;
    let mut rgb = 0.0;
    let mut light = 0.0;
    for (first, last) in first.pixels().zip(last.pixels()) {
        let a = premultiplied(first.0);
        let b = premultiplied(last.0);
        alpha += (a[3] - b[3]).abs();
        rgb += (0..3)
            .map(|channel| (a[channel] - b[channel]).abs())
            .sum::<f64>()
            / 3.0;
        light += (brightness(a) - brightness(b)).abs();
    }
    Some(PixelDifference {
        from_frame,
        to_frame,
        alpha_mean_absolute_difference: alpha / count,
        premultiplied_rgb_mean_absolute_difference: rgb / count,
        brightness_mean_absolute_difference: light / count,
    })
}

fn read_frames(paths: &[PathBuf]) -> Result<Vec<RgbaImage>, image::ImageError> {
    paths
        .iter()
        .map(|path| image::open(path).map(|image| image.to_rgba8()))
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn compute_quality_report_with_pixels(
    bboxes: &[FrameBbox],
    sizes: &[FrameSize],
    processed_paths: &[PathBuf],
    normalized_paths: &[PathBuf],
    loop_animation: bool,
    profile: QualityProfile,
    allow_transparent_tail: bool,
) -> Result<QualityReport, image::ImageError> {
    let processed_source = measure_pixel_sequence(&read_frames(processed_paths)?);
    let normalized = measure_pixel_sequence(&read_frames(normalized_paths)?);
    let mut report = compute_quality_report_for_animation(bboxes, sizes, loop_animation);
    let tail_allowed =
        profile == QualityProfile::Effect && !loop_animation && allow_transparent_tail;
    let first_empty = normalized
        .frames
        .iter()
        .position(|frame| frame.fully_transparent);
    let valid_tail = first_empty.filter(|index| {
        *index > 0
            && normalized.frames[*index..]
                .iter()
                .all(|frame| frame.fully_transparent)
            && processed_source.frames.len() == normalized.frames.len()
            && processed_source.frames[*index..]
                .iter()
                .all(|frame| frame.fully_transparent)
    });
    let missing_foreground = bboxes.iter().enumerate().any(|(index, bbox)| {
        !(bbox.has_foreground() || tail_allowed && valid_tail.is_some_and(|start| index >= start))
    });
    let inconsistent = bboxes.len() != sizes.len()
        || bboxes.len() != normalized.frames.len()
        || processed_source.frames.len() != normalized.frames.len()
        || !report.metrics.frame_size_consistent;
    if profile == QualityProfile::Effect {
        report
            .recommendations
            .retain(|item| matches!(item, QualityRecommendationId::IncreaseCanvasMargin));
        report
            .notes
            .retain(|note| note != "frame_without_foreground");
        report
            .notes
            .push("effect_deformation_is_not_character_anchor_drift".into());
        if tail_allowed && valid_tail.is_some() {
            report
                .notes
                .push("explicit_transparent_non_looping_tail".into());
        }
        if missing_foreground {
            report.notes.push("frame_without_foreground".into());
        }
        report.verdict = if bboxes.len() < 2 || missing_foreground || inconsistent {
            QualityVerdict::Blocked
        } else if !report.metrics.cell_boundary_safe {
            QualityVerdict::NeedsCleanup
        } else {
            QualityVerdict::GameReady
        };
    }
    if inconsistent {
        report.verdict = QualityVerdict::Blocked;
        report
            .notes
            .push("pixel_evidence_frame_count_or_size_inconsistent".into());
    }
    if allow_transparent_tail && !tail_allowed {
        report.verdict = QualityVerdict::Blocked;
        report
            .notes
            .push("transparent_tail_requires_non_looping_effect_profile".into());
    }
    report
        .notes
        .push("structural_quality_is_not_visual_approval".into());
    report.pixel_diagnostics = Some(AnimationPixelDiagnostics {
        schema_version: "1".into(),
        profile,
        visual_approval: "not_assessed".into(),
        loop_check_applicable: loop_animation,
        transparent_tail_allowed: tail_allowed,
        transparent_tail_start: if tail_allowed { valid_tail } else { None },
        processed_source,
        normalized,
    });
    Ok(report)
}
