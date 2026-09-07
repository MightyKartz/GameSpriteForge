use std::collections::BTreeSet;

use image::{imageops::FilterType, RgbaImage};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SOURCE_CYCLE_SAMPLING_PROFILE: &str = "source-cycle-sampling@1.0.0";
pub const SOURCE_CYCLE_SAMPLING_PROFILE_24: &str = "source-cycle-sampling@1.1.0";
const MAXIMUM_HOLD_RATIO: f32 = 1.80;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCycleSamplingPolicyV1 {
    pub minimum_output_frames: usize,
    pub maximum_output_frames: usize,
    pub reconstruction_error_threshold: f32,
}

impl Default for SourceCycleSamplingPolicyV1 {
    fn default() -> Self {
        Self {
            minimum_output_frames: 8,
            maximum_output_frames: 12,
            // Calibrated against the deterministic V8 biped fixture after all
            // eight mandatory contact/passing phases are locked. Values above
            // 12% still fail closed even at the twelve-frame ceiling.
            reconstruction_error_threshold: 0.12,
        }
    }
}

impl SourceCycleSamplingPolicyV1 {
    pub const fn production_up_to_24() -> Self {
        Self {
            minimum_output_frames: 8,
            maximum_output_frames: 24,
            reconstruction_error_threshold: 0.12,
        }
    }

    pub const fn reviewed_exact_24() -> Self {
        Self {
            minimum_output_frames: 24,
            maximum_output_frames: 24,
            reconstruction_error_threshold: 0.12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCycleSamplingReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub selected_start_frame: usize,
    pub selected_end_boundary_frame: usize,
    pub source_cycle_frame_count: usize,
    pub source_timestamps_ms: Vec<u64>,
    pub mandatory_frame_indices: Vec<usize>,
    pub output_frame_indices: Vec<usize>,
    pub output_timestamps_ms: Vec<u64>,
    pub output_frame_count: usize,
    pub reconstruction_error_threshold: f32,
    pub normalized_reconstruction_error: f32,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceCycleSamplingResult {
    pub report: SourceCycleSamplingReportV1,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SourceCycleSamplingError {
    #[error("source cycle sampling requires a valid [start, end-boundary) interval")]
    InvalidInterval,
    #[error("source cycle sampling requires one timestamp for every candidate frame")]
    InvalidTimestamps,
    #[error("source cycle sampling policy must request between 2 and 24 output frames")]
    InvalidPolicy,
    #[error("source cycle contains fewer frames than the minimum output count")]
    TooFewFrames,
    #[error("mandatory source-cycle frame lies outside [start, end-boundary)")]
    MandatoryFrameOutsideCycle,
    #[error(
        "source cycle cannot meet the reconstruction budget with {output_frames} frames: normalized error {normalized_error_ppm}ppm exceeds {threshold_ppm}ppm"
    )]
    ReconstructionBudgetExceeded {
        output_frames: usize,
        normalized_error_ppm: u32,
        threshold_ppm: u32,
    },
    #[error(
        "source cycle cannot meet the sparse-playback cadence budget with {output_frames} frames: maximum hold ratio {maximum_hold_ratio_ppm}ppm exceeds {threshold_ppm}ppm"
    )]
    CadenceBudgetExceeded {
        output_frames: usize,
        maximum_hold_ratio_ppm: u32,
        threshold_ppm: u32,
    },
    #[error("source cycle sampling report is internally inconsistent")]
    InvalidReport,
}

