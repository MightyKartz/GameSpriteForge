use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::character_direction::DirectionViewV1;
use forge_core::keyframe_cleanup::cleanup_keyframe_background;
use forge_core::motion_video::{
    validate_motion_driver_lock, validate_video_candidate_lock, MotionDriverLockV1,
    VideoCandidateLockV1,
};
use forge_core::quality::{
    sample_source_cycle_frames, select_gait_cycle_frames, GaitCyclePolicy,
    SourceCycleSamplingPolicyV1,
};
use image::imageops::FilterType;
use image::RgbaImage;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeSourceReport {
    source_video_sha256: String,
    sample_fps: f32,
    frame_timestamps_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn direction_view(direction: &str) -> Result<DirectionViewV1, Box<dyn Error>> {
    match direction {
        "down" | "front" => Ok(DirectionViewV1::Front),
        "up" | "rear" => Ok(DirectionViewV1::Rear),
        "right" => Ok(DirectionViewV1::Right),
        "left" => Ok(DirectionViewV1::Left),
        other => Err(format!("unsupported locked direction: {other}").into()),
    }
}

fn resize_for_cycle_analysis(frame: &RgbaImage) -> RgbaImage {
    const MAXIMUM_DIMENSION: u32 = 256;
    let longest = frame.width().max(frame.height());
    if longest <= MAXIMUM_DIMENSION {
        return frame.clone();
    }
    let scale = MAXIMUM_DIMENSION as f64 / longest as f64;
    let width = (frame.width() as f64 * scale).round().max(1.0) as u32;
    let height = (frame.height() as f64 * scale).round().max(1.0) as u32;
    image::imageops::resize(frame, width, height, FilterType::Triangle)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let candidate_lock_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: select_locked_video_cycle <candidate-lock> <native-source> <output-directory>",
    )?;
    let native_source_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: select_locked_video_cycle <candidate-lock> <native-source> <output-directory>",
    )?;
    let output_directory = arguments.next().map(PathBuf::from).ok_or(
        "usage: select_locked_video_cycle <candidate-lock> <native-source> <output-directory>",
    )?;
    if arguments.next().is_some() {
        return Err(
            "usage: select_locked_video_cycle <candidate-lock> <native-source> <output-directory>"
                .into(),
        );
    }
    if output_directory.exists() && output_directory.read_dir()?.next().is_some() {
        return Err(format!(
            "refusing to overwrite non-empty {}",
            output_directory.display()
        )
        .into());
    }

    let candidate: VideoCandidateLockV1 = serde_json::from_slice(&fs::read(&candidate_lock_path)?)?;
    validate_video_candidate_lock(&candidate)?;
    let driver: MotionDriverLockV1 =
        serde_json::from_slice(&fs::read(&candidate.motion_driver_lock_path)?)?;
    validate_motion_driver_lock(&driver)?;
    let native: NativeSourceReport = serde_json::from_slice(&fs::read(&native_source_path)?)?;
    if native.source_video_sha256 != candidate.source_video_sha256
        || native.frames.len() != native.frame_timestamps_ms.len()
    {
        return Err("native source does not belong to candidate lock".into());
    }

    let raw = native
        .frames
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    // Cycle search depends on silhouette, contact ownership, and lower-body
    // phase, not delivery resolution. Bound the analysis raster so a 1440p
    // browser candidate does not turn pairwise mask comparison into a costly
    // operation. Replay still reopens and normalizes the original source
    // frames selected by the resulting native indices.
    let analysis_raw = raw
        .iter()
        .map(resize_for_cycle_analysis)
        .collect::<Vec<_>>();
    let cleanup = analysis_raw
        .iter()
        .map(cleanup_keyframe_background)
        .collect::<Vec<_>>();
    let analysis_frames = cleanup
        .iter()
        .map(|(frame, _)| frame.clone())
        .collect::<Vec<_>>();
    let cleanup_reports = cleanup
        .into_iter()
        .map(|(_, report)| report)
        .collect::<Vec<_>>();
    // MotionDriverLock requires the source root and camera to be fixed. Keep
    // cycle selection in native source coordinates and reserve translation-
    // only normalization for the selected 8/10/12 delivery frames. This
    // preserves the required native-cycle-before-normalization order.
    fs::create_dir_all(&output_directory)?;
    fs::write(
        output_directory.join("analysis-policy.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "profile": "locked-video-cycle-analysis-policy@1.0.0",
            "maximumAnalysisDimension": 256,
            "nativeCoordinatesPreserved": true,
            "normalizationAppliedBeforeCycleSelection": false,
            "normalizationDeferredUntilSelectedFrames": true,
        }))?,
    )?;
    fs::write(
        output_directory.join("analysis-background-cleanup-reports.json"),
        serde_json::to_vec_pretty(&cleanup_reports)?,
    )?;
    let expected_cycle_duration_ms = driver
        .requested_duration_ms
        .checked_div(driver.expected_cycles_minimum.max(1) as u64)
        .unwrap_or(800)
        .clamp(700, 2_500);
    let source_duration_ms = native
        .frame_timestamps_ms
        .last()
        .copied()
        .unwrap_or(expected_cycle_duration_ms);
    let mut gait_policy =
        GaitCyclePolicy::ordinary_walk(direction_view(&driver.direction)?, native.sample_fps);
    gait_policy.max_duration_ms = expected_cycle_duration_ms
        .saturating_mul(5)
        .div_ceil(4)
        .min(source_duration_ms.saturating_mul(19).div_ceil(20))
        .clamp(gait_policy.max_duration_ms, 2_500);
    gait_policy.min_duration_ms = expected_cycle_duration_ms
        .saturating_mul(3)
        .div_ceil(4)
        .clamp(700, gait_policy.max_duration_ms);
    gait_policy.preferred_duration_ms =
        expected_cycle_duration_ms.clamp(gait_policy.min_duration_ms, gait_policy.max_duration_ms);
    fs::write(
        output_directory.join("motion-driver-cycle-policy.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "profile": "motion-driver-cycle-policy@1.0.0",
            "requestedDurationMs": driver.requested_duration_ms,
            "expectedCyclesMinimum": driver.expected_cycles_minimum,
            "expectedCycleDurationMs": expected_cycle_duration_ms,
            "gaitMinimumDurationMs": gait_policy.min_duration_ms,
            "gaitPreferredDurationMs": gait_policy.preferred_duration_ms,
            "gaitMaximumDurationMs": gait_policy.max_duration_ms,
        }))?,
    )?;
    let gait = match select_gait_cycle_frames(&analysis_frames, gait_policy) {
        Ok(gait) => gait,
        Err(error) => {
            let summary = serde_json::json!({
                "profile": "locked-video-cycle-selection@1.0.0",
                "candidateVideoSha256": candidate.source_video_sha256,
                "action": driver.action,
                "direction": driver.direction,
                "providerRequestCount": 0,
                "gaitVerdict": "blocked",
                "gaitError": error.to_string(),
                "samplingVerdict": "not_run",
            });
            fs::write(
                output_directory.join("selection-summary.json"),
                serde_json::to_vec_pretty(&summary)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
            return Ok(());
        }
    };
    fs::write(
        output_directory.join("gait-cycle-report.json"),
        serde_json::to_vec_pretty(&gait.report)?,
    )?;
    let sampling = match sample_source_cycle_frames(
        &analysis_frames,
        &native.frame_timestamps_ms,
        gait.report.selected_start_frame,
        gait.report.selected_end_boundary_frame,
        &gait.report.output_frame_indices,
        SourceCycleSamplingPolicyV1::default(),
    ) {
        Ok(sampling) => sampling,
        Err(error) => {
            let summary = serde_json::json!({
                "profile": "locked-video-cycle-selection@1.0.0",
                "candidateVideoSha256": candidate.source_video_sha256,
                "action": driver.action,
                "direction": driver.direction,
                "providerRequestCount": 0,
                "gaitVerdict": gait.report.verdict,
                "gaitReasons": gait.report.reasons,
                "samplingVerdict": "blocked",
                "samplingError": error.to_string(),
            });
            fs::write(
                output_directory.join("selection-summary.json"),
                serde_json::to_vec_pretty(&summary)?,
            )?;
            println!("{}", serde_json::to_string_pretty(&summary)?);
            return Ok(());
        }
    };
    fs::write(
        output_directory.join("source-cycle-sampling-report.json"),
        serde_json::to_vec_pretty(&sampling.report)?,
    )?;
    let summary = serde_json::json!({
        "profile": "locked-video-cycle-selection@1.0.0",
        "candidateVideoSha256": candidate.source_video_sha256,
        "action": driver.action,
        "direction": driver.direction,
        "providerRequestCount": 0,
        "gaitVerdict": gait.report.verdict,
        "gaitReasons": gait.report.reasons,
        "selectedStartFrame": sampling.report.selected_start_frame,
        "selectedEndBoundaryFrame": sampling.report.selected_end_boundary_frame,
        "selectedFrameIndices": sampling.report.output_frame_indices,
        "selectedTimestampsMs": sampling.report.output_timestamps_ms,
        "selectedFrameCount": sampling.report.output_frame_count,
        "reconstructionDecision": sampling.report.decision,
        "reconstructionError": sampling.report.normalized_reconstruction_error,
    });
    fs::write(
        output_directory.join("selection-summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
