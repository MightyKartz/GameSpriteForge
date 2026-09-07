use std::env;
use std::error::Error;
use std::path::PathBuf;

use forge_core::motion_video::{
    create_motion_driver_lock, import_video_candidate, CreateMotionDriverLockRequestV1,
    ImportVideoCandidateRequestV1, MotionDriverSourceKindV1, VideoAcquisitionKindV1,
    MOTION_DRIVER_LOCK_FILE,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let anchor = arguments.next().map(PathBuf::from).ok_or(
        "usage: import_motion_video_candidate <anchor.png> <video.mp4> <prompt.txt> <output-root>",
    )?;
    let video = arguments.next().map(PathBuf::from).ok_or(
        "usage: import_motion_video_candidate <anchor.png> <video.mp4> <prompt.txt> <output-root>",
    )?;
    let prompt = arguments.next().map(PathBuf::from).ok_or(
        "usage: import_motion_video_candidate <anchor.png> <video.mp4> <prompt.txt> <output-root>",
    )?;
    let output_root = arguments.next().map(PathBuf::from).ok_or(
        "usage: import_motion_video_candidate <anchor.png> <video.mp4> <prompt.txt> <output-root>",
    )?;
    let driver_path = output_root.join(MOTION_DRIVER_LOCK_FILE);
    create_motion_driver_lock(&CreateMotionDriverLockRequestV1 {
        output_path: driver_path.clone(),
        id: "fixture-walk-down-driver".into(),
        action: "walk_down".into(),
        direction: "down".into(),
        source_kind: MotionDriverSourceKindV1::ProceduralGaitSpec,
        identity_anchor_path: anchor,
        driver_source_path: None,
        requested_duration_ms: 4_000,
        expected_cycles_minimum: 2,
        maximum_swing_foot_clearance_ratio: 0.12,
    })?;
    let lock = import_video_candidate(&ImportVideoCandidateRequestV1 {
        output_directory: output_root.join("candidate"),
        id: "retained-v10-walk-down".into(),
        acquisition: VideoAcquisitionKindV1::ExternalImport,
        provider: "xai-retained-fixture".into(),
        model: "grok-imagine-video-1.5".into(),
        provider_task_id: None,
        provider_request_count: 0,
        imported_without_provider_request: true,
        motion_driver_lock_path: driver_path,
        prompt_path: prompt,
        source_video_path: video,
        quota: None,
    })?;
    println!("{}", serde_json::to_string_pretty(&lock)?);
    Ok(())
}
