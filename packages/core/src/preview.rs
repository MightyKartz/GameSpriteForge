use crate::matting::chroma::{
    alpha_bbox, apply_chroma_key, ChromaParameters, FrameBBox, MattingResult,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetCanvasMode {
    Original,
    SquareBottom,
    Fixed,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChromaPreviewResult {
    pub source_path: PathBuf,
    pub processed_path: PathBuf,
    pub preview_json_path: PathBuf,
    pub raw_width: u32,
    pub raw_height: u32,
    pub processed_width: u32,
    pub processed_height: u32,
    pub target_canvas_mode: TargetCanvasMode,
    pub bbox: Option<crate::matting::chroma::AlphaBBox>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewManifest {
    raw_frame_path: PathBuf,
    source_path: PathBuf,
    processed_path: PathBuf,
    raw_width: u32,
    raw_height: u32,
    processed_width: u32,
    processed_height: u32,
    target_canvas_mode: TargetCanvasMode,
    chroma: ChromaParameters,
    bbox: Option<crate::matting::chroma::AlphaBBox>,
}

pub fn preview_chroma_frame(
    raw_frame_path: impl AsRef<Path>,
    parameters: &ChromaParameters,
    target_canvas_mode: TargetCanvasMode,
    previews_dir: impl AsRef<Path>,
) -> MattingResult<ChromaPreviewResult> {
    let raw_frame_path = raw_frame_path.as_ref();
    let previews_dir = previews_dir.as_ref();
    fs::create_dir_all(previews_dir)?;

    let source_path = previews_dir.join("source.png");
    let processed_path = previews_dir.join("processed.png");
    let preview_json_path = previews_dir.join("preview.json");

    let raw = image::open(raw_frame_path)?.to_rgba8();
    raw.save(&source_path)?;

    let processed = apply_chroma_key(&raw, parameters)?;
    processed.save(&processed_path)?;

    let bbox = alpha_bbox(&processed);
    let manifest = PreviewManifest {
        raw_frame_path: raw_frame_path.to_path_buf(),
        source_path: source_path.clone(),
        processed_path: processed_path.clone(),
        raw_width: raw.width(),
        raw_height: raw.height(),
        processed_width: processed.width(),
        processed_height: processed.height(),
        target_canvas_mode: target_canvas_mode.clone(),
        chroma: parameters.clone(),
        bbox: bbox.clone(),
    };
    fs::write(&preview_json_path, serde_json::to_vec_pretty(&manifest)?)?;

    Ok(ChromaPreviewResult {
        source_path,
        processed_path,
        preview_json_path,
        raw_width: raw.width(),
        raw_height: raw.height(),
        processed_width: processed.width(),
        processed_height: processed.height(),
        target_canvas_mode,
        bbox,
    })
}

pub fn frame_bbox_for_processed_frame(frame_name: String, image: &image::RgbaImage) -> FrameBBox {
    FrameBBox {
        frame: frame_name,
        width: image.width(),
        height: image.height(),
        bbox: alpha_bbox(image),
    }
}
