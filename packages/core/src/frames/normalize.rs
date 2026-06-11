use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use super::anchor::{default_foot_anchor, FootAnchor};
use super::bbox::{bbox_from_image, FrameBbox, FrameSize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanvasMode {
    #[default]
    SquareBottom,
    SquareCenter,
    AutoWidthCenter,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizeOptions {
    pub mode: CanvasMode,
    pub margin_bottom: u32,
    pub margin: u32,
    pub alpha_threshold: u8,
    #[serde(default)]
    pub manual_anchor: Option<FootAnchor>,
}

impl Default for NormalizeOptions {
    fn default() -> Self {
        Self {
            mode: CanvasMode::SquareBottom,
            margin_bottom: 0,
            margin: 0,
            alpha_threshold: 0,
            manual_anchor: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizeWarning {
    EmptyForeground,
    CroppedByCanvas,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedFrame {
    pub image: RgbaImage,
    pub bbox: FrameBbox,
    pub size: FrameSize,
    pub anchor: FootAnchor,
    pub source_bbox: FrameBbox,
    pub offset_x: i32,
    pub offset_y: i32,
    pub warnings: Vec<NormalizeWarning>,
}

pub fn normalize_frames(frames: &[RgbaImage], options: NormalizeOptions) -> Vec<NormalizedFrame> {
    if frames.is_empty() {
        return Vec::new();
    }

    let source_bboxes: Vec<FrameBbox> = frames
        .iter()
        .map(|frame| bbox_from_image(frame, options.alpha_threshold))
        .collect();
    let size = canvas_size(frames, &source_bboxes, options);
    let anchor = options
        .manual_anchor
        .unwrap_or_else(|| default_foot_anchor(size.width, size.height, options.margin_bottom));

    frames
        .iter()
        .zip(source_bboxes)
        .map(|(frame, source_bbox)| normalize_one(frame, source_bbox, size, anchor, options))
        .collect()
}

fn canvas_size(frames: &[RgbaImage], bboxes: &[FrameBbox], options: NormalizeOptions) -> FrameSize {
    let max_source_width = frames.iter().map(RgbaImage::width).max().unwrap_or(1);
    let max_source_height = frames.iter().map(RgbaImage::height).max().unwrap_or(1);
    let max_bbox_width = bboxes
        .iter()
        .map(|bbox| bbox.width.ceil() as u32)
        .max()
        .unwrap_or(1);
    let max_bbox_height = bboxes
        .iter()
        .map(|bbox| bbox.height.ceil() as u32)
        .max()
        .unwrap_or(1);
    let margin = options.margin.saturating_mul(2);

    match options.mode {
        CanvasMode::SquareBottom | CanvasMode::SquareCenter => {
            let needed_source_width = max_source_width.saturating_add(margin);
            let needed_source_height = match options.mode {
                CanvasMode::SquareBottom => max_source_height.saturating_add(options.margin_bottom),
                CanvasMode::SquareCenter => max_source_height.saturating_add(margin),
                CanvasMode::AutoWidthCenter => max_source_height,
            };
            let side = max_source_width
                .max(max_source_height)
                .max(needed_source_width)
                .max(needed_source_height)
                .max(max_bbox_width.saturating_add(margin))
                .max(max_bbox_height.saturating_add(options.margin_bottom));
            FrameSize::new(side.max(1), side.max(1))
        }
        CanvasMode::AutoWidthCenter => FrameSize::new(
            max_source_width
                .max(max_bbox_width.saturating_add(margin))
                .max(1),
            max_source_height
                .max(max_bbox_height.saturating_add(margin))
                .max(1),
        ),
    }
}

fn normalize_one(
    source: &RgbaImage,
    source_bbox: FrameBbox,
    size: FrameSize,
    anchor: FootAnchor,
    options: NormalizeOptions,
) -> NormalizedFrame {
    let mut image = RgbaImage::from_pixel(size.width, size.height, Rgba([0, 0, 0, 0]));
    let mut warnings = Vec::new();

    if !source_bbox.has_foreground() {
        warnings.push(NormalizeWarning::EmptyForeground);
    }

    let (target_center_x, target_center_y, target_bottom_y) = match options.mode {
        CanvasMode::SquareBottom => (anchor.x, 0.0, anchor.y),
        CanvasMode::SquareCenter | CanvasMode::AutoWidthCenter => (
            size.width as f32 / 2.0,
            size.height as f32 / 2.0,
            size.height as f32,
        ),
    };

    let offset_x = if source_bbox.has_foreground() {
        (target_center_x - source_bbox.center_x).round() as i32
    } else {
        ((size.width as i32 - source.width() as i32) / 2) as i32
    };
    let offset_y = if source_bbox.has_foreground() {
        match options.mode {
            CanvasMode::SquareBottom => (target_bottom_y - source_bbox.bottom_y).round() as i32,
            CanvasMode::SquareCenter | CanvasMode::AutoWidthCenter => {
                (target_center_y - source_bbox.center_y).round() as i32
            }
        }
    } else {
        ((size.height as i32 - source.height() as i32) / 2) as i32
    };

    for (x, y, pixel) in source.enumerate_pixels() {
        let tx = x as i32 + offset_x;
        let ty = y as i32 + offset_y;
        if tx >= 0 && ty >= 0 && tx < size.width as i32 && ty < size.height as i32 {
            image.put_pixel(tx as u32, ty as u32, *pixel);
        } else if pixel.0[3] > options.alpha_threshold {
            warnings.push(NormalizeWarning::CroppedByCanvas);
        }
    }

    let bbox = if source_bbox.has_foreground() {
        source_bbox.translated(offset_x as f32, offset_y as f32)
    } else {
        FrameBbox::empty()
    };

    NormalizedFrame {
        image,
        bbox,
        size,
        anchor,
        source_bbox,
        offset_x,
        offset_y,
        warnings,
    }
}
