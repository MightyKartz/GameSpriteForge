use gif::{Encoder, Frame, Repeat};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

use super::ExportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GifBackground {
    Transparent,
    Checkerboard,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewGifParameters {
    pub fps: f32,
    pub loop_animation: bool,
    pub background: GifBackground,
}

impl Default for PreviewGifParameters {
    fn default() -> Self {
        Self {
            fps: 12.0,
            loop_animation: true,
            background: GifBackground::Transparent,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewGifOutput {
    pub preview_gif_path: PathBuf,
}

pub fn build_preview_gif(
    frame_paths: &[PathBuf],
    output_path: &Path,
    params: PreviewGifParameters,
) -> Result<PreviewGifOutput, ExportError> {
    if frame_paths.is_empty() {
        return Err(ExportError::NoFrames);
    }
    if params.fps <= 0.0 {
        return Err(ExportError::InvalidParameter(
            "fps must be greater than zero".to_string(),
        ));
    }

    let first = image::open(&frame_paths[0])?.to_rgba8();
    let width = u16::try_from(first.width()).map_err(|_| ExportError::GifTooLarge {
        width: first.width(),
        height: first.height(),
    })?;
    let height = u16::try_from(first.height()).map_err(|_| ExportError::GifTooLarge {
        width: first.width(),
        height: first.height(),
    })?;
    let delay = (100.0 / params.fps).round().max(1.0) as u16;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(output_path)?;
    let mut encoder = Encoder::new(file, width, height, &[])?;
    if params.loop_animation {
        encoder.set_repeat(Repeat::Infinite)?;
    }

    write_gif_frame(&mut encoder, width, height, first, delay, params.background)?;
    for path in &frame_paths[1..] {
        let frame = image::open(path)?.to_rgba8();
        if frame.width() != width as u32 || frame.height() != height as u32 {
            return Err(ExportError::FrameSizeMismatch);
        }
        write_gif_frame(&mut encoder, width, height, frame, delay, params.background)?;
    }

    Ok(PreviewGifOutput {
        preview_gif_path: output_path.to_path_buf(),
    })
}

fn write_gif_frame<W: std::io::Write>(
    encoder: &mut Encoder<W>,
    width: u16,
    height: u16,
    frame: RgbaImage,
    delay: u16,
    background: GifBackground,
) -> Result<(), ExportError> {
    let mut rgba = match background {
        GifBackground::Transparent => frame.into_raw(),
        GifBackground::Checkerboard => checkerboard_composite(frame).into_raw(),
    };
    let mut gif_frame = Frame::from_rgba_speed(width, height, rgba.as_mut_slice(), 10);
    gif_frame.delay = delay;
    encoder.write_frame(&gif_frame)?;
    Ok(())
}

fn checkerboard_composite(frame: RgbaImage) -> RgbaImage {
    let mut output = RgbaImage::new(frame.width(), frame.height());
    for (x, y, pixel) in frame.enumerate_pixels() {
        let tile_is_light = ((x / 8) + (y / 8)) % 2 == 0;
        let bg = if tile_is_light {
            [220, 224, 229, 255]
        } else {
            [172, 180, 191, 255]
        };
        output.put_pixel(x, y, alpha_blend(*pixel, Rgba(bg)));
    }
    output
}

fn alpha_blend(foreground: Rgba<u8>, background: Rgba<u8>) -> Rgba<u8> {
    let alpha = foreground[3] as f32 / 255.0;
    let inverse = 1.0 - alpha;
    Rgba([
        (foreground[0] as f32 * alpha + background[0] as f32 * inverse).round() as u8,
        (foreground[1] as f32 * alpha + background[1] as f32 * inverse).round() as u8,
        (foreground[2] as f32 * alpha + background[2] as f32 * inverse).round() as u8,
        255,
    ])
}
