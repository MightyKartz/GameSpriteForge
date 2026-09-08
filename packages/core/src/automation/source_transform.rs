//! Opt-in whole-sheet transforms before fixed-grid slicing. Never aligns frames.
use super::types::FixedGridSplit;
use image::RgbaImage;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_CANVAS_PIXELS: u64 = 64 * 1024 * 1024;

pub(super) fn enabled(grid: &FixedGridSplit) -> bool {
    grid.source_padding_right_px != 0
        || grid.source_padding_bottom_px != 0
        || grid.source_offset_x != 0
        || grid.source_offset_y != 0
}

fn transform(
    path: &Path,
    grid: &FixedGridSplit,
) -> Result<(Vec<u8>, RgbaImage, RgbaImage), String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    let source = image::load_from_memory(&bytes)
        .map_err(|error| error.to_string())?
        .to_rgba8();
    let width = source
        .width()
        .checked_add(grid.source_padding_right_px)
        .ok_or("source padding width overflows")?;
    let height = source
        .height()
        .checked_add(grid.source_padding_bottom_px)
        .ok_or("source padding height overflows")?;
    if width as u64 * height as u64 > MAX_CANVAS_PIXELS {
        return Err("preprocessed source canvas exceeds 64 million pixels".into());
    }
    if grid.frame_width.checked_mul(grid.columns) != Some(width)
        || grid.frame_height.checked_mul(grid.rows) != Some(height)
    {
        return Err("preprocessed source canvas must exactly match fixed_grid dimensions".into());
    }
    let mut derived = RgbaImage::new(width, height);
    for (x, y, pixel) in source.enumerate_pixels() {
        let target_x = x as i64 + grid.source_offset_x as i64;
        let target_y = y as i64 + grid.source_offset_y as i64;
        if target_x >= 0 && target_y >= 0 && target_x < width as i64 && target_y < height as i64 {
            derived.put_pixel(target_x as u32, target_y as u32, *pixel);
        } else if pixel[3] > 0 {
            return Err(format!(
                "source offset would discard a nontransparent pixel at ({x}, {y}); alpha={}",
                pixel[3]
            ));
        }
    }
    Ok((bytes, source, derived))
}

pub(super) fn validate(path: &Path, grid: &FixedGridSplit) -> Result<(), String> {
    if enabled(grid) {
        transform(path, grid)?;
    }
    Ok(())
}

pub(super) fn prepare(
    job_dir: &Path,
    path: &Path,
    grid: &FixedGridSplit,
) -> Result<PathBuf, String> {
    if !enabled(grid) {
        return Ok(path.to_path_buf());
    }
    let (bytes, source, derived) = transform(path, grid)?;
    let directory = job_dir.join("source-preprocessing");
    fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let original_path = directory.join("original.png");
    let derived_path = directory.join("derived.png");
    fs::write(&original_path, &bytes).map_err(|error| error.to_string())?;
    derived
        .save(&derived_path)
        .map_err(|error| error.to_string())?;
    let derived_bytes = fs::read(&derived_path).map_err(|error| error.to_string())?;
    let record = serde_json::json!({
        "schemaVersion":"1", "operation":"fixed_grid_source_transform",
        "sourcePath":path, "originalPath":original_path, "derivedPath":derived_path,
        "sourceSha256":format!("{:x}", Sha256::digest(&bytes)),
        "derivedSha256":format!("{:x}", Sha256::digest(&derived_bytes)),
        "sourceWidth":source.width(), "sourceHeight":source.height(),
        "derivedWidth":derived.width(), "derivedHeight":derived.height(),
        "sourcePaddingRightPx":grid.source_padding_right_px,
        "sourcePaddingBottomPx":grid.source_padding_bottom_px,
        "sourceOffsetX":grid.source_offset_x, "sourceOffsetY":grid.source_offset_y,
        "discardedNontransparentPixels":0,
        "coordinatePolicy":"whole_source_canvas_before_fixed_grid",
    });
    fs::write(
        directory.join("source-transform.json"),
        serde_json::to_vec_pretty(&record).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(derived_path)
}
