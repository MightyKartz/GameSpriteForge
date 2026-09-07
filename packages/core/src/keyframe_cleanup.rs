use std::collections::VecDeque;

use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::asset_project::ConsistencyVerdict;
use crate::frames::bbox_from_image;
use crate::matting::{apply_chroma_key, ChromaParameters};

pub const KEYFRAME_BACKGROUND_CLEANUP_PROFILE: &str = "keyframe-background-cleanup@1.3.0";
pub const CHECKERBOARD_SHEET_MATTING_PROFILE_LEGACY: &str = "checkerboard-sheet-matting@1.0.0";
pub const CHECKERBOARD_SHEET_MATTING_PROFILE: &str = "checkerboard-sheet-matting@1.1.0";
pub const ALPHA_EDGE_HALO_PROFILE: &str = "alpha-edge-halo@1.0.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckerboardSheetMattingReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub strategy: String,
    pub width: u32,
    pub height: u32,
    pub source_alpha_present: bool,
    pub border_checker_candidate_ratio: f32,
    pub darker_checker_luma: u8,
    pub lighter_checker_luma: u8,
    pub background_pixels_removed: usize,
    pub fringe_pixels_removed: usize,
    #[serde(default)]
    pub soft_alpha_pixels_reconstructed: usize,
    #[serde(default)]
    pub decontaminated_edge_pixels: usize,
    #[serde(default)]
    pub transparency_origin: String,
    pub cleaned_alpha_coverage: f32,
    pub cleaned_border_opaque_ratio: f32,
    pub significant_component_count: usize,
    pub neutral_edge_residual_pixels: usize,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AlphaEdgeHaloReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub transparency_origin: String,
    pub soft_edge_pixel_count: usize,
    pub opaque_edge_pixel_count: usize,
    pub neutral_halo_pixel_count: usize,
    pub dark_outline_discontinuity_pixel_count: usize,
    #[serde(default = "default_dark_outline_discontinuity_budget")]
    pub dark_outline_discontinuity_budget: usize,
    pub border_opaque_ratio: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// Converts a model-rendered white/light-gray checkerboard into real alpha
/// before the ordinary DirectionGrid layout gate runs. Only bright neutral
/// pixels connected to the outer sheet border may be cleared. This prevents
/// white eyes, scarf highlights and other enclosed character details from
/// being treated as background.
pub fn matte_checkerboard_sheet_background(
    source: &RgbaImage,
) -> (RgbaImage, CheckerboardSheetMattingReportV1) {
    let width = source.width() as usize;
    let height = source.height() as usize;
    let source_alpha_present = source.pixels().any(|pixel| pixel[3] < 250);
    let mut reasons = Vec::new();
    if width == 0 || height == 0 || width != height || !width.is_multiple_of(2) || width < 128 {
        reasons.push("checkerboard_sheet_geometry_invalid".into());
    }

    let mut border_luma = Vec::new();
    if width > 0 && height > 0 {
        for x in 0..width {
            for y in [0, height - 1] {
                let pixel = source.get_pixel(x as u32, y as u32);
                if is_checkerboard_background(pixel, false) {
                    border_luma.push(neutral_luma(pixel));
                }
            }
        }
        if height > 2 {
            for y in 1..height - 1 {
                for x in [0, width - 1] {
                    let pixel = source.get_pixel(x as u32, y as u32);
                    if is_checkerboard_background(pixel, false) {
                        border_luma.push(neutral_luma(pixel));
                    }
                }
            }
        }
    }
    let border_total = if width == 0 || height == 0 {
        0
    } else if width == 1 || height == 1 {
        width.saturating_mul(height)
    } else {
        width.saturating_mul(2) + height.saturating_sub(2).saturating_mul(2)
    };
    let border_checker_candidate_ratio = border_luma.len() as f32 / border_total.max(1) as f32;
    border_luma.sort_unstable();
    let darker_checker_luma = border_luma
        .get(border_luma.len().saturating_div(4))
        .copied()
        .unwrap_or_default();
    let lighter_checker_luma = border_luma
        .get(border_luma.len().saturating_mul(3).saturating_div(4))
        .copied()
        .unwrap_or_default();

    if !source_alpha_present {
        if border_checker_candidate_ratio < 0.80 {
            reasons.push("checkerboard_border_signature_missing".into());
        }
        if lighter_checker_luma.saturating_sub(darker_checker_luma) < 6 {
            reasons.push("checkerboard_tone_separation_missing".into());
        }
    }

    let mut cleared = vec![false; width.saturating_mul(height)];
    let mut queue = VecDeque::new();
    let enqueue =
        |x: usize, y: usize, cleared: &mut [bool], queue: &mut VecDeque<(usize, usize)>| {
            let index = y * width + x;
            if !cleared[index]
                && (source_alpha_present && source.get_pixel(x as u32, y as u32)[3] <= 16
                    || !source_alpha_present
                        && is_checkerboard_background(source.get_pixel(x as u32, y as u32), false))
            {
                cleared[index] = true;
                queue.push_back((x, y));
            }
        };
    if width > 0 && height > 0 {
        for x in 0..width {
            enqueue(x, 0, &mut cleared, &mut queue);
            enqueue(x, height - 1, &mut cleared, &mut queue);
        }
        for y in 0..height {
            enqueue(0, y, &mut cleared, &mut queue);
            enqueue(width - 1, y, &mut cleared, &mut queue);
        }
    }
    while let Some((x, y)) = queue.pop_front() {
        for (next_x, next_y) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if next_x < width && next_y < height {
                enqueue(next_x, next_y, &mut cleared, &mut queue);
            }
        }
    }
    let background_pixels_removed = cleared.iter().filter(|value| **value).count();

    // Two deterministic erosion passes remove only exposed, neutral matte
    // fringe. Saturated skin, hair, cape, scarf and leather edge pixels are
    // protected even when adjacent to transparency.
    let mut fringe_pixels_removed = 0usize;
    if !source_alpha_present && width > 2 && height > 2 {
        for _ in 0..2 {
            let mut fringe = Vec::new();
            for y in 1..height - 1 {
                for x in 1..width - 1 {
                    let index = y * width + x;
                    if cleared[index]
                        || !is_checkerboard_background(source.get_pixel(x as u32, y as u32), true)
                    {
                        continue;
                    }
                    let exposed = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                        .into_iter()
                        .any(|(neighbor_x, neighbor_y)| cleared[neighbor_y * width + neighbor_x]);
                    if exposed {
                        fringe.push(index);
                    }
                }
            }
            fringe_pixels_removed += fringe.len();
            for index in fringe {
                cleared[index] = true;
            }
        }
    }

    let mut output = source.clone();
    for (index, pixel) in output.pixels_mut().enumerate() {
        if cleared.get(index).copied().unwrap_or(false) || pixel[3] < 32 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
    let (soft_alpha_pixels_reconstructed, decontaminated_edge_pixels) = if source_alpha_present {
        (0, 0)
    } else {
        reconstruct_checkerboard_soft_edges(source, &cleared, &mut output)
    };

    let cleaned_alpha_coverage = alpha_coverage(&output);
    let cleaned_border_opaque_ratio = border_opaque_ratio(&output);
    let components = alpha_components(&output);
    let minimum_component_area = width.saturating_mul(height).div_ceil(400).max(16);
    let significant_component_count = components
        .iter()
        .filter(|component| component.len() >= minimum_component_area)
        .count();
    let neutral_edge_residual_pixels = exposed_neutral_edge_pixels(&output);
    if background_pixels_removed == 0 {
        reasons.push("checkerboard_background_not_removed".into());
    }
    if cleaned_border_opaque_ratio > 0.001 {
        reasons.push("checkerboard_border_residual".into());
    }
    if !(0.05..=0.60).contains(&cleaned_alpha_coverage) {
        reasons.push("checkerboard_foreground_coverage_out_of_range".into());
    }
    if significant_component_count != 4 {
        reasons.push("checkerboard_sheet_subject_count_invalid".into());
    }
    if neutral_edge_residual_pixels > 16 {
        reasons.push("checkerboard_neutral_edge_residual".into());
    }

    let strategy = if source_alpha_present {
        "native_alpha_passthrough"
    } else {
        "border_connected_light_checkerboard_soft_alpha"
    };
    let transparency_origin = if source_alpha_present {
        "native_alpha"
    } else {
        "deterministic_checkerboard_postprocess"
    };
    (
        output,
        CheckerboardSheetMattingReportV1 {
            schema_version: "1".into(),
            profile: CHECKERBOARD_SHEET_MATTING_PROFILE.into(),
            strategy: strategy.into(),
            width: source.width(),
            height: source.height(),
            source_alpha_present,
            border_checker_candidate_ratio,
            darker_checker_luma,
            lighter_checker_luma,
            background_pixels_removed,
            fringe_pixels_removed,
            soft_alpha_pixels_reconstructed,
            decontaminated_edge_pixels,
            transparency_origin: transparency_origin.into(),
            cleaned_alpha_coverage,
            cleaned_border_opaque_ratio,
            significant_component_count,
            neutral_edge_residual_pixels,
            verdict: if reasons.is_empty() {
                ConsistencyVerdict::GameReady
            } else {
                ConsistencyVerdict::Blocked
            },
            reasons,
        },
    )
}

