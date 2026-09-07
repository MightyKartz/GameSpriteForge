use std::collections::{BTreeMap, BTreeSet, VecDeque};

use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::asset_project::{character_body_bbox, CharacterEquipmentKindV1, ConsistencyVerdict};
use crate::frames::FrameBbox;

pub const HAND_EQUIPMENT_CONTACT_PROFILE: &str = "hand-equipment-contact@1.1.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EquipmentSideV1 {
    Left,
    Right,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquipmentLockV1 {
    pub animation: String,
    pub equipment_kind: String,
    pub side: EquipmentSideV1,
    pub reference_frame: usize,
    pub grip_anchor_x_px: f32,
    pub grip_anchor_y_px: f32,
    pub shaft_axis_x_px: f32,
    pub shaft_visible_length_px: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandEquipmentContactAnimationReportV1 {
    pub name: String,
    pub frame_count: usize,
    pub equipment_expected: bool,
    pub equipment_detected_frames: usize,
    pub grip_contact_frames: usize,
    pub equipment_side_flip_frames: usize,
    pub long_thin_protrusion_frames: usize,
    pub maximum_grip_anchor_drift_px: f32,
    pub maximum_hand_region_area_outlier_ratio: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandEquipmentContactReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub equipment_kind: String,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub locks: Vec<EquipmentLockV1>,
    pub animations: Vec<HandEquipmentContactAnimationReportV1>,
}

#[derive(Debug, Clone, Copy)]
struct FrameContactMetrics {
    detected: bool,
    shaft_present: bool,
    side: EquipmentSideV1,
    grip_x: f32,
    grip_y: f32,
    shaft_x: f32,
    shaft_length: f32,
    hand_region_area: f32,
    long_thin_protrusion: bool,
}

pub fn assess_hand_equipment_contact(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    character_prompt: &str,
) -> HandEquipmentContactReportV1 {
    assess_hand_equipment_contact_with_kind(animations, staff_like_kind(character_prompt))
}

pub fn assess_hand_equipment_contact_with_kind(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    equipment_kind: CharacterEquipmentKindV1,
) -> HandEquipmentContactReportV1 {
    let equipment_expected = equipment_kind != CharacterEquipmentKindV1::None;
    let mut locks = Vec::new();
    let mut reports = Vec::with_capacity(animations.len());

    for (name, frames) in animations {
        let metrics = frames
            .iter()
            .map(analyze_frame)
            .collect::<Vec<Option<FrameContactMetrics>>>();
        let detected = metrics
            .iter()
            .flatten()
            .filter(|item| {
                if equipment_expected {
                    item.detected
                } else {
                    item.shaft_present
                }
            })
            .count();
        let dominant_side = dominant_side(&metrics);
        let usable = metrics
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.filter(|item| item.detected).map(|item| (index, item)))
            .collect::<Vec<_>>();
        let median_grip_y = median(usable.iter().map(|(_, item)| item.grip_y).collect());
        let median_grip_x = median(usable.iter().map(|(_, item)| item.grip_x).collect());
        let median_shaft_x = median(usable.iter().map(|(_, item)| item.shaft_x).collect());
        let median_shaft_length =
            median(usable.iter().map(|(_, item)| item.shaft_length).collect());
        let median_hand_area = median(
            usable
                .iter()
                .map(|(_, item)| item.hand_region_area)
                .collect(),
        );
        let grip_contact_frames = usable
            .iter()
            .filter(|(_, item)| item.grip_y.is_finite())
            .count();
        let side_flip_frames = usable
            .iter()
            .filter(|(_, item)| item.side != dominant_side)
            .count();
        let long_thin_protrusion_frames = usable
            .iter()
            .filter(|(_, item)| item.long_thin_protrusion)
            .count();
        let maximum_grip_anchor_drift_px = usable
            .iter()
            .map(|(_, item)| {
                ((item.grip_x - median_grip_x).powi(2) + (item.grip_y - median_grip_y).powi(2))
                    .sqrt()
            })
            .fold(0.0, f32::max);
        let maximum_hand_region_area_outlier_ratio = usable
            .iter()
            .map(|(_, item)| {
                (item.hand_region_area - median_hand_area).abs() / median_hand_area.max(1.0)
            })
            .fold(0.0, f32::max);

        if equipment_expected && !usable.is_empty() {
            let reference_frame = usable
                .iter()
                .min_by(|(_, left), (_, right)| {
                    let left_distance =
                        (left.grip_y - median_grip_y).abs() + (left.shaft_x - median_shaft_x).abs();
                    let right_distance = (right.grip_y - median_grip_y).abs()
                        + (right.shaft_x - median_shaft_x).abs();
                    left_distance.total_cmp(&right_distance)
                })
                .map(|(index, _)| *index)
                .unwrap_or_default();
            locks.push(EquipmentLockV1 {
                animation: name.clone(),
                equipment_kind: equipment_kind_label(equipment_kind).into(),
                side: dominant_side,
                reference_frame,
                grip_anchor_x_px: median_grip_x,
                grip_anchor_y_px: median_grip_y,
                shaft_axis_x_px: median_shaft_x,
                shaft_visible_length_px: median_shaft_length,
            });
        }

        let mut reasons = Vec::new();
        if equipment_expected {
            if detected != frames.len() {
                reasons.push("held_equipment_missing".into());
            }
            if grip_contact_frames != frames.len() {
                reasons.push("grip_contact_missing".into());
            }
            if side_flip_frames > 0 {
                reasons.push("equipment_side_flip".into());
            }
            if maximum_grip_anchor_drift_px > 6.0 {
                reasons.push("grip_anchor_drift".into());
            }
            if maximum_hand_region_area_outlier_ratio > 0.30 {
                reasons.push("hand_region_area_outlier".into());
            }
            if long_thin_protrusion_frames > 0 {
                reasons.push("hand_long_thin_protrusion".into());
            }
        } else {
            let persistent_threshold = frames.len().saturating_mul(3).div_ceil(4);
            if detected >= persistent_threshold.max(2) {
                reasons.push("unexpected_held_equipment".into());
            }
        }
        let verdict = if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        };
        reports.push(HandEquipmentContactAnimationReportV1 {
            name: name.clone(),
            frame_count: frames.len(),
            equipment_expected,
            equipment_detected_frames: detected,
            grip_contact_frames,
            equipment_side_flip_frames: side_flip_frames,
            long_thin_protrusion_frames,
            maximum_grip_anchor_drift_px,
            maximum_hand_region_area_outlier_ratio,
            verdict,
            reasons,
        });
    }

    HandEquipmentContactReportV1 {
        schema_version: "1".into(),
        profile: HAND_EQUIPMENT_CONTACT_PROFILE.into(),
        equipment_kind: equipment_kind_label(equipment_kind).into(),
        verdict: if reports
            .iter()
            .all(|report| report.verdict == ConsistencyVerdict::GameReady)
        {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        locks,
        animations: reports,
    }
}