/// Select original decoded poses only after a complete source cycle has been
/// found. The closure boundary participates in reconstruction scoring but is
/// never exported, preventing a duplicated first pose at the Godot loop seam.
pub fn sample_source_cycle_frames(
    frames: &[RgbaImage],
    timestamps_ms: &[u64],
    start: usize,
    end_boundary: usize,
    mandatory_frame_indices: &[usize],
    policy: SourceCycleSamplingPolicyV1,
) -> Result<SourceCycleSamplingResult, SourceCycleSamplingError> {
    validate(
        frames,
        timestamps_ms,
        start,
        end_boundary,
        mandatory_frame_indices,
        policy,
    )?;

    let features = frames[start..=end_boundary]
        .iter()
        .map(frame_feature)
        .collect::<Vec<_>>();
    let local_timestamps = &timestamps_ms[start..=end_boundary];
    let local_mandatory = mandatory_frame_indices
        .iter()
        .map(|index| index - start)
        .collect::<BTreeSet<_>>();
    let source_count = end_boundary - start;
    let counts = [8usize, 10, 12, 16, 24]
        .into_iter()
        .filter(|count| {
            *count >= policy.minimum_output_frames
                && *count <= policy.maximum_output_frames
                && *count <= source_count
        })
        .collect::<Vec<_>>();

    let motion_scale = cycle_motion_scale(&features).max(1.0e-6);
    let mut selected_choice = None;
    for count in counts.iter().copied() {
        if count < local_mandatory.len() + usize::from(!local_mandatory.contains(&0)) {
            continue;
        }
        let selected = select_for_count(
            &features,
            local_timestamps,
            source_count,
            &local_mandatory,
            count,
        );
        let selected_set = selected.iter().copied().collect::<BTreeSet<_>>();
        let error = reconstruction_error(&features, local_timestamps, source_count, &selected_set)
            / motion_scale;
        let maximum_hold_ratio =
            sparse_playback_maximum_hold_ratio(local_timestamps, source_count, &selected_set);
        let reconstruction_satisfies = error <= policy.reconstruction_error_threshold;
        let cadence_satisfies = maximum_hold_ratio <= MAXIMUM_HOLD_RATIO;
        let satisfies = reconstruction_satisfies && cadence_satisfies;
        selected_choice = Some((
            selected,
            error,
            maximum_hold_ratio,
            reconstruction_satisfies,
            cadence_satisfies,
        ));
        if satisfies {
            break;
        }
    }
    let (selected, error, maximum_hold_ratio, reconstruction_satisfies, cadence_satisfies) =
        selected_choice.ok_or(SourceCycleSamplingError::TooFewFrames)?;
    if !reconstruction_satisfies {
        return Err(SourceCycleSamplingError::ReconstructionBudgetExceeded {
            output_frames: selected.len(),
            normalized_error_ppm: (error * 1_000_000.0).round().max(0.0) as u32,
            threshold_ppm: (policy.reconstruction_error_threshold * 1_000_000.0)
                .round()
                .max(0.0) as u32,
        });
    }
    if !cadence_satisfies {
        return Err(SourceCycleSamplingError::CadenceBudgetExceeded {
            output_frames: selected.len(),
            maximum_hold_ratio_ppm: (maximum_hold_ratio * 1_000_000.0).round().max(0.0) as u32,
            threshold_ppm: (MAXIMUM_HOLD_RATIO * 1_000_000.0).round() as u32,
        });
    }
    let output_frame_indices = selected
        .iter()
        .map(|index| index + start)
        .collect::<Vec<_>>();
    let output_timestamps_ms = output_frame_indices
        .iter()
        .map(|index| timestamps_ms[*index])
        .collect::<Vec<_>>();
    let decision = if output_frame_indices.len() == policy.minimum_output_frames {
        "minimum_count_sufficient"
    } else if output_frame_indices.len() < policy.maximum_output_frames {
        "promoted_for_motion_complexity"
    } else {
        "maximum_count_required"
    };

    Ok(SourceCycleSamplingResult {
        report: SourceCycleSamplingReportV1 {
            schema_version: "1".into(),
            profile: if policy.maximum_output_frames > 12 {
                SOURCE_CYCLE_SAMPLING_PROFILE_24.into()
            } else {
                SOURCE_CYCLE_SAMPLING_PROFILE.into()
            },
            selected_start_frame: start,
            selected_end_boundary_frame: end_boundary,
            source_cycle_frame_count: source_count,
            source_timestamps_ms: timestamps_ms[start..=end_boundary].to_vec(),
            mandatory_frame_indices: mandatory_frame_indices.to_vec(),
            output_frame_count: output_frame_indices.len(),
            output_frame_indices,
            output_timestamps_ms,
            reconstruction_error_threshold: policy.reconstruction_error_threshold,
            normalized_reconstruction_error: error,
            decision: decision.into(),
        },
    })
}

