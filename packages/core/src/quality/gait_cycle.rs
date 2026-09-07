use std::collections::VecDeque;

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::character_direction::DirectionViewV1;
use crate::frames::{bbox_from_image, FrameBbox};

pub const GAIT_CYCLE_PROFILE: &str = "gait-cycle@1.0.0";
const PHASE_COUNT: usize = 8;
const FEATURE_COLUMNS: usize = 16;
const FEATURE_ROWS: usize = 12;

/// Deterministic policy for finding one complete ordinary-walk cycle in a
/// sequence of pre-aligned RGBA frames. The end boundary proves closure but is
/// never included in the eight exported phases.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GaitCyclePolicy {
    pub direction: DirectionViewV1,
    pub candidate_fps: f32,
    pub min_duration_ms: u64,
    pub max_duration_ms: u64,
    pub preferred_duration_ms: u64,
    pub minimum_boundary_iou: f32,
    pub minimum_motion_energy: f32,
    pub maximum_motion_energy: f32,
    pub maximum_transition_energy: f32,
    pub minimum_contact_excursion: f32,
    pub maximum_foot_lobes: usize,
    #[serde(default)]
    pub alpha_threshold: u8,
}

impl GaitCyclePolicy {
    pub fn ordinary_walk(direction: DirectionViewV1, candidate_fps: f32) -> Self {
        Self {
            direction,
            candidate_fps,
            min_duration_ms: 700,
            max_duration_ms: 1_200,
            preferred_duration_ms: 800,
            minimum_boundary_iou: 0.70,
            minimum_motion_energy: 0.004,
            maximum_motion_energy: 0.45,
            maximum_transition_energy: 0.70,
            minimum_contact_excursion: 0.08,
            maximum_foot_lobes: 2,
            alpha_threshold: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GaitCycleVerdict {
    GameReady,
    Regenerate,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GaitPhaseV1 {
    ContactA,
    ContactARelease,
    PassingA,
    ContactBApproach,
    ContactB,
    ContactBRelease,
    PassingB,
    ContactAApproach,
}

const GAIT_PHASE_LABELS: [GaitPhaseV1; PHASE_COUNT] = [
    GaitPhaseV1::ContactA,
    GaitPhaseV1::ContactARelease,
    GaitPhaseV1::PassingA,
    GaitPhaseV1::ContactBApproach,
    GaitPhaseV1::ContactB,
    GaitPhaseV1::ContactBRelease,
    GaitPhaseV1::PassingB,
    GaitPhaseV1::ContactAApproach,
];

/// Machine-readable proof that the selected interval contains contact A,
/// passing A, the opposing contact, passing B, and a return to contact A.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GaitCycleReport {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub direction: DirectionViewV1,
    pub candidate_frame_count: usize,
    pub candidate_fps: f32,
    pub selected_start_frame: usize,
    pub selected_end_boundary_frame: usize,
    pub source_duration_ms: u64,
    /// Eight monotonically increasing source indices. Positions 0 and 4 are
    /// opposing contacts; positions 2 and 6 are the two passing poses.
    pub output_frame_indices: Vec<usize>,
    pub phase_labels: Vec<GaitPhaseV1>,
    pub source_frame_indices: Vec<usize>,
    pub contact_frames: [usize; 2],
    pub passing_frames: [usize; 2],
    pub phase_order_score: f32,
    pub closure_score: f32,
    pub motion_score: f32,
    pub maximum_foot_lobe_count: usize,
    pub verdict: GaitCycleVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GaitCycleResult {
    pub report: GaitCycleReport,
    pub frames: Vec<RgbaImage>,
}

#[derive(Debug, Error, PartialEq)]
pub enum GaitCycleError {
    #[error("gait-cycle selection requires at least nine candidate frames")]
    TooFewFrames,
    #[error("gait-cycle candidate frames must share one non-zero canvas size")]
    InconsistentCanvas,
    #[error("gait-cycle candidate frame {frame} has no foreground")]
    MissingForeground { frame: usize },
    #[error("gait-cycle policy is invalid: {0}")]
    InvalidPolicy(String),
    #[error("no candidate interval satisfies the gait-cycle duration bounds")]
    NoCandidatePeriod,
}

#[derive(Debug)]
struct FrameAnalysis {
    feature: Vec<f32>,
    alpha_mask: Vec<u64>,
    foot_lobes: usize,
    contact_signal: f32,
}

#[derive(Debug)]
struct Candidate {
    report: GaitCycleReport,
    rank_score: f32,
}

pub fn select_gait_cycle_frames(
    frames: &[RgbaImage],
    policy: GaitCyclePolicy,
) -> Result<GaitCycleResult, GaitCycleError> {
    validate_inputs(frames, policy)?;
    let bboxes = frames
        .iter()
        .enumerate()
        .map(|(index, frame)| {
            let bbox = bbox_from_image(frame, policy.alpha_threshold);
            if bbox.has_foreground() {
                Ok(bbox)
            } else {
                Err(GaitCycleError::MissingForeground { frame: index })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let analysis_bounds = union_bounds(&bboxes);
    let analysis = frames
        .iter()
        .zip(&bboxes)
        .map(|(frame, bbox)| FrameAnalysis {
            feature: lower_body_feature(
                frame,
                analysis_bounds,
                policy.direction,
                policy.alpha_threshold,
            ),
            alpha_mask: pack_mask(
                frame
                    .pixels()
                    .map(|pixel| pixel[3] > policy.alpha_threshold),
            ),
            foot_lobes: foot_lobe_count(frame, *bbox, policy.alpha_threshold.max(48)),
            contact_signal: directional_contact_signal(
                frame,
                *bbox,
                policy.direction,
                policy.alpha_threshold.max(48),
            ),
        })
        .collect::<Vec<_>>();

    let min_span =
        minimum_frames_for_duration(policy.min_duration_ms, policy.candidate_fps).max(PHASE_COUNT);
    let max_span = maximum_frames_for_duration(policy.max_duration_ms, policy.candidate_fps)
        .min(frames.len().saturating_sub(1));
    if min_span > max_span {
        return Err(GaitCycleError::NoCandidatePeriod);
    }

    let mut candidates = Vec::new();
    for start in 0..frames.len().saturating_sub(min_span) {
        let last_end = start.saturating_add(max_span).min(frames.len() - 1);
        for end in start + min_span..=last_end {
            candidates.push(score_candidate(&analysis, policy, start, end));
        }
    }
    let report = select_candidate(candidates, policy.preferred_duration_ms)
        .ok_or(GaitCycleError::NoCandidatePeriod)?;
    let selected = report
        .output_frame_indices
        .iter()
        .map(|index| frames[*index].clone())
        .collect();
    Ok(GaitCycleResult {
        report,
        frames: selected,
    })
}

fn validate_inputs(frames: &[RgbaImage], policy: GaitCyclePolicy) -> Result<(), GaitCycleError> {
    if frames.len() < PHASE_COUNT + 1 {
        return Err(GaitCycleError::TooFewFrames);
    }
    let first = &frames[0];
    if first.width() == 0
        || first.height() == 0
        || frames
            .iter()
            .any(|frame| frame.dimensions() != first.dimensions())
    {
        return Err(GaitCycleError::InconsistentCanvas);
    }
    if !policy.candidate_fps.is_finite()
        || policy.candidate_fps <= 0.0
        || policy.min_duration_ms == 0
        || policy.max_duration_ms < policy.min_duration_ms
        || !(policy.min_duration_ms..=policy.max_duration_ms)
            .contains(&policy.preferred_duration_ms)
        || !(0.0..=1.0).contains(&policy.minimum_boundary_iou)
        || !(0.0..=1.0).contains(&policy.minimum_motion_energy)
        || !(policy.minimum_motion_energy..=1.0).contains(&policy.maximum_motion_energy)
        || !(policy.maximum_motion_energy..=1.0).contains(&policy.maximum_transition_energy)
        || !(0.0..=1.0).contains(&policy.minimum_contact_excursion)
        || policy.maximum_foot_lobes == 0
    {
        return Err(GaitCycleError::InvalidPolicy(
            "fps, durations, thresholds, or foot-lobe limit are out of range".into(),
        ));
    }
    Ok(())
}

fn union_bounds(bboxes: &[FrameBbox]) -> FrameBbox {
    let left = bboxes
        .iter()
        .map(|bbox| bbox.left)
        .min_by(f32::total_cmp)
        .unwrap_or_default();
    let top = bboxes
        .iter()
        .map(|bbox| bbox.top)
        .min_by(f32::total_cmp)
        .unwrap_or_default();
    let right = bboxes
        .iter()
        .map(|bbox| bbox.right)
        .max_by(f32::total_cmp)
        .unwrap_or_default();
    let bottom = bboxes
        .iter()
        .map(|bbox| bbox.bottom)
        .max_by(f32::total_cmp)
        .unwrap_or_default();
    FrameBbox::from_bounds(left, top, right, bottom, 1.0)
}

fn lower_body_feature(
    frame: &RgbaImage,
    bounds: FrameBbox,
    direction: DirectionViewV1,
    alpha_threshold: u8,
) -> Vec<f32> {
    let left = bounds.left.floor().max(0.0) as usize;
    let right = bounds.right.ceil().min(frame.width() as f32) as usize;
    let top = (bounds.top + bounds.height * 0.50).floor().max(0.0) as usize;
    let bottom = bounds.bottom.ceil().min(frame.height() as f32) as usize;
    let width = right.saturating_sub(left).max(1);
    let height = bottom.saturating_sub(top).max(1);
    let mut feature = Vec::with_capacity(FEATURE_COLUMNS * FEATURE_ROWS + 4);
    for row in 0..FEATURE_ROWS {
        let y0 = top + row * height / FEATURE_ROWS;
        let y1 = top + (row + 1) * height / FEATURE_ROWS;
        for column in 0..FEATURE_COLUMNS {
            let source_column = match direction {
                DirectionViewV1::Rear | DirectionViewV1::Left => FEATURE_COLUMNS - 1 - column,
                DirectionViewV1::Front | DirectionViewV1::Right => column,
            };
            let x0 = left + source_column * width / FEATURE_COLUMNS;
            let x1 = left + (source_column + 1) * width / FEATURE_COLUMNS;
            let mut foreground = 0usize;
            let mut area = 0usize;
            for y in y0..y1.max(y0 + 1).min(frame.height() as usize) {
                for x in x0..x1.max(x0 + 1).min(frame.width() as usize) {
                    foreground +=
                        usize::from(frame.get_pixel(x as u32, y as u32)[3] > alpha_threshold);
                    area += 1;
                }
            }
            // The lowest rows carry more gait information than a moving coat
            // hem while retaining enough upper-leg signal for side views.
            let row_weight = 0.65 + 0.70 * row as f32 / (FEATURE_ROWS - 1) as f32;
            let column_weight = match direction {
                DirectionViewV1::Right | DirectionViewV1::Left => {
                    let center_distance =
                        ((column as f32 + 0.5) / FEATURE_COLUMNS as f32 - 0.5).abs() * 2.0;
                    0.70 + center_distance * 0.60
                }
                DirectionViewV1::Front | DirectionViewV1::Rear => 1.0,
            };
            feature.push(if area == 0 {
                0.0
            } else {
                foreground as f32 / area as f32 * row_weight * column_weight
            });
        }
    }

    let midpoint = left + width / 2;
    let lower_top = top + height * 2 / 3;
    let mut left_bottom = lower_top;
    let mut right_bottom = lower_top;
    let mut left_area = 0usize;
    let mut right_area = 0usize;
    for y in lower_top..bottom {
        for x in left..right {
            if frame.get_pixel(x as u32, y as u32)[3] <= alpha_threshold {
                continue;
            }
            if x < midpoint {
                left_bottom = left_bottom.max(y + 1);
                left_area += 1;
            } else {
                right_bottom = right_bottom.max(y + 1);
                right_area += 1;
            }
        }
    }
    let direction_sign = match direction {
        DirectionViewV1::Front | DirectionViewV1::Right => 1.0,
        DirectionViewV1::Rear | DirectionViewV1::Left => -1.0,
    };
    let bottom_delta = (left_bottom as f32 - right_bottom as f32) / height as f32;
    let area_delta =
        (left_area as f32 - right_area as f32) / (left_area + right_area).max(1) as f32;
    // Repeat the semantic signals so a small boot motion is not drowned by a
    // large stable cloak in the raster descriptor.
    feature.extend([
        bottom_delta * direction_sign,
        bottom_delta * direction_sign,
        area_delta * direction_sign,
        area_delta * direction_sign,
    ]);
    feature
}

/// A compact view-aware proxy for which foot owns the contact extreme. Front
/// and rear views use the difference between the two boot baselines. Side
/// views emphasize the leading/trailing lower-body mass, because the boot
/// baselines often overlap in a strict profile. Mirrored views invert the sign
/// so phase A retains one canonical meaning without changing visual geometry.
fn directional_contact_signal(
    frame: &RgbaImage,
    body: FrameBbox,
    direction: DirectionViewV1,
    alpha_threshold: u8,
) -> f32 {
    let left = body.left.floor().max(0.0) as usize;
    let right = body.right.ceil().min(frame.width() as f32) as usize;
    let top = (body.top + body.height * 0.68).floor().max(0.0) as usize;
    let bottom = body.bottom.ceil().min(frame.height() as f32) as usize;
    if left >= right || top >= bottom {
        return 0.0;
    }
    let midpoint = left + (right - left) / 2;
    let mut left_bottom = top;
    let mut right_bottom = top;
    let mut left_area = 0usize;
    let mut right_area = 0usize;
    let mut weighted_x = 0.0f32;
    let mut total = 0usize;
    for y in top..bottom {
        for x in left..right {
            if frame.get_pixel(x as u32, y as u32)[3] <= alpha_threshold {
                continue;
            }
            if x < midpoint {
                left_bottom = left_bottom.max(y + 1);
                left_area += 1;
            } else {
                right_bottom = right_bottom.max(y + 1);
                right_area += 1;
            }
            weighted_x += x as f32;
            total += 1;
        }
    }
    let height = (bottom - top).max(1) as f32;
    let width = (right - left).max(1) as f32;
    let bottom_delta = (left_bottom as f32 - right_bottom as f32) / height;
    let area_delta =
        (left_area as f32 - right_area as f32) / (left_area + right_area).max(1) as f32;
    let centroid_delta = if total == 0 {
        0.0
    } else {
        ((weighted_x / total as f32) - (left as f32 + width / 2.0)) / (width / 2.0)
    };
    match direction {
        DirectionViewV1::Front => bottom_delta * 0.80 + area_delta * 0.20,
        DirectionViewV1::Rear => -(bottom_delta * 0.80 + area_delta * 0.20),
        DirectionViewV1::Right => area_delta * 0.70 + centroid_delta * 0.30,
        DirectionViewV1::Left => -(area_delta * 0.70 + centroid_delta * 0.30),
    }
    .clamp(-1.0, 1.0)
}

fn score_candidate(
    analysis: &[FrameAnalysis],
    policy: GaitCyclePolicy,
    start: usize,
    end: usize,
) -> Candidate {
    let span = end - start;
    let duration_ms = ((span as f64 / policy.candidate_fps as f64) * 1_000.0).round() as u64;
    let boundary_mask_iou = packed_mask_iou(&analysis[start].alpha_mask, &analysis[end].alpha_mask);
    let interior_start = start + (span / 4).max(1);
    let interior_end = start + (span * 3 / 4).max(2);
    let opposing_contact = (interior_start..=interior_end.min(end - 1))
        .max_by(|left, right| {
            let left_signal =
                (analysis[start].contact_signal - analysis[*left].contact_signal).abs();
            let right_signal =
                (analysis[start].contact_signal - analysis[*right].contact_signal).abs();
            left_signal.total_cmp(&right_signal).then_with(|| {
                feature_distance(&analysis[start].feature, &analysis[*left].feature).total_cmp(
                    &feature_distance(&analysis[start].feature, &analysis[*right].feature),
                )
            })
        })
        .unwrap_or(start + span / 2);
    let axis = analysis[opposing_contact]
        .feature
        .iter()
        .zip(&analysis[start].feature)
        .map(|(opposite, origin)| opposite - origin)
        .collect::<Vec<_>>();
    let axis_norm = axis.iter().map(|value| value * value).sum::<f32>();
    let projections = (start..=end)
        .map(|index| {
            if axis_norm <= f32::EPSILON {
                0.0
            } else {
                analysis[index]
                    .feature
                    .iter()
                    .zip(&analysis[start].feature)
                    .zip(&axis)
                    .map(|((value, origin), axis)| (value - origin) * axis)
                    .sum::<f32>()
                    / axis_norm
            }
        })
        .collect::<Vec<_>>();
    let raster_contact_excursion = feature_distance(
        &analysis[start].feature,
        &analysis[opposing_contact].feature,
    );
    let contact_excursion =
        (analysis[start].contact_signal - analysis[opposing_contact].contact_signal).abs();
    let contact_balance = {
        let first_half = opposing_contact - start;
        let second_half = end - opposing_contact;
        1.0 - (first_half as f32 - second_half as f32).abs() / span as f32
    }
    .clamp(0.0, 1.0);
    let contact_midpoint =
        (analysis[start].contact_signal + analysis[opposing_contact].contact_signal) / 2.0;
    // Reserve one source frame on each side of a passing pose. This makes the
    // eight-phase contract structural: a passing frame adjacent to a contact
    // can no longer masquerade as a complete gait cycle.
    let first_passing = closest_contact_signal(
        analysis,
        start + 2,
        opposing_contact.saturating_sub(2),
        contact_midpoint,
    );
    let second_passing = closest_contact_signal(
        analysis,
        opposing_contact + 2,
        end.saturating_sub(2),
        contact_midpoint,
    );
    let passing_score = match (first_passing, second_passing) {
        (Some(first), Some(second)) => {
            let scale = (contact_excursion / 2.0).max(0.001);
            let first_score =
                1.0 - (analysis[first].contact_signal - contact_midpoint).abs() / scale;
            let second_score =
                1.0 - (analysis[second].contact_signal - contact_midpoint).abs() / scale;
            ((first_score + second_score) / 2.0).clamp(0.0, 1.0)
        }
        _ => 0.0,
    };
    let phase_order_score = projection_order_score(&projections, opposing_contact - start);
    let phase_frame_indices = match (first_passing, second_passing) {
        (Some(first), Some(second)) => {
            phase_indices(&projections, start, opposing_contact, first, second, end)
                .filter(|indices| indices.len() == PHASE_COUNT)
        }
        _ => None,
    };
    let transition_energies = (start..end)
        .map(|index| feature_distance(&analysis[index].feature, &analysis[index + 1].feature))
        .collect::<Vec<_>>();
    let motion_energy = median(&transition_energies);
    let maximum_transition_energy = transition_energies
        .iter()
        .copied()
        .max_by(f32::total_cmp)
        .unwrap_or_default();
    let maximum_observed_foot_lobes = analysis[start..end]
        .iter()
        .map(|frame| frame.foot_lobes)
        .max()
        .unwrap_or_default();

    let mut reasons = Vec::new();
    if boundary_mask_iou < policy.minimum_boundary_iou {
        reasons.push("gait_boundary_mismatch".into());
    }
    if motion_energy < policy.minimum_motion_energy
        || raster_contact_excursion < policy.minimum_contact_excursion
        || contact_excursion < policy.minimum_contact_excursion
    {
        reasons.push("gait_motion_too_low".into());
    }
    if motion_energy > policy.maximum_motion_energy
        || maximum_transition_energy > policy.maximum_transition_energy
    {
        reasons.push("gait_motion_excessive".into());
    }
    let mean_contact_signal = analysis[start..end]
        .iter()
        .map(|frame| frame.contact_signal)
        .sum::<f32>()
        / span as f32;
    let contacts_have_opposite_extrema = (analysis[start].contact_signal - mean_contact_signal)
        * (analysis[opposing_contact].contact_signal - mean_contact_signal)
        < 0.0;
    if contact_balance < 0.70 || !contacts_have_opposite_extrema {
        reasons.push("gait_contacts_not_opposed".into());
    }
    if passing_score < 0.70 || phase_frame_indices.is_none() {
        reasons.push("gait_passing_poses_missing".into());
    }
    if phase_order_score < 0.65 {
        reasons.push("gait_phase_order_invalid".into());
    }
    if maximum_observed_foot_lobes > policy.maximum_foot_lobes {
        reasons.push("gait_foot_lobe_count_exceeded".into());
    }

    let motion_score = if motion_energy > policy.maximum_motion_energy
        || maximum_transition_energy > policy.maximum_transition_energy
    {
        0.0
    } else {
        (motion_energy / policy.minimum_motion_energy.max(0.001) / 2.0).clamp(0.0, 1.0)
    };
    let contact_score =
        (contact_excursion / policy.minimum_contact_excursion.max(0.001) / 1.5).clamp(0.0, 1.0);
    let phase_score =
        (contact_balance * 0.25 + passing_score * 0.35 + phase_order_score * 0.40).clamp(0.0, 1.0);
    let composite_score = (boundary_mask_iou * 0.20
        + contact_score * 0.20
        + phase_score * 0.40
        + motion_score * 0.20)
        .clamp(0.0, 1.0);
    let verdict = if maximum_observed_foot_lobes > policy.maximum_foot_lobes {
        GaitCycleVerdict::Blocked
    } else if reasons.is_empty() {
        GaitCycleVerdict::GameReady
    } else {
        GaitCycleVerdict::Regenerate
    };
    let output_frame_indices =
        phase_frame_indices.unwrap_or_else(|| sample_excluding_boundary(start, end, PHASE_COUNT));
    let passing_frames = [
        first_passing.unwrap_or(output_frame_indices[2]),
        second_passing.unwrap_or(output_frame_indices[6]),
    ];
    Candidate {
        report: GaitCycleReport {
            schema_version: "1".into(),
            profile: GAIT_CYCLE_PROFILE.into(),
            animation: animation_for_direction(policy.direction).into(),
            direction: policy.direction,
            candidate_frame_count: analysis.len(),
            candidate_fps: policy.candidate_fps,
            selected_start_frame: start,
            selected_end_boundary_frame: end,
            source_duration_ms: duration_ms,
            output_frame_indices: output_frame_indices.clone(),
            phase_labels: GAIT_PHASE_LABELS.to_vec(),
            source_frame_indices: output_frame_indices,
            contact_frames: [start, opposing_contact],
            passing_frames,
            phase_order_score: phase_score,
            closure_score: boundary_mask_iou,
            motion_score,
            maximum_foot_lobe_count: maximum_observed_foot_lobes,
            verdict,
            reasons,
        },
        rank_score: composite_score,
    }
}

fn animation_for_direction(direction: DirectionViewV1) -> &'static str {
    match direction {
        DirectionViewV1::Front => "walk_down",
        DirectionViewV1::Rear => "walk_up",
        DirectionViewV1::Right => "walk_right",
        DirectionViewV1::Left => "walk_left",
    }
}

fn closest_contact_signal(
    analysis: &[FrameAnalysis],
    first: usize,
    last: usize,
    target: f32,
) -> Option<usize> {
    if first > last || last >= analysis.len() {
        return None;
    }
    (first..=last).min_by(|left, right| {
        (analysis[*left].contact_signal - target)
            .abs()
            .total_cmp(&(analysis[*right].contact_signal - target).abs())
    })
}

fn closest_projection(
    projections: &[f32],
    offset: usize,
    first: usize,
    last: usize,
    target: f32,
) -> Option<usize> {
    if first > last || last.saturating_sub(offset) >= projections.len() {
        return None;
    }
    (first..=last).min_by(|left, right| {
        (projections[*left - offset] - target)
            .abs()
            .total_cmp(&(projections[*right - offset] - target).abs())
    })
}

fn phase_indices(
    projections: &[f32],
    start: usize,
    opposing: usize,
    first_passing: usize,
    second_passing: usize,
    end: usize,
) -> Option<Vec<usize>> {
    let phase_1 = closest_projection(projections, start, start + 1, first_passing - 1, 0.25)?;
    let phase_3 = closest_projection(projections, start, first_passing + 1, opposing - 1, 0.75)?;
    let phase_5 = closest_projection(projections, start, opposing + 1, second_passing - 1, 0.75)?;
    let phase_7 = closest_projection(projections, start, second_passing + 1, end - 1, 0.25)?;
    let indices = vec![
        start,
        phase_1,
        first_passing,
        phase_3,
        opposing,
        phase_5,
        second_passing,
        phase_7,
    ];
    indices
        .windows(2)
        .all(|pair| pair[0] < pair[1])
        .then_some(indices)
}

fn projection_order_score(projections: &[f32], opposing: usize) -> f32 {
    if projections.len() < 3 || opposing == 0 || opposing + 1 >= projections.len() {
        return 0.0;
    }
    let tolerance = 0.12;
    let ascending = projections[..=opposing]
        .windows(2)
        .filter(|pair| pair[1] + tolerance >= pair[0])
        .count();
    let descending = projections[opposing..]
        .windows(2)
        .filter(|pair| pair[1] <= pair[0] + tolerance)
        .count();
    (ascending + descending) as f32 / (projections.len() - 1) as f32
}

fn select_candidate(
    candidates: Vec<Candidate>,
    preferred_duration_ms: u64,
) -> Option<GaitCycleReport> {
    candidates
        .into_iter()
        .max_by(|left, right| {
            verdict_rank(left.report.verdict)
                .cmp(&verdict_rank(right.report.verdict))
                .then_with(|| left.rank_score.total_cmp(&right.rank_score))
                .then_with(|| {
                    let left_distance = left
                        .report
                        .source_duration_ms
                        .abs_diff(preferred_duration_ms);
                    let right_distance = right
                        .report
                        .source_duration_ms
                        .abs_diff(preferred_duration_ms);
                    right_distance.cmp(&left_distance)
                })
                .then_with(|| {
                    right
                        .report
                        .selected_start_frame
                        .cmp(&left.report.selected_start_frame)
                })
        })
        .map(|candidate| candidate.report)
}

fn verdict_rank(verdict: GaitCycleVerdict) -> u8 {
    match verdict {
        GaitCycleVerdict::GameReady => 2,
        GaitCycleVerdict::Regenerate => 1,
        GaitCycleVerdict::Blocked => 0,
    }
}

fn minimum_frames_for_duration(duration_ms: u64, fps: f32) -> usize {
    ((duration_ms as f64 / 1_000.0) * fps as f64).ceil() as usize
}

fn maximum_frames_for_duration(duration_ms: u64, fps: f32) -> usize {
    ((duration_ms as f64 / 1_000.0) * fps as f64).floor() as usize
}

fn sample_excluding_boundary(start: usize, end: usize, count: usize) -> Vec<usize> {
    let span = end - start;
    (0..count)
        .map(|position| start + position * span / count)
        .collect()
}

fn feature_distance(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    (left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right).powi(2))
        .sum::<f32>()
        / left.len() as f32)
        .sqrt()
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

fn foot_lobe_count(frame: &RgbaImage, body: FrameBbox, alpha_threshold: u8) -> usize {
    let left = (body.left - body.width * 0.40).floor().max(0.0) as usize;
    let right = (body.right + body.width * 0.40)
        .ceil()
        .min(frame.width() as f32) as usize;
    let top = (body.top + body.height * 0.84).floor().max(0.0) as usize;
    let bottom = body.bottom.ceil().min(frame.height() as f32) as usize;
    if left >= right || top >= bottom {
        return 0;
    }
    let width = right - left;
    let height = bottom - top;
    let mut foreground = vec![false; width * height];
    for y in top..bottom {
        for x in left..right {
            foreground[(y - top) * width + (x - left)] =
                frame.get_pixel(x as u32, y as u32)[3] > alpha_threshold;
        }
    }
    let minimum_area = ((body.width * body.height * 0.0015).round() as usize).max(6);
    let mut visited = vec![false; foreground.len()];
    let mut lobes = 0usize;
    for start in 0..foreground.len() {
        if !foreground[start] || visited[start] {
            continue;
        }
        visited[start] = true;
        let mut queue = VecDeque::from([start]);
        let mut area = 0usize;
        while let Some(index) = queue.pop_front() {
            area += 1;
            let x = index % width;
            let y = index / width;
            for (neighbor_x, neighbor_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if neighbor_x >= width || neighbor_y >= height {
                    continue;
                }
                let neighbor = neighbor_y * width + neighbor_x;
                if foreground[neighbor] && !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
        lobes += usize::from(area >= minimum_area);
    }
    lobes
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

    fn gait_frame(left_bottom: i32, right_bottom: i32, third_foot: bool) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(48, 48, Rgba([0, 0, 0, 0]));
        for y in 8..29 {
            for x in 18..31 {
                image.put_pixel(x, y, Rgba([120, 80, 50, 255]));
            }
        }
        for y in 27..left_bottom {
            for x in 17..22 {
                image.put_pixel(x, y as u32, Rgba([90, 60, 40, 255]));
            }
        }
        for y in 27..right_bottom {
            for x in 27..32 {
                image.put_pixel(x, y as u32, Rgba([90, 60, 40, 255]));
            }
        }
        if third_foot {
            for y in 36..42 {
                for x in 36..40 {
                    image.put_pixel(x, y, Rgba([90, 60, 40, 255]));
                }
            }
        }
        image
    }

    fn complete_cycle() -> Vec<RgbaImage> {
        [
            (42, 34),
            (42, 36),
            (41, 39),
            (39, 41),
            (34, 42),
            (36, 42),
            (39, 41),
            (41, 39),
            (42, 34),
        ]
        .into_iter()
        .map(|(left, right)| gait_frame(left, right, false))
        .collect()
    }

    fn policy(direction: DirectionViewV1) -> GaitCyclePolicy {
        GaitCyclePolicy {
            candidate_fps: 8.0,
            min_duration_ms: 900,
            max_duration_ms: 1_100,
            preferred_duration_ms: 1_000,
            minimum_motion_energy: 0.003,
            minimum_contact_excursion: 0.04,
            ..GaitCyclePolicy::ordinary_walk(direction, 8.0)
        }
    }

    #[test]
    fn finds_two_contacts_two_passings_and_eight_ordered_phases() {
        let frames = complete_cycle();
        let result = select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)).unwrap();

        assert_eq!(result.report.profile, GAIT_CYCLE_PROFILE);
        assert_eq!(result.report.verdict, GaitCycleVerdict::GameReady);
        assert_eq!(result.report.contact_frames, [0, 4]);
        assert_eq!(result.report.passing_frames, [2, 6]);
        assert_eq!(
            result.report.output_frame_indices,
            (0..8).collect::<Vec<_>>()
        );
        assert_eq!(result.report.phase_labels, GAIT_PHASE_LABELS);
        assert_eq!(
            result.report.source_frame_indices,
            result.report.output_frame_indices
        );
        assert_eq!(result.frames.len(), PHASE_COUNT);
        assert!(result.report.reasons.is_empty());
    }

    #[test]
    fn rejects_a_static_fake_loop() {
        let still = gait_frame(42, 34, false);
        let frames = vec![still; 9];
        let result = select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)).unwrap();

        assert_eq!(result.report.verdict, GaitCycleVerdict::Regenerate);
        assert!(result
            .report
            .reasons
            .contains(&"gait_motion_too_low".into()));
    }

    #[test]
    fn blocks_a_residual_third_foot_lobe() {
        let frames = complete_cycle()
            .into_iter()
            .enumerate()
            .map(|(index, mut frame)| {
                if index == 3 {
                    for y in 36..42 {
                        for x in 36..40 {
                            frame.put_pixel(x, y, Rgba([90, 60, 40, 255]));
                        }
                    }
                }
                frame
            })
            .collect::<Vec<_>>();
        let result = select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)).unwrap();

        assert_eq!(result.report.verdict, GaitCycleVerdict::Blocked);
        assert!(result
            .report
            .reasons
            .contains(&"gait_foot_lobe_count_exceeded".into()));
    }

