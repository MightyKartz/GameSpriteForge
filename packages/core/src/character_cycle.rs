use std::collections::BTreeMap;

use image::{imageops::FilterType, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::asset_project::{character_body_bbox, ConsistencyVerdict};

pub const TOPDOWN_CYCLE_WORKFLOW: &str = "topdown-cycle@10.0.0";
pub const TOPDOWN_CYCLE_STABILIZED_WORKFLOW: &str = "topdown-cycle@10.1.0";
pub const CHARACTER_SCALE_LOCK_PROFILE: &str = "character-scale-lock@1.0.0";
pub const CHARACTER_ANCHOR_STABILIZATION_PROFILE: &str = "character-anchor-stabilization@1.0.0";
pub const CHARACTER_VISUAL_ROOT_STABILIZATION_PROFILE: &str =
    "character-visual-root-stabilization@1.0.0";
pub const CHARACTER_NATIVE_PLACEMENT_PROFILE: &str = "character-native-placement@1.0.0";
pub const CHARACTER_VERTICAL_BOB_NORMALIZATION_PROFILE: &str =
    "character-vertical-bob-normalization@1.0.0";

const MAX_BODY_WIDTH_DRIFT_RATIO: f32 = 0.15;
const MAX_BODY_HEIGHT_DRIFT_RATIO: f32 = 0.12;
const MAX_TORSO_WIDTH_DRIFT_RATIO: f32 = 0.08;
const MAX_TORSO_AREA_DRIFT_RATIO: f32 = 0.12;
// Cardinal silhouettes are not horizontally symmetric: a hood, cloak, or
// profile nose can move the measured body-box center by a few pixels even
// after every frame uses the same scale and anchor. Four pixels on the 256px
// delivery canvas permits that outline change while still blocking a visible
// one-cell placement jump.
const MAX_CENTER_DRIFT_PX: f32 = 4.0;
const MAX_FOOT_BASELINE_DRIFT_PX: f32 = 3.0;
const MAX_STABILIZATION_TRANSLATION_PX: i32 = 12;
const MAX_STABILIZATION_CLIPPED_FOREGROUND_RATIO: f32 = 0.001;
const MAX_STABILIZATION_RESIDUAL_DRIFT_PX: f32 = 1.0;
const MAX_STABILIZATION_BODY_HEIGHT_DRIFT_RATIO: f32 = 0.015;
const VISUAL_ROOT_BODY_FRACTION: f32 = 0.5;
const SUPPORT_GROUND_CLUSTER_RADIUS_PX: u32 = 2;
const SUPPORT_GROUND_CANDIDATE_SEPARATION_PX: u32 = 5;
const SUPPORT_GROUND_MAX_CANDIDATES: usize = 10;
const SUPPORT_TRACK_EMISSION_WEIGHT: f32 = 0.5;
const SUPPORT_TRACK_ACCELERATION_WEIGHT: f32 = 0.35;
const MAX_VERTICAL_BOB_SCALE_DELTA: f32 = 0.04;
const MAX_VERTICAL_BOB_RESIDUAL_HEIGHT_DRIFT_PX: f32 = 2.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterAnchorStabilizationFrameV1 {
    pub animation: String,
    pub frame_index: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub input_body_center_x: f32,
    pub input_body_bottom_y: f32,
    pub translation_x: i32,
    pub translation_y: i32,
    pub output_body_center_x: f32,
    pub output_body_bottom_y: f32,
    pub clipped_foreground_pixel_count: u32,
    pub clipped_foreground_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterAnchorStabilizationReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub workflow: String,
    pub animation: String,
    pub translation_only: bool,
    pub target_body_center_x: f32,
    pub target_body_bottom_y: f32,
    pub maximum_translation_px: i32,
    pub maximum_clipped_foreground_ratio: f32,
    pub maximum_residual_center_drift_px: f32,
    pub maximum_residual_foot_drift_px: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub frames: Vec<CharacterAnchorStabilizationFrameV1>,
}

/// Stabilizes a selected complete cycle through integer translation only.
/// Pixel colors, scale, frame order and canvas dimensions remain unchanged.
pub fn stabilize_character_anchors(
    animation: &str,
    images: &[RgbaImage],
) -> (Vec<RgbaImage>, CharacterAnchorStabilizationReportV1) {
    let mut reasons = Vec::new();
    if !matches!(images.len(), 8 | 10 | 12 | 16 | 24) {
        reasons.push(format!(
            "{animation} has {} frames; stabilization requires one complete 8/10/12/16/24-frame cycle",
            images.len()
        ));
    }
    let expected_canvas = images.first().map(RgbaImage::dimensions);
    if expected_canvas.is_none()
        || images
            .iter()
            .any(|image| Some(image.dimensions()) != expected_canvas)
    {
        reasons.push("anchor stabilization requires one shared non-empty canvas".into());
    }
    let measurements = images.iter().map(character_body_bbox).collect::<Vec<_>>();
    if measurements.iter().any(Option::is_none) {
        reasons.push("anchor stabilization could not measure every character body".into());
    }
    let target_body_center_x = median(
        measurements
            .iter()
            .filter_map(|body| body.map(|body| body.center_x)),
    );
    let input_ground_y = cyclic_support_foot_ground_track(images, &measurements);
    let target_body_bottom_y = median(input_ground_y.iter().filter_map(|value| *value));
    if !target_body_center_x.is_finite() || !target_body_bottom_y.is_finite() {
        reasons.push("anchor stabilization target is not measurable".into());
    }

    let mut outputs = Vec::with_capacity(images.len());
    let mut frames = Vec::with_capacity(images.len());
    let mut maximum_translation_px = 0i32;
    let mut maximum_clipped_foreground_ratio = 0.0f32;
    for (frame_index, image) in images.iter().enumerate() {
        let Some(body) = measurements[frame_index] else {
            outputs.push(image.clone());
            continue;
        };
        let translation_x = (target_body_center_x - body.center_x).round() as i32;
        let input_support_ground_y = input_ground_y[frame_index].unwrap_or(body.bottom_y);
        let translation_y = (target_body_bottom_y - input_support_ground_y).round() as i32;
        maximum_translation_px = maximum_translation_px
            .max(translation_x.abs())
            .max(translation_y.abs());
        let mut output = RgbaImage::from_pixel(image.width(), image.height(), Rgba([0, 0, 0, 0]));
        let mut foreground = 0u32;
        let mut clipped = 0u32;
        for (x, y, pixel) in image.enumerate_pixels() {
            let destination_x = x as i32 + translation_x;
            let destination_y = y as i32 + translation_y;
            if pixel[3] > 48 {
                foreground += 1;
                if destination_x < 0
                    || destination_y < 0
                    || destination_x >= image.width() as i32
                    || destination_y >= image.height() as i32
                {
                    clipped += 1;
                }
            }
            if destination_x >= 0
                && destination_y >= 0
                && destination_x < image.width() as i32
                && destination_y < image.height() as i32
            {
                output.put_pixel(destination_x as u32, destination_y as u32, *pixel);
            }
        }
        let clipped_ratio = clipped as f32 / foreground.max(1) as f32;
        maximum_clipped_foreground_ratio = maximum_clipped_foreground_ratio.max(clipped_ratio);
        let output_body = character_body_bbox(&output).unwrap_or(body);
        // The selected contact belongs to the cycle-wide temporal track. Do
        // not re-detect it independently after translation, or the report can
        // switch to the other foot even though the pixels were moved exactly
        // as requested.
        let output_support_ground_y = input_support_ground_y + translation_y as f32;
        frames.push(CharacterAnchorStabilizationFrameV1 {
            animation: animation.into(),
            frame_index: frame_index as u32,
            canvas_width: image.width(),
            canvas_height: image.height(),
            input_body_center_x: body.center_x,
            input_body_bottom_y: input_support_ground_y,
            translation_x,
            translation_y,
            output_body_center_x: output_body.center_x,
            output_body_bottom_y: output_support_ground_y,
            clipped_foreground_pixel_count: clipped,
            clipped_foreground_ratio: clipped_ratio,
        });
        outputs.push(output);
    }
    let maximum_residual_center_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_body_center_x));
    let maximum_residual_foot_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_body_bottom_y));
    let input_body_height_drift_ratio = relative_drift(
        measurements
            .iter()
            .filter_map(|body| body.map(|body| body.height)),
    );
    // The 12px lock was calibrated on the 256px delivery canvas. Browser
    // video candidates are commonly 960px or 1440px and are downscaled only
    // after native-cycle selection, so enforce the same normalized placement
    // tolerance in source coordinates instead of accidentally tightening it
    // by 4-6x at high resolution.
    let maximum_allowed_translation_px = expected_canvas
        .map(|(width, height)| {
            ((width.max(height) as f32 / 256.0).max(1.0) * MAX_STABILIZATION_TRANSLATION_PX as f32)
                .round() as i32
        })
        .unwrap_or(MAX_STABILIZATION_TRANSLATION_PX);
    if maximum_translation_px > maximum_allowed_translation_px {
        reasons.push(format!(
            "anchor translation {maximum_translation_px}px exceeds the source-scaled locked maximum {maximum_allowed_translation_px}px"
        ));
    }
    if maximum_clipped_foreground_ratio > MAX_STABILIZATION_CLIPPED_FOREGROUND_RATIO {
        reasons.push(format!(
            "stabilization clipped foreground ratio {maximum_clipped_foreground_ratio:.6} exceeds {MAX_STABILIZATION_CLIPPED_FOREGROUND_RATIO:.6}"
        ));
    }
    if maximum_residual_center_drift_px > MAX_STABILIZATION_RESIDUAL_DRIFT_PX {
        reasons.push(format!(
            "residual body center drift {maximum_residual_center_drift_px:.4}px exceeds {MAX_STABILIZATION_RESIDUAL_DRIFT_PX:.4}px"
        ));
    }
    if maximum_residual_foot_drift_px > MAX_STABILIZATION_RESIDUAL_DRIFT_PX {
        reasons.push(format!(
            "residual foot baseline drift {maximum_residual_foot_drift_px:.4}px exceeds {MAX_STABILIZATION_RESIDUAL_DRIFT_PX:.4}px"
        ));
    }
    if input_body_height_drift_ratio > MAX_STABILIZATION_BODY_HEIGHT_DRIFT_RATIO {
        reasons.push(format!(
            "body height drift ratio {input_body_height_drift_ratio:.6} exceeds the translation-only source limit {MAX_STABILIZATION_BODY_HEIGHT_DRIFT_RATIO:.6}; source regeneration is required"
        ));
    }
    (
        outputs,
        CharacterAnchorStabilizationReportV1 {
            schema_version: "1".into(),
            profile: CHARACTER_ANCHOR_STABILIZATION_PROFILE.into(),
            workflow: TOPDOWN_CYCLE_STABILIZED_WORKFLOW.into(),
            animation: animation.into(),
            translation_only: true,
            target_body_center_x,
            target_body_bottom_y,
            maximum_translation_px,
            maximum_clipped_foreground_ratio,
            maximum_residual_center_drift_px,
            maximum_residual_foot_drift_px,
            verdict: if reasons.is_empty() {
                ConsistencyVerdict::GameReady
            } else {
                ConsistencyVerdict::Blocked
            },
            reasons,
            frames,
        },
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SupportGroundCandidate {
    y: u32,
    support: usize,
}

#[derive(Debug, Clone)]
struct SupportTrackState {
    cost: f32,
    path: Vec<usize>,
}

fn support_foot_ground_candidates(
    image: &RgbaImage,
    body: crate::frames::FrameBbox,
) -> Vec<SupportGroundCandidate> {
    let lower_top = (body.top + body.height * 0.62)
        .floor()
        .clamp(0.0, image.height().saturating_sub(1) as f32) as u32;
    let mut column_bottoms = Vec::new();
    for x in 0..image.width() {
        if let Some(y) = (lower_top..image.height())
            .rev()
            .find(|y| image.get_pixel(x, *y)[3] > 48)
        {
            column_bottoms.push(y + 1);
        }
    }
    if column_bottoms.len() < 6 {
        return Vec::new();
    }

    // Derive the lower-body band from a robust column percentile instead of
    // body.bottom_y. A detached alpha pixel below the boots can extend the
    // global body box but must not discard both real sole candidates.
    column_bottoms.sort_unstable();
    let robust_index = ((column_bottoms.len() - 1) as f32 * 0.80).round() as usize;
    let robust_bottom = column_bottoms[robust_index.min(column_bottoms.len() - 1)];
    let ground_band_top = robust_bottom.saturating_sub((body.height * 0.20).round() as u32);
    column_bottoms.retain(|bottom| *bottom >= ground_band_top);

    let minimum_support = (image.width() as usize).div_ceil(512).max(6);
    let mut candidates = column_bottoms
        .iter()
        .copied()
        .map(|y| SupportGroundCandidate {
            y,
            support: column_bottoms
                .iter()
                .filter(|value| value.abs_diff(y) <= SUPPORT_GROUND_CLUSTER_RADIUS_PX)
                .count(),
        })
        .filter(|candidate| candidate.support >= minimum_support)
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .support
            .cmp(&left.support)
            .then_with(|| right.y.cmp(&left.y))
    });
    let mut separated = Vec::new();
    for candidate in candidates {
        if separated.iter().all(|selected: &SupportGroundCandidate| {
            selected.y.abs_diff(candidate.y) > SUPPORT_GROUND_CANDIDATE_SEPARATION_PX
        }) {
            separated.push(candidate);
            if separated.len() == SUPPORT_GROUND_MAX_CANDIDATES {
                break;
            }
        }
    }
    separated.sort_by_key(|candidate| candidate.y);
    separated
}

