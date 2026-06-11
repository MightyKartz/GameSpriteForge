use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};

use super::ffmpeg::{resolve_binary, FFMPEG_MISSING_MESSAGE};
use super::VideoError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeVideoParams {
    pub input_path: PathBuf,
    pub configured_ffprobe_path: Option<PathBuf>,
    pub bundled_resource_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoProbe {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
    pub duration_seconds: f64,
    pub frame_count_estimate: u64,
    pub codec: String,
    pub pixel_format: String,
}

pub fn probe_video(params: &ProbeVideoParams) -> Result<VideoProbe, VideoError> {
    let ffprobe_path = resolve_binary(
        "ffprobe",
        params.configured_ffprobe_path.as_deref(),
        params.bundled_resource_path.as_deref(),
    )
    .map_err(|_| VideoError::new("ffmpeg_missing", FFMPEG_MISSING_MESSAGE))?;

    let output = Command::new(ffprobe_path)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height,codec_name,pix_fmt,r_frame_rate,avg_frame_rate,nb_frames,duration:format=duration",
            "-of",
            "json",
        ])
        .arg(&params.input_path)
        .output()?;

    if !output.status.success() {
        return Err(VideoError::command_failed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let ffprobe: FfprobeOutput = serde_json::from_slice(&output.stdout)?;
    let stream = ffprobe
        .streams
        .into_iter()
        .next()
        .ok_or_else(|| VideoError::invalid_output("ffprobe did not return a video stream"))?;

    let fps = parse_rate(stream.avg_frame_rate.as_deref())
        .or_else(|| parse_rate(stream.r_frame_rate.as_deref()))
        .unwrap_or(0.0);
    let duration_seconds = parse_f64(stream.duration.as_deref())
        .or_else(|| {
            ffprobe
                .format
                .and_then(|format| parse_f64(format.duration.as_deref()))
        })
        .unwrap_or(0.0);
    let frame_count_estimate = stream
        .nb_frames
        .as_deref()
        .and_then(parse_u64)
        .unwrap_or_else(|| (duration_seconds * fps).round().max(0.0) as u64);

    Ok(VideoProbe {
        width: stream.width.unwrap_or(0),
        height: stream.height.unwrap_or(0),
        fps,
        duration_seconds,
        frame_count_estimate,
        codec: stream.codec_name.unwrap_or_default(),
        pixel_format: stream.pix_fmt.unwrap_or_default(),
    })
}

fn parse_rate(value: Option<&str>) -> Option<f64> {
    let value = value?;
    if value == "0/0" {
        return None;
    }

    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator.parse::<f64>().ok()?;
        let denominator = denominator.parse::<f64>().ok()?;
        if denominator == 0.0 {
            None
        } else {
            Some(numerator / denominator)
        }
    } else {
        value.parse::<f64>().ok()
    }
}

fn parse_f64(value: Option<&str>) -> Option<f64> {
    value?.parse::<f64>().ok()
}

fn parse_u64(value: &str) -> Option<u64> {
    value.parse::<u64>().ok()
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    width: Option<u32>,
    height: Option<u32>,
    codec_name: Option<String>,
    pix_fmt: Option<String>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    nb_frames: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}