pub fn validate_source_cycle_sampling_report(
    report: &SourceCycleSamplingReportV1,
) -> Result<(), SourceCycleSamplingError> {
    let legacy_profile = report.profile == SOURCE_CYCLE_SAMPLING_PROFILE;
    let modern_profile = report.profile == SOURCE_CYCLE_SAMPLING_PROFILE_24;
    let allowed_count = if legacy_profile {
        matches!(report.output_frame_count, 8 | 10 | 12)
    } else {
        modern_profile && matches!(report.output_frame_count, 8 | 10 | 12 | 16 | 24)
    };
    let output_indices_valid = report.output_frame_indices.len() == report.output_frame_count
        && report.output_frame_indices.first().copied() == Some(report.selected_start_frame)
        && report
            .output_frame_indices
            .windows(2)
            .all(|pair| pair[0] < pair[1])
        && report.output_frame_indices.iter().all(|index| {
            *index >= report.selected_start_frame && *index < report.selected_end_boundary_frame
        });
    let source_count = report
        .selected_end_boundary_frame
        .saturating_sub(report.selected_start_frame);
    let source_timestamps_valid = report.source_cycle_frame_count == source_count
        && report.source_timestamps_ms.len() == source_count + 1
        && report
            .source_timestamps_ms
            .windows(2)
            .all(|pair| pair[0] < pair[1]);
    let output_timestamps_valid = report.output_timestamps_ms.len() == report.output_frame_count
        && report
            .output_frame_indices
            .iter()
            .zip(&report.output_timestamps_ms)
            .all(|(index, timestamp)| {
                report
                    .source_timestamps_ms
                    .get(index - report.selected_start_frame)
                    == Some(timestamp)
            });
    let mandatory_valid = report
        .mandatory_frame_indices
        .windows(2)
        .all(|pair| pair[0] < pair[1])
        && report.mandatory_frame_indices.iter().all(|index| {
            *index >= report.selected_start_frame
                && *index < report.selected_end_boundary_frame
                && report.output_frame_indices.contains(index)
        });
    let decision_valid = if legacy_profile {
        matches!(
            (report.output_frame_count, report.decision.as_str()),
            (8, "minimum_count_sufficient")
                | (10, "promoted_for_motion_complexity")
                | (12, "maximum_count_required")
        )
    } else {
        matches!(
            report.decision.as_str(),
            "minimum_count_sufficient"
                | "promoted_for_motion_complexity"
                | "maximum_count_required"
        )
    };
    let error_valid = report.reconstruction_error_threshold.is_finite()
        && report.reconstruction_error_threshold >= 0.0
        && report.normalized_reconstruction_error.is_finite()
        && report.normalized_reconstruction_error <= report.reconstruction_error_threshold;
    if report.schema_version != "1"
        || (!legacy_profile && !modern_profile)
        || report.selected_start_frame >= report.selected_end_boundary_frame
        || !allowed_count
        || !output_indices_valid
        || !source_timestamps_valid
        || !output_timestamps_valid
        || !mandatory_valid
        || !decision_valid
        || !error_valid
    {
        return Err(SourceCycleSamplingError::InvalidReport);
    }
    Ok(())
}

fn validate(
    frames: &[RgbaImage],
    timestamps_ms: &[u64],
    start: usize,
    end_boundary: usize,
    mandatory_frame_indices: &[usize],
    policy: SourceCycleSamplingPolicyV1,
) -> Result<(), SourceCycleSamplingError> {
    if start >= end_boundary || end_boundary >= frames.len() {
        return Err(SourceCycleSamplingError::InvalidInterval);
    }
    if timestamps_ms.len() != frames.len()
        || timestamps_ms.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(SourceCycleSamplingError::InvalidTimestamps);
    }
    if ![8, 10, 12, 16, 24].contains(&policy.minimum_output_frames)
        || ![8, 10, 12, 16, 24].contains(&policy.maximum_output_frames)
        || (policy.maximum_output_frames > 12 && policy.maximum_output_frames != 24)
        || policy.minimum_output_frames > policy.maximum_output_frames
        || !policy.reconstruction_error_threshold.is_finite()
        || policy.reconstruction_error_threshold < 0.0
    {
        return Err(SourceCycleSamplingError::InvalidPolicy);
    }
    if end_boundary - start < policy.minimum_output_frames {
        return Err(SourceCycleSamplingError::TooFewFrames);
    }
    if mandatory_frame_indices
        .iter()
        .any(|index| *index < start || *index >= end_boundary)
    {
        return Err(SourceCycleSamplingError::MandatoryFrameOutsideCycle);
    }
    Ok(())
}

