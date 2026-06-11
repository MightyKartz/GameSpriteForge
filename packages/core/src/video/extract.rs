use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::ffmpeg::{resolve_binary, FFMPEG_MISSING_MESSAGE};
use super::VideoError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractFramesParams {
    pub input_path: PathBuf,
    pub start_time_seconds: f64,
    pub end_time_seconds: f64,
    pub keep_every_n_frames: u32,
    pub output_directory: PathBuf,
    pub configured_ffmpeg_path: Option<PathBuf>,
    pub bundled_resource_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractFramesResult {
    pub raw_directory: PathBuf,
    pub frames: Vec<PathBuf>,
}

pub fn extract_frames(params: &ExtractFramesParams) -> Result<ExtractFramesResult, VideoError> {
    validate_params(params)?;

    let ffmpeg_path = resolve_binary(
        "ffmpeg",
        params.configured_ffmpeg_path.as_deref(),
        params.bundled_resource_path.as_deref(),
    )
    .map_err(|_| VideoError::new("ffmpeg_missing", FFMPEG_MISSING_MESSAGE))?;

    let raw_directory = params.output_directory.join("raw");
    fs::create_dir_all(&raw_directory)?;
    remove_existing_png_frames(&raw_directory)?;

    let temp_pattern = raw_directory.join("frame_tmp_%08d.png");
    let select_filter = format!("select='not(mod(n\\,{}))'", params.keep_every_n_frames);
    let duration = params.end_time_seconds - params.start_time_seconds;
    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-ss")
        .arg(format_seconds(params.start_time_seconds))
        .arg("-t")
        .arg(format_seconds(duration))
        .arg("-i")
        .arg(&params.input_path)
        .arg("-vf")
        .arg(select_filter)
        .args(["-vsync", "0"])
        .arg(&temp_pattern)
        .output()?;

    if !output.status.success() {
        return Err(VideoError::command_failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let temp_frames = sorted_temp_frames(&raw_directory)?;
    let mut frames = Vec::with_capacity(temp_frames.len());
    for (index, temp_frame) in temp_frames.iter().enumerate() {
        let destination = raw_directory.join(format!("frame_{:05}.png", index + 1));
        fs::rename(temp_frame, &destination)?;
        frames.push(destination);
    }

    Ok(ExtractFramesResult {
        raw_directory,
        frames,
    })
}

fn validate_params(params: &ExtractFramesParams) -> Result<(), VideoError> {
    if params.keep_every_n_frames == 0 {
        return Err(VideoError::invalid_params(
            "keepEveryNFrames must be greater than 0",
        ));
    }

    if !params.start_time_seconds.is_finite()
        || !params.end_time_seconds.is_finite()
        || params.start_time_seconds < 0.0
        || params.end_time_seconds <= params.start_time_seconds
    {
        return Err(VideoError::invalid_params(
            "startTimeSeconds and endTimeSeconds must define a positive range",
        ));
    }

    Ok(())
}

pub(super) fn remove_existing_png_frames(raw_directory: &Path) -> Result<(), VideoError> {
    for entry in fs::read_dir(raw_directory)? {
        let path = entry?.path();
        let is_png = path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.eq_ignore_ascii_case("png"))
            .unwrap_or(false);
        if is_png {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn sorted_temp_frames(raw_directory: &PathBuf) -> Result<Vec<PathBuf>, VideoError> {
    let mut frames = Vec::new();
    for entry in fs::read_dir(raw_directory)? {
        let path = entry?.path();
        let is_temp_frame = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.starts_with("frame_tmp_") && name.ends_with(".png"))
            .unwrap_or(false);
        if is_temp_frame {
            frames.push(path);
        }
    }
    frames.sort();
    Ok(frames)
}

fn format_seconds(seconds: f64) -> String {
    format!("{seconds:.6}")
}
