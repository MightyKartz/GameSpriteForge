use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use chrono::{Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

use super::types::{
    character_workflow_catalog, AssetInput, AutomationOperation, AutomationPlan,
    GenerateCharacterPackRequest, GenerateStaticAssetSetRequest, ImportDirectionGridRequestV1,
    PlanState, PrepareCharacterPackRequest, PreparedPlan, AUTOMATION_SCHEMA_VERSION,
    EXTERNAL_KEYFRAME_WORKFLOW_ID, EXTERNAL_KEYFRAME_WORKFLOW_VERSION,
};
use crate::asset_project::{
    hash_file, read_project, read_style_lock, resolve_relative, CharacterEquipmentKindV1,
    StyleSpecV1,
};
use crate::catalog::{read_project_catalog, CatalogReviewStatusV1, PROJECT_CATALOG_RELATIVE};
use crate::character_camera::CharacterCameraProfileV1;
use crate::character_cycle::{CharacterScaleLockV1, CHARACTER_SCALE_LOCK_PROFILE};
use crate::character_direction_motion::{DirectionMotionGenerationStageV1, DirectionMotionLockV1};
use crate::character_grid::{
    is_direction_grid_lock_profile, DirectionGridApprovalV1, DirectionGridLockV1,
    GRID_APPROVAL_PROFILE, GRID_WORKFLOW,
};
use crate::collection::{
    collection_requires_provider, read_collection_lock, read_collection_spec, CollectionGrounding,
};
use crate::export::CharacterMirrorPolicyV1;
use crate::game_art::{
    compute_build_plan, compute_project_diff, project_source_sha256, GameArtManifestV1,
    ProjectBuildStateV1, ProviderCapabilityInput,
};
use crate::geometry::PortraitFramingProfileV1;
use crate::job::{JobLifecycleState, JobRecord, RepairContext};
use crate::portrait::{
    validate_portrait_base_approval, PortraitGenerationPhaseV1, PortraitNeutralReferencePolicyV1,
    PORTRAIT_BASE_APPROVAL_FILE, PORTRAIT_BASE_LOCK_FILE,
};
use crate::quality::SOURCE_CYCLE_SAMPLING_PROFILE;
use crate::subject::{read_subject_lock, read_subject_spec};
use crate::video::{probe_video, ProbeVideoParams};
use crate::world::{
    read_environment_lock, validate_building_spec, validate_environment_spec, validate_map_spec,
    validate_terrain_spec, EnvironmentSpecV1, MapSpecV1,
};

const APP_SUPPORT_DIR: &str = "Game Sprite Forge";
const AUTOMATION_DIR: &str = "automation";
const PLANS_DIR: &str = "plans";
const OWNERSHIP_MARKER: &str = ".forge-owned.json";
pub const PLAN_TTL_MINUTES: i64 = 15;

#[derive(Debug, Error)]
pub enum PlanStoreError {
    #[error("could not locate the user application support directory")]
    AppSupportDirUnavailable,
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("plan does not exist: {0}")]
    NotFound(String),
    #[error("plan token has already been used: {0}")]
    AlreadyUsed(String),
    #[error("plan token expired at {0}")]
    Expired(chrono::DateTime<Utc>),
    #[error("input changed after plan creation")]
    InputChanged,
    #[error("io error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct PlanStore {
    root: PathBuf,
}

impl PlanStore {
    pub fn default_app_store() -> Result<Self, PlanStoreError> {
        let root = dirs_next::config_dir()
            .ok_or(PlanStoreError::AppSupportDirUnavailable)?
            .join(APP_SUPPORT_DIR)
            .join(AUTOMATION_DIR)
            .join(PLANS_DIR);
        Self::new(root)
    }

    pub fn new(root: impl Into<PathBuf>) -> Result<Self, PlanStoreError> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|source| PlanStoreError::Io {
            path: root.clone(),
            source,
        })?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn prepare(&self, operation: AutomationOperation) -> Result<PreparedPlan, PlanStoreError> {
        self.prepare_with_repair_context(operation, None)
    }

    pub(crate) fn prepare_with_repair_context(
        &self,
        operation: AutomationOperation,
        repair: Option<RepairContext>,
    ) -> Result<PreparedPlan, PlanStoreError> {
        validate_operation(&operation)?;
        let input_fingerprint = fingerprint_operation_inputs(&operation)?;
        let recipe_hash = hash_serializable(&operation)?;
        let now = Utc::now();
        let mut effects = describe_effects(&operation);
        if let Some(context) = &repair {
            effects.insert(
                0,
                format!(
                    "create repair attempt {} from source job {} without changing the source job",
                    context.attempt, context.source_job_id
                ),
            );
            effects.extend(context.changes.iter().map(|change| {
                format!(
                    "change {} {} from {} to {} ({})",
                    change.scope, change.parameter, change.before, change.after, change.reason
                )
            }));
            effects.push("write repair-comparison.json with before/after quality evidence".into());
        }
        let estimate = estimate_operation(&operation);
        let plan = AutomationPlan {
            schema_version: AUTOMATION_SCHEMA_VERSION.to_string(),
            token: Uuid::new_v4().to_string(),
            created_at: now,
            expires_at: now + Duration::minutes(PLAN_TTL_MINUTES),
            state: PlanState::Pending,
            input_fingerprint,
            recipe_hash,
            repair: repair.clone(),
            estimate: estimate.clone(),
            effects,
            operation,
        };
        self.write_pending(&plan)?;
        Ok(PreparedPlan {
            token: plan.token,
            expires_at: plan.expires_at,
            input_fingerprint: plan.input_fingerprint,
            recipe_hash: plan.recipe_hash,
            effects: plan.effects,
            estimate,
            repair,
        })
    }

    pub fn claim(&self, token: &str) -> Result<AutomationPlan, PlanStoreError> {
        validate_token(token)?;
        let pending = self.pending_path(token);
        let claimed = self.claimed_path(token);
        if claimed.exists() {
            return Err(PlanStoreError::AlreadyUsed(token.to_string()));
        }
        if !pending.exists() {
            return Err(PlanStoreError::NotFound(token.to_string()));
        }
        let mut plan: AutomationPlan = read_json(&pending)?;
        if Utc::now() > plan.expires_at {
            fs::rename(&pending, &claimed).map_err(|source| PlanStoreError::Io {
                path: pending.clone(),
                source,
            })?;
            plan.state = PlanState::Expired;
            write_json_atomic(&claimed, &plan)?;
            return Err(PlanStoreError::Expired(plan.expires_at));
        }
        validate_plan_integrity(&plan)?;
        let current_fingerprint = fingerprint_operation_inputs(&plan.operation)?;
        if current_fingerprint != plan.input_fingerprint {
            return Err(PlanStoreError::InputChanged);
        }
        fs::rename(&pending, &claimed).map_err(|source| {
            if claimed.exists() {
                PlanStoreError::AlreadyUsed(token.to_string())
            } else {
                PlanStoreError::Io {
                    path: pending.clone(),
                    source,
                }
            }
        })?;
        // Close the verification→claim window. If a local input changed in
        // between, restore the pending token rather than consuming it.
        if let Err(error) = validate_plan_integrity(&plan) {
            fs::rename(&claimed, &pending).map_err(|source| PlanStoreError::Io {
                path: claimed.clone(),
                source,
            })?;
            return Err(error);
        }
        let post_claim_fingerprint = fingerprint_operation_inputs(&plan.operation)?;
        if post_claim_fingerprint != plan.input_fingerprint {
            fs::rename(&claimed, &pending).map_err(|source| PlanStoreError::Io {
                path: claimed.clone(),
                source,
            })?;
            return Err(PlanStoreError::InputChanged);
        }
        plan.state = PlanState::Claimed;
        write_json_atomic(&claimed, &plan)?;
        Ok(plan)
    }

    /// Read and validate a pending plan without consuming its single-use
    /// token. The CLI uses this for execution preflight (cost acceptance,
    /// feature availability and credential resolution) before `claim`.
    pub fn inspect_pending(&self, token: &str) -> Result<AutomationPlan, PlanStoreError> {
        validate_token(token)?;
        let pending = self.pending_path(token);
        if self.claimed_path(token).exists() {
            return Err(PlanStoreError::AlreadyUsed(token.to_string()));
        }
        if !pending.exists() {
            return Err(PlanStoreError::NotFound(token.to_string()));
        }
        let plan: AutomationPlan = read_json(&pending)?;
        if Utc::now() > plan.expires_at {
            return Err(PlanStoreError::Expired(plan.expires_at));
        }
        validate_plan_integrity(&plan)?;
        Ok(plan)
    }

    fn write_pending(&self, plan: &AutomationPlan) -> Result<(), PlanStoreError> {
        write_json_atomic(&self.pending_path(&plan.token), plan)
    }

    fn pending_path(&self, token: &str) -> PathBuf {
        self.root.join(format!("{token}.pending.json"))
    }

    fn claimed_path(&self, token: &str) -> PathBuf {
        self.root.join(format!("{token}.claimed.json"))
    }
}

fn validate_plan_integrity(plan: &AutomationPlan) -> Result<(), PlanStoreError> {
    if plan.schema_version != AUTOMATION_SCHEMA_VERSION || plan.state != PlanState::Pending {
        return Err(PlanStoreError::InputChanged);
    }
    validate_operation(&plan.operation)?;
    if hash_serializable(&plan.operation)? != plan.recipe_hash
        || estimate_operation(&plan.operation) != plan.estimate
    {
        return Err(PlanStoreError::InputChanged);
    }
    let mut effects = describe_effects(&plan.operation);
    if let Some(context) = &plan.repair {
        effects.insert(
            0,
            format!(
                "create repair attempt {} from source job {} without changing the source job",
                context.attempt, context.source_job_id
            ),
        );
        effects.extend(context.changes.iter().map(|change| {
            format!(
                "change {} {} from {} to {} ({})",
                change.scope, change.parameter, change.before, change.after, change.reason
            )
        }));
        effects.push("write repair-comparison.json with before/after quality evidence".into());
    }
    if effects != plan.effects {
        return Err(PlanStoreError::InputChanged);
    }
    Ok(())
}

fn estimate_operation(operation: &AutomationOperation) -> super::types::PlanEstimateV1 {
    use super::types::PlanEstimateV1;
    match operation {
        AutomationOperation::CreateStyleLock(request) => PlanEstimateV1 {
            provider_request_estimate: 1,
            maximum_provider_requests: 1,
            provider_id: Some(request.provider_id.clone()),
            profile_id: Some(request.profile_id.clone()),
            ..Default::default()
        },
        AutomationOperation::CreateSubjectLock(request) => {
            let count = u32::from(!request.is_local_import());
            PlanEstimateV1 {
                provider_request_estimate: count,
                maximum_provider_requests: count,
                provider_id: (count > 0).then(|| request.provider_id.clone()),
                profile_id: (count > 0).then(|| request.profile_id.clone()),
                ..Default::default()
            }
        }
        AutomationOperation::CreateCollectionLock(request) => {
            let count = u32::from(collection_requires_provider(&request.spec_path).unwrap_or(true));
            PlanEstimateV1 {
                provider_request_estimate: count,
                maximum_provider_requests: count,
                provider_id: (count > 0).then(|| request.provider_id.clone()),
                profile_id: (count > 0).then(|| request.profile_id.clone()),
                ..Default::default()
            }
        }
        AutomationOperation::PrepareCharacterPack(request) => PlanEstimateV1 {
            provider_request_estimate: 0,
            maximum_provider_requests: 0,
            workflow: Some(format!(
                "{}@{}",
                request.workflow.id, request.workflow.version
            )),
            model: Some("external_import".into()),
            ..Default::default()
        },
        AutomationOperation::GenerateCharacterPack(request) => {
            let frame_image_workflow = matches!(
                request.workflow.id.as_str(),
                "topdown-keyframes" | "topdown-keyposes"
            );
            let sprite_sheet = request.workflow.id == "topdown-spritesheet";
            let locked_frames = request.workflow.id == "topdown-frames";
            let locked_video = matches!(
                request.workflow.id.as_str(),
                "topdown-video-locked" | "topdown-video-cycle"
            );
            let direction_pose_chain = request.workflow.id == "topdown-direction-poses"
                && request.workflow.version == "7.0.0";
            let direction_motion = request.workflow.id == "topdown-direction-motion"
                && request.workflow.version == "8.0.0";
            let topdown_cycle = request.workflow.id == "topdown-cycle"
                && matches!(request.workflow.version.as_str(), "10.0.0" | "10.1.0");
            let grid_generation =
                request.workflow.id == "topdown-grid" && request.workflow.version == "9.0.0";
            let image2_direction_grid_probe = is_xai_image2_direction_grid_probe(request);
            let front_authoritative_direction_grid = is_front_authoritative_direction_grid(request);
            let grid_keyframes = request.workflow.id == "topdown-grid"
                && matches!(
                    request.workflow.version.as_str(),
                    "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
                );
            let video_workflow = matches!(
                request.workflow.id.as_str(),
                "topdown"
                    | "topdown-video"
                    | "topdown-video-locked"
                    | "topdown-video-cycle"
                    | "topdown-direction-poses"
                    | "topdown-direction-motion"
                    | "topdown-cycle"
            );
            let frames_per_animation =
                if request.workflow.id == "topdown-keyposes" || sprite_sheet || locked_frames {
                    4
                } else {
                    8
                };
            let local_only = !request.retry_animations.is_empty()
                && request.retry_animations.iter().all(|animation| {
                    request.retry_stages.get(animation).is_some_and(|stage| {
                        matches!(
                            stage,
                            super::types::CharacterRetryStage::Loop
                                | super::types::CharacterRetryStage::Matting
                                | super::types::CharacterRetryStage::Consistency
                        )
                    })
                });
            let selected_frames = request.retry_frames.values().map(Vec::len).sum::<usize>() as u32;
            let validation_request_count = if topdown_cycle {
                request.validation_animations.len() as u32
            } else if direction_motion {
                request
                    .validation_animations
                    .iter()
                    .map(|animation| match animation.as_str() {
                        "walk_down" => 3,
                        "walk_up" | "walk_right" | "walk_left" => 4,
                        "idle_down" => 1,
                        "idle_up" | "idle_right" | "idle_left" => 2,
                        _ => 0,
                    })
                    .sum()
            } else if direction_pose_chain {
                request
                    .validation_animations
                    .iter()
                    .map(|animation| match animation.as_str() {
                        "idle" => 0,
                        "walk_down" => 2,
                        "walk_up" | "walk_right" => 3,
                        _ => 0,
                    })
                    .sum()
            } else if sprite_sheet {
                request.validation_animations.len() as u32
            } else if locked_frames {
                let direction_lock = u32::from(request.reuse_from_job_dir.is_none());
                direction_lock.saturating_add(request.validation_animations.len() as u32)
            } else if locked_video {
                u32::from(request.reuse_from_job_dir.is_none())
                    .saturating_add(request.validation_animations.len() as u32)
            } else if grid_keyframes {
                request.validation_animations.len().saturating_mul(4) as u32
            } else if grid_generation {
                request.validation_animations.len() as u32
            } else if video_workflow {
                request
                    .validation_animations
                    .iter()
                    .map(|animation| {
                        match request
                            .retry_stages
                            .get(animation)
                            .copied()
                            .unwrap_or(super::types::CharacterRetryStage::Still)
                        {
                            super::types::CharacterRetryStage::Auto
                            | super::types::CharacterRetryStage::Still => 2_u32,
                            super::types::CharacterRetryStage::Video => 1,
                            super::types::CharacterRetryStage::Loop
                            | super::types::CharacterRetryStage::Matting
                            | super::types::CharacterRetryStage::Consistency
                            | super::types::CharacterRetryStage::Frame => 0,
                        }
                    })
                    .sum()
            } else {
                request
                    .validation_animations
                    .len()
                    .saturating_mul(frames_per_animation) as u32
            };
            let (estimated, maximum) = if local_only {
                (0, 0)
            } else if image2_direction_grid_probe || front_authoritative_direction_grid {
                (1, 1)
            } else if topdown_cycle {
                let count = if request.validation_only {
                    request.validation_animations.len() as u32
                } else {
                    4
                };
                (count, count)
            } else if direction_motion && request.validation_only {
                (1, 2)
            } else if grid_keyframes && request.validation_only {
                let selected = request.retry_frames.values().map(Vec::len).sum::<usize>() as u32;
                let estimated = if selected > 0 { selected } else { 4 };
                if matches!(
                    request.workflow.version.as_str(),
                    "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
                ) && selected > 0
                {
                    // A structured-gait laterality child validation is a single-shot
                    // fresh retry. It may not reserve a hidden diagnostic
                    // edit after the selected request.
                    (estimated, estimated)
                } else {
                    (estimated, estimated.saturating_mul(2))
                }
            } else if request.validation_only {
                (
                    validation_request_count,
                    validation_request_count.saturating_mul(2),
                )
            } else if frame_image_workflow && selected_frames > 0 {
                (selected_frames, selected_frames)
            } else if locked_frames {
                let direction_lock = u32::from(request.reuse_from_job_dir.is_none());
                let action_requests = if request.retry_animations.is_empty() {
                    4
                } else if selected_frames > 0 {
                    selected_frames
                } else {
                    request.retry_animations.len() as u32
                };
                let estimated = direction_lock.saturating_add(action_requests);
                (estimated, estimated.saturating_mul(2))
            } else if direction_motion {
                let estimated = if request.retry_animations.is_empty() {
                    match request.direction_motion_stage {
                        DirectionMotionGenerationStageV1::ImageLocks => 8,
                        DirectionMotionGenerationStageV1::Complete => {
                            // Complete is a separately authorized child stage.
                            // The approved eight-image lock is immutable and
                            // reused without any new image request.
                            if request.validation_only {
                                1
                            } else {
                                4
                            }
                        }
                    }
                } else {
                    match request.direction_motion_stage {
                        DirectionMotionGenerationStageV1::ImageLocks => {
                            DirectionMotionLockV1::image_retry_closure(&request.retry_animations)
                                .map(|nodes| nodes.len() as u32)
                                .unwrap_or(8)
                        }
                        DirectionMotionGenerationStageV1::Complete => request
                            .retry_animations
                            .iter()
                            .filter(|animation| animation.starts_with("walk_"))
                            .count()
                            as u32,
                    }
                };
                (estimated, estimated.saturating_mul(2))
            } else if grid_keyframes {
                let selected = request.retry_frames.values().map(Vec::len).sum::<usize>() as u32;
                let estimated = if selected > 0 { selected } else { 16 };
                (estimated, estimated.saturating_mul(2))
            } else if grid_generation {
                let estimated = match request.direction_motion_stage {
                    DirectionMotionGenerationStageV1::ImageLocks => 1,
                    DirectionMotionGenerationStageV1::Complete => {
                        let selected_frames =
                            request.retry_frames.values().map(Vec::len).sum::<usize>() as u32;
                        if selected_frames > 0 {
                            selected_frames
                        } else if request.validation_only {
                            request.validation_animations.len() as u32
                        } else {
                            4
                        }
                    }
                };
                (estimated, estimated.saturating_mul(2))
            } else if direction_pose_chain {
                let estimated: u32 = if request.retry_animations.is_empty() {
                    8
                } else {
                    request
                        .retry_animations
                        .iter()
                        .map(|animation| {
                            match request
                                .retry_stages
                                .get(animation)
                                .copied()
                                .unwrap_or(super::types::CharacterRetryStage::Auto)
                            {
                                super::types::CharacterRetryStage::Auto
                                | super::types::CharacterRetryStage::Video => 1,
                                super::types::CharacterRetryStage::Still => 0,
                                super::types::CharacterRetryStage::Loop
                                | super::types::CharacterRetryStage::Matting
                                | super::types::CharacterRetryStage::Consistency
                                | super::types::CharacterRetryStage::Frame => 0,
                            }
                        })
                        .sum()
                };
                (estimated, estimated.saturating_mul(2))
            } else if locked_video {
                let direction_lock = u32::from(request.reuse_from_job_dir.is_none());
                let action_requests = if request.retry_animations.is_empty() {
                    4
                } else {
                    request.retry_animations.len() as u32
                };
                let estimated = direction_lock.saturating_add(action_requests);
                (estimated, estimated.saturating_mul(2))
            } else if sprite_sheet {
                let estimated = if request.retry_animations.is_empty() {
                    4
                } else {
                    request.retry_animations.len() as u32
                };
                (estimated, estimated.saturating_mul(2))
            } else if frame_image_workflow {
                let estimated = frames_per_animation as u32 * 4;
                (estimated, estimated.saturating_mul(2))
            } else if request.workflow.id == "topdown-video" {
                (8, 16)
            } else {
                (9, 17)
            };
            PlanEstimateV1 {
                provider_request_estimate: estimated,
                maximum_provider_requests: maximum,
                provider_id: (!local_only).then(|| request.provider_id.clone()),
                profile_id: (!local_only).then(|| request.profile_id.clone()),
                workflow: Some(format!(
                    "{}@{}",
                    request.workflow.id, request.workflow.version
                )),
                model: if local_only {
                    None
                } else if topdown_cycle {
                    request.generation.video_model.clone()
                } else {
                    request.generation.image_model.clone()
                },
                ..Default::default()
            }
        }
        AutomationOperation::ImportDirectionGrid(_) => PlanEstimateV1 {
            provider_request_estimate: 0,
            maximum_provider_requests: 0,
            provider_id: None,
            profile_id: None,
            workflow: Some("topdown-grid@9.0.0".into()),
            model: Some("external_import".into()),
            ..Default::default()
        },
        AutomationOperation::GenerateStaticAssetSet(request) => {
            let count = if request.consistency_recheck_only {
                0
            } else if request.portrait_phase == PortraitGenerationPhaseV1::BaseOnly {
                1
            } else if request.retry_item_ids.is_empty() {
                request.asset.items.len() as u32
            } else {
                request.retry_item_ids.len() as u32
            };
            PlanEstimateV1 {
                provider_request_estimate: count,
                maximum_provider_requests: count.saturating_mul(2),
                provider_id: Some(request.provider_id.clone()),
                profile_id: Some(request.profile_id.clone()),
                model: request.image_model.clone(),
                workflow: (request.portrait_phase != PortraitGenerationPhaseV1::LegacyAll).then(
                    || match request.portrait_phase {
                        PortraitGenerationPhaseV1::BaseOnly => "portrait-base@1.0.0".into(),
                        PortraitGenerationPhaseV1::Expressions => {
                            "portrait-expressions@1.0.0".into()
                        }
                        PortraitGenerationPhaseV1::LegacyAll => unreachable!(),
                    },
                ),
                ..Default::default()
            }
        }
        AutomationOperation::CreateEnvironmentLock(request) => PlanEstimateV1 {
            provider_request_estimate: 1,
            maximum_provider_requests: 1,
            provider_id: Some(request.provider_id.clone()),
            profile_id: Some(request.profile_id.clone()),
            ..Default::default()
        },
        AutomationOperation::GenerateTerrainSet(request) => PlanEstimateV1 {
            provider_request_estimate: 2,
            maximum_provider_requests: 4,
            provider_id: Some(request.provider_id.clone()),
            profile_id: Some(request.profile_id.clone()),
            ..Default::default()
        },
        AutomationOperation::GenerateBuildingKit(request) => PlanEstimateV1 {
            provider_request_estimate: 3,
            maximum_provider_requests: 6,
            provider_id: Some(request.provider_id.clone()),
            profile_id: Some(request.profile_id.clone()),
            ..Default::default()
        },
        AutomationOperation::BuildProject(_) => {
            // The parent build job makes no provider calls itself; child builds
            // are planned separately and carry their own estimates.
            PlanEstimateV1::default()
        }
        _ => PlanEstimateV1::default(),
    }
}

