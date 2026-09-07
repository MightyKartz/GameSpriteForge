use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::motion_video::{
    validate_motion_driver_lock, validate_video_candidate_lock, MotionDriverLockV1,
    VideoCandidateLockV1,
};
use forge_core::quality::evaluate_identity_metric;
use serde::Deserialize;

const SAME_DIRECTION_IDENTITY_THRESHOLD: f32 = 0.95;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    candidate_video_sha256: String,
    frames: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let candidate_lock_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: evaluate_locked_video_identity <candidate-lock> <replay-summary> <output-report>",
    )?;
    let replay_summary_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: evaluate_locked_video_identity <candidate-lock> <replay-summary> <output-report>",
    )?;
    let output_report_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: evaluate_locked_video_identity <candidate-lock> <replay-summary> <output-report>",
    )?;
    if arguments.next().is_some() {
        return Err("usage: evaluate_locked_video_identity <candidate-lock> <replay-summary> <output-report>".into());
    }
    if output_report_path.exists() {
        return Err(format!("refusing to overwrite {}", output_report_path.display()).into());
    }

    let candidate: VideoCandidateLockV1 = serde_json::from_slice(&fs::read(&candidate_lock_path)?)?;
    validate_video_candidate_lock(&candidate)?;
    let driver: MotionDriverLockV1 =
        serde_json::from_slice(&fs::read(&candidate.motion_driver_lock_path)?)?;
    validate_motion_driver_lock(&driver)?;
    let replay: ReplaySummary = serde_json::from_slice(&fs::read(&replay_summary_path)?)?;
    if replay.candidate_video_sha256 != candidate.source_video_sha256 || replay.frames.is_empty() {
        return Err("identity inputs do not share one non-empty candidate".into());
    }

    let reference = image::open(&driver.identity_anchor_path)?.to_rgba8();
    let frame_reports = replay
        .frames
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let frame = image::open(path)?.to_rgba8();
            let metric = evaluate_identity_metric(&frame, &reference);
            Ok(serde_json::json!({
                "index": index,
                "path": path,
                "composite": metric.scores.composite,
                "scores": metric.scores,
            }))
        })
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let composites = frame_reports
        .iter()
        .filter_map(|report| report["composite"].as_f64().map(|value| value as f32))
        .collect::<Vec<_>>();
    let minimum = composites.iter().copied().reduce(f32::min).unwrap_or(0.0);
    let maximum = composites.iter().copied().reduce(f32::max).unwrap_or(0.0);
    let mean = composites.iter().sum::<f32>() / composites.len().max(1) as f32;
    let failing_frames = composites
        .iter()
        .enumerate()
        .filter_map(|(index, score)| (*score < SAME_DIRECTION_IDENTITY_THRESHOLD).then_some(index))
        .collect::<Vec<_>>();
    let verdict = if failing_frames.is_empty() {
        "game_ready"
    } else {
        "blocked"
    };
    let report = serde_json::json!({
        "schemaVersion": "1",
        "profile": "locked-video-identity-qa@1.0.0",
        "candidateVideoSha256": candidate.source_video_sha256,
        "referencePath": driver.identity_anchor_path,
        "referenceSha256": driver.identity_anchor_sha256,
        "action": driver.action,
        "direction": driver.direction,
        "sameDirectionCompositeThreshold": SAME_DIRECTION_IDENTITY_THRESHOLD,
        "minimumComposite": minimum,
        "meanComposite": mean,
        "maximumComposite": maximum,
        "failingFrames": failing_frames,
        "frameReports": frame_reports,
        "verdict": verdict,
    });
    if let Some(parent) = output_report_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_report_path, serde_json::to_vec_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
