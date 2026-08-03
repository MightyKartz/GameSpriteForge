use image::RgbaImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::asset_project::{image_signature, palette_overlap, ImageSignature};
use crate::frames::{bbox_from_image, FrameBbox};

pub const LOOP_SELECTION_PROFILE: &str = "loop@2.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopSelectionVerdict {
    GameReady,
    AwaitingReview,
    Regenerate,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopSelectionPolicy {
    pub target_frame_count: u32,
    pub candidate_fps: f32,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub minimum_motion_energy: f32,
    #[serde(default)]
    pub alpha_threshold: u8,
}

impl LoopSelectionPolicy {
    pub fn for_animation(
        animation: &str,
        target_frame_count: u32,
        candidate_fps: f32,
        source_duration_ms: u64,
    ) -> Self {
        let idle = animation == "idle";
        Self {
            target_frame_count,
            candidate_fps,
            min_duration_ms: if idle { 600 } else { 350 },
            max_duration_ms: ((source_duration_ms as f64 * 0.95).round() as u64).max(if idle {
                600
            } else {
                350
            }),
            minimum_motion_energy: if idle { 0.002 } else { 0.01 },
            alpha_threshold: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopSelectionReport {
    pub profile: String,
    pub candidate_frame_count: usize,
    pub candidate_fps: f32,
    pub selected_start_frame: usize,
    pub selected_end_boundary_frame: usize,
    pub selected_duration_ms: u64,
    pub output_frame_indices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_frame_sha256: Vec<String>,
    pub boundary_mask_iou: f32,
    pub boundary_palette_overlap: f32,
    pub boundary_edge_overlap: f32,
    pub anchor_closure_px: f32,
    pub transition_continuity: f32,
    pub motion_energy: f32,
    pub composite_score: f32,
    pub verdict: LoopSelectionVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopSelectionResult {
    pub report: LoopSelectionReport,
    pub frames: Vec<RgbaImage>,
}

#[derive(Debug, Error)]
pub enum LoopSelectionError {
    #[error("loop selection requires at least three candidate frames")]
    TooFewFrames,
    #[error("loop selection candidate frames must share one non-zero canvas size")]
    InconsistentCanvas,
    #[error("loop selection policy is invalid: {0}")]
    InvalidPolicy(String),
    #[error("no candidate period satisfies the configured duration and frame-count bounds")]
    NoCandidatePeriod,
}

#[derive(Debug, Clone)]
struct Candidate {
    report: LoopSelectionReport,
}

struct FrameAnalysis {
    bbox: FrameBbox,
    signature: ImageSignature,
    alpha_mask: Vec<u64>,
    edge_mask: Vec<u64>,
}

struct LoopAnalysis {
    frames: Vec<FrameAnalysis>,
    frame_differences: Vec<f32>,
}

impl LoopAnalysis {
    fn new(frames: &[RgbaImage], alpha_threshold: u8) -> Self {
        let analyzed_frames = frames
            .iter()
            .map(|frame| {
                let edges = edge_mask(frame, alpha_threshold);
                FrameAnalysis {
                    bbox: bbox_from_image(frame, alpha_threshold),
                    signature: image_signature(frame),
                    alpha_mask: pack_mask(frame.pixels().map(|pixel| pixel[3] > alpha_threshold)),
                    edge_mask: pack_mask(edges.into_iter()),
                }
            })
            .collect::<Vec<_>>();
        let analysis_frames = frames
            .iter()
            .map(resize_for_motion_analysis)
            .collect::<Vec<_>>();
        let count = frames.len();
        let mut frame_differences = vec![0.0; count * count];
        for left in 0..count {
            for right in left + 1..count {
                let difference = frame_difference(&analysis_frames[left], &analysis_frames[right]);
                frame_differences[left * count + right] = difference;
                frame_differences[right * count + left] = difference;
            }
        }
        Self {
            frames: analyzed_frames,
            frame_differences,
        }
    }

    fn frame_difference(&self, left: usize, right: usize) -> f32 {
        self.frame_differences[left * self.frames.len() + right]
    }
}

pub fn select_loop_frames(
    frames: &[RgbaImage],
    policy: LoopSelectionPolicy,
) -> Result<LoopSelectionResult, LoopSelectionError> {
    validate_inputs(frames, policy)?;
    let min_span = frames_for_duration(policy.min_duration_ms, policy.candidate_fps).max(1);
    let max_span = frames_for_duration(policy.max_duration_ms, policy.candidate_fps)
        .max(min_span)
        .min(frames.len().saturating_sub(1));
    let target = policy.target_frame_count as usize;
    let mut best: Option<Candidate> = None;
    let analysis = LoopAnalysis::new(frames, policy.alpha_threshold);

    for start in 0..frames.len().saturating_sub(1) {
        let first_end = start.saturating_add(min_span.max(target));
        let last_end = start.saturating_add(max_span).min(frames.len() - 1);
        if first_end > last_end {
            continue;
        }
        for end in first_end..=last_end {
            let output_frame_indices = sample_excluding_boundary(start, end, target);
            let report = score_candidate(&analysis, policy, start, end, output_frame_indices);
            let replace = best
                .as_ref()
                .is_none_or(|current| candidate_rank(&report) > candidate_rank(&current.report));
            if replace {
                best = Some(Candidate { report });
            }
        }
    }

    let report = best.ok_or(LoopSelectionError::NoCandidatePeriod)?.report;
    let selected = report
        .output_frame_indices
        .iter()
        .map(|index| frames[*index].clone())
        .collect();
    Ok(LoopSelectionResult {
        report,
        frames: selected,
    })
}

fn validate_inputs(
    frames: &[RgbaImage],
    policy: LoopSelectionPolicy,
) -> Result<(), LoopSelectionError> {
    if frames.len() < 3 {
        return Err(LoopSelectionError::TooFewFrames);
    }
    let Some(first) = frames.first() else {
        return Err(LoopSelectionError::TooFewFrames);
    };
    if first.width() == 0
        || first.height() == 0
        || frames
            .iter()
            .any(|frame| frame.dimensions() != first.dimensions())
    {
        return Err(LoopSelectionError::InconsistentCanvas);
    }
    if policy.target_frame_count < 2
        || policy.target_frame_count as usize >= frames.len()
        || !policy.candidate_fps.is_finite()
        || policy.candidate_fps <= 0.0
        || policy.min_duration_ms == 0
        || policy.max_duration_ms < policy.min_duration_ms
        || !(0.0..=1.0).contains(&policy.minimum_motion_energy)
    {
        return Err(LoopSelectionError::InvalidPolicy(
            "frame count, fps, durations, or motion threshold are out of range".into(),
        ));
    }
    Ok(())
}

fn frames_for_duration(duration_ms: u64, fps: f32) -> usize {
    ((duration_ms as f64 / 1000.0) * fps as f64).ceil() as usize
}

fn sample_excluding_boundary(start: usize, end: usize, count: usize) -> Vec<usize> {
    let span = end - start;
    (0..count)
        .map(|position| start + position * span / count)
        .collect()
}

fn score_candidate(
    analysis: &LoopAnalysis,
    policy: LoopSelectionPolicy,
    start: usize,
    end: usize,
    output_frame_indices: Vec<usize>,
) -> LoopSelectionReport {
    let first = &analysis.frames[start];
    let boundary = &analysis.frames[end];
    let first_bbox = first.bbox;
    let boundary_bbox = boundary.bbox;
    let selected_bboxes = output_frame_indices
        .iter()
        .map(|index| analysis.frames[*index].bbox)
        .collect::<Vec<_>>();
    let hard_blocked = !first_bbox.has_foreground()
        || !boundary_bbox.has_foreground()
        || selected_bboxes.iter().any(|bbox| !bbox.has_foreground());
    let boundary_mask_iou = packed_mask_iou(&first.alpha_mask, &boundary.alpha_mask);
    let boundary_palette_overlap =
        palette_overlap(&first.signature.palette, &boundary.signature.palette);
    let boundary_edge_overlap = packed_mask_iou(&first.edge_mask, &boundary.edge_mask);
    let anchor_closure_px = anchor_distance(first_bbox, boundary_bbox);
    let anchor_score = (1.0 - anchor_closure_px / 6.0).clamp(0.0, 1.0);
    let transition_samples = output_frame_indices
        .windows(2)
        .map(|indices| {
            let gap = indices[1].saturating_sub(indices[0]).max(1) as f32;
            let difference = analysis.frame_difference(indices[0], indices[1]);
            (difference, difference / gap)
        })
        .collect::<Vec<_>>();
    let motion_energy = median(
        &transition_samples
            .iter()
            .map(|(difference, _)| *difference)
            .collect::<Vec<_>>(),
    );
    let transition_speed = median(
        &transition_samples
            .iter()
            .map(|(_, speed)| *speed)
            .collect::<Vec<_>>(),
    );
    let wrap_speed = output_frame_indices
        .last()
        .map(|last| {
            let gap = end.saturating_sub(*last).max(1) as f32;
            analysis.frame_difference(*last, end) / gap
        })
        .unwrap_or(1.0);
    let transition_continuity = if transition_speed <= f32::EPSILON {
        0.0
    } else {
        (1.0 - (wrap_speed - transition_speed).abs() / transition_speed.max(0.01)).clamp(0.0, 1.0)
    };
    let composite_score = (boundary_mask_iou * 0.30
        + boundary_palette_overlap * 0.20
        + boundary_edge_overlap * 0.20
        + anchor_score * 0.15
        + transition_continuity * 0.15)
        .clamp(0.0, 1.0);
    let duration_ms =
        (((end - start) as f64 / policy.candidate_fps as f64) * 1000.0).round() as u64;
    let mut reasons = Vec::new();
    if hard_blocked {
        reasons.push("foreground_missing".into());
    }
    if boundary_mask_iou < 0.75 {
        reasons.push("boundary_mask_mismatch".into());
    }
    if anchor_closure_px > 2.0 {
        reasons.push("anchor_closure_drift".into());
    }
    if transition_continuity < 0.70 {
        reasons.push("wrap_transition_discontinuity".into());
    }
    if motion_energy < policy.minimum_motion_energy {
        reasons.push("motion_energy_too_low".into());
    }
    let verdict = if hard_blocked {
        LoopSelectionVerdict::Blocked
    } else if composite_score >= 0.80
        && boundary_mask_iou >= 0.75
        && anchor_closure_px <= 2.0
        && transition_continuity >= 0.70
        && motion_energy >= policy.minimum_motion_energy
    {
        LoopSelectionVerdict::GameReady
    } else if composite_score >= 0.70
        && boundary_mask_iou >= 0.65
        && anchor_closure_px <= 6.0
        && transition_continuity >= 0.55
        && motion_energy >= policy.minimum_motion_energy
    {
        LoopSelectionVerdict::AwaitingReview
    } else {
        LoopSelectionVerdict::Regenerate
    };
    LoopSelectionReport {
        profile: LOOP_SELECTION_PROFILE.into(),
        candidate_frame_count: analysis.frames.len(),
        candidate_fps: policy.candidate_fps,
        selected_start_frame: start,
        selected_end_boundary_frame: end,
        selected_duration_ms: duration_ms,
        output_frame_indices,
        output_frame_sha256: Vec::new(),
        boundary_mask_iou,
        boundary_palette_overlap,
        boundary_edge_overlap,
        anchor_closure_px,
        transition_continuity,
        motion_energy,
        composite_score,
        verdict,
        reasons,
    }
}

fn candidate_rank(report: &LoopSelectionReport) -> (u8, i64, i64) {
    let verdict = match report.verdict {
        LoopSelectionVerdict::GameReady => 3,
        LoopSelectionVerdict::AwaitingReview => 2,
        LoopSelectionVerdict::Regenerate => 1,
        LoopSelectionVerdict::Blocked => 0,
    };
    (
        verdict,
        (report.composite_score * 1_000_000.0).round() as i64,
        -(report.selected_duration_ms as i64),
    )
}

fn pack_mask(mask: impl Iterator<Item = bool>) -> Vec<u64> {
    let mut packed = Vec::<u64>::new();
    for (index, value) in mask.enumerate() {
        let word = index / 64;
        if word == packed.len() {
            packed.push(0);
        }
        if value {
            packed[word] |= 1_u64 << (index % 64);
        }
    }
    packed
}

fn packed_mask_iou(left: &[u64], right: &[u64]) -> f32 {
    let mut intersection = 0_u32;
    let mut union = 0_u32;
    for (left, right) in left.iter().zip(right) {
        intersection += (left & right).count_ones();
        union += (left | right).count_ones();
    }
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

fn resize_for_motion_analysis(image: &RgbaImage) -> RgbaImage {
    const MAX_ANALYSIS_SIZE: u32 = 64;
    let longest = image.width().max(image.height()).max(1);
    if longest <= MAX_ANALYSIS_SIZE {
        return image.clone();
    }
    let width = (image.width() * MAX_ANALYSIS_SIZE / longest).max(1);
    let height = (image.height() * MAX_ANALYSIS_SIZE / longest).max(1);
    image::imageops::resize(image, width, height, image::imageops::FilterType::Nearest)
}

fn edge_mask(image: &RgbaImage, alpha_threshold: u8) -> Vec<bool> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut edges = vec![false; width * height];
    if width < 3 || height < 3 {
        return edges;
    }
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let center = image.get_pixel(x as u32, y as u32);
            if center[3] <= alpha_threshold {
                continue;
            }
            let right = image.get_pixel((x + 1) as u32, y as u32);
            let down = image.get_pixel(x as u32, (y + 1) as u32);
            let alpha_edge = right[3] <= alpha_threshold || down[3] <= alpha_threshold;
            let luminance = |pixel: &image::Rgba<u8>| {
                pixel[0] as i32 * 3 + pixel[1] as i32 * 6 + pixel[2] as i32
            };
            let contrast = (luminance(center) - luminance(right)).abs()
                + (luminance(center) - luminance(down)).abs();
            edges[y * width + x] = alpha_edge || contrast > 240;
        }
    }
    edges
}

fn anchor_distance(left: FrameBbox, right: FrameBbox) -> f32 {
    if !left.has_foreground() || !right.has_foreground() {
        return f32::INFINITY;
    }
    ((left.center_x - right.center_x).powi(2) + (left.bottom_y - right.bottom_y).powi(2)).sqrt()
}

fn frame_difference(left: &RgbaImage, right: &RgbaImage) -> f32 {
    let mut total = 0.0f32;
    let mut count = 0usize;
    for (left, right) in left.pixels().zip(right.pixels()) {
        if left[3] == 0 && right[3] == 0 {
            continue;
        }
        let alpha = (left[3] as f32 - right[3] as f32).abs() / 255.0;
        let rgb = ((left[0] as f32 - right[0] as f32).abs()
            + (left[1] as f32 - right[1] as f32).abs()
            + (left[2] as f32 - right[2] as f32).abs())
            / (255.0 * 3.0);
        total += alpha * 0.5 + rgb * 0.5;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut values = values.to_vec();
    values.sort_by(f32::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn frame(x: u32, pose: u8) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 0]));
        for y in 12..28 {
            for dx in 0..8 {
                let shade = if (dx + y + pose as u32).is_multiple_of(3) {
                    220
                } else {
                    150
                };
                image.put_pixel(x + dx, y, Rgba([shade, 80 + pose * 4, 60, 255]));
            }
        }
        image
    }