fn frame_feature(frame: &RgbaImage) -> Vec<f32> {
    const SIZE: u32 = 16;
    let resized = image::imageops::resize(frame, SIZE, SIZE, FilterType::Triangle);
    let mut feature = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for pixel in resized.pixels() {
        let alpha = pixel[3] as f32 / 255.0;
        feature.extend([
            alpha,
            pixel[0] as f32 / 255.0 * alpha,
            pixel[1] as f32 / 255.0 * alpha,
            pixel[2] as f32 / 255.0 * alpha,
        ]);
    }
    feature
}

fn feature_distance(left: &[f32], right: &[f32]) -> f32 {
    (left
        .iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum::<f32>()
        / left.len().max(1) as f32)
        .sqrt()
}

fn cycle_motion_scale(features: &[Vec<f32>]) -> f32 {
    let mut maximum = 0.0f32;
    for left in 0..features.len() {
        for right in left + 1..features.len() {
            maximum = maximum.max(feature_distance(&features[left], &features[right]));
        }
    }
    maximum
}

fn sparse_playback_maximum_hold_ratio(
    timestamps_ms: &[u64],
    end_boundary: usize,
    selected: &BTreeSet<usize>,
) -> f32 {
    let mut selected_timestamps = selected
        .iter()
        .filter_map(|index| timestamps_ms.get(*index).copied())
        .collect::<Vec<_>>();
    let Some(boundary_timestamp) = timestamps_ms.get(end_boundary).copied() else {
        return f32::INFINITY;
    };
    selected_timestamps.push(boundary_timestamp);
    let holds = selected_timestamps
        .windows(2)
        .map(|window| window[1].saturating_sub(window[0]))
        .collect::<Vec<_>>();
    let total = holds.iter().sum::<u64>();
    let average = total as f32 / holds.len().max(1) as f32;
    if average <= f32::EPSILON {
        return f32::INFINITY;
    }
    holds.iter().copied().max().unwrap_or_default() as f32 / average
}

fn select_for_count(
    features: &[Vec<f32>],
    timestamps_ms: &[u64],
    end_boundary: usize,
    mandatory: &BTreeSet<usize>,
    count: usize,
) -> Vec<usize> {
    let mut selected = arc_length_seed(features, timestamps_ms, end_boundary, count)
        .into_iter()
        .chain(mandatory.iter().copied())
        .chain(std::iter::once(0))
        .collect::<BTreeSet<_>>();

    while selected.len() > count {
        let removable = selected
            .iter()
            .copied()
            .filter(|index| *index != 0 && !mandatory.contains(index))
            .collect::<Vec<_>>();
        let Some(remove) = removable.into_iter().min_by(|left, right| {
            let mut without_left = selected.clone();
            without_left.remove(left);
            let mut without_right = selected.clone();
            without_right.remove(right);
            reconstruction_error(features, timestamps_ms, end_boundary, &without_left)
                .total_cmp(&reconstruction_error(
                    features,
                    timestamps_ms,
                    end_boundary,
                    &without_right,
                ))
                .then_with(|| right.cmp(left))
        }) else {
            break;
        };
        selected.remove(&remove);
    }

    while selected.len() < count {
        let candidate = (0..end_boundary)
            .filter(|index| !selected.contains(index))
            .max_by(|left, right| {
                point_reconstruction_error(features, timestamps_ms, end_boundary, &selected, *left)
                    .total_cmp(&point_reconstruction_error(
                        features,
                        timestamps_ms,
                        end_boundary,
                        &selected,
                        *right,
                    ))
                    .then_with(|| right.cmp(left))
            });
        let Some(candidate) = candidate else {
            break;
        };
        selected.insert(candidate);
    }
    selected.into_iter().collect()
}

fn arc_length_seed(
    features: &[Vec<f32>],
    timestamps_ms: &[u64],
    end_boundary: usize,
    count: usize,
) -> Vec<usize> {
    let mut cumulative = vec![0.0f32; end_boundary + 1];
    for index in 1..=end_boundary {
        cumulative[index] =
            cumulative[index - 1] + feature_distance(&features[index - 1], &features[index]);
    }
    let total = cumulative[end_boundary];
    (0..count)
        .map(|position| {
            if total <= f32::EPSILON {
                let start = timestamps_ms[0];
                let duration = timestamps_ms[end_boundary].saturating_sub(start);
                let target = start + duration * position as u64 / count as u64;
                nearest_timestamp(timestamps_ms, end_boundary, target)
            } else {
                let target = total * position as f32 / count as f32;
                (0..end_boundary)
                    .min_by(|left, right| {
                        (cumulative[*left] - target)
                            .abs()
                            .total_cmp(&(cumulative[*right] - target).abs())
                            .then_with(|| left.cmp(right))
                    })
                    .unwrap_or(0)
            }
        })
        .collect()
}

