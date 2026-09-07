use image::RgbaImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::asset_project::{character_body_bbox, ConsistencyVerdict};
use crate::gait_laterality::{assess_front_gait_laterality, GaitLateralityReportV1};

pub const GAIT_PAIR_TRANSFORM_PROFILE: &str = "gait-pair-transform-spike@1.0.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GaitPairTransformReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub split_y: u32,
    pub upper_body_changed_pixels: u32,
    pub lower_body_changed_ratio: f32,
    pub boundary_seam_changed_ratio: f32,
    pub source_contact_signal: f32,
    pub transformed_contact_signal: f32,
    pub contact_signal_inverted: bool,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Error)]
pub enum GaitPairTransformError {
    #[error("pair transform source has no foreground")]
    MissingForeground,
}

/// Mirrors only the lower part of one front/down sprite around its foreground
/// center. This is an offline feasibility spike, not a production replacement
/// path: garments crossing the split can still make the result unacceptable.
pub fn mirror_front_lower_body(
    source: &RgbaImage,
) -> Result<(RgbaImage, GaitPairTransformReportV1), GaitPairTransformError> {
    let body = character_body_bbox(source).ok_or(GaitPairTransformError::MissingForeground)?;
    let left = body.left.floor().max(0.0) as u32;
    let right = body.right.ceil().min(source.width() as f32) as u32;
    let split_y = (body.top + body.height * 0.62)
        .round()
        .clamp(0.0, source.height() as f32) as u32;
    let bottom = body.bottom.ceil().min(source.height() as f32) as u32;
    let mut output = source.clone();
    for y in split_y..bottom {
        for x in left..right {
            let mirror_x = left + right.saturating_sub(1).saturating_sub(x);
            output.put_pixel(x, y, *source.get_pixel(mirror_x, y));
        }
    }

    let upper_body_changed_pixels = (0..split_y)
        .flat_map(|y| (0..source.width()).map(move |x| (x, y)))
        .filter(|(x, y)| source.get_pixel(*x, *y) != output.get_pixel(*x, *y))
        .count() as u32;
    let lower_area =
        u64::from(right.saturating_sub(left)) * u64::from(bottom.saturating_sub(split_y));
    let lower_changed = (split_y..bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .filter(|(x, y)| source.get_pixel(*x, *y) != output.get_pixel(*x, *y))
        .count() as u64;
    let lower_body_changed_ratio = ratio(lower_changed, lower_area);
    let seam_top = split_y.saturating_sub(2);
    let seam_bottom = (split_y + 2).min(source.height());
    let seam_area =
        u64::from(right.saturating_sub(left)) * u64::from(seam_bottom.saturating_sub(seam_top));
    let seam_changed = (seam_top..seam_bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .filter(|(x, y)| source.get_pixel(*x, *y) != output.get_pixel(*x, *y))
        .count() as u64;
    let boundary_seam_changed_ratio = ratio(seam_changed, seam_area);

    let source_pair = [
        source.clone(),
        source.clone(),
        output.clone(),
        output.clone(),
    ];
    let laterality: GaitLateralityReportV1 =
        assess_front_gait_laterality("walk_down", &source_pair);
    let source_contact_signal = laterality
        .frames
        .first()
        .map(|frame| frame.contact_signal)
        .unwrap_or_default();
    let transformed_contact_signal = laterality
        .frames
        .get(2)
        .map(|frame| frame.contact_signal)
        .unwrap_or_default();
    let contact_signal_inverted = source_contact_signal * transformed_contact_signal < 0.0
        && (source_contact_signal - transformed_contact_signal).abs() >= 0.04;

    let mut reasons = Vec::new();
    if upper_body_changed_pixels != 0 {
        reasons.push("pair_transform_upper_body_changed".into());
    }
    if !contact_signal_inverted {
        reasons.push("pair_transform_contact_not_inverted".into());
    }
    // A hard horizontal cut with large change directly across its four-row
    // band is a visible garment/leg seam and remains unsuitable for automatic
    // production use.
    if boundary_seam_changed_ratio > 0.20 {
        reasons.push("pair_transform_boundary_seam_risk".into());
    }
    reasons.sort();
    let verdict = if reasons.is_empty() {
        ConsistencyVerdict::GameReady
    } else {
        ConsistencyVerdict::Blocked
    };
    Ok((
        output,
        GaitPairTransformReportV1 {
            schema_version: "1".into(),
            profile: GAIT_PAIR_TRANSFORM_PROFILE.into(),
            split_y,
            upper_body_changed_pixels,
            lower_body_changed_ratio,
            boundary_seam_changed_ratio,
            source_contact_signal,
            transformed_contact_signal,
            contact_signal_inverted,
            verdict,
            reasons,
        },
    ))
}

fn ratio(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn asymmetric_contact() -> RgbaImage {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        for y in 18..62 {
            for x in 30..66 {
                image.put_pixel(x, y, Rgba([90, 120, 70, 255]));
            }
        }
        for y in 62..91 {
            for x in 32..42 {
                image.put_pixel(x, y, Rgba([80, 60, 45, 255]));
            }
        }
        for y in 62..80 {
            for x in 54..64 {
                image.put_pixel(x, y, Rgba([80, 60, 45, 255]));
            }
        }
        image
    }

    #[test]
    fn lower_mirror_preserves_upper_bytes_and_inverts_contact() {
        let source = asymmetric_contact();
        let (output, report) = mirror_front_lower_body(&source).unwrap();
        assert_eq!(report.upper_body_changed_pixels, 0);
        assert!(report.contact_signal_inverted);
        for y in 0..report.split_y {
            for x in 0..source.width() {
                assert_eq!(source.get_pixel(x, y), output.get_pixel(x, y));
            }
        }
    }

    #[test]
    fn real_probe_transform_remains_non_production_if_seam_is_risky() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/\
             7097f67e-0e85-4ead-b5bc-c6c908f4abda/grid-keyframes/walk_down/frame-00.png",
        );
        if !path.is_file() {
            return;
        }
        let source = image::open(path).unwrap().to_rgba8();
        let (_, report) = mirror_front_lower_body(&source).unwrap();
        assert_eq!(report.upper_body_changed_pixels, 0);
        assert!(report.contact_signal_inverted);
        assert!(report.boundary_seam_changed_ratio > 0.20);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .contains(&"pair_transform_boundary_seam_risk".into()));
    }
}
