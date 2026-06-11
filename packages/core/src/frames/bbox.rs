use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSize {
    pub width: u32,
    pub height: u32,
}

impl FrameSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameBbox {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub width: f32,
    pub height: f32,
    #[serde(rename = "centerX")]
    pub center_x: f32,
    #[serde(rename = "centerY")]
    pub center_y: f32,
    #[serde(rename = "bottomY")]
    pub bottom_y: f32,
    pub alpha_coverage: f32,
}

impl FrameBbox {
    pub fn from_bounds(left: f32, top: f32, right: f32, bottom: f32, alpha_coverage: f32) -> Self {
        let width = (right - left).max(0.0);
        let height = (bottom - top).max(0.0);
        Self {
            left,
            top,
            right,
            bottom,
            width,
            height,
            center_x: left + width / 2.0,
            center_y: top + height / 2.0,
            bottom_y: bottom,
            alpha_coverage: alpha_coverage.clamp(0.0, 1.0),
        }
    }

    pub fn empty() -> Self {
        Self::from_bounds(0.0, 0.0, 0.0, 0.0, 0.0)
    }

    pub fn translated(self, dx: f32, dy: f32) -> Self {
        Self::from_bounds(
            self.left + dx,
            self.top + dy,
            self.right + dx,
            self.bottom + dy,
            self.alpha_coverage,
        )
    }

    pub fn has_foreground(self) -> bool {
        self.alpha_coverage > 0.0 && self.width > 0.0 && self.height > 0.0
    }
}

pub fn bbox_from_image(image: &RgbaImage, alpha_threshold: u8) -> FrameBbox {
    bbox_from_alpha_rgba(
        image.width(),
        image.height(),
        image.as_raw(),
        alpha_threshold,
    )
}

pub fn bbox_from_alpha_rgba(
    width: u32,
    height: u32,
    rgba: &[u8],
    alpha_threshold: u8,
) -> FrameBbox {
    if width == 0 || height == 0 {
        return FrameBbox::empty();
    }

    let expected_len = width as usize * height as usize * 4;
    if rgba.len() < expected_len {
        return FrameBbox::empty();
    }

    let mut left = width;
    let mut top = height;
    let mut right = 0;
    let mut bottom = 0;
    let mut foreground_pixels = 0usize;

    for y in 0..height {
        for x in 0..width {
            let alpha_index = ((y * width + x) as usize * 4) + 3;
            if rgba[alpha_index] > alpha_threshold {
                left = left.min(x);
                top = top.min(y);
                right = right.max(x + 1);
                bottom = bottom.max(y + 1);
                foreground_pixels += 1;
            }
        }
    }

    if foreground_pixels == 0 {
        return FrameBbox::empty();
    }

    FrameBbox::from_bounds(
        left as f32,
        top as f32,
        right as f32,
        bottom as f32,
        foreground_pixels as f32 / (width as f32 * height as f32),
    )
}