fn staff_like_kind(prompt: &str) -> CharacterEquipmentKindV1 {
    let prompt = prompt.to_ascii_lowercase();
    if ["unarmed", "no staff", "without a staff", "徒手", "无法杖"]
        .iter()
        .any(|keyword| prompt.contains(keyword))
    {
        return CharacterEquipmentKindV1::None;
    }
    if ["staff", "wand", "spear", "polearm", "法杖", "魔杖", "长矛"]
        .iter()
        .any(|keyword| prompt.contains(keyword))
    {
        CharacterEquipmentKindV1::StaffLike
    } else {
        CharacterEquipmentKindV1::None
    }
}

fn equipment_kind_label(kind: CharacterEquipmentKindV1) -> &'static str {
    match kind {
        CharacterEquipmentKindV1::None => "none",
        CharacterEquipmentKindV1::StaffLike => "staff_like",
    }
}

fn analyze_frame(frame: &RgbaImage) -> Option<FrameContactMetrics> {
    let body = character_body_bbox(frame)?;
    let left = side_metrics(frame, body, EquipmentSideV1::Left);
    let right = side_metrics(frame, body, EquipmentSideV1::Right);
    let mut result = if left.shaft_length > right.shaft_length {
        left
    } else {
        right
    };
    result.shaft_present |=
        detached_long_thin_component(frame, body.height) || prominent_thin_side_column(frame, body);
    Some(result)
}

fn prominent_thin_side_column(frame: &RgbaImage, body: FrameBbox) -> bool {
    let top = body.top.floor().max(0.0) as u32;
    let bottom = body.bottom_y.ceil().min(frame.height() as f32) as u32;
    let minimum_run = (body.height * 0.35).ceil() as usize;
    let minimum_offset = (body.width * 0.42).max(8.0);
    let mut candidates = Vec::new();
    for x in 0..frame.width() {
        let mut run = 0usize;
        let mut maximum_run = 0usize;
        for y in top..bottom {
            if frame.get_pixel(x, y)[3] > 48 {
                run += 1;
                maximum_run = maximum_run.max(run);
            } else {
                run = 0;
            }
        }
        if maximum_run >= minimum_run {
            candidates.push(x);
        }
    }
    if candidates.is_empty() {
        return false;
    }
    let maximum_group_width = (body.width * 0.18).ceil().max(3.0) as u32;
    let mut start = candidates[0];
    let mut previous = candidates[0];
    for x in candidates.into_iter().skip(1) {
        if x == previous + 1 {
            previous = x;
            continue;
        }
        let center = (start + previous) as f32 / 2.0;
        if previous - start < maximum_group_width
            && (center - body.center_x).abs() >= minimum_offset
        {
            return true;
        }
        start = x;
        previous = x;
    }
    let center = (start + previous) as f32 / 2.0;
    previous - start < maximum_group_width && (center - body.center_x).abs() >= minimum_offset
}

