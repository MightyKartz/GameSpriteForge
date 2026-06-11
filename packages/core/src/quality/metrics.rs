use serde::{Deserialize, Serialize};

use crate::frames::{FrameBbox, FrameSize};

use super::looping::loop_match_score;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityMetrics {
    #[serde(rename = "bboxBottomDriftPx")]
    pub bbox_bottom_drift_px: f32,
    #[serde(rename = "bboxCenterXDriftPx")]
    pub bbox_center_x_drift_px: f32,
    #[serde(rename = "bboxCenterYDriftPx")]
    pub bbox_center_y_drift_px: f32,
    #[serde(rename = "bboxWidthVariationPx")]
    pub bbox_width_variation_px: f32,
    pub alpha_coverage_avg: f32,
    pub loop_match_score: f32,
    pub frame_count: usize,
    pub frame_size_consistent: bool,
    pub cell_boundary_safe: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityVerdict {
    GameReady,
    NeedsCleanup,
    PrototypeUsable,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityRecommendationId {
    AdjustAnchor,
    TrimLoopRange,
    IncreaseChromaThreshold,
    ReduceChromaThreshold,
    UseShorterClip,
    IncreaseCanvasMargin,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityReport {
    pub verdict: QualityVerdict,
    pub metrics: QualityMetrics,
    pub recommendations: Vec<QualityRecommendationId>,
    pub notes: Vec<String>,
}

pub fn compute_quality_report(bboxes: &[FrameBbox], sizes: &[FrameSize]) -> QualityReport {
    build_quality_report(bboxes, compute_quality_metrics(bboxes, sizes))
}

pub fn compute_quality_report_with_loop_range(
    bboxes: &[FrameBbox],
    sizes: &[FrameSize],
    loop_range: Option<(usize, usize)>,
) -> QualityReport {
    build_quality_report(
        bboxes,
        compute_quality_metrics_with_loop_range(bboxes, sizes, loop_range),
    )
}

fn build_quality_report(bboxes: &[FrameBbox], metrics: QualityMetrics) -> QualityReport {
    let foreground_missing = bboxes.iter().any(|bbox| !bbox.has_foreground());
    let verdict = quality_verdict(&metrics, foreground_missing);
    let recommendations = quality_recommendations(&metrics, foreground_missing);
    let mut notes = Vec::new();

    if metrics.frame_count < 2 {
        notes.push("frame_count_below_minimum".to_string());
    }
    if foreground_missing {
        notes.push("frame_without_foreground".to_string());
    }
    if !metrics.frame_size_consistent {
        notes.push("frame_size_inconsistent".to_string());
    }

    QualityReport {
        verdict,
        metrics,
        recommendations,
        notes,
    }
}

pub fn compute_quality_metrics(bboxes: &[FrameBbox], sizes: &[FrameSize]) -> QualityMetrics {
    compute_quality_metrics_with_loop_range(bboxes, sizes, None)
}

pub fn compute_quality_metrics_with_loop_range(
    bboxes: &[FrameBbox],
    sizes: &[FrameSize],
    loop_range: Option<(usize, usize)>,
) -> QualityMetrics {
    let frame_count = bboxes.len();
    let foreground: Vec<FrameBbox> = bboxes
        .iter()
        .copied()
        .filter(|bbox| bbox.has_foreground())
        .collect();
    let frame_size_consistent = sizes
        .split_first()
        .map(|(first, rest)| rest.iter().all(|size| size == first))
        .unwrap_or(true);

    let loop_match_score = selected_loop_score(bboxes, loop_range);

    QualityMetrics {
        bbox_bottom_drift_px: range_by(&foreground, |bbox| bbox.bottom_y),
        bbox_center_x_drift_px: range_by(&foreground, |bbox| bbox.center_x),
        bbox_center_y_drift_px: range_by(&foreground, |bbox| bbox.center_y),
        bbox_width_variation_px: range_by(&foreground, |bbox| bbox.width),
        alpha_coverage_avg: average_by(bboxes, |bbox| bbox.alpha_coverage),
        loop_match_score,
        frame_count,
        frame_size_consistent,
        cell_boundary_safe: cell_boundary_safe(bboxes, sizes),
    }
}

fn selected_loop_score(bboxes: &[FrameBbox], loop_range: Option<(usize, usize)>) -> f32 {
    let Some((start, end)) = loop_range else {
        return match (bboxes.first(), bboxes.last()) {
            (Some(first), Some(last)) if bboxes.len() >= 2 => loop_match_score(first, last),
            _ => 0.0,
        };
    };
    if start >= bboxes.len() || end >= bboxes.len() || start >= end {
        return match (bboxes.first(), bboxes.last()) {
            (Some(first), Some(last)) if bboxes.len() >= 2 => loop_match_score(first, last),
            _ => 0.0,
        };
    }
    loop_match_score(&bboxes[start], &bboxes[end])
}

pub fn quality_recommendations(
    metrics: &QualityMetrics,
    foreground_missing: bool,
) -> Vec<QualityRecommendationId> {
    let mut recommendations = Vec::new();

    if metrics.bbox_bottom_drift_px > 2.0 || metrics.bbox_center_x_drift_px > 12.0 {
        recommendations.push(QualityRecommendationId::AdjustAnchor);
    }
    if metrics.loop_match_score < 0.75 && metrics.frame_count >= 2 {
        recommendations.push(QualityRecommendationId::TrimLoopRange);
    }
    if foreground_missing || metrics.alpha_coverage_avg < 0.02 {
        recommendations.push(QualityRecommendationId::ReduceChromaThreshold);
    }
    if metrics.alpha_coverage_avg > 0.60 {
        recommendations.push(QualityRecommendationId::IncreaseChromaThreshold);
    }
    if metrics.frame_count > 12 {
        recommendations.push(QualityRecommendationId::UseShorterClip);
    }
    if !metrics.cell_boundary_safe {
        recommendations.push(QualityRecommendationId::IncreaseCanvasMargin);
    }

    recommendations
}

fn quality_verdict(metrics: &QualityMetrics, foreground_missing: bool) -> QualityVerdict {
    if metrics.frame_count < 2 || foreground_missing || !metrics.frame_size_consistent {
        return QualityVerdict::Blocked;
    }

    if metrics.bbox_bottom_drift_px <= 2.0
        && metrics.bbox_center_x_drift_px <= 12.0
        && metrics.loop_match_score >= 0.75
    {
        return QualityVerdict::GameReady;
    }

    if metrics.bbox_bottom_drift_px <= 8.0 && metrics.bbox_center_x_drift_px <= 32.0 {
        return QualityVerdict::NeedsCleanup;
    }

    QualityVerdict::PrototypeUsable
}

fn range_by<F>(bboxes: &[FrameBbox], value: F) -> f32
where
    F: Fn(&FrameBbox) -> f32,
{
    if bboxes.is_empty() {
        return 0.0;
    }

    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for bbox in bboxes {
        let current = value(bbox);
        min = min.min(current);
        max = max.max(current);
    }
    max - min
}

fn average_by<F>(bboxes: &[FrameBbox], value: F) -> f32
where
    F: Fn(&FrameBbox) -> f32,
{
    if bboxes.is_empty() {
        return 0.0;
    }

    bboxes.iter().map(value).sum::<f32>() / bboxes.len() as f32
}

fn cell_boundary_safe(bboxes: &[FrameBbox], sizes: &[FrameSize]) -> bool {
    if bboxes.len() != sizes.len() {
        return false;
    }

    bboxes.iter().zip(sizes).all(|(bbox, size)| {
        !bbox.has_foreground()
            || (bbox.left >= 1.0
                && bbox.top >= 1.0
                && bbox.right <= size.width as f32 - 1.0
                && bbox.bottom <= size.height as f32 - 1.0)
    })
}
