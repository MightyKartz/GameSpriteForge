use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

use image::RgbaImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::asset_project::{character_body_bbox, ConsistencyVerdict};
use crate::frames::FrameBbox;

pub const CHARACTER_MOTION_SEMANTICS_PROFILE: &str = "motion-semantics@1.5.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MotionSemanticsThresholdsV1 {
    pub walk_min_lower_body_dynamic_degree: f32,
    #[serde(default = "default_walk_max_lower_body_dynamic_degree")]
    pub walk_max_lower_body_dynamic_degree: f32,
    pub walk_min_opposing_contact_change_ratio: f32,
    pub walk_min_distinct_pose_count: usize,
    pub walk_min_phase_order_score: f32,
    pub idle_min_foreground_dynamic_degree: f32,
    pub maximum_stable_upper_body_flicker_ratio: f32,
    #[serde(default = "default_maximum_stable_upper_body_alpha_drift_ratio")]
    pub maximum_stable_upper_body_alpha_drift_ratio: f32,
    pub lower_edge_ghost_minimum_ratio: f32,
    pub lower_edge_ghost_minimum_outlier_ratio: f32,
    pub maximum_foot_lobe_count: usize,
    #[serde(default = "default_walk_side_min_knee_shin_dynamic_degree")]
    pub walk_side_min_knee_shin_dynamic_degree: f32,
    #[serde(default = "default_walk_side_max_knee_shin_dynamic_degree")]
    pub walk_side_max_knee_shin_dynamic_degree: f32,
    #[serde(default = "default_walk_side_max_foot_to_knee_shin_motion_ratio")]
    pub walk_side_max_foot_to_knee_shin_motion_ratio: f32,
    #[serde(default = "default_walk_side_min_contact_passing_cadence_score")]
    pub walk_side_min_contact_passing_cadence_score: f32,
}

impl Default for MotionSemanticsThresholdsV1 {
    fn default() -> Self {
        Self {
            walk_min_lower_body_dynamic_degree: 0.025,
            walk_max_lower_body_dynamic_degree: default_walk_max_lower_body_dynamic_degree(),
            walk_min_opposing_contact_change_ratio: 0.04,
            walk_min_distinct_pose_count: 4,
            walk_min_phase_order_score: 0.45,
            idle_min_foreground_dynamic_degree: 0.002,
            maximum_stable_upper_body_flicker_ratio: 0.08,
            maximum_stable_upper_body_alpha_drift_ratio:
                default_maximum_stable_upper_body_alpha_drift_ratio(),
            lower_edge_ghost_minimum_ratio: 0.18,
            lower_edge_ghost_minimum_outlier_ratio: 2.5,
            maximum_foot_lobe_count: 2,
            walk_side_min_knee_shin_dynamic_degree: default_walk_side_min_knee_shin_dynamic_degree(
            ),
            walk_side_max_knee_shin_dynamic_degree: default_walk_side_max_knee_shin_dynamic_degree(
            ),
            walk_side_max_foot_to_knee_shin_motion_ratio:
                default_walk_side_max_foot_to_knee_shin_motion_ratio(),
            walk_side_min_contact_passing_cadence_score:
                default_walk_side_min_contact_passing_cadence_score(),
        }
    }
}

fn default_walk_max_lower_body_dynamic_degree() -> f32 {
    0.75
}

fn default_maximum_stable_upper_body_alpha_drift_ratio() -> f32 {
    0.20
}

fn default_walk_side_min_knee_shin_dynamic_degree() -> f32 {
    0.14
}

fn default_walk_side_max_knee_shin_dynamic_degree() -> f32 {
    0.80
}

fn default_walk_side_max_foot_to_knee_shin_motion_ratio() -> f32 {
    3.5
}