fn detached_long_thin_component(frame: &RgbaImage, body_height: f32) -> bool {
    let mut foreground = BTreeSet::new();
    for (x, y, pixel) in frame.enumerate_pixels() {
        if pixel[3] > 48 {
            foreground.insert((x, y));
        }
    }
    let mut visited = BTreeSet::new();
    let mut components = Vec::new();
    for start in foreground.iter().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut left = start.0;
        let mut right = start.0;
        let mut top = start.1;
        let mut bottom = start.1;
        let mut area = 0usize;
        while let Some((x, y)) = queue.pop_front() {
            area += 1;
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
            for (next_x, next_y) in orthogonal_neighbors(x, y, frame.width(), frame.height()) {
                if foreground.contains(&(next_x, next_y)) && visited.insert((next_x, next_y)) {
                    queue.push_back((next_x, next_y));
                }
            }
        }
        components.push((area, right - left + 1, bottom - top + 1));
    }
    components.sort_by_key(|(area, _, _)| std::cmp::Reverse(*area));
    components.into_iter().skip(1).any(|(area, width, height)| {
        area >= 12
            && height as f32 >= body_height * 0.30
            && width as f32 <= (height as f32 * 0.35).max(4.0)
    })
}

fn orthogonal_neighbors(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
}

fn side_metrics(frame: &RgbaImage, body: FrameBbox, side: EquipmentSideV1) -> FrameContactMetrics {
    let body_left = body.left.floor().max(0.0) as i32;
    let body_right = body.right.ceil().min(frame.width() as f32) as i32;
    let top = body.top.floor().max(0.0) as i32;
    let bottom = body.bottom_y.ceil().min(frame.height() as f32) as i32;
    let outside = match side {
        // `character_body_bbox` is quantile based, so leave a two-pixel guard
        // band before treating a narrow side column as held equipment.
        EquipmentSideV1::Left => 0..body_left.saturating_sub(2).max(0),
        EquipmentSideV1::Right => {
            body_right.saturating_add(2).min(frame.width() as i32)..frame.width() as i32
        }
        EquipmentSideV1::Unknown => 0..0,
    };
    let mut shaft_x = 0i32;
    let mut shaft_length = 0usize;
    for x in outside.clone() {
        let count = (top..bottom).filter(|y| foreground(frame, x, *y)).count();
        if count > shaft_length {
            shaft_length = count;
            shaft_x = x;
        }
    }
    let boundary_x = match side {
        EquipmentSideV1::Left => body_left,
        EquipmentSideV1::Right => body_right.saturating_sub(1),
        EquipmentSideV1::Unknown => body.center_x.round() as i32,
    };
    // A held staff must occupy a persistent vertical span and sit distinctly
    // outside the body quantile. Without the offset requirement, the first
    // trimmed torso column was misclassified as a shaft on ordinary sprites.
    let shaft_offset = (shaft_x - boundary_x).unsigned_abs() as f32;
    let detected =
        shaft_length as f32 >= body.height * 0.35 && shaft_offset >= (body.width * 0.08).max(3.0);
    let mut contact_rows = Vec::new();
    let mut protrusion_rows = Vec::new();
    for y in top..bottom {
        let extent = connected_outward_extent(frame, boundary_x, y, side, 2);
        if extent >= 3 {
            contact_rows.push(y as f32);
        }
        if extent as f32 >= (body.width * 0.45).max(10.0) {
            protrusion_rows.push(y as f32);
        }
    }
    let grip_y = median(contact_rows.clone());
    let long_thin_protrusion = protrusion_rows
        .iter()
        .any(|row| (*row - grip_y).abs() > 3.0);
    let hand_region_area = if grip_y.is_finite() {
        let x_range = match side {
            EquipmentSideV1::Left => boundary_x - 16..boundary_x + 4,
            EquipmentSideV1::Right => boundary_x - 3..boundary_x + 17,
            EquipmentSideV1::Unknown => boundary_x - 8..boundary_x + 8,
        };
        let grip_y = grip_y.round() as i32;
        (grip_y - 8..=grip_y + 8)
            .flat_map(|y| x_range.clone().map(move |x| (x, y)))
            .filter(|(x, y)| foreground(frame, *x, *y))
            .count() as f32
    } else {
        0.0
    };
    FrameContactMetrics {
        detected: detected && grip_y.is_finite(),
        shaft_present: detected,
        side,
        grip_x: boundary_x as f32,
        grip_y,
        shaft_x: shaft_x as f32,
        shaft_length: shaft_length as f32,
        hand_region_area,
        long_thin_protrusion,
    }
}

