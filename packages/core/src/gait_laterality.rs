use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::asset_project::{character_body_bbox, ConsistencyVerdict};

pub const GAIT_LATERALITY_PROFILE: &str = "gait-laterality@1.0.0";
pub const WALK_LATERALITY_NOT_ALTERNATING: &str = "walk_laterality_not_alternating";
pub const WALK_LATERALITY_AMBIGUOUS: &str = "walk_laterality_ambiguous";

const ALPHA_THRESHOLD: u8 = 32;
const LOWER_BODY_START_RATIO: f32 = 0.62;
const MINIMUM_CONTACT_SIGNAL: f32 = 0.02;
const MINIMUM_OPPOSITE_DELTA: f32 = 0.04;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenSideV1 {
    Left,
    Right,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GaitLateralityFrameReportV1 {
    pub frame_index: u8,
    pub expected_side: ScreenSideV1,
    pub observed_side: ScreenSideV1,
    pub contact_signal: f32,
    pub confidence: f32,
    pub screen_left_bottom_y: u32,
    pub screen_right_bottom_y: u32,
    pub screen_left_area: u32,
    pub screen_right_area: u32,
    pub matches_expected: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GaitLateralityReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub minimum_contact_signal: f32,
    pub minimum_opposite_delta: f32,
    pub opposite_contact_delta: f32,
    pub opposite_passing_delta: f32,
    pub frames: Vec<GaitLateralityFrameReportV1>,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_retry_frames: Vec<u8>,
}

/// Evaluates viewer-space gait laterality for the four-phase V9.2 front/down
/// contract. The report deliberately does not infer anatomical ownership from
/// appearance: it proves only that screen-left and screen-right contact/support
/// extrema alternate in the required raster order.
pub fn assess_front_gait_laterality(
    animation: &str,
    frames: &[RgbaImage],
) -> GaitLateralityReportV1 {
    let expected = [
        ScreenSideV1::Left,
        ScreenSideV1::Left,
        ScreenSideV1::Right,
        ScreenSideV1::Right,
    ];
    let frame_reports = frames
        .iter()
        .take(4)
        .enumerate()
        .map(|(index, frame)| assess_frame(index as u8, expected[index], frame))
        .collect::<Vec<_>>();

    let mut reasons = Vec::new();
    if animation != "walk_down" || frames.len() != 4 || frame_reports.len() != 4 {
        reasons.push("walk_laterality_frame_contract_invalid".into());
    }
    let ambiguous = frame_reports
        .iter()
        .any(|frame| frame.observed_side == ScreenSideV1::Ambiguous);
    if ambiguous {
        reasons.push(WALK_LATERALITY_AMBIGUOUS.into());
    }
    let opposite_contact_delta = opposite_delta(&frame_reports, 0, 2);
    let opposite_passing_delta = opposite_delta(&frame_reports, 1, 3);
    let alternating = frame_reports.len() == 4
        && frame_reports.iter().all(|frame| frame.matches_expected)
        && opposite_contact_delta >= MINIMUM_OPPOSITE_DELTA
        && opposite_passing_delta >= MINIMUM_OPPOSITE_DELTA;
    if !alternating {
        reasons.push(WALK_LATERALITY_NOT_ALTERNATING.into());
    }
    reasons.sort();
    reasons.dedup();

    let mut recommended_retry_frames = frame_reports
        .iter()
        .filter(|frame| frame.frame_index >= 2 && !frame.matches_expected)
        .map(|frame| frame.frame_index)
        .collect::<Vec<_>>();
    if !alternating && recommended_retry_frames.is_empty() {
        // Frames 0 and 1 establish phase A. Correct the opposite half rather
        // than spending on already-coherent seed poses.
        recommended_retry_frames.extend([2, 3]);
    }
    recommended_retry_frames.sort_unstable();
    recommended_retry_frames.dedup();

    GaitLateralityReportV1 {
        schema_version: "1".into(),
        profile: GAIT_LATERALITY_PROFILE.into(),
        animation: animation.into(),
        minimum_contact_signal: MINIMUM_CONTACT_SIGNAL,
        minimum_opposite_delta: MINIMUM_OPPOSITE_DELTA,
        opposite_contact_delta,
        opposite_passing_delta,
        frames: frame_reports,
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        reasons,
        recommended_retry_frames,
    }
}

fn assess_frame(
    frame_index: u8,
    expected_side: ScreenSideV1,
    frame: &RgbaImage,
) -> GaitLateralityFrameReportV1 {
    let Some(body) = character_body_bbox(frame) else {
        return GaitLateralityFrameReportV1 {
            frame_index,
            expected_side,
            observed_side: ScreenSideV1::Ambiguous,
            contact_signal: 0.0,
            confidence: 0.0,
            screen_left_bottom_y: 0,
            screen_right_bottom_y: 0,
            screen_left_area: 0,
            screen_right_area: 0,
            matches_expected: false,
        };
    };
    let left = body.left.floor().max(0.0) as u32;
    let right = body.right.ceil().min(frame.width() as f32) as u32;
    let top = body.top.floor().max(0.0) as u32;
    let bottom = body.bottom.ceil().min(frame.height() as f32) as u32;
    let lower_top = (body.top + body.height * LOWER_BODY_START_RATIO)
        .floor()
        .max(0.0) as u32;
    let midpoint = left + (right.saturating_sub(left)) / 2;
    let (screen_left_bottom_y, screen_left_area) =
        half_evidence(frame, left, midpoint, lower_top, bottom);
    let (screen_right_bottom_y, screen_right_area) =
        half_evidence(frame, midpoint, right, lower_top, bottom);
    let height = bottom.saturating_sub(top).max(1) as f32;
    let bottom_delta = (screen_left_bottom_y as f32 - screen_right_bottom_y as f32) / height;
    // Garments such as capes and long coats can dominate the lower-half area
    // while remaining unrelated to which boot owns the contact extreme. The
    // lowest alpha support on each screen half is the calibrated signal here;
    // area is retained as diagnostic evidence only.
    let contact_signal = bottom_delta.clamp(-1.0, 1.0);
    let confidence = (contact_signal.abs() / MINIMUM_CONTACT_SIGNAL).clamp(0.0, 1.0);
    let observed_side = if contact_signal >= MINIMUM_CONTACT_SIGNAL {
        ScreenSideV1::Left
    } else if contact_signal <= -MINIMUM_CONTACT_SIGNAL {
        ScreenSideV1::Right
    } else {
        ScreenSideV1::Ambiguous
    };
    GaitLateralityFrameReportV1 {
        frame_index,
        expected_side,
        observed_side,
        contact_signal,
        confidence,
        screen_left_bottom_y,
        screen_right_bottom_y,
        screen_left_area,
        screen_right_area,
        matches_expected: observed_side == expected_side,
    }
}

fn half_evidence(
    frame: &RgbaImage,
    first_x: u32,
    last_x: u32,
    first_y: u32,
    last_y: u32,
) -> (u32, u32) {
    let mut bottom = first_y;
    let mut area = 0_u32;
    for y in first_y..last_y {
        for x in first_x..last_x {
            if frame.get_pixel(x, y)[3] <= ALPHA_THRESHOLD {
                continue;
            }
            bottom = bottom.max(y + 1);
            area = area.saturating_add(1);
        }
    }
    (bottom, area)
}

fn opposite_delta(frames: &[GaitLateralityFrameReportV1], first: usize, second: usize) -> f32 {
    match (frames.get(first), frames.get(second)) {
        (Some(first), Some(second)) => (first.contact_signal - second.contact_signal).abs(),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn gait_frame(left_bottom: u32, right_bottom: u32) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        for y in 18..60 {
            for x in 30..66 {
                image.put_pixel(x, y, Rgba([90, 120, 70, 255]));
            }
        }
        for y in 60..left_bottom {
            for x in 32..44 {
                image.put_pixel(x, y, Rgba([80, 60, 45, 255]));
            }
        }
        for y in 60..right_bottom {
            for x in 52..64 {
                image.put_pixel(x, y, Rgba([80, 60, 45, 255]));
            }
        }
        image
    }

    #[test]
    fn alternating_screen_space_cycle_passes() {
        let frames = vec![
            gait_frame(91, 80),
            gait_frame(88, 75),
            gait_frame(80, 91),
            gait_frame(75, 88),
        ];
        let report = assess_front_gait_laterality("walk_down", &frames);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.reasons.is_empty());
        assert!(report.recommended_retry_frames.is_empty());
    }

    #[test]
    fn same_screen_side_cycle_blocks_and_targets_opposite_half() {
        let frames = vec![
            gait_frame(91, 80),
            gait_frame(88, 75),
            gait_frame(90, 79),
            gait_frame(87, 74),
        ];
        let report = assess_front_gait_laterality("walk_down", &frames);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .contains(&WALK_LATERALITY_NOT_ALTERNATING.into()));
        assert_eq!(report.recommended_retry_frames, vec![2, 3]);
    }

    #[test]
    fn ambiguous_feet_fail_closed() {
        let frame = gait_frame(86, 86);
        let report = assess_front_gait_laterality(
            "walk_down",
            &[frame.clone(), frame.clone(), frame.clone(), frame],
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.reasons.contains(&WALK_LATERALITY_AMBIGUOUS.into()));
    }

    #[test]
    fn rejected_v91_real_probe_is_a_same_side_regression_fixture() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/\
             7097f67e-0e85-4ead-b5bc-c6c908f4abda/grid-keyframes/walk_down",
        );
        if !root.is_dir() {
            // The checked-out source distribution intentionally excludes paid
            // JobStore pixels; synthetic fixtures above remain mandatory.
            return;
        }
        let frames = (0..4)
            .map(|frame| {
                image::open(root.join(format!("frame-{frame:02}.png")))
                    .unwrap()
                    .to_rgba8()
            })
            .collect::<Vec<_>>();
        let report = assess_front_gait_laterality("walk_down", &frames);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .contains(&WALK_LATERALITY_NOT_ALTERNATING.into()));
        assert_eq!(report.recommended_retry_frames, vec![2, 3]);
        assert!(report.frames.iter().all(|frame| {
            frame.observed_side == ScreenSideV1::Left
                && frame.contact_signal >= MINIMUM_CONTACT_SIGNAL
        }));
    }
}