pub fn assess_alpha_edge_halo(
    image: &RgbaImage,
    transparency_origin: &str,
) -> AlphaEdgeHaloReportV1 {
    assess_alpha_edge_halo_with_budget(image, transparency_origin, 8)
}

pub fn assess_alpha_edge_halo_with_budget(
    image: &RgbaImage,
    transparency_origin: &str,
    dark_outline_discontinuity_budget: usize,
) -> AlphaEdgeHaloReportV1 {
    let mut soft_edge_pixel_count = 0usize;
    let mut opaque_edge_pixel_count = 0usize;
    let mut neutral_halo_pixel_count = 0usize;
    let mut dark_outline_candidates = std::collections::BTreeSet::new();
    if image.width() >= 3 && image.height() >= 3 {
        for y in 1..image.height() - 1 {
            for x in 1..image.width() - 1 {
                let pixel = image.get_pixel(x, y);
                if pixel[3] == 0 {
                    continue;
                }
                let touches_transparency = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                    .into_iter()
                    .any(|(next_x, next_y)| image.get_pixel(next_x, next_y)[3] <= 16);
                if !touches_transparency {
                    continue;
                }
                if pixel[3] < 250 {
                    soft_edge_pixel_count += 1;
                } else {
                    opaque_edge_pixel_count += 1;
                }
                let minimum = pixel[0].min(pixel[1]).min(pixel[2]);
                let maximum = pixel[0].max(pixel[1]).max(pixel[2]);
                if pixel[3] < 250 && minimum >= 175 && maximum.saturating_sub(minimum) <= 28 {
                    neutral_halo_pixel_count += 1;
                }
                if pixel[3] < 180 && maximum <= 28 {
                    let touches_bright_foreground =
                        [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                            .into_iter()
                            .any(|(next_x, next_y)| {
                                let neighbor = image.get_pixel(next_x, next_y);
                                neighbor[3] >= 220 && neutral_luma(neighbor) >= 96
                            });
                    if touches_bright_foreground {
                        dark_outline_candidates.insert((x, y));
                    }
                }
            }
        }
    }
    let mut visited = std::collections::BTreeSet::new();
    let mut dark_outline_discontinuity_pixel_count = 0usize;
    for start in dark_outline_candidates.iter().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut area = 0usize;
        while let Some((x, y)) = queue.pop_front() {
            area += 1;
            for next_y in y.saturating_sub(1)..=(y + 1).min(image.height().saturating_sub(1)) {
                for next_x in x.saturating_sub(1)..=(x + 1).min(image.width().saturating_sub(1)) {
                    if dark_outline_candidates.contains(&(next_x, next_y))
                        && visited.insert((next_x, next_y))
                    {
                        queue.push_back((next_x, next_y));
                    }
                }
            }
        }
        if area <= 2 {
            dark_outline_discontinuity_pixel_count += area;
        }
    }
    let border_opaque_ratio = border_opaque_ratio(image);
    let mut reasons = Vec::new();
    if border_opaque_ratio > 0.001 {
        reasons.push("alpha_edge_border_residual".into());
    }
    if neutral_halo_pixel_count > 16 {
        reasons.push("alpha_edge_neutral_halo".into());
    }
    if dark_outline_discontinuity_pixel_count > dark_outline_discontinuity_budget {
        reasons.push("alpha_edge_dark_outline_discontinuity".into());
    }
    AlphaEdgeHaloReportV1 {
        schema_version: "1".into(),
        profile: ALPHA_EDGE_HALO_PROFILE.into(),
        transparency_origin: transparency_origin.into(),
        soft_edge_pixel_count,
        opaque_edge_pixel_count,
        neutral_halo_pixel_count,
        dark_outline_discontinuity_pixel_count,
        dark_outline_discontinuity_budget,
        border_opaque_ratio,
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        reasons,
    }
}

const fn default_dark_outline_discontinuity_budget() -> usize {
    8
}

