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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleVideoFramesParams {
    pub input_path: PathBuf,
    pub start_time_ms: u64,
    pub end_time_ms: Option<u64>,
    pub target_frame_count: u32,
    pub output_directory: PathBuf,
    pub configured_ffmpeg_path: Option<PathBuf>,
    pub bundled_resource_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractCandidateFramesParams {
    pub input_path: PathBuf,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub maximum_fps: f32,
    pub maximum_frame_count: u32,
    pub output_directory: PathBuf,
    pub configured_ffmpeg_path: Option<PathBuf>,
    pub bundled_resource_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractCandidateFramesResult {
    pub raw_directory: PathBuf,
    pub frames: Vec<PathBuf>,
    pub sample_fps: f32,
    pub duration_ms: u64,
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

pub fn extract_sampled_frames(
    params: &SampleVideoFramesParams,
) -> Result<ExtractFramesResult, VideoError> {
    if !(2..=24).contains(&params.target_frame_count) {
        return Err(VideoError::invalid_params(
            "targetFrameCount must be between 2 and 24",
        ));
    }
    let end_time_ms = params.end_time_ms.ok_or_else(|| {
        VideoError::invalid_params("endTimeMs must be resolved before video extraction")
    })?;
    if end_time_ms <= params.start_time_ms {
        return Err(VideoError::invalid_params(
            "endTimeMs must be greater than startTimeMs",
        ));
    }
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
    let duration_seconds = (end_time_ms - params.start_time_ms) as f64 / 1000.0;
    let sample_fps = params.target_frame_count as f64 / duration_seconds;
    let frame_count = params.target_frame_count.to_string();
    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-ss")
        .arg(format_seconds(params.start_time_ms as f64 / 1000.0))
        .arg("-t")
        .arg(format_seconds(duration_seconds))
        .arg("-i")
        .arg(&params.input_path)
        .arg("-vf")
        .arg(format!("fps={sample_fps:.8}"))
        .args(["-frames:v", frame_count.as_str()])
        .args(["-vsync", "0"])
        .arg(&temp_pattern)
        .output()?;
    if !output.status.success() {
        return Err(VideoError::command_failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let temp_frames = sorted_temp_frames(&raw_directory)?;
    if temp_frames.len() != params.target_frame_count as usize {
        return Err(VideoError::invalid_output(format!(
            "expected {} sampled frames, ffmpeg produced {}",
            params.target_frame_count,
            temp_frames.len()
        )));
    }
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

pub fn extract_candidate_frames(
    params: &ExtractCandidateFramesParams,
) -> Result<ExtractCandidateFramesResult, VideoError> {
    if params.end_time_ms <= params.start_time_ms {
        return Err(VideoError::invalid_params(
            "endTimeMs must be greater than startTimeMs",
        ));
    }
    if !params.maximum_fps.is_finite()
        || params.maximum_fps <= 0.0
        || params.maximum_fps > 60.0
        || !(3..=240).contains(&params.maximum_frame_count)
    {
        return Err(VideoError::invalid_params(
            "maximumFps must be in (0, 60] and maximumFrameCount in 3..=240",
        ));
    }
    let ffmpeg_path = resolve_binary(
        "ffmpeg",
        params.configured_ffmpeg_path.as_deref(),
        params.bundled_resource_path.as_deref(),
    )
    .map_err(|_| VideoError::new("ffmpeg_missing", FFMPEG_MISSING_MESSAGE))?;
    let duration_ms = params.end_time_ms - params.start_time_ms;
    let duration_seconds = duration_ms as f64 / 1000.0;
    let sample_fps =
        (params.maximum_frame_count as f64 / duration_seconds).min(params.maximum_fps as f64);
    let expected_frames = (duration_seconds * sample_fps)
        .floor()
        .max(1.0)
        .min(params.maximum_frame_count as f64) as u32;
    if expected_frames < 3 {
        return Err(VideoError::invalid_output(
            "video is too short to produce three loop candidate frames",
        ));
    }
    let raw_directory = params.output_directory.join("raw");
    fs::create_dir_all(&raw_directory)?;
    remove_existing_png_frames(&raw_directory)?;
    let temp_pattern = raw_directory.join("frame_tmp_%08d.png");
    let frame_count = expected_frames.to_string();
    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-ss")
        .arg(format_seconds(params.start_time_ms as f64 / 1000.0))
        .arg("-t")
        .arg(format_seconds(duration_seconds))
        .arg("-i")
        .arg(&params.input_path)
        .arg("-vf")
        .arg(format!("fps={sample_fps:.8}"))
        .args(["-frames:v", frame_count.as_str()])
        .args(["-vsync", "0"])
        .arg(&temp_pattern)
        .output()?;
    if !output.status.success() {
        return Err(VideoError::command_failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let temp_frames = sorted_temp_frames(&raw_directory)?;
    if temp_frames.len() < 3 || temp_frames.len() > params.maximum_frame_count as usize {
        return Err(VideoError::invalid_output(format!(
            "expected 3..={} candidate frames, ffmpeg produced {}",
            params.maximum_frame_count,
            temp_frames.len()
        )));
    }
    let mut frames = Vec::with_capacity(temp_frames.len());
    for (index, temp_frame) in temp_frames.iter().enumerate() {
        let destination = raw_directory.join(format!("frame_{:05}.png", index + 1));
        fs::rename(temp_frame, &destination)?;
        frames.push(destination);
    }
    Ok(ExtractCandidateFramesResult {
        raw_directory,
        frames,
        sample_fps: sample_fps as f32,
        duration_ms,
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