fn validate_operation(operation: &AutomationOperation) -> Result<(), PlanStoreError> {
    match operation {
        AutomationOperation::PrepareAsset(request) => {
            if request.metadata.name.trim().is_empty() {
                return Err(PlanStoreError::InvalidRequest(
                    "metadata.name is required".into(),
                ));
            }
            if request.metadata.fps <= 0.0 {
                return Err(PlanStoreError::InvalidRequest(
                    "metadata.fps must be positive".into(),
                ));
            }
            match &request.input {
                AssetInput::PngSequence { paths } => {
                    if paths.len() < 2 {
                        return Err(PlanStoreError::InvalidRequest(
                            "png_sequence requires at least two PNG files".into(),
                        ));
                    }
                    for path in paths {
                        validate_png(path)?;
                    }
                }
                AssetInput::SpriteSheet { path, split } => {
                    validate_png(path)?;
                    if let super::types::SpriteSheetSplit::FixedGrid(grid) = split {
                        if grid.frame_width == 0
                            || grid.frame_height == 0
                            || grid.columns == 0
                            || grid.rows == 0
                        {
                            return Err(PlanStoreError::InvalidRequest(
                                "fixed_grid dimensions must be positive".into(),
                            ));
                        }
                    }
                }
                AssetInput::Gsfpack { path } => {
                    forge_pack::validate_pack_layout(path).map_err(|error| {
                        PlanStoreError::InvalidRequest(format!("invalid .gsfpack: {error}"))
                    })?;
                }
                AssetInput::VideoClip {
                    path,
                    start_time_ms,
                    end_time_ms,
                    target_frame_count,
                } => validate_video_clip(path, *start_time_ms, *end_time_ms, *target_frame_count)?,
            }
        }
        AutomationOperation::PrepareCharacterPack(request) => {
            validate_character_pack_request(request)?;
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            validate_generate_character_pack_request(request)?;
        }
        AutomationOperation::ImportDirectionGrid(request) => {
            validate_import_direction_grid_request(request)?;
        }
        AutomationOperation::CreateStyleLock(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "style lock requests require schemaVersion \"1\"".into(),
                ));
            }
            validate_provider_selection(&request.provider_id, &request.profile_id)?;
            read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let spec: StyleSpecV1 =
                serde_json::from_slice(&fs::read(&request.spec_path).map_err(|source| {
                    PlanStoreError::Io {
                        path: request.spec_path.clone(),
                        source,
                    }
                })?)?;
            if spec.schema_version != "1" || spec.prompt.trim().is_empty() {
                return Err(PlanStoreError::InvalidRequest(
                    "style spec requires schemaVersion \"1\" and a prompt".into(),
                ));
            }
            if spec.reference_images.len() > 3 {
                return Err(PlanStoreError::InvalidRequest(
                    "style spec accepts at most three reference images".into(),
                ));
            }
            let spec_root = request.spec_path.parent().unwrap_or_else(|| Path::new("."));
            for reference in &spec.reference_images {
                validate_png(&resolve_relative(spec_root, reference))?;
            }
        }
        AutomationOperation::CreateSubjectLock(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "subject lock requests require schemaVersion \"1\"".into(),
                ));
            }
            if request.is_local_import() && !cfg!(feature = "subject-import") {
                return Err(PlanStoreError::InvalidRequest(
                    "subject import requires a Forge build with subject-import enabled".into(),
                ));
            }
            validate_provider_selection(&request.provider_id, &request.profile_id)?;
            let project = read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if project.provider.id != request.provider_id
                || project.provider.profile_id != request.profile_id
            {
                return Err(PlanStoreError::InvalidRequest(
                    "subject generation must use the project Provider profile".into(),
                ));
            }
            if project.current_style_revision.is_none() {
                return Err(PlanStoreError::InvalidRequest(
                    "run `forge style create` before creating a Subject Lock".into(),
                ));
            }
            let spec = read_subject_spec(&request.spec_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            match (
                request.canonical_import_path.as_deref(),
                request.import_approval_note.as_deref(),
            ) {
                (Some(canonical), Some(note)) => {
                    if note.trim().is_empty() {
                        return Err(PlanStoreError::InvalidRequest(
                            "subject import requires a non-empty approval note".into(),
                        ));
                    }
                    if !spec.reference_images.is_empty() {
                        return Err(PlanStoreError::InvalidRequest(
                            "subject import uses canonicalImportPath as its only identity input"
                                .into(),
                        ));
                    }
                    validate_png(canonical)?;
                }
                (Some(_), None) => {
                    return Err(PlanStoreError::InvalidRequest(
                        "subject import requires importApprovalNote".into(),
                    ));
                }
                (None, Some(_)) => {
                    return Err(PlanStoreError::InvalidRequest(
                        "importApprovalNote requires canonicalImportPath".into(),
                    ));
                }
                (None, None) => {}
            }
        }
        AutomationOperation::CreateCollectionLock(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "collection lock requests require schemaVersion \"1\"".into(),
                ));
            }
            validate_provider_selection(&request.provider_id, &request.profile_id)?;
            let project = read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if project.provider.id != request.provider_id
                || project.provider.profile_id != request.profile_id
            {
                return Err(PlanStoreError::InvalidRequest(
                    "collection generation must use the project Provider profile".into(),
                ));
            }
            if project.current_style_revision.is_none() {
                return Err(PlanStoreError::InvalidRequest(
                    "run `forge style create` before creating a Collection Lock".into(),
                ));
            }
            read_collection_spec(&request.spec_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            validate_static_asset_set_request(request)?;
        }
        AutomationOperation::CreateEnvironmentLock(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "environment requests require schemaVersion 1".into(),
                ));
            }
            validate_provider_selection(&request.provider_id, &request.profile_id)?;
            let project = read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if project.provider.id != request.provider_id
                || project.provider.profile_id != request.profile_id
            {
                return Err(PlanStoreError::InvalidRequest(
                    "environment generation must use the project Provider profile".into(),
                ));
            }
            let spec: EnvironmentSpecV1 = read_json(&request.spec_path)?;
            validate_environment_spec(&spec)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        }
        AutomationOperation::GenerateTerrainSet(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "terrain generation requires schemaVersion 1".into(),
                ));
            }
            validate_provider_selection(&request.provider_id, &request.profile_id)?;
            read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let environment = read_environment_lock(&request.environment_lock_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if environment.provider_id != request.provider_id
                || environment.profile_id != request.profile_id
            {
                return Err(PlanStoreError::InvalidRequest(
                    "terrain generation must use the Provider locked by the environment".into(),
                ));
            }
            validate_terrain_spec(&request.asset, &environment)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        }
        AutomationOperation::GenerateBuildingKit(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "building generation requires schemaVersion 1".into(),
                ));
            }
            validate_provider_selection(&request.provider_id, &request.profile_id)?;
            read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let environment = read_environment_lock(&request.environment_lock_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if environment.provider_id != request.provider_id
                || environment.profile_id != request.profile_id
            {
                return Err(PlanStoreError::InvalidRequest(
                    "building generation must use the Provider locked by the environment".into(),
                ));
            }
            validate_building_spec(&request.asset, &environment)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        }
        AutomationOperation::CompileMap(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "map compilation requires schemaVersion 1".into(),
                ));
            }
            read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let spec: MapSpecV1 = read_json(&request.spec_path)?;
            validate_map_spec(&spec)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let root = request.spec_path.parent().unwrap_or_else(|| Path::new("."));
            for relative in spec
                .dependencies
                .terrain_sets
                .iter()
                .chain(&spec.dependencies.building_kits)
                .chain(&spec.dependencies.prop_sets)
            {
                forge_pack::validate_pack_layout(&root.join(relative)).map_err(|error| {
                    PlanStoreError::InvalidRequest(format!(
                        "invalid map dependency {}: {error}",
                        relative.display()
                    ))
                })?;
            }
        }
        AutomationOperation::InstallGodot(request) => {
            forge_pack::validate_pack_layout(&request.pack_path).map_err(|error| {
                PlanStoreError::InvalidRequest(format!("invalid .gsfpack: {error}"))
            })?;
            if !request.project_path.join("project.godot").is_file() {
                return Err(PlanStoreError::InvalidRequest(
                    "projectPath must contain project.godot".into(),
                ));
            }
            let target_components = request.target.components().collect::<Vec<_>>();
            if request.target.as_os_str().is_empty()
                || request.target == Path::new(".")
                || !request.target.starts_with("addons/forge_assets")
                || request.target.is_absolute()
                || request.target.components().any(|part| {
                    matches!(
                        part,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
            {
                return Err(PlanStoreError::InvalidRequest(
                    "target must be inside addons/forge_assets and may not contain '..'".into(),
                ));
            }
            if target_components.len() < 2 {
                return Err(PlanStoreError::InvalidRequest(
                    "target must name a directory below addons/forge_assets".into(),
                ));
            }
            let asset_key = request
                .asset_key
                .as_deref()
                .or_else(|| request.target.file_name().and_then(|value| value.to_str()))
                .unwrap_or_default();
            if !is_engine_safe_name(asset_key) {
                return Err(PlanStoreError::InvalidRequest(
                    "assetKey must contain only letters, numbers, '-' or '_'".into(),
                ));
            }
            if request.provider_refs.len() > 32
                || request.provider_refs.iter().any(|reference| {
                    reference.provider.trim().is_empty()
                        || reference.provider.len() > 64
                        || reference.provider.chars().any(char::is_control)
                        || reference.asset_id.as_ref().is_some_and(|asset_id| {
                            asset_id.trim().is_empty()
                                || asset_id.len() > 256
                                || asset_id.chars().any(char::is_control)
                        })
                })
            {
                return Err(PlanStoreError::InvalidRequest(
                    "providerRefs must contain at most 32 valid providers and optional asset IDs"
                        .into(),
                ));
            }
            if let Some(catalog_project) = &request.catalog_project_path {
                read_project(catalog_project)
                    .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
                let catalog = read_project_catalog(catalog_project)
                    .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
                let canonical_pack =
                    fs::canonicalize(&request.pack_path).map_err(|source| PlanStoreError::Io {
                        path: request.pack_path.clone(),
                        source,
                    })?;
                let found = catalog.assets.values().find(|entry| {
                    fs::canonicalize(&entry.pack_path)
                        .ok()
                        .is_some_and(|path| path == canonical_pack)
                });
                let entry = found.ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "catalogProjectPath does not contain the requested Pack".into(),
                    )
                })?;
                if entry.game_ready == Some(false)
                    || entry
                        .review
                        .as_ref()
                        .is_some_and(|review| review.status == CatalogReviewStatusV1::Quarantined)
                {
                    return Err(PlanStoreError::InvalidRequest(
                        "catalog asset is quarantined or not game-ready".into(),
                    ));
                }
            }
            let target = request.project_path.join(&request.target);
            validate_godot_target_location(&request.project_path, &target)?;
            if target.exists() && !target.join(OWNERSHIP_MARKER).is_file() {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "existing Godot target is not Forge-owned: {}",
                    target.display()
                )));
            }
        }
        AutomationOperation::BuildProject(request) => {
            if request.schema_version != "1" {
                return Err(PlanStoreError::InvalidRequest(
                    "build project requests require schemaVersion \"1\"".into(),
                ));
            }
            if request.project_path.as_os_str().is_empty() {
                return Err(PlanStoreError::InvalidRequest(
                    "projectPath must not be empty".into(),
                ));
            }
            if !request.project_path.is_absolute() {
                return Err(PlanStoreError::InvalidRequest(
                    "projectPath must be an absolute path".into(),
                ));
            }
            if !request.manifest_path.is_absolute() {
                return Err(PlanStoreError::InvalidRequest(
                    "manifestPath must be an absolute path".into(),
                ));
            }
            if !request.project_path.is_dir() {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "project directory does not exist: {}",
                    request.project_path.display()
                )));
            }
            if request.manifest_path.is_symlink() {
                return Err(PlanStoreError::InvalidRequest(
                    "manifestPath must not be a symbolic link".into(),
                ));
            }
            if !request.manifest_path.is_file() {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "manifest file does not exist: {}",
                    request.manifest_path.display()
                )));
            }
        }
    }
    Ok(())
}

fn validate_png(path: &Path) -> Result<(), PlanStoreError> {
    if !path.is_file()
        || !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
    {
        return Err(PlanStoreError::InvalidRequest(format!(
            "PNG file does not exist: {}",
            path.display()
        )));
    }
    Ok(())
}

