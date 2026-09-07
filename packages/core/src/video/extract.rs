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
    #[serde(default)]
    pub preserve_source_pts: bool,
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
    /// Original decoded presentation timestamps, in milliseconds on the
    /// source-video timeline. These are never synthesized from frame index.
    pub frame_timestamps_ms: Vec<u64>,
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
    if !params.preserve_source_pts {
        return extract_uniform_candidate_frames(params, &ffmpeg_path, duration_ms);
    }
    let ffprobe_path = resolve_binary("ffprobe", None, params.bundled_resource_path.as_deref())
        .map_err(|_| VideoError::new("ffmpeg_missing", FFMPEG_MISSING_MESSAGE))?;
    let source_timestamps = probe_frame_timestamps_ms(&ffprobe_path, &params.input_path)?;
    let selected = select_candidate_timestamps(
        &source_timestamps,
        params.start_time_ms,
        params.end_time_ms,
        params.maximum_fps,
        params.maximum_frame_count as usize,
    );
    if selected.len() < 3 {
        return Err(VideoError::invalid_output(
            "video is too short to produce three loop candidate frames",
        ));
    }
    let sample_fps = if selected.len() < 2 {
        0.0
    } else {
        (selected.len() - 1) as f64 * 1000.0
            / selected
                .last()
                .unwrap()
                .1
                .saturating_sub(selected[0].1)
                .max(1) as f64
    };
    let raw_directory = params.output_directory.join("raw");
    fs::create_dir_all(&raw_directory)?;
    remove_existing_png_frames(&raw_directory)?;
    let temp_pattern = raw_directory.join("frame_tmp_%08d.png");
    let frame_count = selected.len().to_string();
    let select_filter = selected
        .iter()
        .map(|(index, _)| format!("eq(n\\,{index})"))
        .collect::<Vec<_>>()
        .join("+");
    let output = Command::new(ffmpeg_path)
        .arg("-y")
        .arg("-i")
        .arg(&params.input_path)
        .arg("-vf")
        .arg(format!("select='{select_filter}'"))
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
    if temp_frames.len() != selected.len() {
        return Err(VideoError::invalid_output(format!(
            "expected {} PTS-selected candidate frames, ffmpeg produced {}",
            selected.len(),
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
        frame_timestamps_ms: selected
            .into_iter()
            .map(|(_, timestamp)| timestamp)
            .collect(),
    })
}

fn extract_uniform_candidate_frames(
    params: &ExtractCandidateFramesParams,
    ffmpeg_path: &Path,
    duration_ms: u64,
) -> Result<ExtractCandidateFramesResult, VideoError> {
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
    let frame_timestamps_ms = (0..frames.len())
        .map(|index| params.start_time_ms + (index as f64 * 1000.0 / sample_fps).round() as u64)
        .collect();
    Ok(ExtractCandidateFramesResult {
        raw_directory,
        frames,
        sample_fps: sample_fps as f32,
        duration_ms,
        frame_timestamps_ms,
    })
}

fn probe_frame_timestamps_ms(ffprobe: &Path, input: &Path) -> Result<Vec<u64>, VideoError> {
    let output = Command::new(ffprobe)
        .args(["-v", "error", "-select_streams", "v:0"])
        .args(["-show_entries", "frame=best_effort_timestamp_time"])
        .args(["-of", "csv=p=0"])
        .arg(input)
        .output()?;
    if !output.status.success() {
        return Err(VideoError::command_failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    parse_frame_timestamps_csv(&output.stdout)
}

fn parse_frame_timestamps_csv(bytes: &[u8]) -> Result<Vec<u64>, VideoError> {
    let mut timestamps = Vec::new();
    for line in String::from_utf8_lossy(bytes).lines() {
        let value = line.split(',').next().unwrap_or_default().trim();
        let seconds = value.parse::<f64>().map_err(|_| {
            VideoError::invalid_output(
                "ffprobe returned a missing or non-numeric decoded frame PTS",
            )
        })?;
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(VideoError::invalid_output(
                "ffprobe returned a negative or non-finite decoded frame PTS",
            ));
        }
        timestamps.push((seconds * 1000.0).round() as u64);
    }
    if timestamps.len() < 3 || timestamps.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(VideoError::invalid_output(
            "ffprobe did not return a strictly increasing decoded PTS timeline",
        ));
    }
    Ok(timestamps)
}

fn select_candidate_timestamps(
    timestamps: &[u64],
    start_ms: u64,
    end_ms: u64,
    maximum_fps: f32,
    maximum_count: usize,
) -> Vec<(usize, u64)> {
    let in_range = timestamps
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, timestamp)| *timestamp >= start_ms && *timestamp < end_ms)
        .collect::<Vec<_>>();
    let mut deltas = in_range
        .windows(2)
        .map(|pair| pair[1].1.saturating_sub(pair[0].1))
        .filter(|delta| *delta > 0)
        .collect::<Vec<_>>();
    deltas.sort_unstable();
    let nominal_fps = deltas
        .get(deltas.len() / 2)
        .map(|delta| 1000.0 / *delta as f64)
        .unwrap_or_default();
    // Common media time bases report 12.5 for an intended 12 FPS stream. Do
    // not turn that harmless 4% difference into a destructive 2x decimation.
    if nominal_fps <= maximum_fps as f64 * 1.10 && in_range.len() <= maximum_count {
        return in_range;
    }
    let first = in_range.first().map(|entry| entry.1).unwrap_or_default();
    let last = in_range.last().map(|entry| entry.1).unwrap_or(first);
    let duration_seconds = last.saturating_sub(first) as f64 / 1_000.0;
    let desired_count = ((duration_seconds * maximum_fps as f64).floor() as usize + 1)
        .clamp(1, maximum_count.min(in_range.len()).max(1));
    (0..desired_count)
        .map(|output| {
            let target = if desired_count <= 1 {
                first
            } else {
                first + last.saturating_sub(first) * output as u64 / (desired_count - 1) as u64
            };
            let minimum_source = output;
            let maximum_source = in_range.len() - (desired_count - output);
            in_range[minimum_source..=maximum_source]
                .iter()
                .min_by_key(|entry| entry.1.abs_diff(target))
                .copied()
                .unwrap_or(in_range[minimum_source])
        })
        .collect()
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

#[cfg(test)]
mod candidate_timing_tests {
    use super::*;

    #[test]
    fn missing_negative_or_duplicate_pts_fail_closed() {
        assert!(parse_frame_timestamps_csv(b"0.000\nN/A\n0.083\n").is_err());
        assert!(parse_frame_timestamps_csv(b"0.000\n-0.042\n0.083\n").is_err());
        assert!(parse_frame_timestamps_csv(b"0.000\n0.000\n0.083\n").is_err());
    }

    #[test]
    fn thirty_fps_timeline_is_evenly_reduced_near_twenty_four_fps() {
        let timestamps = (0..30)
            .map(|index| (index as f64 * 1_000.0 / 30.0).round() as u64)
            .collect::<Vec<_>>();
        let selected = select_candidate_timestamps(&timestamps, 0, 1_000, 24.0, 120);
        assert_eq!(selected.len(), 24);
        assert_eq!(selected.first().copied(), Some((0, 0)));
        assert_eq!(selected.last().copied(), Some((29, 967)));
        assert!(selected.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }
}
