use std::collections::VecDeque;

use image::RgbaImage;
use serde::{Deserialize, Serialize};

pub const ASSET_GEOMETRY_REPORT_PROFILE: &str = "asset-geometry@1.0.0";
pub const DIALOGUE_BUST_PROFILE: &str = "dialogue_bust@1.0.0";
pub const FULL_BODY_PROFILE: &str = "full_body@1.0.0";
pub const CHARACTER_SPRITE_PROFILE: &str = "character_sprite@1.0.0";

const ALPHA_THRESHOLD: u8 = 16;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortraitFramingProfileV1 {
    #[default]
    #[serde(rename = "dialogue_bust@1.0.0")]
    DialogueBust,
    #[serde(rename = "full_body@1.0.0")]
    FullBody,
}

impl PortraitFramingProfileV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DialogueBust => DIALOGUE_BUST_PROFILE,
            Self::FullBody => FULL_BODY_PROFILE,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetGeometryProfileV1 {
    #[serde(rename = "dialogue_bust@1.0.0")]
    DialogueBust,
    #[serde(rename = "full_body@1.0.0")]
    FullBody,
    #[serde(rename = "character_sprite@1.0.0")]
    CharacterSprite,
}

impl AssetGeometryProfileV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DialogueBust => DIALOGUE_BUST_PROFILE,
            Self::FullBody => FULL_BODY_PROFILE,
            Self::CharacterSprite => CHARACTER_SPRITE_PROFILE,
        }
    }
}

