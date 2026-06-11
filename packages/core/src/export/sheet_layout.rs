#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteSheetLayoutParams {
    pub frame_count: usize,
    pub frame_width: u32,
    pub frame_height: u32,
    pub requested_columns: u32,
    pub padding_px: u32,
    pub margin_px: u32,
    pub max_texture_size: u32,
    pub allow_multi_sheet: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetLayout {
    pub frame_width: u32,
    pub frame_height: u32,
    pub page_columns: u32,
    pub page_rows: u32,
    pub frames_per_page: usize,
    pub pages: Vec<SpriteSheetPageLayout>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetPageLayout {
    pub page_index: usize,
    pub image_name: String,
    pub columns: u32,
    pub rows: u32,
    /// Content width excluding the outer page margin.
    pub width: u32,
    /// Content height excluding the outer page margin.
    pub height: u32,
    /// Full image width including the outer page margin on both sides.
    pub image_width: u32,
    /// Full image height including the outer page margin on both sides.
    pub image_height: u32,
    pub margin_px: u32,
    pub frames: Vec<SpriteSheetFramePlacement>,
}

impl SpriteSheetPageLayout {
    pub fn image_x_for(&self, frame: &SpriteSheetFramePlacement) -> u32 {
        self.margin_px.saturating_add(frame.x)
    }

    pub fn image_y_for(&self, frame: &SpriteSheetFramePlacement) -> u32 {
        self.margin_px.saturating_add(frame.y)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpriteSheetFramePlacement {
    pub frame_index: usize,
    pub page_index: usize,
    pub name: String,
    /// Content-space x coordinate, excluding the page margin.
    pub x: u32,
    /// Content-space y coordinate, excluding the page margin.
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SpriteSheetLayoutError {
    #[error("no frames were provided")]
    NoFrames,
    #[error("columns must be greater than zero")]
    InvalidColumns,
    #[error("frame dimensions must be greater than zero: {width}x{height}")]
    InvalidFrameDimensions { width: u32, height: u32 },
    #[error("frame {width}x{height} exceeds max texture size {max_texture_size}")]
    FrameExceedsMaxTexture {
        width: u32,
        height: u32,
        max_texture_size: u32,
    },
    #[error("sprite sheet {width}x{height} exceeds max texture size {max_texture_size}")]
    SheetExceedsMaxTexture {
        width: u32,
        height: u32,
        max_texture_size: u32,
    },
}

pub fn plan_sprite_sheet_layout(
    params: SpriteSheetLayoutParams,
) -> Result<SpriteSheetLayout, SpriteSheetLayoutError> {
    if params.frame_count == 0 {
        return Err(SpriteSheetLayoutError::NoFrames);
    }
    if params.requested_columns == 0 {
        return Err(SpriteSheetLayoutError::InvalidColumns);
    }
    if params.frame_width == 0 || params.frame_height == 0 {
        return Err(SpriteSheetLayoutError::InvalidFrameDimensions {
            width: params.frame_width,
            height: params.frame_height,
        });
    }
    if params.frame_width > params.max_texture_size || params.frame_height > params.max_texture_size
    {
        return Err(SpriteSheetLayoutError::FrameExceedsMaxTexture {
            width: params.frame_width,
            height: params.frame_height,
            max_texture_size: params.max_texture_size,
        });
    }

    let legacy_columns = min_u32_with_usize(params.requested_columns, params.frame_count);
    let legacy_rows = div_ceil_usize(params.frame_count, legacy_columns as usize);
    let legacy_width = image_dimension(
        params.frame_width,
        legacy_columns as usize,
        params.padding_px,
        params.margin_px,
    );
    let legacy_height = image_dimension(
        params.frame_height,
        legacy_rows,
        params.padding_px,
        params.margin_px,
    );
    let max_texture_size = u64::from(params.max_texture_size);
    let legacy_exceeds_limit = legacy_width > max_texture_size || legacy_height > max_texture_size;

    if !params.allow_multi_sheet && legacy_exceeds_limit {
        return Err(SpriteSheetLayoutError::SheetExceedsMaxTexture {
            width: saturating_u32(legacy_width),
            height: saturating_u32(legacy_height),
            max_texture_size: params.max_texture_size,
        });
    }

    if !legacy_exceeds_limit {
        return Ok(build_layout(
            params,
            legacy_columns,
            saturating_u32(legacy_rows as u64),
        ));
    }

    let max_columns = max_cells_that_fit(
        params.frame_width,
        params.padding_px,
        params.margin_px,
        params.max_texture_size,
    );
    let max_rows = max_cells_that_fit(
        params.frame_height,
        params.padding_px,
        params.margin_px,
        params.max_texture_size,
    );
    if max_columns == 0 || max_rows == 0 {
        return Err(SpriteSheetLayoutError::SheetExceedsMaxTexture {
            width: saturating_u32(image_dimension(
                params.frame_width,
                1,
                params.padding_px,
                params.margin_px,
            )),
            height: saturating_u32(image_dimension(
                params.frame_height,
                1,
                params.padding_px,
                params.margin_px,
            )),
            max_texture_size: params.max_texture_size,
        });
    }

    let page_columns = params
        .requested_columns
        .min(max_columns)
        .min(min_u32_with_usize(u32::MAX, params.frame_count))
        .max(1);
    Ok(build_layout(params, page_columns, max_rows))
}

fn build_layout(
    params: SpriteSheetLayoutParams,
    page_columns: u32,
    page_rows: u32,
) -> SpriteSheetLayout {
    let frames_per_page = (page_columns as usize).saturating_mul(page_rows as usize);
    let page_count = div_ceil_usize(params.frame_count, frames_per_page);
    let mut pages = Vec::with_capacity(page_count);

    for page_index in 0..page_count {
        let first_frame_index = page_index * frames_per_page;
        let page_frame_count = frames_per_page.min(params.frame_count - first_frame_index);
        let rows = div_ceil_usize(page_frame_count, page_columns as usize) as u32;
        let columns = page_columns;
        let mut frames = Vec::with_capacity(page_frame_count);

        for local_frame_index in 0..page_frame_count {
            let frame_index = first_frame_index + local_frame_index;
            let column = local_frame_index as u32 % page_columns;
            let row = local_frame_index as u32 / page_columns;
            frames.push(SpriteSheetFramePlacement {
                frame_index,
                page_index,
                name: format!("frame_{:03}.png", frame_index + 1),
                x: column.saturating_mul(params.frame_width.saturating_add(params.padding_px)),
                y: row.saturating_mul(params.frame_height.saturating_add(params.padding_px)),
                width: params.frame_width,
                height: params.frame_height,
            });
        }

        let width = content_dimension(params.frame_width, columns as usize, params.padding_px);
        let height = content_dimension(params.frame_height, rows as usize, params.padding_px);
        let image_width = image_dimension(
            params.frame_width,
            columns as usize,
            params.padding_px,
            params.margin_px,
        );
        let image_height = image_dimension(
            params.frame_height,
            rows as usize,
            params.padding_px,
            params.margin_px,
        );
        pages.push(SpriteSheetPageLayout {
            page_index,
            image_name: page_image_name(page_index),
            columns,
            rows,
            width: saturating_u32(width),
            height: saturating_u32(height),
            image_width: saturating_u32(image_width),
            image_height: saturating_u32(image_height),
            margin_px: params.margin_px,
            frames,
        });
    }

    SpriteSheetLayout {
        frame_width: params.frame_width,
        frame_height: params.frame_height,
        page_columns,
        page_rows,
        frames_per_page,
        pages,
    }
}

fn max_cells_that_fit(
    cell_size: u32,
    padding_px: u32,
    margin_px: u32,
    max_texture_size: u32,
) -> u32 {
    let cell_size = u64::from(cell_size);
    let padding_px = u64::from(padding_px);
    let margin_px = u64::from(margin_px);
    let max_texture_size = u64::from(max_texture_size);

    if cell_size == 0 {
        return 0;
    }

    let margin_size = margin_px.saturating_mul(2);
    if cell_size.saturating_add(margin_size) > max_texture_size {
        return 0;
    }

    let denominator = cell_size.saturating_add(padding_px);
    if denominator == 0 {
        return 0;
    }

    let available = max_texture_size - margin_size;
    let cells = available.saturating_add(padding_px) / denominator;
    saturating_u32(cells.max(1))
}

fn min_u32_with_usize(value: u32, count: usize) -> u32 {
    if count < value as usize {
        count as u32
    } else {
        value
    }
}

fn image_dimension(cell_size: u32, cells: usize, padding_px: u32, margin_px: u32) -> u64 {
    content_dimension(cell_size, cells, padding_px).saturating_add(u64::from(margin_px) * 2)
}

fn content_dimension(cell_size: u32, cells: usize, padding_px: u32) -> u64 {
    let cells = u64::try_from(cells).unwrap_or(u64::MAX);
    u64::from(cell_size).saturating_mul(cells).saturating_add(
        cells
            .saturating_sub(1)
            .saturating_mul(u64::from(padding_px)),
    )
}

fn saturating_u32(value: u64) -> u32 {
    value.min(u64::from(u32::MAX)) as u32
}

fn div_ceil_usize(value: usize, divisor: usize) -> usize {
    if divisor == 0 {
        return 0;
    }
    value.div_ceil(divisor)
}

fn page_image_name(page_index: usize) -> String {
    if page_index == 0 {
        "sprite_sheet.png".to_string()
    } else {
        format!("sprite_sheet_{:03}.png", page_index + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_small_exports_on_one_legacy_page() {
        let layout = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 6,
            frame_width: 64,
            frame_height: 64,
            requested_columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap();

        assert_eq!(layout.pages.len(), 1);
        assert_eq!(layout.page_columns, 4);
        assert_eq!(layout.page_rows, 2);
        assert_eq!(layout.pages[0].image_name, "sprite_sheet.png");
        assert_eq!(layout.pages[0].width, 262);
        assert_eq!(layout.pages[0].height, 130);
    }

    #[test]
    fn reports_content_and_full_image_dimensions_with_margin() {
        let layout = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 6,
            frame_width: 64,
            frame_height: 64,
            requested_columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap();

        let page = &layout.pages[0];
        assert_eq!(page.width, 262);
        assert_eq!(page.height, 130);
        assert_eq!(page.margin_px, 2);
        assert_eq!(page.image_width, 266);
        assert_eq!(page.image_height, 134);

        for frame in &page.frames {
            assert!(frame.x + frame.width <= page.width);
            assert!(frame.y + frame.height <= page.height);
            assert!(page.image_x_for(frame) + frame.width <= page.image_width);
            assert!(page.image_y_for(frame) + frame.height <= page.image_height);
        }
    }

    #[test]
    fn splits_high_resolution_exports_across_pages() {
        let layout = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 30,
            frame_width: 1304,
            frame_height: 696,
            requested_columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap();

        assert_eq!(layout.page_columns, 1);
        assert_eq!(layout.page_rows, 2);
        assert_eq!(layout.frames_per_page, 2);
        assert_eq!(layout.pages.len(), 15);
        assert_eq!(layout.pages[0].image_name, "sprite_sheet.png");
        assert_eq!(layout.pages[1].image_name, "sprite_sheet_002.png");
        assert!(layout
            .pages
            .iter()
            .all(|page| page.width <= 2048 && page.height <= 2048));
        assert!(layout
            .pages
            .iter()
            .all(|page| page.image_width <= 2048 && page.image_height <= 2048));
        assert_eq!(layout.pages[14].frames[1].frame_index, 29);
    }

    #[test]
    fn reports_actual_legacy_dimensions_when_multi_sheet_is_disabled() {
        let error = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 30,
            frame_width: 1304,
            frame_height: 696,
            requested_columns: 4,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: false,
        })
        .unwrap_err();

        assert_eq!(
            error,
            SpriteSheetLayoutError::SheetExceedsMaxTexture {
                width: 5226,
                height: 5586,
                max_texture_size: 2048,
            }
        );
    }

    #[test]
    fn rejects_a_single_frame_larger_than_the_limit() {
        let error = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 1,
            frame_width: 4096,
            frame_height: 256,
            requested_columns: 1,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap_err();

        assert_eq!(
            error,
            SpriteSheetLayoutError::FrameExceedsMaxTexture {
                width: 4096,
                height: 256,
                max_texture_size: 2048,
            }
        );
    }

    #[test]
    fn rejects_zero_frame_dimensions() {
        let error = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 1,
            frame_width: 0,
            frame_height: 256,
            requested_columns: 1,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 2048,
            allow_multi_sheet: true,
        })
        .unwrap_err();

        assert_eq!(
            error,
            SpriteSheetLayoutError::InvalidFrameDimensions {
                width: 0,
                height: 256,
            }
        );
    }

    #[test]
    fn handles_oversized_dimension_math_without_wrapping() {
        let error = plan_sprite_sheet_layout(SpriteSheetLayoutParams {
            frame_count: 2,
            frame_width: u32::MAX - 8,
            frame_height: 1,
            requested_columns: 2,
            padding_px: u32::MAX,
            margin_px: u32::MAX,
            max_texture_size: u32::MAX,
            allow_multi_sheet: false,
        })
        .unwrap_err();

        assert_eq!(
            error,
            SpriteSheetLayoutError::SheetExceedsMaxTexture {
                width: u32::MAX,
                height: u32::MAX,
                max_texture_size: u32::MAX,
            }
        );
    }
}