pub fn fingerprint_operation_inputs(
    operation: &AutomationOperation,
) -> Result<String, PlanStoreError> {
    let mut hasher = Sha256::new();
    match operation {
        AutomationOperation::PrepareAsset(request) => match &request.input {
            AssetInput::PngSequence { paths } => hash_files(&mut hasher, paths)?,
            AssetInput::SpriteSheet { path, .. } => {
                hash_files(&mut hasher, std::slice::from_ref(path))?
            }
            AssetInput::Gsfpack { path } => hash_directory(&mut hasher, path)?,
            AssetInput::VideoClip { path, .. } => {
                hash_files(&mut hasher, std::slice::from_ref(path))?
            }
        },
        AutomationOperation::PrepareCharacterPack(request) => {
            for animation in &request.animations {
                hasher.update(animation.name.as_bytes());
                match &animation.input {
                    AssetInput::PngSequence { paths } => hash_files(&mut hasher, paths)?,
                    AssetInput::SpriteSheet { path, .. } => {
                        hash_files(&mut hasher, std::slice::from_ref(path))?
                    }
                    AssetInput::Gsfpack { path } => hash_directory(&mut hasher, path)?,
                    AssetInput::VideoClip { path, .. } => {
                        hash_files(&mut hasher, std::slice::from_ref(path))?
                    }
                }
            }
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            hasher.update(serde_json::to_vec(operation).map_err(PlanStoreError::Json)?);
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            hasher.update(request.character.prompt.as_bytes());
            hasher.update(serde_json::to_vec(&request.equipment).map_err(PlanStoreError::Json)?);
            hasher.update([u8::from(request.equipment_explicit)]);
            if let Some(path) = &request.equipment.reference_image {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
            hasher.update([u8::from(request.validation_only)]);
            for animation in &request.validation_animations {
                hasher.update(animation.as_bytes());
            }
            if let Some(project_path) = &request.project_path {
                hash_files(&mut hasher, &[project_path.join("forge-project.json")])?;
            }
            if let Some(path) = &request.character.reference_image_path {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
            if let Some(path) = &request.style_lock_path {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
            if let Some(path) = &request.subject_lock_path {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
            if let Some(source) = &request.reuse_from_job_dir {
                for animation in &request.retry_animations {
                    hasher.update(animation.as_bytes());
                    if let Some(frames) = request.retry_frames.get(animation) {
                        hasher.update(frames);
                    }
                }
                let keyframe = matches!(
                    request.workflow.id.as_str(),
                    "topdown-keyframes" | "topdown-keyposes"
                );
                let sprite_sheet = matches!(
                    request.workflow.id.as_str(),
                    "topdown-spritesheet" | "topdown-frames"
                );
                let locked_video = matches!(
                    request.workflow.id.as_str(),
                    "topdown-video-locked" | "topdown-video-cycle"
                );
                let topdown_cycle = request.workflow.id == "topdown-cycle"
                    && matches!(request.workflow.version.as_str(), "10.0.0" | "10.1.0");
                let image2_direction_grid_probe = is_xai_image2_direction_grid_probe(request);
                let grid_keyframes = request.workflow.id == "topdown-grid"
                    && matches!(
                        request.workflow.version.as_str(),
                        "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
                    );
                let v95_source = if request.workflow.id == "topdown-grid"
                    && request.workflow.version == "9.5.0"
                {
                    Some(
                        crate::grid_retry_source::validate_v95_footwear_cleanup_source_closure(
                            source,
                            request
                                .retry_frames
                                .get("walk_down")
                                .map(Vec::as_slice)
                                .unwrap_or_default(),
                        )
                        .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?,
                    )
                } else {
                    None
                };
                let material_source = v95_source
                    .as_ref()
                    .map(|validated| validated.material_source_job_dir.as_path())
                    .unwrap_or(source);
                let mut retry_files = vec![source.join("job.json")];
                if material_source != source {
                    retry_files.push(source.join("provider-usage.json"));
                    retry_files.push(material_source.join("job.json"));
                }
                let manifest = if image2_direction_grid_probe {
                    source.join("source/grid-provider-manifest.json")
                } else if grid_keyframes && request.retry_frames.is_empty() {
                    material_source.join("source/grid-provider-manifest.json")
                } else if grid_keyframes {
                    material_source.join("source/grid-keyframe-provider-manifest.json")
                } else if keyframe {
                    source.join("source/keyframe-provider-manifest.json")
                } else if sprite_sheet {
                    source.join("source/spritesheet-provider-manifest.json")
                } else {
                    source.join("source/provider-manifest.json")
                };
                if manifest.is_file() {
                    retry_files.push(manifest);
                }
                if keyframe {
                    retry_files.push(source.join("workflow-graph.json"));
                }
                if grid_keyframes {
                    retry_files.extend([
                        material_source.join("source/direction-grid-lock.json"),
                        material_source.join(if request.retry_frames.is_empty() {
                            crate::character_grid::GRID_APPROVAL_FILE
                        } else {
                            "source/direction-grid-approval.json"
                        }),
                    ]);
                    let appearance =
                        material_source.join(crate::character_grid::DIRECTION_GRID_APPEARANCE_FILE);
                    if appearance.is_file() {
                        retry_files.push(appearance);
                    }
                    let graph = material_source.join("workflow-graph.json");
                    if graph.is_file() {
                        retry_files.push(graph);
                    }
                }
                if topdown_cycle {
                    let cycle_approval = [
                        source.join(crate::character_grid::GRID_APPROVAL_FILE),
                        source.join("source/direction-grid-approval.json"),
                    ]
                    .into_iter()
                    .find(|path| path.is_file())
                    .unwrap_or_else(|| source.join(crate::character_grid::GRID_APPROVAL_FILE));
                    retry_files.extend([
                        source.join("source/direction-grid-lock.json"),
                        cycle_approval,
                        source.join(crate::character_grid::DIRECTION_GRID_APPEARANCE_FILE),
                    ]);
                }
                if image2_direction_grid_probe {
                    retry_files.extend([
                        source.join("source/direction-grid-lock.json"),
                        source.join(crate::character_grid::GRID_APPROVAL_FILE),
                    ]);
                    let retry_evidence = source.join("source/direction-grid-retry-evidence.json");
                    if retry_evidence.is_file() {
                        retry_files.push(retry_evidence);
                    }
                    let appearance =
                        source.join(crate::character_grid::DIRECTION_GRID_APPEARANCE_FILE);
                    if appearance.is_file() {
                        retry_files.push(appearance);
                    }
                }
                if grid_keyframes || topdown_cycle {
                    let import_lock_path = material_source.join("source/direction-grid-lock.json");
                    if import_lock_path.is_file() {
                        let import_lock: DirectionGridLockV1 = read_json(&import_lock_path)?;
                        if import_lock.profile
                            == crate::character_grid::DIRECTION_GRID_IMPORT_LOCK_PROFILE
                        {
                            retry_files.extend(
                                [
                                    import_lock.import_consistency_report_path,
                                    import_lock.alpha_edge_report_path,
                                    import_lock
                                        .producer
                                        .and_then(|producer| producer.evidence_path),
                                ]
                                .into_iter()
                                .flatten(),
                            );
                            let authority_alignment = material_source.join(
                                "source/imported-direction-grid/authority-alignment-report.json",
                            );
                            if authority_alignment.is_file() {
                                retry_files.push(authority_alignment);
                            }
                            let matted_alpha = material_source.join(
                                "source/imported-direction-grid/alpha-edge-halo-matted-report.json",
                            );
                            if matted_alpha.is_file() {
                                retry_files.push(matted_alpha);
                            }
                        }
                    }
                }
                if locked_video {
                    retry_files.push(source.join("source/direction-lock.json"));
                }
                hash_files(&mut hasher, &retry_files)?;
                hash_directory(
                    &mut hasher,
                    &material_source.join(
                        if topdown_cycle
                            || image2_direction_grid_probe
                            || (grid_keyframes && request.retry_frames.is_empty())
                        {
                            "direction-grid"
                        } else if grid_keyframes {
                            "grid-keyframes"
                        } else if keyframe {
                            "source/provider-keyframes"
                        } else if sprite_sheet {
                            "source/provider-animation-sheets"
                        } else if locked_video {
                            "direction-lock"
                        } else {
                            "source/provider"
                        },
                    ),
                )?;
                if request.workflow.id == "topdown-cycle" && request.workflow.version == "10.1.0" {
                    // V10.1 is a zero-request replay of the immutable V10 media.
                    // Bind the exact retained clip and the quality evidence that
                    // made it eligible for translation-only stabilization.
                    hash_directory(&mut hasher, &source.join("source/provider/walk_down"))?;
                    hash_files(
                        &mut hasher,
                        &[
                            source.join("character-scale-lock.json"),
                            source.join("character-motion-semantics-report.json"),
                            source.join("character-silhouette-temporal-report.json"),
                            source.join("provider-usage.json"),
                            source.join("workflow-graph.json"),
                        ],
                    )?;
                }
                if grid_keyframes && !request.retry_frames.is_empty() {
                    hash_directory(
                        &mut hasher,
                        &material_source.join("source/grid-keyframe-actions"),
                    )?;
                }
            }
        }
        AutomationOperation::ImportDirectionGrid(request) => {
            hasher.update(serde_json::to_vec(operation).map_err(PlanStoreError::Json)?);
            let input_paths = request
                .ordered_input_paths()
                .into_iter()
                .map(|(_, path)| path.clone())
                .collect::<Vec<_>>();
            hash_files(&mut hasher, &input_paths)?;
            let source = &request.source_job_dir;
            let approval = [
                source.join(crate::character_grid::GRID_APPROVAL_FILE),
                source.join("source/direction-grid-approval.json"),
            ]
            .into_iter()
            .find(|path| path.is_file())
            .ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "direction_grid_import_source_invalid: approval is missing".into(),
                )
            })?;
            let mut closure = vec![
                source.join("job.json"),
                source.join("source/direction-grid-lock.json"),
                source.join(crate::character_grid::DIRECTION_GRID_APPEARANCE_FILE),
                approval,
            ];
            let manifest = source.join("source/grid-provider-manifest.json");
            if manifest.is_file() {
                closure.push(manifest);
            }
            hash_files(&mut hasher, &closure)?;
            hash_directory(&mut hasher, &source.join("direction-grid"))?;
            let source_job: JobRecord = read_json(&source.join("job.json"))?;
            let source_operation: AutomationOperation =
                serde_json::from_value(source_job.recipe.ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "direction_grid_import_source_invalid: source recipe is missing".into(),
                    )
                })?)?;
            let AutomationOperation::GenerateCharacterPack(source_request) = source_operation
            else {
                return Err(PlanStoreError::InvalidRequest(
                    "direction_grid_import_source_invalid: source is not Character generation"
                        .into(),
                ));
            };
            for path in [
                source_request.style_lock_path.as_ref(),
                source_request.subject_lock_path.as_ref(),
                source_request.character.reference_image_path.as_ref(),
            ]
            .into_iter()
            .flatten()
            {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
        }
        AutomationOperation::CreateStyleLock(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.spec_path.clone(),
                ],
            )?;
            let spec: StyleSpecV1 =
                serde_json::from_slice(&fs::read(&request.spec_path).map_err(|source| {
                    PlanStoreError::Io {
                        path: request.spec_path.clone(),
                        source,
                    }
                })?)?;
            let root = request.spec_path.parent().unwrap_or_else(|| Path::new("."));
            let references = spec
                .reference_images
                .iter()
                .map(|path| resolve_relative(root, path))
                .collect::<Vec<_>>();
            hash_files(&mut hasher, &references)?;
        }
        AutomationOperation::CreateSubjectLock(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            let project = read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let style_revision = project.current_style_revision.ok_or_else(|| {
                PlanStoreError::InvalidRequest("project has no current Style revision".into())
            })?;
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.spec_path.clone(),
                    request
                        .project_path
                        .join(".forge/styles")
                        .join(style_revision)
                        .join("style-lock.json"),
                ],
            )?;
            let spec = read_subject_spec(&request.spec_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            hash_files(&mut hasher, &spec.reference_images)?;
            if let Some(canonical) = request.canonical_import_path.as_ref() {
                hash_files(&mut hasher, std::slice::from_ref(canonical))?;
                hasher.update(
                    request
                        .import_approval_note
                        .as_deref()
                        .unwrap_or_default()
                        .as_bytes(),
                );
            }
        }
        AutomationOperation::CreateCollectionLock(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            let project = read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let style_revision = project.current_style_revision.ok_or_else(|| {
                PlanStoreError::InvalidRequest("project has no current Style revision".into())
            })?;
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.spec_path.clone(),
                    request
                        .project_path
                        .join(".forge/styles")
                        .join(style_revision)
                        .join("style-lock.json"),
                ],
            )?;
            let spec = read_collection_spec(&request.spec_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if let Some(anchor) = spec.anchor_image {
                hash_files(&mut hasher, &[anchor])?;
            }
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            hasher.update(serde_json::to_vec(&request.portrait_phase)?);
            hasher.update(serde_json::to_vec(&request.neutral_reference_policy)?);
            if let Some(parent) = &request.portrait_base_parent_job_id {
                hasher.update(parent.as_bytes());
            }
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.style_lock_path.clone(),
                ],
            )?;
            if let Some(path) = &request.collection_lock_path {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
            if let Some(path) = &request.subject_lock_path {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
            if let Some(path) = &request.source_spec_path {
                hash_files(&mut hasher, std::slice::from_ref(path))?;
            }
            hasher.update(serde_json::to_vec(&request.framing_profile)?);
            hasher.update(serde_json::to_vec(&request.item_metadata)?);
            for item in &request.asset.items {
                hasher.update(item.id.as_bytes());
                hasher.update(item.prompt.as_bytes());
                if let Some(path) = &item.reference_image {
                    hash_files(&mut hasher, std::slice::from_ref(path))?;
                }
            }
            if let Some(source) = &request.reuse_from_job_dir {
                for item in &request.retry_item_ids {
                    hasher.update(item.as_bytes());
                }
                hash_files(&mut hasher, &[source.join("job.json")])?;
                let source_report = source.join("consistency-report.json");
                if source_report.is_file() {
                    hash_files(&mut hasher, &[source_report])?;
                }
                hash_directory(&mut hasher, &source.join("normalized/static"))?;
            }
            if let (Some(source), Some(parent_job_id)) = (
                request.reuse_from_job_dir.as_deref(),
                request.portrait_base_parent_job_id.as_deref(),
            ) {
                let jobs_root = source.parent().ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "PortraitBase source Job is not inside a JobStore".into(),
                    )
                })?;
                let base_root = jobs_root.join(parent_job_id).join("portrait-base");
                hash_files(
                    &mut hasher,
                    &[
                        jobs_root.join(parent_job_id).join("job.json"),
                        base_root.join(PORTRAIT_BASE_LOCK_FILE),
                        base_root.join(PORTRAIT_BASE_APPROVAL_FILE),
                    ],
                )?;
                hash_directory(&mut hasher, &base_root)?;
            }
            let replacement_paths = request
                .replacement_item_paths
                .values()
                .cloned()
                .collect::<Vec<_>>();
            hash_files(&mut hasher, &replacement_paths)?;
        }
        AutomationOperation::CreateEnvironmentLock(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            let project = read_project(&request.project_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let style_revision = project.current_style_revision.ok_or_else(|| {
                PlanStoreError::InvalidRequest("project has no current Style revision".into())
            })?;
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.spec_path.clone(),
                    request
                        .project_path
                        .join(".forge/styles")
                        .join(style_revision)
                        .join("style-lock.json"),
                ],
            )?;
        }
        AutomationOperation::GenerateTerrainSet(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            hasher.update(serde_json::to_vec(&request.asset)?);
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.environment_lock_path.clone(),
                ],
            )?;
            let environment = read_environment_lock(&request.environment_lock_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            hash_files(&mut hasher, &[environment.board_path])?;
        }
        AutomationOperation::GenerateBuildingKit(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            hasher.update(serde_json::to_vec(&request.asset)?);
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.environment_lock_path.clone(),
                ],
            )?;
            let environment = read_environment_lock(&request.environment_lock_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            hash_files(&mut hasher, &[environment.board_path])?;
        }
        AutomationOperation::CompileMap(request) => {
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.spec_path.clone(),
                ],
            )?;
            let spec: MapSpecV1 = read_json(&request.spec_path)?;
            let root = request.spec_path.parent().unwrap_or_else(|| Path::new("."));
            for relative in spec
                .dependencies
                .terrain_sets
                .iter()
                .chain(&spec.dependencies.building_kits)
                .chain(&spec.dependencies.prop_sets)
            {
                hash_directory(&mut hasher, &root.join(relative))?;
            }
        }
        AutomationOperation::InstallGodot(request) => {
            hash_directory(&mut hasher, &request.pack_path)?;
            hash_files(&mut hasher, &[request.project_path.join("project.godot")])?;
            if let Some(catalog_project) = &request.catalog_project_path {
                hash_files(
                    &mut hasher,
                    &[
                        catalog_project.join("forge-project.json"),
                        catalog_project.join(PROJECT_CATALOG_RELATIVE),
                    ],
                )?;
            }
            hash_godot_target_identity(
                &mut hasher,
                &request.project_path,
                &request.project_path.join(&request.target),
            )?;
        }
        AutomationOperation::BuildProject(request) => {
            let canonical_project =
                fs::canonicalize(&request.project_path).map_err(|source| PlanStoreError::Io {
                    path: request.project_path.clone(),
                    source,
                })?;
            hasher.update(canonical_project.to_string_lossy().as_bytes());
            let validated = GameArtManifestV1::load_validated(&request.manifest_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let diff = compute_project_diff(&canonical_project, &validated)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let capabilities = ProviderCapabilityInput {
                capabilities: request.provider_capabilities.iter().cloned().collect(),
                image_model: request.image_model.clone(),
                video_model: request.video_model.clone(),
            };
            let plan = compute_build_plan(&canonical_project, &validated, &diff, &capabilities)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let source_sha256 = project_source_sha256(&canonical_project, &validated, &diff)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            hasher.update(b"build-project-plan-v1\0");
            hasher.update(plan.plan_sha256().as_bytes());
            if let Some(expected) = &request.expected_plan_sha256 {
                hasher.update(expected.as_bytes());
            }
            if let Some(source_job_id) = &request.resume_from_job_id {
                hasher.update(b"resume-from\0");
                hasher.update(source_job_id.as_bytes());
            }
            hasher.update(b"source-closure\0");
            hasher.update(source_sha256.as_bytes());
            if let Some(expected) = &request.expected_source_sha256 {
                hasher.update(expected.as_bytes());
            }
            hash_files(
                &mut hasher,
                &[
                    request.manifest_path.clone(),
                    canonical_project.join("forge-project.json"),
                ],
            )?;
            if let Some(revision) = &diff.style_revision {
                hash_directory(
                    &mut hasher,
                    &canonical_project.join(".forge/styles").join(revision),
                )?;
            }
            let mut lock_directories = BTreeSet::new();
            for action in &diff.actions {
                for reference in &action.lock_refs {
                    let directory = match reference.kind {
                        crate::game_art::LockKind::Style => canonical_project
                            .join(".forge/styles")
                            .join(&reference.revision),
                        crate::game_art::LockKind::Subject => canonical_project
                            .join(".forge/subjects")
                            .join(&reference.id)
                            .join(&reference.revision),
                        crate::game_art::LockKind::Collection => canonical_project
                            .join(".forge/collections")
                            .join(&reference.id)
                            .join(&reference.revision),
                    };
                    lock_directories.insert(directory);
                }
            }
            for directory in lock_directories {
                hash_directory(&mut hasher, &directory)?;
            }
            let catalog_path = canonical_project.join(PROJECT_CATALOG_RELATIVE);
            if catalog_path.is_file() {
                hash_files(&mut hasher, std::slice::from_ref(&catalog_path))?;
            }
            let catalog = read_project_catalog(&canonical_project)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            for asset in &validated.manifest.assets {
                let Some(entry) = catalog.assets.get(&asset.id) else {
                    continue;
                };
                let pack_path = resolve_relative(&canonical_project, &entry.pack_path);
                forge_pack::validate_pack_layout(&pack_path).map_err(|error| {
                    PlanStoreError::InvalidRequest(format!(
                        "catalog pack for {} is invalid: {error}",
                        asset.id
                    ))
                })?;
                hash_directory(&mut hasher, &pack_path)?;
            }
            match (
                request.resume_state_path.as_ref(),
                request.resume_state_sha256.as_ref(),
            ) {
                (Some(path), Some(expected)) => {
                    hash_files(&mut hasher, std::slice::from_ref(path))?;
                    let bytes = fs::read(path).map_err(|source| PlanStoreError::Io {
                        path: path.clone(),
                        source,
                    })?;
                    let actual = format!("{:x}", Sha256::digest(&bytes));
                    hasher.update(expected.as_bytes());
                    hasher.update(actual.as_bytes());
                    let state: ProjectBuildStateV1 = serde_json::from_slice(&bytes)?;
                    for entry in state.assets {
                        if let Some(pack_path) = entry.pack_path {
                            forge_pack::validate_pack_layout(&pack_path).map_err(|error| {
                                PlanStoreError::InvalidRequest(format!(
                                    "resume pack is invalid: {error}"
                                ))
                            })?;
                            hash_directory(&mut hasher, &pack_path)?;
                        }
                        if let Some(child_job_id) = entry.child_job_id {
                            let jobs_root =
                                path.parent().and_then(Path::parent).ok_or_else(|| {
                                    PlanStoreError::InvalidRequest(
                                        "resume state is not inside a JobStore".into(),
                                    )
                                })?;
                            let child_record_path = jobs_root.join(child_job_id).join("job.json");
                            hash_files(&mut hasher, std::slice::from_ref(&child_record_path))?;
                            let child: JobRecord = read_json(&child_record_path)?;
                            for artifact in child
                                .artifacts
                                .iter()
                                .filter(|artifact| artifact.kind == "gsfpack")
                            {
                                forge_pack::validate_pack_layout(&artifact.path).map_err(
                                    |error| {
                                        PlanStoreError::InvalidRequest(format!(
                                            "resume child pack is invalid: {error}"
                                        ))
                                    },
                                )?;
                                hash_directory(&mut hasher, &artifact.path)?;
                            }
                        }
                    }
                }
                (None, None) => {}
                _ => {
                    return Err(PlanStoreError::InvalidRequest(
                        "resumeStatePath and resumeStateSha256 must be supplied together".into(),
                    ));
                }
            }
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn validate_character_pack_request(
    request: &PrepareCharacterPackRequest,
) -> Result<(), PlanStoreError> {
    if request.schema_version != "2" {
        return Err(PlanStoreError::InvalidRequest(
            "Character Pack requests require schemaVersion \"2\"".into(),
        ));
    }
    if request.metadata.name.trim().is_empty() {
        return Err(PlanStoreError::InvalidRequest(
            "metadata.name is required".into(),
        ));
    }
    if request.animations.len() < 2 {
        return Err(PlanStoreError::InvalidRequest(
            "a Character Pack requires at least two animations".into(),
        ));
    }
    if let Some(profile) = request.source_cycle_sampling_profile.as_deref() {
        if profile != SOURCE_CYCLE_SAMPLING_PROFILE {
            return Err(PlanStoreError::InvalidRequest(format!(
                "unsupported sourceCycleSamplingProfile: {profile}"
            )));
        }
        if !request.animations.iter().any(|animation| {
            animation.loop_animation && matches!(animation.input, AssetInput::VideoClip { .. })
        }) {
            return Err(PlanStoreError::InvalidRequest(
                "sourceCycleSamplingProfile requires at least one looping video_clip animation"
                    .into(),
            ));
        }
    }
    if request.source_cycle_sampling_preview && request.source_cycle_sampling_profile.is_none() {
        return Err(PlanStoreError::InvalidRequest(
            "sourceCycleSamplingPreview requires sourceCycleSamplingProfile".into(),
        ));
    }
    let mut names = HashSet::new();
    for animation in &request.animations {
        if !is_engine_safe_name(&animation.name) {
            return Err(PlanStoreError::InvalidRequest(format!(
                "animation name must contain only letters, numbers, '-' or '_': {}",
                animation.name
            )));
        }
        if !names.insert(animation.name.clone()) {
            return Err(PlanStoreError::InvalidRequest(format!(
                "duplicate animation name: {}",
                animation.name
            )));
        }
        if animation.fps <= 0.0 {
            return Err(PlanStoreError::InvalidRequest(format!(
                "animation fps must be positive: {}",
                animation.name
            )));
        }
        match &animation.input {
            AssetInput::PngSequence { paths } => {
                let minimum_frames = if animation.loop_animation { 2 } else { 1 };
                if paths.len() < minimum_frames {
                    return Err(PlanStoreError::InvalidRequest(format!(
                        "animation {} requires at least {minimum_frames} PNG frame(s)",
                        animation.name,
                    )));
                }
                for path in paths {
                    validate_png(path)?;
                }
            }
            AssetInput::SpriteSheet { path, split } => {
                validate_png(path)?;
                if let super::types::SpriteSheetSplit::FixedGrid(grid) = split {
                    if grid.frame_width == 0
                        || grid.frame_height == 0
                        || grid.columns == 0
                        || grid.rows == 0
                    {
                        return Err(PlanStoreError::InvalidRequest(format!(
                            "fixed_grid dimensions must be positive for {}",
                            animation.name
                        )));
                    }
                }
            }
            AssetInput::Gsfpack { .. } => {
                return Err(PlanStoreError::InvalidRequest(
                    "Character Pack V1 accepts PNG sequences, sprite sheets, and video clips; merge from .gsfpack is reserved for a later iteration".into(),
                ));
            }
            AssetInput::VideoClip {
                path,
                start_time_ms,
                end_time_ms,
                target_frame_count,
            } => validate_video_clip(path, *start_time_ms, *end_time_ms, *target_frame_count)?,
        }
    }
    if !names.contains(&request.metadata.default_animation) {
        return Err(PlanStoreError::InvalidRequest(
            "metadata.defaultAnimation must name one animation".into(),
        ));
    }
    let catalog = character_workflow_catalog();
    let workflow = catalog
        .workflows
        .iter()
        .find(|workflow| {
            workflow.id == request.workflow.id && workflow.version == request.workflow.version
        })
        .ok_or_else(|| {
            PlanStoreError::InvalidRequest(format!(
                "unknown character workflow: {}@{}",
                request.workflow.id, request.workflow.version
            ))
        })?;
    let missing = workflow
        .required_animations
        .iter()
        .filter(|animation| !names.contains(&animation.name))
        .map(|animation| animation.name.clone())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(PlanStoreError::InvalidRequest(format!(
            "workflow {} requires animations: {}",
            workflow.id,
            missing.join(", ")
        )));
    }
    if request.workflow.id == EXTERNAL_KEYFRAME_WORKFLOW_ID
        && request.workflow.version == EXTERNAL_KEYFRAME_WORKFLOW_VERSION
    {
        validate_external_keyframe_request(request, &names)?;
    }
    Ok(())
}

fn validate_external_keyframe_request(
    request: &PrepareCharacterPackRequest,
    names: &HashSet<String>,
) -> Result<(), PlanStoreError> {
    if request.metadata.default_animation != "idle_down" {
        return Err(PlanStoreError::InvalidRequest(
            "external_keyframes_default_invalid: defaultAnimation must be idle_down".into(),
        ));
    }
    let prompt = request
        .character_prompt
        .as_deref()
        .unwrap_or_default()
        .trim();
    if prompt.is_empty() || prompt.len() > 4_000 || prompt.chars().any(char::is_control) {
        return Err(PlanStoreError::InvalidRequest(
            "external_keyframes_character_prompt_invalid: characterPrompt must contain 1..=4000 printable characters"
                .into(),
        ));
    }
    if !matches!(
        request.camera_profile,
        Some(
            CharacterCameraProfileV1::TopdownOrthographic
                | CharacterCameraProfileV1::TopdownThreeQuarter
        )
    ) {
        return Err(PlanStoreError::InvalidRequest(
            "external_keyframes_camera_invalid: cameraProfile must be topdown-orthographic@2.0.0 or topdown-three-quarter@1.0.0"
                .into(),
        ));
    }
    if let Some(path) = request.equipment.reference_image.as_ref() {
        validate_png(path)?;
    }

    let allowed = [
        "idle_down",
        "idle_up",
        "idle_right",
        "idle_left",
        "walk_down",
        "walk_up",
        "walk_right",
        "walk_left",
    ];
    if names.iter().any(|name| !allowed.contains(&name.as_str())) {
        return Err(PlanStoreError::InvalidRequest(
            "external_keyframes_animation_invalid: only cardinal idle_* and walk_* animations are allowed"
                .into(),
        ));
    }
    match request.metadata.rendering.mirror_policy {
        CharacterMirrorPolicyV1::Auto => {
            return Err(PlanStoreError::InvalidRequest(
                "external_keyframes_mirror_policy_required: choose right_only, explicit_left, or mirror_right_to_left"
                    .into(),
            ));
        }
        CharacterMirrorPolicyV1::RightOnly => {
            if !names.contains("idle_right") || !names.contains("walk_right") {
                return Err(PlanStoreError::InvalidRequest(
                    "external_keyframes_right_missing: right_only requires idle_right and walk_right"
                        .into(),
                ));
            }
            if names.contains("idle_left") || names.contains("walk_left") {
                return Err(PlanStoreError::InvalidRequest(
                    "external_keyframes_left_forbidden: right_only must omit idle_left and walk_left"
                        .into(),
                ));
            }
        }
        CharacterMirrorPolicyV1::ExplicitLeft => {
            if !names.contains("idle_left") || !names.contains("walk_left") {
                return Err(PlanStoreError::InvalidRequest(
                    "external_keyframes_left_missing: explicit_left requires idle_left and walk_left"
                        .into(),
                ));
            }
        }
        CharacterMirrorPolicyV1::MirrorRightToLeft => {
            if names.contains("idle_left") || names.contains("walk_left") {
                return Err(PlanStoreError::InvalidRequest(
                    "external_keyframes_left_redundant: mirror_right_to_left must omit idle_left and walk_left"
                        .into(),
                ));
            }
        }
    }

    let mut dimensions = None;
    let mut hashes = HashSet::new();
    for animation in &request.animations {
        let AssetInput::PngSequence { paths } = &animation.input else {
            return Err(PlanStoreError::InvalidRequest(format!(
                "external_keyframes_input_invalid: {} must use png_sequence, never a generated sheet or video",
                animation.name
            )));
        };
        let is_idle = animation.name.starts_with("idle_");
        let expected_frames = if is_idle { 1 } else { 4 };
        if paths.len() != expected_frames
            || animation.loop_animation == is_idle
            || (animation.fps - if is_idle { 1.0 } else { 4.0 }).abs() > f32::EPSILON
        {
            return Err(PlanStoreError::InvalidRequest(format!(
                "external_keyframes_cadence_invalid: {} requires {expected_frames} frame(s), fps {}, loop {}",
                animation.name,
                if is_idle { 1 } else { 4 },
                !is_idle
            )));
        }
        for path in paths {
            let image = image::open(path).map_err(|error| {
                PlanStoreError::InvalidRequest(format!(
                    "external_keyframes_png_invalid: cannot decode {}: {error}",
                    path.display()
                ))
            })?;
            let size = (image.width(), image.height());
            if size.0 != size.1 {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "external_keyframes_canvas_invalid: {} must use a square canvas",
                    path.display()
                )));
            }
            if dimensions.is_some_and(|expected| expected != size) {
                return Err(PlanStoreError::InvalidRequest(
                    "external_keyframes_canvas_invalid: every frame must use the same canvas dimensions"
                        .into(),
                ));
            }
            dimensions = Some(size);
            let rgba = image.to_rgba8();
            if !rgba.pixels().any(|pixel| pixel[3] == 0)
                || !rgba.pixels().any(|pixel| pixel[3] >= 128)
            {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "external_keyframes_alpha_invalid: {} must contain transparent background and opaque subject pixels",
                    path.display()
                )));
            }
            let sha256 = hash_file(path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if !hashes.insert(sha256) {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "external_keyframes_duplicate_frame: {} duplicates another supplied frame",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

pub(super) fn is_xai_image2_direction_grid_probe(request: &GenerateCharacterPackRequest) -> bool {
    request.provider_id == "xai"
        && request.profile_id == "default"
        && request.workflow.id == "topdown-grid"
        && request.workflow.version == "9.0.0"
        && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
        && request.generation.image_model.as_deref() == Some(super::XAI_IMAGE_2_CANDIDATE_MODEL)
        && request.generation.max_attempts_per_animation == 1
        && request.generation.target_frame_count == 4
        && request.generation.image_model.is_some()
        && request.reuse_from_job_dir.is_some()
        && request.retry_animations == ["direction_grid"]
        && request.retry_stages.len() == 1
        && request.retry_stages.get("direction_grid")
            == Some(&super::types::CharacterRetryStage::Still)
        && request.retry_frames.is_empty()
        && request.direction_grid_cape_contract.is_none()
        && !request.validation_only
        && request.validation_animations.is_empty()
}

pub(super) fn is_front_authoritative_direction_grid(
    request: &GenerateCharacterPackRequest,
) -> bool {
    request.workflow.id == "topdown-grid"
        && request.workflow.version == "9.0.0"
        && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
        && request.direction_grid_cape_contract
            == Some(crate::character_grid::CapeHemContractV1::FrontAuthoritativeNoSkirtHem)
        && request.generation.max_attempts_per_animation == 1
        && request.generation.target_frame_count == 4
        && request.generation.image_model.is_some()
        && request.reuse_from_job_dir.is_some()
        && request.retry_animations == ["direction_grid"]
        && request.retry_stages.len() == 1
        && request.retry_stages.get("direction_grid")
            == Some(&super::types::CharacterRetryStage::Still)
        && request.retry_frames.is_empty()
        && !request.validation_only
        && request.validation_animations.is_empty()
}

pub(super) fn validate_import_direction_grid_request(
    request: &ImportDirectionGridRequestV1,
) -> Result<(), PlanStoreError> {
    if request.schema_version != "1" {
        return Err(PlanStoreError::InvalidRequest(
            "DirectionGrid import requests require schemaVersion \"1\"".into(),
        ));
    }
    if !is_engine_safe_name(&request.generator) {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_import_generator_invalid: generator must be an engine-safe name".into(),
        ));
    }
    if request.note.trim().is_empty() || request.note.chars().count() > 500 {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_import_note_invalid: note must contain 1-500 characters".into(),
        ));
    }
    if request.sheet_path.is_some() == request.direction_paths.is_some() {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_import_input_invalid: exactly one of sheetPath and directionPaths is required"
                .into(),
        ));
    }
    if request.cape_hem_contract.is_some() && request.direction_paths.is_none() {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_cape_hem_contract_invalid: capeHemContract requires explicit front/rear/right/left files"
                .into(),
        ));
    }
    let inputs = request.ordered_input_paths();
    let mut dimensions = Vec::with_capacity(inputs.len());
    for (role, path) in &inputs {
        validate_png(path)?;
        let (width, height) = image::image_dimensions(path).map_err(|error| {
            PlanStoreError::InvalidRequest(format!(
                "direction_grid_import_input_invalid: cannot inspect {role} PNG: {error}"
            ))
        })?;
        if width != height || !(128..=4096).contains(&width) {
            return Err(PlanStoreError::InvalidRequest(format!(
                "direction_grid_import_input_invalid: {role} must be a square PNG between 128 and 4096 pixels"
            )));
        }
        dimensions.push((width, height));
    }
    if request.sheet_path.is_some() {
        let (width, _) = dimensions[0];
        if !width.is_multiple_of(2) {
            return Err(PlanStoreError::InvalidRequest(
                "direction_grid_import_sheet_invalid: sheet dimensions must be even".into(),
            ));
        }
    } else if dimensions.len() != 4 || dimensions.iter().any(|value| *value != dimensions[0]) {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_import_input_invalid: front/rear/right/left PNGs must have identical square dimensions"
                .into(),
        ));
    }

    let source = fs::canonicalize(&request.source_job_dir).map_err(|error| {
        PlanStoreError::InvalidRequest(format!(
            "direction_grid_import_source_invalid: cannot resolve source Job: {error}"
        ))
    })?;
    for (role, path) in inputs {
        let input = fs::canonicalize(path).map_err(|error| {
            PlanStoreError::InvalidRequest(format!(
                "direction_grid_import_input_invalid: cannot resolve {role}: {error}"
            ))
        })?;
        if input.starts_with(&source) {
            return Err(PlanStoreError::InvalidRequest(format!(
                "direction_grid_import_input_invalid: external {role} must not live inside the immutable source Job"
            )));
        }
    }
    let source_job: JobRecord = read_json(&source.join("job.json"))?;
    if source_job.lifecycle_state != JobLifecycleState::Succeeded
        || source.file_name().and_then(|value| value.to_str()) != Some(source_job.job_id.as_str())
    {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_import_source_invalid: source must be a succeeded canonical Job directory"
                .into(),
        ));
    }
    let source_operation: AutomationOperation =
        serde_json::from_value(source_job.recipe.clone().ok_or_else(|| {
            PlanStoreError::InvalidRequest(
                "direction_grid_import_source_invalid: source Job has no immutable recipe".into(),
            )
        })?)?;
    let source_recipe_hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&source_operation)?)
    );
    let AutomationOperation::GenerateCharacterPack(source_request) = source_operation else {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_import_source_invalid: source is not Character generation".into(),
        ));
    };
    if source_job.recipe_hash.as_deref() != Some(source_recipe_hash.as_str())
        || source_request.workflow.id != "topdown-grid"
        || source_request.workflow.version != "9.0.0"
        || source_request.direction_motion_stage != DirectionMotionGenerationStageV1::ImageLocks
    {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_import_source_invalid: source must be an immutable V9 DirectionGrid image-lock Job"
                .into(),
        ));
    }
    validate_topdown_cycle_direction_source(&source_request, &source)
}

