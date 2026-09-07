use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::motion_video::{validate_video_candidate_lock, VideoCandidateLockV1};
use forge_core::video::{extract_candidate_frames, ExtractCandidateFramesParams};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let lock_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: extract_locked_video_source <video-candidate-lock.json> <output-directory>",
    )?;
    let output_directory = arguments.next().map(PathBuf::from).ok_or(
        "usage: extract_locked_video_source <video-candidate-lock.json> <output-directory>",
    )?;
    let lock: VideoCandidateLockV1 = serde_json::from_slice(&fs::read(&lock_path)?)?;
    validate_video_candidate_lock(&lock)?;
    let duration_ms = (lock.probe.duration_seconds * 1000.0).round() as u64;
    let result = extract_candidate_frames(&ExtractCandidateFramesParams {
        input_path: lock.source_video_path.clone(),
        start_time_ms: 0,
        end_time_ms: duration_ms,
        maximum_fps: 24.0,
        maximum_frame_count: lock.probe.frame_count_estimate.clamp(120, 600) as u32,
        preserve_source_pts: true,
        output_directory: output_directory.clone(),
        configured_ffmpeg_path: None,
        bundled_resource_path: None,
    })?;
    let report = serde_json::json!({
        "profile": "locked-video-native-source@1.0.0",
        "candidateLock": lock_path.canonicalize()?,
        "sourceVideoSha256": lock.source_video_sha256,
        "frameCount": result.frames.len(),
        "sampleFps": result.sample_fps,
        "durationMs": result.duration_ms,
        "frameTimestampsMs": result.frame_timestamps_ms,
        "frames": result.frames,
        "providerRequestCount": 0
    });
    fs::create_dir_all(&output_directory)?;
    fs::write(
        output_directory.join("native-source.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
