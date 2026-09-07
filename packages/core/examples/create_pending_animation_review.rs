use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use forge_core::character_animation_review::build_pending_animation_review;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    candidate_video_sha256: String,
    action: String,
    frame_durations_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let replay_summary_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: create_pending_animation_review <replay-summary> <prompt-sha256> <output>",
    )?;
    let prompt_sha256 = arguments
        .next()
        .ok_or("usage: create_pending_animation_review <replay-summary> <prompt-sha256> <output>")?
        .to_string_lossy()
        .into_owned();
    let output_path = arguments.next().map(PathBuf::from).ok_or(
        "usage: create_pending_animation_review <replay-summary> <prompt-sha256> <output>",
    )?;
    if arguments.next().is_some() {
        return Err(
            "usage: create_pending_animation_review <replay-summary> <prompt-sha256> <output>"
                .into(),
        );
    }
    if output_path.exists() {
        return Err(format!("refusing to overwrite {}", output_path.display()).into());
    }
    let replay: ReplaySummary = serde_json::from_slice(&fs::read(&replay_summary_path)?)?;
    let review = build_pending_animation_review(
        replay.action,
        replay.candidate_video_sha256,
        prompt_sha256,
        &replay_summary_path,
        &replay.frames,
        replay.frame_durations_ms,
    )?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output_path, serde_json::to_vec_pretty(&review)?)?;
    println!("{}", serde_json::to_string_pretty(&review)?);
    Ok(())
}
