use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::character_cycle::{
    assess_character_native_placement, normalize_character_vertical_bob,
    stabilize_character_anchors, stabilize_character_visual_root,
};
use forge_core::keyframe_cleanup::cleanup_keyframe_background;
use forge_core::motion_semantics::assess_character_motion_semantics;
use forge_core::motion_video::{
    validate_motion_driver_lock, validate_video_candidate_lock, MotionDriverLockV1,
    VideoCandidateLockV1,
};
use forge_core::quality::{GaitCycleReport, GAIT_CYCLE_PROFILE};
use serde::Deserialize;

const USAGE: &str = "usage: replay_locked_gait_diagnostic <candidate-lock> <native-source> <gait-report> <output-directory> [uniform-frame-count] [--native-placement-passthrough | --stabilize-visual-root | --normalize-vertical-bob]";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NativeSourceReport {
    source_video_sha256: String,
    frame_timestamps_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let candidate_lock_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let native_source_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let gait_report_path = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let output_directory = arguments.next().map(PathBuf::from).ok_or(USAGE)?;
    let mut uniform_frame_count = None;
    let mut native_placement_passthrough = false;
    let mut normalize_vertical_bob = false;
    let mut stabilize_visual_root = false;
    for argument in arguments {
        match argument.to_string_lossy().as_ref() {
            "--native-placement-passthrough" if !native_placement_passthrough => {
                native_placement_passthrough = true;
            }
            "--normalize-vertical-bob" if !normalize_vertical_bob => {
                normalize_vertical_bob = true;
            }
            "--stabilize-visual-root" if !stabilize_visual_root => {
                stabilize_visual_root = true;
            }
            value if uniform_frame_count.is_none() => {
                uniform_frame_count = Some(value.parse::<usize>().map_err(|_| USAGE)?);
            }
            _ => return Err(USAGE.into()),
        }
    }
    if [
        native_placement_passthrough,
        normalize_vertical_bob,
        stabilize_visual_root,
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count()
        > 1
    {
        return Err("native placement, visual-root translation, and vertical rescaling are mutually exclusive".into());
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
    let gait: GaitCycleReport = serde_json::from_slice(&fs::read(&gait_report_path)?)?;
    if native.source_video_sha256 != candidate.source_video_sha256
        || native.frames.len() != native.frame_timestamps_ms.len()
        || gait.profile != GAIT_CYCLE_PROFILE
        || gait.animation != driver.action
        || gait.candidate_frame_count != native.frames.len()
        || gait.selected_end_boundary_frame >= native.frames.len()
    {
        return Err("diagnostic replay inputs do not share one locked candidate".into());
    }
    let selected_frame_indices = if let Some(target) = uniform_frame_count {
        if !matches!(target, 8 | 10 | 12 | 16 | 24)
            || gait.selected_end_boundary_frame - gait.selected_start_frame < target
        {
            return Err("uniform diagnostic frame count must be 8/10/12/16/24 and fit inside the gait interval".into());
        }
        let span = gait.selected_end_boundary_frame - gait.selected_start_frame;
        (0..target)
            .map(|step| gait.selected_start_frame + step * span / target)
            .collect::<Vec<_>>()
    } else {
        gait.output_frame_indices.clone()
    };
    let selection_mode = if uniform_frame_count.is_some() {
        "uniform_native_cycle"
    } else {
        "gait_semantic_phases"
    };
    let selected_sources = selected_frame_indices
        .iter()
        .map(|index| {
            native
                .frames
                .get(*index)
                .cloned()
                .ok_or_else(|| format!("gait frame {index} missing from native source"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected_timestamps = selected_frame_indices
        .iter()
        .map(|index| native.frame_timestamps_ms[*index])
        .collect::<Vec<_>>();
    let boundary_timestamp = native.frame_timestamps_ms[gait.selected_end_boundary_frame];
    let mut frame_durations_ms = selected_timestamps
        .windows(2)
        .map(|timestamps| timestamps[1] - timestamps[0])
        .collect::<Vec<_>>();
    frame_durations_ms.push(
        boundary_timestamp
            .checked_sub(*selected_timestamps.last().ok_or("gait has no frames")?)
            .filter(|duration| *duration > 0)
            .ok_or("gait boundary does not follow the final phase")?,
    );

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
    let (stabilized, stabilization, native_placement) = if native_placement_passthrough {
        let report = assess_character_native_placement(&driver.action, &matted);
        (matted, None, Some(report))
    } else {
        let (stabilized, report) = stabilize_character_anchors(&driver.action, &matted);
        (stabilized, Some(report), None)
    };
    let (root_stabilized, visual_root) = if stabilize_visual_root {
        let (root_stabilized, report) =
            stabilize_character_visual_root(&driver.action, &stabilized);
        (root_stabilized, Some(report))
    } else {
        (stabilized, None)
    };
    let (delivery_frames, vertical_bob) = if normalize_vertical_bob {
        let (normalized, report) =
            normalize_character_vertical_bob(&driver.action, &root_stabilized);
        (normalized, Some(report))
    } else {
        (root_stabilized, None)
    };
    let motion = assess_character_motion_semantics(&BTreeMap::from([(
        driver.action.clone(),
        delivery_frames.clone(),
    )]));

    let frames_directory = output_directory.join("frames").join(&driver.action);
    fs::create_dir_all(&frames_directory)?;
    let mut frame_paths = Vec::new();
    for (index, frame) in delivery_frames.iter().enumerate() {
        let path = frames_directory.join(format!("{index:02}.png"));
        frame.save(&path)?;
        frame_paths.push(path);
    }
    fs::write(
        output_directory.join("gait-cycle-report.json"),
        serde_json::to_vec_pretty(&gait)?,
    )?;
    fs::write(
        output_directory.join("background-cleanup-reports.json"),
        serde_json::to_vec_pretty(&cleanup_reports)?,
    )?;
    if let Some(report) = &stabilization {
        fs::write(
            output_directory.join("character-anchor-stabilization-report.json"),
            serde_json::to_vec_pretty(report)?,
        )?;
    }
    if let Some(report) = &native_placement {
        fs::write(
            output_directory.join("character-native-placement-report.json"),
            serde_json::to_vec_pretty(report)?,
        )?;
    }
    if let Some(report) = &visual_root {
        fs::write(
            output_directory.join("character-visual-root-stabilization-report.json"),
            serde_json::to_vec_pretty(report)?,
        )?;
    }
    if let Some(report) = &vertical_bob {
        fs::write(
            output_directory.join("character-vertical-bob-normalization-report.json"),
            serde_json::to_vec_pretty(report)?,
        )?;
    }
    fs::write(
        output_directory.join("character-motion-semantics-report.json"),
        serde_json::to_vec_pretty(&motion)?,
    )?;
    let summary = serde_json::json!({
        "profile": "locked-video-gait-diagnostic-replay@1.0.0",
        "candidateVideoSha256": candidate.source_video_sha256,
        "action": driver.action,
        "direction": driver.direction,
        "providerRequestCount": 0,
        "diagnosticOnly": true,
        "productionEligible": false,
        "gaitVerdict": gait.verdict,
        "gaitReasons": gait.reasons,
        "selectionMode": selection_mode,
        "nativePlacementPassthroughApplied": native_placement_passthrough,
        "nativePlacementVerdict": native_placement.as_ref().map(|report| report.verdict),
        "visualRootStabilizationApplied": stabilize_visual_root,
        "visualRootVerdict": visual_root.as_ref().map(|report| report.verdict),
        "verticalBobNormalizationApplied": normalize_vertical_bob,
        "verticalBobVerdict": vertical_bob.as_ref().map(|report| report.verdict),
        "selectedFrameCount": frame_paths.len(),
        "selectedFrameIndices": selected_frame_indices,
        "selectedTimestampsMs": selected_timestamps,
        "frameDurationsMs": frame_durations_ms,
        "stabilizationVerdict": stabilization
            .as_ref()
            .map(|report| report.verdict)
            .or_else(|| native_placement.as_ref().map(|report| report.verdict)),
        "motionVerdict": motion.verdict,
        "frames": frame_paths,
    });
    fs::write(
        output_directory.join("replay-summary.json"),
        serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}