impl From<PortraitFramingProfileV1> for AssetGeometryProfileV1 {
    fn from(value: PortraitFramingProfileV1) -> Self {
        match value {
            PortraitFramingProfileV1::DialogueBust => Self::DialogueBust,
            PortraitFramingProfileV1::FullBody => Self::FullBody,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetGeometryVerdictV1 {
    GameReady,
    AwaitingReview,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetGeometryMetricsV1 {
    pub foreground_width_ratio: f32,
    pub foreground_height_ratio: f32,
    pub foreground_area_ratio: f32,
    pub top_margin_ratio: f32,
    pub bottom_margin_ratio: f32,
    pub subject_count: u32,
    /// A deterministic silhouette proxy, not an anatomical leg/foot detector.
    pub lower_body_presence_proxy: f32,
    pub lower_body_two_run_ratio: f32,
    pub lower_body_width_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetGeometryReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub geometry_profile: AssetGeometryProfileV1,
    pub verdict: AssetGeometryVerdictV1,
    pub metrics: AssetGeometryMetricsV1,
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// Assesses transparent/matted pixels without claiming semantic body-part detection.
///
/// `lowerBodyPresenceProxy` only describes lower-silhouette topology. It can reject
/// an obvious bust used as a full-body asset, while robes and occluded top-down
/// poses remain reviewable until a licensed semantic vision component is present.
pub fn assess_asset_geometry(
    image: &RgbaImage,
    profile: AssetGeometryProfileV1,
) -> AssetGeometryReportV1 {
    let bounds = alpha_bounds(image);
    let subject_count = major_subject_count(image);
    let mut reasons = Vec::new();
    let Some((left, top, right, bottom)) = bounds else {
        reasons.push("geometry_foreground_missing".into());
        return report(
            profile,
            AssetGeometryVerdictV1::Blocked,
            AssetGeometryMetricsV1 {
                foreground_width_ratio: 0.0,
                foreground_height_ratio: 0.0,
                foreground_area_ratio: 0.0,
                top_margin_ratio: 0.0,
                bottom_margin_ratio: 0.0,
                subject_count: 0,
                lower_body_presence_proxy: 0.0,
                lower_body_two_run_ratio: 0.0,
                lower_body_width_ratio: 0.0,
            },
            reasons,
        );
    };

    let canvas_width = image.width().max(1);
    let canvas_height = image.height().max(1);
    let bbox_width = right - left + 1;
    let bbox_height = bottom - top + 1;
    let foreground = image
        .pixels()
        .filter(|pixel| pixel[3] > ALPHA_THRESHOLD)
        .count();
    let (two_run_ratio, lower_width_ratio) = lower_body_topology(image, (left, top, right, bottom));
    let lower_body_presence_proxy =
        (0.85 * two_run_ratio + 0.15 * (1.0 - lower_width_ratio).clamp(0.0, 1.0)).clamp(0.0, 1.0);
    let metrics = AssetGeometryMetricsV1 {
        foreground_width_ratio: bbox_width as f32 / canvas_width as f32,
        foreground_height_ratio: bbox_height as f32 / canvas_height as f32,
        foreground_area_ratio: foreground as f32 / (canvas_width * canvas_height) as f32,
        top_margin_ratio: top as f32 / canvas_height as f32,
        bottom_margin_ratio: canvas_height.saturating_sub(bottom + 1) as f32 / canvas_height as f32,
        subject_count,
        lower_body_presence_proxy,
        lower_body_two_run_ratio: two_run_ratio,
        lower_body_width_ratio: lower_width_ratio,
    };

    if subject_count == 0 {
        reasons.push("geometry_foreground_missing".into());
    } else if subject_count > 1 {
        reasons.push("geometry_multiple_subjects".into());
    }
    if left == 0 || top == 0 || right + 1 >= image.width() || bottom + 1 >= image.height() {
        reasons.push("geometry_foreground_clipped".into());
    }
    if !reasons.is_empty() {
        return report(profile, AssetGeometryVerdictV1::Blocked, metrics, reasons);
    }

    match profile {
        AssetGeometryProfileV1::DialogueBust => {
            // Dialogue portraits intentionally allow compact bust crops. This
            // profile protects the established Stage 3 behavior and does not
            // infer or require off-canvas lower-body anatomy.
            if metrics.foreground_width_ratio < 0.20
                || metrics.foreground_height_ratio < 0.35
                || metrics.foreground_area_ratio < 0.08
            {
                reasons.push("dialogue_bust_extent_invalid".into());
                report(profile, AssetGeometryVerdictV1::Blocked, metrics, reasons)
            } else {
                report(profile, AssetGeometryVerdictV1::GameReady, metrics, reasons)
            }
        }
        AssetGeometryProfileV1::FullBody | AssetGeometryProfileV1::CharacterSprite => {
            if metrics.foreground_height_ratio < 0.55
                || metrics.lower_body_width_ratio > 0.82
                || metrics.lower_body_presence_proxy < 0.04
            {
                reasons.push("full_body_lower_body_missing".into());
                report(profile, AssetGeometryVerdictV1::Blocked, metrics, reasons)
            } else if metrics.lower_body_two_run_ratio < 0.30 {
                reasons.push("full_body_lower_body_ambiguous".into());
                report(
                    profile,
                    AssetGeometryVerdictV1::AwaitingReview,
                    metrics,
                    reasons,
                )
            } else {
                report(profile, AssetGeometryVerdictV1::GameReady, metrics, reasons)
            }
        }
    }
}

pub fn assess_portrait_geometry(
    image: &RgbaImage,
    profile: PortraitFramingProfileV1,
) -> AssetGeometryReportV1 {
    assess_asset_geometry(image, profile.into())
}

fn report(
    geometry_profile: AssetGeometryProfileV1,
    verdict: AssetGeometryVerdictV1,
    metrics: AssetGeometryMetricsV1,
    reasons: Vec<String>,
) -> AssetGeometryReportV1 {
    AssetGeometryReportV1 {
        schema_version: "1".into(),
        profile: ASSET_GEOMETRY_REPORT_PROFILE.into(),
        geometry_profile,
        verdict,
        metrics,
        reasons,
    }
}

fn alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] <= ALPHA_THRESHOLD {
            continue;
        }
        found = true;
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    found.then_some((left, top, right, bottom))
}

fn lower_body_topology(
    image: &RgbaImage,
    (left, top, right, bottom): (u32, u32, u32, u32),
) -> (f32, f32) {
    let bbox_height = bottom - top + 1;
    let bbox_width = right - left + 1;
    let lower_start = top + bbox_height * 3 / 5;
    let middle_start = top + bbox_height / 4;
    let middle_end = top + bbox_height * 3 / 5;
    let terminal_start = top + bbox_height * 17 / 20;
    let minimum_run = (bbox_width / 32).max(2);
    let mut lower_rows = 0usize;
    let mut two_run_rows = 0usize;
    let mut terminal_occupied = 0usize;
    let mut terminal_rows = 0usize;
    let mut middle_max = 0usize;

    for y in top..=bottom {
        let (runs, occupied) = row_runs(image, y, left, right, minimum_run);
        if (middle_start..middle_end).contains(&y) {
            middle_max = middle_max.max(occupied);
        }
        if y >= lower_start {
            lower_rows += 1;
            if runs >= 2 {
                two_run_rows += 1;
            }
        }
        if y >= terminal_start {
            terminal_rows += 1;
            terminal_occupied += occupied;
        }
    }

    let two_run_ratio = two_run_rows as f32 / lower_rows.max(1) as f32;
    let terminal_mean = terminal_occupied as f32 / terminal_rows.max(1) as f32;
    let lower_width_ratio = terminal_mean / middle_max.max(1) as f32;
    (
        two_run_ratio.clamp(0.0, 1.0),
        lower_width_ratio.clamp(0.0, 2.0),
    )
}

fn row_runs(image: &RgbaImage, y: u32, left: u32, right: u32, minimum_run: u32) -> (u32, usize) {
    let mut runs = 0u32;
    let mut occupied = 0usize;
    let mut current = 0u32;
    for x in left..=right {
        if image.get_pixel(x, y)[3] > ALPHA_THRESHOLD {
            current += 1;
            occupied += 1;
        } else {
            if current >= minimum_run {
                runs += 1;
            }
            current = 0;
        }
    }
    if current >= minimum_run {
        runs += 1;
    }
    (runs, occupied)
}

fn major_subject_count(image: &RgbaImage) -> u32 {
    let width = image.width() as usize;
    let height = image.height() as usize;
    if width == 0 || height == 0 {
        return 0;
    }
    let foreground = image
        .pixels()
        .map(|pixel| pixel[3] > ALPHA_THRESHOLD)
        .collect::<Vec<_>>();
    let foreground_total = foreground.iter().filter(|value| **value).count();
    if foreground_total == 0 {
        return 0;
    }
    let minimum_area = (foreground_total / 20).max(4);
    let mut visited = vec![false; foreground.len()];
    let mut major = 0u32;
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
            for (next_x, next_y) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if next_x >= width || next_y >= height {
                    continue;
                }
                let next = next_y * width + next_x;
                if foreground[next] && !visited[next] {
                    visited[next] = true;
                    queue.push_back(next);
                }
            }
        }
        if area >= minimum_area {
            major += 1;
        }
    }
    major
}