fn reconstruct_checkerboard_soft_edges(
    source: &RgbaImage,
    cleared: &[bool],
    output: &mut RgbaImage,
) -> (usize, usize) {
    if source.width() < 3 || source.height() < 3 {
        return (0, 0);
    }
    let width = source.width() as usize;
    let mut updates = Vec::new();
    for y in 1..source.height() - 1 {
        for x in 1..source.width() - 1 {
            let index = y as usize * width + x as usize;
            if cleared.get(index).copied().unwrap_or(false) || output.get_pixel(x, y)[3] < 250 {
                continue;
            }
            let background_neighbors = [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .into_iter()
                .filter(|(next_x, next_y)| {
                    cleared
                        .get(*next_y as usize * width + *next_x as usize)
                        .copied()
                        .unwrap_or(false)
                })
                .collect::<Vec<_>>();
            if background_neighbors.is_empty() {
                continue;
            }
            let mut background = [0u32; 3];
            for (next_x, next_y) in &background_neighbors {
                let pixel = source.get_pixel(*next_x, *next_y);
                for channel in 0..3 {
                    background[channel] += u32::from(pixel[channel]);
                }
            }
            let count = background_neighbors.len() as u32;
            let background = background.map(|value| (value / count) as u8);
            let observed = source.get_pixel(x, y);
            let maximum_delta = (0..3)
                .map(|channel| observed[channel].abs_diff(background[channel]))
                .max()
                .unwrap_or_default();
            if !(12..180).contains(&maximum_delta) {
                continue;
            }
            let alpha = (((f32::from(maximum_delta) - 8.0) / 172.0) * 255.0)
                .round()
                .clamp(32.0, 244.0) as u8;
            let mut foreground = [0u8; 3];
            for channel in 0..3 {
                let numerator = i32::from(observed[channel]) * 255
                    - i32::from(background[channel]) * (255 - i32::from(alpha));
                foreground[channel] = (numerator / i32::from(alpha)).clamp(0, 255) as u8;
            }
            updates.push((
                x,
                y,
                Rgba([foreground[0], foreground[1], foreground[2], alpha]),
            ));
        }
    }
    for (x, y, pixel) in &updates {
        output.put_pixel(*x, *y, *pixel);
    }
    (updates.len(), updates.len())
}

fn neutral_luma(pixel: &Rgba<u8>) -> u8 {
    ((u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2])) / 3) as u8
}

fn is_checkerboard_background(pixel: &Rgba<u8>, loose_fringe: bool) -> bool {
    if pixel[3] <= 16 {
        return true;
    }
    let minimum = pixel[0].min(pixel[1]).min(pixel[2]);
    let maximum = pixel[0].max(pixel[1]).max(pixel[2]);
    minimum >= if loose_fringe { 180 } else { 218 }
        && maximum.saturating_sub(minimum) <= if loose_fringe { 24 } else { 14 }
}

