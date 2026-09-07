use std::env;
use std::error::Error;
use std::path::PathBuf;

use forge_core::motion_video::{
    create_motion_driver_lock, import_video_candidate, BrowserQuotaSnapshotV1,
    CreateMotionDriverLockRequestV1, ImportVideoCandidateRequestV1, MotionDriverSourceKindV1,
    VideoAcquisitionKindV1, MOTION_DRIVER_LOCK_FILE,
};

fn next_argument(
    arguments: &mut impl Iterator<Item = std::ffi::OsString>,
    usage: &'static str,
) -> Result<std::ffi::OsString, Box<dyn Error>> {
    arguments.next().ok_or_else(|| usage.into())
}

fn parse_bool(value: std::ffi::OsString) -> Result<bool, Box<dyn Error>> {
    match value.to_string_lossy().as_ref() {
        "true" => Ok(true),
        "false" => Ok(false),
        other => Err(format!("expected true or false, received {other}").into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    const USAGE: &str = "usage: import_browser_motion_video_candidate <anchor.png> <video.mp4> <prompt.txt> <output-root> <candidate-id> <action> <direction> <provider> <model> <provider-task-id|-> <quota-before|-> <quota-after|-> <free-used> <reward-claimed>";
    let mut arguments = env::args_os().skip(1);
    let anchor = PathBuf::from(next_argument(&mut arguments, USAGE)?);
    let video = PathBuf::from(next_argument(&mut arguments, USAGE)?);
    let prompt = PathBuf::from(next_argument(&mut arguments, USAGE)?);
    let output_root = PathBuf::from(next_argument(&mut arguments, USAGE)?);
    let candidate_id = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let action = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let direction = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let provider = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let model = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let task_id = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let quota_before = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let quota_after = next_argument(&mut arguments, USAGE)?
        .to_string_lossy()
        .into_owned();
    let free_generation_used = parse_bool(next_argument(&mut arguments, USAGE)?)?;
    let reward_claimed = parse_bool(next_argument(&mut arguments, USAGE)?)?;
    if arguments.next().is_some() {
        return Err(USAGE.into());
    }

    let driver_path = output_root.join(MOTION_DRIVER_LOCK_FILE);
    create_motion_driver_lock(&CreateMotionDriverLockRequestV1 {
        output_path: driver_path.clone(),
        id: format!("{candidate_id}-driver"),
        action,
        direction,
        source_kind: MotionDriverSourceKindV1::ProceduralGaitSpec,
        identity_anchor_path: anchor,
        driver_source_path: None,
        requested_duration_ms: 4_000,
        expected_cycles_minimum: 2,
        maximum_swing_foot_clearance_ratio: 0.12,
    })?;
    let quota = match (quota_before.as_str(), quota_after.as_str()) {
        ("-", "-") => None,
        ("-", _) | (_, "-") => {
            return Err("quota before and after must both be numbers or both be '-'".into())
        }
        (before, after) => Some(BrowserQuotaSnapshotV1 {
            unit: "credits".into(),
            before: before.parse::<f64>()?,
            after: after.parse::<f64>()?,
            free_generation_used,
            reward_claimed,
        }),
    };
    let lock = import_video_candidate(&ImportVideoCandidateRequestV1 {
        output_directory: output_root.join("candidate"),
        id: candidate_id,
        acquisition: VideoAcquisitionKindV1::Browser,
        provider,
        model,
        provider_task_id: (task_id != "-").then_some(task_id),
        provider_request_count: 1,
        imported_without_provider_request: false,
        motion_driver_lock_path: driver_path,
        prompt_path: prompt,
        source_video_path: video,
        quota,
    })?;
    println!("{}", serde_json::to_string_pretty(&lock)?);
    Ok(())
}