fn validate_generate_character_pack_request(
    request: &GenerateCharacterPackRequest,
) -> Result<(), PlanStoreError> {
    if request.schema_version != "3" {
        return Err(PlanStoreError::InvalidRequest(
            "generated Character Pack requests require schemaVersion \"3\"".into(),
        ));
    }
    if !is_engine_safe_name(&request.provider_id) {
        return Err(PlanStoreError::InvalidRequest(
            "providerId must contain only letters, numbers, '-' or '_'".into(),
        ));
    }
    if !is_engine_safe_name(&request.profile_id) {
        return Err(PlanStoreError::InvalidRequest(
            "profileId must contain only letters, numbers, '-' or '_'".into(),
        ));
    }
    if request
        .asset_id
        .as_deref()
        .is_some_and(|asset_id| !is_engine_safe_name(asset_id))
    {
        return Err(PlanStoreError::InvalidRequest(
            "assetId must contain only letters, numbers, '-' or '_'".into(),
        ));
    }
    if let Some(project_path) = &request.project_path {
        let project = read_project(project_path)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        if project.provider.id != request.provider_id
            || project.provider.profile_id != request.profile_id
        {
            return Err(PlanStoreError::InvalidRequest(
                "character generation must use the project Provider profile".into(),
            ));
        }
    }
    let prompt = request.character.prompt.trim();
    if prompt.is_empty() || prompt.len() > 4_000 || prompt.chars().any(char::is_control) {
        return Err(PlanStoreError::InvalidRequest(
            "character.prompt must contain 1..=4000 printable characters".into(),
        ));
    }
    if request.metadata.name.trim().is_empty() {
        return Err(PlanStoreError::InvalidRequest(
            "metadata.name is required".into(),
        ));
    }
    let legacy_video_workflow =
        request.workflow.id == "topdown" && request.workflow.version == "1.0.0";
    let stable_video_workflow =
        request.workflow.id == "topdown-video" && request.workflow.version == "2.0.0";
    let locked_video_workflow =
        request.workflow.id == "topdown-video-locked" && request.workflow.version == "5.0.0";
    let video_cycle_workflow =
        request.workflow.id == "topdown-video-cycle" && request.workflow.version == "6.0.0";
    let direction_pose_chain_workflow =
        request.workflow.id == "topdown-direction-poses" && request.workflow.version == "7.0.0";
    let direction_motion_workflow =
        request.workflow.id == "topdown-direction-motion" && request.workflow.version == "8.0.0";
    let topdown_cycle_v101_workflow =
        request.workflow.id == "topdown-cycle" && request.workflow.version == "10.1.0";
    let topdown_cycle_workflow = request.workflow.id == "topdown-cycle"
        && matches!(request.workflow.version.as_str(), "10.0.0" | "10.1.0");
    let grid_generation_workflow =
        request.workflow.id == "topdown-grid" && request.workflow.version == "9.0.0";
    let grid_keyframe_workflow =
        request.workflow.id == "topdown-grid" && request.workflow.version == "9.1.0";
    let grid_structured_gait_workflow = request.workflow.id == "topdown-grid"
        && matches!(
            request.workflow.version.as_str(),
            "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
        );
    let image2_requested =
        request.generation.image_model.as_deref() == Some(super::XAI_IMAGE_2_CANDIDATE_MODEL);
    if request.direction_grid_cape_contract.is_some() && !grid_generation_workflow {
        return Err(PlanStoreError::InvalidRequest(
            "direction_grid_cape_contract_scope_invalid: the generated-grid cape contract is available only for topdown-grid@9.0.0"
                .into(),
        ));
    }
    if image2_requested && !is_xai_image2_direction_grid_probe(request) {
        return Err(PlanStoreError::InvalidRequest(
            "xai_image2_candidate_scope_forbidden: grok-imagine-image-2.0 is evaluation-only and requires an approved xai/default topdown-grid@9.0.0 DirectionGrid retry, direction_grid at still stage, exactly one attempt, and no validation, frame, or expanded retry scope"
                .into(),
        ));
    }
    let direction_motion_local_only = direction_motion_workflow
        && !request.retry_animations.is_empty()
        && request.retry_animations.iter().all(|animation| {
            request.retry_stages.get(animation).is_some_and(|stage| {
                matches!(
                    stage,
                    super::types::CharacterRetryStage::Loop
                        | super::types::CharacterRetryStage::Matting
                        | super::types::CharacterRetryStage::Consistency
                )
            })
        });
    let video_workflow = legacy_video_workflow || stable_video_workflow;
    let keyframe_workflow = request.workflow.id == "topdown-keyframes"
        && matches!(
            request.workflow.version.as_str(),
            "2.0.0" | "2.1.0" | "2.2.0" | "2.3.0"
        );
    let keypose_workflow = request.workflow.id == "topdown-keyposes"
        && matches!(request.workflow.version.as_str(), "2.4.0" | "2.5.0");
    let sprite_sheet_workflow =
        request.workflow.id == "topdown-spritesheet" && request.workflow.version == "3.0.0";
    let locked_frames_workflow =
        request.workflow.id == "topdown-frames" && request.workflow.version == "4.0.0";
    if !video_workflow
        && !keyframe_workflow
        && !keypose_workflow
        && !sprite_sheet_workflow
        && !locked_frames_workflow
        && !locked_video_workflow
        && !video_cycle_workflow
        && !direction_pose_chain_workflow
        && !direction_motion_workflow
        && !topdown_cycle_workflow
        && !grid_generation_workflow
        && !grid_keyframe_workflow
        && !grid_structured_gait_workflow
    {
        return Err(PlanStoreError::InvalidRequest(
            "schema V3 supports topdown@1.0.0, topdown-video@2.0.0, topdown-keyframes@2.0.0 through @2.3.0, topdown-keyposes@2.4.0 through @2.5.0, topdown-spritesheet@3.0.0, topdown-frames@4.0.0, topdown-video-locked@5.0.0, topdown-video-cycle@6.0.0, topdown-direction-poses@7.0.0, topdown-direction-motion@8.0.0, topdown-grid@9.0.0 through @9.5.0, and topdown-cycle@10.0.0/@10.1.0"
                .into(),
        ));
    }
    if (grid_generation_workflow
        || grid_keyframe_workflow
        || grid_structured_gait_workflow
        || topdown_cycle_workflow)
        && !cfg!(feature = "grid-generation")
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown-grid@9.x and topdown-cycle@10.x require a Forge build with grid-generation enabled"
                .into(),
        ));
    }
    if stable_video_workflow
        || locked_video_workflow
        || video_cycle_workflow
        || direction_pose_chain_workflow
        || direction_motion_workflow
        || topdown_cycle_workflow
        || grid_generation_workflow
        || grid_keyframe_workflow
        || grid_structured_gait_workflow
    {
        match request.camera_profile {
            Some(CharacterCameraProfileV1::TopdownOrthographic)
            | Some(CharacterCameraProfileV1::TopdownThreeQuarter) => {}
            Some(CharacterCameraProfileV1::LegacyThreeQuarter) | None => {
                return Err(PlanStoreError::InvalidRequest(
                    "direction-locked video, V8/V9, and topdown-cycle@10.0.0 workflows require cameraProfile topdown-orthographic@2.0.0 or topdown-three-quarter@1.0.0"
                        .into(),
                ));
            }
        }
    }
    if !(1..=2).contains(&request.generation.max_attempts_per_animation) {
        return Err(PlanStoreError::InvalidRequest(
            "generation.maxAttemptsPerAnimation must be 1 or 2".into(),
        ));
    }
    if topdown_cycle_workflow && request.generation.max_attempts_per_animation != 1 {
        return Err(PlanStoreError::InvalidRequest(
            "topdown-cycle@10.x is single-shot and requires maxAttemptsPerAnimation 1".into(),
        ));
    }
    if !(2..=24).contains(&request.generation.target_frame_count) {
        return Err(PlanStoreError::InvalidRequest(
            "generation.targetFrameCount must be between 2 and 24".into(),
        ));
    }
    if !(1..=15).contains(&request.generation.video_duration_seconds) {
        return Err(PlanStoreError::InvalidRequest(
            "generation.videoDurationSeconds must be between 1 and 15".into(),
        ));
    }
    for (label, model) in [
        ("imageModel", request.generation.image_model.as_deref()),
        ("videoModel", request.generation.video_model.as_deref()),
    ] {
        if model.is_some_and(|value| {
            value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control)
        }) {
            return Err(PlanStoreError::InvalidRequest(format!(
                "generation.{label} must be a printable model identifier"
            )));
        }
    }
    if let Some(path) = &request.character.reference_image_path {
        validate_png(path)?;
    }
    let style_lock = request
        .style_lock_path
        .as_ref()
        .map(|path| read_style_lock(path))
        .transpose()
        .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
    if direction_motion_workflow {
        match request.direction_motion_stage {
            DirectionMotionGenerationStageV1::ImageLocks => {
                if let Some(source) = request.reuse_from_job_dir.as_deref() {
                    if request.retry_animations.is_empty()
                        || request.retry_stages.len() != request.retry_animations.len()
                        || !request.retry_animations.iter().all(|animation| {
                            matches!(
                                request.retry_stages.get(animation),
                                Some(
                                    super::types::CharacterRetryStage::Auto
                                        | super::types::CharacterRetryStage::Still,
                                )
                            )
                        })
                    {
                        return Err(PlanStoreError::InvalidRequest(
                            "V8 image-lock retry requires retryAnimations with auto or still stages"
                                .into(),
                        ));
                    }
                    DirectionMotionLockV1::image_retry_closure(&request.retry_animations)
                        .map_err(PlanStoreError::InvalidRequest)?;
                    for relative in [
                        "job.json",
                        "source/direction-motion-lock.json",
                        "source/direction-motion-provider-manifest.json",
                        "direction-motion-lock",
                        "direction-motion-lock/generation-masters",
                    ] {
                        if !source.join(relative).exists() {
                            return Err(PlanStoreError::InvalidRequest(format!(
                                "V8 image-lock retry source is missing {relative}"
                            )));
                        }
                    }
                } else if !request.retry_animations.is_empty() {
                    return Err(PlanStoreError::InvalidRequest(
                        "V8 image-lock retry requires reuseFromJobDir".into(),
                    ));
                }
            }
            DirectionMotionGenerationStageV1::Complete => {
                let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "V8 complete stage requires an explicitly approved image-lock Job".into(),
                    )
                })?;
                let required = if direction_motion_local_only {
                    vec![
                        "job.json",
                        "source/direction-motion-lock.json",
                        "source/direction-motion-approval.json",
                        "source/direction-motion-provider-manifest.json",
                    ]
                } else {
                    vec![
                        "job.json",
                        "source/direction-motion-lock.json",
                        crate::character_direction_motion::DIRECTION_MOTION_APPROVAL_FILE,
                    ]
                };
                for relative in required {
                    if !source.join(relative).is_file() {
                        return Err(PlanStoreError::InvalidRequest(format!(
                            "approved V8 image-lock source is missing {relative}"
                        )));
                    }
                }
            }
        }
        if !(8..=16).contains(&request.generation.target_frame_count) {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-direction-motion@8.0.0 targetFrameCount must be 8..=16; it is an extraction preference, not a forced resample count"
                    .into(),
            ));
        }
        if request.validation_only {
            if request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete {
                return Err(PlanStoreError::InvalidRequest(
                    "V8 validation requires the separately approved complete stage".into(),
                ));
            }
            if request.validation_animations.len() != 1
                || !matches!(
                    request.validation_animations[0].as_str(),
                    "walk_down" | "walk_up" | "walk_right" | "walk_left"
                )
            {
                return Err(PlanStoreError::InvalidRequest(
                    "V8 validation requires exactly one walk animation".into(),
                ));
            }
        }
        if let Some(path) = request.subject_lock_path.as_ref() {
            let subject = read_subject_lock(path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if subject.provider_id != request.provider_id
                || subject.profile_id != request.profile_id
            {
                return Err(PlanStoreError::InvalidRequest(
                    "optional V8 SubjectLock must use the planned Provider profile".into(),
                ));
            }
        }
    }
    if topdown_cycle_workflow {
        if topdown_cycle_v101_workflow {
            if request.validation_only
                && request.validation_animations == ["walk_down"]
                && request.metadata.default_animation == "walk_down"
                && request.direction_motion_stage == DirectionMotionGenerationStageV1::Complete
                && request.generation.target_frame_count == 12
                && request.generation.video_model.is_some()
                && request.motion_profile
                    == crate::character_direction_motion::CharacterMotionProfileV1::BipedWalk
                && request.generation.pose_guidance
                    == crate::character_grid::GridPoseGuidanceV1::Disabled
                && request.equipment_explicit
                && request.retry_animations == ["walk_down"]
                && request.retry_stages.len() == 1
                && request.retry_stages.get("walk_down")
                    == Some(&super::types::CharacterRetryStage::Loop)
                && request.retry_frames.is_empty()
            {
                let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "topdown-cycle@10.1.0 requires reuseFromJobDir for one failed V10 walk_down probe"
                            .into(),
                    )
                })?;
                validate_topdown_cycle_v101_source(request, source)?;
            } else {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown_cycle_v101_scope_invalid: V10.1 is validationOnly walk_down, target 12, loop-stage local replay with exactly one failed V10 source and no frame retry"
                        .into(),
                ));
            }
        } else {
            let real_walk_down_probe = request.provider_id == "xai"
                && request.profile_id == "default"
                && request.validation_only
                && request.validation_animations == ["walk_down"]
                && request.metadata.default_animation == "walk_down"
                && request.generation.video_duration_seconds == 4
                && request.generation.video_model.as_deref() == Some("grok-imagine-video-1.5");
            if request.provider_id != "fixture" && !real_walk_down_probe {
                return Err(PlanStoreError::InvalidRequest(
                "topdown_cycle_v10_real_scope_forbidden: only the independently authorized xai/default grok-imagine-video-1.5 validationOnly walk_down 1/1 probe is enabled; full or other real routes remain forbidden"
                    .into(),
            ));
            }
            if request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete {
                return Err(PlanStoreError::InvalidRequest(
                "topdown-cycle@10.0.0 is complete-only and reuses an approved V9 DirectionGridLock"
                    .into(),
            ));
            }
            if request.generation.target_frame_count != 12 {
                return Err(PlanStoreError::InvalidRequest(
                "topdown-cycle@10.0.0 requires targetFrameCount 12 as the adaptive 8/10/12 source-cycle ceiling"
                    .into(),
            ));
            }
            if request.generation.video_model.is_none() {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-cycle@10.0.0 requires an explicitly locked videoModel".into(),
                ));
            }
            if request.motion_profile
                != crate::character_direction_motion::CharacterMotionProfileV1::BipedWalk
            {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-cycle@10.0.0 requires motionProfile biped_walk".into(),
                ));
            }
            if request.generation.pose_guidance
                != crate::character_grid::GridPoseGuidanceV1::Disabled
            {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-cycle@10.0.0 forbids PoseStructure and requires poseGuidance disabled"
                        .into(),
                ));
            }
            if !request.equipment_explicit {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-cycle@10.0.0 requires an explicit equipment declaration".into(),
                ));
            }
            if !request.retry_animations.is_empty()
                || !request.retry_stages.is_empty()
                || !request.retry_frames.is_empty()
            {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-cycle@10.0.0 initial fixture acceptance does not permit retries"
                        .into(),
                ));
            }
            if request.validation_only {
                if request.validation_animations != ["walk_down"]
                    || request.metadata.default_animation != "walk_down"
                {
                    return Err(PlanStoreError::InvalidRequest(
                    "topdown-cycle@10.0.0 validation requires exactly walk_down and defaultAnimation walk_down"
                        .into(),
                ));
                }
            } else if !request.validation_animations.is_empty()
                || request.metadata.default_animation != "idle_down"
            {
                return Err(PlanStoreError::InvalidRequest(
                "complete topdown-cycle@10.0.0 requires defaultAnimation idle_down and no validationAnimations"
                    .into(),
            ));
            }
            let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                "topdown-cycle@10.0.0 requires reuseFromJobDir for an approved V9 DirectionGridLock"
                    .into(),
            )
            })?;
            validate_topdown_cycle_direction_source(request, source)?;
        }
    }
    if grid_generation_workflow {
        if request.direction_grid_cape_contract.is_some() {
            if !is_front_authoritative_direction_grid(request) {
                return Err(PlanStoreError::InvalidRequest(
                    "front_authoritative_direction_grid_scope_invalid: requires ImageLocks, an approved source, only direction_grid at still stage, targetFrameCount 4, maxAttemptsPerAnimation 1, no frames, and no validation"
                        .into(),
                ));
            }
            let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "front_authoritative_direction_grid_source_missing: approved V9 DirectionGrid source is required"
                        .into(),
                )
            })?;
            validate_topdown_cycle_direction_source(request, source)?;
        }
        if request.generation.pose_guidance == crate::character_grid::GridPoseGuidanceV1::Grayscale
        {
            return Err(PlanStoreError::InvalidRequest(
                "poseGuidance grayscale is reserved for topdown-grid@9.2.0 through @9.5.0".into(),
            ));
        }
        match request.direction_motion_stage {
            DirectionMotionGenerationStageV1::ImageLocks => {
                if let Some(source) = request.reuse_from_job_dir.as_deref() {
                    if request.retry_animations != ["direction_grid"]
                        || request.retry_stages.get("direction_grid")
                            != Some(&super::types::CharacterRetryStage::Still)
                        || request.retry_stages.len() != 1
                        || !request.retry_frames.is_empty()
                    {
                        return Err(PlanStoreError::InvalidRequest(
                            "topdown-grid@9.0.0 direction retry requires only direction_grid at still stage"
                                .into(),
                        ));
                    }
                    for relative in [
                        "job.json",
                        "source/direction-grid-lock.json",
                        "direction-grid/contact-sheet.png",
                    ] {
                        if !source.join(relative).is_file() {
                            return Err(PlanStoreError::InvalidRequest(format!(
                                "topdown-grid direction retry source is missing {relative}"
                            )));
                        }
                    }
                } else if !request.retry_animations.is_empty()
                    || !request.retry_stages.is_empty()
                    || !request.retry_frames.is_empty()
                {
                    return Err(PlanStoreError::InvalidRequest(
                        "topdown-grid@9.0.0 direction retry requires reuseFromJobDir".into(),
                    ));
                }
            }
            DirectionMotionGenerationStageV1::Complete => {
                let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "topdown-grid@9.0.0 action_grid requires an approved direction_grid Job"
                            .into(),
                    )
                })?;
                let approval_relative = if request.retry_frames.is_empty() {
                    crate::character_grid::GRID_APPROVAL_FILE
                } else {
                    "source/direction-grid-approval.json"
                };
                for relative in [
                    "job.json",
                    "source/direction-grid-lock.json",
                    approval_relative,
                ] {
                    if !source.join(relative).is_file() {
                        return Err(PlanStoreError::InvalidRequest(format!(
                            "approved topdown-grid source is missing {relative}"
                        )));
                    }
                }
                if !request.retry_frames.is_empty() {
                    if request.retry_animations.len() != 1
                        || !matches!(
                            request.retry_animations[0].as_str(),
                            "walk_down" | "walk_up" | "walk_right" | "walk_left"
                        )
                    {
                        return Err(PlanStoreError::InvalidRequest(
                            "topdown-grid@9.0.0 frame retry requires exactly one walk animation"
                                .into(),
                        ));
                    }
                    let animation = &request.retry_animations[0];
                    if request.retry_stages.get(animation)
                        != Some(&super::types::CharacterRetryStage::Frame)
                    {
                        return Err(PlanStoreError::InvalidRequest(
                            "topdown-grid@9.0.0 frame retry requires --stage frame".into(),
                        ));
                    }
                    for relative in [
                        "source/grid-provider-manifest.json",
                        "source/direction-grid-lock.json",
                        "source/direction-grid-approval.json",
                    ] {
                        if !source.join(relative).is_file() {
                            return Err(PlanStoreError::InvalidRequest(format!(
                                "topdown-grid frame retry source is missing {relative}"
                            )));
                        }
                    }
                } else if !request.retry_animations.is_empty() || !request.retry_stages.is_empty() {
                    return Err(PlanStoreError::InvalidRequest(
                        "topdown-grid@9.0.0 complete stage supports only targeted frame retries"
                            .into(),
                    ));
                }
            }
        }
        if image2_requested {
            let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "xai_image2_candidate_source_missing: approved V9 DirectionGrid source is required"
                        .into(),
                )
            })?;
            validate_topdown_cycle_direction_source(request, source)?;
        }
        if request.generation.target_frame_count != 4 {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.0.0 requires targetFrameCount 4".into(),
            ));
        }
        if request.validation_only {
            if request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete
                || request.validation_animations.len() != 1
                || !matches!(
                    request.validation_animations[0].as_str(),
                    "walk_down" | "walk_up" | "walk_right" | "walk_left"
                )
                || !request.retry_animations.is_empty()
                || !request.retry_stages.is_empty()
                || !request.retry_frames.is_empty()
            {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-grid@9.0.0 validation requires the approved complete stage, exactly one walk animation, and no retry scope"
                        .into(),
                ));
            }
        } else if !request.validation_animations.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.0.0 validationAnimations requires validationOnly".into(),
            ));
        }
    }
    if grid_keyframe_workflow {
        if request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.1.0 is complete-only and requires an approved V9.0 Direction Grid"
                    .into(),
            ));
        }
        let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
            PlanStoreError::InvalidRequest(
                "topdown-grid@9.1.0 requires an approved V9.0 Direction Grid Job".into(),
            )
        })?;
        let approval_relative = if request.retry_frames.is_empty() {
            crate::character_grid::GRID_APPROVAL_FILE
        } else {
            "source/direction-grid-approval.json"
        };
        for relative in [
            "job.json",
            "source/direction-grid-lock.json",
            approval_relative,
        ] {
            if !source.join(relative).is_file() {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "approved V9.0 Direction Grid source is missing {relative}"
                )));
            }
        }
        if request.generation.target_frame_count != 4 {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.1.0 requires exactly four keyframes per action".into(),
            ));
        }
        if request.generation.pose_guidance != crate::character_grid::GridPoseGuidanceV1::Disabled {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.1.0 requires poseGuidance disabled".into(),
            ));
        }
        if request.validation_only {
            if request.validation_animations.len() != 1
                || !matches!(
                    request.validation_animations[0].as_str(),
                    "walk_down" | "walk_up" | "walk_right" | "walk_left"
                )
            {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-grid@9.1.0 validation requires exactly one walk animation".into(),
                ));
            }
        } else if !request.validation_animations.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.1.0 validationAnimations requires validationOnly".into(),
            ));
        }
        if !request.retry_frames.is_empty() {
            if request.retry_animations.len() != 1
                || request.retry_stages.get(&request.retry_animations[0])
                    != Some(&super::types::CharacterRetryStage::Frame)
                || request.retry_stages.len() != 1
            {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-grid@9.1.0 child retry requires one walk animation at frame stage"
                        .into(),
                ));
            }
            for relative in [
                "source/grid-keyframe-provider-manifest.json",
                "source/direction-grid-lock.json",
                "source/direction-grid-approval.json",
            ] {
                if !source.join(relative).is_file() {
                    return Err(PlanStoreError::InvalidRequest(format!(
                        "topdown-grid@9.1.0 frame retry source is missing {relative}"
                    )));
                }
            }
        } else if !request.retry_animations.is_empty() || !request.retry_stages.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.1.0 supports only explicit frame retries".into(),
            ));
        }
    }
    if grid_structured_gait_workflow {
        if request.workflow.version == "9.3.0" && request.retry_frames.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.3.0 is only an evidence-selected V9.2 walk_down frame-2 remediation child"
                    .into(),
            ));
        }
        if request.workflow.version == "9.4.0" && request.retry_frames.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.4.0 is only an evidence-selected V9.3 walk_down frame-2 platform-safe remediation child"
                    .into(),
            ));
        }
        if request.workflow.version == "9.5.0" && request.retry_frames.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-grid@9.5.0 is only the manually selected V9.4 walk_down frame-2 footwear cleanup child"
                    .into(),
            ));
        }
        if request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete {
            return Err(PlanStoreError::InvalidRequest(
                "structured topdown-grid is complete-only and requires an approved V9.0 Direction Grid"
                    .into(),
            ));
        }
        let requested_source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
            PlanStoreError::InvalidRequest(
                "structured topdown-grid requires an approved V9.0 Direction Grid Job".into(),
            )
        })?;
        let v95_source = if request.workflow.version == "9.5.0" {
            Some(validate_v94_footwear_cleanup_child_source_with_evidence(
                requested_source,
                request
                    .retry_frames
                    .get("walk_down")
                    .map(Vec::as_slice)
                    .unwrap_or_default(),
            )?)
        } else {
            None
        };
        let source = v95_source
            .as_ref()
            .map(|validated| validated.material_source_job_dir.as_path())
            .unwrap_or(requested_source);
        let approval_relative = if request.retry_frames.is_empty() {
            crate::character_grid::GRID_APPROVAL_FILE
        } else {
            "source/direction-grid-approval.json"
        };
        for relative in [
            "job.json",
            "source/direction-grid-lock.json",
            approval_relative,
        ] {
            if !source.join(relative).is_file() {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "approved V9.0 Direction Grid source is missing {relative}"
                )));
            }
        }
        if request.generation.target_frame_count != 4 {
            return Err(PlanStoreError::InvalidRequest(
                "structured topdown-grid requires exactly four keyframes".into(),
            ));
        }
        if request.generation.pose_guidance != crate::character_grid::GridPoseGuidanceV1::Grayscale
        {
            return Err(PlanStoreError::InvalidRequest(
                "structured topdown-grid requires poseGuidance grayscale".into(),
            ));
        }
        if !request.validation_only
            || request.validation_animations != ["walk_down"]
            || request.metadata.default_animation != "walk_down"
        {
            return Err(PlanStoreError::InvalidRequest(
                "structured topdown-grid is validation-only and accepts exactly walk_down".into(),
            ));
        }
        if !request.retry_frames.is_empty() {
            if request.retry_animations != ["walk_down"]
                || request.retry_stages.get("walk_down")
                    != Some(&super::types::CharacterRetryStage::Frame)
                || request.retry_stages.len() != 1
            {
                return Err(PlanStoreError::InvalidRequest(
                    "structured topdown-grid child validation requires only walk_down at frame stage"
                        .into(),
                ));
            }
            let selected = request.retry_frames.get("walk_down").ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "structured topdown-grid child validation requires one selected frame".into(),
                )
            })?;
            if request.workflow.version == "9.5.0" {
                validate_v94_footwear_cleanup_child_source(source, selected)?;
            } else if request.workflow.version == "9.4.0" {
                validate_v93_platform_child_source(source, selected)?;
            } else {
                validate_v92_laterality_child_source(source, selected)?;
            }
        } else if !request.retry_animations.is_empty() || !request.retry_stages.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "structured topdown-grid supports only an evidence-selected laterality frame retry"
                    .into(),
            ));
        }
    }
    if stable_video_workflow
        || locked_video_workflow
        || video_cycle_workflow
        || direction_pose_chain_workflow
        || topdown_cycle_workflow
        || keyframe_workflow
        || keypose_workflow
        || sprite_sheet_workflow
        || locked_frames_workflow
        || grid_generation_workflow
        || grid_keyframe_workflow
        || grid_structured_gait_workflow
    {
        if !topdown_cycle_workflow {
            let path = request.subject_lock_path.as_ref().ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "topdown-video/keyframe/keypose/spritesheet/frames workflows require subjectLockPath"
                        .into(),
                )
            })?;
            let subject = read_subject_lock(path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if subject.provider_id != request.provider_id
                || subject.profile_id != request.profile_id
            {
                return Err(PlanStoreError::InvalidRequest(
                    "character generation must use the Provider and profile locked by SubjectLock"
                        .into(),
                ));
            }
            if let Some(style) = style_lock.as_ref() {
                if subject.style_revision != style.revision {
                    return Err(PlanStoreError::InvalidRequest(
                        "subject_style_revision_mismatch: SubjectLock and StyleLock revisions differ"
                            .into(),
                    ));
                }
                if subject.style_sha256 != style.board_sha256 {
                    return Err(PlanStoreError::InvalidRequest(
                        "subject_style_hash_mismatch: SubjectLock is bound to a different style board"
                            .into(),
                    ));
                }
            }
        }
        let required_frame_count = if topdown_cycle_workflow {
            12
        } else if keypose_workflow
            || sprite_sheet_workflow
            || locked_frames_workflow
            || grid_generation_workflow
            || grid_keyframe_workflow
            || grid_structured_gait_workflow
        {
            4
        } else {
            8
        };
        if request.generation.target_frame_count != required_frame_count {
            return Err(PlanStoreError::InvalidRequest(format!(
                "{}@{} requires exactly {required_frame_count} frames per action",
                request.workflow.id, request.workflow.version
            )));
        }
        if stable_video_workflow
            || locked_video_workflow
            || video_cycle_workflow
            || direction_pose_chain_workflow
            || topdown_cycle_workflow
            || matches!(
                request.workflow.version.as_str(),
                "2.2.0"
                    | "2.3.0"
                    | "2.4.0"
                    | "2.5.0"
                    | "3.0.0"
                    | "4.0.0"
                    | "9.0.0"
                    | "9.1.0"
                    | "9.2.0"
                    | "9.3.0"
                    | "9.4.0"
                    | "9.5.0"
            )
        {
            if !request.equipment_explicit {
                return Err(PlanStoreError::InvalidRequest(
                    "topdown-video@2.0.0, topdown-video-locked@5.0.0, topdown-video-cycle@6.0.0, topdown-direction-poses@7.0.0, topdown-keyframes@2.2.0/@2.3.0, topdown-keyposes@2.4.0/@2.5.0, topdown-spritesheet@3.0.0, and topdown-frames@4.0.0 require an explicit equipment declaration"
                        .into(),
                ));
            }
            match request.equipment.kind {
                CharacterEquipmentKindV1::None => {
                    if request.equipment.reference_image.is_some() {
                        return Err(PlanStoreError::InvalidRequest(
                            "equipment.referenceImage is forbidden when equipment.kind is none"
                                .into(),
                        ));
                    }
                }
                CharacterEquipmentKindV1::StaffLike => {
                    let path = request.equipment.reference_image.as_ref().ok_or_else(|| {
                        PlanStoreError::InvalidRequest(
                            "staff_like equipment requires a clean referenceImage".into(),
                        )
                    })?;
                    validate_png(path)?;
                }
            }
        } else if request.equipment.kind != CharacterEquipmentKindV1::None
            || request.equipment.reference_image.is_some()
        {
            return Err(PlanStoreError::InvalidRequest(
                "explicit Character equipment requires topdown-keyframes@2.2.0 or @2.3.0, topdown-keyposes@2.4.0/@2.5.0, topdown-spritesheet@3.0.0, topdown-frames@4.0.0, topdown-video@2.0.0, topdown-video-locked@5.0.0, topdown-video-cycle@6.0.0, or topdown-direction-poses@7.0.0".into(),
            ));
        }
    } else if request.subject_lock_path.is_some()
        && !direction_motion_workflow
        && !topdown_cycle_workflow
    {
        return Err(PlanStoreError::InvalidRequest(
            "subjectLockPath is reserved for topdown-video/keyframe/keypose/spritesheet/frames workflows"
                .into(),
        ));
    }
    let allowed = [
        "idle",
        "idle_down",
        "idle_up",
        "idle_right",
        "idle_left",
        "walk_up",
        "walk_right",
        "walk_left",
        "walk_down",
    ];
    if request.validation_only {
        if !(stable_video_workflow
            || locked_video_workflow
            || video_cycle_workflow
            || direction_pose_chain_workflow
            || direction_motion_workflow
            || topdown_cycle_workflow
            || grid_generation_workflow
            || grid_keyframe_workflow
            || grid_structured_gait_workflow
            || sprite_sheet_workflow
            || locked_frames_workflow
            || keypose_workflow
            || (keyframe_workflow
                && matches!(request.workflow.version.as_str(), "2.2.0" | "2.3.0")))
        {
            return Err(PlanStoreError::InvalidRequest(
                "validationOnly requires topdown-keyframes@2.2.0 or @2.3.0, topdown-keyposes@2.4.0/@2.5.0, topdown-spritesheet@3.0.0, topdown-frames@4.0.0, topdown-video@2.0.0, topdown-video-locked@5.0.0, topdown-video-cycle@6.0.0, topdown-direction-poses@7.0.0, topdown-direction-motion@8.0.0, or topdown-grid@9.x"
                    .into(),
            ));
        }
        if request.validation_animations.len() != 1
            || request
                .validation_animations
                .iter()
                .any(|name| !allowed.contains(&name.as_str()))
        {
            return Err(PlanStoreError::InvalidRequest(
                "validationOnly requires exactly one supported validationAnimation".into(),
            ));
        }
        let has_retry = !request.retry_animations.is_empty()
            || !request.retry_stages.is_empty()
            || !request.retry_frames.is_empty();
        if has_retry {
            let validation_animation = &request.validation_animations[0];
            let retry_scope_matches = request.retry_animations.len() == 1
                && request.retry_animations[0] == *validation_animation
                && request
                    .retry_stages
                    .keys()
                    .all(|name| name == validation_animation)
                && request
                    .retry_frames
                    .keys()
                    .all(|name| name == validation_animation);
            if request.reuse_from_job_dir.is_none() || !retry_scope_matches {
                return Err(PlanStoreError::InvalidRequest(
                    "validationOnly retry must reuse exactly its validationAnimation".into(),
                ));
            }
        } else if request.reuse_from_job_dir.is_some()
            && !direction_motion_workflow
            && !topdown_cycle_workflow
            && !grid_generation_workflow
            && !grid_keyframe_workflow
            && !grid_structured_gait_workflow
        {
            return Err(PlanStoreError::InvalidRequest(
                "validationOnly reuse requires an explicit retry stage".into(),
            ));
        }
        if request.metadata.default_animation != request.validation_animations[0] {
            return Err(PlanStoreError::InvalidRequest(
                "validationOnly metadata.defaultAnimation must match validationAnimation".into(),
            ));
        }
    } else {
        if !request.validation_animations.is_empty() {
            return Err(PlanStoreError::InvalidRequest(
                "validationAnimations requires validationOnly".into(),
            ));
        }
        let expected_default = if direction_motion_workflow
            || topdown_cycle_workflow
            || grid_generation_workflow
            || grid_keyframe_workflow
            || grid_structured_gait_workflow
        {
            "idle_down"
        } else {
            "idle"
        };
        if request.metadata.default_animation != expected_default {
            return Err(PlanStoreError::InvalidRequest(format!(
                "generated top-down packs require metadata.defaultAnimation \"{expected_default}\""
            )));
        }
    }
    let retry_name_allowed = |name: &str| {
        allowed.contains(&name)
            || (grid_generation_workflow
                && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
                && name == "direction_grid")
    };
    if request
        .retry_animations
        .iter()
        .any(|name| !retry_name_allowed(name))
    {
        return Err(PlanStoreError::InvalidRequest(
            "retryAnimations contains an unknown topdown animation".into(),
        ));
    }
    if request
        .retry_stages
        .iter()
        .any(|(name, _)| !retry_name_allowed(name) || !request.retry_animations.contains(name))
    {
        return Err(PlanStoreError::InvalidRequest(
            "retryStages keys must name animations present in retryAnimations".into(),
        ));
    }
    if (locked_video_workflow
        || video_cycle_workflow
        || direction_pose_chain_workflow
        || topdown_cycle_workflow)
        && request.retry_stages.values().any(|stage| {
            matches!(
                stage,
                super::types::CharacterRetryStage::Still | super::types::CharacterRetryStage::Frame
            )
        })
    {
        return Err(PlanStoreError::InvalidRequest(
            "direction-locked video workflows keep DirectionLock immutable; retry video, loop, matting, consistency, or auto"
                .into(),
        ));
    }
    let retry_frame_count = if keypose_workflow
        || sprite_sheet_workflow
        || locked_frames_workflow
        || grid_generation_workflow
        || grid_keyframe_workflow
        || grid_structured_gait_workflow
    {
        4
    } else {
        8
    };
    for (animation, frames) in &request.retry_frames {
        if !request.retry_animations.contains(animation)
            || frames.is_empty()
            || frames.len() > retry_frame_count
            || frames.iter().any(|frame| *frame >= retry_frame_count as u8)
        {
            return Err(PlanStoreError::InvalidRequest(format!(
                "retryFrames must contain unique frame indices 0..={} for a retried animation",
                retry_frame_count - 1
            )));
        }
        let unique = frames.iter().copied().collect::<HashSet<_>>();
        if unique.len() != frames.len() {
            return Err(PlanStoreError::InvalidRequest(
                "retryFrames may not contain duplicate frame indices".into(),
            ));
        }
    }
    let retry_required_paths = if keyframe_workflow || keypose_workflow {
        ["workflow-graph.json", "source/provider-keyframes"]
    } else if sprite_sheet_workflow || locked_frames_workflow {
        [
            "source/spritesheet-provider-manifest.json",
            "source/provider-animation-sheets",
        ]
    } else if direction_pose_chain_workflow {
        ["source/direction-pose-lock.json", "direction-pose-lock"]
    } else if locked_video_workflow || video_cycle_workflow {
        ["source/direction-lock.json", "direction-lock"]
    } else {
        ["source/provider", "source/provider"]
    };
    if !direction_motion_workflow
        && !grid_generation_workflow
        && !grid_keyframe_workflow
        && !grid_structured_gait_workflow
        && !topdown_cycle_workflow
    {
        validate_retry_pair(
            request.reuse_from_job_dir.as_deref(),
            &request.retry_animations,
            &retry_required_paths,
        )?;
    }
    Ok(())
}