fn exposed_neutral_edge_pixels(image: &RgbaImage) -> usize {
    if image.width() < 3 || image.height() < 3 {
        return 0;
    }
    let mut count = 0usize;
    for y in 1..image.height() - 1 {
        for x in 1..image.width() - 1 {
            let pixel = image.get_pixel(x, y);
            if pixel[3] <= 32 || !is_checkerboard_background(pixel, true) {
                continue;
            }
            if [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
                .into_iter()
                .any(|(neighbor_x, neighbor_y)| image.get_pixel(neighbor_x, neighbor_y)[3] <= 32)
            {
                count += 1;
            }
        }
    }
    count
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KeyframeBackgroundCleanupReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub strategy: String,
    pub raw_alpha_coverage: f32,
    pub raw_border_opaque_ratio: f32,
    pub cleaned_alpha_coverage: f32,
    pub cleaned_border_opaque_ratio: f32,
    pub source_component_count: usize,
    pub retained_component_count: usize,
    pub removed_foreground_pixels: usize,
    /// Pixels removed from a thin, wide baseline/floor mark near the bottom of
    /// the canvas. This cleanup runs before component retention because image
    /// models often draw the line through the character's boots.
    #[serde(default)]
    pub ground_line_removed_pixels: usize,
    #[serde(default)]
    pub ground_line_residual: bool,
    /// Small exposed line-like chroma-key remnants removed only from the lower
    /// foot plane after matting. Broad or vertically substantial green boots,
    /// clothing, and equipment remain character art.
    #[serde(default)]
    pub foot_plane_chroma_removed_pixels: usize,
    #[serde(default)]
    pub foot_plane_chroma_residual: bool,
    /// Magenta compression spill removed after subject-component retention.
    /// Thin connected contamination in a concave gap is removed as one
    /// component when any part of it remains exposed to transparency.
    #[serde(default)]
    pub magenta_spill_removed_pixels: usize,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

/// Removes a provider-created scene or flat background before a frame can be
/// used as a later image reference. The operation is deterministic and does
/// not crop or rescale the subject; normalization remains a separate stage.
pub fn cleanup_keyframe_background(
    source: &RgbaImage,
) -> (RgbaImage, KeyframeBackgroundCleanupReportV1) {
    let raw_alpha_coverage = alpha_coverage(source);
    let raw_border_opaque_ratio = border_opaque_ratio(source);
    let has_useful_native_alpha = source.pixels().any(|pixel| pixel[3] <= 16)
        && source.pixels().any(|pixel| pixel[3] >= 240)
        && raw_border_opaque_ratio <= 0.10;
    let corner_key = average_corner_color(source);
    let dark_neutral_background = corner_key.is_some_and(|key| {
        let maximum = key.iter().copied().max().unwrap_or_default();
        let minimum = key.iter().copied().min().unwrap_or_default();
        maximum <= 48 && maximum.saturating_sub(minimum) <= 20
    });
    let (mut candidate, strategy) = if has_useful_native_alpha {
        (source.clone(), "native_alpha")
    } else if dark_neutral_background {
        (
            remove_dark_border_background(source, corner_key.expect("dark key exists")),
            "border_connected_dark",
        )
    } else {
        let mut chroma = ChromaParameters::default();
        if corner_key.is_some_and(is_magenta_key) {
            // Browser video compression blends a 1-3px magenta band into the
            // opaque character outline. A three-source-pixel alpha contraction
            // removes that exposed contamination while remaining sub-pixel
            // after a 1440px source is scaled to the Godot review canvas.
            chroma.halo_pixels = 3;
        }
        (
            apply_chroma_key(source, &chroma).unwrap_or_else(|_| source.clone()),
            "chroma_auto",
        )
    };
    for pixel in candidate.pixels_mut() {
        if pixel[3] < 32 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
    let (candidate_without_ground, ground_line_removed_pixels) =
        remove_horizontal_ground_artifact(&candidate);
    candidate = candidate_without_ground;
    let ground_line_residual = has_horizontal_ground_artifact(&candidate);
    let (candidate_without_chroma, foot_plane_chroma_removed_pixels) =
        remove_foot_plane_chroma_residual(&candidate);
    candidate = candidate_without_chroma;
    let foot_plane_chroma_residual = has_foot_plane_chroma_residual(&candidate);

    let components = alpha_components(&candidate);
    let source_component_count = components.len();
    let (mut retained, retained_component_count) = if has_useful_native_alpha {
        retain_native_alpha_components(&candidate, &components)
    } else {
        retain_subject_components(&candidate, &components)
    };
    let magenta_spill_removed_pixels = if corner_key.is_some_and(is_magenta_key) {
        remove_exposed_magenta_spill(&mut retained, 12)
    } else {
        0
    };
    let before = candidate.pixels().filter(|pixel| pixel[3] > 32).count();
    let after = retained.pixels().filter(|pixel| pixel[3] > 32).count();
    let cleaned_alpha_coverage = alpha_coverage(&retained);
    let cleaned_border_opaque_ratio = border_opaque_ratio(&retained);
    let mut reasons = Vec::new();
    if after == 0 {
        reasons.push("background_cleanup_foreground_missing".into());
    }
    if cleaned_border_opaque_ratio > 0.01 {
        reasons.push("background_cleanup_border_residual".into());
    }
    if !(0.005..=0.75).contains(&cleaned_alpha_coverage) {
        reasons.push("background_cleanup_coverage_out_of_range".into());
    }
    if ground_line_residual {
        reasons.push("background_cleanup_ground_line_residual".into());
    }
    if foot_plane_chroma_residual {
        reasons.push("background_cleanup_foot_plane_chroma_residual".into());
    }
    (
        retained,
        KeyframeBackgroundCleanupReportV1 {
            schema_version: "1".into(),
            profile: KEYFRAME_BACKGROUND_CLEANUP_PROFILE.into(),
            strategy: strategy.into(),
            raw_alpha_coverage,
            raw_border_opaque_ratio,
            cleaned_alpha_coverage,
            cleaned_border_opaque_ratio,
            source_component_count,
            retained_component_count,
            removed_foreground_pixels: before.saturating_sub(after),
            ground_line_removed_pixels,
            ground_line_residual,
            foot_plane_chroma_removed_pixels,
            foot_plane_chroma_residual,
            magenta_spill_removed_pixels,
            verdict: if reasons.is_empty() {
                ConsistencyVerdict::GameReady
            } else {
                ConsistencyVerdict::Blocked
            },
            reasons,
        },
    )
}

fn is_magenta_key(key: [u8; 3]) -> bool {
    key[0] >= key[1].saturating_add(64)
        && key[2] >= key[1].saturating_add(64)
        && key[0].abs_diff(key[2]) <= 64
}

fn remove_exposed_magenta_spill(image: &mut RgbaImage, radius: u32) -> usize {
    let mut total_removed = 0usize;
    for _ in 0..4 {
        let removed = remove_exposed_magenta_spill_pass(image, radius);
        total_removed += removed;
        if removed == 0 {
            break;
        }
    }
    total_removed
}

fn remove_exposed_magenta_spill_pass(image: &mut RgbaImage, radius: u32) -> usize {
    let source = image.clone();
    let width = source.width() as usize;
    let height = source.height() as usize;
    let mut candidates = vec![false; width.saturating_mul(height)];
    for (x, y, pixel) in source.enumerate_pixels() {
        if is_magenta_spill_pixel(pixel) {
            candidates[y as usize * width + x as usize] = true;
        }
    }
    let maximum_artifact_area = width.saturating_mul(height).div_ceil(512).max(256);
    let mut visited = vec![false; candidates.len()];
    let mut removed = Vec::new();
    for start_y in 0..height {
        for start_x in 0..width {
            let start = start_y * width + start_x;
            if visited[start] || !candidates[start] {
                continue;
            }
            visited[start] = true;
            let mut queue = VecDeque::from([(start_x, start_y)]);
            let mut component = Vec::new();
            let mut exposed_pixels = Vec::new();
            let mut minimum_x = start_x;
            let mut maximum_x = start_x;
            let mut minimum_y = start_y;
            let mut maximum_y = start_y;
            while let Some((x, y)) = queue.pop_front() {
                component.push((x as u32, y as u32));
                minimum_x = minimum_x.min(x);
                maximum_x = maximum_x.max(x);
                minimum_y = minimum_y.min(y);
                maximum_y = maximum_y.max(y);
                if touches_transparency_within(&source, x as u32, y as u32, radius) {
                    exposed_pixels.push((x as u32, y as u32));
                }
                for next_y in y.saturating_sub(1)..=(y + 1).min(height.saturating_sub(1)) {
                    for next_x in x.saturating_sub(1)..=(x + 1).min(width.saturating_sub(1)) {
                        let next = next_y * width + next_x;
                        if !visited[next] && candidates[next] {
                            visited[next] = true;
                            queue.push_back((next_x, next_y));
                        }
                    }
                }
            }
            if exposed_pixels.is_empty() {
                continue;
            }
            let component_width = maximum_x - minimum_x + 1;
            let component_height = maximum_y - minimum_y + 1;
            let short_side = component_width.min(component_height);
            let long_side = component_width.max(component_height);
            let line_like =
                short_side <= radius as usize + 2 || long_side >= short_side.saturating_mul(3);
            if line_like && component.len() <= maximum_artifact_area {
                removed.extend(component);
            } else {
                removed.extend(exposed_pixels);
            }
        }
    }
    removed.sort_unstable();
    removed.dedup();
    for (x, y) in &removed {
        image.put_pixel(*x, *y, Rgba([0, 0, 0, 0]));
    }
    removed.len()
}

fn is_magenta_spill_pixel(pixel: &Rgba<u8>) -> bool {
    pixel[3] > 16
        && pixel[0] >= pixel[1].saturating_add(24)
        && pixel[2] >= pixel[1].saturating_add(24)
        && pixel[0].max(pixel[2]) >= 64
        && pixel[0].abs_diff(pixel[2]) <= 96
}

fn touches_transparency_within(image: &RgbaImage, x: u32, y: u32, radius: u32) -> bool {
    (y.saturating_sub(radius)..=(y + radius).min(image.height().saturating_sub(1))).any(|next_y| {
        (x.saturating_sub(radius)..=(x + radius).min(image.width().saturating_sub(1)))
            .any(|next_x| image.get_pixel(next_x, next_y)[3] <= 16)
    })
}

fn remove_foot_plane_chroma_residual(source: &RgbaImage) -> (RgbaImage, usize) {
    let artifact_pixels = foot_plane_chroma_artifact_pixels(source);
    let mut output = source.clone();
    for (x, y) in &artifact_pixels {
        output.put_pixel(*x, *y, Rgba([0, 0, 0, 0]));
    }
    (output, artifact_pixels.len())
}

fn has_foot_plane_chroma_residual(source: &RgbaImage) -> bool {
    !foot_plane_chroma_artifact_pixels(source).is_empty()
}

/// Return only small, exposed, line-like chroma components at the foot plane.
/// A broad or vertically substantial saturated-green component is treated as
/// character art (for example boots, a robe, or equipment) and is preserved.
fn foot_plane_chroma_artifact_pixels(source: &RgbaImage) -> Vec<(u32, u32)> {
    let bbox = bbox_from_image(source, 32);
    if !bbox.has_foreground() {
        return Vec::new();
    }
    let minimum_y = (bbox.top + bbox.height * 0.68)
        .max(source.height() as f32 * 0.65)
        .floor() as u32;
    let width = source.width() as usize;
    let height = source.height() as usize;
    let mut visited = vec![false; width.saturating_mul(height)];
    let maximum_artifact_area = width.saturating_mul(height).div_ceil(200).max(64);
    let mut artifacts = Vec::new();
    for start_y in minimum_y.min(source.height())..source.height() {
        for start_x in 0..source.width() {
            let start_index = start_y as usize * width + start_x as usize;
            if visited[start_index]
                || source.get_pixel(start_x, start_y)[3] == 0
                || !is_foot_plane_chroma_green(source.get_pixel(start_x, start_y))
            {
                continue;
            }
            visited[start_index] = true;
            let mut queue = VecDeque::from([(start_x, start_y)]);
            let mut component = Vec::new();
            let mut min_x = start_x;
            let mut max_x = start_x;
            let mut min_y = start_y;
            let mut max_y = start_y;
            let mut exposed = false;
            while let Some((x, y)) = queue.pop_front() {
                component.push((x, y));
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
                for offset_y in -1_i32..=1 {
                    for offset_x in -1_i32..=1 {
                        if offset_x == 0 && offset_y == 0 {
                            continue;
                        }
                        let neighbor_x = x as i32 + offset_x;
                        let neighbor_y = y as i32 + offset_y;
                        if neighbor_x < 0
                            || neighbor_y < 0
                            || neighbor_x >= source.width() as i32
                            || neighbor_y >= source.height() as i32
                        {
                            exposed = true;
                            continue;
                        }
                        let neighbor_x = neighbor_x as u32;
                        let neighbor_y = neighbor_y as u32;
                        let neighbor = source.get_pixel(neighbor_x, neighbor_y);
                        if neighbor[3] <= 32 {
                            exposed = true;
                            continue;
                        }
                        if neighbor_y < minimum_y || !is_foot_plane_chroma_green(neighbor) {
                            continue;
                        }
                        let neighbor_index = neighbor_y as usize * width + neighbor_x as usize;
                        if !visited[neighbor_index] {
                            visited[neighbor_index] = true;
                            queue.push_back((neighbor_x, neighbor_y));
                        }
                    }
                }
            }
            let component_width = max_x - min_x + 1;
            let component_height = max_y - min_y + 1;
            let component_bbox_area = component_width as usize * component_height as usize;
            let sparse = component.len().saturating_mul(2) <= component_bbox_area;
            let line_like = component_height <= 3
                || component_width <= 3
                || (component_height <= 6 && component_width >= component_height * 3)
                || sparse;
            if exposed && line_like && component.len() <= maximum_artifact_area {
                artifacts.extend(component);
            }
        }
    }
    artifacts
}

fn is_foot_plane_chroma_green(pixel: &Rgba<u8>) -> bool {
    pixel[1] >= 64
        && pixel[1] >= pixel[0].saturating_add(18)
        && pixel[1] >= pixel[2].saturating_add(15)
}

/// Remove the common generated "floor" artifact without resizing or
/// inpainting the subject. A suspect row must be both wide and predominantly
/// unsupported vertically, which distinguishes a 1-3px baseline from boots,
/// capes, and other legitimate broad silhouettes. Pixels with substantial
/// foreground immediately above or below are protected.
fn remove_horizontal_ground_artifact(source: &RgbaImage) -> (RgbaImage, usize) {
    if source.width() < 16 || source.height() < 16 {
        return (source.clone(), 0);
    }
    let suspect_rows = horizontal_ground_rows(source);
    if suspect_rows.is_empty() {
        return (source.clone(), 0);
    }
    let mut output = source.clone();
    let mut removed = 0usize;
    for suspect_y in suspect_rows {
        let first_y = suspect_y.saturating_sub(1);
        let last_y = (suspect_y + 1).min(source.height() - 1);
        for y in first_y..=last_y {
            for x in 0..source.width() {
                if output.get_pixel(x, y)[3] <= 32 {
                    continue;
                }
                let support_above = vertical_support(source, x, y, -6, -2);
                let support_below = vertical_support(source, x, y, 2, 6);
                if support_above >= 2 || support_below >= 2 {
                    continue;
                }
                output.put_pixel(x, y, Rgba([0, 0, 0, 0]));
                removed += 1;
            }
        }
    }
    (output, removed)
}

fn has_horizontal_ground_artifact(image: &RgbaImage) -> bool {
    !horizontal_ground_rows(image).is_empty()
}

fn horizontal_ground_rows(image: &RgbaImage) -> Vec<u32> {
    let minimum_y = image.height() * 65 / 100;
    let minimum_opaque = (image.width() * 35 / 100).max(8);
    let mut rows = Vec::new();
    for y in minimum_y..image.height() {
        let opaque = (0..image.width())
            .filter(|x| image.get_pixel(*x, y)[3] > 32)
            .count() as u32;
        if opaque < minimum_opaque {
            continue;
        }
        let unsupported = (0..image.width())
            .filter(|x| image.get_pixel(*x, y)[3] > 32)
            .filter(|x| {
                vertical_support(image, *x, y, -5, -1) + vertical_support(image, *x, y, 1, 5) <= 2
            })
            .count() as u32;
        if unsupported * 100 >= opaque * 40 {
            rows.push(y);
        }
    }
    rows
}

fn vertical_support(image: &RgbaImage, x: u32, y: u32, start: i32, end: i32) -> usize {
    (start..=end)
        .filter_map(|offset| {
            let candidate = y as i32 + offset;
            (candidate >= 0 && candidate < image.height() as i32).then_some(candidate as u32)
        })
        .filter(|candidate_y| image.get_pixel(x, *candidate_y)[3] > 32)
        .count()
}

fn average_corner_color(image: &RgbaImage) -> Option<[u8; 3]> {
    if image.width() == 0 || image.height() == 0 {
        return None;
    }
    let corners = [
        image.get_pixel(0, 0),
        image.get_pixel(image.width() - 1, 0),
        image.get_pixel(0, image.height() - 1),
        image.get_pixel(image.width() - 1, image.height() - 1),
    ];
    Some([
        (corners.iter().map(|pixel| u16::from(pixel[0])).sum::<u16>() / 4) as u8,
        (corners.iter().map(|pixel| u16::from(pixel[1])).sum::<u16>() / 4) as u8,
        (corners.iter().map(|pixel| u16::from(pixel[2])).sum::<u16>() / 4) as u8,
    ])
}

/// Dark backgrounds need a border-connected operation. Applying a global
/// black chroma key destroys legitimate outlines, boots, hair, and clothing.
/// Non-background color seeds protect a scale-aware neighborhood before the
/// dark border flood is cleared.
fn remove_dark_border_background(source: &RgbaImage, key: [u8; 3]) -> RgbaImage {
    let width = source.width() as usize;
    let height = source.height() as usize;
    if width == 0 || height == 0 {
        return source.clone();
    }
    let threshold_squared = 72_i32.pow(2);
    let differs_from_key = |pixel: &Rgba<u8>| {
        let red = i32::from(pixel[0]) - i32::from(key[0]);
        let green = i32::from(pixel[1]) - i32::from(key[1]);
        let blue = i32::from(pixel[2]) - i32::from(key[2]);
        red * red + green * green + blue * blue > threshold_squared
    };
    let mut protected = vec![false; width * height];
    let radius = (source.width().max(source.height()) / 256).clamp(1, 6) as i32;
    for (x, y, pixel) in source.enumerate_pixels() {
        if pixel[3] <= 32 || !differs_from_key(pixel) {
            continue;
        }
        for offset_y in -radius..=radius {
            for offset_x in -radius..=radius {
                let next_x = x as i32 + offset_x;
                let next_y = y as i32 + offset_y;
                if next_x >= 0 && next_y >= 0 && next_x < width as i32 && next_y < height as i32 {
                    protected[next_y as usize * width + next_x as usize] = true;
                }
            }
        }
    }
    let close_to_key = |pixel: &Rgba<u8>| !differs_from_key(pixel);
    let mut visited = vec![false; width * height];
    let mut queue = VecDeque::new();
    let enqueue = |x: usize, y: usize, visited: &mut [bool], queue: &mut VecDeque<usize>| {
        let index = y * width + x;
        if !visited[index]
            && !protected[index]
            && close_to_key(source.get_pixel(x as u32, y as u32))
        {
            visited[index] = true;
            queue.push_back(index);
        }
    };
    for x in 0..width {
        enqueue(x, 0, &mut visited, &mut queue);
        if height > 1 {
            enqueue(x, height - 1, &mut visited, &mut queue);
        }
    }
    for y in 0..height {
        enqueue(0, y, &mut visited, &mut queue);
        if width > 1 {
            enqueue(width - 1, y, &mut visited, &mut queue);
        }
    }
    let mut output = source.clone();
    while let Some(index) = queue.pop_front() {
        let x = index % width;
        let y = index / width;
        output.get_pixel_mut(x as u32, y as u32)[3] = 0;
        for (next_x, next_y) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if next_x < width && next_y < height {
                enqueue(next_x, next_y, &mut visited, &mut queue);
            }
        }
    }
    output
}

fn alpha_coverage(image: &RgbaImage) -> f32 {
    let total = image.width() as usize * image.height() as usize;
    if total == 0 {
        return 0.0;
    }
    image.pixels().filter(|pixel| pixel[3] > 32).count() as f32 / total as f32
}

fn border_opaque_ratio(image: &RgbaImage) -> f32 {
    if image.width() == 0 || image.height() == 0 {
        return 0.0;
    }
    let mut opaque = 0usize;
    let mut total = 0usize;
    for x in 0..image.width() {
        for y in [0, image.height() - 1] {
            total += 1;
            opaque += usize::from(image.get_pixel(x, y)[3] > 32);
        }
    }
    if image.height() > 2 {
        for y in 1..image.height() - 1 {
            for x in [0, image.width() - 1] {
                total += 1;
                opaque += usize::from(image.get_pixel(x, y)[3] > 32);
            }
        }
    }
    opaque as f32 / total.max(1) as f32
}

fn alpha_components(image: &RgbaImage) -> Vec<Vec<(u32, u32)>> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut visited = vec![false; width.saturating_mul(height)];
    let mut components = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            if visited[index] || image.get_pixel(x as u32, y as u32)[3] <= 32 {
                continue;
            }
            visited[index] = true;
            let mut queue = VecDeque::from([(x, y)]);
            let mut component = Vec::new();
            while let Some((current_x, current_y)) = queue.pop_front() {
                component.push((current_x as u32, current_y as u32));
                for (next_x, next_y) in [
                    (current_x.wrapping_sub(1), current_y),
                    (current_x + 1, current_y),
                    (current_x, current_y.wrapping_sub(1)),
                    (current_x, current_y + 1),
                ] {
                    if next_x >= width || next_y >= height {
                        continue;
                    }
                    let next = next_y * width + next_x;
                    if !visited[next] && image.get_pixel(next_x as u32, next_y as u32)[3] > 32 {
                        visited[next] = true;
                        queue.push_back((next_x, next_y));
                    }
                }
            }
            components.push(component);
        }
    }
    components.sort_by_key(|component| std::cmp::Reverse(component.len()));
    components
}

