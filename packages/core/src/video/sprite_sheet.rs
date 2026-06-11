use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::extract::{remove_existing_png_frames, ExtractFramesResult};
use super::VideoError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceSpriteSheetParams {
    pub sheet_path: PathBuf,
    pub output_directory: PathBuf,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceSpriteSheetTransparentParams {
    pub sheet_path: PathBuf,
    pub output_directory: PathBuf,
    pub alpha_threshold: u8,
    pub min_gap_px: u32,
}

pub fn slice_sprite_sheet_grid(
    params: &SliceSpriteSheetParams,
) -> Result<ExtractFramesResult, VideoError> {
    if params.frame_width == 0
        || params.frame_height == 0
        || params.columns == 0
        || params.rows == 0
    {
        return Err(VideoError::invalid_params(
            "sprite sheet frame size and grid dimensions must be greater than zero",
        ));
    }

    let sheet = image::open(&params.sheet_path)
        .map_err(|error| VideoError::invalid_params(error.to_string()))?
        .to_rgba8();
    let required_width = params
        .frame_width
        .checked_mul(params.columns)
        .ok_or_else(|| VideoError::invalid_params("sprite sheet grid width is too large"))?;
    let required_height = params
        .frame_height
        .checked_mul(params.rows)
        .ok_or_else(|| VideoError::invalid_params("sprite sheet grid height is too large"))?;
    if required_width > sheet.width() || required_height > sheet.height() {
        return Err(VideoError::invalid_params(
            "sprite sheet grid exceeds source image bounds",
        ));
    }

    let raw_directory = params.output_directory.join("raw");
    fs::create_dir_all(&raw_directory)?;
    remove_existing_png_frames(&raw_directory)?;

    let mut frames = Vec::with_capacity((params.columns * params.rows) as usize);
    for row in 0..params.rows {
        for column in 0..params.columns {
            let index = frames.len() + 1;
            let frame = image::imageops::crop_imm(
                &sheet,
                column * params.frame_width,
                row * params.frame_height,
                params.frame_width,
                params.frame_height,
            )
            .to_image();
            let path = raw_directory.join(format!("frame_{index:05}.png"));
            frame
                .save(&path)
                .map_err(|error| VideoError::invalid_params(error.to_string()))?;
            frames.push(path);
        }
    }

    Ok(ExtractFramesResult {
        raw_directory,
        frames,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SpriteRegion {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

pub fn slice_sprite_sheet_transparent(
    params: &SliceSpriteSheetTransparentParams,
) -> Result<ExtractFramesResult, VideoError> {
    let sheet = image::open(&params.sheet_path)
        .map_err(|error| VideoError::invalid_params(error.to_string()))?
        .to_rgba8();
    let regions = transparent_regions(&sheet, params.alpha_threshold, params.min_gap_px.max(1));
    if regions.is_empty() {
        return Err(VideoError::invalid_params(
            "sprite sheet contains no opaque regions",
        ));
    }

    let frame_width = regions.iter().map(|region| region.width).max().unwrap_or(1);
    let frame_height = regions
        .iter()
        .map(|region| region.height)
        .max()
        .unwrap_or(1);
    let raw_directory = params.output_directory.join("raw");
    fs::create_dir_all(&raw_directory)?;
    remove_existing_png_frames(&raw_directory)?;

    let mut frames = Vec::with_capacity(regions.len());
    for (index, region) in regions.iter().enumerate() {
        let source =
            image::imageops::crop_imm(&sheet, region.x, region.y, region.width, region.height)
                .to_image();
        let mut frame = image::RgbaImage::new(frame_width, frame_height);
        let dx = (frame_width - region.width) / 2;
        let dy = frame_height - region.height;
        image::imageops::replace(&mut frame, &source, dx.into(), dy.into());
        let path = raw_directory.join(format!("frame_{:05}.png", index + 1));
        frame
            .save(&path)
            .map_err(|error| VideoError::invalid_params(error.to_string()))?;
        frames.push(path);
    }

    Ok(ExtractFramesResult {
        raw_directory,
        frames,
    })
}

fn transparent_regions(
    image: &image::RgbaImage,
    alpha_threshold: u8,
    min_gap_px: u32,
) -> Vec<SpriteRegion> {
    let row_gaps = transparent_row_runs(image, alpha_threshold, min_gap_px);
    let row_regions = content_regions_from_gaps(&row_gaps, image.height());
    let mut regions = Vec::new();

    for (y0, y1) in row_regions {
        let col_gaps = transparent_col_runs(image, y0, y1, alpha_threshold, min_gap_px);
        for (x0, x1) in content_regions_from_gaps(&col_gaps, image.width()) {
            if region_has_opaque_pixel(image, x0, y0, x1, y1, alpha_threshold) {
                regions.push(SpriteRegion {
                    x: x0,
                    y: y0,
                    width: x1 - x0,
                    height: y1 - y0,
                });
            }
        }
    }

    regions
}

fn transparent_row_runs(
    image: &image::RgbaImage,
    alpha_threshold: u8,
    min_gap_px: u32,
) -> Vec<(u32, u32)> {
    let mut rows = Vec::new();
    for y in 0..image.height() {
        let transparent = (0..image.width()).all(|x| image.get_pixel(x, y).0[3] <= alpha_threshold);
        if transparent {
            rows.push(y);
        }
    }
    runs_with_min_len(&rows, min_gap_px)
}

fn transparent_col_runs(
    image: &image::RgbaImage,
    y0: u32,
    y1: u32,
    alpha_threshold: u8,
    min_gap_px: u32,
) -> Vec<(u32, u32)> {
    let mut cols = Vec::new();
    for x in 0..image.width() {
        let transparent = (y0..y1).all(|y| image.get_pixel(x, y).0[3] <= alpha_threshold);
        if transparent {
            cols.push(x);
        }
    }
    runs_with_min_len(&cols, min_gap_px)
}

fn runs_with_min_len(values: &[u32], min_gap_px: u32) -> Vec<(u32, u32)> {
    if values.is_empty() {
        return Vec::new();
    }

    let mut runs = Vec::new();
    let mut start = values[0];
    let mut last = start;
    for value in values.iter().copied().skip(1) {
        if value == last + 1 {
            last = value;
        } else {
            if last - start + 1 >= min_gap_px {
                runs.push((start, last + 1));
            }
            start = value;
            last = value;
        }
    }
    if last - start + 1 >= min_gap_px {
        runs.push((start, last + 1));
    }

    runs
}

fn content_regions_from_gaps(gaps: &[(u32, u32)], total: u32) -> Vec<(u32, u32)> {
    let mut regions = Vec::new();
    let mut cursor = 0;
    for &(start, end) in gaps {
        if cursor < start {
            regions.push((cursor, start));
        }
        cursor = cursor.max(end);
    }
    if cursor < total {
        regions.push((cursor, total));
    }
    regions
}

fn region_has_opaque_pixel(
    image: &image::RgbaImage,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    alpha_threshold: u8,
) -> bool {
    (y0..y1).any(|y| (x0..x1).any(|x| image.get_pixel(x, y).0[3] > alpha_threshold))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    #[test]
    fn sprite_sheet_grid_slices_into_raw_frames() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("sheet.png");
        let output_dir = temp.path().join("job");
        write_quadrant_sheet(&sheet_path);

        let result = slice_sprite_sheet_grid(&SliceSpriteSheetParams {
            sheet_path,
            output_directory: output_dir,
            frame_width: 8,
            frame_height: 8,
            columns: 2,
            rows: 2,
        })
        .unwrap();

        assert_eq!(result.frames.len(), 4);
        assert_eq!(image::open(&result.frames[0]).unwrap().width(), 8);
        let expected_colors = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
        ];
        for (frame, expected) in result.frames.iter().zip(expected_colors) {
            assert_eq!(
                image::open(frame).unwrap().to_rgba8().get_pixel(0, 0).0,
                expected
            );
        }
    }

    #[test]
    fn sprite_sheet_grid_rejects_out_of_bounds_grid() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("sheet.png");
        RgbaImage::new(16, 16).save(&sheet_path).unwrap();

        let error = slice_sprite_sheet_grid(&SliceSpriteSheetParams {
            sheet_path,
            output_directory: temp.path().join("job"),
            frame_width: 8,
            frame_height: 8,
            columns: 3,
            rows: 2,
        })
        .unwrap_err();

        assert_eq!(
            error.message,
            "sprite sheet grid exceeds source image bounds"
        );
    }

    #[test]
    fn sprite_sheet_grid_rejects_zero_dimensions() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("sheet.png");
        RgbaImage::new(16, 16).save(&sheet_path).unwrap();

        let error = slice_sprite_sheet_grid(&SliceSpriteSheetParams {
            sheet_path,
            output_directory: temp.path().join("job"),
            frame_width: 0,
            frame_height: 8,
            columns: 2,
            rows: 2,
        })
        .unwrap_err();

        assert_eq!(
            error.message,
            "sprite sheet frame size and grid dimensions must be greater than zero"
        );
    }

    #[test]
    fn sprite_sheet_grid_removes_stale_pngs_before_slicing() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("sheet.png");
        let output_dir = temp.path().join("job");
        let raw_dir = output_dir.join("raw");
        fs::create_dir_all(&raw_dir).unwrap();
        fs::write(raw_dir.join("frame_99999.png"), b"stale").unwrap();
        fs::write(raw_dir.join("notes.txt"), b"keep").unwrap();
        write_quadrant_sheet(&sheet_path);

        let result = slice_sprite_sheet_grid(&SliceSpriteSheetParams {
            sheet_path,
            output_directory: output_dir,
            frame_width: 8,
            frame_height: 8,
            columns: 2,
            rows: 2,
        })
        .unwrap();

        assert_eq!(result.frames.len(), 4);
        assert!(!raw_dir.join("frame_99999.png").exists());
        assert!(raw_dir.join("notes.txt").exists());
    }

    #[test]
    fn sprite_sheet_transparent_splits_guttered_regions() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("transparent-sheet.png");
        let output_dir = temp.path().join("job");
        write_transparent_gutter_sheet(&sheet_path);

        let result = slice_sprite_sheet_transparent(&SliceSpriteSheetTransparentParams {
            sheet_path,
            output_directory: output_dir,
            alpha_threshold: 0,
            min_gap_px: 1,
        })
        .unwrap();

        assert_eq!(result.frames.len(), 4);
        let colors = [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
        ];
        for (frame, expected) in result.frames.iter().zip(colors) {
            let image = image::open(frame).unwrap().to_rgba8();
            assert_eq!(image.width(), 4);
            assert_eq!(image.height(), 4);
            assert_eq!(image.get_pixel(1, 1).0, expected);
        }
    }

    #[test]
    fn sprite_sheet_transparent_rejects_empty_sheet() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("empty.png");
        RgbaImage::new(8, 8).save(&sheet_path).unwrap();

        let error = slice_sprite_sheet_transparent(&SliceSpriteSheetTransparentParams {
            sheet_path,
            output_directory: temp.path().join("job"),
            alpha_threshold: 0,
            min_gap_px: 1,
        })
        .unwrap_err();

        assert_eq!(error.message, "sprite sheet contains no opaque regions");
    }

    #[test]
    fn sprite_sheet_transparent_removes_stale_pngs_before_slicing() {
        let temp = tempfile::tempdir().unwrap();
        let sheet_path = temp.path().join("transparent-sheet.png");
        let output_dir = temp.path().join("job");
        let raw_dir = output_dir.join("raw");
        fs::create_dir_all(&raw_dir).unwrap();
        fs::write(raw_dir.join("frame_99999.png"), b"stale").unwrap();
        write_transparent_gutter_sheet(&sheet_path);

        let result = slice_sprite_sheet_transparent(&SliceSpriteSheetTransparentParams {
            sheet_path,
            output_directory: output_dir,
            alpha_threshold: 0,
            min_gap_px: 1,
        })
        .unwrap();

        assert_eq!(result.frames.len(), 4);
        assert!(!raw_dir.join("frame_99999.png").exists());
    }

    fn write_quadrant_sheet(sheet_path: &std::path::Path) {
        let mut sheet = RgbaImage::new(16, 16);

        for y in 0..16 {
            for x in 0..16 {
                let pixel = match (x >= 8, y >= 8) {
                    (false, false) => Rgba([255, 0, 0, 255]),
                    (true, false) => Rgba([0, 255, 0, 255]),
                    (false, true) => Rgba([0, 0, 255, 255]),
                    (true, true) => Rgba([255, 255, 0, 255]),
                };
                sheet.put_pixel(x, y, pixel);
            }
        }
        sheet.save(sheet_path).unwrap();
    }

    fn write_transparent_gutter_sheet(sheet_path: &std::path::Path) {
        let mut sheet = RgbaImage::new(10, 10);
        paint_rect(&mut sheet, 0, 0, 4, 4, Rgba([255, 0, 0, 255]));
        paint_rect(&mut sheet, 6, 0, 4, 4, Rgba([0, 255, 0, 255]));
        paint_rect(&mut sheet, 0, 6, 4, 4, Rgba([0, 0, 255, 255]));
        paint_rect(&mut sheet, 6, 6, 4, 4, Rgba([255, 255, 0, 255]));
        sheet.save(sheet_path).unwrap();
    }

    fn paint_rect(
        image: &mut RgbaImage,
        x0: u32,
        y0: u32,
        width: u32,
        height: u32,
        color: Rgba<u8>,
    ) {
        for y in y0..y0 + height {
            for x in x0..x0 + width {
                image.put_pixel(x, y, color);
            }
        }
    }
}