fn validate_provider_selection(provider_id: &str, profile_id: &str) -> Result<(), PlanStoreError> {
    if !is_engine_safe_name(provider_id) || !is_engine_safe_name(profile_id) {
        return Err(PlanStoreError::InvalidRequest(
            "providerId and profileId must contain only letters, numbers, '-' or '_'".into(),
        ));
    }
    Ok(())
}

pub(super) fn validate_topdown_cycle_v101_source(
    request: &GenerateCharacterPackRequest,
    source: &Path,
) -> Result<PathBuf, PlanStoreError> {
    let canonical_source = fs::canonicalize(source).map_err(|error| {
        PlanStoreError::InvalidRequest(format!(
            "topdown-cycle@10.1.0 source cannot be resolved: {error}"
        ))
    })?;
    let source_job: JobRecord = read_json(&canonical_source.join("job.json"))?;
    if canonical_source
        .file_name()
        .and_then(|value| value.to_str())
        != Some(source_job.job_id.as_str())
        || source_job.lifecycle_state != JobLifecycleState::Failed
        || source_job.error_code.as_deref() != Some("character_scale_lock_failed")
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_invalid: source must be the canonical failed V10 scale-lock Job"
                .into(),
        ));
    }
    let source_operation: AutomationOperation =
        serde_json::from_value(source_job.recipe.clone().ok_or_else(|| {
            PlanStoreError::InvalidRequest(
                "topdown_cycle_v101_source_invalid: source Job has no immutable recipe".into(),
            )
        })?)?;
    let source_recipe_hash = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&source_operation)?)
    );
    if source_job.recipe_hash.as_deref() != Some(source_recipe_hash.as_str()) {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_hash_mismatch: source recipe changed after execution".into(),
        ));
    }
    let AutomationOperation::GenerateCharacterPack(source_request) = source_operation else {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_invalid: source is not Character generation".into(),
        ));
    };
    if source_request.workflow.id != "topdown-cycle"
        || source_request.workflow.version != "10.0.0"
        || !source_request.validation_only
        || source_request.validation_animations != ["walk_down"]
        || source_request.metadata.default_animation != "walk_down"
        || !source_request.retry_animations.is_empty()
        || !source_request.retry_stages.is_empty()
        || !source_request.retry_frames.is_empty()
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_invalid: source must be the original V10 walk_down validation probe"
                .into(),
        ));
    }

    let mut expected = source_request.clone();
    expected.workflow.version = "10.1.0".into();
    expected.reuse_from_job_dir = Some(canonical_source.clone());
    expected.retry_animations = vec!["walk_down".into()];
    expected.retry_stages = [("walk_down".into(), super::types::CharacterRetryStage::Loop)].into();
    expected.retry_frames.clear();
    expected.metadata.name = request.metadata.name.clone();
    if &expected != request {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_mismatch: replay may change only workflow version, source binding, loop retry scope, and display name"
                .into(),
        ));
    }

    let artifact = |kind: &str, relative: &str| -> Result<PathBuf, PlanStoreError> {
        let path = canonical_source.join(relative);
        let canonical = fs::canonicalize(&path).map_err(|error| {
            PlanStoreError::InvalidRequest(format!(
                "topdown_cycle_v101_source_artifact_missing: {relative}: {error}"
            ))
        })?;
        if !canonical.starts_with(&canonical_source) {
            return Err(PlanStoreError::InvalidRequest(format!(
                "topdown_cycle_v101_source_artifact_escaped: {relative}"
            )));
        }
        let sha256 = hash_file(&canonical)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        let bound = source_job.artifacts.iter().any(|artifact| {
            artifact.kind == kind
                && artifact.path == canonical
                && artifact.sha256.as_deref() == Some(sha256.as_str())
        });
        if !bound {
            return Err(PlanStoreError::InvalidRequest(format!(
                "topdown_cycle_v101_source_hash_mismatch: {kind} is not bound by the source Job"
            )));
        }
        Ok(canonical)
    };
    let provider_manifest = artifact("provider_manifest", "source/provider-manifest.json")?;
    let scale_path = artifact("character_scale_lock", "character-scale-lock.json")?;
    artifact(
        "character_motion_semantics_report",
        "character-motion-semantics-report.json",
    )?;
    artifact(
        "character_silhouette_temporal_report",
        "character-silhouette-temporal-report.json",
    )?;
    artifact("provider_usage", "provider-usage.json")?;
    artifact("workflow_graph", "workflow-graph.json")?;

    let scale: CharacterScaleLockV1 = read_json(&scale_path)?;
    if scale.profile != CHARACTER_SCALE_LOCK_PROFILE
        || scale.workflow != "topdown-cycle@10.0.0"
        || scale.verdict != crate::asset_project::ConsistencyVerdict::Blocked
        || scale.reasons.is_empty()
        || scale.reasons.iter().any(|reason| {
            !reason.starts_with("body center drift px")
                && !reason.starts_with("foot baseline drift px")
        })
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_invalid: scale lock must be blocked only by center/baseline drift"
                .into(),
        ));
    }
    let manifest: serde_json::Value = read_json(&provider_manifest)?;
    if manifest.get("workflow").and_then(serde_json::Value::as_str) != Some("topdown-cycle@10.0.0")
        || manifest
            .get("providerId")
            .and_then(serde_json::Value::as_str)
            != Some(source_request.provider_id.as_str())
        || manifest
            .get("profileId")
            .and_then(serde_json::Value::as_str)
            != Some(source_request.profile_id.as_str())
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_manifest_mismatch: retained Provider manifest changed"
                .into(),
        ));
    }
    let video_path = manifest
        .pointer("/animations/walk_down/videoPath")
        .and_then(serde_json::Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| {
            PlanStoreError::InvalidRequest(
                "topdown_cycle_v101_source_video_missing: manifest has no walk_down clip".into(),
            )
        })?;
    let video_sha256 = manifest
        .pointer("/animations/walk_down/videoSha256")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            PlanStoreError::InvalidRequest(
                "topdown_cycle_v101_source_video_missing: manifest has no walk_down hash".into(),
            )
        })?;
    let canonical_video = fs::canonicalize(&video_path).map_err(|error| {
        PlanStoreError::InvalidRequest(format!("topdown_cycle_v101_source_video_missing: {error}"))
    })?;
    if !canonical_video.starts_with(&canonical_source)
        || hash_file(&canonical_video)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?
            != video_sha256
        || !source_job.artifacts.iter().any(|artifact| {
            artifact.kind == "provider_video_walk_down_attempt_1"
                && artifact.path == canonical_video
                && artifact.sha256.as_deref() == Some(video_sha256)
        })
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_video_hash_mismatch: retained clip is not source-bound"
                .into(),
        ));
    }

    let direction_source = source_request
        .reuse_from_job_dir
        .as_deref()
        .ok_or_else(|| {
            PlanStoreError::InvalidRequest(
            "topdown_cycle_v101_source_invalid: V10 source has no approved DirectionGrid parent"
                .into(),
        )
        })?;
    validate_topdown_cycle_direction_source(&source_request, direction_source)?;
    fs::canonicalize(direction_source).map_err(|error| {
        PlanStoreError::InvalidRequest(format!(
            "topdown_cycle_v101_direction_source_missing: {error}"
        ))
    })
}

