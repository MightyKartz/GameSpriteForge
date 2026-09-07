use gif::{Encoder, Frame, Repeat};
use image::{ImageEncoder, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};

use super::ExportError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GifBackground {
    Transparent,
    Checkerboard,
    ChromaGreen,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewGifParameters {
    pub fps: f32,
    pub loop_animation: bool,
    pub background: GifBackground,
    #[serde(default = "default_preview_scale")]
    pub scale: u8,
}

fn default_preview_scale() -> u8 {
    1
}

impl Default for PreviewGifParameters {
    fn default() -> Self {
        Self {
            fps: 12.0,
            loop_animation: true,
            background: GifBackground::Transparent,
            scale: 1,
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
    build_preview_gif_with_durations(frame_paths, output_path, params, None)
}

pub fn build_preview_gif_with_durations(
    frame_paths: &[PathBuf],
    output_path: &Path,
    params: PreviewGifParameters,
    frame_durations_ms: Option<&[u64]>,
) -> Result<PreviewGifOutput, ExportError> {
    if frame_paths.is_empty() {
        return Err(ExportError::NoFrames);
    }
    if params.fps <= 0.0 {
        return Err(ExportError::InvalidParameter(
            "fps must be greater than zero".to_string(),
        ));
    }
    if !(1..=4).contains(&params.scale) {
        return Err(ExportError::InvalidParameter(
            "preview scale must be between 1 and 4".to_string(),
        ));
    }
    if let Some(durations) = frame_durations_ms {
        if durations.len() != frame_paths.len() || durations.contains(&0) {
            return Err(ExportError::InvalidParameter(
                "frame durations must contain one positive value per frame".to_string(),
            ));
        }
    }

    let source_first = image::open(&frame_paths[0])?.to_rgba8();
    let first = scale_frame(source_first, params.scale);
    let width = u16::try_from(first.width()).map_err(|_| ExportError::GifTooLarge {
        width: first.width(),
        height: first.height(),
    })?;
    let height = u16::try_from(first.height()).map_err(|_| ExportError::GifTooLarge {
        width: first.width(),
        height: first.height(),
    })?;
    let default_delay = (100.0 / params.fps).round().max(1.0) as u16;
    let frame_delays = gif_delays(frame_paths.len(), frame_durations_ms, default_delay);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(output_path)?;
    let mut encoder = Encoder::new(file, width, height, &[])?;
    if params.loop_animation {
        encoder.set_repeat(Repeat::Infinite)?;
    }

    write_gif_frame(
        &mut encoder,
        width,
        height,
        first,
        frame_delays[0],
        params.background,
    )?;
    for (index, path) in frame_paths[1..].iter().enumerate() {
        let frame = scale_frame(image::open(path)?.to_rgba8(), params.scale);
        if frame.width() != width as u32 || frame.height() != height as u32 {
            return Err(ExportError::FrameSizeMismatch);
        }
        write_gif_frame(
            &mut encoder,
            width,
            height,
            frame,
            frame_delays[index + 1],
            params.background,
        )?;
    }

    Ok(PreviewGifOutput {
        preview_gif_path: output_path.to_path_buf(),
    })
}

#[cfg(test)]
fn gif_delay(duration_ms: Option<u64>, default_delay: u16) -> u16 {
    duration_ms
        .map(|duration| ((duration as f64 / 10.0).round() as u64).clamp(1, u16::MAX as u64) as u16)
        .unwrap_or(default_delay)
}

fn gif_delays(
    frame_count: usize,
    frame_durations_ms: Option<&[u64]>,
    default_delay: u16,
) -> Vec<u16> {
    match frame_durations_ms {
        Some(durations) => {
            // GIF stores delays in centiseconds. Quantize cumulative source
            // time instead of rounding each frame independently so irregular
            // native PTS durations do not accumulate visible speed drift.
            let mut source_cumulative_ms = 0_u64;
            let mut encoded_cumulative_cs = 0_u64;
            durations
                .iter()
                .map(|duration| {
                    source_cumulative_ms = source_cumulative_ms.saturating_add(*duration);
                    let target_cumulative_cs = ((source_cumulative_ms as f64 / 10.0).round()
                        as u64)
                        .clamp(1, u16::MAX as u64 * frame_count as u64);
                    let delay = target_cumulative_cs
                        .saturating_sub(encoded_cumulative_cs)
                        .clamp(1, u16::MAX as u64);
                    encoded_cumulative_cs = encoded_cumulative_cs.saturating_add(delay);
                    delay as u16
                })
                .collect()
        }
        None => vec![default_delay; frame_count],
    }
}

pub fn build_preview_contact_sheet(
    frame_paths: &[PathBuf],
    output_path: &Path,
    background: GifBackground,
    scale: u8,
) -> Result<(), ExportError> {
    if frame_paths.is_empty() {
        return Err(ExportError::NoFrames);
    }
    if !(1..=4).contains(&scale) {
        return Err(ExportError::InvalidParameter(
            "preview scale must be between 1 and 4".to_string(),
        ));
    }
    let first = scale_frame(image::open(&frame_paths[0])?.to_rgba8(), scale);
    let columns = frame_paths.len().min(4) as u32;
    let rows = (frame_paths.len() as u32).div_ceil(columns);
    let mut sheet = RgbaImage::new(first.width() * columns, first.height() * rows);
    for (index, path) in frame_paths.iter().enumerate() {
        let frame = scale_frame(image::open(path)?.to_rgba8(), scale);
        if frame.dimensions() != first.dimensions() {
            return Err(ExportError::FrameSizeMismatch);
        }
        let composited = composite_frame(frame, background);
        image::imageops::overlay(
            &mut sheet,
            &composited,
            ((index as u32 % columns) * first.width()).into(),
            ((index as u32 / columns) * first.height()).into(),
        );
    }
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(output_path)?;
    image::codecs::png::PngEncoder::new_with_quality(
        file,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    )
    .write_image(
        sheet.as_raw(),
        sheet.width(),
        sheet.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(())
}

fn scale_frame(frame: RgbaImage, scale: u8) -> RgbaImage {
    if scale == 1 {
        frame
    } else {
        image::imageops::resize(
            &frame,
            frame.width() * scale as u32,
            frame.height() * scale as u32,
            image::imageops::FilterType::Nearest,
        )
    }
}

fn write_gif_frame<W: std::io::Write>(
    encoder: &mut Encoder<W>,
    width: u16,
    height: u16,
    frame: RgbaImage,
    delay: u16,
    background: GifBackground,
) -> Result<(), ExportError> {
    let mut rgba = composite_frame(frame, background).into_raw();
    // Preview GIFs are diagnostic artifacts, not archival color masters.
    // Fast quantization keeps 2x/4x runtime checks practical in CI while the
    // source PNG hashes remain the authoritative pixels.
    let mut gif_frame = Frame::from_rgba_speed(width, height, rgba.as_mut_slice(), 30);
    gif_frame.delay = delay;
    encoder.write_frame(&gif_frame)?;
    Ok(())
}

fn composite_frame(frame: RgbaImage, background: GifBackground) -> RgbaImage {
    match background {
        GifBackground::Transparent => frame,
        GifBackground::Checkerboard => checkerboard_composite(frame),
        GifBackground::ChromaGreen => solid_composite(frame, Rgba([0, 255, 0, 255])),
        GifBackground::Dark => solid_composite(frame, Rgba([30, 33, 39, 255])),
    }
}

fn solid_composite(frame: RgbaImage, background: Rgba<u8>) -> RgbaImage {
    let mut output = RgbaImage::new(frame.width(), frame.height());
    for (x, y, pixel) in frame.enumerate_pixels() {
        output.put_pixel(x, y, alpha_blend(*pixel, background));
    }
    output
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

#[cfg(test)]
mod tests {
    use super::*;

    fn write_test_frames(dir: &Path, count: usize) -> Vec<PathBuf> {
        (0..count)
            .map(|index| {
                let path = dir.join(format!("frame-{index}.png"));
                RgbaImage::from_pixel(2, 2, Rgba([(index * 17) as u8, 64, 192, 255]))
                    .save(&path)
                    .unwrap();
                path
            })
            .collect()
    }

    fn decoded_delays_ms(path: &Path) -> Vec<u64> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(File::open(path).unwrap()).unwrap();
        let mut delays = Vec::new();
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            delays.push(u64::from(frame.delay) * 10);
        }
        delays
    }

    #[test]
    fn gif_delay_uses_explicit_frame_duration_before_fps_default() {
        assert_eq!(gif_delay(Some(125), 8), 13);
        assert_eq!(gif_delay(Some(80), 8), 8);
        assert_eq!(gif_delay(None, 8), 8);
    }

    #[test]
    fn preview_gif_preserves_explicit_uniform_playback_durations() {
        for total_duration_ms in [800_u64, 1_600] {
            let temp = tempfile::tempdir().unwrap();
            let frame_paths = write_test_frames(temp.path(), 8);
            let frame_duration_ms = total_duration_ms / frame_paths.len() as u64;
            let frame_durations_ms = vec![frame_duration_ms; frame_paths.len()];
            let output_path = temp.path().join("preview.gif");

            build_preview_gif_with_durations(
                &frame_paths,
                &output_path,
                PreviewGifParameters {
                    // Deliberately conflicts with the explicit timing so this
                    // test proves frame durations take precedence over FPS.
                    fps: 60.0,
                    loop_animation: true,
                    background: GifBackground::Transparent,
                    scale: 1,
                },
                Some(&frame_durations_ms),
            )
            .unwrap();

            let decoded = decoded_delays_ms(&output_path);
            assert_eq!(decoded, frame_durations_ms);
            let decoded_total_ms = decoded.iter().sum::<u64>();
            assert!(decoded_total_ms.abs_diff(total_duration_ms) <= 10);
        }
    }

    #[test]
    fn preview_gif_uses_each_explicit_frame_duration() {
        let temp = tempfile::tempdir().unwrap();
        let frame_paths = write_test_frames(temp.path(), 4);
        let frame_durations_ms = [80, 120, 160, 240];
        let output_path = temp.path().join("preview.gif");

        build_preview_gif_with_durations(
            &frame_paths,
            &output_path,
            PreviewGifParameters::default(),
            Some(&frame_durations_ms),
        )
        .unwrap();

        assert_eq!(decoded_delays_ms(&output_path), frame_durations_ms);
    }

    #[test]
    fn preview_gif_preserves_irregular_native_pts_total_without_rounding_drift() {
        let temp = tempfile::tempdir().unwrap();
        let frame_paths = write_test_frames(temp.path(), 12);
        let frame_durations_ms = [42, 83, 84, 83, 83, 84, 83, 42, 41, 84, 166, 167];
        let output_path = temp.path().join("preview.gif");

        build_preview_gif_with_durations(
            &frame_paths,
            &output_path,
            PreviewGifParameters::default(),
            Some(&frame_durations_ms),
        )
        .unwrap();

        let decoded = decoded_delays_ms(&output_path);
        assert_eq!(decoded.len(), frame_durations_ms.len());
        assert!(
            decoded
                .iter()
                .sum::<u64>()
                .abs_diff(frame_durations_ms.iter().sum::<u64>())
                <= 5
        );
    }

    #[test]
    fn legacy_preview_gif_call_still_uses_fps() {
        let temp = tempfile::tempdir().unwrap();
        let frame_paths = write_test_frames(temp.path(), 3);
        let output_path = temp.path().join("preview.gif");

        build_preview_gif(
            &frame_paths,
            &output_path,
            PreviewGifParameters {
                fps: 10.0,
                ..PreviewGifParameters::default()
            },
        )
        .unwrap();

        assert_eq!(decoded_delays_ms(&output_path), vec![100, 100, 100]);
    }

    #[test]
    fn preview_gif_rejects_invalid_explicit_durations() {
        let temp = tempfile::tempdir().unwrap();
        let frame_paths = write_test_frames(temp.path(), 2);

        for durations in [vec![100], vec![100, 0]] {
            let error = build_preview_gif_with_durations(
                &frame_paths,
                &temp.path().join("preview.gif"),
                PreviewGifParameters::default(),
                Some(&durations),
            )
            .unwrap_err();
            assert!(matches!(error, ExportError::InvalidParameter(_)));
        }
    }
}