fn nearest_timestamp(timestamps_ms: &[u64], end_boundary: usize, target: u64) -> usize {
    (0..end_boundary)
        .min_by_key(|index| timestamps_ms[*index].abs_diff(target))
        .unwrap_or(0)
}

fn reconstruction_error(
    features: &[Vec<f32>],
    timestamps_ms: &[u64],
    end_boundary: usize,
    selected: &BTreeSet<usize>,
) -> f32 {
    (0..end_boundary)
        .map(|index| {
            point_reconstruction_error(features, timestamps_ms, end_boundary, selected, index)
        })
        .fold(0.0, f32::max)
}

fn point_reconstruction_error(
    features: &[Vec<f32>],
    timestamps_ms: &[u64],
    end_boundary: usize,
    selected: &BTreeSet<usize>,
    index: usize,
) -> f32 {
    if selected.contains(&index) {
        return 0.0;
    }
    let left = selected.range(..index).next_back().copied().unwrap_or(0);
    let right = selected
        .range(index + 1..)
        .next()
        .copied()
        .unwrap_or(end_boundary);
    let span = timestamps_ms[right]
        .saturating_sub(timestamps_ms[left])
        .max(1) as f32;
    let offset = timestamps_ms[index].saturating_sub(timestamps_ms[left]) as f32;
    let ratio = (offset / span).clamp(0.0, 1.0);
    let predicted = features[left]
        .iter()
        .zip(&features[right])
        .map(|(left, right)| left + (right - left) * ratio)
        .collect::<Vec<_>>();
    feature_distance(&features[index], &predicted)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn cycle_frames(complex: bool) -> (Vec<RgbaImage>, Vec<u64>) {
        let frames = (0..=24)
            .map(|index| {
                let phase = if index == 24 { 0 } else { index };
                let mut frame = RgbaImage::new(64, 64);
                let triangle = if phase <= 12 { phase } else { 24 - phase };
                let x = 8 + triangle;
                let y = if complex && phase % 3 == 1 { 22 } else { 24 };
                for pixel_y in y..y + 16 {
                    for pixel_x in x..x + 12 {
                        frame.put_pixel(pixel_x, pixel_y, Rgba([120, 80, 180, 255]));
                    }
                }
                if complex && phase % 2 == 0 {
                    frame.put_pixel(50, 40, Rgba([255, 255, 255, 255]));
                }
                frame
            })
            .collect::<Vec<_>>();
        let timestamps = (0..=24).map(|index| index * 42).collect();
        (frames, timestamps)
    }

    #[test]
    fn simple_native_cycle_reduces_to_eight_original_frames() {
        let (frames, timestamps) = cycle_frames(false);
        let result = sample_source_cycle_frames(
            &frames,
            &timestamps,
            0,
            24,
            &[],
            SourceCycleSamplingPolicyV1 {
                reconstruction_error_threshold: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.report.output_frame_count, 8);
        assert_eq!(result.report.output_frame_indices[0], 0);
        assert!(!result.report.output_frame_indices.contains(&24));
        assert!(result
            .report
            .output_frame_indices
            .windows(2)
            .all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn complex_cycle_promotes_to_twelve_when_error_is_strict() {
        let (frames, timestamps) = cycle_frames(true);
        let features = frames.iter().map(frame_feature).collect::<Vec<_>>();
        let motion_scale = cycle_motion_scale(&features);
        let ten = select_for_count(&features, &timestamps, 24, &BTreeSet::new(), 10)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let twelve = select_for_count(&features, &timestamps, 24, &BTreeSet::new(), 12)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let ten_error = reconstruction_error(&features, &timestamps, 24, &ten) / motion_scale;
        let twelve_error = reconstruction_error(&features, &timestamps, 24, &twelve) / motion_scale;
        assert!(twelve_error < ten_error);
        let result = sample_source_cycle_frames(
            &frames,
            &timestamps,
            0,
            24,
            &[],
            SourceCycleSamplingPolicyV1 {
                reconstruction_error_threshold: (ten_error + twelve_error) / 2.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.report.output_frame_count, 12);
        assert_eq!(result.report.decision, "maximum_count_required");
    }

    #[test]
    fn cycle_fails_closed_when_twelve_frames_exceed_the_budget() {
        let (frames, timestamps) = cycle_frames(true);
        assert!(matches!(
            sample_source_cycle_frames(
                &frames,
                &timestamps,
                0,
                24,
                &[],
                SourceCycleSamplingPolicyV1 {
                    reconstruction_error_threshold: 0.0,
                    ..Default::default()
                },
            )
            .unwrap_err(),
            SourceCycleSamplingError::ReconstructionBudgetExceeded { .. }
        ));
    }

    #[test]
    fn nine_and_eleven_source_frames_never_export_nine_or_eleven() {
        let (frames, timestamps) = cycle_frames(false);
        for end_boundary in [9, 11] {
            let result = sample_source_cycle_frames(
                &frames,
                &timestamps,
                0,
                end_boundary,
                &[],
                SourceCycleSamplingPolicyV1 {
                    reconstruction_error_threshold: 1.0,
                    ..Default::default()
                },
            )
            .unwrap();
            assert!(matches!(result.report.output_frame_count, 8 | 10 | 12));
            assert_ne!(result.report.output_frame_count, end_boundary);
        }
    }

    #[test]
    fn semantic_phase_indices_are_never_removed() {
        let (frames, timestamps) = cycle_frames(false);
        let mandatory = [0, 3, 6, 9, 12, 15, 18, 21];
        let result = sample_source_cycle_frames(
            &frames,
            &timestamps,
            0,
            24,
            &mandatory,
            SourceCycleSamplingPolicyV1 {
                reconstruction_error_threshold: 1.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.report.output_frame_indices, mandatory);
    }

    #[test]
    fn reviewed_exact_twenty_four_preserves_every_native_pose_and_validates() {
        let (frames, timestamps) = cycle_frames(true);
        let result = sample_source_cycle_frames(
            &frames,
            &timestamps,
            0,
            24,
            &[],
            SourceCycleSamplingPolicyV1::reviewed_exact_24(),
        )
        .unwrap();

        assert_eq!(result.report.profile, SOURCE_CYCLE_SAMPLING_PROFILE_24);
        assert_eq!(result.report.output_frame_count, 24);
        assert_eq!(
            result.report.output_frame_indices,
            (0..24).collect::<Vec<_>>()
        );
        validate_source_cycle_sampling_report(&result.report).unwrap();
    }

    #[test]
    fn closure_boundary_cannot_be_mandatory_output() {
        let (frames, timestamps) = cycle_frames(false);
        assert_eq!(
            sample_source_cycle_frames(
                &frames,
                &timestamps,
                0,
                24,
                &[24],
                SourceCycleSamplingPolicyV1::default(),
            )
            .unwrap_err(),
            SourceCycleSamplingError::MandatoryFrameOutsideCycle
        );
    }

    #[test]
    fn report_validator_rejects_a_missing_mandatory_phase() {
        let (frames, timestamps) = cycle_frames(false);
        let mut report = sample_source_cycle_frames(
            &frames,
            &timestamps,
            0,
            24,
            &[3],
            SourceCycleSamplingPolicyV1 {
                reconstruction_error_threshold: 1.0,
                ..Default::default()
            },
        )
        .unwrap()
        .report;
        validate_source_cycle_sampling_report(&report).unwrap();
        report.output_frame_indices.retain(|index| *index != 3);
        report.output_frame_count = report.output_frame_indices.len();
        assert_eq!(
            validate_source_cycle_sampling_report(&report).unwrap_err(),
            SourceCycleSamplingError::InvalidReport
        );
    }

    #[test]
    fn report_validator_requires_the_cycle_start_as_the_first_output() {
        let (frames, timestamps) = cycle_frames(false);
        let mut report = sample_source_cycle_frames(
            &frames,
            &timestamps,
            0,
            24,
            &[],
            SourceCycleSamplingPolicyV1 {
                reconstruction_error_threshold: 1.0,
                ..Default::default()
            },
        )
        .unwrap()
        .report;
        report.output_frame_indices[0] = 1;
        report.output_timestamps_ms[0] = timestamps[1];
        assert_eq!(
            validate_source_cycle_sampling_report(&report).unwrap_err(),
            SourceCycleSamplingError::InvalidReport
        );
    }
}