fn retain_subject_components(
    image: &RgbaImage,
    components: &[Vec<(u32, u32)>],
) -> (RgbaImage, usize) {
    let mut output = image.clone();
    for pixel in output.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }
    let Some(main) = components.iter().find(|component| {
        !component
            .iter()
            .any(|(x, y)| *x == 0 || *y == 0 || *x + 1 == image.width() || *y + 1 == image.height())
    }) else {
        return (output, 0);
    };
    let (main_left, main_top, main_right, main_bottom) = component_bounds(main);
    let proximity = image.width().max(image.height()).div_ceil(32).max(2);
    let minimum_satellite = (main.len() / 100).max(4);
    let mut retained_count = 0usize;
    for component in components {
        let touches_border = component.iter().any(|(x, y)| {
            *x == 0 || *y == 0 || *x + 1 == image.width() || *y + 1 == image.height()
        });
        let bounds = component_bounds(component);
        let close_to_main =
            rectangle_gap((main_left, main_top, main_right, main_bottom), bounds) <= proximity;
        if touches_border
            || (!std::ptr::eq(component, main)
                && (component.len() < minimum_satellite || !close_to_main))
        {
            continue;
        }
        retained_count += 1;
        for (x, y) in component {
            output.put_pixel(*x, *y, *image.get_pixel(*x, *y));
        }
    }
    (output, retained_count)
}

