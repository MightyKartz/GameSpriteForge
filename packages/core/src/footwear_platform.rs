use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::asset_project::ConsistencyVerdict;

pub const FOOTWEAR_PLATFORM_PROFILE: &str = "footwear-platform@1.1.0";
pub const FOOTWEAR_PLATFORM_REASON: &str = "footwear_platform_or_skate_detected";
pub const FOOTWEAR_GUIDE_LEAK_REASON: &str = "footwear_neutral_gray_guide_leak_detected";
pub const FOOTWEAR_NEUTRAL_GRAY_REPAIR_PROFILE: &str = "footwear-neutral-gray-repair@1.0.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FootwearNeutralGrayRepairReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub verdict: ConsistencyVerdict,
    pub detected_pixels: u32,
    pub modified_pixels: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[u32; 4]>,
    pub non_mask_pixels_unchanged: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScreenSideV1 {
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FootwearPlatformSideMetricsV1 {
    pub side: ScreenSideV1,
    pub local_bottom_y: u32,
    pub shelf_width_px: u32,
    pub stem_width_px: u32,
    pub shelf_thickness_px: u32,
    pub shelf_width_to_body_ratio: f32,
    pub shelf_to_stem_ratio: f32,
    pub platform_like: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FootwearPlatformReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub frame_index: u8,
    pub verdict: ConsistencyVerdict,
    pub body_width_px: u32,
    pub body_height_px: u32,
    pub sides: Vec<FootwearPlatformSideMetricsV1>,
    #[serde(default)]
    pub neutral_gray_leak_pixels: u32,
    #[serde(default)]
    pub neutral_gray_leak_like: bool,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FootwearPlatformActionReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub verdict: ConsistencyVerdict,
    pub frames: Vec<FootwearPlatformReportV1>,
    #[serde(default)]
    pub reasons: Vec<String>,
}

pub fn assess_footwear_platform_action(
    animation: &str,
    frames: &[RgbaImage],
) -> FootwearPlatformActionReportV1 {
    let reports = frames
        .iter()
        .enumerate()
        .map(|(index, image)| assess_footwear_platform(animation, index as u8, image))
        .collect::<Vec<_>>();
    let blocked = reports
        .iter()
        .any(|report| report.verdict != ConsistencyVerdict::GameReady);
    let reasons = reports
        .iter()
        .flat_map(|report| report.reasons.iter().cloned())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    FootwearPlatformActionReportV1 {
        schema_version: "1".into(),
        profile: FOOTWEAR_PLATFORM_PROFILE.into(),
        animation: animation.into(),
        verdict: if blocked {
            ConsistencyVerdict::Blocked
        } else {
            ConsistencyVerdict::GameReady
        },
        frames: reports,
        reasons,
    }
}

/// Detects a thin, abnormally wide foreground shelf attached beneath either
/// boot. The gate is deliberately silhouette-only: it catches a recolored or
/// textured PoseStructure sole without treating ordinary gray clothing as a
/// guide leak.
pub fn assess_footwear_platform(
    animation: &str,
    frame_index: u8,
    image: &RgbaImage,
) -> FootwearPlatformReportV1 {
    let Some((left, top, right, bottom)) = alpha_bounds(image) else {
        return FootwearPlatformReportV1 {
            schema_version: "1".into(),
            profile: FOOTWEAR_PLATFORM_PROFILE.into(),
            animation: animation.into(),
            frame_index,
            verdict: ConsistencyVerdict::Blocked,
            body_width_px: 0,
            body_height_px: 0,
            sides: Vec::new(),
            neutral_gray_leak_pixels: 0,
            neutral_gray_leak_like: false,
            reasons: vec![FOOTWEAR_PLATFORM_REASON.into()],
        };
    };
    let body_width = right - left + 1;
    let body_height = bottom - top + 1;
    let center = left + body_width / 2;
    let ranges = [
        (ScreenSideV1::Left, left, center.saturating_sub(1)),
        (ScreenSideV1::Right, center, right),
    ];
    let sides = ranges
        .into_iter()
        .map(|(side, side_left, side_right)| {
            assess_side(
                image,
                side,
                side_left,
                side_right,
                top,
                bottom,
                body_width,
                body_height,
            )
        })
        .collect::<Vec<_>>();
    let neutral_gray_leak_pixels =
        largest_lower_neutral_gray_component(image, left, top, right, bottom);
    let neutral_gray_leak_like = neutral_gray_leak_pixels >= 16;
    let platform_like = sides.iter().any(|side| side.platform_like);
    let blocked = platform_like || neutral_gray_leak_like;
    let mut reasons = Vec::new();
    if platform_like {
        reasons.push(FOOTWEAR_PLATFORM_REASON.into());
    }
    if neutral_gray_leak_like {
        reasons.push(FOOTWEAR_GUIDE_LEAK_REASON.into());
    }
    FootwearPlatformReportV1 {
        schema_version: "1".into(),
        profile: FOOTWEAR_PLATFORM_PROFILE.into(),
        animation: animation.into(),
        frame_index,
        verdict: if blocked {
            ConsistencyVerdict::Blocked
        } else {
            ConsistencyVerdict::GameReady
        },
        body_width_px: body_width,
        body_height_px: body_height,
        sides,
        neutral_gray_leak_pixels,
        neutral_gray_leak_like,
        reasons,
    }
}

fn largest_lower_neutral_gray_component(
    image: &RgbaImage,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) -> u32 {
    largest_lower_neutral_gray_component_pixels(image, left, top, right, bottom).len() as u32
}

fn largest_lower_neutral_gray_component_pixels(
    image: &RgbaImage,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) -> Vec<(u32, u32)> {
    let body_height = bottom - top + 1;
    let lower_top = top + body_height.saturating_mul(3) / 4;
    let mut candidates = std::collections::BTreeSet::new();
    for y in lower_top..=bottom {
        for x in left..=right {
            let [red, green, blue, alpha] = image.get_pixel(x, y).0;
            let maximum = red.max(green).max(blue);
            let minimum = red.min(green).min(blue);
            let brightness = (u32::from(red) + u32::from(green) + u32::from(blue)) / 3;
            if alpha >= 160 && brightness >= 110 && maximum.saturating_sub(minimum) <= 35 {
                candidates.insert((x, y));
            }
        }
    }
    let mut maximum = Vec::new();
    while let Some(start) = candidates.pop_first() {
        let mut queue = std::collections::VecDeque::from([start]);
        let mut component = Vec::new();
        while let Some((x, y)) = queue.pop_front() {
            component.push((x, y));
            for next_y in y.saturating_sub(1)..=(y + 1).min(bottom) {
                for next_x in x.saturating_sub(1)..=(x + 1).min(right) {
                    if candidates.remove(&(next_x, next_y)) {
                        queue.push_back((next_x, next_y));
                    }
                }
            }
        }
        if component.len() > maximum.len() {
            maximum = component;
        }
    }
    maximum
}

/// Removes only the strict neutral-gray connected component identified by the
/// footwear leak gate. Every pixel outside the recorded component remains
/// byte-identical to the source; no boot or pose synthesis is performed.
pub fn repair_neutral_gray_footwear_leak(
    image: &RgbaImage,
) -> (RgbaImage, FootwearNeutralGrayRepairReportV1) {
    let Some((left, top, right, bottom)) = alpha_bounds(image) else {
        return failed_repair(image, 0, "footwear_neutral_gray_repair_source_empty");
    };
    let component = largest_lower_neutral_gray_component_pixels(image, left, top, right, bottom);
    let core_pixels = component.len() as u32;
    if core_pixels < 16 {
        return failed_repair(
            image,
            core_pixels,
            "footwear_neutral_gray_repair_source_not_reproducible",
        );
    }
    let mut mask = component
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    for (x, y) in &component {
        for next_y in y.saturating_sub(1)..=(y + 1).min(bottom) {
            for next_x in x.saturating_sub(1)..=(x + 1).min(right) {
                if mask.contains(&(next_x, next_y)) {
                    continue;
                }
                let [red, green, blue, alpha] = image.get_pixel(next_x, next_y).0;
                let maximum = red.max(green).max(blue);
                let minimum = red.min(green).min(blue);
                let brightness = (u32::from(red) + u32::from(green) + u32::from(blue)) / 3;
                if alpha > 0 && brightness >= 55 && maximum.saturating_sub(minimum) <= 42 {
                    mask.insert((next_x, next_y));
                }
            }
        }
    }
    let component = mask.iter().copied().collect::<Vec<_>>();
    let detected_pixels = component.len() as u32;
    let min_x = component.iter().map(|(x, _)| *x).min().unwrap_or(0);
    let min_y = component.iter().map(|(_, y)| *y).min().unwrap_or(0);
    let max_x = component.iter().map(|(x, _)| *x).max().unwrap_or(0);
    let max_y = component.iter().map(|(_, y)| *y).max().unwrap_or(0);
    let mut repaired = image.clone();
    for (x, y) in &component {
        repaired.put_pixel(*x, *y, Rgba([0, 0, 0, 0]));
    }
    let non_mask_pixels_unchanged =
        image
            .pixels()
            .zip(repaired.pixels())
            .enumerate()
            .all(|(index, (before, after))| {
                let x = index as u32 % image.width();
                let y = index as u32 / image.width();
                mask.contains(&(x, y)) || before == after
            });
    let after = assess_footwear_platform("walk_down", 2, &repaired);
    let verdict = if non_mask_pixels_unchanged && after.verdict == ConsistencyVerdict::GameReady {
        ConsistencyVerdict::GameReady
    } else {
        ConsistencyVerdict::Blocked
    };
    let reasons = if verdict == ConsistencyVerdict::GameReady {
        Vec::new()
    } else {
        after.reasons
    };
    (
        repaired,
        FootwearNeutralGrayRepairReportV1 {
            schema_version: "1".into(),
            profile: FOOTWEAR_NEUTRAL_GRAY_REPAIR_PROFILE.into(),
            verdict,
            detected_pixels,
            modified_pixels: detected_pixels,
            bounds: Some([min_x, min_y, max_x, max_y]),
            non_mask_pixels_unchanged,
            reasons,
        },
    )
}

fn failed_repair(
    image: &RgbaImage,
    detected_pixels: u32,
    reason: &str,
) -> (RgbaImage, FootwearNeutralGrayRepairReportV1) {
    (
        image.clone(),
        FootwearNeutralGrayRepairReportV1 {
            schema_version: "1".into(),
            profile: FOOTWEAR_NEUTRAL_GRAY_REPAIR_PROFILE.into(),
            verdict: ConsistencyVerdict::Blocked,
            detected_pixels,
            modified_pixels: 0,
            bounds: None,
            non_mask_pixels_unchanged: true,
            reasons: vec![reason.into()],
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn assess_side(
    image: &RgbaImage,
    side: ScreenSideV1,
    side_left: u32,
    side_right: u32,
    body_top: u32,
    body_bottom: u32,
    body_width: u32,
    body_height: u32,
) -> FootwearPlatformSideMetricsV1 {
    let local_bottom = (body_top..=body_bottom)
        .rev()
        .find(|y| row_longest_run(image, *y, side_left, side_right).0 > 0)
        .unwrap_or(body_top);
    let bottom_band = (body_height / 14).clamp(5, 18);
    let shelf_start = local_bottom.saturating_sub(bottom_band.saturating_sub(1));
    let shelf_rows = (shelf_start..=local_bottom)
        .map(|y| {
            let (width, start, end) = row_longest_run(image, y, side_left, side_right);
            (y, width, start, end)
        })
        .collect::<Vec<_>>();
    let (_, shelf_width, shelf_left, shelf_right) = shelf_rows
        .iter()
        .max_by_key(|(_, width, _, _)| *width)
        .copied()
        .unwrap_or((local_bottom, 0, side_left, side_left));

    let stem_near = (body_height / 20).max(8);
    let stem_far = (body_height / 7).max(stem_near + 5);
    let stem_start = local_bottom.saturating_sub(stem_far).max(body_top);
    let stem_end = local_bottom.saturating_sub(stem_near).max(stem_start);
    let mut stem_widths = (stem_start..=stem_end)
        .map(|y| row_longest_run_in_window(image, y, shelf_left, shelf_right))
        .filter(|width| *width > 0)
        .collect::<Vec<_>>();
    stem_widths.sort_unstable();
    let stem_width = stem_widths
        .get(stem_widths.len().saturating_sub(1) / 3)
        .copied()
        .unwrap_or(1)
        .max(1);
    let broad_threshold = shelf_width.saturating_mul(3) / 4;
    let shelf_thickness = shelf_rows
        .iter()
        .filter(|(_, width, _, _)| *width >= broad_threshold && *width > 0)
        .count() as u32;
    let shelf_width_to_body_ratio = shelf_width as f32 / body_width.max(1) as f32;
    let shelf_to_stem_ratio = shelf_width as f32 / stem_width as f32;
    let platform_like = shelf_width >= 18
        && shelf_width_to_body_ratio >= 0.24
        && shelf_to_stem_ratio >= 2.0
        && shelf_width.saturating_sub(stem_width) >= 12
        && shelf_thickness <= (body_height / 9).max(6);
    FootwearPlatformSideMetricsV1 {
        side,
        local_bottom_y: local_bottom,
        shelf_width_px: shelf_width,
        stem_width_px: stem_width,
        shelf_thickness_px: shelf_thickness,
        shelf_width_to_body_ratio,
        shelf_to_stem_ratio,
        platform_like,
    }
}

fn alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] <= 16 {
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

fn row_longest_run(image: &RgbaImage, y: u32, left: u32, right: u32) -> (u32, u32, u32) {
    let mut best = (0, left, left);
    let mut start = None;
    for x in left..=right.min(image.width().saturating_sub(1)) {
        if image.get_pixel(x, y)[3] > 16 {
            start.get_or_insert(x);
        } else if let Some(run_start) = start.take() {
            let width = x - run_start;
            if width > best.0 {
                best = (width, run_start, x - 1);
            }
        }
    }
    if let Some(run_start) = start {
        let run_end = right.min(image.width().saturating_sub(1));
        let width = run_end - run_start + 1;
        if width > best.0 {
            best = (width, run_start, run_end);
        }
    }
    best
}

fn row_longest_run_in_window(image: &RgbaImage, y: u32, left: u32, right: u32) -> u32 {
    row_longest_run(image, y, left, right).0
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::*;

    fn sprite(platform_width: u32, boot_width: u32) -> RgbaImage {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        fill(&mut image, 28, 12, 68, 62, Rgba([40, 120, 60, 255]));
        fill(&mut image, 35, 58, 43, 84, Rgba([90, 50, 30, 255]));
        fill(&mut image, 54, 58, 62, 84, Rgba([90, 50, 30, 255]));
        fill(
            &mut image,
            39_u32.saturating_sub(boot_width / 2),
            84,
            39 + boot_width / 2,
            89,
            Rgba([75, 45, 30, 255]),
        );
        fill(
            &mut image,
            58_u32.saturating_sub(platform_width / 2),
            84,
            58 + platform_width / 2,
            90,
            Rgba([75, 45, 30, 255]),
        );
        image
    }

    fn fill(image: &mut RgbaImage, left: u32, top: u32, right: u32, bottom: u32, color: Rgba<u8>) {
        for y in top..=bottom {
            for x in left..=right {
                image.put_pixel(x, y, color);
            }
        }
    }

    #[test]
    fn wide_thin_platform_under_one_boot_is_blocked() {
        let report = assess_footwear_platform("walk_down", 2, &sprite(34, 14));
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert_eq!(report.reasons, [FOOTWEAR_PLATFORM_REASON]);
        assert!(report.sides.iter().any(|side| side.platform_like));
    }

    #[test]
    fn ordinary_boots_are_not_platforms() {
        let report = assess_footwear_platform("walk_down", 2, &sprite(14, 14));
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.reasons.is_empty());
    }

    #[test]
    fn detector_is_color_independent() {
        let mut image = sprite(34, 14);
        fill(&mut image, 41, 84, 75, 90, Rgba([190, 45, 20, 255]));
        assert_eq!(
            assess_footwear_platform("walk_down", 2, &image).verdict,
            ConsistencyVerdict::Blocked
        );
    }

    #[test]
    fn compact_neutral_gray_boot_leak_is_blocked() {
        let mut image = sprite(14, 14);
        fill(&mut image, 52, 70, 59, 77, Rgba([180, 178, 179, 255]));
        let report = assess_footwear_platform("walk_down", 2, &image);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.neutral_gray_leak_like);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason == FOOTWEAR_GUIDE_LEAK_REASON));
    }

    #[test]
    fn neutral_gray_repair_changes_only_detected_component() {
        let mut image = sprite(14, 14);
        fill(&mut image, 52, 70, 59, 77, Rgba([180, 178, 179, 255]));
        let source = image.clone();
        let (repaired, report) = repair_neutral_gray_footwear_leak(&image);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(report.modified_pixels, 64);
        assert!(report.non_mask_pixels_unchanged);
        for (index, (before, after)) in source.pixels().zip(repaired.pixels()).enumerate() {
            let x = index as u32 % source.width();
            let y = index as u32 / source.width();
            if (52..=59).contains(&x) && (70..=77).contains(&y) {
                assert_eq!(*after, Rgba([0, 0, 0, 0]));
            } else {
                assert_eq!(before, after);
            }
        }
    }
}