fn default_walk_side_min_contact_passing_cadence_score() -> f32 {
    0.65
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterMotionAnimationReportV1 {
    pub name: String,
    pub frame_count: usize,
    pub foreground_dynamic_degree: f32,
    pub lower_body_dynamic_degree: f32,
    pub opposing_contact_change_ratio: f32,
    pub distinct_pose_count: usize,
    pub phase_order_score: f32,
    pub stable_upper_body_flicker_ratio_max: f32,
    #[serde(default)]
    pub stable_upper_body_alpha_drift_ratio_max: f32,
    #[serde(default)]
    pub stable_upper_body_color_flicker_ratio_max: f32,
    pub unsupported_lower_edge_ratio_max: f32,
    pub lower_edge_outlier_ratio_max: f32,
    pub foot_lobe_count_max: usize,
    #[serde(default)]
    pub proximal_leg_dynamic_degree: f32,
    #[serde(default)]
    pub knee_shin_dynamic_degree: f32,
    #[serde(default)]
    pub foot_dynamic_degree: f32,
    #[serde(default)]
    pub foot_to_knee_shin_motion_ratio: f32,
    #[serde(default)]
    pub side_contact_passing_spread_ratio: f32,
    #[serde(default)]
    pub side_contact_passing_cadence_score: f32,
    #[serde(default)]
    pub side_laterality_review_required: bool,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_retry_frames: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterMotionSemanticsReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub thresholds: MotionSemanticsThresholdsV1,
    pub verdict: ConsistencyVerdict,
    pub animations: Vec<CharacterMotionAnimationReportV1>,
}

#[derive(Debug, Error)]
pub enum MotionSemanticsError {
    #[error("pack error: {0}")]
    Pack(#[from] forge_pack::PackError),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("character pack contains no animations")]
    NoAnimations,
    #[error("animation {animation} references missing frame {frame}")]
    MissingFrame { animation: String, frame: usize },
}

pub fn assess_character_motion_semantics(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
) -> CharacterMotionSemanticsReportV1 {
    assess_character_motion_semantics_with_thresholds(animations, Default::default())
}

pub fn assess_character_motion_semantics_with_thresholds(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    thresholds: MotionSemanticsThresholdsV1,
) -> CharacterMotionSemanticsReportV1 {
    let animation_reports = animations
        .iter()
        .map(|(name, frames)| assess_animation(name, frames, &thresholds))
        .collect::<Vec<_>>();
    let verdict = if animation_reports
        .iter()
        .all(|report| report.verdict == ConsistencyVerdict::GameReady)
    {
        ConsistencyVerdict::GameReady
    } else {
        ConsistencyVerdict::Blocked
    };
    CharacterMotionSemanticsReportV1 {
        schema_version: "1".into(),
        profile: CHARACTER_MOTION_SEMANTICS_PROFILE.into(),
        thresholds,
        verdict,
        animations: animation_reports,
    }
}

pub fn audit_character_pack_motion(
    pack_path: &Path,
) -> Result<CharacterMotionSemanticsReportV1, MotionSemanticsError> {
    let imported = forge_pack::import_pack(pack_path)?;
    let mut animations = BTreeMap::<String, Vec<RgbaImage>>::new();
    let Some(entries) = imported
        .forgepack
        .get("animations")
        .and_then(serde_json::Value::as_array)
    else {
        return Err(MotionSemanticsError::NoAnimations);
    };
    for entry in entries {
        let Some(name) = entry.get("name").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(indices) = entry.get("frames").and_then(serde_json::Value::as_array) else {
            continue;
        };
        let frames = indices
            .iter()
            .filter_map(serde_json::Value::as_u64)
            .map(|index| {
                let index = index as usize;
                let path = imported.frame_paths.get(index).ok_or_else(|| {
                    MotionSemanticsError::MissingFrame {
                        animation: name.into(),
                        frame: index,
                    }
                })?;
                Ok(image::open(path)?.to_rgba8())
            })
            .collect::<Result<Vec<_>, MotionSemanticsError>>()?;
        animations.insert(name.into(), frames);
    }
    if animations.is_empty() {
        return Err(MotionSemanticsError::NoAnimations);
    }
    Ok(assess_character_motion_semantics(&animations))
}

fn assess_animation(
    name: &str,
    frames: &[RgbaImage],
    thresholds: &MotionSemanticsThresholdsV1,
) -> CharacterMotionAnimationReportV1 {
    let bodies = frames
        .iter()
        .filter_map(character_body_bbox)
        .collect::<Vec<_>>();
    let mut reasons = Vec::<String>::new();
    if frames.len() < 2 || bodies.len() != frames.len() {
        reasons.push("motion_frames_missing".into());
    }

    let mut foreground_differences = Vec::new();
    let mut lower_differences = Vec::new();
    let mut upper_flicker = Vec::new();
    let mut upper_alpha_drift = Vec::new();
    let mut lower_edge_ratios = Vec::new();
    let mut proximal_leg_differences = Vec::new();
    let mut knee_shin_differences = Vec::new();
    let mut foot_differences = Vec::new();
    if bodies.len() == frames.len() && frames.len() >= 2 {
        for index in 0..frames.len() {
            let next = (index + 1) % frames.len();
            foreground_differences.push(region_alpha_difference(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
                Region::Whole,
            ));
            lower_differences.push(region_alpha_difference(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
                Region::Lower,
            ));
            upper_flicker.push(stable_upper_body_flicker(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
            ));
            upper_alpha_drift.push(stable_upper_body_alpha_drift(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
            ));
            lower_edge_ratios.push(unsupported_lower_edge_ratio(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
            ));
            proximal_leg_differences.push(region_alpha_difference(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
                Region::ProximalLeg,
            ));
            knee_shin_differences.push(region_alpha_difference(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
                Region::KneeShin,
            ));
            foot_differences.push(region_alpha_difference(
                &frames[index],
                bodies[index],
                &frames[next],
                bodies[next],
                Region::Foot,
            ));
        }
    }

    let foreground_dynamic_degree = median(&foreground_differences);
    let lower_body_dynamic_degree = median(&lower_differences);
    let opposing_contact_change_ratio = if frames.len() >= 4 && bodies.len() == frames.len() {
        let half = frames.len() / 2;
        let first = region_alpha_difference(
            &frames[0],
            bodies[0],
            &frames[half],
            bodies[half],
            Region::Lower,
        );
        let second_start = frames.len() / 4;
        let second = region_alpha_difference(
            &frames[second_start],
            bodies[second_start],
            &frames[(second_start + half) % frames.len()],
            bodies[(second_start + half) % frames.len()],
            Region::Lower,
        );
        first.min(second)
    } else {
        0.0
    };
    let distinct_pose_count = if bodies.len() == frames.len() {
        distinct_lower_body_pose_count(frames, &bodies, 0.025)
    } else {
        0
    };
    let phase_order_score = phase_order_score(&lower_differences, frames, &bodies);
    let side_facing = name.contains("left") || name.contains("right");
    let (side_contact_passing_spread_ratio, side_contact_passing_cadence_score) =
        if side_facing && frames.len() == 4 && bodies.len() == frames.len() {
            side_contact_passing_cadence(&bodies, phase_order_score)
        } else {
            (0.0, 0.0)
        };
    let side_laterality_review_required = side_facing && frames.len() == 4;
    let stable_upper_body_flicker_ratio_max = maximum(&upper_flicker);
    let stable_upper_body_color_flicker_ratio_max = stable_upper_body_flicker_ratio_max;
    let stable_upper_body_alpha_drift_ratio_max = maximum(&upper_alpha_drift);
    let unsupported_lower_edge_ratio_max = maximum(&lower_edge_ratios);
    let lower_edge_median = median(&lower_edge_ratios);
    let lower_edge_outlier_ratio_max = if lower_edge_median <= 0.005 {
        if unsupported_lower_edge_ratio_max <= 0.005 {
            1.0
        } else {
            unsupported_lower_edge_ratio_max / 0.005
        }
    } else {
        unsupported_lower_edge_ratio_max / lower_edge_median
    };
    let foot_lobe_counts = if bodies.len() == frames.len() {
        frames
            .iter()
            .zip(&bodies)
            .map(|(frame, body)| foot_lobe_count(frame, *body))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let foot_lobe_count_max = foot_lobe_counts.iter().copied().max().unwrap_or_default();
    let proximal_leg_dynamic_degree = median(&proximal_leg_differences);
    let knee_shin_dynamic_degree = median(&knee_shin_differences);
    let foot_dynamic_degree = median(&foot_differences);
    let foot_to_knee_shin_motion_ratio = foot_dynamic_degree / knee_shin_dynamic_degree.max(0.005);

    if name.starts_with("walk") {
        if lower_body_dynamic_degree < thresholds.walk_min_lower_body_dynamic_degree {
            reasons.push("walk_motion_missing".into());
        }
        if lower_body_dynamic_degree > thresholds.walk_max_lower_body_dynamic_degree {
            reasons.push("walk_motion_excessive".into());
        }
        if opposing_contact_change_ratio < thresholds.walk_min_opposing_contact_change_ratio {
            reasons.push("walk_contact_poses_too_similar".into());
        }
        if distinct_pose_count < thresholds.walk_min_distinct_pose_count {
            reasons.push("walk_pose_diversity_missing".into());
        }
        if side_facing && frames.len() == 4 {
            if side_contact_passing_cadence_score
                < thresholds.walk_side_min_contact_passing_cadence_score
            {
                reasons.push("walk_side_contact_passing_cadence_invalid".into());
            }
        } else if phase_order_score < thresholds.walk_min_phase_order_score {
            reasons.push("walk_phase_order_invalid".into());
        }
        if side_facing
            && knee_shin_dynamic_degree < thresholds.walk_side_min_knee_shin_dynamic_degree
        {
            reasons.push("walk_side_knee_shin_motion_missing".into());
        }
        if side_facing
            && knee_shin_dynamic_degree > thresholds.walk_side_max_knee_shin_dynamic_degree
        {
            reasons.push("walk_side_knee_shin_motion_excessive".into());
        }
        if side_facing
            && foot_dynamic_degree >= 0.05
            && foot_to_knee_shin_motion_ratio
                > thresholds.walk_side_max_foot_to_knee_shin_motion_ratio
        {
            reasons.push("walk_side_foot_only_motion".into());
        }
    } else if name == "idle"
        && foreground_dynamic_degree < thresholds.idle_min_foreground_dynamic_degree
    {
        reasons.push("idle_motion_missing".into());
    }
    if stable_upper_body_color_flicker_ratio_max
        > thresholds.maximum_stable_upper_body_flicker_ratio
    {
        reasons.push("stable_upper_body_color_flicker".into());
    }
    if stable_upper_body_alpha_drift_ratio_max
        > thresholds.maximum_stable_upper_body_alpha_drift_ratio
    {
        reasons.push("stable_upper_body_alpha_drift".into());
    }
    if unsupported_lower_edge_ratio_max >= thresholds.lower_edge_ghost_minimum_ratio
        && lower_edge_outlier_ratio_max >= thresholds.lower_edge_ghost_minimum_outlier_ratio
    {
        reasons.push("lower_body_edge_ghost".into());
    }
    if foot_lobe_count_max > thresholds.maximum_foot_lobe_count {
        reasons.push("foot_lobe_count_exceeded".into());
    }

    reasons.sort();
    reasons.dedup();
    let mut recommended_retry_frames = Vec::<u8>::new();
    if name.starts_with("walk")
        && reasons.iter().any(|reason| {
            matches!(
                reason.as_str(),
                "walk_motion_missing"
                    | "walk_motion_excessive"
                    | "walk_contact_poses_too_similar"
                    | "walk_pose_diversity_missing"
                    | "walk_phase_order_invalid"
                    | "walk_side_contact_passing_cadence_invalid"
                    | "walk_side_knee_shin_motion_missing"
                    | "walk_side_knee_shin_motion_excessive"
                    | "walk_side_foot_only_motion"
            )
        })
    {
        recommended_retry_frames
            .extend((1..frames.len()).filter_map(|index| u8::try_from(index).ok()));
    }
    if reasons
        .iter()
        .any(|reason| reason == "stable_upper_body_color_flicker")
    {
        extend_transition_frames(&mut recommended_retry_frames, &upper_flicker);
    }
    if reasons
        .iter()
        .any(|reason| reason == "lower_body_edge_ghost")
    {
        extend_transition_frames(&mut recommended_retry_frames, &lower_edge_ratios);
    }
    if reasons
        .iter()
        .any(|reason| reason == "foot_lobe_count_exceeded")
    {
        recommended_retry_frames.extend(
            foot_lobe_counts
                .iter()
                .enumerate()
                .filter(|(_, count)| **count > thresholds.maximum_foot_lobe_count)
                .filter_map(|(index, _)| u8::try_from(index).ok()),
        );
    }
    recommended_retry_frames.sort_unstable();
    recommended_retry_frames.dedup();
    CharacterMotionAnimationReportV1 {
        name: name.into(),
        frame_count: frames.len(),
        foreground_dynamic_degree,
        lower_body_dynamic_degree,
        opposing_contact_change_ratio,
        distinct_pose_count,
        phase_order_score,
        stable_upper_body_flicker_ratio_max,
        stable_upper_body_alpha_drift_ratio_max,
        stable_upper_body_color_flicker_ratio_max,
        unsupported_lower_edge_ratio_max,
        lower_edge_outlier_ratio_max,
        foot_lobe_count_max,
        proximal_leg_dynamic_degree,
        knee_shin_dynamic_degree,
        foot_dynamic_degree,
        foot_to_knee_shin_motion_ratio,
        side_contact_passing_spread_ratio,
        side_contact_passing_cadence_score,
        side_laterality_review_required,
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        reasons,
        recommended_retry_frames,
    }
}

fn extend_transition_frames(target: &mut Vec<u8>, values: &[f32]) {
    let Some((index, _)) = values
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
    else {
        return;
    };
    if let Ok(index) = u8::try_from(index) {
        target.push(index);
    }
    if !values.is_empty() {
        let next = (index + 1) % values.len();
        if let Ok(next) = u8::try_from(next) {
            target.push(next);
        }
    }
}

#[derive(Clone, Copy)]
enum Region {
    Whole,
    Lower,
    ProximalLeg,
    KneeShin,
    Foot,
}

fn region_alpha_difference(
    first: &RgbaImage,
    first_body: FrameBbox,
    second: &RgbaImage,
    second_body: FrameBbox,
    region: Region,
) -> f32 {
    if first.dimensions() != second.dimensions() {
        return 1.0;
    }
    let left = first_body.left.floor().max(0.0) as i32;
    let right = first_body.right.ceil().min(first.width() as f32) as i32;
    let (region_start, region_end) = match region {
        Region::Whole => (0.0, 1.0),
        Region::Lower => (0.66, 1.0),
        Region::ProximalLeg => (0.74, 0.86),
        Region::KneeShin => (0.84, 0.94),
        Region::Foot => (0.94, 1.0),
    };
    let top = (first_body.top + first_body.height * region_start)
        .floor()
        .max(0.0) as i32;
    let bottom = (first_body.top + first_body.height * region_end)
        .ceil()
        .min(first.height() as f32) as i32;
    let uses_upper_body_alignment = matches!(
        region,
        Region::ProximalLeg | Region::KneeShin | Region::Foot
    );
    let first_center_x = if uses_upper_body_alignment {
        upper_body_center_x(first, first_body)
    } else {
        first_body.center_x
    };
    let second_center_x = if uses_upper_body_alignment {
        upper_body_center_x(second, second_body)
    } else {
        second_body.center_x
    };
    let dx = (first_center_x - second_center_x).round() as i32;
    let dy = (first_body.bottom_y - second_body.bottom_y).round() as i32;
    let mut changed = 0usize;
    let mut union = 0usize;
    for y in top..bottom {
        for x in left..right {
            let first_alpha = alpha(first, x, y);
            let second_alpha = alpha(second, x - dx, y - dy);
            if first_alpha || second_alpha {
                union += 1;
                changed += usize::from(first_alpha != second_alpha);
            }
        }
    }
    ratio(changed, union)
}

fn upper_body_center_x(image: &RgbaImage, body: FrameBbox) -> f32 {
    upper_body_anchor(image, body).0
}

fn upper_body_anchor(image: &RgbaImage, body: FrameBbox) -> (f32, f32) {
    let top = body.top.floor().max(0.0) as i32;
    let bottom = (body.top + body.height * 0.60)
        .ceil()
        .min(image.height() as f32) as i32;
    let mut minimum_x = image.width() as i32;
    let mut maximum_x = -1;
    for y in top..bottom {
        for x in 0..image.width() as i32 {
            if alpha(image, x, y) {
                minimum_x = minimum_x.min(x);
                maximum_x = maximum_x.max(x);
            }
        }
    }
    if maximum_x < minimum_x {
        (body.center_x, body.top + body.height * 0.30)
    } else {
        let mut minimum_y = image.height() as i32;
        let mut maximum_y = -1;
        for y in top..bottom {
            for x in 0..image.width() as i32 {
                if alpha(image, x, y) {
                    minimum_y = minimum_y.min(y);
                    maximum_y = maximum_y.max(y);
                }
            }
        }
        (
            (minimum_x + maximum_x + 1) as f32 / 2.0,
            (minimum_y + maximum_y + 1) as f32 / 2.0,
        )
    }
}

fn stable_upper_body_alpha_drift(
    first: &RgbaImage,
    first_body: FrameBbox,
    second: &RgbaImage,
    second_body: FrameBbox,
) -> f32 {
    if first.dimensions() != second.dimensions() {
        return 1.0;
    }
    let left = first_body.left.floor().max(0.0) as i32;
    let right = first_body.right.ceil().min(first.width() as f32) as i32;
    let top = first_body.top.floor().max(0.0) as i32;
    let bottom = (first_body.top + first_body.height * 0.58)
        .ceil()
        .min(first.height() as f32) as i32;
    let first_anchor = upper_body_anchor(first, first_body);
    let second_anchor = upper_body_anchor(second, second_body);
    let dx = (first_anchor.0 - second_anchor.0).round() as i32;
    let dy = (first_anchor.1 - second_anchor.1).round() as i32;
    let mut changed = 0usize;
    let mut union = 0usize;
    for y in top..bottom {
        for x in left..right {
            let first_alpha = alpha(first, x, y);
            let second_alpha = alpha(second, x - dx, y - dy);
            if first_alpha || second_alpha {
                union += 1;
                changed += usize::from(first_alpha != second_alpha);
            }
        }
    }
    ratio(changed, union)
}

fn stable_upper_body_flicker(
    first: &RgbaImage,
    first_body: FrameBbox,
    second: &RgbaImage,
    second_body: FrameBbox,
) -> f32 {
    if first.dimensions() != second.dimensions() {
        return 1.0;
    }
    let left = first_body.left.floor().max(0.0) as i32;
    let right = first_body.right.ceil().min(first.width() as f32) as i32;
    let top = first_body.top.floor().max(0.0) as i32;
    let bottom = (first_body.top + first_body.height * 0.58)
        .ceil()
        .min(first.height() as f32) as i32;
    let first_anchor = upper_body_anchor(first, first_body);
    let second_anchor = upper_body_anchor(second, second_body);
    let dx = (first_anchor.0 - second_anchor.0).round() as i32;
    let dy = (first_anchor.1 - second_anchor.1).round() as i32;
    let mut changed = 0usize;
    let mut stable = 0usize;
    for y in top..bottom {
        for x in left..right {
            if !interior_alpha_radius(first, x, y, 2)
                || !interior_alpha_radius(second, x - dx, y - dy, 2)
                || !locally_uniform_color(first, x, y)
                || !locally_uniform_color(second, x - dx, y - dy)
            {
                continue;
            }
            stable += 1;
            let first_pixel = first.get_pixel(x as u32, y as u32);
            let second_pixel = second.get_pixel((x - dx) as u32, (y - dy) as u32);
            let delta = first_pixel[0].abs_diff(second_pixel[0]) as u16
                + first_pixel[1].abs_diff(second_pixel[1]) as u16
                + first_pixel[2].abs_diff(second_pixel[2]) as u16;
            changed += usize::from(delta > 36);
        }
    }
    ratio(changed, stable)
}

fn unsupported_lower_edge_ratio(
    first: &RgbaImage,
    first_body: FrameBbox,
    second: &RgbaImage,
    second_body: FrameBbox,
) -> f32 {
    if first.dimensions() != second.dimensions() {
        return 1.0;
    }
    let dx = (first_body.center_x - second_body.center_x).round() as i32;
    let dy = (first_body.bottom_y - second_body.bottom_y).round() as i32;
    let top = (first_body.top + first_body.height * 0.66).floor().max(0.0) as i32;
    let bottom = first_body.bottom.ceil().min(first.height() as f32) as i32;
    let left = first_body.left.floor().max(0.0) as i32;
    let right = first_body.right.ceil().min(first.width() as f32) as i32;
    let mut edge_count = 0usize;
    let mut unsupported = 0usize;
    for y in top..bottom {
        for x in left..right {
            if !alpha_edge(first, x, y) {
                continue;
            }
            edge_count += 1;
            let target_x = x - dx;
            let target_y = y - dy;
            let supported = (-2..=2).any(|offset_y| {
                (-2..=2)
                    .any(|offset_x| alpha_edge(second, target_x + offset_x, target_y + offset_y))
            });
            unsupported += usize::from(!supported);
        }
    }
    ratio(unsupported, edge_count)
}

fn distinct_lower_body_pose_count(
    frames: &[RgbaImage],
    bodies: &[FrameBbox],
    minimum_difference: f32,
) -> usize {
    let mut selected = Vec::<usize>::new();
    for index in 0..frames.len() {
        if selected.iter().all(|selected_index| {
            region_alpha_difference(
                &frames[index],
                bodies[index],
                &frames[*selected_index],
                bodies[*selected_index],
                Region::Lower,
            ) >= minimum_difference
        }) {
            selected.push(index);
        }
    }
    selected.len()
}

fn phase_order_score(
    adjacent_differences: &[f32],
    frames: &[RgbaImage],
    bodies: &[FrameBbox],
) -> f32 {
    if adjacent_differences.len() < 4 || bodies.len() != frames.len() {
        return 0.0;
    }
    let minimum_adjacent = adjacent_differences
        .iter()
        .copied()
        .fold(f32::INFINITY, f32::min);
    let maximum_adjacent = maximum(adjacent_differences);
    let adjacent_balance = if maximum_adjacent <= f32::EPSILON {
        0.0
    } else {
        (minimum_adjacent / maximum_adjacent).clamp(0.0, 1.0)
    };
    let half = frames.len() / 2;
    let first_opposite = region_alpha_difference(
        &frames[0],
        bodies[0],
        &frames[half],
        bodies[half],
        Region::Lower,
    );
    let quarter = frames.len() / 4;
    let second_opposite = region_alpha_difference(
        &frames[quarter],
        bodies[quarter],
        &frames[(quarter + half) % frames.len()],
        bodies[(quarter + half) % frames.len()],
        Region::Lower,
    );
    let opposite_balance = if first_opposite.max(second_opposite) <= f32::EPSILON {
        0.0
    } else {
        first_opposite.min(second_opposite) / first_opposite.max(second_opposite)
    };
    let adjacent_median = median(adjacent_differences);
    let opposite_median = median(&[first_opposite, second_opposite]);
    let cyclic_locality = if opposite_median <= f32::EPSILON {
        0.0
    } else {
        ((opposite_median - adjacent_median) / opposite_median).clamp(0.0, 1.0)
    };
    let geometric_order =
        (adjacent_balance * 0.40 + opposite_balance * 0.20 + cyclic_locality * 0.40)
            .clamp(0.0, 1.0);
    if frames.len() == 4 {
        (geometric_order * 0.25 + four_pose_contact_order_score(frames, bodies) * 0.75)
            .clamp(0.0, 1.0)
    } else {
        geometric_order
    }
}

fn four_pose_contact_order_score(frames: &[RgbaImage], bodies: &[FrameBbox]) -> f32 {
    if frames.len() != 4 || bodies.len() != 4 {
        return 0.0;
    }
    let contacts = frames
        .iter()
        .zip(bodies)
        .map(|(frame, body)| {
            let top = (body.top + body.height * 0.66).floor().max(0.0) as i32;
            let bottom = body.bottom.ceil().min(frame.height() as f32) as i32;
            let left = body.left.floor().max(0.0) as i32;
            let right = body.right.ceil().min(frame.width() as f32) as i32;
            let center = body.center_x.round() as i32;
            let mut left_bottom = top;
            let mut right_bottom = top;
            for y in top..bottom {
                for x in left..right {
                    if !alpha(frame, x, y) {
                        continue;
                    }
                    if x < center {
                        left_bottom = left_bottom.max(y);
                    } else {
                        right_bottom = right_bottom.max(y);
                    }
                }
            }
            let difference = left_bottom - right_bottom;
            if difference.abs() <= 1 {
                0.0
            } else {
                difference as f32
            }
        })
        .collect::<Vec<_>>();
    let first_pair = f32::from(contacts[0] * contacts[1] > 0.0);
    let second_pair = f32::from(contacts[2] * contacts[3] > 0.0);
    let first_mean = (contacts[0] + contacts[1]) / 2.0;
    let second_mean = (contacts[2] + contacts[3]) / 2.0;
    let opposing_halves = f32::from(first_mean * second_mean < 0.0);
    let scale = (bodies.iter().map(|body| body.height).sum::<f32>() / 4.0 * 0.05).max(2.0);
    let contact_strength = contacts
        .iter()
        .map(|contact| (contact.abs() / scale).clamp(0.0, 1.0))
        .sum::<f32>()
        / 4.0;
    (first_pair * 0.25 + second_pair * 0.25 + opposing_halves * 0.30 + contact_strength * 0.20)
        .clamp(0.0, 1.0)
}

fn side_contact_passing_cadence(bodies: &[FrameBbox], legacy_phase_order_score: f32) -> (f32, f32) {
    if bodies.len() != 4 {
        return (0.0, 0.0);
    }
    let spans = bodies
        .iter()
        .map(|body| body.width / body.height.max(1.0))
        .collect::<Vec<_>>();
    let contacts = [spans[0], spans[2]];
    let passing = [spans[1], spans[3]];
    let contact_median = (contacts[0] + contacts[1]) / 2.0;
    let passing_median = (passing[0] + passing[1]) / 2.0;
    let spread_ratio = (contact_median - passing_median).max(0.0);
    let strict_separation = (contacts[0].min(contacts[1]) - passing[0].max(passing[1])).max(0.0);
    let separation_signal = (strict_separation / 0.03).clamp(0.0, 1.0);
    let contact_balance = contacts[0].min(contacts[1]) / contacts[0].max(contacts[1]).max(0.001);
    let passing_balance = passing[0].min(passing[1]) / passing[0].max(passing[1]).max(0.001);
    let width_cadence_score =
        (separation_signal * 0.60 + contact_balance * 0.20 + passing_balance * 0.20)
            .clamp(0.0, 1.0);
    (
        spread_ratio,
        width_cadence_score.max(legacy_phase_order_score),
    )
}

fn foot_lobe_count(frame: &RgbaImage, body: FrameBbox) -> usize {
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
                frame.get_pixel(x as u32, y as u32)[3] > 48;
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

fn alpha(image: &RgbaImage, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && x < image.width() as i32
        && y < image.height() as i32
        && image.get_pixel(x as u32, y as u32)[3] > 48
}

fn interior_alpha_radius(image: &RgbaImage, x: i32, y: i32, radius: i32) -> bool {
    alpha(image, x, y)
        && (-radius..=radius)
            .all(|offset| alpha(image, x + offset, y) && alpha(image, x, y + offset))
}

fn locally_uniform_color(image: &RgbaImage, x: i32, y: i32) -> bool {
    if !alpha(image, x, y) {
        return false;
    }
    let center = image.get_pixel(x as u32, y as u32);
    [
        (-2, 0),
        (-1, 0),
        (1, 0),
        (2, 0),
        (0, -2),
        (0, -1),
        (0, 1),
        (0, 2),
    ]
    .into_iter()
    .all(|(dx, dy)| {
        if !alpha(image, x + dx, y + dy) {
            return false;
        }
        let neighbor = image.get_pixel((x + dx) as u32, (y + dy) as u32);
        let delta = center[0].abs_diff(neighbor[0]) as u16
            + center[1].abs_diff(neighbor[1]) as u16
            + center[2].abs_diff(neighbor[2]) as u16;
        delta <= 48
    })
}

fn alpha_edge(image: &RgbaImage, x: i32, y: i32) -> bool {
    alpha(image, x, y)
        && [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .iter()
            .any(|(dx, dy)| !alpha(image, x + dx, y + dy))
}

fn ratio(numerator: usize, denominator: usize) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn maximum(values: &[f32]) -> f32 {
    values.iter().copied().fold(0.0, f32::max)
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
    use image::{ImageBuffer, Rgba};

    use super::*;

    fn walk_frame(phase: usize, ghost: bool) -> RgbaImage {
        let mut image = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 12..82 {
            for x in 38..90 {
                image.put_pixel(x, y, Rgba([72, 91, 48, 255]));
            }
        }
        let legs = match phase {
            0 => [(40, 54, 78, 119), (70, 82, 86, 112)],
            1 => [(44, 56, 78, 116), (62, 74, 84, 112)],
            2 => [(50, 62, 86, 112), (70, 84, 78, 119)],
            _ => [(48, 60, 84, 112), (64, 76, 78, 116)],
        };
        for (left, right, top, bottom) in legs {
            for y in top..bottom {
                for x in left..right {
                    image.put_pixel(x, y, Rgba([86, 57, 38, 255]));
                }
            }
        }
        if ghost {
            for y in 104..119 {
                for x in 98..108 {
                    image.put_pixel(x, y, Rgba([86, 57, 38, 255]));
                }
            }
        }
        image
    }

    fn foot_only_walk_frame(phase: usize) -> RgbaImage {
        let mut image = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 12..88 {
            for x in 38..90 {
                image.put_pixel(x, y, Rgba([72, 91, 48, 255]));
            }
        }
        // Both leg columns remain mechanically fixed through every frame.
        for y in 78..113 {
            for x in 46..59 {
                image.put_pixel(x, y, Rgba([86, 57, 38, 255]));
            }
            for x in 69..82 {
                image.put_pixel(x, y, Rgba([86, 57, 38, 255]));
            }
        }
        // Only the boot silhouettes move, reproducing the failure mode where
        // feet slide beneath frozen thighs and knees.
        let boots = match phase {
            0 => [(28, 59), (69, 92)],
            1 => [(40, 68), (63, 88)],
            2 => [(39, 63), (69, 100)],
            _ => [(34, 59), (62, 90)],
        };
        for (left, right) in boots {
            for y in 113..121 {
                for x in left..right {
                    image.put_pixel(x, y, Rgba([98, 66, 43, 255]));
                }
            }
        }
        image
    }

    fn animation(name: &str, frames: Vec<RgbaImage>) -> BTreeMap<String, Vec<RgbaImage>> {
        [(name.into(), frames)].into_iter().collect()
    }

    #[test]
    fn accepts_four_distinct_walk_phases() {
        let report = assess_character_motion_semantics(&animation(
            "walk_right",
            (0..4).map(|phase| walk_frame(phase, false)).collect(),
        ));
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:#?}");
    }

    #[test]
    fn blocks_static_walk_even_when_colors_flicker() {
        let mut frames = vec![walk_frame(0, false); 4];
        for pixel in frames[1].pixels_mut() {
            if pixel[3] > 0 {
                pixel[0] = pixel[0].saturating_add(40);
            }
        }
        let report = assess_character_motion_semantics(&animation("walk_down", frames));
        let walk = &report.animations[0];
        assert_eq!(walk.verdict, ConsistencyVerdict::Blocked);
        assert!(walk.reasons.contains(&"walk_motion_missing".into()));
        assert!(walk
            .reasons
            .contains(&"stable_upper_body_color_flicker".into()));
    }

    #[test]
    fn blocks_third_foot_lobe() {
        let mut frames = (0..4)
            .map(|phase| walk_frame(phase, false))
            .collect::<Vec<_>>();
        frames[2] = walk_frame(2, true);
        let report = assess_character_motion_semantics(&animation("walk_down", frames));
        let walk = &report.animations[0];
        assert_eq!(walk.verdict, ConsistencyVerdict::Blocked);
        assert!(
            walk.reasons.contains(&"foot_lobe_count_exceeded".into())
                || walk.reasons.contains(&"lower_body_edge_ghost".into()),
            "{walk:#?}"
        );
    }

    #[test]
    fn blocks_side_walk_when_only_boots_move_beneath_fixed_legs() {
        let report = assess_character_motion_semantics(&animation(
            "walk_right",
            (0..4).map(foot_only_walk_frame).collect(),
        ));
        let walk = &report.animations[0];
        assert_eq!(walk.verdict, ConsistencyVerdict::Blocked, "{walk:#?}");
        assert!(
            walk.reasons
                .iter()
                .any(|reason| reason == "walk_side_knee_shin_motion_missing"
                    || reason == "walk_side_foot_only_motion"),
            "{walk:#?}"
        );
        assert!(
            walk.foot_dynamic_degree > walk.knee_shin_dynamic_degree,
            "{walk:#?}"
        );
    }

    #[test]
    fn blocks_side_walk_when_knee_shin_motion_exceeds_profile() {
        let thresholds = MotionSemanticsThresholdsV1 {
            walk_side_max_knee_shin_dynamic_degree: 0.10,
            ..Default::default()
        };
        let report = assess_character_motion_semantics_with_thresholds(
            &animation(
                "walk_right",
                (0..4).map(|phase| walk_frame(phase, false)).collect(),
            ),
            thresholds,
        );
        let walk = &report.animations[0];
        assert_eq!(walk.verdict, ConsistencyVerdict::Blocked, "{walk:#?}");
        assert!(
            walk.reasons
                .contains(&"walk_side_knee_shin_motion_excessive".into()),
            "{walk:#?}"
        );
    }

    #[test]
    fn blocks_out_of_order_walk_phases() {
        let report = assess_character_motion_semantics(&animation(
            "walk_right",
            [0, 2, 1, 3]
                .into_iter()
                .map(|phase| walk_frame(phase, false))
                .collect(),
        ));
        let walk = &report.animations[0];
        assert_eq!(walk.verdict, ConsistencyVerdict::Blocked, "{walk:#?}");
        assert!(
            walk.reasons
                .contains(&"walk_side_contact_passing_cadence_invalid".into()),
            "{walk:#?}"
        );
    }

    #[test]
    fn blocks_excessive_walk_motion() {
        let frames = (0..4)
            .map(|phase| {
                let mut frame = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
                for y in 12..84 {
                    for x in 38..90 {
                        frame.put_pixel(x, y, Rgba([72, 91, 48, 255]));
                    }
                }
                for y in 84..120 {
                    for x in 34..94 {
                        if ((x / 3) + phase) % 2 == 0 {
                            frame.put_pixel(x, y, Rgba([86, 57, 38, 255]));
                        }
                    }
                }
                frame
            })
            .collect();
        let report = assess_character_motion_semantics(&animation("walk_down", frames));
        let walk = &report.animations[0];
        assert_eq!(walk.verdict, ConsistencyVerdict::Blocked, "{walk:#?}");
        assert!(
            walk.reasons.contains(&"walk_motion_excessive".into()),
            "{walk:#?}"
        );
    }

    #[test]
    fn accepts_small_idle_bob_but_rejects_frozen_idle() {
        let base = walk_frame(0, false);
        let mut bobbed = base.clone();
        for y in 30..72 {
            for x in 42..46 {
                bobbed.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
            for x in 82..86 {
                bobbed.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        let passing = assess_character_motion_semantics(&animation(
            "idle",
            vec![base.clone(), bobbed.clone(), base.clone(), bobbed],
        ));
        assert_eq!(
            passing.verdict,
            ConsistencyVerdict::GameReady,
            "{passing:#?}"
        );

        let blocked = assess_character_motion_semantics(&animation("idle", vec![base; 4]));
        assert_eq!(blocked.verdict, ConsistencyVerdict::Blocked);
        assert!(blocked.animations[0]
            .reasons
            .contains(&"idle_motion_missing".into()));
    }
}