fn retain_native_alpha_components(
    image: &RgbaImage,
    components: &[Vec<(u32, u32)>],
) -> (RgbaImage, usize) {
    let mut output = image.clone();
    for pixel in output.pixels_mut() {
        *pixel = Rgba([0, 0, 0, 0]);
    }
    let mut retained_count = 0usize;
    for component in components {
        let touches_border = component.iter().any(|(x, y)| {
            *x == 0 || *y == 0 || *x + 1 == image.width() || *y + 1 == image.height()
        });
        if touches_border || component.len() < 4 {
            continue;
        }
        retained_count += 1;
        for (x, y) in component {
            output.put_pixel(*x, *y, *image.get_pixel(*x, *y));
        }
    }
    (output, retained_count)
}

fn component_bounds(component: &[(u32, u32)]) -> (u32, u32, u32, u32) {
    (
        component.iter().map(|(x, _)| *x).min().unwrap_or(0),
        component.iter().map(|(_, y)| *y).min().unwrap_or(0),
        component.iter().map(|(x, _)| *x).max().unwrap_or(0),
        component.iter().map(|(_, y)| *y).max().unwrap_or(0),
    )
}

fn rectangle_gap(a: (u32, u32, u32, u32), b: (u32, u32, u32, u32)) -> u32 {
    let horizontal = if a.2 < b.0 {
        b.0 - a.2
    } else if b.2 < a.0 {
        a.0.saturating_sub(b.2)
    } else {
        0
    };
    let vertical = if a.3 < b.1 {
        b.1 - a.3
    } else if b.3 < a.1 {
        a.1.saturating_sub(b.3)
    } else {
        0
    };
    horizontal.max(vertical)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkerboard_sheet() -> RgbaImage {
        let mut image = RgbaImage::new(256, 256);
        for y in 0..256 {
            for x in 0..256 {
                let tone = if (x / 32 + y / 32) % 2 == 0 { 236 } else { 254 };
                image.put_pixel(x, y, Rgba([tone, tone, tone, 255]));
            }
        }
        for (left, top, color) in [
            (40, 30, [90, 120, 60, 255]),
            (168, 30, [100, 130, 70, 255]),
            (40, 158, [110, 80, 50, 255]),
            (168, 158, [120, 90, 60, 255]),
        ] {
            for y in top..top + 68 {
                for x in left..left + 48 {
                    image.put_pixel(x, y, Rgba(color));
                }
            }
            // Enclosed bright character detail must not be globally keyed.
            image.put_pixel(left + 24, top + 20, Rgba([250, 250, 250, 255]));
            // One simulated antialiased edge pixel blended with the light
            // checker must become soft Alpha rather than a hard fringe.
            image.put_pixel(left - 1, top + 10, Rgba([160, 180, 148, 255]));
        }
        image
    }

    #[test]
    fn checkerboard_sheet_matting_removes_only_border_connected_neutral_tiles() {
        let source = checkerboard_sheet();
        let (cleaned, report) = matte_checkerboard_sheet_background(&source);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(
            report.strategy,
            "border_connected_light_checkerboard_soft_alpha"
        );
        assert_eq!(
            report.transparency_origin,
            "deterministic_checkerboard_postprocess"
        );
        assert_eq!(report.significant_component_count, 4);
        assert!(report.background_pixels_removed > 40_000);
        assert!(report.soft_alpha_pixels_reconstructed >= 4);
        assert_eq!(report.neutral_edge_residual_pixels, 0);
        assert_eq!(cleaned.get_pixel(0, 0), &Rgba([0, 0, 0, 0]));
        assert_eq!(cleaned.get_pixel(20, 50), &Rgba([0, 0, 0, 0]));
        assert_eq!(cleaned.get_pixel(40 + 24, 30 + 20)[3], 255);
        assert!((32..245).contains(&cleaned.get_pixel(39, 40)[3]));
        let halo = assess_alpha_edge_halo(&cleaned, &report.transparency_origin);
        assert_eq!(halo.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(halo.neutral_halo_pixel_count, 0);
    }

    #[test]
    fn alpha_edge_halo_blocks_many_isolated_dark_partial_fragments() {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 0]));
        for index in 0..9u32 {
            let y = 6 + index * 9;
            image.put_pixel(40, y, Rgba([180, 120, 80, 255]));
            image.put_pixel(41, y, Rgba([10, 10, 10, 120]));
        }
        let report = assess_alpha_edge_halo(&image, "deterministic_checkerboard_postprocess");
        assert_eq!(report.dark_outline_discontinuity_pixel_count, 9);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .contains(&"alpha_edge_dark_outline_discontinuity".into()));
    }

    #[test]
    fn flat_white_sheet_is_not_misrepresented_as_a_checkerboard() {
        let source = RgbaImage::from_pixel(256, 256, Rgba([250, 250, 250, 255]));
        let (_, report) = matte_checkerboard_sheet_background(&source);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .contains(&"checkerboard_tone_separation_missing".into()));
        assert!(report
            .reasons
            .contains(&"checkerboard_sheet_subject_count_invalid".into()));
    }

    #[test]
    fn saturated_border_residual_fails_closed() {
        let mut source = checkerboard_sheet();
        for y in 0..32 {
            source.put_pixel(0, y, Rgba([200, 20, 30, 255]));
        }
        let (_, report) = matte_checkerboard_sheet_background(&source);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .contains(&"checkerboard_border_residual".into()));
    }

    #[test]
    fn removes_opaque_border_background_and_detached_noise() {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([40, 52, 70, 255]));
        for y in 20..84 {
            for x in 34..62 {
                image.put_pixel(x, y, Rgba([180, 80, 120, 255]));
            }
        }
        for y in 40..44 {
            for x in 78..82 {
                image.put_pixel(x, y, Rgba([220, 180, 50, 255]));
            }
        }
        let (cleaned, report) = cleanup_keyframe_background(&image);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(cleaned.get_pixel(0, 0)[3], 0);
        assert_eq!(cleaned.get_pixel(79, 41)[3], 0);
        assert!(cleaned.get_pixel(48, 48)[3] > 32);
    }

    #[test]
    fn magenta_video_cleanup_contracts_spill_band() {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 255, 255]));
        for y in 8..56 {
            for x in 18..46 {
                image.put_pixel(x, y, Rgba([160, 20, 160, 255]));
            }
        }
        for y in 10..54 {
            for x in 20..44 {
                image.put_pixel(x, y, Rgba([120, 75, 45, 255]));
            }
        }
        image.put_pixel(21, 31, Rgba([120, 20, 118, 255]));

        let (cleaned, report) = cleanup_keyframe_background(&image);

        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(cleaned.get_pixel(18, 32)[3], 0);
        assert_eq!(cleaned.get_pixel(19, 32)[3], 0);
        assert_eq!(cleaned.get_pixel(20, 32)[3], 0);
        assert!(cleaned.get_pixel(21, 32)[3] > 32);
        assert_eq!(cleaned.get_pixel(21, 31)[3], 0);
        assert!(cleaned.get_pixel(32, 32)[3] > 32);
    }

    #[test]
    fn magenta_spill_cleanup_removes_an_enclosed_tail_from_one_exposed_seed() {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 16..56 {
            for x in 18..30 {
                image.put_pixel(x, y, Rgba([80, 48, 30, 255]));
            }
            for x in 34..46 {
                image.put_pixel(x, y, Rgba([80, 48, 30, 255]));
            }
        }
        for y in 20..56 {
            for x in 30..34 {
                image.put_pixel(x, y, Rgba([110, 20, 100, 255]));
            }
        }

        let removed = remove_exposed_magenta_spill(&mut image, 6);

        assert_eq!(removed, 4 * 36);
        assert_eq!(image.get_pixel(31, 22)[3], 0);
        assert_eq!(image.get_pixel(31, 54)[3], 0);
        assert_eq!(image.get_pixel(24, 30), &Rgba([80, 48, 30, 255]));
    }

    #[test]
    fn keeps_existing_transparent_subject() {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 10..58 {
            for x in 22..42 {
                image.put_pixel(x, y, Rgba([130, 60, 210, 255]));
            }
        }
        let (cleaned, report) = cleanup_keyframe_background(&image);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(cleaned, image);
    }

    #[test]
    fn removes_thin_generated_ground_line_without_erasing_boots() {
        let mut image = RgbaImage::from_pixel(128, 128, Rgba([0, 255, 0, 255]));
        for y in 20..111 {
            for x in 40..88 {
                image.put_pixel(x, y, Rgba([120, 75, 45, 255]));
            }
        }
        for x in 14..114 {
            image.put_pixel(x, 112, Rgba([35, 28, 20, 255]));
        }
        let (cleaned, report) = cleanup_keyframe_background(&image);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.ground_line_removed_pixels >= 40);
        assert!(!report.ground_line_residual);
        assert_eq!(cleaned.get_pixel(20, 112)[3], 0);
        assert!(cleaned.get_pixel(64, 109)[3] > 32);
    }

    #[test]
    fn does_not_remove_supported_wide_character_silhouette() {
        let mut image = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 40..116 {
            for x in 28..100 {
                image.put_pixel(x, y, Rgba([80, 120, 70, 255]));
            }
        }
        let (cleaned, report) = cleanup_keyframe_background(&image);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(report.ground_line_removed_pixels, 0);
        assert_eq!(cleaned, image);
    }

    #[test]
    fn removes_saturated_foot_plane_chroma_without_erasing_moss_cloak() {
        let mut image = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 24..100 {
            for x in 36..92 {
                image.put_pixel(x, y, Rgba([74, 96, 48, 255]));
            }
        }
        for y in 100..118 {
            for x in 46..62 {
                image.put_pixel(x, y, Rgba([70, 44, 28, 255]));
            }
            for x in 68..84 {
                image.put_pixel(x, y, Rgba([70, 44, 28, 255]));
            }
        }
        for x in [50, 51, 76, 77] {
            image.put_pixel(x, 118, Rgba([4, 210, 8, 180]));
        }

        let (cleaned, report) = cleanup_keyframe_background(&image);

        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(report.foot_plane_chroma_removed_pixels, 4);
        assert!(!report.foot_plane_chroma_residual);
        assert_eq!(cleaned.get_pixel(50, 118)[3], 0);
        assert_eq!(cleaned.get_pixel(48, 80), &Rgba([74, 96, 48, 255]));
        assert_eq!(cleaned.get_pixel(50, 110), &Rgba([70, 44, 28, 255]));
    }

    #[test]
    fn preserves_saturated_green_boots_as_character_art() {
        let mut image = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 24..100 {
            for x in 36..92 {
                image.put_pixel(x, y, Rgba([90, 70, 48, 255]));
            }
        }
        for y in 98..120 {
            for x in 42..60 {
                image.put_pixel(x, y, Rgba([12, 190, 28, 255]));
            }
            for x in 68..86 {
                image.put_pixel(x, y, Rgba([12, 190, 28, 255]));
            }
        }

        let (cleaned, report) = cleanup_keyframe_background(&image);

        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(report.foot_plane_chroma_removed_pixels, 0);
        assert_eq!(cleaned, image);
    }

    #[test]
    fn removes_sparse_t_shaped_chroma_spill_between_boots() {
        let mut image = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 24..100 {
            for x in 36..92 {
                image.put_pixel(x, y, Rgba([90, 70, 48, 255]));
            }
        }
        for y in 100..120 {
            for x in 40..58 {
                image.put_pixel(x, y, Rgba([70, 44, 28, 255]));
            }
            for x in 70..88 {
                image.put_pixel(x, y, Rgba([70, 44, 28, 255]));
            }
        }
        for y in 108..122 {
            image.put_pixel(64, y, Rgba([4, 210, 8, 180]));
            image.put_pixel(65, y, Rgba([4, 210, 8, 180]));
        }
        for x in 54..76 {
            image.put_pixel(x, 121, Rgba([4, 210, 8, 180]));
        }

        let (cleaned, report) = cleanup_keyframe_background(&image);

        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.foot_plane_chroma_removed_pixels >= 40);
        assert_eq!(cleaned.get_pixel(64, 112)[3], 0);
        assert_eq!(cleaned.get_pixel(56, 121)[3], 0);
        assert_eq!(cleaned.get_pixel(48, 110), &Rgba([70, 44, 28, 255]));
    }

    #[test]
    fn preserves_disconnected_native_alpha_body_parts() {
        let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 8..28 {
            for x in 20..44 {
                image.put_pixel(x, y, Rgba([160, 90, 60, 255]));
            }
        }
        for y in 36..56 {
            for x in 18..30 {
                image.put_pixel(x, y, Rgba([45, 35, 30, 255]));
            }
            for x in 34..46 {
                image.put_pixel(x, y, Rgba([45, 35, 30, 255]));
            }
        }
        let (cleaned, report) = cleanup_keyframe_background(&image);
        assert_eq!(report.strategy, "native_alpha");
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(cleaned, image);
    }

    #[test]
    fn dark_border_cleanup_preserves_enclosed_black_details() {
        let mut image = RgbaImage::from_pixel(96, 96, Rgba([0, 0, 0, 255]));
        for y in 18..84 {
            for x in 28..68 {
                image.put_pixel(x, y, Rgba([170, 90, 60, 255]));
            }
        }
        for y in 40..56 {
            for x in 40..56 {
                image.put_pixel(x, y, Rgba([8, 8, 8, 255]));
            }
        }
        let (cleaned, report) = cleanup_keyframe_background(&image);
        assert_eq!(report.strategy, "border_connected_dark");
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(cleaned.get_pixel(0, 0)[3], 0);
        assert_eq!(cleaned.get_pixel(48, 48), &Rgba([8, 8, 8, 255]));
    }
}
