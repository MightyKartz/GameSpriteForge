use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::asset_project::hash_file;
use forge_core::keyframe_cleanup::cleanup_keyframe_background;
use forge_core::motion_video::{
    validate_idle_motion_driver_lock, validate_video_candidate_lock, IdleMotionDriverLockV1,
    VideoCandidateLockV1,
};
use forge_core::quality::{
    sample_source_cycle_frames, select_loop_frames_with_anchor, AnimationCadenceProfile,
    LoopAnchorPolicyV1, LoopSelectionPolicy, SourceCycleSamplingPolicyV1,
};
use image::imageops::FilterType;
use image::RgbaImage;
use serde::Deserialize;

const USAGE: &str = "usage: select_locked_idle_cycle <candidate-lock> <native-source> <native-frame-lock> <output-directory>";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeSourceReport {
    profile: String,
    candidate_lock: PathBuf,
    duration_ms: u64,
    frame_count: usize,
    source_video_sha256: String,
    sample_fps: f32,
    frame_timestamps_ms: Vec<u64>,
    frames: Vec<PathBuf>,
    provider_request_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeFrameLockV1 {
    schema_version: String,
    profile: String,
    candidate_lock_path: PathBuf,
    candidate_lock_sha256: String,
    source_video_path: PathBuf,
    source_video_sha256: String,
    native_source_report_path: PathBuf,
    native_source_report_sha256: String,
    frame_paths: Vec<PathBuf>,
    frame_sha256: Vec<String>,
    frame_timestamps_ms: Vec<u64>,
    provider_request_count: usize,
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

fn same_file(left: &Path, right: &Path) -> Result<bool, Box<dyn Error>> {
    Ok(left.canonicalize()? == right.canonicalize()?)
}

fn validate_native_inputs(
    candidate_lock_path: &Path,
    native_source_path: &Path,
    candidate: &VideoCandidateLockV1,
    native: &NativeSourceReport,
    native_lock: &NativeFrameLockV1,
) -> Result<(), Box<dyn Error>> {
    if native.profile != "locked-video-native-source@1.0.0"
        || native_lock.schema_version != "1"
        || native_lock.profile != "native-frame-lock@1.0.0"
        || native.source_video_sha256 != candidate.source_video_sha256
        || native_lock.source_video_sha256 != candidate.source_video_sha256
        || !same_file(&native_lock.source_video_path, &candidate.source_video_path)?
        || !same_file(&native.candidate_lock, candidate_lock_path)?
        || !same_file(&native_lock.candidate_lock_path, candidate_lock_path)?
        || native_lock.candidate_lock_sha256 != hash_file(candidate_lock_path)?
        || !same_file(&native_lock.native_source_report_path, native_source_path)?
        || native_lock.native_source_report_sha256 != hash_file(native_source_path)?
        || native.frame_count != native.frames.len()
        || native.duration_ms == 0
        || native.provider_request_count != 0
        || native_lock.provider_request_count != 0
        || native.frames.len() != native.frame_timestamps_ms.len()
        || native.frames != native_lock.frame_paths
        || native.frame_timestamps_ms != native_lock.frame_timestamps_ms
        || native.frames.len() != native_lock.frame_sha256.len()
        || native
            .frames
            .iter()
            .zip(&native_lock.frame_sha256)
            .any(|(path, sha)| {
                !path.is_file() || hash_file(path).map(|actual| actual != *sha).unwrap_or(true)
            })
        || !native.sample_fps.is_finite()
        || native.sample_fps <= 0.0
        || native
            .frame_timestamps_ms
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        return Err("native inputs do not share one locked idle candidate".into());
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let candidate_lock_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let native_source_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let native_frame_lock_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let output_directory = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    if arguments.next().is_some() {
        return Err(USAGE.into());
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
    let driver: IdleMotionDriverLockV1 =
        serde_json::from_slice(&fs::read(&candidate.motion_driver_lock_path)?)?;
    validate_idle_motion_driver_lock(&driver)?;
    let native: NativeSourceReport = serde_json::from_slice(&fs::read(&native_source_path)?)?;
    let native_lock: NativeFrameLockV1 =
        serde_json::from_slice(&fs::read(&native_frame_lock_path)?)?;
    validate_native_inputs(
        &candidate_lock_path,
        &native_source_path,
        &candidate,
        &native,
        &native_lock,
    )?;

    let raw = native
        .frames
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let cleanup = raw
        .iter()
        .map(resize_for_cycle_analysis)
        .map(|frame| cleanup_keyframe_background(&frame))
        .collect::<Vec<_>>();
    let analysis_frames = cleanup
        .iter()
        .map(|(frame, _)| frame.clone())
        .collect::<Vec<_>>();
    let cleanup_reports = cleanup
        .into_iter()
        .map(|(_, report)| report)
        .collect::<Vec<_>>();

    let maximum_start_frame = (native.sample_fps * 0.25).ceil() as usize;
    let policy = LoopSelectionPolicy {
        target_frame_count: 8,
        candidate_fps: native.sample_fps,
        min_duration_ms: 900,
        max_duration_ms: 2_400,
        minimum_motion_energy: 0.002,
        alpha_threshold: 0,
        cadence_profile: AnimationCadenceProfile::Idle,
    };
    let anchor = LoopAnchorPolicyV1 {
        reference_frame: 0,
        maximum_start_frame,
        minimum_start_similarity: 0.88,
    };

    fs::create_dir_all(&output_directory)?;
    fs::write(
        output_directory.join("idle-cycle-policy.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "profile": "locked-idle-cycle-policy@1.0.0",
            "minimumDurationMs": policy.min_duration_ms,
            "maximumDurationMs": policy.max_duration_ms,
            "maximumStartFrame": maximum_start_frame,
            "maximumStartMs": 250,
            "minimumStartSimilarity": anchor.minimum_start_similarity,
            "analysisMaximumDimension": 256,
            "nativeCoordinatesPreserved": true,
            "providerRequestCount": 0,
        }))?,
    )?;
    fs::write(
        output_directory.join("analysis-background-cleanup-reports.json"),
        serde_json::to_vec_pretty(&cleanup_reports)?,
    )?;

    let selection = match select_loop_frames_with_anchor(&analysis_frames, policy, anchor) {
        Ok(selection) => selection,
        Err(error) => {
            let summary = serde_json::json!({
                "profile": "locked-idle-cycle-selection@1.0.0",
                "candidateVideoSha256": candidate.source_video_sha256,
                "action": driver.action,
                "direction": driver.direction,
                "providerRequestCount": 0,
                "idleCycleVerdict": "blocked",
                "idleCycleError": error.to_string(),
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
        output_directory.join("idle-cycle-report.json"),
        serde_json::to_vec_pretty(&selection.report)?,
    )?;
    let sampling = match sample_source_cycle_frames(
        &analysis_frames,
        &native.frame_timestamps_ms,
        selection.report.selected_start_frame,
        selection.report.selected_end_boundary_frame,
        &selection.report.output_frame_indices,
        // Keep the selected native interval fixed while permitting later idle
        // delivery to promote from the eight semantic samples to 12/24 source
        // poses if the deterministic reconstruction budget requires it.
        SourceCycleSamplingPolicyV1::production_up_to_24(),
    ) {
        Ok(sampling) => sampling,
        Err(error) => {
            let summary = serde_json::json!({
                "profile": "locked-idle-cycle-selection@1.0.0",
                "candidateVideoSha256": candidate.source_video_sha256,
                "action": driver.action,
                "direction": driver.direction,
                "providerRequestCount": 0,
                "idleCycleVerdict": selection.report.verdict,
                "idleCycleReasons": selection.report.reasons,
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
        "profile": "locked-idle-cycle-selection@1.0.0",
        "candidateVideoSha256": candidate.source_video_sha256,
        "action": driver.action,
        "direction": driver.direction,
        "providerRequestCount": 0,
        "idleCycleVerdict": selection.report.verdict,
        "idleCycleReasons": selection.report.reasons,
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
