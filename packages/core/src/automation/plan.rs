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
    GenerateCharacterPackRequest, GenerateStaticAssetSetRequest, PlanState,
    PrepareCharacterPackRequest, PreparedPlan, AUTOMATION_SCHEMA_VERSION,
};
use crate::asset_project::{read_project, read_style_lock, resolve_relative, StyleSpecV1};
use crate::catalog::{read_project_catalog, PROJECT_CATALOG_RELATIVE};
use crate::game_art::{
    compute_build_plan, compute_project_diff, project_source_sha256, GameArtManifestV1,
    ProjectBuildStateV1, ProviderCapabilityInput,
};
use crate::job::{JobRecord, RepairContext};
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
        AutomationOperation::CreateSubjectLock(request) => PlanEstimateV1 {
            provider_request_estimate: 1,
            maximum_provider_requests: 1,
            provider_id: Some(request.provider_id.clone()),
            profile_id: Some(request.profile_id.clone()),
            ..Default::default()
        },
        AutomationOperation::GenerateCharacterPack(request) => {
            let keyframe = request.workflow.id == "topdown-keyframes";
            let local_only = !request.retry_animations.is_empty()
                && request.retry_stages.values().all(|stage| {
                    matches!(
                        stage,
                        super::types::CharacterRetryStage::Loop
                            | super::types::CharacterRetryStage::Matting
                    )
                });
            let selected_frames = request.retry_frames.values().map(Vec::len).sum::<usize>() as u32;
            let (estimated, maximum) = if local_only {
                (0, 0)
            } else if keyframe && selected_frames > 0 {
                (selected_frames, selected_frames)
            } else if keyframe {
                (32, 64)
            } else {
                (9, 17)
            };
            PlanEstimateV1 {
                provider_request_estimate: estimated,
                maximum_provider_requests: maximum,
                provider_id: Some(request.provider_id.clone()),
                profile_id: Some(request.profile_id.clone()),
                workflow: Some(format!(
                    "{}@{}",
                    request.workflow.id, request.workflow.version
                )),
                model: request.generation.image_model.clone(),
                ..Default::default()
            }
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            let count = if request.consistency_recheck_only {
                0
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
            read_subject_spec(&request.spec_path)
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
                let found = catalog.assets.values().any(|entry| {
                    fs::canonicalize(&entry.pack_path)
                        .ok()
                        .is_some_and(|path| path == canonical_pack)
                });
                if !found {
                    return Err(PlanStoreError::InvalidRequest(
                        "catalogProjectPath does not contain the requested Pack".into(),
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
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            hasher.update(request.character.prompt.as_bytes());
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
                let keyframe = request.workflow.id == "topdown-keyframes";
                let mut retry_files = vec![source.join("job.json")];
                let manifest = if keyframe {
                    source.join("source/keyframe-provider-manifest.json")
                } else {
                    source.join("source/provider-manifest.json")
                };
                if manifest.is_file() {
                    retry_files.push(manifest);
                }
                if keyframe {
                    retry_files.push(source.join("workflow-graph.json"));
                }
                hash_files(&mut hasher, &retry_files)?;
                hash_directory(
                    &mut hasher,
                    &source.join(if keyframe {
                        "source/provider-keyframes"
                    } else {
                        "source/provider"
                    }),
                )?;
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
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            hasher.update(request.provider_id.as_bytes());
            hasher.update(request.profile_id.as_bytes());
            hash_files(
                &mut hasher,
                &[
                    request.project_path.join("forge-project.json"),
                    request.style_lock_path.clone(),
                ],
            )?;
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
                hash_files(
                    &mut hasher,
                    &[
                        source.join("job.json"),
                        source.join("consistency-report.json"),
                    ],
                )?;
                hash_directory(&mut hasher, &source.join("normalized/static"))?;
            }
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
                if paths.len() < 2 {
                    return Err(PlanStoreError::InvalidRequest(format!(
                        "animation {} requires at least two PNG frames",
                        animation.name
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
    Ok(())
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
    if request.metadata.default_animation != "idle" {
        return Err(PlanStoreError::InvalidRequest(
            "generated top-down packs require metadata.defaultAnimation \"idle\"".into(),
        ));
    }
    let video_workflow = request.workflow.id == "topdown" && request.workflow.version == "1.0.0";
    let keyframe_workflow =
        request.workflow.id == "topdown-keyframes" && request.workflow.version == "2.0.0";
    if !video_workflow && !keyframe_workflow {
        return Err(PlanStoreError::InvalidRequest(
            "schema V3 supports topdown@1.0.0 and topdown-keyframes@2.0.0".into(),
        ));
    }
    if !(1..=2).contains(&request.generation.max_attempts_per_animation) {
        return Err(PlanStoreError::InvalidRequest(
            "generation.maxAttemptsPerAnimation must be 1 or 2".into(),
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
    if let Some(path) = &request.style_lock_path {
        read_style_lock(path).map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
    }
    if keyframe_workflow {
        let path = request.subject_lock_path.as_ref().ok_or_else(|| {
            PlanStoreError::InvalidRequest(
                "topdown-keyframes@2.0.0 requires subjectLockPath".into(),
            )
        })?;
        let subject = read_subject_lock(path)
            .map_err(|error| PlanStoreError::InvalidRequest(error.to_string()))?;
        if subject.provider_id != request.provider_id || subject.profile_id != request.profile_id {
            return Err(PlanStoreError::InvalidRequest(
                "keyframe generation must use the Provider and profile locked by SubjectLock"
                    .into(),
            ));
        }
        if request.generation.target_frame_count != 8 {
            return Err(PlanStoreError::InvalidRequest(
                "topdown-keyframes@2.0.0 requires exactly eight frames per action".into(),
            ));
        }
    } else if request.subject_lock_path.is_some() {
        return Err(PlanStoreError::InvalidRequest(
            "subjectLockPath is reserved for topdown-keyframes@2.0.0".into(),
        ));
    }
    let allowed = ["idle", "walk_up", "walk_right", "walk_down"];
    if request
        .retry_animations
        .iter()
        .any(|name| !allowed.contains(&name.as_str()))
    {
        return Err(PlanStoreError::InvalidRequest(
            "retryAnimations contains an unknown topdown animation".into(),
        ));
    }
    if request.retry_stages.iter().any(|(name, _)| {
        !allowed.contains(&name.as_str()) || !request.retry_animations.contains(name)
    }) {
        return Err(PlanStoreError::InvalidRequest(
            "retryStages keys must name animations present in retryAnimations".into(),
        ));
    }
    for (animation, frames) in &request.retry_frames {
        if !request.retry_animations.contains(animation)
            || frames.is_empty()
            || frames.len() > 8
            || frames.iter().any(|frame| *frame > 7)
        {
            return Err(PlanStoreError::InvalidRequest(
                "retryFrames must contain unique frame indices 0..=7 for a retried animation"
                    .into(),
            ));
        }
        let unique = frames.iter().copied().collect::<HashSet<_>>();
        if unique.len() != frames.len() {
            return Err(PlanStoreError::InvalidRequest(
                "retryFrames may not contain duplicate frame indices".into(),
            ));
        }
    }
    let retry_required_paths = if keyframe_workflow {
        ["workflow-graph.json", "source/provider-keyframes"]
    } else {
        ["source/provider", "source/provider"]
    };
    validate_retry_pair(
        request.reuse_from_job_dir.as_deref(),
        &request.retry_animations,
        &retry_required_paths,
    )?;
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

fn validate_static_asset_set_request(
    request: &GenerateStaticAssetSetRequest,
) -> Result<(), PlanStoreError> {
    if request.schema_version != "4" {
        return Err(PlanStoreError::InvalidRequest(
            "static asset generation requires schemaVersion \"4\"".into(),
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
    if request.asset.schema_version != "1"
        || !is_engine_safe_name(&request.asset.id)
        || request.asset.name.trim().is_empty()
        || request.asset.items.is_empty()
        || request.asset.items.len() > 64
    {
        return Err(PlanStoreError::InvalidRequest(
            "static asset spec requires schemaVersion 1, a safe id/name, and 1..=64 items".into(),
        ));
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
    validate_retry_pair(
        request.reuse_from_job_dir.as_deref(),
        &request.retry_item_ids,
        &["consistency-report.json", "normalized/static"],
    )?;
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
        AutomationOperation::PrepareCharacterPack(request) => vec![
            format!(
                "create a {}@{} Character Pack job for {} with {} animations",
                request.workflow.id,
                request.workflow.version,
                request.metadata.name,
                request.animations.len()
            ),
            "ingest and matte each animation independently".into(),
            "normalize every animation against one shared canvas and foot anchor".into(),
            "write per-animation quality evidence, previews, and one multi-animation .gsfpack"
                .into(),
        ],
        AutomationOperation::GenerateCharacterPack(request) => vec![
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
            "generate idle, walk_up, walk_right, and walk_down; retry each failed animation at most once"
                .into(),
            "extract exact video samples, matte and normalize all directions on one shared canvas"
                .into(),
            "export a .gsfpack only when every required animation is game_ready".into(),
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
        AutomationOperation::CreateSubjectLock(request) => vec![
            format!(
                "create an immutable Subject Lock for {} using provider {} profile {}",
                request.project_path.display(), request.provider_id, request.profile_id
            ),
            "materialize and hash the canonical identity image, foreground mask, references, and identity baseline".into(),
            "store the immutable revision under .forge/subjects without changing an earlier revision".into(),
        ],
        AutomationOperation::GenerateStaticAssetSet(request) => vec![
            format!(
                "generate {} {} with {} locked items",
                request.asset.kind.as_str(),
                request.asset.name,
                request.asset.items.len()
            ),
            "derive every item from the immutable style board and one accepted anchor item".into(),
            "run consistency@1.2.0 and retry only failed items at most once".into(),
            "export a static .gsfpack only after consistency gates pass".into(),
        ],
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