pub(super) fn validate_topdown_cycle_direction_source(
    request: &GenerateCharacterPackRequest,
    source: &Path,
) -> Result<(), PlanStoreError> {
    let canonical_source = fs::canonicalize(source).map_err(|error| {
        PlanStoreError::InvalidRequest(format!(
            "topdown-cycle@10.0.0 source cannot be resolved: {error}"
        ))
    })?;
    let source_job: JobRecord = read_json(&canonical_source.join("job.json"))?;
    if is_front_authoritative_direction_grid(request)
        && source_job.lifecycle_state != crate::job::JobLifecycleState::Succeeded
    {
        return Err(PlanStoreError::InvalidRequest(
            "front_authoritative_direction_grid_source_invalid: source DirectionGrid Job is not succeeded"
                .into(),
        ));
    }
    if is_xai_image2_direction_grid_probe(request) {
        if source_job.lifecycle_state != crate::job::JobLifecycleState::Succeeded {
            return Err(PlanStoreError::InvalidRequest(
                "xai_image2_candidate_source_invalid: source DirectionGrid Job is not succeeded"
                    .into(),
            ));
        }
        let source_operation: AutomationOperation =
            serde_json::from_value(source_job.recipe.clone().ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "xai_image2_candidate_source_invalid: source Job has no immutable recipe"
                        .into(),
                )
            })?)?;
        let source_recipe_hash = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&source_operation)?)
        );
        let AutomationOperation::GenerateCharacterPack(source_request) = source_operation else {
            return Err(PlanStoreError::InvalidRequest(
                "xai_image2_candidate_source_invalid: source is not Character generation".into(),
            ));
        };
        if source_job.recipe_hash.as_deref() != Some(source_recipe_hash.as_str())
            || source_request.provider_id != "xai"
            || source_request.profile_id != "default"
            || source_request.workflow.id != "topdown-grid"
            || source_request.workflow.version != "9.0.0"
            || source_request.direction_motion_stage != DirectionMotionGenerationStageV1::ImageLocks
            || source_request.generation.image_model.as_deref()
                != Some("grok-imagine-image-quality")
        {
            return Err(PlanStoreError::InvalidRequest(
                "xai_image2_candidate_source_invalid: source must be an immutable xai/default Image Quality topdown-grid@9.0.0 DirectionGrid Job"
                .into(),
            ));
        }
        let source_was_initial = source_request.retry_animations.is_empty()
            && source_request.retry_stages.is_empty()
            && source_request.reuse_from_job_dir.is_none();
        let source_was_direction_retry = source_request.retry_animations == ["direction_grid"]
            && source_request.retry_stages.len() == 1
            && source_request.retry_stages.get("direction_grid")
                == Some(&super::types::CharacterRetryStage::Still)
            && source_request.retry_frames.is_empty()
            && source_request.reuse_from_job_dir.is_some();
        if !source_was_initial && !source_was_direction_retry {
            return Err(PlanStoreError::InvalidRequest(
                "xai_image2_candidate_source_invalid: approved source has an unsupported retry history"
                    .into(),
            ));
        }
        if source_was_direction_retry {
            let evidence_path = canonical_source.join("source/direction-grid-retry-evidence.json");
            let evidence_sha256 = hash_file(&evidence_path)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if source_job
                .artifacts
                .iter()
                .find(|artifact| artifact.kind == "direction_grid_retry_evidence")
                .and_then(|artifact| artifact.sha256.as_deref())
                != Some(evidence_sha256.as_str())
            {
                return Err(PlanStoreError::InvalidRequest(
                    "xai_image2_candidate_source_invalid: source retry prompt evidence is not hash-bound by its Job"
                        .into(),
                ));
            }
        }
    }
    let lock_path = canonical_source.join("source/direction-grid-lock.json");
    let lock: DirectionGridLockV1 = read_json(&lock_path)?;
    let lock_sha256 =
        hash_file(&lock_path).map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
    if !is_direction_grid_lock_profile(&lock.profile) || lock.workflow != GRID_WORKFLOW {
        return Err(PlanStoreError::InvalidRequest(
            "topdown-cycle@10.0.0 source must be an original approved topdown-grid@9.0.0 DirectionGridLock"
                .into(),
        ));
    }
    if request.provider_id != "fixture"
        && (lock.provider_id != "xai" || lock.profile_id != "default")
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown-cycle@10.0.0 real probe requires an approved xai/default V9 DirectionGridLock"
                .into(),
        ));
    }
    if is_xai_image2_direction_grid_probe(request)
        && lock.image_model != "grok-imagine-image-quality"
    {
        return Err(PlanStoreError::InvalidRequest(
            "xai_image2_candidate_source_invalid: approved source lock is not the Image Quality incumbent"
                .into(),
        ));
    }
    if is_front_authoritative_direction_grid(request)
        && request.generation.image_model.as_deref() != Some(lock.image_model.as_str())
    {
        return Err(PlanStoreError::InvalidRequest(
            "front_authoritative_direction_grid_model_mismatch: the child must retain the approved source image model"
                .into(),
        ));
    }
    if lock.camera_profile != request.camera_profile.unwrap_or_default()
        || lock.equipment_kind != request.equipment.kind
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown-cycle@10.0.0 camera/equipment contract does not match the approved DirectionGridLock"
                .into(),
        ));
    }
    if let Some(style_path) = request.style_lock_path.as_deref() {
        let style = read_style_lock(style_path)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        if style.revision != lock.style_revision {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-cycle@10.0.0 StyleLock revision does not match the approved DirectionGridLock"
                    .into(),
            ));
        }
    }
    if let Some(subject_path) = request.subject_lock_path.as_deref() {
        let subject = read_subject_lock(subject_path)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        if subject.id != lock.subject_id
            || subject.revision != lock.subject_revision
            || subject.canonical_sha256 != lock.subject_canonical_sha256
            || subject.style_revision != lock.style_revision
        {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-cycle@10.0.0 SubjectLock does not match the approved DirectionGridLock"
                    .into(),
            ));
        }
    }
    let expected_nodes = ["front_idle", "back_idle", "right_idle", "left_idle"];
    if lock.nodes.len() != expected_nodes.len()
        || expected_nodes
            .iter()
            .any(|node_id| lock.node(node_id).is_none())
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown-cycle@10.0.0 requires exactly four approved cardinal direction anchors".into(),
        ));
    }
    for node in &lock.nodes {
        for (label, path, expected_sha256) in [
            ("delivery", &node.path, &node.sha256),
            (
                "generation master",
                &node.generation_master_path,
                &node.generation_master_sha256,
            ),
        ] {
            let canonical = fs::canonicalize(path).map_err(|error| {
                PlanStoreError::InvalidRequest(format!(
                    "topdown-cycle@10.0.0 {} node {label} is unavailable: {error}",
                    node.node_id
                ))
            })?;
            if !canonical.starts_with(&canonical_source) {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "topdown-cycle@10.0.0 {} node {label} escaped the source Job",
                    node.node_id
                )));
            }
            let actual = hash_file(&canonical)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            if actual != *expected_sha256 {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "topdown-cycle@10.0.0 {} node {label} hash changed after approval",
                    node.node_id
                )));
            }
        }
    }
    let approval_path = [
        canonical_source.join(crate::character_grid::GRID_APPROVAL_FILE),
        canonical_source.join("source/direction-grid-approval.json"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .ok_or_else(|| {
        PlanStoreError::InvalidRequest(
            "topdown-cycle@10.0.0 source is missing DirectionGrid approval".into(),
        )
    })?;
    let approval: DirectionGridApprovalV1 = read_json(&approval_path)?;
    if approval.profile != GRID_APPROVAL_PROFILE
        || approval.source_job_id != source_job.job_id
        || !approval.matches_lock(&lock, &lock_sha256)
    {
        return Err(PlanStoreError::InvalidRequest(
            "topdown-cycle@10.0.0 DirectionGrid approval does not match the immutable source lock"
                .into(),
        ));
    }
    Ok(())
}

fn validate_v92_laterality_child_source(
    source: &Path,
    selected_frames: &[u8],
) -> Result<(), PlanStoreError> {
    crate::grid_retry_source::validate_v92_laterality_source_closure(source, selected_frames)
        .map(|_| ())
        .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))
}

