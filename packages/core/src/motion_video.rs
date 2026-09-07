use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::asset_project::{hash_file, AssetProjectError};
use crate::video::{probe_video, ProbeVideoParams, VideoError, VideoProbe};

pub const MOTION_DRIVER_LOCK_PROFILE: &str = "motion-driver-lock@1.0.0";
pub const IDLE_MOTION_DRIVER_LOCK_PROFILE: &str = "idle-motion-driver-lock@1.0.0";
pub const VIDEO_CANDIDATE_LOCK_PROFILE: &str = "video-candidate-lock@1.0.0";
pub const MOTION_DRIVER_LOCK_FILE: &str = "motion-driver-lock.json";
pub const VIDEO_CANDIDATE_LOCK_FILE: &str = "video-candidate-lock.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionDriverSourceKindV1 {
    ProceduralGaitSpec,
    DrivingVideo,
    PoseVideo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdleMotionDriverSourceKindV1 {
    IdentityAnchor,
    DrivingVideo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoAcquisitionKindV1 {
    Browser,
    Api,
    ExternalImport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MotionPhaseMarkerV1 {
    pub role: String,
    pub normalized_time: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdleMotionPhaseMarkerV1 {
    pub role: String,
    pub normalized_time: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MotionDriverLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub id: String,
    pub action: String,
    pub direction: String,
    pub source_kind: MotionDriverSourceKindV1,
    pub identity_anchor_path: PathBuf,
    pub identity_anchor_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_source_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_source_sha256: Option<String>,
    pub requested_duration_ms: u64,
    pub expected_cycles_minimum: usize,
    pub camera_fixed: bool,
    pub root_fixed: bool,
    pub ordinary_walk: bool,
    pub phase_markers: Vec<MotionPhaseMarkerV1>,
    pub maximum_swing_foot_clearance_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdleMotionDriverLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub id: String,
    pub action: String,
    pub direction: String,
    pub source_kind: IdleMotionDriverSourceKindV1,
    pub identity_anchor_path: PathBuf,
    pub identity_anchor_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_source_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub driver_source_sha256: Option<String>,
    pub requested_duration_ms: u64,
    pub expected_cycles_minimum: usize,
    pub camera_fixed: bool,
    pub root_fixed: bool,
    pub feet_planted: bool,
    pub identity_locked: bool,
    pub breathing_loop: bool,
    pub phase_markers: Vec<IdleMotionPhaseMarkerV1>,
    pub maximum_torso_vertical_excursion_ratio: f32,
    pub maximum_limb_displacement_ratio: f32,
    pub background_hex: String,
    pub aspect_ratio: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserQuotaSnapshotV1 {
    pub unit: String,
    pub before: f64,
    pub after: f64,
    pub free_generation_used: bool,
    pub reward_claimed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoCandidateLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub id: String,
    pub acquisition: VideoAcquisitionKindV1,
    pub provider: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_task_id: Option<String>,
    pub provider_request_count: usize,
    pub imported_without_provider_request: bool,
    pub motion_driver_lock_path: PathBuf,
    pub motion_driver_lock_sha256: String,
    pub prompt_path: PathBuf,
    pub prompt_sha256: String,
    pub source_video_path: PathBuf,
    pub source_video_sha256: String,
    pub probe: VideoProbe,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quota: Option<BrowserQuotaSnapshotV1>,
}

#[derive(Debug, Error)]
pub enum MotionVideoError {
    #[error("asset error: {0}")]
    Asset(#[from] AssetProjectError),
    #[error("video error: {0}")]
    Video(#[from] VideoError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid motion video contract: {0}")]
    Invalid(String),
}

#[derive(Debug, Clone)]
pub struct CreateMotionDriverLockRequestV1 {
    pub output_path: PathBuf,
    pub id: String,
    pub action: String,
    pub direction: String,
    pub source_kind: MotionDriverSourceKindV1,
    pub identity_anchor_path: PathBuf,
    pub driver_source_path: Option<PathBuf>,
    pub requested_duration_ms: u64,
    pub expected_cycles_minimum: usize,
    pub maximum_swing_foot_clearance_ratio: f32,
}

#[derive(Debug, Clone)]
pub struct CreateIdleMotionDriverLockRequestV1 {
    pub output_path: PathBuf,
    pub id: String,
    pub action: String,
    pub direction: String,
    pub source_kind: IdleMotionDriverSourceKindV1,
    pub identity_anchor_path: PathBuf,
    pub driver_source_path: Option<PathBuf>,
    pub requested_duration_ms: u64,
    pub expected_cycles_minimum: usize,
    pub maximum_torso_vertical_excursion_ratio: f32,
    pub maximum_limb_displacement_ratio: f32,
    pub background_hex: String,
    pub aspect_ratio: String,
}

#[derive(Debug, Clone)]
pub struct ImportVideoCandidateRequestV1 {
    pub output_directory: PathBuf,
    pub id: String,
    pub acquisition: VideoAcquisitionKindV1,
    pub provider: String,
    pub model: String,
    pub provider_task_id: Option<String>,
    pub provider_request_count: usize,
    pub imported_without_provider_request: bool,
    pub motion_driver_lock_path: PathBuf,
    pub prompt_path: PathBuf,
    pub source_video_path: PathBuf,
    pub quota: Option<BrowserQuotaSnapshotV1>,
}

pub fn create_motion_driver_lock(
    request: &CreateMotionDriverLockRequestV1,
) -> Result<MotionDriverLockV1, MotionVideoError> {
    if request.output_path.exists() {
        return Err(MotionVideoError::Invalid(format!(
            "refusing to overwrite {}",
            request.output_path.display()
        )));
    }
    let identity_anchor_path = request.identity_anchor_path.canonicalize()?;
    let driver_source_path = request
        .driver_source_path
        .as_ref()
        .map(|path| path.canonicalize())
        .transpose()?;
    let lock = MotionDriverLockV1 {
        schema_version: "1".into(),
        profile: MOTION_DRIVER_LOCK_PROFILE.into(),
        id: request.id.clone(),
        action: request.action.clone(),
        direction: request.direction.clone(),
        source_kind: request.source_kind,
        identity_anchor_sha256: hash_file(&identity_anchor_path)?,
        identity_anchor_path,
        driver_source_sha256: driver_source_path.as_deref().map(hash_file).transpose()?,
        driver_source_path,
        requested_duration_ms: request.requested_duration_ms,
        expected_cycles_minimum: request.expected_cycles_minimum,
        camera_fixed: true,
        root_fixed: true,
        ordinary_walk: true,
        phase_markers: default_walk_phase_markers(),
        maximum_swing_foot_clearance_ratio: request.maximum_swing_foot_clearance_ratio,
    };
    validate_motion_driver_lock(&lock)?;
    if let Some(parent) = request.output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&request.output_path, serde_json::to_vec_pretty(&lock)?)?;
    Ok(lock)
}

pub fn create_idle_motion_driver_lock(
    request: &CreateIdleMotionDriverLockRequestV1,
) -> Result<IdleMotionDriverLockV1, MotionVideoError> {
    if request.output_path.exists() {
        return Err(MotionVideoError::Invalid(format!(
            "refusing to overwrite {}",
            request.output_path.display()
        )));
    }
    let identity_anchor_path = request.identity_anchor_path.canonicalize()?;
    let driver_source_path = request
        .driver_source_path
        .as_ref()
        .map(|path| path.canonicalize())
        .transpose()?;
    let lock = IdleMotionDriverLockV1 {
        schema_version: "1".into(),
        profile: IDLE_MOTION_DRIVER_LOCK_PROFILE.into(),
        id: request.id.clone(),
        action: request.action.clone(),
        direction: request.direction.clone(),
        source_kind: request.source_kind,
        identity_anchor_sha256: hash_file(&identity_anchor_path)?,
        identity_anchor_path,
        driver_source_sha256: driver_source_path.as_deref().map(hash_file).transpose()?,
        driver_source_path,
        requested_duration_ms: request.requested_duration_ms,
        expected_cycles_minimum: request.expected_cycles_minimum,
        camera_fixed: true,
        root_fixed: true,
        feet_planted: true,
        identity_locked: true,
        breathing_loop: true,
        phase_markers: default_idle_phase_markers(),
        maximum_torso_vertical_excursion_ratio: request.maximum_torso_vertical_excursion_ratio,
        maximum_limb_displacement_ratio: request.maximum_limb_displacement_ratio,
        background_hex: request.background_hex.to_uppercase(),
        aspect_ratio: request.aspect_ratio.clone(),
    };
    validate_idle_motion_driver_lock(&lock)?;
    if let Some(parent) = request.output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&request.output_path, serde_json::to_vec_pretty(&lock)?)?;
    Ok(lock)
}

pub fn import_video_candidate(
    request: &ImportVideoCandidateRequestV1,
) -> Result<VideoCandidateLockV1, MotionVideoError> {
    if request.output_directory.exists() && request.output_directory.read_dir()?.next().is_some() {
        return Err(MotionVideoError::Invalid(format!(
            "refusing to overwrite non-empty {}",
            request.output_directory.display()
        )));
    }
    let driver_lock_path = request.motion_driver_lock_path.canonicalize()?;
    let driver_lock: MotionDriverLockV1 = serde_json::from_slice(&fs::read(&driver_lock_path)?)?;
    validate_motion_driver_lock(&driver_lock)?;
    let prompt_path = request.prompt_path.canonicalize()?;
    let source_video = request.source_video_path.canonicalize()?;
    fs::create_dir_all(&request.output_directory)?;
    let canonical_video = request.output_directory.join("source-video.mp4");
    fs::copy(&source_video, &canonical_video)?;
    let canonical_video = canonical_video.canonicalize()?;
    let lock = VideoCandidateLockV1 {
        schema_version: "1".into(),
        profile: VIDEO_CANDIDATE_LOCK_PROFILE.into(),
        id: request.id.clone(),
        acquisition: request.acquisition,
        provider: request.provider.clone(),
        model: request.model.clone(),
        provider_task_id: request.provider_task_id.clone(),
        provider_request_count: request.provider_request_count,
        imported_without_provider_request: request.imported_without_provider_request,
        motion_driver_lock_sha256: hash_file(&driver_lock_path)?,
        motion_driver_lock_path: driver_lock_path,
        prompt_sha256: hash_file(&prompt_path)?,
        prompt_path,
        source_video_sha256: hash_file(&canonical_video)?,
        probe: probe_video(&ProbeVideoParams {
            input_path: canonical_video.clone(),
            configured_ffprobe_path: None,
            bundled_resource_path: None,
        })?,
        source_video_path: canonical_video,
        quota: request.quota.clone(),
    };
    validate_video_candidate_lock(&lock)?;
    fs::write(
        request.output_directory.join(VIDEO_CANDIDATE_LOCK_FILE),
        serde_json::to_vec_pretty(&lock)?,
    )?;
    Ok(lock)
}

pub fn validate_motion_driver_lock(lock: &MotionDriverLockV1) -> Result<(), MotionVideoError> {
    let phases = lock
        .phase_markers
        .iter()
        .map(|phase| phase.role.as_str())
        .collect::<Vec<_>>();
    if lock.schema_version != "1"
        || lock.profile != MOTION_DRIVER_LOCK_PROFILE
        || lock.id.trim().is_empty()
        || lock.action.trim().is_empty()
        || lock.direction.trim().is_empty()
        || !lock.identity_anchor_path.is_file()
        || hash_file(&lock.identity_anchor_path)? != lock.identity_anchor_sha256
        || lock.requested_duration_ms < 1000
        || lock.expected_cycles_minimum < 1
        || !lock.camera_fixed
        || !lock.root_fixed
        || !lock.ordinary_walk
        || phases
            != [
                "contact_a",
                "passing_a",
                "contact_b",
                "passing_b",
                "closure",
            ]
        || lock
            .phase_markers
            .windows(2)
            .any(|pair| pair[0].normalized_time >= pair[1].normalized_time)
        || lock
            .phase_markers
            .first()
            .map(|phase| phase.normalized_time)
            != Some(0.0)
        || lock.phase_markers.last().map(|phase| phase.normalized_time) != Some(1.0)
        || !(0.02..=0.18).contains(&lock.maximum_swing_foot_clearance_ratio)
    {
        return Err(MotionVideoError::Invalid(
            "motion driver lock failed validation".into(),
        ));
    }
    match lock.source_kind {
        MotionDriverSourceKindV1::ProceduralGaitSpec => {
            if lock.driver_source_path.is_some() || lock.driver_source_sha256.is_some() {
                return Err(MotionVideoError::Invalid(
                    "procedural gait spec must not claim a driver file".into(),
                ));
            }
        }
        MotionDriverSourceKindV1::DrivingVideo | MotionDriverSourceKindV1::PoseVideo => {
            let path = lock.driver_source_path.as_deref().ok_or_else(|| {
                MotionVideoError::Invalid("driver-backed lock requires driverSourcePath".into())
            })?;
            let sha = lock.driver_source_sha256.as_deref().ok_or_else(|| {
                MotionVideoError::Invalid("driver-backed lock requires driverSourceSha256".into())
            })?;
            if !path.is_file() || hash_file(path)? != sha {
                return Err(MotionVideoError::Invalid(
                    "driver source hash mismatch".into(),
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_idle_motion_driver_lock(
    lock: &IdleMotionDriverLockV1,
) -> Result<(), MotionVideoError> {
    let phases = lock
        .phase_markers
        .iter()
        .map(|phase| phase.role.as_str())
        .collect::<Vec<_>>();
    let expected_action = format!("idle_{}", lock.direction);
    if lock.schema_version != "1"
        || lock.profile != IDLE_MOTION_DRIVER_LOCK_PROFILE
        || lock.id.trim().is_empty()
        || lock.action != expected_action
        || !matches!(lock.direction.as_str(), "right" | "left" | "up" | "down")
        || !lock.identity_anchor_path.is_file()
        || hash_file(&lock.identity_anchor_path)? != lock.identity_anchor_sha256
        || !(2000..=10_000).contains(&lock.requested_duration_ms)
        || !(1..=4).contains(&lock.expected_cycles_minimum)
        || !lock.camera_fixed
        || !lock.root_fixed
        || !lock.feet_planted
        || !lock.identity_locked
        || !lock.breathing_loop
        || phases
            != [
                "neutral_start",
                "inhale_peak",
                "neutral_mid",
                "exhale_peak",
                "closure",
            ]
        || lock
            .phase_markers
            .windows(2)
            .any(|pair| pair[0].normalized_time >= pair[1].normalized_time)
        || lock
            .phase_markers
            .first()
            .map(|phase| phase.normalized_time)
            != Some(0.0)
        || lock.phase_markers.last().map(|phase| phase.normalized_time) != Some(1.0)
        || !(0.001..=0.03).contains(&lock.maximum_torso_vertical_excursion_ratio)
        || !(0.0..=0.02).contains(&lock.maximum_limb_displacement_ratio)
        || lock.background_hex != "#FF00FF"
        || lock.aspect_ratio != "1:1"
    {
        return Err(MotionVideoError::Invalid(
            "idle motion driver lock failed validation".into(),
        ));
    }
    match lock.source_kind {
        IdleMotionDriverSourceKindV1::IdentityAnchor => {
            if lock.driver_source_path.is_some() || lock.driver_source_sha256.is_some() {
                return Err(MotionVideoError::Invalid(
                    "identity-anchor idle must not claim a driver file".into(),
                ));
            }
        }
        IdleMotionDriverSourceKindV1::DrivingVideo => {
            let path = lock.driver_source_path.as_deref().ok_or_else(|| {
                MotionVideoError::Invalid("driver-backed idle requires driverSourcePath".into())
            })?;
            let sha = lock.driver_source_sha256.as_deref().ok_or_else(|| {
                MotionVideoError::Invalid("driver-backed idle requires driverSourceSha256".into())
            })?;
            if !path.is_file() || hash_file(path)? != sha {
                return Err(MotionVideoError::Invalid(
                    "idle driver source hash mismatch".into(),
                ));
            }
        }
    }
    Ok(())
}

pub fn validate_video_candidate_lock(lock: &VideoCandidateLockV1) -> Result<(), MotionVideoError> {
    if lock.schema_version != "1"
        || lock.profile != VIDEO_CANDIDATE_LOCK_PROFILE
        || lock.id.trim().is_empty()
        || lock.provider.trim().is_empty()
        || lock.model.trim().is_empty()
        || !lock.source_video_path.is_file()
        || hash_file(&lock.source_video_path)? != lock.source_video_sha256
        || !lock.motion_driver_lock_path.is_file()
        || hash_file(&lock.motion_driver_lock_path)? != lock.motion_driver_lock_sha256
        || !lock.prompt_path.is_file()
        || hash_file(&lock.prompt_path)? != lock.prompt_sha256
        || lock.probe.width == 0
        || lock.probe.height == 0
        || lock.probe.fps <= 0.0
        || lock.probe.duration_seconds <= 0.0
    {
        return Err(MotionVideoError::Invalid(
            "video candidate lock failed validation".into(),
        ));
    }
    Ok(())
}

fn default_walk_phase_markers() -> Vec<MotionPhaseMarkerV1> {
    [
        ("contact_a", 0.0),
        ("passing_a", 0.25),
        ("contact_b", 0.50),
        ("passing_b", 0.75),
        ("closure", 1.0),
    ]
    .into_iter()
    .map(|(role, normalized_time)| MotionPhaseMarkerV1 {
        role: role.into(),
        normalized_time,
    })
    .collect()
}

fn default_idle_phase_markers() -> Vec<IdleMotionPhaseMarkerV1> {
    [
        ("neutral_start", 0.0),
        ("inhale_peak", 0.25),
        ("neutral_mid", 0.50),
        ("exhale_peak", 0.75),
        ("closure", 1.0),
    ]
    .into_iter()
    .map(|(role, normalized_time)| IdleMotionPhaseMarkerV1 {
        role: role.into(),
        normalized_time,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_walk_phases_are_strictly_ordered() {
        let phases = default_walk_phase_markers();
        assert_eq!(phases.len(), 5);
        assert!(phases
            .windows(2)
            .all(|pair| pair[0].normalized_time < pair[1].normalized_time));
        assert_eq!(phases.first().unwrap().role, "contact_a");
        assert_eq!(phases.last().unwrap().role, "closure");
    }

    #[test]
    fn default_idle_phases_are_strictly_ordered() {
        let phases = default_idle_phase_markers();
        assert_eq!(phases.len(), 5);
        assert!(phases
            .windows(2)
            .all(|pair| pair[0].normalized_time < pair[1].normalized_time));
        assert_eq!(phases.first().unwrap().role, "neutral_start");
        assert_eq!(phases.last().unwrap().role, "closure");
    }

    #[test]
    fn creates_hash_bound_identity_anchor_idle_lock() {
        let directory = tempfile::tempdir().unwrap();
        let anchor = directory.path().join("right_idle.png");
        fs::write(&anchor, b"locked-right-idle").unwrap();
        let output = directory.path().join("idle-motion-driver-lock.json");
        let lock = create_idle_motion_driver_lock(&CreateIdleMotionDriverLockRequestV1 {
            output_path: output.clone(),
            id: "idle-right-v1".into(),
            action: "idle_right".into(),
            direction: "right".into(),
            source_kind: IdleMotionDriverSourceKindV1::IdentityAnchor,
            identity_anchor_path: anchor,
            driver_source_path: None,
            requested_duration_ms: 5000,
            expected_cycles_minimum: 2,
            maximum_torso_vertical_excursion_ratio: 0.02,
            maximum_limb_displacement_ratio: 0.01,
            background_hex: "#ff00ff".into(),
            aspect_ratio: "1:1".into(),
        })
        .unwrap();
        assert!(output.is_file());
        assert_eq!(lock.action, "idle_right");
        assert_eq!(lock.background_hex, "#FF00FF");
        assert!(validate_idle_motion_driver_lock(&lock).is_ok());
    }
}