    fn policy(target: u32, minimum_motion_energy: f32) -> LoopSelectionPolicy {
        LoopSelectionPolicy {
            target_frame_count: target,
            candidate_fps: 10.0,
            min_duration_ms: 400,
            max_duration_ms: 1_200,
            minimum_motion_energy,
            alpha_threshold: 0,
        }
    }

    #[test]
    fn excludes_the_matching_boundary_frame_from_output() {
        let frames = vec![
            frame(12, 0),
            frame(12, 1),
            frame(12, 2),
            frame(12, 3),
            frame(12, 0),
            frame(12, 1),
        ];
        let result = select_loop_frames(&frames, policy(4, 0.001)).unwrap();

        assert_eq!(
            frames[result.report.selected_start_frame],
            frames[result.report.selected_end_boundary_frame]
        );
        assert_eq!(result.report.output_frame_indices.len(), 4);
        assert!(!result
            .report
            .output_frame_indices
            .contains(&result.report.selected_end_boundary_frame));
    }

    #[test]
    fn finds_a_cycle_inside_noisy_leading_and_trailing_frames() {
        let frames = vec![
            frame(2, 7),
            frame(12, 0),
            frame(12, 1),
            frame(12, 2),
            frame(12, 3),
            frame(12, 0),
            frame(22, 8),
        ];
        let result = select_loop_frames(&frames, policy(4, 0.001)).unwrap();

        assert_eq!(result.report.selected_start_frame, 1);
        assert_eq!(result.report.selected_end_boundary_frame, 5);
        assert_eq!(result.report.output_frame_indices, vec![1, 2, 3, 4]);
    }

