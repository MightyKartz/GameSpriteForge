use image::RgbaImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::asset_project::{image_signature, palette_overlap, ImageSignature};
use crate::frames::{bbox_from_image, FrameBbox};

pub const LOOP_SELECTION_PROFILE: &str = "loop@2.0.0";
pub const ANCHORED_LOOP_SELECTION_PROFILE: &str = "loop@2.1.0";
pub const ANIMATION_TIMING_PROFILE: &str = "animation-timing@1.0.0";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationCadenceProfile {
    Idle,
    Walk,
    #[default]
    Unconstrained,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimationTimingV1 {
    pub profile: String,
    pub cadence_profile: AnimationCadenceProfile,
    pub fundamental_period_ms: u64,
    pub output_timestamps_ms: Vec<u64>,
    pub frame_durations_ms: Vec<u64>,
    pub playback_duration_ms: u64,
    pub playback_fps: f32,
    pub cadence_score: f32,
    pub period_peak_prominence: f32,
    pub periodicity_confidence: f32,
    pub playback_speed_ratio: f32,
}

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
    #[serde(default)]
    pub cadence_profile: AnimationCadenceProfile,
}

/// Optional semantic anchoring for image-to-video clips whose first frame is
/// an immutable game asset. V1 intentionally supports only an initial-frame
/// reference: this prevents a visually closed but semantically unrelated
/// subsection (for example a closed-eye idle hold) from winning loop search.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopAnchorPolicyV1 {
    pub reference_frame: usize,
    pub maximum_start_frame: usize,
    pub minimum_start_similarity: f32,
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
            cadence_profile: if idle {
                AnimationCadenceProfile::Idle
            } else {
                AnimationCadenceProfile::Walk
            },
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<AnimationTimingV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_frame_sha256: Vec<String>,
    pub boundary_mask_iou: f32,
    pub boundary_palette_overlap: f32,
    pub boundary_edge_overlap: f32,
    pub anchor_closure_px: f32,
    pub transition_continuity: f32,
    pub motion_energy: f32,
    pub composite_score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_reference_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_maximum_start_frame: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_anchor_similarity: Option<f32>,
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
    select_loop_frames_inner(frames, policy, None)
}

pub fn select_loop_frames_with_anchor(
    frames: &[RgbaImage],
    policy: LoopSelectionPolicy,
    anchor: LoopAnchorPolicyV1,
) -> Result<LoopSelectionResult, LoopSelectionError> {
    if anchor.reference_frame >= frames.len()
        || anchor.maximum_start_frame >= frames.len()
        || !(0.0..=1.0).contains(&anchor.minimum_start_similarity)
    {
        return Err(LoopSelectionError::InvalidPolicy(
            "loop anchor frame, start window, or similarity is invalid".into(),
        ));
    }
    select_loop_frames_inner(frames, policy, Some(anchor))
}

