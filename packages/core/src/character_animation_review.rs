use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const ANIMATION_HUMAN_REVIEW_PROFILE: &str = "animation-human-review@1.0.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimationReviewStatusV1 {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnimationHumanReviewChecksV1 {
    pub motion_acceptable: Option<bool>,
    pub placement_acceptable: Option<bool>,
    pub edge_cleanup_acceptable: Option<bool>,
    pub foot_alternation_acceptable: Option<bool>,
    pub pivot_acceptable: Option<bool>,
    pub collision_acceptable: Option<bool>,
}

impl AnimationHumanReviewChecksV1 {
    pub const fn pending() -> Self {
        Self {
            motion_acceptable: None,
            placement_acceptable: None,
            edge_cleanup_acceptable: None,
            foot_alternation_acceptable: None,
            pivot_acceptable: None,
            collision_acceptable: None,
        }
    }

    fn values(&self) -> [Option<bool>; 6] {
        [
            self.motion_acceptable,
            self.placement_acceptable,
            self.edge_cleanup_acceptable,
            self.foot_alternation_acceptable,
            self.pivot_acceptable,
            self.collision_acceptable,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnimationHumanReviewV1 {
    pub schema_version: String,
    pub profile: String,
    pub animation: String,
    pub source_video_sha256: String,
    pub prompt_sha256: String,
    pub replay_summary_sha256: String,
    pub frame_sha256: Vec<String>,
    pub frame_durations_ms: Vec<u64>,
    pub reviewer: String,
    pub status: AnimationReviewStatusV1,
    pub checks: AnimationHumanReviewChecksV1,
    pub review_note: String,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Error)]
pub enum AnimationHumanReviewError {
    #[error("animation human review is unsupported or incomplete")]
    InvalidReview,
    #[error("animation human review no longer matches its immutable frame closure")]
    ClosureMismatch,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub fn build_pending_animation_review(
    animation: impl Into<String>,
    source_video_sha256: impl Into<String>,
    prompt_sha256: impl Into<String>,
    replay_summary_path: &Path,
    frame_paths: &[PathBuf],
    frame_durations_ms: Vec<u64>,
) -> Result<AnimationHumanReviewV1, AnimationHumanReviewError> {
    let frame_sha256 = frame_paths
        .iter()
        .map(|path| hash_file(path))
        .collect::<Result<Vec<_>, _>>()?;
    let review = AnimationHumanReviewV1 {
        schema_version: "1".into(),
        profile: ANIMATION_HUMAN_REVIEW_PROFILE.into(),
        animation: animation.into(),
        source_video_sha256: source_video_sha256.into(),
        prompt_sha256: prompt_sha256.into(),
        replay_summary_sha256: hash_file(replay_summary_path)?,
        frame_sha256,
        frame_durations_ms,
        reviewer: "human".into(),
        status: AnimationReviewStatusV1::Pending,
        checks: AnimationHumanReviewChecksV1::pending(),
        review_note: "Awaiting human review in the moving Godot test scene.".into(),
        reviewed_at: None,
    };
    validate_animation_review_shape(&review)?;
    Ok(review)
}

pub fn validate_animation_review_closure(
    review: &AnimationHumanReviewV1,
    replay_summary_path: &Path,
    frame_paths: &[PathBuf],
    frame_durations_ms: &[u64],
) -> Result<(), AnimationHumanReviewError> {
    validate_animation_review_shape(review)?;
    let frame_sha256 = frame_paths
        .iter()
        .map(|path| hash_file(path))
        .collect::<Result<Vec<_>, _>>()?;
    if review.replay_summary_sha256 != hash_file(replay_summary_path)?
        || review.frame_sha256 != frame_sha256
        || review.frame_durations_ms != frame_durations_ms
    {
        return Err(AnimationHumanReviewError::ClosureMismatch);
    }
    Ok(())
}

pub fn review_is_approved(review: &AnimationHumanReviewV1) -> bool {
    validate_animation_review_shape(review).is_ok()
        && review.status == AnimationReviewStatusV1::Approved
        && review
            .checks
            .values()
            .into_iter()
            .all(|value| value == Some(true))
        && review.reviewed_at.is_some()
}

pub fn approve_animation_review(
    pending: &AnimationHumanReviewV1,
    review_note: impl Into<String>,
    reviewed_at: DateTime<Utc>,
) -> Result<AnimationHumanReviewV1, AnimationHumanReviewError> {
    validate_animation_review_shape(pending)?;
    if pending.status != AnimationReviewStatusV1::Pending {
        return Err(AnimationHumanReviewError::InvalidReview);
    }
    let mut approved = pending.clone();
    approved.status = AnimationReviewStatusV1::Approved;
    approved.checks = AnimationHumanReviewChecksV1 {
        motion_acceptable: Some(true),
        placement_acceptable: Some(true),
        edge_cleanup_acceptable: Some(true),
        foot_alternation_acceptable: Some(true),
        pivot_acceptable: Some(true),
        collision_acceptable: Some(true),
    };
    approved.review_note = review_note.into();
    approved.reviewed_at = Some(reviewed_at);
    validate_animation_review_shape(&approved)?;
    Ok(approved)
}

pub fn validate_animation_review_shape(
    review: &AnimationHumanReviewV1,
) -> Result<(), AnimationHumanReviewError> {
    let hashes_valid = [
        review.source_video_sha256.as_str(),
        review.prompt_sha256.as_str(),
        review.replay_summary_sha256.as_str(),
    ]
    .into_iter()
    .chain(review.frame_sha256.iter().map(String::as_str))
    .all(is_sha256);
    let frame_count_valid = matches!(review.frame_sha256.len(), 8 | 10 | 12 | 16 | 24)
        && review.frame_durations_ms.len() == review.frame_sha256.len()
        && !review.frame_durations_ms.contains(&0);
    let check_values = review.checks.values();
    let status_valid = match review.status {
        AnimationReviewStatusV1::Pending => {
            check_values.into_iter().all(|value| value.is_none()) && review.reviewed_at.is_none()
        }
        AnimationReviewStatusV1::Approved => {
            check_values.into_iter().all(|value| value == Some(true))
                && review.reviewed_at.is_some()
        }
        AnimationReviewStatusV1::Rejected => {
            check_values.into_iter().any(|value| value == Some(false))
                && review.reviewed_at.is_some()
        }
    };
    if review.schema_version != "1"
        || review.profile != ANIMATION_HUMAN_REVIEW_PROFILE
        || review.animation.trim().is_empty()
        || review.reviewer != "human"
        || review.review_note.trim().is_empty()
        || !hashes_valid
        || !frame_count_valid
        || !status_valid
    {
        return Err(AnimationHumanReviewError::InvalidReview);
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_review_is_hash_bound_and_not_approved() {
        let temp = tempfile::tempdir().unwrap();
        let replay = temp.path().join("replay-summary.json");
        let frame = temp.path().join("00.png");
        fs::write(&replay, b"replay").unwrap();
        fs::write(&frame, b"frame").unwrap();
        let frames = vec![frame; 8];
        let review = build_pending_animation_review(
            "walk_right",
            "a".repeat(64),
            "b".repeat(64),
            &replay,
            &frames,
            vec![83; 8],
        )
        .unwrap();
        validate_animation_review_closure(&review, &replay, &frames, &[83; 8]).unwrap();
        assert_eq!(review.status, AnimationReviewStatusV1::Pending);
        assert!(!review_is_approved(&review));
    }

    #[test]
    fn review_closure_rejects_changed_frame_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let replay = temp.path().join("replay-summary.json");
        let frame = temp.path().join("00.png");
        fs::write(&replay, b"replay").unwrap();
        fs::write(&frame, b"frame").unwrap();
        let frames = vec![frame.clone(); 8];
        let review = build_pending_animation_review(
            "walk_right",
            "a".repeat(64),
            "b".repeat(64),
            &replay,
            &frames,
            vec![83; 8],
        )
        .unwrap();
        fs::write(&frame, b"changed").unwrap();
        assert!(matches!(
            validate_animation_review_closure(&review, &replay, &frames, &[83; 8]),
            Err(AnimationHumanReviewError::ClosureMismatch)
        ));
    }

    #[test]
    fn explicit_approval_sets_all_six_checks_without_changing_hashes() {
        let temp = tempfile::tempdir().unwrap();
        let replay = temp.path().join("replay-summary.json");
        let frame = temp.path().join("00.png");
        fs::write(&replay, b"replay").unwrap();
        fs::write(&frame, b"frame").unwrap();
        let frames = vec![frame; 8];
        let pending = build_pending_animation_review(
            "walk_right",
            "a".repeat(64),
            "b".repeat(64),
            &replay,
            &frames,
            vec![83; 8],
        )
        .unwrap();
        let reviewed_at = Utc::now();
        let approved =
            approve_animation_review(&pending, "human approved all six checks", reviewed_at)
                .unwrap();

        assert!(review_is_approved(&approved));
        assert_eq!(approved.frame_sha256, pending.frame_sha256);
        assert_eq!(
            approved.replay_summary_sha256,
            pending.replay_summary_sha256
        );
        assert_eq!(approved.reviewed_at, Some(reviewed_at));
    }
}
