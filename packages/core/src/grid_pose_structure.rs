use std::path::Path;

use image::{Rgba, RgbaImage};
use thiserror::Error;

pub const GRID_POSE_STRUCTURE_PROFILE_LEGACY: &str = "grid-pose-structure@1.0.0";
pub const GRID_POSE_STRUCTURE_PROFILE: &str = "grid-pose-structure@1.1.0";
pub const GRID_POSE_STRUCTURE_PROFILE_PLATFORM_SAFE: &str = "grid-pose-structure@1.2.0";
pub const POSE_STRUCTURE_LEAK_REASON: &str = "pose_structure_grayscale_leak";

const LEGACY_INK: Rgba<u8> = Rgba([173, 173, 173, 196]);
const LEGACY_JOINT: Rgba<u8> = Rgba([91, 91, 91, 224]);
const NEUTRAL_INK: Rgba<u8> = Rgba([160, 160, 160, 220]);
const GROUNDED_INK: Rgba<u8> = Rgba([232, 232, 232, 232]);
const GROUNDED_JOINT: Rgba<u8> = Rgba([208, 208, 208, 232]);
const GROUNDED_SOLE: Rgba<u8> = Rgba([248, 248, 248, 240]);
const RAISED_INK: Rgba<u8> = Rgba([72, 72, 72, 232]);
const RAISED_JOINT: Rgba<u8> = Rgba([88, 88, 88, 232]);
const RAISED_BOOT: Rgba<u8> = Rgba([56, 56, 56, 240]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LimbRole {
    Grounded,
    Raised,
}

#[derive(Debug, Error)]
pub enum GridPoseStructureError {
    #[error("pose structure supports only walk_down frames 0..3")]
    UnsupportedPhase,
    #[error("pose structure canvas must be at least 64 pixels")]
    CanvasTooSmall,
    #[error("pose structure contains a non-grayscale pixel")]
    ColoredPixel,
    #[error("pose structure contains an opaque canvas panel")]
    OpaquePanel,
    #[error("pose structure contains no structural ink")]
    Empty,
    #[error("pose structure contains ink outside the deterministic V1.1 palette")]
    UnsupportedInk,
    #[error("pose structure is missing explicit grounded or raised limb-role ink")]
    LimbRoleMissing,
    #[error("pose structure limb-role ink is on the wrong viewer-space side")]
    LimbRoleSideMismatch,
    #[error("pose structure raised boot is not visibly above the grounded boot")]
    LiftInsufficient,
    #[error("pose structure grounded boot has no wide horizontal sole marker")]
    GroundedSoleMissing,
    #[error("pose structure grounded contact marker is missing")]
    GroundedContactMissing,
    #[error("pose structure grounded contact marker is too wide and could become a platform")]
    GroundedContactTooWide,
    #[error("pose structure contains a floor-like horizontal line")]
    FloorLine,
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
}

/// Writes the asymmetric V1.1 structure contract. Bright/thick ink and a
/// horizontal sole encode the viewer-space grounded leg; dark/thin ink and a
/// compact boot encode the raised leg. The values are motion-role metadata,
/// never appearance colors.
pub fn write_walk_down_pose_structure(
    path: &Path,
    frame: u8,
    canvas: u32,
) -> Result<(), GridPoseStructureError> {
    write_walk_down_pose_structure_contract(path, frame, canvas, true, false)
}

/// Writes the platform-safe V1.2 structure contract. Limb brightness and
/// thickness encode support versus swing, while the planted endpoint remains
/// compact. No horizontal sole, floor, or platform-like marker is present.
pub fn write_walk_down_pose_structure_v12(
    path: &Path,
    frame: u8,
    canvas: u32,
) -> Result<(), GridPoseStructureError> {
    write_walk_down_pose_structure_contract(path, frame, canvas, true, true)
}

/// Preserves the exact V1.0 guide used by immutable topdown-grid@9.2.0 Jobs.
pub fn write_walk_down_pose_structure_legacy(
    path: &Path,
    frame: u8,
    canvas: u32,
) -> Result<(), GridPoseStructureError> {
    write_walk_down_pose_structure_contract(path, frame, canvas, false, false)
}

fn write_walk_down_pose_structure_contract(
    path: &Path,
    frame: u8,
    canvas: u32,
    asymmetric_roles: bool,
    platform_safe: bool,
) -> Result<(), GridPoseStructureError> {
    if frame > 3 {
        return Err(GridPoseStructureError::UnsupportedPhase);
    }
    if canvas < 64 {
        return Err(GridPoseStructureError::CanvasTooSmall);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(image::ImageError::IoError)?;
    }
    let mut image = RgbaImage::from_pixel(canvas, canvas, Rgba([0, 0, 0, 0]));
    let center = (canvas / 2) as i32;
    let scale = canvas as f32 / 256.0;
    let passing = frame % 2 == 1;
    let screen_left_phase = frame < 2;
    let bob = if passing {
        -(5.0 * scale).round() as i32
    } else {
        0
    };
    let head_y = (canvas as f32 * 0.30) as i32 + bob;
    let shoulder_y = (canvas as f32 * 0.43) as i32 + bob;
    let hip_y = (canvas as f32 * 0.63) as i32 + bob;
    let ground_y = (canvas as f32 * 0.84) as i32;

    let neutral = if asymmetric_roles {
        NEUTRAL_INK
    } else {
        LEGACY_INK
    };
    draw_disc(&mut image, center, head_y, scaled(14.0, scale), neutral);
    draw_line(
        &mut image,
        center,
        head_y + scaled(13.0, scale),
        center,
        hip_y,
        neutral,
        scaled(7.0, scale),
    );
    let arm = if screen_left_phase { 5 } else { -5 };
    draw_line(
        &mut image,
        center - scaled(7.0, scale),
        shoulder_y,
        center - scaled(12.0, scale) + scaled(arm as f32, scale),
        shoulder_y + scaled(28.0, scale),
        neutral,
        scaled(4.0, scale),
    );
    draw_line(
        &mut image,
        center + scaled(7.0, scale),
        shoulder_y,
        center + scaled(12.0, scale) - scaled(arm as f32, scale),
        shoulder_y + scaled(28.0, scale),
        neutral,
        scaled(4.0, scale),
    );

    let (screen_left_foot, screen_right_foot) = match (asymmetric_roles, frame) {
        (true, 0) => (
            (center - scaled(30.0, scale), ground_y),
            (center + scaled(10.0, scale), ground_y - scaled(30.0, scale)),
        ),
        (true, 1) => (
            (center - scaled(8.0, scale), ground_y),
            (center + scaled(7.0, scale), ground_y - scaled(38.0, scale)),
        ),
        (true, 2) => (
            (center - scaled(10.0, scale), ground_y - scaled(30.0, scale)),
            (center + scaled(30.0, scale), ground_y),
        ),
        (true, 3) => (
            (center - scaled(7.0, scale), ground_y - scaled(38.0, scale)),
            (center + scaled(8.0, scale), ground_y),
        ),
        (false, 0) => (
            (center - scaled(25.0, scale), ground_y),
            (center + scaled(12.0, scale), ground_y - scaled(10.0, scale)),
        ),
        (false, 1) => (
            (center - scaled(7.0, scale), ground_y),
            (center + scaled(3.0, scale), ground_y - scaled(29.0, scale)),
        ),
        (false, 2) => (
            (center - scaled(12.0, scale), ground_y - scaled(10.0, scale)),
            (center + scaled(25.0, scale), ground_y),
        ),
        (false, 3) => (
            (center - scaled(3.0, scale), ground_y - scaled(29.0, scale)),
            (center + scaled(7.0, scale), ground_y),
        ),
        _ => unreachable!(),
    };
    if asymmetric_roles {
        let screen_left_role = if screen_left_phase {
            LimbRole::Grounded
        } else {
            LimbRole::Raised
        };
        let screen_right_role = if screen_left_phase {
            LimbRole::Raised
        } else {
            LimbRole::Grounded
        };
        draw_asymmetric_limb(
            &mut image,
            (center - scaled(4.0, scale), hip_y),
            screen_left_foot,
            scale,
            screen_left_role,
            platform_safe,
        );
        draw_asymmetric_limb(
            &mut image,
            (center + scaled(4.0, scale), hip_y),
            screen_right_foot,
            scale,
            screen_right_role,
            platform_safe,
        );
    } else {
        draw_legacy_limb(
            &mut image,
            (center - scaled(4.0, scale), hip_y),
            screen_left_foot,
            scale,
        );
        draw_legacy_limb(
            &mut image,
            (center + scaled(4.0, scale), hip_y),
            screen_right_foot,
            scale,
        );
    }
    validate_pose_structure(&image)?;
    if asymmetric_roles {
        if platform_safe {
            validate_walk_down_pose_structure_v12(&image, frame)?;
        } else {
            validate_walk_down_pose_structure_v11(&image, frame)?;
        }
    }
    image.save(path)?;
    Ok(())
}

pub fn validate_pose_structure(image: &RgbaImage) -> Result<(), GridPoseStructureError> {
    let mut ink = 0usize;
    for pixel in image.pixels() {
        if pixel[3] <= 8 {
            continue;
        }
        ink += 1;
        if pixel[0] != pixel[1] || pixel[1] != pixel[2] {
            return Err(GridPoseStructureError::ColoredPixel);
        }
    }
    if ink == 0 {
        return Err(GridPoseStructureError::Empty);
    }
    let canvas = usize::try_from(image.width()).unwrap_or_default()
        * usize::try_from(image.height()).unwrap_or_default();
    if canvas > 0 && ink as f32 / canvas as f32 > 0.35 {
        return Err(GridPoseStructureError::OpaquePanel);
    }
    Ok(())
}

/// Validates the explicit viewer-space role legend used by V1.1. The
/// thresholds intentionally inspect only exact deterministic guide ink, not a
/// generated sprite.
pub fn validate_walk_down_pose_structure_v11(
    image: &RgbaImage,
    frame: u8,
) -> Result<(), GridPoseStructureError> {
    if frame > 3 {
        return Err(GridPoseStructureError::UnsupportedPhase);
    }
    validate_pose_structure(image)?;
    let allowed_ink = [
        NEUTRAL_INK,
        GROUNDED_INK,
        GROUNDED_JOINT,
        GROUNDED_SOLE,
        RAISED_INK,
        RAISED_JOINT,
        RAISED_BOOT,
    ];
    if image
        .pixels()
        .any(|pixel| pixel[3] > 8 && !allowed_ink.contains(pixel))
    {
        return Err(GridPoseStructureError::UnsupportedInk);
    }
    let center = image.width() / 2;
    let lower_start = image.height() * 2 / 3;
    let grounded_left = frame < 2;
    let mut grounded = Vec::new();
    let mut raised = Vec::new();
    let mut widest_row = 0_u32;
    for y in 0..image.height() {
        let mut row_min = image.width();
        let mut row_max = 0_u32;
        let mut row_ink = false;
        for x in 0..image.width() {
            let pixel = image.get_pixel(x, y);
            if pixel[3] <= 8 {
                continue;
            }
            row_min = row_min.min(x);
            row_max = row_max.max(x);
            row_ink = true;
            if y < lower_start {
                continue;
            }
            if pixel[0] >= 200 {
                grounded.push((x, y));
            } else if pixel[0] <= 96 {
                raised.push((x, y));
            }
        }
        if row_ink {
            widest_row = widest_row.max(row_max.saturating_sub(row_min).saturating_add(1));
        }
    }
    if grounded.is_empty() || raised.is_empty() {
        return Err(GridPoseStructureError::LimbRoleMissing);
    }
    let grounded_expected = grounded
        .iter()
        .filter(|(x, _)| (*x < center) == grounded_left)
        .count();
    let raised_expected = raised
        .iter()
        .filter(|(x, _)| (*x < center) != grounded_left)
        .count();
    if grounded_expected * 10 < grounded.len() * 9 || raised_expected * 10 < raised.len() * 9 {
        return Err(GridPoseStructureError::LimbRoleSideMismatch);
    }
    let grounded_bottom = grounded.iter().map(|(_, y)| *y).max().unwrap_or_default();
    let raised_bottom = raised.iter().map(|(_, y)| *y).max().unwrap_or_default();
    let minimum_lift = (image.height() / 25).max(2);
    if grounded_bottom < raised_bottom.saturating_add(minimum_lift) {
        return Err(GridPoseStructureError::LiftInsufficient);
    }
    let sole_band_start = grounded_bottom.saturating_sub((image.height() / 64).max(2));
    let sole_x = grounded
        .iter()
        .filter(|(_, y)| *y >= sole_band_start)
        .map(|(x, _)| *x)
        .collect::<Vec<_>>();
    let sole_span = sole_x
        .iter()
        .max()
        .zip(sole_x.iter().min())
        .map(|(maximum, minimum)| maximum.saturating_sub(*minimum).saturating_add(1))
        .unwrap_or_default();
    if sole_span < (image.width() / 18).max(6) {
        return Err(GridPoseStructureError::GroundedSoleMissing);
    }
    if widest_row > image.width() * 3 / 5 {
        return Err(GridPoseStructureError::FloorLine);
    }
    Ok(())
}

/// Validates V1.2 and fails closed if the grounded endpoint expands into a
/// horizontal sole. This keeps screen-side role strength without presenting a
/// drawable platform primitive to an image model.
pub fn validate_walk_down_pose_structure_v12(
    image: &RgbaImage,
    frame: u8,
) -> Result<(), GridPoseStructureError> {
    if frame > 3 {
        return Err(GridPoseStructureError::UnsupportedPhase);
    }
    validate_pose_structure(image)?;
    let allowed_ink = [
        NEUTRAL_INK,
        GROUNDED_INK,
        GROUNDED_JOINT,
        RAISED_INK,
        RAISED_JOINT,
        RAISED_BOOT,
    ];
    if image
        .pixels()
        .any(|pixel| pixel[3] > 8 && !allowed_ink.contains(pixel))
    {
        return Err(GridPoseStructureError::UnsupportedInk);
    }
    let center = image.width() / 2;
    let lower_start = image.height() * 2 / 3;
    let grounded_left = frame < 2;
    let mut grounded = Vec::new();
    let mut raised = Vec::new();
    let mut widest_row = 0_u32;
    for y in 0..image.height() {
        let mut row_min = image.width();
        let mut row_max = 0_u32;
        let mut row_ink = false;
        for x in 0..image.width() {
            let pixel = image.get_pixel(x, y);
            if pixel[3] <= 8 {
                continue;
            }
            row_min = row_min.min(x);
            row_max = row_max.max(x);
            row_ink = true;
            if y < lower_start {
                continue;
            }
            if pixel[0] >= 200 {
                grounded.push((x, y));
            } else if pixel[0] <= 96 {
                raised.push((x, y));
            }
        }
        if row_ink {
            widest_row = widest_row.max(row_max.saturating_sub(row_min).saturating_add(1));
        }
    }
    if grounded.is_empty() || raised.is_empty() {
        return Err(GridPoseStructureError::LimbRoleMissing);
    }
    let grounded_expected = grounded
        .iter()
        .filter(|(x, _)| (*x < center) == grounded_left)
        .count();
    let raised_expected = raised
        .iter()
        .filter(|(x, _)| (*x < center) != grounded_left)
        .count();
    if grounded_expected * 10 < grounded.len() * 9 || raised_expected * 10 < raised.len() * 9 {
        return Err(GridPoseStructureError::LimbRoleSideMismatch);
    }
    let grounded_bottom = grounded.iter().map(|(_, y)| *y).max().unwrap_or_default();
    let raised_bottom = raised.iter().map(|(_, y)| *y).max().unwrap_or_default();
    let minimum_lift = (image.height() / 25).max(2);
    if grounded_bottom < raised_bottom.saturating_add(minimum_lift) {
        return Err(GridPoseStructureError::LiftInsufficient);
    }
    let contact_band_start = grounded_bottom.saturating_sub((image.height() / 48).max(3));
    let contact_x = grounded
        .iter()
        .filter(|(_, y)| *y >= contact_band_start)
        .map(|(x, _)| *x)
        .collect::<Vec<_>>();
    let contact_span = contact_x
        .iter()
        .max()
        .zip(contact_x.iter().min())
        .map(|(maximum, minimum)| maximum.saturating_sub(*minimum).saturating_add(1))
        .unwrap_or_default();
    if contact_span < (image.width() / 64).max(3) {
        return Err(GridPoseStructureError::GroundedContactMissing);
    }
    if contact_span > (image.width() / 16).max(8) {
        return Err(GridPoseStructureError::GroundedContactTooWide);
    }
    if widest_row > image.width() * 3 / 5 {
        return Err(GridPoseStructureError::FloorLine);
    }
    Ok(())
}

/// Detects literal guide-ink copying. It intentionally requires both rare
/// grayscale values and spatial agreement with the guide, so ordinary gray
/// clothing away from the skeleton does not become a false hard failure.
pub fn pose_structure_leak_ratio(candidate: &RgbaImage, guide: &RgbaImage) -> f32 {
    if candidate.dimensions() != guide.dimensions() {
        return 1.0;
    }
    let mut guide_ink = 0usize;
    let mut matching = 0usize;
    for (candidate, guide) in candidate.pixels().zip(guide.pixels()) {
        if guide[3] <= 8 {
            continue;
        }
        guide_ink += 1;
        let gray = i16::from(guide[0]);
        let distance = (i16::from(candidate[0]) - gray).abs()
            + (i16::from(candidate[1]) - gray).abs()
            + (i16::from(candidate[2]) - gray).abs();
        matching += usize::from(candidate[3] > 16 && distance <= 18);
    }
    if guide_ink == 0 {
        1.0
    } else {
        matching as f32 / guide_ink as f32
    }
}

pub fn has_pose_structure_leak(candidate: &RgbaImage, guide: &RgbaImage) -> bool {
    pose_structure_leak_ratio(candidate, guide) > 0.22
}

fn draw_legacy_limb(image: &mut RgbaImage, hip: (i32, i32), foot: (i32, i32), scale: f32) {
    let knee = (
        (hip.0 + foot.0) / 2,
        (hip.1 + foot.1) / 2 - scaled(3.0, scale),
    );
    draw_line(
        image,
        hip.0,
        hip.1,
        knee.0,
        knee.1,
        LEGACY_INK,
        scaled(6.0, scale),
    );
    draw_disc(image, knee.0, knee.1, scaled(4.0, scale), LEGACY_JOINT);
    draw_line(
        image,
        knee.0,
        knee.1,
        foot.0,
        foot.1,
        LEGACY_INK,
        scaled(6.0, scale),
    );
    draw_disc(image, foot.0, foot.1, scaled(5.0, scale), LEGACY_JOINT);
}

fn draw_asymmetric_limb(
    image: &mut RgbaImage,
    hip: (i32, i32),
    foot: (i32, i32),
    scale: f32,
    role: LimbRole,
    platform_safe: bool,
) {
    let knee = (
        (hip.0 + foot.0) / 2,
        (hip.1 + foot.1) / 2 - scaled(3.0, scale),
    );
    let (ink, joint, width) = match role {
        LimbRole::Grounded => (GROUNDED_INK, GROUNDED_JOINT, scaled(8.0, scale)),
        LimbRole::Raised => (RAISED_INK, RAISED_JOINT, scaled(4.0, scale)),
    };
    draw_line(image, hip.0, hip.1, knee.0, knee.1, ink, width);
    draw_disc(image, knee.0, knee.1, width / 2, joint);
    draw_line(image, knee.0, knee.1, foot.0, foot.1, ink, width);
    match role {
        LimbRole::Grounded if platform_safe => {
            draw_disc(image, foot.0, foot.1, scaled(4.0, scale), GROUNDED_JOINT);
            draw_line(
                image,
                foot.0,
                foot.1 - scaled(4.0, scale),
                foot.0,
                foot.1 + scaled(2.0, scale),
                GROUNDED_JOINT,
                scaled(4.0, scale),
            );
        }
        LimbRole::Grounded => draw_line(
            image,
            foot.0 - scaled(10.0, scale),
            foot.1,
            foot.0 + scaled(10.0, scale),
            foot.1,
            GROUNDED_SOLE,
            scaled(6.0, scale),
        ),
        LimbRole::Raised => {
            draw_disc(image, foot.0, foot.1, scaled(3.0, scale), RAISED_BOOT);
            draw_line(
                image,
                foot.0 - scaled(2.0, scale),
                foot.1 - scaled(2.0, scale),
                foot.0 + scaled(5.0, scale),
                foot.1 + scaled(2.0, scale),
                RAISED_BOOT,
                scaled(3.0, scale),
            );
        }
    }
}

fn scaled(value: f32, scale: f32) -> i32 {
    (value * scale).round().max(1.0) as i32
}

fn draw_disc(image: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    for y in cy - radius..=cy + radius {
        for x in cx - radius..=cx + radius {
            if (x - cx).pow(2) + (y - cy).pow(2) <= radius.pow(2) {
                put(image, x, y, color);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_line(
    image: &mut RgbaImage,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Rgba<u8>,
    width: i32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        draw_disc(image, x0, y0, width / 2, color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let doubled = error * 2;
        if doubled >= dy {
            error += dy;
            x0 += sx;
        }
        if doubled <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn put(image: &mut RgbaImage, x: i32, y: i32, color: Rgba<u8>) {
    if x >= 0 && y >= 0 && x < image.width() as i32 && y < image.height() as i32 {
        image.put_pixel(x as u32, y as u32, color);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use sha2::{Digest, Sha256};

    use super::*;

    #[test]
    fn four_walk_down_structures_are_distinct_transparent_and_grayscale() {
        let temp = tempfile::tempdir().unwrap();
        let mut hashes = BTreeSet::new();
        for frame in 0..4 {
            let path = temp.path().join(format!("frame-{frame}.png"));
            write_walk_down_pose_structure(&path, frame, 256).unwrap();
            let image = image::open(path).unwrap().to_rgba8();
            validate_pose_structure(&image).unwrap();
            validate_walk_down_pose_structure_v11(&image, frame).unwrap();
            assert!(image.pixels().any(|pixel| pixel[3] == 0));
            assert!(image
                .pixels()
                .filter(|pixel| pixel[3] > 8)
                .all(|pixel| pixel[0] == pixel[1] && pixel[1] == pixel[2]));
            hashes.insert(format!("{:x}", Sha256::digest(image.as_raw())));
        }
        assert_eq!(hashes.len(), 4);
    }

    #[test]
    fn asymmetric_role_legend_flips_viewer_space_side_and_lifts_the_swing_boot() {
        let temp = tempfile::tempdir().unwrap();
        for frame in 0..4_u8 {
            let path = temp.path().join(format!("frame-{frame}.png"));
            write_walk_down_pose_structure(&path, frame, 256).unwrap();
            let image = image::open(path).unwrap().to_rgba8();
            let center = image.width() / 2;
            let lower_start = image.height() * 2 / 3;
            let bright = image
                .enumerate_pixels()
                .filter(|(_, y, pixel)| *y >= lower_start && pixel[3] > 8 && pixel[0] >= 200)
                .map(|(x, y, _)| (x, y))
                .collect::<Vec<_>>();
            let dark = image
                .enumerate_pixels()
                .filter(|(_, y, pixel)| *y >= lower_start && pixel[3] > 8 && pixel[0] <= 96)
                .map(|(x, y, _)| (x, y))
                .collect::<Vec<_>>();
            let grounded_left = frame < 2;
            assert!(
                bright
                    .iter()
                    .filter(|(x, _)| (*x < center) == grounded_left)
                    .count()
                    * 10
                    >= bright.len() * 9
            );
            assert!(
                dark.iter()
                    .filter(|(x, _)| (*x < center) != grounded_left)
                    .count()
                    * 10
                    >= dark.len() * 9
            );
            assert!(bright.iter().map(|(_, y)| y).max() > dark.iter().map(|(_, y)| y).max());
        }
    }

    #[test]
    fn legacy_structure_remains_distinct_from_the_asymmetric_contract() {
        let temp = tempfile::tempdir().unwrap();
        let legacy_path = temp.path().join("legacy.png");
        let asymmetric_path = temp.path().join("asymmetric.png");
        write_walk_down_pose_structure_legacy(&legacy_path, 2, 256).unwrap();
        write_walk_down_pose_structure(&asymmetric_path, 2, 256).unwrap();
        let legacy = image::open(legacy_path).unwrap().to_rgba8();
        let asymmetric = image::open(asymmetric_path).unwrap().to_rgba8();
        validate_pose_structure(&legacy).unwrap();
        assert!(validate_walk_down_pose_structure_v11(&legacy, 2).is_err());
        assert_ne!(legacy.as_raw(), asymmetric.as_raw());
    }

    #[test]
    fn v12_platform_safe_guides_have_compact_contacts_and_no_sole_ink() {
        let temp = tempfile::tempdir().unwrap();
        let mut hashes = BTreeSet::new();
        for frame in 0..4_u8 {
            let path = temp.path().join(format!("frame-{frame}.png"));
            write_walk_down_pose_structure_v12(&path, frame, 256).unwrap();
            let image = image::open(path).unwrap().to_rgba8();
            validate_walk_down_pose_structure_v12(&image, frame).unwrap();
            assert!(!image.pixels().any(|pixel| *pixel == GROUNDED_SOLE));
            assert!(matches!(
                validate_walk_down_pose_structure_v11(&image, frame),
                Err(GridPoseStructureError::GroundedSoleMissing)
                    | Err(GridPoseStructureError::UnsupportedInk)
            ));
            hashes.insert(format!("{:x}", Sha256::digest(image.as_raw())));
        }
        assert_eq!(hashes.len(), 4);
    }

    #[test]
    fn v12_validator_rejects_a_reintroduced_horizontal_platform_marker() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("guide.png");
        write_walk_down_pose_structure_v12(&path, 2, 256).unwrap();
        let mut image = image::open(path).unwrap().to_rgba8();
        let center = image.width() as i32 / 2;
        let ground_y = (image.height() as f32 * 0.84) as i32;
        draw_line(
            &mut image,
            center + 18,
            ground_y,
            center + 46,
            ground_y,
            GROUNDED_JOINT,
            5,
        );
        assert!(matches!(
            validate_walk_down_pose_structure_v12(&image, 2),
            Err(GridPoseStructureError::GroundedContactTooWide)
        ));
    }

    #[test]
    fn literal_structure_copy_is_blocked_but_unrelated_color_is_not() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("guide.png");
        write_walk_down_pose_structure(&path, 0, 96).unwrap();
        let guide = image::open(path).unwrap().to_rgba8();
        assert!(has_pose_structure_leak(&guide, &guide));
        let unrelated = RgbaImage::from_pixel(96, 96, Rgba([120, 80, 40, 255]));
        assert!(!has_pose_structure_leak(&unrelated, &guide));
    }

    #[test]
    fn v11_validator_rejects_unversioned_ink_and_floor_lines() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("guide.png");
        write_walk_down_pose_structure(&path, 2, 256).unwrap();
        let guide = image::open(path).unwrap().to_rgba8();

        let mut unknown_ink = guide.clone();
        unknown_ink.put_pixel(0, 0, Rgba([123, 123, 123, 200]));
        assert!(matches!(
            validate_walk_down_pose_structure_v11(&unknown_ink, 2),
            Err(GridPoseStructureError::UnsupportedInk)
        ));

        let mut floor_line = guide;
        for x in 0..floor_line.width() {
            floor_line.put_pixel(x, floor_line.height() - 1, NEUTRAL_INK);
        }
        assert!(matches!(
            validate_walk_down_pose_structure_v11(&floor_line, 2),
            Err(GridPoseStructureError::FloorLine)
        ));
    }
}