fn support_foot_ground_y(image: &RgbaImage, body: crate::frames::FrameBbox) -> f32 {
    support_foot_ground_candidates(image, body)
        .into_iter()
        .max_by(|left, right| {
            left.support
                .cmp(&right.support)
                .then_with(|| left.y.cmp(&right.y))
        })
        .map(|candidate| candidate.y as f32)
        .unwrap_or(body.bottom_y)
}

fn support_track_emission_cost(candidate: SupportGroundCandidate, maximum_support: usize) -> f32 {
    let support_ratio = candidate.support as f32 / maximum_support.max(1) as f32;
    SUPPORT_TRACK_EMISSION_WEIGHT * (1.0 - support_ratio).powi(2)
}

fn support_track_velocity_cost(left_y: u32, right_y: u32, source_scale: f32) -> f32 {
    ((right_y as f32 - left_y as f32) / source_scale).powi(2)
}

fn support_track_acceleration_cost(
    first_y: u32,
    second_y: u32,
    third_y: u32,
    source_scale: f32,
) -> f32 {
    let first_velocity = (second_y as f32 - first_y as f32) / source_scale;
    let second_velocity = (third_y as f32 - second_y as f32) / source_scale;
    SUPPORT_TRACK_ACCELERATION_WEIGHT * (second_velocity - first_velocity).powi(2)
}