#[cfg(test)]
mod tests {
    use image::{ImageBuffer, Rgba};

    use super::*;

    fn transparent_canvas() -> RgbaImage {
        ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]))
    }

    fn fill(image: &mut RgbaImage, x: std::ops::Range<u32>, y: std::ops::Range<u32>) {
        for row in y {
            for column in x.clone() {
                image.put_pixel(column, row, Rgba([120, 80, 40, 255]));
            }
        }
    }

    fn bust() -> RgbaImage {
        let mut image = transparent_canvas();
        fill(&mut image, 46..82, 12..42);
        fill(&mut image, 26..102, 42..108);
        image
    }

    fn full_body() -> RgbaImage {
        let mut image = transparent_canvas();
        fill(&mut image, 48..80, 8..32);
        fill(&mut image, 40..88, 32..78);
        fill(&mut image, 42..58, 78..116);
        fill(&mut image, 70..86, 78..116);
        image
    }

    fn robe() -> RgbaImage {
        let mut image = transparent_canvas();
        fill(&mut image, 48..80, 8..32);
        fill(&mut image, 40..88, 32..78);
        for y in 78..116 {
            let inset = (y - 78) / 5;
            fill(&mut image, (42 + inset)..(86 - inset), y..(y + 1));
        }
        image
    }

    #[test]
    fn dialogue_bust_does_not_claim_or_require_legs() {
        let report = assess_portrait_geometry(&bust(), PortraitFramingProfileV1::DialogueBust);
        assert_eq!(report.verdict, AssetGeometryVerdictV1::GameReady);
        assert!(report.metrics.lower_body_presence_proxy < 0.10);
        let json = serde_json::to_value(report).unwrap();
        assert!(json["metrics"].get("lowerBodyPresenceProxy").is_some());
        assert!(json.to_string().find("legsVerified").is_none());
    }

    #[test]
    fn obvious_bust_is_blocked_by_full_body_profile() {
        let report = assess_portrait_geometry(&bust(), PortraitFramingProfileV1::FullBody);
        assert_eq!(report.verdict, AssetGeometryVerdictV1::Blocked);
        assert!(report
            .reasons
            .contains(&"full_body_lower_body_missing".into()));
    }

    #[test]
    fn valid_synthetic_full_body_passes_the_proxy() {
        let report = assess_portrait_geometry(&full_body(), PortraitFramingProfileV1::FullBody);
        assert_eq!(report.verdict, AssetGeometryVerdictV1::GameReady);
        assert!(report.metrics.lower_body_two_run_ratio >= 0.30);
    }

    #[test]
    fn ambiguous_robe_requires_review_instead_of_claiming_leg_semantics() {
        let report = assess_portrait_geometry(&robe(), PortraitFramingProfileV1::FullBody);
        assert_eq!(report.verdict, AssetGeometryVerdictV1::AwaitingReview);
        assert!(report
            .reasons
            .contains(&"full_body_lower_body_ambiguous".into()));
    }
}
