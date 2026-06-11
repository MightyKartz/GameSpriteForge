use image::{GenericImage, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use super::sheet_layout::{
    plan_sprite_sheet_layout, SpriteSheetLayoutError, SpriteSheetLayoutParams,
};
use super::ExportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteSheetParameters {
    pub columns: u32,
    pub padding_px: u32,
    pub margin_px: u32,
    pub max_texture_size: u32,
    #[serde(default)]
    pub allow_multi_sheet: bool,
}

impl Default for SpriteSheetParameters {
    fn default() -> Self {
        Self {
            columns: 4,
            padding_px: 0,
            margin_px: 0,
            max_texture_size: 4096,
            allow_multi_sheet: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Atlas {
    pub image: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
    pub frames: Vec<AtlasFrame>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AtlasFrame {
    pub index: usize,
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub page: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpriteSheetOutput {
    pub sprite_sheet_path: PathBuf,
    pub sprite_sheet_paths: Vec<PathBuf>,
    pub atlas_path: PathBuf,
    pub atlas: Atlas,
}

pub fn build_sprite_sheet(
    frame_paths: &[PathBuf],
    output_dir: &Path,
    params: SpriteSheetParameters,
) -> Result<SpriteSheetOutput, ExportError> {
    if frame_paths.is_empty() {
        return Err(ExportError::NoFrames);
    }

    fs::create_dir_all(output_dir)?;
    let frames = load_frames(frame_paths)?;
    let frame_width = frames[0].width();
    let frame_height = frames[0].height();

    for frame in &frames {
        if frame.width() != frame_width || frame.height() != frame_height {
            return Err(ExportError::FrameSizeMismatch);
        }
    }

    let layout = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
        frame_count: frames.len(),
        frame_width,
        frame_height,
        requested_columns: params.columns,
        padding_px: params.padding_px,
        margin_px: params.margin_px,
        max_texture_size: params.max_texture_size,
        allow_multi_sheet: params.allow_multi_sheet,
    })
    .map_err(map_layout_error)?;

    let multi_page = layout.pages.len() > 1;
    let images = if multi_page {
        layout
            .pages
            .iter()
            .map(|page| page.image_name.clone())
            .collect()
    } else {
        Vec::new()
    };
    let mut sprite_sheet_paths = Vec::with_capacity(layout.pages.len());
    let mut atlas_frames = Vec::with_capacity(frames.len());

    for page in &layout.pages {
        let mut sheet = RgbaImage::new(page.image_width, page.image_height);
        for placement in &page.frames {
            let frame = &frames[placement.frame_index];
            let x = page.image_x_for(placement);
            let y = page.image_y_for(placement);
            sheet.copy_from(frame, x, y)?;
            atlas_frames.push(AtlasFrame {
                index: placement.frame_index,
                name: placement.name.clone(),
                x,
                y,
                width: placement.width,
                height: placement.height,
                page: placement.page_index,
                image: if multi_page && placement.page_index > 0 {
                    Some(page.image_name.clone())
                } else {
                    None
                },
            });
        }

        let sprite_sheet_path = output_dir.join(&page.image_name);
        sheet.save(&sprite_sheet_path)?;
        sprite_sheet_paths.push(sprite_sheet_path);
    }

    let sprite_sheet_path = sprite_sheet_paths
        .first()
        .cloned()
        .ok_or(ExportError::NoFrames)?;
    let atlas_path = output_dir.join("atlas.json");

    let atlas = Atlas {
        image: "sprite_sheet.png".to_string(),
        images,
        frame_width,
        frame_height,
        columns: layout.page_columns,
        rows: layout.page_rows,
        frames: atlas_frames,
    };
    fs::write(&atlas_path, serde_json::to_vec_pretty(&atlas)?)?;

    Ok(SpriteSheetOutput {
        sprite_sheet_path,
        sprite_sheet_paths,
        atlas_path,
        atlas,
    })
}

fn load_frames(frame_paths: &[PathBuf]) -> Result<Vec<RgbaImage>, ExportError> {
    frame_paths
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect()
}

fn map_layout_error(error: SpriteSheetLayoutError) -> ExportError {
    match error {
        SpriteSheetLayoutError::NoFrames => ExportError::NoFrames,
        SpriteSheetLayoutError::InvalidColumns => {
            ExportError::InvalidParameter("columns must be greater than zero".to_string())
        }
        SpriteSheetLayoutError::InvalidFrameDimensions { width, height } => {
            ExportError::InvalidParameter(format!(
                "frame dimensions must be greater than zero: {width}x{height}"
            ))
        }
        SpriteSheetLayoutError::FrameExceedsMaxTexture {
            width,
            height,
            max_texture_size,
        }
        | SpriteSheetLayoutError::SheetExceedsMaxTexture {
            width,
            height,
            max_texture_size,
        } => ExportError::TextureTooLarge {
            width,
            height,
            max_texture_size,
        },
    }
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}