fn connected_outward_extent(
    frame: &RgbaImage,
    boundary_x: i32,
    y: i32,
    side: EquipmentSideV1,
    allowed_gap: usize,
) -> usize {
    let direction = if side == EquipmentSideV1::Left { -1 } else { 1 };
    let mut extent = 0usize;
    let mut gap = 0usize;
    for step in 0..frame.width().min(64) as i32 {
        let x = boundary_x + direction * step;
        if x < 0 || x >= frame.width() as i32 {
            break;
        }
        if foreground(frame, x, y) {
            extent = step as usize + 1;
            gap = 0;
        } else {
            gap += 1;
            if gap > allowed_gap {
                break;
            }
        }
    }
    extent
}

fn dominant_side(metrics: &[Option<FrameContactMetrics>]) -> EquipmentSideV1 {
    let left = metrics
        .iter()
        .flatten()
        .filter(|item| item.detected && item.side == EquipmentSideV1::Left)
        .count();
    let right = metrics
        .iter()
        .flatten()
        .filter(|item| item.detected && item.side == EquipmentSideV1::Right)
        .count();
    match left.cmp(&right) {
        std::cmp::Ordering::Greater => EquipmentSideV1::Left,
        std::cmp::Ordering::Less => EquipmentSideV1::Right,
        std::cmp::Ordering::Equal => EquipmentSideV1::Unknown,
    }
}

fn foreground(frame: &RgbaImage, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && x < frame.width() as i32
        && y < frame.height() as i32
        && frame.get_pixel(x as u32, y as u32)[3] > 48
}

fn median(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return f32::NAN;
    }
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

    fn staff_frame(extra_finger: bool) -> RgbaImage {
        let mut frame = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 12..56 {
            for x in 22..42 {
                frame.put_pixel(x, y, Rgba([120, 80, 55, 255]));
            }
        }
        for y in 18..59 {
            frame.put_pixel(48, y, Rgba([95, 70, 38, 255]));
        }
        for y in 32..35 {
            for x in 40..=48 {
                frame.put_pixel(x, y, Rgba([150, 100, 70, 255]));
            }
        }
        if extra_finger {
            for x in 40..59 {
                frame.put_pixel(x, 27, Rgba([150, 100, 70, 255]));
            }
        }
        frame
    }

    fn unarmed_frame() -> RgbaImage {
        let mut frame = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 12..56 {
            for x in 22..42 {
                frame.put_pixel(x, y, Rgba([120, 80, 55, 255]));
            }
        }
        frame
    }

    #[test]
    fn staff_gate_accepts_a_stable_compact_grip() {
        let report = assess_hand_equipment_contact(
            &BTreeMap::from([("walk_right".into(), vec![staff_frame(false); 8])]),
            "hooded ranger holding a wooden staff",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(report.locks.len(), 1);
        assert_eq!(report.animations[0].grip_contact_frames, 8);
    }

    #[test]
    fn staff_gate_blocks_a_long_finger_like_protrusion() {
        let mut frames = vec![staff_frame(false); 8];
        frames[3] = staff_frame(true);
        let report = assess_hand_equipment_contact(
            &BTreeMap::from([("walk_right".into(), frames)]),
            "hooded ranger holding a wooden staff",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.animations[0]
            .reasons
            .contains(&"hand_long_thin_protrusion".into()));
    }

    #[test]
    fn undeclared_staff_is_blocked_as_reference_content_leakage() {
        let report = assess_hand_equipment_contact(
            &BTreeMap::from([("walk_right".into(), vec![staff_frame(false); 8])]),
            "unarmed hooded ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.animations[0]
            .reasons
            .contains(&"unexpected_held_equipment".into()));
    }

    #[test]
    fn unarmed_characters_without_staff_pass() {
        let report = assess_hand_equipment_contact(
            &BTreeMap::from([("walk_right".into(), vec![unarmed_frame(); 8])]),
            "unarmed hooded ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.locks.is_empty());
    }
}