    #[test]
    fn normalizes_wrap_continuity_for_non_uniform_sample_gaps() {
        let frames = [12, 13, 14, 15, 16, 17, 16, 15, 14, 13, 12, 2]
            .into_iter()
            .map(|x| frame(x, 0))
            .collect::<Vec<_>>();
        let result = select_loop_frames(
            &frames,
            LoopSelectionPolicy {
                target_frame_count: 8,
                candidate_fps: 10.0,
                min_duration_ms: 900,
                max_duration_ms: 1_000,
                minimum_motion_energy: 0.001,
                alpha_threshold: 0,
            },
        )
        .unwrap();

        assert_eq!(result.report.selected_start_frame, 0);
        assert_eq!(result.report.selected_end_boundary_frame, 10);
        assert!(result.report.transition_continuity >= 0.70);
        assert_eq!(result.report.verdict, LoopSelectionVerdict::GameReady);
    }

    #[test]
    fn rejects_a_static_walk_even_with_perfect_boundary_match() {
        let still = frame(12, 0);
        let frames = vec![still; 7];
        let result = select_loop_frames(&frames, policy(4, 0.01)).unwrap();

        assert_eq!(result.report.verdict, LoopSelectionVerdict::Regenerate);
        assert!(result
            .report
            .reasons
            .contains(&"motion_energy_too_low".to_string()));
    }

    #[test]
    fn blocks_candidates_with_missing_foreground() {
        let empty = RgbaImage::from_pixel(32, 32, Rgba([0, 0, 0, 0]));
        let frames = vec![empty; 7];
        let result = select_loop_frames(&frames, policy(4, 0.0)).unwrap();

        assert_eq!(result.report.verdict, LoopSelectionVerdict::Blocked);
    }

    #[test]
    fn requires_one_shared_canvas() {
        let frames = vec![frame(12, 0), frame(12, 1), RgbaImage::new(16, 16)];

        assert!(matches!(
            select_loop_frames(&frames, policy(2, 0.0)),
            Err(LoopSelectionError::InconsistentCanvas)
        ));
    }
}
