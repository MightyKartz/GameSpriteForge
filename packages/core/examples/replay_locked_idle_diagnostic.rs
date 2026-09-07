use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use forge_core::asset_project::hash_file;
use forge_core::character_cycle::assess_character_native_placement;
use forge_core::keyframe_cleanup::cleanup_keyframe_background;
use forge_core::motion_video::{
    validate_idle_motion_driver_lock, validate_video_candidate_lock, IdleMotionDriverLockV1,
    VideoCandidateLockV1,
};
use forge_core::quality::{validate_source_cycle_sampling_report, SourceCycleSamplingReportV1};
use serde::Deserialize;

const USAGE: &str = "usage: replay_locked_idle_diagnostic <candidate-lock> <native-source> <native-frame-lock> <sampling-report> <output-directory> [uniform-frame-count: 8|12|24]";

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
        || !native.sample_fps.is_finite()
        || native.sample_fps <= 0.0
        || native
            .frames
            .iter()
            .zip(&native_lock.frame_sha256)
            .any(|(path, sha)| {
                !path.is_file() || hash_file(path).map(|actual| actual != *sha).unwrap_or(true)
            })
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
    let sampling_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let output_directory = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let uniform_frame_count = arguments
        .next()
        .map(|value| value.to_string_lossy().parse::<usize>())
        .transpose()
        .map_err(|_| USAGE)?;
    if arguments.next().is_some()
        || uniform_frame_count.is_some_and(|count| !matches!(count, 8 | 12 | 24))
    {
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
    let sampling: SourceCycleSamplingReportV1 = serde_json::from_slice(&fs::read(&sampling_path)?)?;
    validate_source_cycle_sampling_report(&sampling)?;
    if sampling.selected_end_boundary_frame >= native.frames.len()
        || sampling.output_timestamps_ms.len() != sampling.output_frame_indices.len()
        || sampling
            .output_frame_indices
            .iter()
            .zip(&sampling.output_timestamps_ms)
            .any(|(index, timestamp)| native.frame_timestamps_ms.get(*index) != Some(timestamp))
    {
        return Err("sampling report does not belong to native idle frames".into());
    }

    let selected_frame_indices = if let Some(target) = uniform_frame_count {
        let span = sampling
            .selected_end_boundary_frame
            .checked_sub(sampling.selected_start_frame)
            .ok_or("idle sampling interval is invalid")?;
        if span < target {
            return Err(
                "uniform idle frame count must fit inside the locked cycle interval".into(),
            );
        }
        (0..target)
            .map(|step| sampling.selected_start_frame + step * span / target)
            .collect::<Vec<_>>()
    } else {
        sampling.output_frame_indices.clone()
    };
    let selection_mode = if uniform_frame_count.is_some() {
        "uniform_locked_native_cycle"
    } else {
        "sampling_report"
    };
    let selected_sources = selected_frame_indices
        .iter()
        .map(|index| {
            native
                .frames
                .get(*index)
                .cloned()
                .ok_or_else(|| format!("sample index {index} missing from native source"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected_timestamps = selected_frame_indices
        .iter()
        .map(|index| native.frame_timestamps_ms[*index])
        .collect::<Vec<_>>();
    let boundary_timestamp = native.frame_timestamps_ms[sampling.selected_end_boundary_frame];
    let mut frame_durations_ms = selected_timestamps
        .windows(2)
        .map(|timestamps| timestamps[1] - timestamps[0])
        .collect::<Vec<_>>();
    frame_durations_ms.push(
        boundary_timestamp
            .checked_sub(
                *selected_timestamps
                    .last()
                    .ok_or("sampling report has no frames")?,
            )
            .filter(|duration| *duration > 0)
            .ok_or("idle loop boundary does not follow its final frame")?,
    );

    let selected = selected_sources
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    // This is intentionally the only image operation before export. The
    // cleanup keeps the original canvas dimensions and coordinates while
    // retaining the subject-connected component(s); it does not crop,
    // translate, scale, resample, or warp the character.
    let cleanup = selected
        .iter()
        .map(cleanup_keyframe_background)
        .collect::<Vec<_>>();
    let transparent_frames = cleanup
        .iter()
        .map(|(frame, _)| frame.clone())
        .collect::<Vec<_>>();
    let cleanup_reports = cleanup
        .into_iter()
        .map(|(_, report)| report)
        .collect::<Vec<_>>();
    if transparent_frames
        .iter()
        .zip(&selected)
        .any(|(cleaned, source)| cleaned.dimensions() != source.dimensions())
    {
        return Err("cleanup changed a native canvas dimension".into());
    }
    let native_placement = assess_character_native_placement(&driver.action, &transparent_frames);

    let frames_directory = output_directory.join("frames").join(&driver.action);
    fs::create_dir_all(&frames_directory)?;
    let mut frame_paths = Vec::with_capacity(transparent_frames.len());
    for (index, frame) in transparent_frames.iter().enumerate() {
        let path = frames_directory.join(format!("{index:02}.png"));
        frame.save(&path)?;
        frame_paths.push(path);
    }
    fs::write(
        output_directory.join("source-cycle-sampling-report.json"),
        serde_json::to_vec_pretty(&sampling)?,
    )?;
    fs::write(
        output_directory.join("background-cleanup-reports.json"),
        serde_json::to_vec_pretty(&cleanup_reports)?,
    )?;
    fs::write(
        output_directory.join("character-native-placement-report.json"),
        serde_json::to_vec_pretty(&native_placement)?,
    )?;
    let summary = serde_json::json!({
        "profile": "locked-idle-diagnostic-replay@1.0.0",
        "candidateVideoSha256": candidate.source_video_sha256,
        "action": driver.action,
        "direction": driver.direction,
        "providerRequestCount": 0,
        "diagnosticOnly": true,
        "productionEligible": false,
        "deterministicBackgroundCleanupApplied": true,
        "unconnectedWatermarkOrWidgetRemoval": "cleanup_keyframe_background_component_retention",
        "nativePlacementPassthroughApplied": true,
        "translationApplied": false,
        "scaleApplied": false,
        "warpApplied": false,
        "nativePlacementVerdict": native_placement.verdict,
        "selectionMode": selection_mode,
        "selectedFrameCount": frame_paths.len(),
        "selectedFrameIndices": selected_frame_indices,
        "selectedTimestampsMs": selected_timestamps,
        "frameDurationsMs": frame_durations_ms,
        "frames": frame_paths,
    });
    fs::write(
        output_directory.join("replay-summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