/// Score an already selected semantic cycle without allowing the generic loop
/// search to replace its phase frames. This is used by gait-cycle@1.0.0 after
/// it has proved the two contacts and two passing poses. The end boundary is a
/// closure witness and must not appear in `output_frame_indices`.
pub fn assess_loop_window(
    frames: &[RgbaImage],
    policy: LoopSelectionPolicy,
    start: usize,
    end: usize,
    output_frame_indices: Vec<usize>,
) -> Result<LoopSelectionResult, LoopSelectionError> {
    validate_inputs(frames, policy)?;
    if start >= end
        || end >= frames.len()
        || output_frame_indices.len() != policy.target_frame_count as usize
        || output_frame_indices.first().copied() != Some(start)
        || output_frame_indices
            .iter()
            .any(|index| *index < start || *index >= end)
        || output_frame_indices
            .windows(2)
            .any(|indices| indices[0] >= indices[1])
    {
        return Err(LoopSelectionError::InvalidPolicy(
            "fixed loop window or semantic phase indices are invalid".into(),
        ));
    }
    let duration_ms =
        (((end - start) as f64 / policy.candidate_fps as f64) * 1000.0).round() as u64;
    if duration_ms < policy.min_duration_ms || duration_ms > policy.max_duration_ms {
        return Err(LoopSelectionError::InvalidPolicy(
            "fixed loop window falls outside configured duration bounds".into(),
        ));
    }
    let analysis = LoopAnalysis::new(frames, policy.alpha_threshold);
    let report = score_candidate(&analysis, policy, None, start, end, output_frame_indices);
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

fn select_loop_frames_inner(
    frames: &[RgbaImage],
    policy: LoopSelectionPolicy,
    anchor: Option<LoopAnchorPolicyV1>,
) -> Result<LoopSelectionResult, LoopSelectionError> {
    validate_inputs(frames, policy)?;
    let min_span = frames_for_duration(policy.min_duration_ms, policy.candidate_fps).max(1);
    let max_span = frames_for_duration(policy.max_duration_ms, policy.candidate_fps)
        .max(min_span)
        .min(frames.len().saturating_sub(1));
    let target = policy.target_frame_count as usize;
    let mut candidates = Vec::<Candidate>::new();
    let analysis = LoopAnalysis::new(frames, policy.alpha_threshold);

    for start in 0..frames.len().saturating_sub(1) {
        let first_end = start.saturating_add(min_span.max(target));
        let last_end = start.saturating_add(max_span).min(frames.len() - 1);
        if first_end > last_end {
            continue;
        }
        for end in first_end..=last_end {
            let output_frame_indices = sample_excluding_boundary(start, end, target);
            let report =
                score_candidate(&analysis, policy, anchor, start, end, output_frame_indices);
            candidates.push(Candidate { report });
        }
    }

    let report = select_fundamental_candidate(candidates)
        .ok_or(LoopSelectionError::NoCandidatePeriod)?
        .report;
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
    anchor: Option<LoopAnchorPolicyV1>,
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
    let closure_score = (boundary_mask_iou * 0.30
        + boundary_palette_overlap * 0.20
        + boundary_edge_overlap * 0.20
        + anchor_score * 0.15
        + transition_continuity * 0.15)
        .clamp(0.0, 1.0);
    let start_anchor_similarity = anchor.map(|anchor| {
        (1.0 - analysis.frame_difference(anchor.reference_frame, start)).clamp(0.0, 1.0)
    });
    let composite_score = if let Some(similarity) = start_anchor_similarity {
        (closure_score * 0.85 + similarity * 0.15).clamp(0.0, 1.0)
    } else {
        closure_score
    };
    let duration_ms =
        (((end - start) as f64 / policy.candidate_fps as f64) * 1000.0).round() as u64;
    let cadence_score = cadence_score(policy.cadence_profile, duration_ms);
    let period_peak_prominence = period_peak_prominence(analysis, end - start);
    let lag_error = lag_error(analysis, end - start);
    let periodicity_confidence =
        ((1.0 - lag_error).clamp(0.0, 1.0) * 0.70 + period_peak_prominence * 0.30).clamp(0.0, 1.0);
    let timing = animation_timing(
        policy.cadence_profile,
        policy.candidate_fps,
        start,
        duration_ms,
        &output_frame_indices,
        cadence_score,
        period_peak_prominence,
        periodicity_confidence,
    );
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
    if anchor.is_some_and(|anchor| start > anchor.maximum_start_frame) {
        reasons.push("anchor_start_outside_window".into());
    }
    if anchor
        .zip(start_anchor_similarity)
        .is_some_and(|(anchor, similarity)| similarity < anchor.minimum_start_similarity)
    {
        reasons.push("anchor_start_similarity_low".into());
    }
    let anchor_contract_ready = anchor.is_none_or(|anchor| {
        start <= anchor.maximum_start_frame
            && start_anchor_similarity
                .is_some_and(|similarity| similarity >= anchor.minimum_start_similarity)
    });
    let mut verdict = if hard_blocked {
        LoopSelectionVerdict::Blocked
    } else if composite_score >= 0.80
        && boundary_mask_iou >= 0.75
        && anchor_closure_px <= 2.0
        && transition_continuity >= 0.70
        && motion_energy >= policy.minimum_motion_energy
        && anchor_contract_ready
    {
        LoopSelectionVerdict::GameReady
    } else if composite_score >= 0.70
        && boundary_mask_iou >= 0.65
        && anchor_closure_px <= 6.0
        && transition_continuity >= 0.55
        && motion_energy >= policy.minimum_motion_energy
        && anchor_contract_ready
    {
        LoopSelectionVerdict::AwaitingReview
    } else {
        LoopSelectionVerdict::Regenerate
    };
    if !(0.5..=2.0).contains(&timing.playback_speed_ratio) {
        reasons.push("playback_retime_exceeds_safe_ratio".into());
        if verdict != LoopSelectionVerdict::Blocked {
            verdict = LoopSelectionVerdict::Regenerate;
        }
    }
    LoopSelectionReport {
        profile: if anchor.is_some() {
            ANCHORED_LOOP_SELECTION_PROFILE
        } else {
            LOOP_SELECTION_PROFILE
        }
        .into(),
        candidate_frame_count: analysis.frames.len(),
        candidate_fps: policy.candidate_fps,
        selected_start_frame: start,
        selected_end_boundary_frame: end,
        selected_duration_ms: duration_ms,
        output_frame_indices,
        timing: Some(timing),
        output_frame_sha256: Vec::new(),
        boundary_mask_iou,
        boundary_palette_overlap,
        boundary_edge_overlap,
        anchor_closure_px,
        transition_continuity,
        motion_energy,
        composite_score,
        anchor_reference_frame: anchor.map(|anchor| anchor.reference_frame),
        anchor_maximum_start_frame: anchor.map(|anchor| anchor.maximum_start_frame),
        start_anchor_similarity,
        verdict,
        reasons,
    }
}

fn verdict_rank(verdict: LoopSelectionVerdict) -> u8 {
    match verdict {
        LoopSelectionVerdict::GameReady => 3,
        LoopSelectionVerdict::AwaitingReview => 2,
        LoopSelectionVerdict::Regenerate => 1,
        LoopSelectionVerdict::Blocked => 0,
    }
}

fn select_fundamental_candidate(mut candidates: Vec<Candidate>) -> Option<Candidate> {
    let best_verdict = candidates
        .iter()
        .map(|candidate| verdict_rank(candidate.report.verdict))
        .max()?;
    candidates.retain(|candidate| verdict_rank(candidate.report.verdict) == best_verdict);
    let best_composite = candidates
        .iter()
        .map(|candidate| candidate.report.composite_score)
        .max_by(f32::total_cmp)?;
    // A fundamental may have a slightly rougher sampled transition than its
    // two-cycle harmonic. Keep only materially equivalent closed intervals,
    // then prefer the shortest strong period.
    candidates.retain(|candidate| candidate.report.composite_score + 0.05 >= best_composite);
    let best_periodicity = candidates
        .iter()
        .filter_map(|candidate| candidate.report.timing.as_ref())
        .map(|timing| timing.periodicity_confidence)
        .max_by(f32::total_cmp)
        .unwrap_or_default();
    candidates.retain(|candidate| {
        candidate
            .report
            .timing
            .as_ref()
            .map(|timing| timing.periodicity_confidence + 0.05 >= best_periodicity)
            .unwrap_or(best_periodicity <= f32::EPSILON)
    });
    candidates.into_iter().min_by(|left, right| {
        left.report
            .selected_duration_ms
            .cmp(&right.report.selected_duration_ms)
            .then_with(|| {
                right
                    .report
                    .composite_score
                    .total_cmp(&left.report.composite_score)
            })
            .then_with(|| {
                left.report
                    .selected_start_frame
                    .cmp(&right.report.selected_start_frame)
            })
    })
}

fn cadence_score(profile: AnimationCadenceProfile, duration_ms: u64) -> f32 {
    let Some((preferred_min, preferred_max, outer_min, outer_max)) = (match profile {
        AnimationCadenceProfile::Idle => Some((900.0, 2_400.0, 600.0, 3_200.0)),
        AnimationCadenceProfile::Walk => Some((800.0, 1_250.0, 600.0, 1_600.0)),
        AnimationCadenceProfile::Unconstrained => None,
    }) else {
        return 1.0;
    };
    let duration = duration_ms as f32;
    if (preferred_min..=preferred_max).contains(&duration) {
        1.0
    } else if duration < preferred_min {
        ((duration - outer_min) / (preferred_min - outer_min)).clamp(0.0, 1.0)
    } else {
        ((outer_max - duration) / (outer_max - preferred_max)).clamp(0.0, 1.0)
    }
}

fn lag_error(analysis: &LoopAnalysis, lag: usize) -> f32 {
    if lag == 0 || lag >= analysis.frames.len() {
        return 1.0;
    }
    median(
        &(0..analysis.frames.len() - lag)
            .map(|index| analysis.frame_difference(index, index + lag))
            .collect::<Vec<_>>(),
    )
}

fn period_peak_prominence(analysis: &LoopAnalysis, lag: usize) -> f32 {
    let exact = lag_error(analysis, lag);
    let mut neighbors = Vec::new();
    if lag > 1 {
        neighbors.push(lag_error(analysis, lag - 1));
    }
    if lag + 1 < analysis.frames.len() {
        neighbors.push(lag_error(analysis, lag + 1));
    }
    let neighbor = median(&neighbors);
    if neighbor <= f32::EPSILON {
        0.0
    } else {
        ((neighbor - exact) / neighbor).clamp(0.0, 1.0)
    }
}

#[allow(clippy::too_many_arguments)]
fn animation_timing(
    cadence_profile: AnimationCadenceProfile,
    candidate_fps: f32,
    start: usize,
    fundamental_period_ms: u64,
    output_frame_indices: &[usize],
    cadence_score: f32,
    period_peak_prominence: f32,
    periodicity_confidence: f32,
) -> AnimationTimingV1 {
    let output_timestamps_ms = output_frame_indices
        .iter()
        .map(|index| {
            (((index.saturating_sub(start)) as f64 / candidate_fps as f64) * 1000.0).round() as u64
        })
        .collect::<Vec<_>>();
    let source_frame_durations_ms = output_timestamps_ms
        .windows(2)
        .map(|timestamps| timestamps[1].saturating_sub(timestamps[0]).max(1))
        .collect::<Vec<_>>();
    let final_duration = fundamental_period_ms
        .saturating_sub(output_timestamps_ms.last().copied().unwrap_or_default())
        .max(1);
    let mut source_frame_durations_ms = source_frame_durations_ms;
    if !output_timestamps_ms.is_empty() {
        source_frame_durations_ms.push(final_duration);
    }
    let target_playback_duration_ms = match cadence_profile {
        AnimationCadenceProfile::Idle => fundamental_period_ms.clamp(900, 2_400),
        AnimationCadenceProfile::Walk => fundamental_period_ms.clamp(800, 1_250),
        AnimationCadenceProfile::Unconstrained => fundamental_period_ms,
    };
    let mut frame_durations_ms = if fundamental_period_ms == 0 {
        source_frame_durations_ms.clone()
    } else {
        source_frame_durations_ms
            .iter()
            .map(|duration| {
                ((*duration as f64 * target_playback_duration_ms as f64
                    / fundamental_period_ms as f64)
                    .round() as u64)
                    .max(1)
            })
            .collect::<Vec<_>>()
    };
    let prefix = frame_durations_ms
        .iter()
        .take(frame_durations_ms.len().saturating_sub(1))
        .copied()
        .sum::<u64>();
    if let Some(last) = frame_durations_ms.last_mut() {
        *last = target_playback_duration_ms.saturating_sub(prefix).max(1);
    }
    let playback_duration_ms = frame_durations_ms.iter().copied().sum::<u64>();
    let playback_fps = if playback_duration_ms == 0 {
        0.0
    } else {
        output_frame_indices.len() as f32 * 1000.0 / playback_duration_ms as f32
    };
    AnimationTimingV1 {
        profile: ANIMATION_TIMING_PROFILE.into(),
        cadence_profile,
        fundamental_period_ms,
        output_timestamps_ms,
        frame_durations_ms,
        playback_duration_ms,
        playback_fps,
        cadence_score,
        period_peak_prominence,
        periodicity_confidence,
        playback_speed_ratio: if fundamental_period_ms == 0 {
            0.0
        } else {
            playback_duration_ms as f32 / fundamental_period_ms as f32
        },
    }
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
            cadence_profile: AnimationCadenceProfile::Unconstrained,
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
    fn anchored_selection_keeps_direction_locked_idle_near_initial_frame() {
        let frames = vec![
            frame(12, 9),
            frame(12, 1),
            frame(12, 2),
            frame(12, 3),
            frame(12, 9),
            frame(12, 1),
            frame(12, 2),
            frame(12, 3),
            frame(12, 9),
        ];
        let result = select_loop_frames_with_anchor(
            &frames,
            policy(4, 0.001),
            LoopAnchorPolicyV1 {
                reference_frame: 0,
                maximum_start_frame: 1,
                minimum_start_similarity: 0.95,
            },
        )
        .unwrap();

        assert!(result.report.selected_start_frame <= 1);
        assert_eq!(result.report.profile, ANCHORED_LOOP_SELECTION_PROFILE);
        assert_eq!(result.report.anchor_reference_frame, Some(0));
        assert_eq!(result.report.anchor_maximum_start_frame, Some(1));
        assert!(result.report.start_anchor_similarity.unwrap() >= 0.95);
        assert!(!result
            .report
            .reasons
            .contains(&"anchor_start_outside_window".into()));
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
                cadence_profile: AnimationCadenceProfile::Unconstrained,
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

    #[test]
    fn walk_profile_selects_the_fundamental_cycle_instead_of_a_long_harmonic() {
        let frames = (0..31)
            .map(|index| frame(12, (index % 10) as u8))
            .collect::<Vec<_>>();
        let result = select_loop_frames(
            &frames,
            LoopSelectionPolicy::for_animation("walk_right", 8, 10.0, 3_000),
        )
        .unwrap();

        assert_eq!(
            result.report.selected_end_boundary_frame - result.report.selected_start_frame,
            10
        );
        let timing = result.report.timing.expect("animation timing");
        assert_eq!(timing.fundamental_period_ms, 1_000);
        assert_eq!(timing.playback_duration_ms, 1_000);
        assert!((timing.playback_speed_ratio - 1.0).abs() < f32::EPSILON);
        assert_eq!(timing.frame_durations_ms.len(), 8);
        assert_eq!(timing.frame_durations_ms.iter().sum::<u64>(), 1_000);
    }

    #[test]
    fn timing_preserves_non_uniform_source_sample_intervals() {
        let timing = animation_timing(
            AnimationCadenceProfile::Walk,
            10.0,
            0,
            1_000,
            &[0, 1, 2, 3, 5, 6, 7, 8],
            1.0,
            0.8,
            0.9,
        );

        assert_eq!(
            timing.output_timestamps_ms,
            vec![0, 100, 200, 300, 500, 600, 700, 800]
        );
        assert_eq!(
            timing.frame_durations_ms,
            vec![100, 100, 100, 200, 100, 100, 100, 200]
        );
        assert_eq!(timing.playback_duration_ms, 1_000);
        assert!((timing.playback_fps - 8.0).abs() < f32::EPSILON);
    }

    #[test]
    fn timing_slows_a_short_walk_cycle_without_changing_phase_proportions() {
        let timing = animation_timing(
            AnimationCadenceProfile::Walk,
            12.0,
            0,
            667,
            &[0, 1, 2, 3, 4, 5, 6, 7],
            0.335,
            0.9,
            0.95,
        );

        assert_eq!(timing.fundamental_period_ms, 667);
        assert_eq!(timing.playback_duration_ms, 800);
        assert_eq!(timing.frame_durations_ms.iter().sum::<u64>(), 800);
        assert!((timing.playback_speed_ratio - 1.1994).abs() < 0.001);
    }

    #[test]
    fn fixed_semantic_window_preserves_phase_indices_and_excludes_boundary() {
        let frames = (0..13)
            .map(|index| frame(12, (index % 12) as u8))
            .collect::<Vec<_>>();
        let phase_indices = vec![1, 2, 3, 4, 6, 7, 9, 10];
        let result = assess_loop_window(
            &frames,
            LoopSelectionPolicy {
                target_frame_count: 8,
                candidate_fps: 10.0,
                min_duration_ms: 600,
                max_duration_ms: 1_600,
                minimum_motion_energy: 0.001,
                alpha_threshold: 0,
                cadence_profile: AnimationCadenceProfile::Walk,
            },
            1,
            11,
            phase_indices.clone(),
        )
        .unwrap();

        assert_eq!(result.report.output_frame_indices, phase_indices);
        assert!(!result.report.output_frame_indices.contains(&11));
        assert_eq!(result.frames.len(), 8);
    }
}