    #[test]
    fn rejects_a_closed_half_cycle_without_two_passing_poses() {
        let frames = [
            (42, 34),
            (42, 34),
            (41, 35),
            (36, 40),
            (34, 42),
            (34, 42),
            (36, 40),
            (41, 35),
            (42, 34),
        ]
        .into_iter()
        .map(|(left, right)| gait_frame(left, right, false))
        .collect::<Vec<_>>();
        let result = select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)).unwrap();

        assert_ne!(result.report.verdict, GaitCycleVerdict::GameReady);
        assert!(result.report.reasons.iter().any(|reason| {
            reason == "gait_passing_poses_missing" || reason == "gait_phase_order_invalid"
        }));
    }

    #[test]
    fn selects_a_complete_cycle_between_noisy_frames() {
        let mut frames = vec![gait_frame(30, 43, false)];
        frames.extend(complete_cycle());
        frames.push(gait_frame(43, 30, false));
        let result = select_gait_cycle_frames(&frames, policy(DirectionViewV1::Right)).unwrap();

        assert_eq!(result.report.verdict, GaitCycleVerdict::GameReady);
        assert_eq!(result.report.selected_start_frame, 1);
        assert_eq!(result.report.selected_end_boundary_frame, 9);
        assert_eq!(result.report.direction, DirectionViewV1::Right);
    }

    #[test]
    fn serializes_a_versioned_direction_aware_report() {
        let result =
            select_gait_cycle_frames(&complete_cycle(), policy(DirectionViewV1::Rear)).unwrap();
        let json = serde_json::to_value(&result.report).unwrap();

        assert_eq!(json["profile"], GAIT_CYCLE_PROFILE);
        assert_eq!(json["schemaVersion"], "1");
        assert_eq!(json["animation"], "walk_up");
        assert_eq!(json["direction"], "rear");
        assert_eq!(json["outputFrameIndices"].as_array().unwrap().len(), 8);
        assert_eq!(json["phaseLabels"].as_array().unwrap().len(), 8);
        assert_eq!(json["sourceFrameIndices"].as_array().unwrap().len(), 8);
    }

    #[test]
    fn contact_proxy_distinguishes_front_rear_and_mirrored_side_views() {
        let frame = gait_frame(42, 34, false);
        let bbox = bbox_from_image(&frame, 0);
        let front = directional_contact_signal(&frame, bbox, DirectionViewV1::Front, 0);
        let rear = directional_contact_signal(&frame, bbox, DirectionViewV1::Rear, 0);
        let right = directional_contact_signal(&frame, bbox, DirectionViewV1::Right, 0);
        let left = directional_contact_signal(&frame, bbox, DirectionViewV1::Left, 0);

        assert!(front > 0.0);
        assert!((front + rear).abs() < f32::EPSILON);
        assert!(right > 0.0);
        assert!((right + left).abs() < f32::EPSILON);
        assert!((front - right).abs() > 0.01);
    }

    #[test]
    fn rejects_upper_body_flicker_without_foot_contact_motion() {
        let mut frames = vec![gait_frame(42, 34, false); 9];
        for (index, frame) in frames.iter_mut().enumerate().take(8).skip(1) {
            for y in 8..16 {
                for x in 8..8 + index as u32 {
                    frame.put_pixel(x, y, Rgba([220, 40, 200, 255]));
                }
            }
        }
        let result = select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)).unwrap();

        assert_eq!(result.report.verdict, GaitCycleVerdict::Regenerate);
        assert!(result
            .report
            .reasons
            .contains(&"gait_motion_too_low".into()));
        assert!(result
            .report
            .reasons
            .contains(&"gait_contacts_not_opposed".into()));
    }

    #[test]
    fn rejects_excessive_lower_body_motion() {
        let mut strict = policy(DirectionViewV1::Front);
        strict.maximum_motion_energy = 0.10;
        strict.maximum_transition_energy = 0.20;
        let result = select_gait_cycle_frames(&complete_cycle(), strict).unwrap();

        assert_eq!(result.report.verdict, GaitCycleVerdict::Regenerate);
        assert!(result
            .report
            .reasons
            .contains(&"gait_motion_excessive".into()));
    }

    #[test]
    fn maximum_duration_uses_floor_not_an_out_of_range_ceil() {
        let mut frames = complete_cycle();
        frames.push(gait_frame(42, 36, false));
        let result = select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)).unwrap();

        assert!(result.report.source_duration_ms <= 1_100);
        assert_eq!(result.report.selected_end_boundary_frame, 8);
    }

    #[test]
    fn requires_a_shared_canvas_and_foreground() {
        let mut frames = complete_cycle();
        frames[4] = RgbaImage::new(24, 24);
        assert_eq!(
            select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)),
            Err(GaitCycleError::InconsistentCanvas)
        );

        let mut frames = complete_cycle();
        frames[4] = RgbaImage::new(48, 48);
        assert_eq!(
            select_gait_cycle_frames(&frames, policy(DirectionViewV1::Front)),
            Err(GaitCycleError::MissingForeground { frame: 4 })
        );
    }
}
