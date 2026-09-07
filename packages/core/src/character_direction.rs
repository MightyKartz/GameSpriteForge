use image::RgbaImage;
use serde::{Deserialize, Serialize};

use crate::asset_project::{
    assess_character_identity_reference, character_body_bbox, CharacterEquipmentKindV1,
    ConsistencyVerdict,
};
use crate::character_camera::CharacterCameraProfileV1;

pub const DIRECTION_LOCK_PROFILE: &str = "direction-lock@1.0.0";
pub const DIRECTION_LOCK_GENERATION_MASTER_PROFILE: &str = "direction-lock@1.1.0";
pub const DIRECTION_ANCHOR_PROFILE: &str = "direction-anchor@1.2.0";
pub const DIRECTION_GUIDE_PROFILE: &str = "topdown-direction-guides@1.0.0";
pub const DIRECTION_STILL_PREFLIGHT_PROFILE: &str = "direction-still-preflight@1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectionViewV1 {
    Front,
    Rear,
    Right,
    Left,
}

impl DirectionViewV1 {
    pub fn for_animation(animation: &str) -> Option<Self> {
        match animation {
            "idle" | "idle_down" | "walk_down" => Some(Self::Front),
            "idle_up" | "walk_up" => Some(Self::Rear),
            "idle_right" | "walk_right" => Some(Self::Right),
            "idle_left" | "walk_left" => Some(Self::Left),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionAnchorMetricsV1 {
    pub body_present: bool,
    pub visible_face_proxy: bool,
    pub enclosed_feature_count: u32,
    pub warm_upper_pixel_ratio: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warm_upper_centroid_offset_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warm_upper_centroid_y_ratio: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warm_upper_vertical_span_ratio: Option<f32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionAnchorAssessmentV1 {
    pub schema_version: String,
    pub profile: String,
    pub direction: DirectionViewV1,
    pub metrics: DirectionAnchorMetricsV1,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionStillFramingMetricsV1 {
    pub body_top_margin_ratio: f32,
    pub body_bottom_margin_ratio: f32,
    pub body_height_ratio: f32,
    pub body_center_offset_ratio: f32,
    pub reference_scale_ratio: f32,
    pub reference_top_drift_ratio: f32,
    pub reference_foot_drift_ratio: f32,
    pub foreground_touches_boundary: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionStillPreflightReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub camera_profile: CharacterCameraProfileV1,
    pub animation: String,
    pub direction: DirectionViewV1,
    pub direction_anchor: DirectionAnchorAssessmentV1,
    pub framing: DirectionStillFramingMetricsV1,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionLockEntryV1 {
    pub direction: DirectionViewV1,
    pub animation: String,
    pub frame: u8,
    pub attempt: u8,
    pub path: std::path::PathBuf,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_master_path: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_master_sha256: Option<String>,
    pub assessment: DirectionAnchorAssessmentV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirectionLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub workflow: String,
    pub provider_id: String,
    pub profile_id: String,
    pub model: String,
    pub style_revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub subject_sha256: String,
    pub equipment_kind: CharacterEquipmentKindV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment_reference_sha256: Option<String>,
    pub entries: Vec<DirectionLockEntryV1>,
}

pub fn assess_direction_anchor(
    image: &RgbaImage,
    character_prompt: &str,
    direction: DirectionViewV1,
) -> DirectionAnchorAssessmentV1 {
    let identity = assess_character_identity_reference(image, character_prompt);
    let body = character_body_bbox(image);
    let (warm_ratio, warm_offset, warm_centroid_y, warm_vertical_span) = body
        .as_ref()
        .map(|bbox| warm_upper_metrics(image, bbox.left, bbox.top, bbox.width, bbox.height))
        .unwrap_or((0.0, None, None, None));
    // A cleaned sprite keeps facial line art opaque. Requiring transparent
    // holes as the only face proxy therefore rejects valid front views after
    // background removal. Keep the identity detector as the strongest signal,
    // but also accept a substantial, centered warm face region in the upper
    // body. The upper crop deliberately ends above scarves and belt details.
    let visible_face_proxy = warm_ratio >= 0.08
        && warm_offset.is_some_and(|offset| offset.abs() <= 0.08)
        && warm_centroid_y.is_some_and(|centroid| centroid <= 0.33)
        && warm_vertical_span.is_some_and(|span| span >= 0.12);
    let mut reasons = Vec::new();
    if body.is_none() {
        reasons.push("direction_anchor_body_missing".into());
    } else {
        match direction {
            DirectionViewV1::Front => {
                if !visible_face_proxy {
                    reasons.push("direction_anchor_front_face_missing".into());
                }
                if warm_offset.is_some_and(|offset| offset.abs() > 0.14) {
                    reasons.push("direction_anchor_front_not_centered".into());
                }
            }
            DirectionViewV1::Rear => {
                if visible_face_proxy {
                    reasons.push("direction_anchor_rear_face_visible".into());
                }
            }
            DirectionViewV1::Right => {
                if warm_ratio < 0.015 {
                    reasons.push("direction_anchor_right_face_missing".into());
                }
                if warm_offset.is_none_or(|offset| offset < 0.055) {
                    reasons.push("direction_anchor_right_profile_missing".into());
                }
            }
            DirectionViewV1::Left => {
                if warm_ratio < 0.015 {
                    reasons.push("direction_anchor_left_face_missing".into());
                }
                if warm_offset.is_none_or(|offset| offset > -0.055) {
                    reasons.push("direction_anchor_left_profile_missing".into());
                }
            }
        }
    }
    DirectionAnchorAssessmentV1 {
        schema_version: "1".into(),
        profile: DIRECTION_ANCHOR_PROFILE.into(),
        direction,
        metrics: DirectionAnchorMetricsV1 {
            body_present: body.is_some(),
            visible_face_proxy,
            enclosed_feature_count: identity.metrics.enclosed_feature_count,
            warm_upper_pixel_ratio: warm_ratio,
            warm_upper_centroid_offset_ratio: warm_offset,
            warm_upper_centroid_y_ratio: warm_centroid_y,
            warm_upper_vertical_span_ratio: warm_vertical_span,
        },
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        reasons,
    }
}

/// Hard gate applied before any paid image-to-video request.
///
/// It combines the direction detector with absolute safe framing and framing
/// drift relative to the locked identity reference. A rejected still is never
/// allowed to spend a video request.
pub fn assess_direction_still_preflight(
    image: &RgbaImage,
    identity_reference: &RgbaImage,
    character_prompt: &str,
    animation: &str,
    camera_profile: CharacterCameraProfileV1,
) -> DirectionStillPreflightReportV1 {
    let direction = DirectionViewV1::for_animation(animation).unwrap_or(DirectionViewV1::Front);
    let direction_anchor = assess_direction_anchor(image, character_prompt, direction);
    let body = character_body_bbox(image);
    let reference_body = character_body_bbox(identity_reference);
    let width = image.width().max(1) as f32;
    let height = image.height().max(1) as f32;
    let (top, bottom, body_height, center_offset) = body
        .map(|bbox| {
            (
                bbox.top / height,
                (height - bbox.bottom_y).max(0.0) / height,
                bbox.height / height,
                (bbox.center_x - width / 2.0).abs() / width,
            )
        })
        .unwrap_or((1.0, 1.0, 0.0, 1.0));
    let (reference_scale_ratio, reference_top_drift_ratio, reference_foot_drift_ratio) = body
        .zip(reference_body)
        .map(|(current, reference)| {
            let current_height_ratio = current.height / height;
            let reference_height_ratio =
                reference.height / identity_reference.height().max(1) as f32;
            (
                current_height_ratio / reference_height_ratio.max(f32::EPSILON),
                (current.top / height - reference.top / identity_reference.height().max(1) as f32)
                    .abs(),
                (current.bottom_y / height
                    - reference.bottom_y / identity_reference.height().max(1) as f32)
                    .abs(),
            )
        })
        .unwrap_or((0.0, 1.0, 1.0));
    let foreground_touches_boundary = foreground_touches_boundary(image, 32, 1);
    let mut reasons = direction_anchor.reasons.clone();
    if body.is_none() || reference_body.is_none() {
        reasons.push("direction_still_body_missing".into());
    } else {
        if top > 0.30 {
            reasons.push("direction_still_excessive_top_margin".into());
        }
        if bottom > 0.16 {
            reasons.push("direction_still_foot_baseline_too_high".into());
        }
        if !(0.50..=0.90).contains(&body_height) {
            reasons.push("direction_still_body_scale_out_of_range".into());
        }
        if center_offset > 0.08 {
            reasons.push("direction_still_not_centered".into());
        }
        if !(0.85..=1.15).contains(&reference_scale_ratio) {
            reasons.push("direction_still_reference_scale_drift".into());
        }
        if reference_top_drift_ratio > 0.10 {
            reasons.push("direction_still_reference_top_drift".into());
        }
        if reference_foot_drift_ratio > 0.08 {
            reasons.push("direction_still_reference_foot_drift".into());
        }
    }
    if foreground_touches_boundary {
        reasons.push("direction_still_cropped".into());
    }
    reasons.sort();
    reasons.dedup();
    DirectionStillPreflightReportV1 {
        schema_version: "1".into(),
        profile: DIRECTION_STILL_PREFLIGHT_PROFILE.into(),
        camera_profile,
        animation: animation.into(),
        direction,
        direction_anchor,
        framing: DirectionStillFramingMetricsV1 {
            body_top_margin_ratio: top,
            body_bottom_margin_ratio: bottom,
            body_height_ratio: body_height,
            body_center_offset_ratio: center_offset,
            reference_scale_ratio,
            reference_top_drift_ratio,
            reference_foot_drift_ratio,
            foreground_touches_boundary,
        },
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        reasons,
    }
}

fn foreground_touches_boundary(image: &RgbaImage, alpha_threshold: u8, inset: u32) -> bool {
    if image.width() <= inset * 2 || image.height() <= inset * 2 {
        return true;
    }
    let right = image.width() - 1 - inset;
    let bottom = image.height() - 1 - inset;
    (inset..=right).any(|x| {
        image.get_pixel(x, inset)[3] > alpha_threshold
            || image.get_pixel(x, bottom)[3] > alpha_threshold
    }) || (inset..=bottom).any(|y| {
        image.get_pixel(inset, y)[3] > alpha_threshold
            || image.get_pixel(right, y)[3] > alpha_threshold
    })
}

fn warm_upper_metrics(
    image: &RgbaImage,
    body_left: f32,
    body_top: f32,
    body_width: f32,
    body_height: f32,
) -> (f32, Option<f32>, Option<f32>, Option<f32>) {
    let left = body_left.max(0.0) as u32;
    let right = (body_left + body_width).ceil().min(image.width() as f32) as u32;
    let top = (body_top + body_height * 0.08).max(0.0) as u32;
    let bottom = (body_top + body_height * 0.40)
        .ceil()
        .min(image.height() as f32) as u32;
    let mut pixels = 0usize;
    let mut x_sum = 0f32;
    let mut y_sum = 0f32;
    let mut minimum_y = None::<u32>;
    let mut maximum_y = None::<u32>;
    for y in top..bottom {
        for x in left..right {
            let pixel = image.get_pixel(x, y);
            if pixel[3] > 32
                && pixel[0] >= 75
                && pixel[0] >= pixel[1].saturating_add(12)
                && pixel[1] >= pixel[2].saturating_add(4)
            {
                pixels += 1;
                x_sum += x as f32 + 0.5;
                y_sum += y as f32 + 0.5;
                minimum_y = Some(minimum_y.map_or(y, |current| current.min(y)));
                maximum_y = Some(maximum_y.map_or(y, |current| current.max(y)));
            }
        }
    }
    let area = (right.saturating_sub(left) * bottom.saturating_sub(top)).max(1) as f32;
    let ratio = pixels as f32 / area;
    let x_offset = (pixels > 0).then(|| {
        let centroid = x_sum / pixels as f32;
        let body_center = body_left + body_width / 2.0;
        (centroid - body_center) / body_width.max(1.0)
    });
    let y_centroid = (pixels > 0).then(|| {
        let centroid = y_sum / pixels as f32;
        (centroid - body_top) / body_height.max(1.0)
    });
    let vertical_span = minimum_y.zip(maximum_y).map(|(minimum, maximum)| {
        (maximum.saturating_sub(minimum) + 1) as f32 / body_height.max(1.0)
    });
    (ratio, x_offset, y_centroid, vertical_span)
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn body(direction: DirectionViewV1) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        for y in 28..84 {
            for x in 34..62 {
                image.put_pixel(x, y, Rgba([130, 60, 210, 255]));
            }
        }
        match direction {
            DirectionViewV1::Rear => {
                for y in 34..51 {
                    for x in 39..57 {
                        image.put_pixel(x, y, Rgba([40, 110, 80, 255]));
                    }
                }
            }
            DirectionViewV1::Front => draw_face(&mut image, 39, 57, true),
            DirectionViewV1::Right => draw_face(&mut image, 48, 62, false),
            DirectionViewV1::Left => draw_face(&mut image, 34, 48, false),
        }
        image
    }

    fn draw_face(image: &mut RgbaImage, left: u32, right: u32, two_eyes: bool) {
        for y in 34..51 {
            for x in left..right {
                image.put_pixel(x, y, Rgba([190, 126, 88, 255]));
            }
        }
        image.put_pixel(left + 4, 39, Rgba([35, 22, 18, 255]));
        if two_eyes {
            image.put_pixel(right - 5, 39, Rgba([35, 22, 18, 255]));
        }
        for x in (left + 6)..right.saturating_sub(6) {
            image.put_pixel(x, 48, Rgba([70, 36, 28, 255]));
        }
    }

    #[test]
    fn distinguishes_four_cardinal_fixture_views() {
        for direction in [
            DirectionViewV1::Front,
            DirectionViewV1::Rear,
            DirectionViewV1::Right,
            DirectionViewV1::Left,
        ] {
            let report = assess_direction_anchor(&body(direction), "human ranger", direction);
            assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:?}");
        }
        assert_eq!(
            assess_direction_anchor(
                &body(DirectionViewV1::Front),
                "human ranger",
                DirectionViewV1::Rear,
            )
            .verdict,
            ConsistencyVerdict::Blocked
        );
    }

    #[test]
    fn rear_scarf_below_face_band_is_not_mistaken_for_a_front_face() {
        let mut rear = body(DirectionViewV1::Rear);
        for y in 52..58 {
            for x in 35..61 {
                rear.put_pixel(x, y, Rgba([210, 110, 35, 255]));
            }
        }
        let report = assess_direction_anchor(&rear, "human ranger", DirectionViewV1::Rear);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:?}");
        assert!(!report.metrics.visible_face_proxy);
    }

    #[test]
    fn direction_still_preflight_accepts_locked_cardinal_views() {
        let reference = body(DirectionViewV1::Front);
        for (animation, direction) in [
            ("idle", DirectionViewV1::Front),
            ("walk_down", DirectionViewV1::Front),
            ("walk_up", DirectionViewV1::Rear),
            ("walk_right", DirectionViewV1::Right),
        ] {
            let report = assess_direction_still_preflight(
                &body(direction),
                &reference,
                "human ranger",
                animation,
                CharacterCameraProfileV1::TopdownOrthographic,
            );
            assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:?}");
            assert_eq!(
                report.camera_profile,
                CharacterCameraProfileV1::TopdownOrthographic
            );
        }
    }

    #[test]
    fn direction_still_preflight_blocks_excessive_empty_top_space() {
        let reference = body(DirectionViewV1::Front);
        let mut shifted = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        for y in 28..84 {
            for x in 0..96 {
                shifted.put_pixel(x, y + 12, *reference.get_pixel(x, y));
            }
        }
        let report = assess_direction_still_preflight(
            &shifted,
            &reference,
            "human ranger",
            "idle",
            CharacterCameraProfileV1::TopdownOrthographic,
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .contains(&"direction_still_excessive_top_margin".into()));
    }
}
