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

/// Additive, per-action diagnostics. Indices are zero-based within the action;
/// evidence_path is a JSON pointer relative to this action's QualityReport.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationIssue {
    pub code: String,
    pub severity: String,
    pub certainty: String,
    pub frame_index: usize,
    pub related_frame_index: Option<usize>,
    pub evidence_path: String,
    pub message: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationPixelDiagnostics {
    pub schema_version: String,
    #[serde(default)]
    pub issues: Vec<AnimationIssue>,
    pub profile: QualityProfile,
    pub visual_approval: String,
    pub loop_check_applicable: bool,
    pub transparent_tail_allowed: bool,
    pub transparent_tail_start: Option<usize>,
    /// Frames after matting / selected loop extraction, before normalization.
    pub processed_source: PixelSequenceEvidence,
    pub normalized: PixelSequenceEvidence,
}

fn frame_issues(
    evidence: &PixelSequenceEvidence,
    bboxes: &[FrameBbox],
    transparent_tail: Option<usize>,
) -> Vec<AnimationIssue> {
    let mut issues = Vec::new();
    for (index, frame) in evidence.frames.iter().enumerate() {
        let path = format!("/pixelDiagnostics/normalized/frames/{index}");
        let mut add =
            |code: &str, severity: &str, certainty: &str, message: &str, options: &[&str]| {
                issues.push(AnimationIssue {
                    code: code.into(),
                    severity: severity.into(),
                    certainty: certainty.into(),
                    frame_index: index,
                    related_frame_index: None,
                    evidence_path: path.clone(),
                    message: message.into(),
                    options: options.iter().map(|v| (*v).into()).collect(),
                });
            };
        if frame.fully_transparent && transparent_tail.is_none_or(|start| index < start) {
            add(
                "empty_frame",
                "error",
                "deterministic",
                "Frame has no nontransparent pixels.",
                &["replace_source_frame", "inspect_matting"],
            );
        } else if !frame.fully_transparent
            && bboxes.get(index).is_some_and(|bbox| !bbox.has_foreground())
        {
            add(
                "foreground_below_threshold",
                "error",
                "deterministic",
                "Visible alpha exists but no foreground survives the configured bounds threshold.",
                &["inspect_alpha_threshold", "inspect_source_frame"],
            );
        }
        if frame.alpha_touches_edge {
            add(
                "canvas_edge_contact",
                "warning",
                "review_required",
                "Nontransparent pixels touch a canvas edge; this does not prove clipping.",
                &[
                    "inspect_source_frame",
                    "replace_source_frame",
                    "review_shared_canvas",
                ],
            );
        }
        if evidence
            .frames
            .first()
            .is_some_and(|first| (first.width, first.height) != (frame.width, frame.height))
        {
            add(
                "frame_size_mismatch",
                "error",
                "deterministic",
                "Frame canvas differs from the first frame.",
                &["restore_shared_canvas"],
            );
        }
    }
    for delta in evidence.adjacent_differences.iter().flatten() {
        if delta.alpha_mean_absolute_difference == 0.0
            && delta.premultiplied_rgb_mean_absolute_difference == 0.0
        {
            issues.push(AnimationIssue {
                code: "identical_visible_frames".into(), severity: "info".into(), certainty: "review_required".into(),
                frame_index: delta.to_frame, related_frame_index: Some(delta.from_frame),
                evidence_path: format!("/pixelDiagnostics/normalized/adjacentDifferences/{}", delta.from_frame),
                message: "Adjacent visible pixels are identical; an intentional hold is valid and is not removed.".into(),
                options: vec!["keep_intentional_hold".into(), "inspect_source_frame".into()],
            });
        }
    }
    issues
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
        issues: frame_issues(
            &normalized,
            bboxes,
            if tail_allowed { valid_tail } else { None },
        ),
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