fn validate_v93_platform_child_source(
    source: &Path,
    selected_frames: &[u8],
) -> Result<(), PlanStoreError> {
    crate::grid_retry_source::validate_v93_platform_source_closure(source, selected_frames)
        .map(|_| ())
        .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))
}

fn validate_v94_footwear_cleanup_child_source(
    source: &Path,
    selected_frames: &[u8],
) -> Result<(), PlanStoreError> {
    validate_v94_footwear_cleanup_child_source_with_evidence(source, selected_frames).map(|_| ())
}

fn validate_v94_footwear_cleanup_child_source_with_evidence(
    source: &Path,
    selected_frames: &[u8],
) -> Result<crate::grid_retry_source::ValidatedGridFootwearCleanupSourceV1, PlanStoreError> {
    crate::grid_retry_source::validate_v95_footwear_cleanup_source_closure(source, selected_frames)
        .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))
}

fn validate_static_asset_set_request(
    request: &GenerateStaticAssetSetRequest,
) -> Result<(), PlanStoreError> {
    if !matches!(request.schema_version.as_str(), "4" | "5") {
        return Err(PlanStoreError::InvalidRequest(
            "static asset generation requires schemaVersion \"4\" or \"5\"".into(),
        ));
    }
    validate_provider_selection(&request.provider_id, &request.profile_id)?;
    read_project(&request.project_path)
        .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
    let style = read_style_lock(&request.style_lock_path)
        .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
    if style.provider_id != request.provider_id || style.profile_id != request.profile_id {
        return Err(PlanStoreError::InvalidRequest(
            "static generation must use the Provider and profile locked by the style revision"
                .into(),
        ));
    }
    if !matches!(request.asset.schema_version.as_str(), "1" | "2")
        || !is_engine_safe_name(&request.asset.id)
        || request.asset.name.trim().is_empty()
        || request.asset.items.is_empty()
        || request.asset.items.len() > 64
    {
        return Err(PlanStoreError::InvalidRequest(
            "static asset spec requires schemaVersion 1 or 2, a safe id/name, and 1..=64 items"
                .into(),
        ));
    }
    let stage3_kind = matches!(
        request.asset.kind,
        crate::asset_project::StaticAssetKind::PortraitSet
            | crate::asset_project::StaticAssetKind::EquipmentSet
            | crate::asset_project::StaticAssetKind::DecalSet
    );
    if stage3_kind && request.collection_lock_path.is_none() {
        return Err(PlanStoreError::InvalidRequest(
            "portrait, equipment, and decal generation require collectionLockPath".into(),
        ));
    }
    if let Some(path) = &request.collection_lock_path {
        let collection = read_collection_lock(path)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        if collection.asset_kind != request.asset.kind
            || collection.style_revision != style.revision
            || collection.provider_id != request.provider_id
            || collection.profile_id != request.profile_id
        {
            return Err(PlanStoreError::InvalidRequest(
                "CollectionLock kind, Style revision, and Provider profile must match the request"
                    .into(),
            ));
        }
        if request.asset.kind == crate::asset_project::StaticAssetKind::PortraitSet {
            let locked_profile = collection
                .effective_portrait_framing_profile()
                .unwrap_or_default();
            if request.framing_profile.unwrap_or(locked_profile) != locked_profile {
                return Err(PlanStoreError::InvalidRequest(
                    "portrait framingProfile must match the immutable CollectionLock".into(),
                ));
            }
            if locked_profile == PortraitFramingProfileV1::FullBody
                && collection.grounding != CollectionGrounding::Feet
            {
                return Err(PlanStoreError::InvalidRequest(
                    "full_body portrait generation requires a feet-grounded CollectionLock".into(),
                ));
            }
        }
    }
    if request.asset.kind != crate::asset_project::StaticAssetKind::PortraitSet
        && request.framing_profile.is_some()
    {
        return Err(PlanStoreError::InvalidRequest(
            "framingProfile is reserved for portrait_set".into(),
        ));
    }
    if request.asset.kind == crate::asset_project::StaticAssetKind::PortraitSet {
        let subject_path = request.subject_lock_path.as_ref().ok_or_else(|| {
            PlanStoreError::InvalidRequest("portrait generation requires subjectLockPath".into())
        })?;
        let subject = read_subject_lock(subject_path)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        if subject.style_revision != style.revision
            || subject.provider_id != request.provider_id
            || subject.profile_id != request.profile_id
        {
            return Err(PlanStoreError::InvalidRequest(
                "Portrait SubjectLock must match Style and Provider profile".into(),
            ));
        }
    } else if request.subject_lock_path.is_some() {
        return Err(PlanStoreError::InvalidRequest(
            "subjectLockPath is reserved for portrait_set".into(),
        ));
    }
    if request
        .item_metadata
        .keys()
        .any(|id| !request.asset.items.iter().any(|item| &item.id == id))
    {
        return Err(PlanStoreError::InvalidRequest(
            "itemMetadata contains an id outside the immutable asset spec".into(),
        ));
    }
    if let Some(path) = &request.source_spec_path {
        let metadata = fs::symlink_metadata(path).map_err(|source| PlanStoreError::Io {
            path: path.clone(),
            source,
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(PlanStoreError::InvalidRequest(
                "sourceSpecPath must be a regular non-symlink file".into(),
            ));
        }
    }
    if !(1..=2).contains(&request.max_attempts_per_item) {
        return Err(PlanStoreError::InvalidRequest(
            "maxAttemptsPerItem must be 1 or 2".into(),
        ));
    }
    let mut ids = HashSet::new();
    for item in &request.asset.items {
        if !is_engine_safe_name(&item.id)
            || !ids.insert(item.id.clone())
            || item.name.trim().is_empty()
            || item.prompt.trim().is_empty()
            || item.prompt.len() > 4_000
            || item.prompt.chars().any(char::is_control)
        {
            return Err(PlanStoreError::InvalidRequest(format!(
                "invalid or duplicate static asset item: {}",
                item.id
            )));
        }
        if let Some(reference) = &item.reference_image {
            validate_png(reference)?;
        }
    }
    if request
        .retry_item_ids
        .iter()
        .any(|id| !request.asset.items.iter().any(|item| item.id == *id))
    {
        return Err(PlanStoreError::InvalidRequest(
            "retryItemIds contains an item outside the immutable asset spec".into(),
        ));
    }
    validate_portrait_phase_contract(request, &style)?;
    if !request.replacement_item_paths.is_empty() {
        if !request.consistency_recheck_only
            || request.replacement_item_paths.len() != request.retry_item_ids.len()
            || request
                .replacement_item_paths
                .keys()
                .any(|id| !request.retry_item_ids.contains(id))
        {
            return Err(PlanStoreError::InvalidRequest(
                "replacementItemPaths requires a local consistency recheck and exactly the retried items"
                    .into(),
            ));
        }
        for path in request.replacement_item_paths.values() {
            validate_png(path)?;
        }
    }
    let required_retry_paths: &[&str] =
        if request.resume_incomplete_static || request.consistency_recheck_only {
            &["normalized/static"]
        } else {
            &["consistency-report.json", "normalized/static"]
        };
    validate_retry_pair(
        request.reuse_from_job_dir.as_deref(),
        &request.retry_item_ids,
        required_retry_paths,
    )?;
    if request.resume_incomplete_static && request.consistency_recheck_only {
        return Err(PlanStoreError::InvalidRequest(
            "resumeIncompleteStatic cannot be combined with consistencyRecheckOnly".into(),
        ));
    }
    Ok(())
}

fn validate_portrait_phase_contract(
    request: &GenerateStaticAssetSetRequest,
    style: &crate::asset_project::StyleLockV1,
) -> Result<(), PlanStoreError> {
    use crate::asset_project::StaticAssetKind;

    let portrait_v2 =
        request.asset.kind == StaticAssetKind::PortraitSet && request.asset.schema_version == "2";
    if request.schema_version == "4"
        && request.portrait_phase != PortraitGenerationPhaseV1::LegacyAll
    {
        return Err(PlanStoreError::InvalidRequest(
            "two-phase Portrait generation requires request schemaVersion 5".into(),
        ));
    }
    if request.schema_version == "5"
        && (!portrait_v2 || request.portrait_phase == PortraitGenerationPhaseV1::LegacyAll)
    {
        return Err(PlanStoreError::InvalidRequest(
            "request schemaVersion 5 is reserved for two-phase Portrait V2 generation".into(),
        ));
    }
    if !portrait_v2 {
        if request.portrait_phase != PortraitGenerationPhaseV1::LegacyAll
            || request.neutral_reference_policy
                != PortraitNeutralReferencePolicyV1::LegacyThreeReference
            || request.portrait_base_parent_job_id.is_some()
        {
            return Err(PlanStoreError::InvalidRequest(
                "two-phase Portrait fields require a portrait_set schemaVersion 2 spec".into(),
            ));
        }
        return Ok(());
    }
    match request.portrait_phase {
        PortraitGenerationPhaseV1::LegacyAll => {
            if request.portrait_base_parent_job_id.is_some() {
                return Err(PlanStoreError::InvalidRequest(
                    "legacy_all Portrait generation cannot reference a base parent Job".into(),
                ));
            }
        }
        PortraitGenerationPhaseV1::BaseOnly => {
            if request.portrait_base_parent_job_id.is_some()
                || request.retry_item_ids.iter().any(|item| item != "neutral")
            {
                return Err(PlanStoreError::InvalidRequest(
                    "base_only Portrait generation may target only neutral and has no approved parent"
                        .into(),
                ));
            }
        }
        PortraitGenerationPhaseV1::Expressions => {
            if request.retry_item_ids.is_empty()
                || request.retry_item_ids.iter().any(|item| item == "neutral")
            {
                return Err(PlanStoreError::InvalidRequest(
                    "Portrait expressions require one or more non-neutral retry targets".into(),
                ));
            }
            let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "Portrait expressions require a reusable source Job".into(),
                )
            })?;
            let base_job_id = request
                .portrait_base_parent_job_id
                .as_deref()
                .ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "Portrait expressions require portraitBaseParentJobId".into(),
                    )
                })?;
            let jobs_root = source.parent().ok_or_else(|| {
                PlanStoreError::InvalidRequest(
                    "Portrait expression source is not inside a JobStore".into(),
                )
            })?;
            let base_job_dir = jobs_root.join(base_job_id);
            let base_record: JobRecord = read_json(&base_job_dir.join("job.json"))?;
            if base_record.job_id != base_job_id
                || base_record.lifecycle_state != JobLifecycleState::Succeeded
                || base_record
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.kind == "gsfpack")
            {
                return Err(PlanStoreError::InvalidRequest(
                    "PortraitBase parent must be an approved succeeded Job without a Pack".into(),
                ));
            }
            let base_operation: AutomationOperation =
                serde_json::from_value(base_record.recipe.clone().ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "PortraitBase parent has no immutable recipe".into(),
                    )
                })?)?;
            let AutomationOperation::GenerateStaticAssetSet(base_request) = base_operation else {
                return Err(PlanStoreError::InvalidRequest(
                    "PortraitBase parent is not a static Portrait Job".into(),
                ));
            };
            if base_request.portrait_phase != PortraitGenerationPhaseV1::BaseOnly {
                return Err(PlanStoreError::InvalidRequest(
                    "PortraitBase parent recipe is not base_only".into(),
                ));
            }
            let (lock, _approval) = validate_portrait_base_approval(&base_job_dir, base_job_id)
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
            let subject = request
                .subject_lock_path
                .as_deref()
                .map(read_subject_lock)
                .transpose()
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?
                .ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "Portrait expressions require SubjectLock closure".into(),
                    )
                })?;
            let collection = request
                .collection_lock_path
                .as_deref()
                .map(read_collection_lock)
                .transpose()
                .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?
                .ok_or_else(|| {
                    PlanStoreError::InvalidRequest(
                        "Portrait expressions require CollectionLock closure".into(),
                    )
                })?;
            let expected_model = request
                .image_model
                .clone()
                .or_else(|| style.image_model.clone());
            let expected_framing = request.framing_profile.unwrap_or_else(|| {
                collection
                    .effective_portrait_framing_profile()
                    .unwrap_or_default()
            });
            if lock.asset_id != request.asset.id
                || lock.provider_id != request.provider_id
                || lock.profile_id != request.profile_id
                || lock.model != expected_model
                || lock.style_revision != style.revision
                || lock.subject_id != subject.id
                || lock.subject_revision != subject.revision
                || lock.collection_id != collection.id
                || lock.collection_revision != collection.revision
                || lock.framing_profile != expected_framing
                || lock.neutral_reference_policy != request.neutral_reference_policy
            {
                return Err(PlanStoreError::InvalidRequest(
                    "approved PortraitBase provenance does not match this expression request"
                        .into(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_retry_pair(
    source: Option<&Path>,
    targets: &[String],
    required_paths: &[&str],
) -> Result<(), PlanStoreError> {
    match (source, targets.is_empty()) {
        (None, true) => return Ok(()),
        (None, false) | (Some(_), true) => {
            return Err(PlanStoreError::InvalidRequest(
                "targeted retry requires both reuseFromJobDir and a non-empty target list".into(),
            ))
        }
        (Some(_), false) => {}
    }
    let source = source.expect("validated above");
    if !source.is_dir() || !source.join("job.json").is_file() {
        return Err(PlanStoreError::InvalidRequest(format!(
            "retry source is not a Forge job directory: {}",
            source.display()
        )));
    }
    for relative in required_paths {
        if !source.join(relative).exists() {
            return Err(PlanStoreError::InvalidRequest(format!(
                "retry source is missing {relative}"
            )));
        }
    }
    let mut unique = HashSet::new();
    if targets.iter().any(|target| !unique.insert(target)) {
        return Err(PlanStoreError::InvalidRequest(
            "targeted retry entries must be unique".into(),
        ));
    }
    Ok(())
}

fn is_engine_safe_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn validate_video_clip(
    path: &Path,
    start_time_ms: u64,
    end_time_ms: Option<u64>,
    target_frame_count: u32,
) -> Result<(), PlanStoreError> {
    if !path.is_file() {
        return Err(PlanStoreError::InvalidRequest(format!(
            "video file does not exist: {}",
            path.display()
        )));
    }
    if !(2..=24).contains(&target_frame_count) {
        return Err(PlanStoreError::InvalidRequest(
            "video targetFrameCount must be between 2 and 24".into(),
        ));
    }
    let probe = probe_video(&ProbeVideoParams {
        input_path: path.to_path_buf(),
        configured_ffprobe_path: None,
        bundled_resource_path: None,
    })
    .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
    let duration_ms = (probe.duration_seconds * 1000.0).round().max(0.0) as u64;
    let resolved_end = end_time_ms.unwrap_or(duration_ms);
    if duration_ms == 0 || resolved_end <= start_time_ms || resolved_end > duration_ms + 1 {
        return Err(PlanStoreError::InvalidRequest(format!(
            "video clip range must be inside 0..={duration_ms}ms"
        )));
    }
    Ok(())
}

fn validate_godot_target_location(project: &Path, target: &Path) -> Result<(), PlanStoreError> {
    let canonical_project = fs::canonicalize(project).map_err(|source| PlanStoreError::Io {
        path: project.to_path_buf(),
        source,
    })?;
    let relative = target.strip_prefix(project).map_err(|_| {
        PlanStoreError::InvalidRequest("Godot target is outside the project".into())
    })?;
    let mut cursor = project.to_path_buf();
    for component in relative.components() {
        cursor.push(component.as_os_str());
        if cursor.exists() {
            let metadata = fs::symlink_metadata(&cursor).map_err(|source| PlanStoreError::Io {
                path: cursor.clone(),
                source,
            })?;
            if metadata.file_type().is_symlink() {
                return Err(PlanStoreError::InvalidRequest(format!(
                    "Godot target may not traverse a symbolic link: {}",
                    cursor.display()
                )));
            }
        }
    }

    let mut existing = target;
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            PlanStoreError::InvalidRequest("Godot target has no project ancestor".into())
        })?;
    }
    let canonical_existing = fs::canonicalize(existing).map_err(|source| PlanStoreError::Io {
        path: existing.to_path_buf(),
        source,
    })?;
    if !canonical_existing.starts_with(canonical_project) {
        return Err(PlanStoreError::InvalidRequest(
            "Godot target resolves outside the project".into(),
        ));
    }
    Ok(())
}

fn hash_godot_target_identity(
    hasher: &mut Sha256,
    project: &Path,
    target: &Path,
) -> Result<(), PlanStoreError> {
    validate_godot_target_location(project, target)?;
    let canonical_project = fs::canonicalize(project).map_err(|source| PlanStoreError::Io {
        path: project.to_path_buf(),
        source,
    })?;
    hasher.update(canonical_project.to_string_lossy().as_bytes());
    let mut existing = target;
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            PlanStoreError::InvalidRequest("Godot target has no project ancestor".into())
        })?;
    }
    let canonical_existing = fs::canonicalize(existing).map_err(|source| PlanStoreError::Io {
        path: existing.to_path_buf(),
        source,
    })?;
    hasher.update(canonical_existing.to_string_lossy().as_bytes());
    let marker = target.join(OWNERSHIP_MARKER);
    if marker.is_file() {
        hash_file_contents(hasher, &marker)?;
    } else {
        hasher.update(b"forge-target-unowned-or-new");
    }
    Ok(())
}

