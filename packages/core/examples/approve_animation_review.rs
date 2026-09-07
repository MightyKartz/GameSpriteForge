use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use forge_core::character_animation_review::{
    approve_animation_review, review_is_approved, validate_animation_review_closure,
    AnimationHumanReviewV1,
};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplaySummary {
    frame_durations_ms: Vec<u64>,
    frames: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 5 {
        return Err("usage: approve_animation_review <replay-summary> <pending-review> <approved-review> <reviewed-at-rfc3339> <review-note>".into());
    }
    let replay_summary_path = PathBuf::from(&arguments[0]);
    let pending_review_path = PathBuf::from(&arguments[1]);
    let approved_review_path = PathBuf::from(&arguments[2]);
    let reviewed_at_raw = arguments[3]
        .to_str()
        .ok_or("reviewed-at must be valid UTF-8")?;
    let review_note = arguments[4]
        .to_str()
        .ok_or("review note must be valid UTF-8")?;
    if approved_review_path.exists() {
        return Err(format!("refusing to overwrite {}", approved_review_path.display()).into());
    }
    let replay: ReplaySummary = serde_json::from_slice(&fs::read(&replay_summary_path)?)?;
    let pending: AnimationHumanReviewV1 = serde_json::from_slice(&fs::read(&pending_review_path)?)?;
    validate_animation_review_closure(
        &pending,
        &replay_summary_path,
        &replay.frames,
        &replay.frame_durations_ms,
    )?;
    if review_is_approved(&pending) {
        return Err("input review is already approved".into());
    }
    let reviewed_at = DateTime::parse_from_rfc3339(reviewed_at_raw)?.with_timezone(&Utc);
    let approved = approve_animation_review(&pending, review_note, reviewed_at)?;
    validate_animation_review_closure(
        &approved,
        &replay_summary_path,
        &replay.frames,
        &replay.frame_durations_ms,
    )?;
    if !review_is_approved(&approved) {
        return Err("Forge approval API did not produce an approved review".into());
    }
    if let Some(parent) = approved_review_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut bytes = serde_json::to_vec_pretty(&approved)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&approved_review_path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    let approved_sha256 = format!("{:x}", Sha256::digest(&bytes));
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "profile": "animation-human-review-approval-result@1.0.0",
            "approvedReview": approved_review_path,
            "approvedReviewSha256": approved_sha256,
            "animation": approved.animation,
            "frameCount": approved.frame_sha256.len(),
            "reviewedAt": approved.reviewed_at,
            "sixChecksPassed": true,
            "providerRequestCount": 0
        }))?
    );
    Ok(())
}