fn replace_support_track_state(slot: &mut Option<SupportTrackState>, cost: f32, path: Vec<usize>) {
    let replace = slot.as_ref().is_none_or(|current| {
        cost.total_cmp(&current.cost).is_lt()
            || (cost.total_cmp(&current.cost).is_eq() && path < current.path)
    });
    if replace {
        *slot = Some(SupportTrackState { cost, path });
    }
}

/// Select one support-contact baseline per frame using the complete cycle.
/// Candidate strength is only an observation; continuity, acceleration, and
/// the last-to-first transition prevent the anchor from jumping to the swing
/// foot during a support-leg handoff.
fn cyclic_support_foot_ground_track(
    images: &[RgbaImage],
    bodies: &[Option<crate::frames::FrameBbox>],
) -> Vec<Option<f32>> {
    let fallback = || {
        images
            .iter()
            .zip(bodies)
            .map(|(image, body)| body.map(|body| support_foot_ground_y(image, body)))
            .collect::<Vec<_>>()
    };
    if images.len() < 2 || images.len() != bodies.len() {
        return fallback();
    }
    let mut candidate_sets = Vec::with_capacity(images.len());
    for (image, body) in images.iter().zip(bodies) {
        let Some(body) = body else {
            return fallback();
        };
        let candidates = support_foot_ground_candidates(image, *body);
        if candidates.is_empty() {
            return fallback();
        }
        candidate_sets.push(candidates);
    }
    let maximum_support = candidate_sets
        .iter()
        .map(|candidates| {
            candidates
                .iter()
                .map(|candidate| candidate.support)
                .max()
                .unwrap_or(1)
        })
        .collect::<Vec<_>>();
    let source_scale = images
        .first()
        .map(|image| image.width().max(image.height()) as f32 / 256.0)
        .unwrap_or(1.0)
        .max(1.0);
    let mut best_cycle: Option<SupportTrackState> = None;

    for first_index in 0..candidate_sets[0].len() {
        for second_index in 0..candidate_sets[1].len() {
            let first = candidate_sets[0][first_index];
            let second = candidate_sets[1][second_index];
            let initial_cost = support_track_emission_cost(first, maximum_support[0])
                + support_track_emission_cost(second, maximum_support[1])
                + support_track_velocity_cost(first.y, second.y, source_scale);
            let mut states = vec![vec![None; candidate_sets[1].len()]; candidate_sets[0].len()];
            states[first_index][second_index] = Some(SupportTrackState {
                cost: initial_cost,
                path: vec![first_index, second_index],
            });

            for frame_index in 2..candidate_sets.len() {
                let mut next = vec![
                    vec![None; candidate_sets[frame_index].len()];
                    candidate_sets[frame_index - 1].len()
                ];
                for (previous_previous_index, row) in states.iter().enumerate() {
                    for (previous_index, state) in row.iter().enumerate() {
                        let Some(state) = state else {
                            continue;
                        };
                        let first_y = candidate_sets[frame_index - 2][previous_previous_index].y;
                        let second_y = candidate_sets[frame_index - 1][previous_index].y;
                        for (candidate_index, candidate) in
                            candidate_sets[frame_index].iter().copied().enumerate()
                        {
                            let cost = state.cost
                                + support_track_emission_cost(
                                    candidate,
                                    maximum_support[frame_index],
                                )
                                + support_track_velocity_cost(second_y, candidate.y, source_scale)
                                + support_track_acceleration_cost(
                                    first_y,
                                    second_y,
                                    candidate.y,
                                    source_scale,
                                );
                            let mut path = state.path.clone();
                            path.push(candidate_index);
                            replace_support_track_state(
                                &mut next[previous_index][candidate_index],
                                cost,
                                path,
                            );
                        }
                    }
                }
                states = next;
            }

            for (previous_index, row) in states.iter().enumerate() {
                for (last_index, state) in row.iter().enumerate() {
                    let Some(state) = state else {
                        continue;
                    };
                    let previous_y = candidate_sets[candidate_sets.len() - 2][previous_index].y;
                    let last_y = candidate_sets[candidate_sets.len() - 1][last_index].y;
                    let closure_cost = support_track_velocity_cost(last_y, first.y, source_scale)
                        + support_track_acceleration_cost(
                            previous_y,
                            last_y,
                            first.y,
                            source_scale,
                        )
                        + support_track_acceleration_cost(last_y, first.y, second.y, source_scale);
                    replace_support_track_state(
                        &mut best_cycle,
                        state.cost + closure_cost,
                        state.path.clone(),
                    );
                }
            }
        }
    }

    best_cycle
        .map(|cycle| {
            cycle
                .path
                .into_iter()
                .enumerate()
                .map(|(frame_index, candidate_index)| {
                    Some(candidate_sets[frame_index][candidate_index].y as f32)
                })
                .collect()
        })
        .unwrap_or_else(fallback)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterVisualRootFrameV1 {
    pub frame_index: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub input_body_top_y: f32,
    pub input_body_bottom_y: f32,
    pub input_body_height: f32,
    pub input_visual_root_y: f32,
    pub translation_y: i32,
    pub output_body_top_y: f32,
    pub output_body_bottom_y: f32,
    pub output_body_height: f32,
    pub output_visual_root_y: f32,
    pub clipped_foreground_pixel_count: u32,
    pub clipped_foreground_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterVisualRootStabilizationReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub translation_only: bool,
    pub body_fraction: f32,
    pub target_visual_root_y: f32,
    pub maximum_translation_px: i32,
    pub maximum_clipped_foreground_ratio: f32,
    pub input_body_height_drift_px: f32,
    pub input_body_height_drift_ratio: f32,
    pub output_visual_root_drift_px: f32,
    pub output_body_top_drift_px: f32,
    pub output_foot_baseline_drift_px: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub frames: Vec<CharacterVisualRootFrameV1>,
}

/// Locks the perceived full-body root at the middle of the measured
/// silhouette using integer Y translation only. It is a diagnostic fallback
/// for generated video whose body scale changes around the torso: the root can
/// be held still, but the report must continue to block source-scale drift
/// because translation cannot lock both head and feet simultaneously.
pub fn stabilize_character_visual_root(
    animation: &str,
    images: &[RgbaImage],
) -> (Vec<RgbaImage>, CharacterVisualRootStabilizationReportV1) {
    let mut reasons = Vec::new();
    if !matches!(images.len(), 8 | 10 | 12 | 16 | 24) {
        reasons.push(format!(
            "{animation} has {} frames; visual-root stabilization requires one complete 8/10/12/16/24-frame cycle",
            images.len()
        ));
    }
    let expected_canvas = images.first().map(RgbaImage::dimensions);
    if expected_canvas.is_none()
        || images
            .iter()
            .any(|image| Some(image.dimensions()) != expected_canvas)
    {
        reasons.push("visual-root stabilization requires one shared non-empty canvas".into());
    }
    let input_bodies = images.iter().map(character_body_bbox).collect::<Vec<_>>();
    if input_bodies.iter().any(Option::is_none) {
        reasons.push("visual-root stabilization could not measure every character body".into());
    }
    let input_roots = input_bodies
        .iter()
        .map(|body| body.map(|body| body.top + body.height * VISUAL_ROOT_BODY_FRACTION))
        .collect::<Vec<_>>();
    let target_visual_root_y = median(input_roots.iter().filter_map(|root| *root));
    let input_body_height_drift_px = absolute_drift(
        input_bodies
            .iter()
            .filter_map(|body| body.map(|body| body.height)),
    );
    let input_body_height_drift_ratio = relative_drift(
        input_bodies
            .iter()
            .filter_map(|body| body.map(|body| body.height)),
    );
    if !target_visual_root_y.is_finite() {
        reasons.push("visual-root stabilization target is not measurable".into());
    }

    let mut outputs = Vec::with_capacity(images.len());
    let mut frames = Vec::with_capacity(images.len());
    let mut maximum_translation_px = 0i32;
    let mut maximum_clipped_foreground_ratio = 0.0f32;
    for (frame_index, image) in images.iter().enumerate() {
        let Some(body) = input_bodies[frame_index] else {
            outputs.push(image.clone());
            continue;
        };
        let input_visual_root_y =
            input_roots[frame_index].unwrap_or(body.top + body.height * VISUAL_ROOT_BODY_FRACTION);
        let translation_y = (target_visual_root_y - input_visual_root_y).round() as i32;
        maximum_translation_px = maximum_translation_px.max(translation_y.abs());
        let mut output = RgbaImage::from_pixel(image.width(), image.height(), Rgba([0, 0, 0, 0]));
        let mut foreground = 0u32;
        let mut clipped = 0u32;
        for (x, y, pixel) in image.enumerate_pixels() {
            let destination_y = y as i32 + translation_y;
            if pixel[3] > 48 {
                foreground += 1;
                if destination_y < 0 || destination_y >= image.height() as i32 {
                    clipped += 1;
                }
            }
            if destination_y >= 0 && destination_y < image.height() as i32 {
                output.put_pixel(x, destination_y as u32, *pixel);
            }
        }
        let clipped_ratio = clipped as f32 / foreground.max(1) as f32;
        maximum_clipped_foreground_ratio = maximum_clipped_foreground_ratio.max(clipped_ratio);
        let output_body = character_body_bbox(&output).unwrap_or(body);
        frames.push(CharacterVisualRootFrameV1 {
            frame_index: frame_index as u32,
            canvas_width: image.width(),
            canvas_height: image.height(),
            input_body_top_y: body.top,
            input_body_bottom_y: body.bottom_y,
            input_body_height: body.height,
            input_visual_root_y,
            translation_y,
            output_body_top_y: output_body.top,
            output_body_bottom_y: output_body.bottom_y,
            output_body_height: output_body.height,
            output_visual_root_y: output_body.top + output_body.height * VISUAL_ROOT_BODY_FRACTION,
            clipped_foreground_pixel_count: clipped,
            clipped_foreground_ratio: clipped_ratio,
        });
        outputs.push(output);
    }
    let output_visual_root_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_visual_root_y));
    let output_body_top_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_body_top_y));
    let output_foot_baseline_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_body_bottom_y));
    let maximum_allowed_translation_px = expected_canvas
        .map(|(width, height)| {
            ((width.max(height) as f32 / 256.0).max(1.0) * MAX_STABILIZATION_TRANSLATION_PX as f32)
                .round() as i32
        })
        .unwrap_or(MAX_STABILIZATION_TRANSLATION_PX);
    if maximum_translation_px > maximum_allowed_translation_px {
        reasons.push(format!(
            "visual-root translation {maximum_translation_px}px exceeds the source-scaled locked maximum {maximum_allowed_translation_px}px"
        ));
    }
    if maximum_clipped_foreground_ratio > MAX_STABILIZATION_CLIPPED_FOREGROUND_RATIO {
        reasons.push(format!(
            "visual-root stabilization clipped foreground ratio {maximum_clipped_foreground_ratio:.6} exceeds {MAX_STABILIZATION_CLIPPED_FOREGROUND_RATIO:.6}"
        ));
    }
    if output_visual_root_drift_px > MAX_STABILIZATION_RESIDUAL_DRIFT_PX {
        reasons.push(format!(
            "residual visual-root drift {output_visual_root_drift_px:.4}px exceeds {MAX_STABILIZATION_RESIDUAL_DRIFT_PX:.4}px"
        ));
    }
    if input_body_height_drift_ratio > MAX_STABILIZATION_BODY_HEIGHT_DRIFT_RATIO {
        reasons.push(format!(
            "body height drift ratio {input_body_height_drift_ratio:.6} exceeds the translation-only source limit {MAX_STABILIZATION_BODY_HEIGHT_DRIFT_RATIO:.6}; source regeneration is required"
        ));
    }

    (
        outputs,
        CharacterVisualRootStabilizationReportV1 {
            schema_version: "1".into(),
            profile: CHARACTER_VISUAL_ROOT_STABILIZATION_PROFILE.into(),
            animation: animation.into(),
            translation_only: true,
            body_fraction: VISUAL_ROOT_BODY_FRACTION,
            target_visual_root_y,
            maximum_translation_px,
            maximum_clipped_foreground_ratio,
            input_body_height_drift_px,
            input_body_height_drift_ratio,
            output_visual_root_drift_px,
            output_body_top_drift_px,
            output_foot_baseline_drift_px,
            verdict: if reasons.is_empty() {
                ConsistencyVerdict::GameReady
            } else {
                ConsistencyVerdict::Blocked
            },
            reasons,
            frames,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterNativePlacementFrameV1 {
    pub frame_index: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub body_top_y: f32,
    pub body_bottom_y: f32,
    pub body_height: f32,
    pub body_center_x: f32,
    pub torso_width: f32,
    pub torso_area: f32,
    pub translation_x: i32,
    pub translation_y: i32,
    pub scale_x: f32,
    pub scale_y: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterNativePlacementReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub native_coordinates_preserved: bool,
    pub transforms_applied: bool,
    pub maximum_translation_px: i32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub body_top_range_px: f32,
    pub body_bottom_range_px: f32,
    pub body_height_range_px: f32,
    pub torso_width_drift_ratio: f32,
    pub torso_area_drift_ratio: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub frames: Vec<CharacterNativePlacementFrameV1>,
}

/// Audits cleaned native video frames without changing placement or scale.
/// Full-body bounds remain diagnostic for articulated side walks because leg
/// extension legitimately changes the lowest opaque pixel. Scale gating uses
/// the stable torso band instead of converting that pose change into a false
/// per-frame translation or rescale request.
pub fn assess_character_native_placement(
    animation: &str,
    images: &[RgbaImage],
) -> CharacterNativePlacementReportV1 {
    let mut reasons = Vec::new();
    if !matches!(images.len(), 8 | 10 | 12 | 16 | 24) {
        reasons.push(format!(
            "{animation} has {} frames; native placement requires one complete 8/10/12/16/24-frame cycle",
            images.len()
        ));
    }
    let expected_canvas = images.first().map(RgbaImage::dimensions);
    if expected_canvas.is_none()
        || images
            .iter()
            .any(|image| Some(image.dimensions()) != expected_canvas)
    {
        reasons.push("native placement requires one shared non-empty canvas".into());
    }
    let bodies = images.iter().map(character_body_bbox).collect::<Vec<_>>();
    if bodies.iter().any(Option::is_none) {
        reasons.push("native placement could not measure every character body".into());
    }
    let reference_body_height = median(
        bodies
            .iter()
            .filter_map(|body| body.map(|body| body.height)),
    );
    let mut frames = Vec::with_capacity(images.len());
    for (frame_index, image) in images.iter().enumerate() {
        let Some(body) = bodies[frame_index] else {
            continue;
        };
        let (torso_width, torso_area) =
            stable_torso_signature(image, body.top, reference_body_height);
        if torso_width <= 0.0 || torso_area <= 0.0 {
            reasons.push(format!(
                "{animation} frame {frame_index} has no measurable stable torso"
            ));
        }
        frames.push(CharacterNativePlacementFrameV1 {
            frame_index: frame_index as u32,
            canvas_width: image.width(),
            canvas_height: image.height(),
            body_top_y: body.top,
            body_bottom_y: body.bottom_y,
            body_height: body.height,
            body_center_x: body.center_x,
            torso_width,
            torso_area,
            translation_x: 0,
            translation_y: 0,
            scale_x: 1.0,
            scale_y: 1.0,
        });
    }
    let body_top_range_px = absolute_drift(frames.iter().map(|frame| frame.body_top_y));
    let body_bottom_range_px = absolute_drift(frames.iter().map(|frame| frame.body_bottom_y));
    let body_height_range_px = absolute_drift(frames.iter().map(|frame| frame.body_height));
    let torso_width_drift_ratio = relative_drift(frames.iter().map(|frame| frame.torso_width));
    let torso_area_drift_ratio = relative_drift(frames.iter().map(|frame| frame.torso_area));
    if torso_width_drift_ratio > MAX_TORSO_WIDTH_DRIFT_RATIO {
        reasons.push(format!(
            "native torso width drift ratio {torso_width_drift_ratio:.6} exceeds {MAX_TORSO_WIDTH_DRIFT_RATIO:.6}"
        ));
    }
    if torso_area_drift_ratio > MAX_TORSO_AREA_DRIFT_RATIO {
        reasons.push(format!(
            "native torso area drift ratio {torso_area_drift_ratio:.6} exceeds {MAX_TORSO_AREA_DRIFT_RATIO:.6}"
        ));
    }

    CharacterNativePlacementReportV1 {
        schema_version: "1".into(),
        profile: CHARACTER_NATIVE_PLACEMENT_PROFILE.into(),
        animation: animation.into(),
        native_coordinates_preserved: true,
        transforms_applied: false,
        maximum_translation_px: 0,
        scale_x: 1.0,
        scale_y: 1.0,
        body_top_range_px,
        body_bottom_range_px,
        body_height_range_px,
        torso_width_drift_ratio,
        torso_area_drift_ratio,
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        reasons,
        frames,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterVerticalBobFrameV1 {
    pub frame_index: u32,
    pub input_body_top_y: f32,
    pub input_body_bottom_y: f32,
    pub input_body_height: f32,
    pub requested_scale_y: f32,
    pub applied_scale_y: f32,
    pub output_body_top_y: f32,
    pub output_body_bottom_y: f32,
    pub output_body_height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterVerticalBobNormalizationReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub foot_anchored: bool,
    pub target_body_bottom_y: f32,
    pub target_body_height: f32,
    pub maximum_scale_delta: f32,
    pub input_body_top_drift_px: f32,
    pub output_body_top_drift_px: f32,
    pub input_body_height_drift_px: f32,
    pub output_body_height_drift_px: f32,
    pub output_foot_baseline_drift_px: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub frames: Vec<CharacterVerticalBobFrameV1>,
}

/// Reduces generated full-body rise/fall through a small vertical-only scale
/// about the already stabilized foot plane. Canvas size, horizontal geometry,
/// frame order, and the foot baseline remain unchanged.
pub fn normalize_character_vertical_bob(
    animation: &str,
    images: &[RgbaImage],
) -> (Vec<RgbaImage>, CharacterVerticalBobNormalizationReportV1) {
    let mut reasons = Vec::new();
    if !matches!(images.len(), 8 | 10 | 12 | 16 | 24) {
        reasons.push(format!(
            "{animation} has {} frames; vertical bob normalization requires one complete 8/10/12/16/24-frame cycle",
            images.len()
        ));
    }
    let expected_canvas = images.first().map(RgbaImage::dimensions);
    if expected_canvas.is_none()
        || images
            .iter()
            .any(|image| Some(image.dimensions()) != expected_canvas)
    {
        reasons.push("vertical bob normalization requires one shared non-empty canvas".into());
    }
    let input_bodies = images.iter().map(character_body_bbox).collect::<Vec<_>>();
    if input_bodies.iter().any(Option::is_none) {
        reasons.push("vertical bob normalization could not measure every character body".into());
    }
    let target_body_bottom_y = median(
        input_bodies
            .iter()
            .filter_map(|body| body.map(|body| body.bottom_y)),
    );
    let target_body_height = median(
        input_bodies
            .iter()
            .filter_map(|body| body.map(|body| body.height)),
    );
    let input_body_top_drift_px = absolute_drift(
        input_bodies
            .iter()
            .filter_map(|body| body.map(|body| body.top)),
    );
    let input_body_height_drift_px = absolute_drift(
        input_bodies
            .iter()
            .filter_map(|body| body.map(|body| body.height)),
    );
    if !target_body_bottom_y.is_finite() || !target_body_height.is_finite() {
        reasons.push("vertical bob normalization target is not measurable".into());
    }

    let mut outputs = Vec::with_capacity(images.len());
    let mut frames = Vec::with_capacity(images.len());
    let mut maximum_scale_delta = 0.0f32;
    for (frame_index, image) in images.iter().enumerate() {
        let Some(body) = input_bodies[frame_index] else {
            outputs.push(image.clone());
            continue;
        };
        let requested_scale_y = target_body_height / body.height.max(1.0);
        let applied_scale_y = requested_scale_y.clamp(
            1.0 - MAX_VERTICAL_BOB_SCALE_DELTA,
            1.0 + MAX_VERTICAL_BOB_SCALE_DELTA,
        );
        maximum_scale_delta = maximum_scale_delta.max((applied_scale_y - 1.0).abs());
        if (requested_scale_y - applied_scale_y).abs() > 1.0e-4 {
            reasons.push(format!(
                "frame {frame_index} requested vertical scale {requested_scale_y:.6} outside the locked {:.2}% envelope",
                MAX_VERTICAL_BOB_SCALE_DELTA * 100.0
            ));
        }
        let scaled_height = ((image.height() as f32 * applied_scale_y).round() as u32).max(1);
        let resized =
            image::imageops::resize(image, image.width(), scaled_height, FilterType::Triangle);
        let destination_y = (target_body_bottom_y - body.bottom_y * applied_scale_y).round() as i64;
        let mut output = RgbaImage::from_pixel(image.width(), image.height(), Rgba([0, 0, 0, 0]));
        image::imageops::overlay(&mut output, &resized, 0, destination_y);
        let mut output_body = character_body_bbox(&output).unwrap_or(body);
        let foot_correction_y = (target_body_bottom_y - output_body.bottom_y).round() as i64;
        if foot_correction_y != 0 {
            let mut corrected =
                RgbaImage::from_pixel(image.width(), image.height(), Rgba([0, 0, 0, 0]));
            image::imageops::overlay(&mut corrected, &output, 0, foot_correction_y);
            output = corrected;
            output_body = character_body_bbox(&output).unwrap_or(output_body);
        }
        frames.push(CharacterVerticalBobFrameV1 {
            frame_index: frame_index as u32,
            input_body_top_y: body.top,
            input_body_bottom_y: body.bottom_y,
            input_body_height: body.height,
            requested_scale_y,
            applied_scale_y,
            output_body_top_y: output_body.top,
            output_body_bottom_y: output_body.bottom_y,
            output_body_height: output_body.height,
        });
        outputs.push(output);
    }
    let output_body_top_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_body_top_y));
    let output_body_height_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_body_height));
    let output_foot_baseline_drift_px =
        absolute_drift(frames.iter().map(|frame| frame.output_body_bottom_y));
    if output_body_height_drift_px > MAX_VERTICAL_BOB_RESIDUAL_HEIGHT_DRIFT_PX {
        reasons.push(format!(
            "residual body height drift {output_body_height_drift_px:.4}px exceeds {MAX_VERTICAL_BOB_RESIDUAL_HEIGHT_DRIFT_PX:.4}px"
        ));
    }
    if output_foot_baseline_drift_px > MAX_STABILIZATION_RESIDUAL_DRIFT_PX {
        reasons.push(format!(
            "vertical bob correction foot baseline drift {output_foot_baseline_drift_px:.4}px exceeds {MAX_STABILIZATION_RESIDUAL_DRIFT_PX:.4}px"
        ));
    }

    (
        outputs,
        CharacterVerticalBobNormalizationReportV1 {
            schema_version: "1".into(),
            profile: CHARACTER_VERTICAL_BOB_NORMALIZATION_PROFILE.into(),
            animation: animation.into(),
            foot_anchored: true,
            target_body_bottom_y,
            target_body_height,
            maximum_scale_delta,
            input_body_top_drift_px,
            output_body_top_drift_px,
            input_body_height_drift_px,
            output_body_height_drift_px,
            output_foot_baseline_drift_px,
            verdict: if reasons.is_empty() {
                ConsistencyVerdict::GameReady
            } else {
                ConsistencyVerdict::Blocked
            },
            reasons,
            frames,
        },
    )
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterScaleFrameV1 {
    pub animation: String,
    pub frame_index: u32,
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub body_width: f32,
    pub body_height: f32,
    pub body_center_x: f32,
    pub body_bottom_y: f32,
    pub torso_width: f32,
    pub torso_area: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterScaleLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub workflow: String,
    pub direction_grid_lock_sha256: String,
    pub shared_scale: f32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub max_body_width_drift_ratio: f32,
    pub max_body_height_drift_ratio: f32,
    pub max_torso_width_drift_ratio: f32,
    pub max_torso_area_drift_ratio: f32,
    pub max_center_drift_px: f32,
    pub max_foot_baseline_drift_px: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
    pub frames: Vec<CharacterScaleFrameV1>,
}

pub fn assess_character_scale_lock(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    direction_grid_lock_sha256: impl Into<String>,
    shared_scale: f32,
    anchor_x: f32,
    anchor_y: f32,
) -> CharacterScaleLockV1 {
    assess_character_scale_lock_for_workflow(
        animations,
        direction_grid_lock_sha256,
        shared_scale,
        anchor_x,
        anchor_y,
        TOPDOWN_CYCLE_WORKFLOW,
    )
}

pub fn assess_character_scale_lock_for_workflow(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    direction_grid_lock_sha256: impl Into<String>,
    shared_scale: f32,
    anchor_x: f32,
    anchor_y: f32,
    workflow: &str,
) -> CharacterScaleLockV1 {
    let mut reasons = Vec::new();
    let mut frames = Vec::new();
    let mut expected_canvas = None;

    for (animation, animation_frames) in animations {
        let expected_count = if animation.starts_with("walk_") {
            matches!(animation_frames.len(), 8 | 10 | 12)
        } else {
            animation_frames.len() == 1
        };
        if !expected_count {
            reasons.push(format!(
                "{animation} has {} frames; walks require one complete 8/10/12-frame cycle and idles require one frame",
                animation_frames.len()
            ));
        }

        for (frame_index, image) in animation_frames.iter().enumerate() {
            let canvas = (image.width(), image.height());
            if expected_canvas
                .replace(canvas)
                .is_some_and(|value| value != canvas)
            {
                reasons.push(format!(
                    "{animation} frame {frame_index} does not use the shared canvas"
                ));
            }
            let Some(body) = character_body_bbox(image) else {
                reasons.push(format!(
                    "{animation} frame {frame_index} has no measurable character body"
                ));
                continue;
            };
            let (torso_width, torso_area) = stable_torso_signature(image, body.top, body.height);
            if torso_width <= 0.0 || torso_area <= 0.0 {
                reasons.push(format!(
                    "{animation} frame {frame_index} has no stable upper-body signature"
                ));
                continue;
            }
            frames.push(CharacterScaleFrameV1 {
                animation: animation.clone(),
                frame_index: frame_index as u32,
                canvas_width: image.width(),
                canvas_height: image.height(),
                body_width: body.width,
                body_height: body.height,
                body_center_x: body.center_x,
                body_bottom_y: body.bottom_y,
                torso_width,
                torso_area,
            });
        }
    }

    let measured = frames.iter().collect::<Vec<_>>();
    let max_body_width_drift_ratio = relative_drift(measured.iter().map(|frame| frame.body_width));
    let max_body_height_drift_ratio =
        relative_drift(measured.iter().map(|frame| frame.body_height));
    let max_torso_width_drift_ratio =
        relative_drift(measured.iter().map(|frame| frame.torso_width));
    let max_torso_area_drift_ratio = relative_drift(measured.iter().map(|frame| frame.torso_area));
    let max_center_drift_px = absolute_drift(measured.iter().map(|frame| frame.body_center_x));
    let max_foot_baseline_drift_px =
        absolute_drift(measured.iter().map(|frame| frame.body_bottom_y));

    record_limit(
        &mut reasons,
        "body width drift",
        max_body_width_drift_ratio,
        MAX_BODY_WIDTH_DRIFT_RATIO,
    );
    record_limit(
        &mut reasons,
        "body height drift",
        max_body_height_drift_ratio,
        MAX_BODY_HEIGHT_DRIFT_RATIO,
    );
    record_limit(
        &mut reasons,
        "stable torso width drift",
        max_torso_width_drift_ratio,
        MAX_TORSO_WIDTH_DRIFT_RATIO,
    );
    record_limit(
        &mut reasons,
        "stable torso area drift",
        max_torso_area_drift_ratio,
        MAX_TORSO_AREA_DRIFT_RATIO,
    );
    record_limit(
        &mut reasons,
        "body center drift px",
        max_center_drift_px,
        MAX_CENTER_DRIFT_PX,
    );
    record_limit(
        &mut reasons,
        "foot baseline drift px",
        max_foot_baseline_drift_px,
        MAX_FOOT_BASELINE_DRIFT_PX,
    );
    if !shared_scale.is_finite() || shared_scale <= 0.0 {
        reasons.push("shared scale must be finite and positive".into());
    }
    if !anchor_x.is_finite() || !anchor_y.is_finite() {
        reasons.push("shared anchor must be finite".into());
    }

    CharacterScaleLockV1 {
        schema_version: "1".into(),
        profile: CHARACTER_SCALE_LOCK_PROFILE.into(),
        workflow: workflow.into(),
        direction_grid_lock_sha256: direction_grid_lock_sha256.into(),
        shared_scale,
        anchor_x,
        anchor_y,
        max_body_width_drift_ratio,
        max_body_height_drift_ratio,
        max_torso_width_drift_ratio,
        max_torso_area_drift_ratio,
        max_center_drift_px,
        max_foot_baseline_drift_px,
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        reasons,
        frames,
    }
}

fn median(values: impl Iterator<Item = f32>) -> f32 {
    let mut values = values.filter(|value| value.is_finite()).collect::<Vec<_>>();
    if values.is_empty() {
        return f32::NAN;
    }
    values.sort_by(f32::total_cmp);
    values[values.len() / 2]
}

fn stable_torso_signature(image: &RgbaImage, body_top: f32, body_height: f32) -> (f32, f32) {
    let top = body_top.floor().max(0.0) as u32;
    let bottom = (body_top + body_height * 0.62)
        .ceil()
        .min(image.height() as f32) as u32;
    let mut minimum_x = image.width();
    let mut maximum_x = 0u32;
    let mut alpha_area = 0u32;
    for y in top..bottom {
        for x in 0..image.width() {
            if image.get_pixel(x, y)[3] > 48 {
                minimum_x = minimum_x.min(x);
                maximum_x = maximum_x.max(x);
                alpha_area += 1;
            }
        }
    }
    if alpha_area == 0 {
        return (0.0, 0.0);
    }
    (
        maximum_x.saturating_sub(minimum_x).saturating_add(1) as f32,
        alpha_area as f32,
    )
}

fn relative_drift(values: impl Iterator<Item = f32>) -> f32 {
    let mut values = values
        .filter(|value| value.is_finite() && *value > 0.0)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return f32::INFINITY;
    }
    values.sort_by(f32::total_cmp);
    let median = values[values.len() / 2];
    values
        .iter()
        .map(|value| (value - median).abs() / median.max(1.0))
        .fold(0.0, f32::max)
}