fn hash_files(hasher: &mut Sha256, paths: &[PathBuf]) -> Result<(), PlanStoreError> {
    for path in paths {
        let canonical = fs::canonicalize(path).map_err(|source| PlanStoreError::Io {
            path: path.clone(),
            source,
        })?;
        hasher.update(canonical.to_string_lossy().as_bytes());
        hash_file_contents(hasher, &canonical)?;
    }
    Ok(())
}

fn hash_directory(hasher: &mut Sha256, root: &Path) -> Result<(), PlanStoreError> {
    let root_metadata = fs::symlink_metadata(root).map_err(|source| PlanStoreError::Io {
        path: root.to_path_buf(),
        source,
    })?;
    if root_metadata.file_type().is_symlink() {
        return Err(PlanStoreError::InvalidRequest(format!(
            "directory input root is a symbolic link: {}",
            root.display()
        )));
    }
    let mut files = Vec::new();
    collect_files(root, root, &mut files)?;
    files.sort();
    hasher.update(b"forge-directory-hash-v2\0");
    for relative in files {
        let relative_text = relative.to_string_lossy();
        hasher.update(b"file\0");
        hasher.update((relative_text.len() as u64).to_le_bytes());
        hasher.update(relative_text.as_bytes());
        let contents = fs::read(root.join(&relative)).map_err(|source| PlanStoreError::Io {
            path: root.join(&relative),
            source,
        })?;
        hasher.update((contents.len() as u64).to_le_bytes());
        hasher.update(contents);
    }
    Ok(())
}

fn collect_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), PlanStoreError> {
    for entry in fs::read_dir(directory).map_err(|source| PlanStoreError::Io {
        path: directory.to_path_buf(),
        source,
    })? {
        let entry = entry.map_err(|source| PlanStoreError::Io {
            path: directory.to_path_buf(),
            source,
        })?;
        let metadata = entry.file_type().map_err(|source| PlanStoreError::Io {
            path: entry.path(),
            source,
        })?;
        if metadata.is_symlink() {
            return Err(PlanStoreError::InvalidRequest(format!(
                "directory input contains symbolic link: {}",
                entry.path().display()
            )));
        } else if metadata.is_dir() {
            collect_files(root, &entry.path(), files)?;
        } else if metadata.is_file() {
            files.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(&entry.path())
                    .to_path_buf(),
            );
        } else {
            return Err(PlanStoreError::InvalidRequest(format!(
                "directory input contains unsupported entry: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn hash_file_contents(hasher: &mut Sha256, path: &Path) -> Result<(), PlanStoreError> {
    let mut file = fs::File::open(path).map_err(|source| PlanStoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|source| PlanStoreError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    Ok(())
}

fn hash_serializable(value: &impl Serialize) -> Result<String, PlanStoreError> {
    let bytes = serde_json::to_vec(value)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn describe_effects(operation: &AutomationOperation) -> Vec<String> {
    match operation {
        AutomationOperation::PrepareAsset(request) => vec![
            format!("create a Forge job for {}", request.metadata.name),
            "write processed frames, quality evidence, preview, and .gsfpack under the job directory".into(),
        ],
        AutomationOperation::PrepareCharacterPack(request) => {
            let mut effects = vec![
                format!(
                    "create a {}@{} Character Pack job for {} with {} animations",
                    request.workflow.id,
                    request.workflow.version,
                    request.metadata.name,
                    request.animations.len()
                ),
                "ingest and matte each animation independently".into(),
                "normalize every animation against one shared canvas and foot anchor".into(),
            ];
            effects.push(if request.source_cycle_sampling_preview {
                "write local-only source-cycle diagnostics and previews; never export a Pack"
                    .into()
            } else {
                "write per-animation quality evidence, previews, and one multi-animation .gsfpack"
                    .into()
            });
            effects
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            if request.workflow.id == "topdown-cycle"
                && request.workflow.version == "10.1.0"
            {
                return vec![
                    format!(
                        "create a zero-provider-request V10.1 validation child for {}",
                        request.metadata.name
                    ),
                    "byte-reuse the immutable failed V10 walk_down video and bind its source Job, manifest, video SHA-256, reports, and workflow graph".into(),
                    "translate complete-cycle frames by integer pixels only to align body center and foot baseline; never rescale, recolor, reorder, regenerate, or edit frame content".into(),
                    "rerun scale, gait, motion, silhouette, identity and equipment gates; stop without Pack, catalog, or Godot mutation".into(),
                ];
            }
            let mut effects = vec![
                format!(
                    "use provider {} with profile {} to generate {}@{} character media for {}",
                    request.provider_id,
                    request.profile_id,
                    request.workflow.id,
                    request.workflow.version,
                    request.metadata.name
                ),
                "lock this job to one provider and persist validated provider outputs with SHA-256 provenance"
                    .into(),
            ];
            if is_xai_image2_direction_grid_probe(request) {
                effects.extend([
                    "byte-preserve the approved V9 DirectionGrid source and create one comparison child without mutating its lock or approval".into(),
                    "submit exactly one grok-imagine-image-2.0 direction_grid edit with no hidden retry or model fallback".into(),
                    "run the existing four-direction identity, equipment, transparency and appearance gates, then stop for native review with no Pack, catalog or Godot mutation".into(),
                ]);
                return effects;
            }
            if is_front_authoritative_direction_grid(request) {
                effects.extend([
                    "byte-preserve the approved V9 source and use only its front_idle node as the image reference and garment-topology authority".into(),
                    "submit exactly one 2x2 DirectionGrid image edit; the rear/right/left cells are rotations of front_idle, with no hidden retry or independent direction generation".into(),
                    "enforce the front_authoritative_no_skirt_hem cape contract, identity, equipment, transparency and appearance gates; stop for native review with no Pack, catalog or Godot mutation".into(),
                ]);
                return effects;
            }
            if request.validation_only {
                if request.workflow.id == "topdown-cycle"
                    && request.workflow.version == "10.0.0"
                {
                    effects.extend([
                        "reuse the approved V9 front direction anchor and submit exactly one continuous walk_down image-to-video generation with no retry"
                            .into(),
                        "discover one complete native gait cycle and select 8, 10, or 12 source-timed frames without independent per-frame generation or scaling"
                            .into(),
                        "run direction, gait, motion, silhouette, equipment, identity and character-scale gates; stop at native review with no Pack, catalog or Godot mutation"
                            .into(),
                    ]);
                } else if matches!(
                    request.workflow.id.as_str(),
                    "topdown"
                        | "topdown-video"
                        | "topdown-video-locked"
                        | "topdown-video-cycle"
                ) {
                    effects.extend([
                        format!(
                            "generate only {} as one locked direction anchor plus one image-to-video clip per attempt",
                            request.validation_animations.join(", ")
                        ),
                        "reject DirectionLock, camera, framing, ground-line, or crop defects before spending the video request"
                            .into(),
                        if request.workflow.id == "topdown-video-cycle" {
                            "prove a complete gait cycle before loop closure and preserve the shared playback cadence in every output".into()
                        } else {
                            "search the full local clip for a closed loop and write validation reports without exporting a partial .gsfpack".into()
                        },
                    ]);
                } else {
                    let mut image_effects = vec![
                        format!(
                            "generate only {} as explicit image frames; retry each failed frame at most once",
                            request.validation_animations.join(", ")
                        ),
                        "run deterministic normalization, consistency, hand/equipment, silhouette, and playback checks"
                            .into(),
                        "write validation reports and previews without exporting or cataloging a partial .gsfpack"
                            .into(),
                    ];
                    if request.workflow.id == "topdown-grid"
                        && matches!(
                            request.workflow.version.as_str(),
                            "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
                        )
                    {
                        if let Some(frame) = request
                            .retry_frames
                            .get("walk_down")
                            .and_then(|frames| frames.first())
                        {
                            let retry_contract = if request.workflow.version == "9.5.0" {
                                "edit only the manually rejected gray-footwear walk_down frame"
                            } else if request.workflow.version == "9.4.0" {
                                "fresh-retry only platform-rejected walk_down frame"
                            } else {
                                "fresh-retry only immutable laterality-recommended walk_down frame"
                            };
                            image_effects.push(format!(
                                "{retry_contract} {frame}; byte-reuse the other three frames with no second request"
                            ));
                        }
                        image_effects.extend([
                            if request.workflow.version == "9.5.0" {
                                "use the rejected frame as EditTarget with no PoseStructure reference and validate viewer-space gait laterality".into()
                            } else {
                                "bind one transparent grayscale PoseStructure to each walk_down frame and validate viewer-space gait laterality".into()
                            },
                            "generate no other direction, video, Pack, catalog entry, or Godot mutation".into(),
                        ]);
                    }
                    effects.extend(image_effects);
                }
            } else {
                effects.extend([
                    "generate idle, walk_up, walk_right, and walk_down; retry each failed animation at most once"
                        .into(),
                    "extract exact video samples, matte and normalize all directions on one shared canvas"
                        .into(),
                    "export a .gsfpack only when every required animation is game_ready".into(),
                ]);
            }
            effects
        }
        AutomationOperation::ImportDirectionGrid(request) => vec![
            format!(
                "create one local-only DirectionGrid import child from {} without mutating the approved source",
                request.source_job_dir.display()
            ),
            format!(
                "ingest the external {} with provenance generator {} and make zero Provider requests",
                if request.sheet_path.is_some() {
                    "2x2 sheet"
                } else {
                    "named front/rear/right/left files"
                },
                request.generator,
            ),
            "deterministically reconstruct soft Alpha from border-connected light checkerboard pixels and reject halo residuals".into(),
            "run fixed front/rear/right/left extraction, shared size/baseline alignment, absolute appearance and same-direction source-relative identity gates".into(),
            "stop at native DirectionGrid review with no approval, Pack, catalog or Godot mutation".into(),
        ],
        AutomationOperation::CreateStyleLock(request) => vec![
            format!(
                "create an immutable style revision for {} using provider {} profile {}",
                request.project_path.display(),
                request.provider_id,
                request.profile_id
            ),
            "materialize and hash the style board, references, palette, and visual baseline".into(),
            "update forge-project.json to point at the new immutable style revision".into(),
        ],
        AutomationOperation::CreateSubjectLock(request) => {
            if request.is_local_import() {
                vec![
                    format!(
                        "import an approved canonical into an immutable Subject Lock for {} with zero Provider requests",
                        request.project_path.display()
                    ),
                    "fingerprint and locally materialize the canonical, foreground mask, diagnostic identity report, and import provenance".into(),
                    "store the immutable revision under .forge/subjects without changing an earlier revision".into(),
                ]
            } else {
                vec![
                    format!(
                        "create an immutable Subject Lock for {} using provider {} profile {}",
                        request.project_path.display(), request.provider_id, request.profile_id
                    ),
                    "materialize and hash the canonical identity image, foreground mask, references, and identity baseline".into(),
                    "store the immutable revision under .forge/subjects without changing an earlier revision".into(),
                ]
            }
        }
        AutomationOperation::CreateCollectionLock(request) => vec![
            format!(
                "create an immutable Collection Lock for {} using provider {} profile {}",
                request.project_path.display(), request.provider_id, request.profile_id
            ),
            "materialize and hash the collection anchor, medoid seed, materials, scale, and outlier baseline".into(),
            "store the immutable revision under .forge/collections without changing an earlier revision".into(),
        ],
        AutomationOperation::GenerateStaticAssetSet(request) => match request.portrait_phase {
            PortraitGenerationPhaseV1::BaseOnly => vec![
                format!("generate only neutral PortraitBase for {}", request.asset.name),
                format!(
                    "use neutral reference policy {}",
                    request.neutral_reference_policy.as_str()
                ),
                "run hard/local quality gates and stop for explicit review without exporting a Pack"
                    .into(),
            ],
            PortraitGenerationPhaseV1::Expressions => vec![
                format!(
                    "reuse approved PortraitBase {} and generate selected expressions",
                    request
                        .portrait_base_parent_job_id
                        .as_deref()
                        .unwrap_or("<missing>")
                ),
                "derive every expression independently from the same approved neutral edit target"
                    .into(),
                "export a static .gsfpack only after all local consistency gates pass".into(),
            ],
            PortraitGenerationPhaseV1::LegacyAll => vec![
                format!(
                    "generate {} {} with {} locked items",
                    request.asset.kind.as_str(),
                    request.asset.name,
                    request.asset.items.len()
                ),
                "derive every item from the immutable style board and one accepted anchor item"
                    .into(),
                "run consistency@1.2.0 and retry only failed items at most once".into(),
                "export a static .gsfpack only after consistency gates pass".into(),
            ],
        },
        AutomationOperation::CreateEnvironmentLock(request) => vec![
            format!(
                "create an immutable top-down environment revision for {} using provider {} profile {}",
                request.project_path.display(), request.provider_id, request.profile_id
            ),
            "materialize and hash an environment board locked to the current Style revision".into(),
            "update forge-project.json currentEnvironmentRevision".into(),
        ],
        AutomationOperation::GenerateTerrainSet(request) => vec![
            format!("generate terrain set {} from two locked material plates", request.asset.name),
            "synthesize one base tile and fifteen deterministic dual-grid masks".into(),
            "run exhaustive seam validation and export a V3 terrain_set Pack".into(),
        ],
        AutomationOperation::GenerateBuildingKit(request) => vec![
            format!("generate modular building kit {}", request.asset.name),
            "synthesize the fixed topdown-exterior@1.0.0 module catalog".into(),
            "validate footprints and entrances and export a V3 building_kit Pack".into(),
        ],
        AutomationOperation::CompileMap(request) => vec![
            format!("compile JSON map spec {} without a Provider", request.spec_path.display()),
            "fingerprint and self-contain all Pack dependencies".into(),
            "try up to twenty deterministic candidates and export only a validated V3 map Pack".into(),
        ],
        AutomationOperation::InstallGodot(request) => vec![
            format!(
                "install the pack into {}/{}",
                request.project_path.display(),
                request.target.display()
            ),
            "create a Forge ownership marker and replace only Forge-owned prior output".into(),
            "write a Godot usage descriptor and atomically register the asset in .forge/assets.json"
                .into(),
            if request.catalog_project_path.is_some() {
                "link the Godot installation back to the source .forge/catalog.json".into()
            } else {
                "leave the optional source project catalog unchanged".into()
            },
        ],
        AutomationOperation::BuildProject(request) => vec![format!(
            "build project from manifest {} into {}",
            request.manifest_path.display(),
            request.project_path.display()
        )],
    }
}

fn validate_token(token: &str) -> Result<(), PlanStoreError> {
    if token.is_empty()
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(PlanStoreError::NotFound(token.to_string()));
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, PlanStoreError> {
    let bytes = fs::read(path).map_err(|source| PlanStoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), PlanStoreError> {
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?).map_err(|source| {
        PlanStoreError::Io {
            path: temporary.clone(),
            source,
        }
    })?;
    fs::rename(&temporary, path).map_err(|source| PlanStoreError::Io {
        path: path.to_path_buf(),
        source,
    })
}
