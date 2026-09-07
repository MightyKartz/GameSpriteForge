use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::character_cycle::stabilize_character_anchors;
use forge_core::keyframe_cleanup::cleanup_keyframe_background;
use forge_core::motion_semantics::assess_character_motion_semantics;
use forge_core::motion_video::{
    validate_motion_driver_lock, validate_video_candidate_lock, MotionDriverLockV1,
    VideoCandidateLockV1,
};
use forge_core::quality::source_cycle_sampling::{
    validate_source_cycle_sampling_report, SourceCycleSamplingReportV1,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeSourceReport {
    source_video_sha256: String,
    frames: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let candidate_lock_path = args.next().map(PathBuf::from).ok_or(
        "usage: replay_locked_video_cycle <candidate-lock> <native-source> <sampling-report> <output-dir>",
    )?;
    let native_source_path = args.next().map(PathBuf::from).ok_or(
        "usage: replay_locked_video_cycle <candidate-lock> <native-source> <sampling-report> <output-dir>",
    )?;
    let sampling_path = args.next().map(PathBuf::from).ok_or(
        "usage: replay_locked_video_cycle <candidate-lock> <native-source> <sampling-report> <output-dir>",
    )?;
    let output_dir = args.next().map(PathBuf::from).ok_or(
        "usage: replay_locked_video_cycle <candidate-lock> <native-source> <sampling-report> <output-dir>",
    )?;
    if output_dir.exists() && output_dir.read_dir()?.next().is_some() {
        return Err(format!("refusing to overwrite non-empty {}", output_dir.display()).into());
    }

    let candidate: VideoCandidateLockV1 = serde_json::from_slice(&fs::read(&candidate_lock_path)?)?;
    validate_video_candidate_lock(&candidate)?;
    let driver: MotionDriverLockV1 =
        serde_json::from_slice(&fs::read(&candidate.motion_driver_lock_path)?)?;
    validate_motion_driver_lock(&driver)?;
    let native: NativeSourceReport = serde_json::from_slice(&fs::read(&native_source_path)?)?;
    if native.source_video_sha256 != candidate.source_video_sha256 {
        return Err("native source does not belong to candidate lock".into());
    }
    let sampling: SourceCycleSamplingReportV1 = serde_json::from_slice(&fs::read(&sampling_path)?)?;
    validate_source_cycle_sampling_report(&sampling)?;

    let selected_sources = sampling
        .output_frame_indices
        .iter()
        .map(|index| {
            native
                .frames
                .get(*index)
                .cloned()
                .ok_or_else(|| format!("sample index {index} missing from native source"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected = selected_sources
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let cleanup = selected
        .iter()
        .map(cleanup_keyframe_background)
        .collect::<Vec<_>>();
    let matted = cleanup
        .iter()
        .map(|(frame, _)| frame.clone())
        .collect::<Vec<_>>();
    let cleanup_reports = cleanup
        .into_iter()
        .map(|(_, report)| report)
        .collect::<Vec<_>>();
    let (stabilized, stabilization) = stabilize_character_anchors(&driver.action, &matted);
    let motion = assess_character_motion_semantics(&BTreeMap::from([(
        driver.action.clone(),
        stabilized.clone(),
    )]));

    let frames_dir = output_dir.join("frames").join(&driver.action);
    fs::create_dir_all(&frames_dir)?;
    let mut frame_paths = Vec::new();
    for (index, frame) in stabilized.iter().enumerate() {
        let path = frames_dir.join(format!("{index:02}.png"));
        frame.save(&path)?;
        frame_paths.push(path);
    }
    fs::write(
        output_dir.join("source-cycle-sampling-report.json"),
        serde_json::to_vec_pretty(&sampling)?,
    )?;
    fs::write(
        output_dir.join("background-cleanup-reports.json"),
        serde_json::to_vec_pretty(&cleanup_reports)?,
    )?;
    fs::write(
        output_dir.join("character-anchor-stabilization-report.json"),
        serde_json::to_vec_pretty(&stabilization)?,
    )?;
    fs::write(
        output_dir.join("character-motion-semantics-report.json"),
        serde_json::to_vec_pretty(&motion)?,
    )?;
    let summary = serde_json::json!({
        "profile": "locked-video-cycle-replay@1.0.0",
        "candidateVideoSha256": candidate.source_video_sha256,
        "action": driver.action,
        "direction": driver.direction,
        "providerRequestCount": 0,
        "selectedFrameCount": frame_paths.len(),
        "selectedFrameIndices": sampling.output_frame_indices,
        "selectedTimestampsMs": sampling.output_timestamps_ms,
        "stabilizationVerdict": stabilization.verdict,
        "motionVerdict": motion.verdict,
        "frames": frame_paths
    });
    fs::write(
        output_dir.join("replay-summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