fn absolute_drift(values: impl Iterator<Item = f32>) -> f32 {
    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    for value in values.filter(|value| value.is_finite()) {
        minimum = minimum.min(value);
        maximum = maximum.max(value);
    }
    if !minimum.is_finite() || !maximum.is_finite() {
        f32::INFINITY
    } else {
        maximum - minimum
    }
}

fn record_limit(reasons: &mut Vec<String>, label: &str, observed: f32, maximum: f32) {
    if !observed.is_finite() || observed > maximum {
        reasons.push(format!(
            "{label} {observed:.4} exceeds the locked maximum {maximum:.4}"
        ));
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn frame(width: u32, height: u32, offset_x: i32) -> RgbaImage {
        let mut image = RgbaImage::new(96, 96);
        let left = (36 + offset_x) as u32;
        for y in 20..(20 + height) {
            for x in left..(left + width) {
                image.put_pixel(x, y, Rgba([60, 110, 55, 255]));
            }
        }
        image
    }

    #[test]
    fn scale_lock_accepts_complete_cycle_with_shared_size() {
        let mut animations = BTreeMap::new();
        animations.insert(
            "walk_down".into(),
            (0..8).map(|index| frame(24, 52, index % 2)).collect(),
        );
        let report = assess_character_scale_lock(&animations, "lock-sha", 0.75, 0.5, 0.82);
        assert_eq!(
            report.verdict,
            ConsistencyVerdict::GameReady,
            "{:?}",
            report.reasons
        );
        assert_eq!(report.frames.len(), 8);
    }

    #[test]
    fn scale_lock_blocks_independent_frame_rescaling() {
        let mut frames = (0..8).map(|_| frame(24, 52, 0)).collect::<Vec<_>>();
        frames[4] = frame(32, 66, 0);
        let mut animations = BTreeMap::new();
        animations.insert("walk_down".into(), frames);
        let report = assess_character_scale_lock(&animations, "lock-sha", 0.75, 0.5, 0.82);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.reasons.iter().any(|reason| reason.contains("drift")));
    }

    #[test]
    fn scale_lock_allows_four_pixels_of_direction_outline_shift_but_blocks_five() {
        let mut accepted = BTreeMap::new();
        accepted.insert(
            "walk_down".into(),
            (0..8)
                .map(|index| frame(24, 52, if index == 7 { 4 } else { 0 }))
                .collect(),
        );
        let accepted = assess_character_scale_lock(&accepted, "lock-sha", 0.75, 0.5, 0.82);
        assert_eq!(accepted.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(accepted.max_center_drift_px, 4.0);

        let mut blocked = BTreeMap::new();
        blocked.insert(
            "walk_down".into(),
            (0..8)
                .map(|index| frame(24, 52, if index == 7 { 5 } else { 0 }))
                .collect(),
        );
        let blocked = assess_character_scale_lock(&blocked, "lock-sha", 0.75, 0.5, 0.82);
        assert_eq!(blocked.verdict, ConsistencyVerdict::Blocked);
        assert!(blocked
            .reasons
            .iter()
            .any(|reason| reason.contains("body center drift px")));
    }

    #[test]
    fn anchor_stabilization_translates_without_resizing_and_removes_drift() {
        let inputs = (0..8)
            .map(|index| {
                let mut image = frame(24, 52, index - 4);
                if index % 2 == 0 {
                    let shifted = image::imageops::crop_imm(&image, 0, 0, 96, 95).to_image();
                    image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
                    image::imageops::overlay(&mut image, &shifted, 0, 1);
                }
                image
            })
            .collect::<Vec<_>>();
        let input_foreground = inputs
            .iter()
            .map(|image| image.pixels().filter(|pixel| pixel[3] > 48).count())
            .collect::<Vec<_>>();
        let (outputs, report) = stabilize_character_anchors("walk_down", &inputs);
        assert_eq!(
            report.verdict,
            ConsistencyVerdict::GameReady,
            "{:?}",
            report.reasons
        );
        assert!(report.translation_only);
        assert!(report.maximum_residual_center_drift_px <= 1.0);
        assert!(report.maximum_residual_foot_drift_px <= 1.0);
        assert_eq!(
            input_foreground,
            outputs
                .iter()
                .map(|image| image.pixels().filter(|pixel| pixel[3] > 48).count())
                .collect::<Vec<_>>()
        );
        assert!(outputs.iter().all(|image| image.dimensions() == (96, 96)));
    }

    #[test]
    fn anchor_stabilization_blocks_clipping_or_extreme_translation() {
        let mut inputs = (0..8).map(|_| frame(24, 52, 0)).collect::<Vec<_>>();
        inputs[7] = frame(24, 52, 30);
        let (_, report) = stabilize_character_anchors("walk_down", &inputs);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.reasons.iter().any(|reason| {
            reason.contains("anchor translation") || reason.contains("clipped foreground")
        }));
    }

    #[test]
    fn anchor_stabilization_accepts_twenty_four_frame_cycle() {
        let inputs = (0..24)
            .map(|index| frame(24, 52, index % 5 - 2))
            .collect::<Vec<_>>();
        let (_, report) = stabilize_character_anchors("walk_right", &inputs);
        assert_eq!(
            report.verdict,
            ConsistencyVerdict::GameReady,
            "{:?}",
            report.reasons
        );
        assert_eq!(report.frames.len(), 24);
    }

    #[test]
    fn anchor_stabilization_blocks_source_height_drift_that_translation_cannot_fix() {
        let mut inputs = (0..8).map(|_| frame(24, 52, 0)).collect::<Vec<_>>();
        inputs[4] = frame(24, 60, 0);

        let (_, report) = stabilize_character_anchors("walk_right", &inputs);

        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("source regeneration is required")));
    }

    #[test]
    fn visual_root_stabilization_is_translation_only_and_removes_root_drift() {
        let inputs = (0..8)
            .map(|index| {
                let mut image = RgbaImage::new(96, 96);
                let top = 16 + index;
                for y in top..top + 52 {
                    for x in 36..60 {
                        image.put_pixel(x, y, Rgba([60, 110, 55, 255]));
                    }
                }
                image
            })
            .collect::<Vec<_>>();
        let input_foreground = inputs
            .iter()
            .map(|image| image.pixels().filter(|pixel| pixel[3] > 48).count())
            .collect::<Vec<_>>();

        let (outputs, report) = stabilize_character_visual_root("walk_right", &inputs);

        assert_eq!(
            report.verdict,
            ConsistencyVerdict::GameReady,
            "{:?}",
            report.reasons
        );
        assert!(report.translation_only);
        assert!(report.output_visual_root_drift_px <= 1.0);
        assert_eq!(
            input_foreground,
            outputs
                .iter()
                .map(|image| image.pixels().filter(|pixel| pixel[3] > 48).count())
                .collect::<Vec<_>>()
        );
        assert!(outputs.iter().all(|image| image.dimensions() == (96, 96)));
    }

    #[test]
    fn visual_root_stabilization_reports_unfixable_height_drift_without_scaling() {
        let mut inputs = (0..8).map(|_| frame(24, 52, 0)).collect::<Vec<_>>();
        inputs[6] = frame(24, 60, 0);

        let (_, report) = stabilize_character_visual_root("walk_right", &inputs);

        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.translation_only);
        assert!(report.input_body_height_drift_px >= 8.0);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason.contains("source regeneration is required")));
    }

    #[test]
    fn native_placement_keeps_zero_transforms_when_only_leg_extent_changes() {
        let inputs = (0..8)
            .map(|index| {
                let mut image = RgbaImage::new(96, 96);
                for y in 20..30 {
                    for x in 38..58 {
                        image.put_pixel(x, y, Rgba([70, 105, 55, 255]));
                    }
                }
                for y in 30..66 {
                    for x in 32..64 {
                        image.put_pixel(x, y, Rgba([70, 105, 55, 255]));
                    }
                }
                for y in 66..(80 + index) {
                    for x in 36..46 {
                        image.put_pixel(x, y, Rgba([80, 48, 30, 255]));
                    }
                    for x in 50..60 {
                        image.put_pixel(x, y, Rgba([80, 48, 30, 255]));
                    }
                }
                image
            })
            .collect::<Vec<_>>();

        let report = assess_character_native_placement("walk_right", &inputs);

        assert_eq!(
            report.verdict,
            ConsistencyVerdict::GameReady,
            "{:?}",
            report.reasons
        );
        assert!(report.native_coordinates_preserved);
        assert!(!report.transforms_applied);
        assert_eq!(report.maximum_translation_px, 0);
        assert_eq!(report.scale_x, 1.0);
        assert_eq!(report.scale_y, 1.0);
        assert!(report.body_height_range_px >= 7.0);
        assert!(report.frames.iter().all(|frame| {
            frame.translation_x == 0
                && frame.translation_y == 0
                && frame.scale_x == 1.0
                && frame.scale_y == 1.0
        }));
    }

    #[test]
    fn native_placement_blocks_real_torso_scale_drift() {
        let mut inputs = (0..8)
            .map(|_| {
                let mut image = RgbaImage::new(96, 96);
                for y in 20..30 {
                    for x in 38..58 {
                        image.put_pixel(x, y, Rgba([70, 105, 55, 255]));
                    }
                }
                for y in 30..66 {
                    for x in 32..64 {
                        image.put_pixel(x, y, Rgba([70, 105, 55, 255]));
                    }
                }
                for y in 66..82 {
                    for x in 36..46 {
                        image.put_pixel(x, y, Rgba([80, 48, 30, 255]));
                    }
                    for x in 50..60 {
                        image.put_pixel(x, y, Rgba([80, 48, 30, 255]));
                    }
                }
                image
            })
            .collect::<Vec<_>>();
        for y in 30..66 {
            for x in 24..72 {
                inputs[5].put_pixel(x, y, Rgba([70, 105, 55, 255]));
            }
        }

        let report = assess_character_native_placement("walk_right", &inputs);

        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.reasons.iter().any(|reason| reason.contains("torso")));
    }

    #[test]
    fn support_foot_ground_ignores_lifted_foot_and_detached_low_pixel() {
        let mut image = RgbaImage::new(96, 96);
        for y in 20..60 {
            for x in 36..60 {
                image.put_pixel(x, y, Rgba([60, 110, 55, 255]));
            }
        }
        for y in 60..78 {
            for x in 36..54 {
                image.put_pixel(x, y, Rgba([90, 55, 35, 255]));
            }
        }
        for y in 60..82 {
            for x in 58..66 {
                image.put_pixel(x, y, Rgba([90, 55, 35, 255]));
            }
        }
        image.put_pixel(24, 90, Rgba([255, 255, 255, 255]));

        let body = character_body_bbox(&image).expect("body");
        assert_eq!(support_foot_ground_y(&image, body), 78.0);
    }

    #[test]
    fn support_foot_track_keeps_one_cyclic_contact_through_a_mode_switch() {
        let inputs = (0..8)
            .map(|frame_index| {
                let mut image = RgbaImage::new(96, 96);
                for y in 20..60 {
                    for x in 36..60 {
                        image.put_pixel(x, y, Rgba([60, 110, 55, 255]));
                    }
                }
                // A narrow planted sole remains on y=80 throughout the
                // cycle. The wider swing foot briefly ends at y=64, which is
                // strong enough to win an independent per-frame mode.
                for y in 60..80 {
                    for x in 36..44 {
                        image.put_pixel(x, y, Rgba([90, 55, 35, 255]));
                    }
                }
                let swing_bottom = if frame_index == 3 { 64 } else { 80 };
                for y in 60..swing_bottom {
                    for x in 64..84 {
                        image.put_pixel(x, y, Rgba([90, 55, 35, 255]));
                    }
                }
                image
            })
            .collect::<Vec<_>>();
        let bodies = inputs.iter().map(character_body_bbox).collect::<Vec<_>>();

        assert_eq!(
            support_foot_ground_y(&inputs[3], bodies[3].expect("body")),
            64.0
        );
        assert_eq!(
            cyclic_support_foot_ground_track(&inputs, &bodies),
            vec![Some(80.0); 8]
        );
    }

    #[test]
    fn vertical_bob_normalization_preserves_feet_and_equalizes_height() {
        let inputs = (0..24)
            .map(|index| {
                let height = 50 + index % 5;
                let bottom = 80u32;
                let mut image = RgbaImage::new(96, 96);
                for y in bottom - height..bottom {
                    for x in 36..60 {
                        image.put_pixel(x, y, Rgba([60, 110, 55, 255]));
                    }
                }
                image
            })
            .collect::<Vec<_>>();

        let (outputs, report) = normalize_character_vertical_bob("walk_right", &inputs);

        assert_eq!(
            report.verdict,
            ConsistencyVerdict::GameReady,
            "{:?}",
            report.reasons
        );
        assert_eq!(outputs.len(), 24);
        assert!(report.input_body_height_drift_px >= 4.0);
        assert!(report.output_body_height_drift_px <= 2.0);
        assert!(report.output_foot_baseline_drift_px <= 1.0);
        assert!(report.maximum_scale_delta <= MAX_VERTICAL_BOB_SCALE_DELTA);
    }
}
