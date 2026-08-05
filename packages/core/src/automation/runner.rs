use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use image::{ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::asset_project::{
    apply_keyframe_hard_defects, assess_consistency, build_style_lock, export_static_pack,
    hash_file as hash_asset_file, image_signature, normalize_static_image, read_style_lock,
    write_contact_sheet, ConsistencyItemReport, ConsistencyReportV1, ConsistencyVerdict,
    StaticAssetKind, StaticPackItem, CONSISTENCY_PROFILE, KEYFRAME_HARD_GATE_PROFILE,
};
use crate::catalog::{
    link_catalog_install, register_catalog_asset, CatalogProviderRefV1, CatalogStyleRefV1,
    CatalogSubjectRefV1, ProjectCatalogEntryV1,
};
use crate::export::{
    export_character_pack, export_pack, AnimationQualityEntry, CharacterAnimationExport,
    CharacterPackExportParams, CharacterPackMetadataParams, CharacterQualityReport,
    ExportPackParams, GifBackground, PackMetadataParams, PreviewGifParameters,
};
use crate::frames::{bbox_from_image, normalize_frames, FrameSize};
use crate::job::{
    JobArtifactRecord, JobLifecycleState, JobOperationKind, JobRecord, JobState, JobStepRecord,
    JobStore, JobStoreError, SourceKind, JOB_WORKSPACE_JSON,
};
use crate::matting::{process_chroma_batch, ChromaKeyMode, ChromaParameters};
use crate::project::{
    register_project_asset, ProviderAssetRef, RegisterProjectAsset, PROJECT_MANIFEST_RELATIVE,
};
use crate::provider::{
    EditImageRequest, EditVideoRequest, GenerateImageRequest, GenerateVideoRequest,
    MediaGenerationProvider, ProviderCapability, ProviderError, ProviderImageReference,
    ProviderInputRef, ProviderMedia, ProviderPoll, ProviderTicket, ProviderUsage, ReferenceRole,
    VideoGenerationMode,
};
use crate::quality::{
    compute_quality_report, compute_quality_report_for_animation, select_loop_frames,
    LoopSelectionPolicy, LoopSelectionReport, LoopSelectionVerdict, QualityMetrics,
    QualityRecommendationId, QualityReport, QualityVerdict, LOOP_SELECTION_PROFILE,
};
use crate::subject::{build_subject_lock, read_subject_lock};
use crate::video::{
    extract_candidate_frames, extract_sampled_frames, probe_video, slice_sprite_sheet_grid,
    slice_sprite_sheet_transparent, ExtractCandidateFramesParams, ProbeVideoParams,
    SampleVideoFramesParams, SliceSpriteSheetParams, SliceSpriteSheetTransparentParams,
};
use crate::workflow_graph::{
    compute_cache_key, read_workflow_graph, write_workflow_graph, ContentCache, WorkflowArtifactV1,
    WorkflowGraphV1, WorkflowNodeV1, WORKFLOW_GRAPH_FILE,
};
use crate::world::{
    build_environment_lock, compile_map_pack, generate_building_kit, generate_terrain_set,
    read_environment_lock,
};

use super::plan::{PlanStore, PlanStoreError};
use super::repair::active_repair_child;
use super::types::{
    AssetInput, AutomationOperation, AutomationPlan, CharacterAnimationRecipe, CharacterRetryStage,
    CompileMapRequest, CreateEnvironmentLockRequest, CreateStyleLockRequest,
    CreateSubjectLockRequest, GenerateBuildingKitRequest, GenerateCharacterPackRequest,
    GenerateStaticAssetSetRequest, GenerateTerrainSetRequest, GodotInstallRequest, MattingRecipe,
    PrepareAssetRequest, PrepareCharacterPackRequest, SpriteSheetSplit,
};
use super::{character_quality_snapshot, single_quality_snapshot, write_repair_comparison};

const GODOT_INSTALL_SCRIPT: &str = include_str!("../../../../scripts/godot/install_forge_pack.gd");
const OWNERSHIP_MARKER: &str = ".forge-owned.json";

#[derive(Debug, Error)]
pub enum AutomationRunError {
    #[error("job error: {0}")]
    Job(#[from] JobStoreError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("asset processing failed: {0}")]
    Processing(String),
    #[error("{0}")]
    Provider(#[from] ProviderError),
    #[error("plan store error: {0}")]
    Plan(#[from] PlanStoreError),
    #[error("{0}")]
    GameArt(#[from] crate::game_art::GameArtError),
    #[error("job was cancelled")]
    Cancelled,
    #[error("project build failed: {0}")]
    ProjectBuildFailed(String),
}

impl AutomationRunError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Job(_) => "job_error",
            Self::Io(_) => "io_error",
            Self::Image(_) => "image_error",
            Self::Json(_) => "invalid_json",
            Self::Processing(_) => "automation_failed",
            Self::Provider(error) => error.code(),
            Self::Plan(_) => "plan_store_error",
            Self::GameArt(error) => error.code(),
            Self::Cancelled => "cancelled",
            Self::ProjectBuildFailed(_) => "project_build_failed",
        }
    }
}

pub fn stage_plan_job(
    store: &JobStore,
    plan: &AutomationPlan,
) -> Result<JobRecord, AutomationRunError> {
    if let Some(repair) = &plan.repair {
        if let Some(child) = active_repair_child(store, &repair.source_job_id)? {
            return Err(AutomationRunError::Processing(format!(
                "source job already has an active repair job: {}",
                child.job_id
            )));
        }
    }
    let (source_kind, operation_kind) = match &plan.operation {
        AutomationOperation::PrepareAsset(request) => (
            source_kind_for_input(&request.input),
            JobOperationKind::PrepareAsset,
        ),
        AutomationOperation::PrepareCharacterPack(_) => {
            (SourceKind::FromCode, JobOperationKind::PrepareCharacterPack)
        }
        AutomationOperation::GenerateCharacterPack(_) => (
            SourceKind::FromCode,
            JobOperationKind::GenerateCharacterPack,
        ),
        AutomationOperation::CreateStyleLock(_) => {
            (SourceKind::FromCode, JobOperationKind::CreateStyleLock)
        }
        AutomationOperation::CreateSubjectLock(_) => {
            (SourceKind::FromCode, JobOperationKind::CreateSubjectLock)
        }
        AutomationOperation::GenerateStaticAssetSet(_) => (
            SourceKind::FromCode,
            JobOperationKind::GenerateStaticAssetSet,
        ),
        AutomationOperation::CreateEnvironmentLock(_) => (
            SourceKind::FromCode,
            JobOperationKind::CreateEnvironmentLock,
        ),
        AutomationOperation::GenerateTerrainSet(_) => {
            (SourceKind::FromCode, JobOperationKind::GenerateTerrainSet)
        }
        AutomationOperation::GenerateBuildingKit(_) => {
            (SourceKind::FromCode, JobOperationKind::GenerateBuildingKit)
        }
        AutomationOperation::CompileMap(_) => (SourceKind::FromCode, JobOperationKind::CompileMap),
        AutomationOperation::InstallGodot(_) => {
            (SourceKind::ImportGsfpack, JobOperationKind::InstallGodot)
        }
        AutomationOperation::BuildProject(_) => {
            (SourceKind::FromCode, JobOperationKind::BuildProject)
        }
    };
    let mut record = store.create_job(source_kind)?;
    let (asset_id, reuse_from_job_dir) = match &plan.operation {
        AutomationOperation::GenerateCharacterPack(request) => (
            request.asset_id.clone(),
            request.reuse_from_job_dir.as_ref(),
        ),
        AutomationOperation::GenerateStaticAssetSet(request) => (
            Some(request.asset.id.clone()),
            request.reuse_from_job_dir.as_ref(),
        ),
        AutomationOperation::GenerateTerrainSet(request) => (Some(request.asset.id.clone()), None),
        AutomationOperation::GenerateBuildingKit(request) => (Some(request.asset.id.clone()), None),
        _ => (None, None),
    };
    if asset_id.is_some() {
        record.asset_id = asset_id;
    }
    record.parent_job_id = plan
        .repair
        .as_ref()
        .map(|repair| repair.source_job_id.clone())
        .or_else(|| {
            reuse_from_job_dir.and_then(|directory| {
                fs::read(directory.join("job.json"))
                    .ok()
                    .and_then(|bytes| serde_json::from_slice::<JobRecord>(&bytes).ok())
                    .map(|source| source.job_id)
            })
        });
    record.operation_kind = operation_kind;
    record.lifecycle_state = JobLifecycleState::Queued;
    record.input_hash = Some(plan.input_fingerprint.clone());
    record.recipe_hash = Some(plan.recipe_hash.clone());
    record.recipe = Some(serde_json::to_value(&plan.operation)?);
    record.repair = plan.repair.clone();
    record.steps = steps_for_operation(&plan.operation);
    record.next_actions = vec!["poll_job".to_string(), "job_report".to_string()];
    write_json_atomic(&record.job_dir.join(JOB_WORKSPACE_JSON), plan)?;
    store.write_record(&record)?;
    Ok(record)
}

pub fn run_operation(
    store: &JobStore,
    job_id: &str,
    operation: &AutomationOperation,
) -> Result<JobRecord, AutomationRunError> {
    run_operation_with_provider(store, job_id, operation, None)
}

pub fn run_operation_with_provider(
    store: &JobStore,
    job_id: &str,
    operation: &AutomationOperation,
    provider: Option<&dyn MediaGenerationProvider>,
) -> Result<JobRecord, AutomationRunError> {
    // BuildProject is an orchestrator: only its child jobs call the Provider.
    // Tracking the shared provider again at the parent would duplicate every
    // child request in provider-usage.json.
    let usage_provider = if matches!(operation, AutomationOperation::BuildProject(_)) {
        None
    } else {
        provider
    };
    let provider_usage_baseline = usage_provider.map(MediaGenerationProvider::usage);
    store.update_record(job_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.progress = 0.02;
        record.worker_pid = Some(std::process::id());
        record.next_actions = vec!["poll_job".to_string(), "cancel_job".to_string()];
    })?;

    let result = match operation {
        AutomationOperation::PrepareAsset(request) => run_prepare_asset(store, job_id, request),
        AutomationOperation::PrepareCharacterPack(request) => {
            run_prepare_character_pack(store, job_id, request)
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            let provider = provider.ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "provider {} must be resolved before running this plan",
                    request.provider_id
                ))
            })?;
            run_generate_character_pack(store, job_id, request, provider)
        }
        AutomationOperation::CreateStyleLock(request) => {
            let provider = provider.ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "provider {} must be resolved before running this plan",
                    request.provider_id
                ))
            })?;
            run_create_style_lock(store, job_id, request, provider)
        }
        AutomationOperation::CreateSubjectLock(request) => {
            let provider = provider.ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "provider {} must be resolved before running this plan",
                    request.provider_id
                ))
            })?;
            run_create_subject_lock(store, job_id, request, provider)
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            let provider = provider.ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "provider {} must be resolved before running this plan",
                    request.provider_id
                ))
            })?;
            run_generate_static_asset_set(store, job_id, request, provider)
        }
        AutomationOperation::CreateEnvironmentLock(request) => {
            let provider = provider.ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "provider {} must be resolved before running this plan",
                    request.provider_id
                ))
            })?;
            run_create_environment_lock(store, job_id, request, provider)
        }
        AutomationOperation::GenerateTerrainSet(request) => {
            let provider = provider.ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "provider {} must be resolved before running this plan",
                    request.provider_id
                ))
            })?;
            run_generate_terrain_set(store, job_id, request, provider)
        }
        AutomationOperation::GenerateBuildingKit(request) => {
            let provider = provider.ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "provider {} must be resolved before running this plan",
                    request.provider_id
                ))
            })?;
            run_generate_building_kit(store, job_id, request, provider)
        }
        AutomationOperation::CompileMap(request) => run_compile_map(store, job_id, request),
        AutomationOperation::InstallGodot(request) => run_install_godot(store, job_id, request),
        AutomationOperation::BuildProject(request) => {
            // Children are planned through the same PlanStore the CLI uses:
            // FORGE_PLAN_STORE when set, otherwise the default app store.
            // The parent build job itself makes no provider calls; every
            // child goes back through run_operation_with_provider so the
            // real-provider cost guard stays in force.
            let plans = match std::env::var_os("FORGE_PLAN_STORE") {
                Some(root) => PlanStore::new(root),
                None => PlanStore::default_app_store(),
            }
            .map_err(AutomationRunError::Plan)?;
            crate::game_art::run_build_project(store, &plans, job_id, request, provider)
                .and_then(|_report| store.read_record(job_id).map_err(AutomationRunError::Job))
        }
    };
    if result.is_err() && operation_is_video_character(operation) {
        if let (
            AutomationOperation::GenerateCharacterPack(request),
            Some(provider),
            Some(provider_usage_baseline),
        ) = (operation, provider, provider_usage_baseline.as_ref())
        {
            let _ = persist_failed_character_provider_manifest(
                store,
                job_id,
                request,
                provider,
                provider_usage_baseline,
            );
        }
    }
    let usage_result = usage_provider.map(|provider| {
        persist_provider_usage(
            store,
            job_id,
            provider,
            provider_usage_baseline
                .as_ref()
                .expect("Provider baseline exists"),
        )
    });

    match result {
        Ok(record) => {
            if let Some(usage_result) = usage_result {
                usage_result?;
                return store.read_record(&record.job_id).map_err(Into::into);
            }
            Ok(record)
        }
        Err(AutomationRunError::Cancelled)
        | Err(AutomationRunError::Provider(ProviderError::Cancelled)) => {
            if let Some(usage_result) = usage_result {
                usage_result?;
            }
            let record = store.update_record(job_id, |record| {
                record.lifecycle_state = JobLifecycleState::Cancelled;
                record.progress = 1.0;
                record.worker_pid = None;
                record.error_code = Some("cancelled".into());
                record.next_actions = vec!["job_report".into()];
            })?;
            Ok(record)
        }
        Err(error) => {
            let _ = usage_result.transpose();
            let message = error.to_string();
            let _ = store.update_record(job_id, |record| {
                record.lifecycle_state = JobLifecycleState::Failed;
                record.state = JobState::Failed;
                record.worker_pid = None;
                record.error_summary = Some(message.clone());
                record.error_code = Some(error.code().into());
                record.recoverable = true;
                record.next_actions = vec!["job_report".into(), "prepare_new_plan".into()];
            });
            Err(error)
        }
    }
}

fn operation_is_video_character(operation: &AutomationOperation) -> bool {
    matches!(
        operation,
        AutomationOperation::GenerateCharacterPack(request)
            if request.workflow.id == "topdown" && request.workflow.version == "1.0.0"
    )
}

fn persist_provider_usage(
    store: &JobStore,
    job_id: &str,
    provider: &dyn MediaGenerationProvider,
    baseline: &ProviderUsage,
) -> Result<(), AutomationRunError> {
    let record = store.read_record(job_id)?;
    let path = record.job_dir.join("provider-usage.json");
    let value = serde_json::json!({
        "schemaVersion": "1",
        "providerId": provider.id(),
        "usage": provider_usage_delta(&provider.usage(), baseline),
    });
    write_json_atomic(&path, &value)?;
    let sha256 = hash_file(&path)?;
    store.update_record(job_id, |record| {
        record
            .artifacts
            .retain(|artifact| artifact.kind != "provider_usage");
        record.artifacts.push(JobArtifactRecord {
            kind: "provider_usage".into(),
            path: path.clone(),
            sha256: Some(sha256.clone()),
        });
    })?;
    Ok(())
}

fn provider_usage_delta(current: &ProviderUsage, baseline: &ProviderUsage) -> ProviderUsage {
    ProviderUsage {
        requests: current.requests.saturating_sub(baseline.requests),
        generated_images: current
            .generated_images
            .saturating_sub(baseline.generated_images),
        generated_videos: current
            .generated_videos
            .saturating_sub(baseline.generated_videos),
        edited_videos: current.edited_videos.saturating_sub(baseline.edited_videos),
        private_file_uploads: current
            .private_file_uploads
            .saturating_sub(baseline.private_file_uploads),
        cost_in_usd_ticks: current
            .cost_in_usd_ticks
            .map(|current| current.saturating_sub(baseline.cost_in_usd_ticks.unwrap_or_default())),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedMediaRecord {
    animation: String,
    attempt: u8,
    #[serde(default)]
    still_attempt: u8,
    #[serde(default)]
    video_attempt: u8,
    #[serde(default = "default_retry_method")]
    retry_method: String,
    still_path: PathBuf,
    still_sha256: String,
    video_path: PathBuf,
    video_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    still_asset_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    video_asset_id: Option<String>,
}

fn default_retry_method() -> String {
    "image_to_video".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderManifestReference {
    path: PathBuf,
    sha256: String,
    #[serde(default)]
    asset_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderManifestRecord {
    reference: ProviderManifestReference,
    animations: BTreeMap<String, GeneratedMediaRecord>,
}

fn run_create_style_lock(
    store: &JobStore,
    job_id: &str,
    request: &CreateStyleLockRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    require_provider_capabilities(provider, &[ProviderCapability::GenerateImage])?;
    let record = store.read_record(job_id)?;
    step(store, job_id, "style:materialize", "running", 0.15, None)?;
    let output = build_style_lock(
        &request.project_path,
        &request.spec_path,
        &request.provider_id,
        &request.profile_id,
        provider,
        &record.job_dir.join("source/provider/style"),
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let lock_hash = hash_file(&output.style_lock_path)?;
    step(store, job_id, "style:materialize", "succeeded", 0.75, None)?;
    store.update_record(job_id, |record| {
        record.state = JobState::Exported;
        record.lifecycle_state = JobLifecycleState::Succeeded;
        record.progress = 1.0;
        record.worker_pid = None;
        record.asset_id = Some(output.revision.clone());
        record.artifacts.extend([
            JobArtifactRecord {
                kind: "style_lock".into(),
                path: output.style_lock_path.clone(),
                sha256: Some(lock_hash.clone()),
            },
            JobArtifactRecord {
                kind: "style_board".into(),
                path: output.board_path.clone(),
                sha256: Some(output.board_sha256.clone()),
            },
        ]);
        record.next_actions = vec!["generate_asset".into(), "job_report".into()];
        if let Some(current) = record
            .steps
            .iter_mut()
            .find(|step| step.name == "style:lock")
        {
            current.state = "succeeded".into();
        }
    })?;
    store.read_record(job_id).map_err(Into::into)
}

fn run_create_subject_lock(
    store: &JobStore,
    job_id: &str,
    request: &CreateSubjectLockRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    if provider.id() != request.provider_id {
        return Err(AutomationRunError::Processing(format!(
            "resolved provider {} does not match plan provider {}",
            provider.id(),
            request.provider_id
        )));
    }
    require_provider_capabilities(provider, &[ProviderCapability::EditImage])?;
    let constraints = provider.health_check().constraints.unwrap_or_default();
    if constraints
        .max_image_references
        .is_some_and(|maximum| maximum < 3)
    {
        return Err(AutomationRunError::Processing(
            "Subject Lock requires a Provider that accepts up to three image references".into(),
        ));
    }
    let record = store.read_record(job_id)?;
    step(store, job_id, "subject:materialize", "running", 0.15, None)?;
    let output = build_subject_lock(
        &request.project_path,
        &request.spec_path,
        &request.provider_id,
        &request.profile_id,
        provider,
        &record.job_dir.join("source/provider/subject"),
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let lock_hash = hash_file(&output.subject_lock_path)?;
    step(
        store,
        job_id,
        "subject:materialize",
        "succeeded",
        0.75,
        None,
    )?;
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record.asset_id = Some(format!("{}@{}", output.id, output.revision));
            record.artifacts.extend([
                JobArtifactRecord {
                    kind: "subject_lock".into(),
                    path: output.subject_lock_path.clone(),
                    sha256: Some(lock_hash.clone()),
                },
                JobArtifactRecord {
                    kind: "subject_canonical".into(),
                    path: output.canonical_path.clone(),
                    sha256: Some(output.canonical_sha256.clone()),
                },
                JobArtifactRecord {
                    kind: "subject_mask".into(),
                    path: output.mask_path.clone(),
                    sha256: Some(output.mask_sha256.clone()),
                },
            ]);
            record.next_actions = vec!["generate_character".into(), "job_report".into()];
            if let Some(current) = record
                .steps
                .iter_mut()
                .find(|step| step.name == "subject:lock")
            {
                current.state = "succeeded".into();
            }
        })
        .map_err(Into::into)
}

fn run_create_environment_lock(
    store: &JobStore,
    job_id: &str,
    request: &CreateEnvironmentLockRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    if provider.id() != request.provider_id {
        return Err(AutomationRunError::Processing(format!(
            "resolved provider {} does not match plan provider {}",
            provider.id(),
            request.provider_id
        )));
    }
    require_provider_capabilities(provider, &[ProviderCapability::EditImage])?;
    let record = store.read_record(job_id)?;
    step(
        store,
        job_id,
        "environment:materialize",
        "running",
        0.15,
        None,
    )?;
    let output = build_environment_lock(
        &request.project_path,
        &request.spec_path,
        &request.provider_id,
        &request.profile_id,
        provider,
        &record.job_dir.join("source/provider/environment"),
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let lock_hash = hash_file(&output.lock_path)?;
    step(
        store,
        job_id,
        "environment:materialize",
        "succeeded",
        0.75,
        None,
    )?;
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record.asset_id = Some(output.revision.clone());
            record.artifacts.extend([
                JobArtifactRecord {
                    kind: "environment_lock".into(),
                    path: output.lock_path.clone(),
                    sha256: Some(lock_hash.clone()),
                },
                JobArtifactRecord {
                    kind: "environment_board".into(),
                    path: output.board_path.clone(),
                    sha256: Some(output.board_sha256.clone()),
                },
            ]);
            record.next_actions = vec!["generate_terrain_set".into(), "job_report".into()];
            if let Some(current) = record
                .steps
                .iter_mut()
                .find(|step| step.name == "environment:lock")
            {
                current.state = "succeeded".into();
            }
        })
        .map_err(Into::into)
}

fn run_generate_terrain_set(
    store: &JobStore,
    job_id: &str,
    request: &GenerateTerrainSetRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    if provider.id() != request.provider_id {
        return Err(AutomationRunError::Processing(format!(
            "resolved provider {} does not match plan provider {}",
            provider.id(),
            request.provider_id
        )));
    }
    require_provider_capabilities(provider, &[ProviderCapability::EditImage])?;
    let record = store.read_record(job_id)?;
    let environment = read_environment_lock(&request.environment_lock_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    step(store, job_id, "terrain:materials", "running", 0.10, None)?;
    let output = generate_terrain_set(
        &record.job_dir.join("exports"),
        &record.job_dir,
        &request.asset,
        &environment,
        &request.provider_id,
        &request.profile_id,
        provider,
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let pack_hash = hash_directory(&output.pack_dir)?;
    for name in [
        "terrain:materials",
        "terrain:compose",
        "terrain:validate",
        "pack:export",
    ] {
        step(store, job_id, name, "succeeded", 0.95, None)?;
    }
    let material_artifacts = output
        .material_paths
        .iter()
        .enumerate()
        .map(|(index, path)| JobArtifactRecord {
            kind: format!("terrain_material_{}", index + 1),
            path: path.clone(),
            sha256: hash_file(path).ok(),
        })
        .collect::<Vec<_>>();
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record.asset_id = Some(request.asset.id.clone());
            record.artifacts.extend([
                JobArtifactRecord {
                    kind: "gsfpack".into(),
                    path: output.pack_dir.clone(),
                    sha256: Some(pack_hash.clone()),
                },
                JobArtifactRecord {
                    kind: "terrain_atlas".into(),
                    path: output.atlas_path.clone(),
                    sha256: hash_file(&output.atlas_path).ok(),
                },
                JobArtifactRecord {
                    kind: "terrain_preview".into(),
                    path: output.preview_path.clone(),
                    sha256: hash_file(&output.preview_path).ok(),
                },
                JobArtifactRecord {
                    kind: "terrain_quality_report".into(),
                    path: output.quality_report_path.clone(),
                    sha256: hash_file(&output.quality_report_path).ok(),
                },
            ]);
            record.artifacts.extend(material_artifacts.clone());
            record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
        })
        .map_err(Into::into)
}

fn run_generate_building_kit(
    store: &JobStore,
    job_id: &str,
    request: &GenerateBuildingKitRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    if provider.id() != request.provider_id {
        return Err(AutomationRunError::Processing(format!(
            "resolved provider {} does not match plan provider {}",
            provider.id(),
            request.provider_id
        )));
    }
    require_provider_capabilities(provider, &[ProviderCapability::EditImage])?;
    let record = store.read_record(job_id)?;
    let environment = read_environment_lock(&request.environment_lock_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    step(store, job_id, "building:materials", "running", 0.10, None)?;
    let output = generate_building_kit(
        &record.job_dir.join("exports"),
        &record.job_dir,
        &request.asset,
        &environment,
        &request.provider_id,
        &request.profile_id,
        provider,
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let pack_hash = hash_directory(&output.pack_dir)?;
    for name in [
        "building:materials",
        "building:compose",
        "building:validate",
        "pack:export",
    ] {
        step(store, job_id, name, "succeeded", 0.95, None)?;
    }
    let material_artifacts = output
        .material_paths
        .iter()
        .enumerate()
        .map(|(index, path)| JobArtifactRecord {
            kind: format!("building_material_{}", index + 1),
            path: path.clone(),
            sha256: hash_file(path).ok(),
        })
        .collect::<Vec<_>>();
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record.asset_id = Some(request.asset.id.clone());
            record.artifacts.extend([
                JobArtifactRecord {
                    kind: "gsfpack".into(),
                    path: output.pack_dir.clone(),
                    sha256: Some(pack_hash.clone()),
                },
                JobArtifactRecord {
                    kind: "building_atlas".into(),
                    path: output.atlas_path.clone(),
                    sha256: hash_file(&output.atlas_path).ok(),
                },
                JobArtifactRecord {
                    kind: "building_preview".into(),
                    path: output.preview_path.clone(),
                    sha256: hash_file(&output.preview_path).ok(),
                },
                JobArtifactRecord {
                    kind: "building_quality_report".into(),
                    path: output.quality_report_path.clone(),
                    sha256: hash_file(&output.quality_report_path).ok(),
                },
            ]);
            record.artifacts.extend(material_artifacts.clone());
            record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
        })
        .map_err(Into::into)
}

fn run_compile_map(
    store: &JobStore,
    job_id: &str,
    request: &CompileMapRequest,
) -> Result<JobRecord, AutomationRunError> {
    let record = store.read_record(job_id)?;
    step(store, job_id, "map:validate_spec", "running", 0.10, None)?;
    let output = match compile_map_pack(&record.job_dir.join("exports"), &request.spec_path) {
        Ok(output) => output,
        Err(error) => {
            let report = record.job_dir.join("exports/map-validation-report.json");
            if report.is_file() {
                store.update_record(job_id, |record| {
                    record.artifacts.push(JobArtifactRecord {
                        kind: "map_validation_report".into(),
                        path: report.clone(),
                        sha256: hash_file(&report).ok(),
                    });
                })?;
            }
            return Err(AutomationRunError::Processing(error.to_string()));
        }
    };
    let pack_hash = hash_directory(&output.pack_dir)?;
    for name in [
        "map:validate_spec",
        "map:compile",
        "map:validate",
        "pack:export",
    ] {
        step(store, job_id, name, "succeeded", 0.95, None)?;
    }
    let map_spec: crate::world::MapSpecV1 = serde_json::from_slice(&fs::read(&request.spec_path)?)?;
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record.asset_id = Some(map_spec.id.clone());
            record.artifacts.extend([
                JobArtifactRecord {
                    kind: "gsfpack".into(),
                    path: output.pack_dir.clone(),
                    sha256: Some(pack_hash.clone()),
                },
                JobArtifactRecord {
                    kind: "map_layout".into(),
                    path: output.map_path.clone(),
                    sha256: Some(output.layout_sha256.clone()),
                },
                JobArtifactRecord {
                    kind: "map_preview".into(),
                    path: output.preview_path.clone(),
                    sha256: hash_file(&output.preview_path).ok(),
                },
                JobArtifactRecord {
                    kind: "map_validation_report".into(),
                    path: output.validation_report_path.clone(),
                    sha256: hash_file(&output.validation_report_path).ok(),
                },
            ]);
            record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
        })
        .map_err(Into::into)
}

fn run_generate_static_asset_set(
    store: &JobStore,
    job_id: &str,
    request: &GenerateStaticAssetSetRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    if provider.id() != request.provider_id {
        return Err(AutomationRunError::Processing(format!(
            "resolved provider {} does not match plan provider {}",
            provider.id(),
            request.provider_id
        )));
    }
    if !request.consistency_recheck_only {
        require_provider_capabilities(provider, &[ProviderCapability::EditImage])?;
    }
    let style = read_style_lock(&request.style_lock_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let record = store.read_record(job_id)?;
    let provider_root = record.job_dir.join("source/provider");
    let normalized_root = record.job_dir.join("normalized/static");
    fs::create_dir_all(&provider_root)?;
    fs::create_dir_all(&normalized_root)?;
    let canvas = match request.asset.kind {
        StaticAssetKind::IconSet => style.icon_canvas_size,
        StaticAssetKind::PropSet => style.prop_canvas_size,
    };
    let mut generated = Vec::<StaticPackItem>::new();
    let mut reports = Vec::<ConsistencyItemReport>::new();
    let mut anchor_path: Option<PathBuf> = None;
    let retry_ids = request
        .retry_item_ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let mut source_attempts = HashMap::<String, u8>::new();

    if let Some(source_job_dir) = &request.reuse_from_job_dir {
        let source_report_path = source_job_dir.join("consistency-report.json");
        let source_report: ConsistencyReportV1 =
            serde_json::from_slice(&fs::read(&source_report_path)?)?;
        for source_item in source_report.items {
            source_attempts.insert(source_item.id.clone(), source_item.attempt);
            if retry_ids.contains(&source_item.id) {
                continue;
            }
            let item = request
                .asset
                .items
                .iter()
                .find(|candidate| candidate.id == source_item.id)
                .ok_or_else(|| {
                    AutomationRunError::Processing(format!(
                        "retry source contains unknown item {}",
                        source_item.id
                    ))
                })?;
            let source_path = source_job_dir
                .join("normalized/static")
                .join(format!("{}.png", item.id));
            validate_retry_source_file(source_job_dir, &source_path)?;
            let normalized = image::open(&source_path).map_err(|error| {
                AutomationRunError::Processing(format!(
                    "retry source item {} is not a valid image: {error}",
                    item.id
                ))
            })?;
            let destination = normalized_root.join(format!("{}.png", item.id));
            fs::copy(&source_path, &destination)?;
            let signature = image_signature(&normalized.to_rgba8());
            let identity_signature = if let Some(reference) = &item.reference_image {
                Some(image_signature(&image::open(reference)?.to_rgba8()))
            } else {
                None
            };
            let report = assess_consistency(
                &item.id,
                source_item.attempt,
                &signature,
                &style,
                identity_signature.as_ref(),
                None,
                canvas,
            );
            if matches!(
                report.verdict,
                ConsistencyVerdict::GameReady | ConsistencyVerdict::AwaitingReview
            ) {
                if anchor_path.is_none() {
                    anchor_path = Some(destination.clone());
                }
                generated.push(StaticPackItem {
                    id: item.id.clone(),
                    name: item.name.clone(),
                    image_path: destination.clone(),
                });
            }
            reports.push(report);
            store.update_record(job_id, |record| {
                record.artifacts.push(JobArtifactRecord {
                    kind: format!("reused_item_{}", item.id),
                    path: destination.clone(),
                    sha256: hash_asset_file(&destination).ok(),
                });
            })?;
        }
    }

    for (item_index, item) in request.asset.items.iter().enumerate() {
        if !retry_ids.is_empty() && !retry_ids.contains(&item.id) {
            continue;
        }
        let step_name = format!("item:{}", item.id);
        let mut accepted = None;
        let mut final_report = None;
        if request.consistency_recheck_only {
            check_cancelled(store, job_id)?;
            step(
                store,
                job_id,
                &step_name,
                "running",
                0.1 + item_index as f32 / request.asset.items.len() as f32 * 0.65,
                Some("local consistency recheck".into()),
            )?;
            let source_job_dir = request.reuse_from_job_dir.as_ref().ok_or_else(|| {
                AutomationRunError::Processing(
                    "consistency recheck requires a source Job directory".into(),
                )
            })?;
            let source_path = source_job_dir
                .join("normalized/static")
                .join(format!("{}.png", item.id));
            validate_retry_source_file(source_job_dir, &source_path)?;
            let destination = normalized_root.join(format!("{}.png", item.id));
            fs::copy(&source_path, &destination)?;
            let normalized = image::open(&destination)?.to_rgba8();
            let signature = image_signature(&normalized);
            let identity_signature = if let Some(reference) = &item.reference_image {
                Some(image_signature(&image::open(reference)?.to_rgba8()))
            } else {
                None
            };
            let report = assess_consistency(
                &item.id,
                source_attempts.get(&item.id).copied().unwrap_or(1),
                &signature,
                &style,
                identity_signature.as_ref(),
                None,
                canvas,
            );
            store.update_record(job_id, |record| {
                record.artifacts.push(JobArtifactRecord {
                    kind: format!("rechecked_item_{}", item.id),
                    path: destination.clone(),
                    sha256: hash_asset_file(&destination).ok(),
                });
            })?;
            if matches!(
                report.verdict,
                ConsistencyVerdict::GameReady | ConsistencyVerdict::AwaitingReview
            ) {
                accepted = Some((destination, signature));
            }
            final_report = Some(report);
        } else {
            for attempt in 1..=request.max_attempts_per_item {
                check_cancelled(store, job_id)?;
                step(
                    store,
                    job_id,
                    &step_name,
                    "running",
                    0.1 + item_index as f32 / request.asset.items.len() as f32 * 0.65,
                    Some(format!("generation attempt {attempt}")),
                )?;
                let attempt_root = provider_root
                    .join(&item.id)
                    .join(format!("attempt-{attempt}"));
                fs::create_dir_all(&attempt_root)?;
                let raw_path = attempt_root.join("source.png");
                let mut references = vec![ProviderImageReference::from_path(
                    ReferenceRole::Style,
                    style.board_path.clone(),
                )
                .map_err(provider_error)?];
                if let Some(anchor) = &anchor_path {
                    references.push(
                        ProviderImageReference::from_path(
                            ReferenceRole::EditTarget,
                            anchor.clone(),
                        )
                        .map_err(provider_error)?,
                    );
                }
                if let Some(reference) = &item.reference_image {
                    references.push(
                        ProviderImageReference::from_path(
                            ReferenceRole::SubjectIdentity,
                            reference.clone(),
                        )
                        .map_err(provider_error)?,
                    );
                }
                references.truncate(3);
                let media = provider
                    .edit_image(
                        &EditImageRequest {
                            prompt: static_item_prompt(request.asset.kind, &style, &item.prompt),
                            model: request
                                .image_model
                                .clone()
                                .or_else(|| style.image_model.clone()),
                            references,
                            aspect_ratio: "1:1".into(),
                            resolution: "1k".into(),
                        },
                        &raw_path,
                    )
                    .map_err(provider_error)?;
                let raw_hash = validate_provider_image(&provider_root, &media)?;
                let normalized_path = normalized_root.join(format!("{}.png", item.id));
                let normalized = normalize_static_image(
                    &media.path,
                    &normalized_path,
                    canvas,
                    request.asset.kind == StaticAssetKind::PropSet,
                )
                .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
                let signature = image_signature(&normalized);
                let identity_signature = if let Some(reference) = &item.reference_image {
                    Some(image_signature(&image::open(reference)?.to_rgba8()))
                } else {
                    None
                };
                let report = assess_consistency(
                    &item.id,
                    source_attempts.get(&item.id).copied().unwrap_or_default() + attempt,
                    &signature,
                    &style,
                    identity_signature.as_ref(),
                    None,
                    canvas,
                );
                store.update_record(job_id, |record| {
                    record.artifacts.push(JobArtifactRecord {
                        kind: format!("provider_item_{}_attempt_{}", item.id, attempt),
                        path: media.path.clone(),
                        sha256: Some(raw_hash.clone()),
                    });
                })?;
                final_report = Some(report.clone());
                if matches!(
                    report.verdict,
                    ConsistencyVerdict::GameReady | ConsistencyVerdict::AwaitingReview
                ) {
                    accepted = Some((normalized_path, signature));
                    break;
                }
            }
        }
        let report = final_report.ok_or_else(|| {
            AutomationRunError::Processing(format!("no consistency result for {}", item.id))
        })?;
        reports.push(report.clone());
        if let Some((path, _signature)) = accepted {
            if anchor_path.is_none() {
                anchor_path = Some(path.clone());
            }
            generated.push(StaticPackItem {
                id: item.id.clone(),
                name: item.name.clone(),
                image_path: path,
            });
            step(
                store,
                job_id,
                &step_name,
                "succeeded",
                0.75,
                Some(format!("{:?}", report.verdict)),
            )?;
        } else {
            step(
                store,
                job_id,
                &step_name,
                "failed",
                0.75,
                Some("consistency gate failed after final attempt".into()),
            )?;
        }
    }
    let item_order = request
        .asset
        .items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.id.as_str(), index))
        .collect::<HashMap<_, _>>();
    generated.sort_by_key(|item| {
        item_order
            .get(item.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    reports.sort_by_key(|item| {
        item_order
            .get(item.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    let verdict =
        aggregate_consistency_verdict(&reports, generated.len() == request.asset.items.len());
    let report = ConsistencyReportV1 {
        schema_version: "1".into(),
        profile: CONSISTENCY_PROFILE.into(),
        asset_type: request.asset.kind.as_str().into(),
        style_revision: style.revision.clone(),
        style_baseline_profile: Some(style.baseline_profile.clone()),
        verdict,
        items: reports,
    };
    let report_path = record.job_dir.join("consistency-report.json");
    write_json_atomic(&report_path, &report)?;
    let contact_sheet = record.job_dir.join("contact-sheet.png");
    if !generated.is_empty() {
        write_contact_sheet(
            &generated
                .iter()
                .map(|item| item.image_path.clone())
                .collect::<Vec<_>>(),
            &contact_sheet,
            canvas,
        )
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    }
    store.update_record(job_id, |record| {
        record.artifacts.push(JobArtifactRecord {
            kind: "consistency_report".into(),
            path: report_path.clone(),
            sha256: hash_asset_file(&report_path).ok(),
        });
        if contact_sheet.is_file() {
            record.artifacts.push(JobArtifactRecord {
                kind: "contact_sheet".into(),
                path: contact_sheet.clone(),
                sha256: hash_asset_file(&contact_sheet).ok(),
            });
        }
    })?;
    if verdict != ConsistencyVerdict::GameReady {
        return store
            .update_record(job_id, |record| {
                record.lifecycle_state = JobLifecycleState::AwaitingReview;
                record.progress = 1.0;
                record.worker_pid = None;
                record.recoverable = true;
                record.error_code = Some("consistency_review_required".into());
                record.error_summary =
                    Some("one or more items require consistency review or targeted retry".into());
                record.next_actions = vec![
                    "job_report".into(),
                    "retry_item".into(),
                    "review_job".into(),
                ];
            })
            .map_err(Into::into);
    }
    let output = export_static_pack(
        &record.job_dir.join("exports"),
        &request.asset,
        &style,
        provider.id(),
        &generated,
        &report,
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let pack_hash = hash_directory(&output.pack_dir)?;
    store.update_record(job_id, |record| {
        record.state = JobState::Exported;
        record.lifecycle_state = JobLifecycleState::Succeeded;
        record.progress = 1.0;
        record.worker_pid = None;
        record.asset_id = Some(request.asset.id.clone());
        record.artifacts.extend([
            JobArtifactRecord {
                kind: "gsfpack".into(),
                path: output.pack_dir.clone(),
                sha256: Some(pack_hash.clone()),
            },
            JobArtifactRecord {
                kind: "contact_sheet".into(),
                path: output.contact_sheet_path.clone(),
                sha256: hash_asset_file(&output.contact_sheet_path).ok(),
            },
        ]);
        record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
    })?;
    let catalog_path = register_catalog_asset(
        &request.project_path,
        ProjectCatalogEntryV1 {
            asset_id: request.asset.id.clone(),
            name: request.asset.name.clone(),
            kind: request.asset.kind.as_str().into(),
            pack_path: output.pack_dir.clone(),
            pack_sha256: pack_hash,
            source_job_id: job_id.into(),
            parent_job_id: record.parent_job_id.clone(),
            style: Some(CatalogStyleRefV1 {
                revision: style.revision.clone(),
            }),
            subject: None,
            workflow: "static-set@1.0.0".into(),
            provider: Some(CatalogProviderRefV1 {
                provider_id: request.provider_id.clone(),
                profile_id: request.profile_id.clone(),
                model: request.image_model.clone().or(style.image_model.clone()),
            }),
            installed: None,
            created_at: Utc::now(),
        },
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let catalog_sha256 = hash_file(&catalog_path)?;
    store.update_record(job_id, |record| {
        record.artifacts.push(JobArtifactRecord {
            kind: "project_catalog".into(),
            path: catalog_path.clone(),
            sha256: Some(catalog_sha256.clone()),
        });
    })?;
    store.read_record(job_id).map_err(Into::into)
}

fn static_item_prompt(
    kind: StaticAssetKind,
    style: &crate::asset_project::StyleLockV1,
    item: &str,
) -> String {
    let subject = match kind {
        StaticAssetKind::IconSet => "one inventory icon",
        StaticAssetKind::PropSet => "one isolated game prop",
    };
    format!(
        "Preserve the exact palette, rendering, line weight, lighting, and camera language of the style references. Create {subject}: {item}. Perspective: {}. Lighting: {}. Outline: {}. Centered, full object, solid chroma green background, no text, no border, no UI, no shadow outside the object.",
        style.perspective, style.lighting, style.outline
    )
}

fn aggregate_consistency_verdict(
    reports: &[ConsistencyItemReport],
    all_materialized: bool,
) -> ConsistencyVerdict {
    if !all_materialized
        || reports
            .iter()
            .any(|report| report.verdict == ConsistencyVerdict::Blocked)
    {
        ConsistencyVerdict::Blocked
    } else if reports
        .iter()
        .any(|report| report.verdict == ConsistencyVerdict::Regenerate)
    {
        ConsistencyVerdict::Regenerate
    } else if reports
        .iter()
        .any(|report| report.verdict == ConsistencyVerdict::AwaitingReview)
    {
        ConsistencyVerdict::AwaitingReview
    } else {
        ConsistencyVerdict::GameReady
    }
}

fn validate_retry_source_file(
    source_job_dir: &Path,
    path: &Path,
) -> Result<(), AutomationRunError> {
    if !source_job_dir.is_dir() || !source_job_dir.join("job.json").is_file() {
        return Err(AutomationRunError::Processing(format!(
            "retry source is not a Forge job directory: {}",
            source_job_dir.display()
        )));
    }
    if !path.is_file() {
        return Err(AutomationRunError::Processing(format!(
            "retry source file is missing: {}",
            path.display()
        )));
    }
    let canonical_root = fs::canonicalize(source_job_dir)?;
    let canonical_path = fs::canonicalize(path)?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(AutomationRunError::Processing(
            "retry source file escaped its Forge job directory".into(),
        ));
    }
    Ok(())
}

fn read_provider_manifest(
    source_job_dir: &Path,
) -> Result<ProviderManifestRecord, AutomationRunError> {
    let path = source_job_dir.join("source/provider-manifest.json");
    if path.is_file() {
        validate_retry_source_file(source_job_dir, &path)?;
        return serde_json::from_slice(&fs::read(path)?).map_err(Into::into);
    }
    reconstruct_provider_manifest(source_job_dir)
}

fn reconstruct_provider_manifest(
    source_job_dir: &Path,
) -> Result<ProviderManifestRecord, AutomationRunError> {
    let job_path = source_job_dir.join("job.json");
    validate_retry_source_file(source_job_dir, &job_path)?;
    let record: JobRecord = serde_json::from_slice(&fs::read(job_path)?)?;
    let reference = record
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "provider_reference")
        .ok_or_else(|| {
            AutomationRunError::Processing(
                "retry source has no Provider reference or manifest".into(),
            )
        })?;
    let reference_sha256 = reference.sha256.clone().ok_or_else(|| {
        AutomationRunError::Processing("retry source Provider reference has no SHA-256".into())
    })?;
    let mut candidates = HashMap::<
        String,
        BTreeMap<u8, (Option<&JobArtifactRecord>, Option<&JobArtifactRecord>)>,
    >::new();
    for artifact in &record.artifacts {
        let (animation, attempt, still) =
            if let Some(kind) = artifact.kind.strip_prefix("provider_still_") {
                let Some((animation, attempt)) = kind.rsplit_once("_attempt_") else {
                    continue;
                };
                let Ok(attempt) = attempt.parse::<u8>() else {
                    continue;
                };
                (animation, attempt, true)
            } else if let Some(kind) = artifact.kind.strip_prefix("provider_video_") {
                let Some((animation, attempt)) = kind.rsplit_once("_attempt_") else {
                    continue;
                };
                let Ok(attempt) = attempt.parse::<u8>() else {
                    continue;
                };
                (animation, attempt, false)
            } else if let Some(animation) = artifact.kind.strip_prefix("reused_still_") {
                (animation, 0, true)
            } else if let Some(animation) = artifact.kind.strip_prefix("reused_video_") {
                (animation, 0, false)
            } else {
                continue;
            };
        let pair = candidates
            .entry(animation.into())
            .or_default()
            .entry(attempt)
            .or_default();
        if still {
            pair.0 = Some(artifact);
        } else {
            pair.1 = Some(artifact);
        }
    }
    let mut animations = BTreeMap::new();
    for (animation, attempts) in candidates {
        let Some((attempt, (still, video))) = attempts
            .into_iter()
            .rev()
            .find(|(_, pair)| pair.0.is_some() && pair.1.is_some())
        else {
            continue;
        };
        let still = still.expect("paired Provider still checked above");
        let video = video.expect("paired Provider video checked above");
        let still_sha256 = still.sha256.clone().ok_or_else(|| {
            AutomationRunError::Processing(format!(
                "retry source Provider still has no SHA-256 for {animation}"
            ))
        })?;
        let video_sha256 = video.sha256.clone().ok_or_else(|| {
            AutomationRunError::Processing(format!(
                "retry source Provider video has no SHA-256 for {animation}"
            ))
        })?;
        animations.insert(
            animation.clone(),
            GeneratedMediaRecord {
                animation,
                attempt,
                still_attempt: attempt,
                video_attempt: attempt,
                retry_method: default_retry_method(),
                still_path: still.path.clone(),
                still_sha256,
                video_path: video.path.clone(),
                video_sha256,
                still_asset_id: None,
                video_asset_id: None,
            },
        );
    }
    if animations.is_empty() {
        return Err(AutomationRunError::Processing(
            "retry source has no complete Provider animation pairs or manifest".into(),
        ));
    }
    Ok(ProviderManifestRecord {
        reference: ProviderManifestReference {
            path: reference.path.clone(),
            sha256: reference_sha256,
            asset_id: None,
        },
        animations,
    })
}

fn persist_failed_character_provider_manifest(
    store: &JobStore,
    job_id: &str,
    request: &GenerateCharacterPackRequest,
    provider: &dyn MediaGenerationProvider,
    provider_usage_baseline: &ProviderUsage,
) -> Result<(), AutomationRunError> {
    let record = store.read_record(job_id)?;
    let reconstructed = reconstruct_provider_manifest(&record.job_dir)?;
    let reference = ProviderMedia {
        path: reconstructed.reference.path,
        mime_type: "image/png".into(),
        provider_asset_id: reconstructed.reference.asset_id,
        revised_prompt: None,
    };
    write_provider_manifest(
        store,
        job_id,
        request,
        provider,
        provider_usage_baseline,
        &reference,
        &reconstructed.reference.sha256,
        &reconstructed.animations,
    )
}

fn copy_reused_character_media(
    source_job_dir: &Path,
    provider_root: &Path,
    name: &str,
    source: &GeneratedMediaRecord,
) -> Result<GeneratedMediaRecord, AutomationRunError> {
    validate_retry_source_file(source_job_dir, &source.still_path)?;
    validate_retry_source_file(source_job_dir, &source.video_path)?;
    if hash_file(&source.still_path)? != source.still_sha256
        || hash_file(&source.video_path)? != source.video_sha256
    {
        return Err(AutomationRunError::Processing(format!(
            "retry source hashes do not match the provider manifest for {name}"
        )));
    }
    let reused_root = provider_root.join(name).join("reused");
    fs::create_dir_all(&reused_root)?;
    let still_extension = source
        .still_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    let video_extension = source
        .video_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("mp4");
    let still_path = reused_root.join(format!("direction.{still_extension}"));
    let video_path = reused_root.join(format!("animation.{video_extension}"));
    fs::copy(&source.still_path, &still_path)?;
    fs::copy(&source.video_path, &video_path)?;
    let still = ProviderMedia {
        path: still_path.clone(),
        mime_type: "image/png".into(),
        provider_asset_id: source.still_asset_id.clone(),
        revised_prompt: None,
    };
    let video = ProviderMedia {
        path: video_path.clone(),
        mime_type: "video/mp4".into(),
        provider_asset_id: source.video_asset_id.clone(),
        revised_prompt: None,
    };
    let still_sha256 = validate_provider_image(provider_root, &still)?;
    let video_sha256 = validate_provider_video(provider_root, &video)?;
    if still_sha256 != source.still_sha256 || video_sha256 != source.video_sha256 {
        return Err(AutomationRunError::Processing(format!(
            "retry source changed while materializing {name}"
        )));
    }
    Ok(GeneratedMediaRecord {
        animation: name.into(),
        attempt: source.attempt,
        still_attempt: source.still_attempt.max(source.attempt),
        video_attempt: source.video_attempt.max(source.attempt),
        retry_method: source.retry_method.clone(),
        still_path,
        still_sha256,
        video_path,
        video_sha256,
        still_asset_id: source.still_asset_id.clone(),
        video_asset_id: source.video_asset_id.clone(),
    })
}

fn record_reused_character_media(
    store: &JobStore,
    job_id: &str,
    name: &str,
    media: &GeneratedMediaRecord,
) -> Result<(), AutomationRunError> {
    store.update_record(job_id, |record| {
        record.artifacts.extend([
            JobArtifactRecord {
                kind: format!("reused_still_{name}"),
                path: media.still_path.clone(),
                sha256: Some(media.still_sha256.clone()),
            },
            JobArtifactRecord {
                kind: format!("reused_video_{name}"),
                path: media.video_path.clone(),
                sha256: Some(media.video_sha256.clone()),
            },
        ]);
    })?;
    Ok(())
}

fn require_provider_capabilities(
    provider: &dyn MediaGenerationProvider,
    required: &[ProviderCapability],
) -> Result<(), AutomationRunError> {
    let available = provider.capabilities();
    let missing = required
        .iter()
        .filter(|capability| !available.contains(capability))
        .map(|capability| format!("{capability:?}"))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(AutomationRunError::Processing(format!(
            "provider_capability_missing: {}",
            missing.join(", ")
        )))
    }
}

fn run_generate_keyframe_character_pack(
    store: &JobStore,
    job_id: &str,
    request: &GenerateCharacterPackRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    let usage_baseline = provider.usage();
    if provider.id() != request.provider_id {
        return Err(AutomationRunError::Processing(format!(
            "resolved provider {} does not match plan provider {}",
            provider.id(),
            request.provider_id
        )));
    }
    require_provider_capabilities(provider, &[ProviderCapability::EditImage])?;
    let health = provider.health_check();
    if !health.available || !health.authenticated {
        return Err(AutomationRunError::Processing(
            health
                .message
                .unwrap_or_else(|| format!("provider {} is not authenticated", provider.id())),
        ));
    }
    if health
        .constraints
        .as_ref()
        .and_then(|constraints| constraints.max_image_references)
        .is_some_and(|maximum| maximum < 3)
    {
        return Err(AutomationRunError::Processing(
            "provider_reference_limit: topdown-keyframes@2.0.0 requires three image references"
                .into(),
        ));
    }
    let style_path = request.style_lock_path.as_deref().ok_or_else(|| {
        AutomationRunError::Processing("keyframe workflow requires styleLockPath".into())
    })?;
    let subject_path = request.subject_lock_path.as_deref().ok_or_else(|| {
        AutomationRunError::Processing("keyframe workflow requires subjectLockPath".into())
    })?;
    let style = read_style_lock(style_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let subject = read_subject_lock(subject_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    if subject.style_revision != style.revision
        || subject.provider_id != request.provider_id
        || subject.profile_id != request.profile_id
    {
        return Err(AutomationRunError::Processing(
            "SubjectLock, StyleLock, and Provider profile do not match".into(),
        ));
    }
    let image_model = provider
        .resolved_image_model(
            request
                .generation
                .image_model
                .as_deref()
                .or(subject.image_model.as_deref())
                .or(style.image_model.as_deref()),
        )
        .ok_or_else(|| {
            AutomationRunError::Processing(
                "provider_model_unresolved: keyframe image model must be locked before execution"
                    .into(),
            )
        })?;

    let record = store.read_record(job_id)?;
    let provider_root = record.job_dir.join("source/provider-keyframes");
    let pose_root = record
        .job_dir
        .join("source/pose-guides/topdown-poses@1.0.0");
    let accepted_root = record.job_dir.join("keyframes");
    fs::create_dir_all(&provider_root)?;
    fs::create_dir_all(&pose_root)?;
    fs::create_dir_all(&accepted_root)?;
    let cache = ContentCache::default_store()
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let identity_image = image::open(&subject.canonical_path)?.to_rgba8();
    let identity = image_signature(&identity_image);
    let source_graph = request
        .reuse_from_job_dir
        .as_ref()
        .map(|source| read_workflow_graph(&source.join(WORKFLOW_GRAPH_FILE)))
        .transpose()
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let selected_retry_frames = request
        .retry_frames
        .iter()
        .flat_map(|(animation, frames)| frames.iter().map(move |frame| (animation.clone(), *frame)))
        .collect::<HashSet<_>>();
    let provider_frame_retry = request
        .retry_stages
        .values()
        .any(|stage| *stage == CharacterRetryStage::Frame);

    let actions = [
        ("idle", 8.0_f32),
        ("walk_up", 12.0_f32),
        ("walk_right", 12.0_f32),
        ("walk_down", 12.0_f32),
    ];
    let mut reports = Vec::<ConsistencyItemReport>::new();
    let mut graph_nodes = Vec::<WorkflowNodeV1>::new();
    let mut frame_groups = BTreeMap::<String, Vec<PathBuf>>::new();
    let mut manifest_frames = Vec::<serde_json::Value>::new();

    for (action_index, (action, _)) in actions.iter().enumerate() {
        let action_root = accepted_root.join(action);
        fs::create_dir_all(&action_root)?;
        let mut paths = Vec::with_capacity(8);
        for frame in 0..8_u8 {
            check_cancelled(store, job_id)?;
            let node_id = format!("frame_image:{action}:{frame}");
            let output_path = action_root.join(format!("frame-{frame:02}.png"));
            let pose_path = pose_root.join(format!("{action}-frame-{frame:02}.png"));
            write_pose_guide(&pose_path, action, frame, style.character_canvas_size)?;
            let pose_sha256 = hash_file(&pose_path)?;
            let pose_image = image::open(&pose_path)?.to_rgba8();
            let should_regenerate = request.reuse_from_job_dir.is_none()
                || (provider_frame_retry
                    && selected_retry_frames.contains(&((*action).to_string(), frame)));
            if !should_regenerate {
                let graph = source_graph.as_ref().ok_or_else(|| {
                    AutomationRunError::Processing(
                        "legacy_artifact_missing: retry source has no WorkflowGraphV1".into(),
                    )
                })?;
                let source_node = graph
                    .nodes
                    .iter()
                    .find(|node| node.id == node_id)
                    .ok_or_else(|| {
                        AutomationRunError::Processing(format!(
                            "legacy_artifact_missing: retry source has no {node_id}"
                        ))
                    })?;
                let source_output = source_node.outputs.first().ok_or_else(|| {
                    AutomationRunError::Processing(format!(
                        "legacy_artifact_missing: {node_id} has no output"
                    ))
                })?;
                let source_root = request
                    .reuse_from_job_dir
                    .as_deref()
                    .expect("reuse root checked above");
                validate_retry_source_file(source_root, &source_output.path)?;
                if hash_file(&source_output.path)? != source_output.sha256 {
                    return Err(AutomationRunError::Processing(format!(
                        "retry source frame hash changed: {node_id}"
                    )));
                }
                fs::copy(&source_output.path, &output_path)?;
                let actual = hash_file(&output_path)?;
                let reused_image = image::open(&output_path)?.to_rgba8();
                let signature = image_signature(&reused_image);
                let mut report = assess_consistency(
                    &format!("{action}/frame-{frame:02}"),
                    0,
                    &signature,
                    &style,
                    Some(&identity),
                    Some(&identity),
                    style.character_canvas_size,
                );
                apply_keyframe_hard_defects(
                    &reused_image,
                    &identity_image,
                    &pose_image,
                    &mut report,
                );
                reports.push(report);
                let mut reused = source_node.clone();
                reused.outputs = vec![WorkflowArtifactV1 {
                    sha256: actual,
                    path: output_path.clone(),
                }];
                reused.cache_hit = true;
                reused.provider_request = false;
                graph_nodes.push(reused);
                paths.push(output_path);
                continue;
            }

            let input_sha256 = vec![
                subject.canonical_sha256.clone(),
                style.board_sha256.clone(),
                pose_sha256.clone(),
            ];
            let explicit_retry = selected_retry_frames.contains(&((*action).to_string(), frame));
            let first_attempt = if explicit_retry { 2 } else { 1 };
            let mut final_report = None;
            let mut final_raw_sha = None;
            let mut final_cache_key = None;
            let mut provider_request = false;
            let mut cache_hit = false;
            let mut attempt_used = first_attempt;
            for attempt in first_attempt..=2 {
                attempt_used = attempt;
                let parameters = serde_json::json!({
                    "workflow": "topdown-keyframes@2.0.0",
                    "poseProfile": "topdown-poses@1.0.0",
                    "styleRevision": style.revision,
                    "subjectRevision": subject.revision,
                    "action": action,
                    "frame": frame,
                    "attempt": attempt,
                    "canvas": style.character_canvas_size,
                });
                let cache_key = compute_cache_key(
                    "frame_image",
                    "provider-image-edit@2.0.0",
                    Some(provider.id()),
                    Some(&image_model),
                    &parameters,
                    &input_sha256,
                )
                .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
                let attempt_root = provider_root
                    .join(action)
                    .join(format!("frame-{frame:02}"))
                    .join(format!("attempt-{attempt}"));
                fs::create_dir_all(&attempt_root)?;
                let raw_path = attempt_root.join("source.png");
                let can_use_cache = !explicit_retry && attempt == 1;
                let cached_sha = if can_use_cache {
                    cache
                        .lookup_output_sha256(&cache_key)
                        .map_err(|error| AutomationRunError::Processing(error.to_string()))?
                } else {
                    None
                };
                let raw_sha256 = if let Some(expected) = cached_sha {
                    if cache
                        .materialize_file(&cache_key, &expected, &raw_path)
                        .map_err(|error| AutomationRunError::Processing(error.to_string()))?
                    {
                        cache_hit = true;
                        expected
                    } else {
                        return Err(AutomationRunError::Processing(
                            "cache metadata existed without its object".into(),
                        ));
                    }
                } else {
                    provider_request = true;
                    let references = vec![
                        ProviderImageReference::from_path(
                            ReferenceRole::SubjectIdentity,
                            subject.canonical_path.clone(),
                        )
                        .map_err(provider_error)?,
                        ProviderImageReference::from_path(
                            ReferenceRole::Style,
                            style.board_path.clone(),
                        )
                        .map_err(provider_error)?,
                        ProviderImageReference::from_path(
                            ReferenceRole::PoseStructure,
                            pose_path.clone(),
                        )
                        .map_err(provider_error)?,
                    ];
                    let media = provider
                        .edit_image(
                            &EditImageRequest {
                                prompt: keyframe_prompt(action, frame, attempt),
                                model: Some(image_model.clone()),
                                references,
                                aspect_ratio: "1:1".into(),
                                resolution: "1k".into(),
                            },
                            &raw_path,
                        )
                        .map_err(provider_error)?;
                    let sha = validate_provider_image(&provider_root, &media)?;
                    cache
                        .put_file(&cache_key, &media.path)
                        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
                    sha
                };
                let normalized = normalize_static_image(
                    &raw_path,
                    &output_path,
                    style.character_canvas_size,
                    true,
                )
                .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
                let signature = image_signature(&normalized);
                let mut report = assess_consistency(
                    &format!("{action}/frame-{frame:02}"),
                    attempt,
                    &signature,
                    &style,
                    Some(&identity),
                    Some(&identity),
                    style.character_canvas_size,
                );
                apply_keyframe_hard_defects(&normalized, &identity_image, &pose_image, &mut report);
                final_raw_sha = Some(raw_sha256);
                final_cache_key = Some(cache_key);
                let accepted = report.verdict == ConsistencyVerdict::GameReady;
                final_report = Some(report);
                if accepted {
                    break;
                }
            }
            let report = final_report.expect("keyframe attempt always runs");
            let output_sha256 = hash_file(&output_path)?;
            reports.push(report.clone());
            let cache_key = final_cache_key.expect("keyframe attempt sets cache key");
            graph_nodes.push(WorkflowNodeV1 {
                id: node_id.clone(),
                stage: "frame_image".into(),
                item: Some((*action).into()),
                frame: Some(frame),
                implementation_version: "provider-image-edit@2.0.0".into(),
                depends_on: Vec::new(),
                invalidates: vec![
                    format!("matting:{action}"),
                    "shared_normalize".into(),
                    "collection_consistency".into(),
                    "loop_quality".into(),
                    "pack".into(),
                    "godot_install".into(),
                ],
                inputs: vec![
                    WorkflowArtifactV1 {
                        sha256: subject.canonical_sha256.clone(),
                        path: subject.canonical_path.clone(),
                    },
                    WorkflowArtifactV1 {
                        sha256: style.board_sha256.clone(),
                        path: style.board_path.clone(),
                    },
                    WorkflowArtifactV1 {
                        sha256: pose_sha256,
                        path: pose_path.clone(),
                    },
                ],
                outputs: vec![WorkflowArtifactV1 {
                    sha256: output_sha256.clone(),
                    path: output_path.clone(),
                }],
                cache_key,
                provider_request,
                cache_hit,
                provider_id: Some(provider.id().into()),
                model: Some(image_model.clone()),
            });
            manifest_frames.push(serde_json::json!({
                "animation": action,
                "frame": frame,
                "attempt": attempt_used,
                "roles": ["subject_identity", "style", "pose_structure"],
                "rawSha256": final_raw_sha,
                "outputSha256": output_sha256,
                "cacheKey": graph_nodes.last().map(|node| &node.cache_key),
                "cacheHit": cache_hit,
                "providerRequest": provider_request,
            }));
            store.update_record(job_id, |record| {
                record.artifacts.push(JobArtifactRecord {
                    kind: format!("keyframe_{action}_{frame:02}"),
                    path: output_path.clone(),
                    sha256: Some(output_sha256.clone()),
                });
            })?;
            paths.push(output_path);
            let completed = action_index * 8 + frame as usize + 1;
            step(
                store,
                job_id,
                &format!("provider:{action}:frame-{frame:02}"),
                "succeeded",
                0.05 + completed as f32 / 32.0 * 0.45,
                None,
            )?;
        }
        frame_groups.insert((*action).into(), paths);
    }

    let overall = aggregate_keyframe_consistency_verdict(&reports);
    let consistency = ConsistencyReportV1 {
        schema_version: "1".into(),
        profile: CONSISTENCY_PROFILE.into(),
        asset_type: "character".into(),
        style_revision: style.revision.clone(),
        style_baseline_profile: Some(style.baseline_profile.clone()),
        verdict: overall,
        items: reports,
    };
    let consistency_path = record.job_dir.join("consistency-report.json");
    write_json_atomic(&consistency_path, &consistency)?;
    let consistency_sha256 = hash_file(&consistency_path)?;
    let usage = provider_usage_delta(&provider.usage(), &usage_baseline);
    let provider_manifest_path = record
        .job_dir
        .join("source/keyframe-provider-manifest.json");
    write_json_atomic(
        &provider_manifest_path,
        &serde_json::json!({
            "schemaVersion": "1",
            "providerId": provider.id(),
            "profileId": request.profile_id,
            "model": image_model.clone(),
            "hardGateProfile": KEYFRAME_HARD_GATE_PROFILE,
            "workflow": "topdown-keyframes@2.0.0",
            "subject": {
                "id": subject.id,
                "revision": subject.revision,
                "sha256": subject.canonical_sha256,
            },
            "styleRevision": style.revision,
            "frames": manifest_frames,
            "usage": usage,
        }),
    )?;
    let manifest_sha256 = hash_file(&provider_manifest_path)?;
    store.update_record(job_id, |record| {
        record.artifacts.extend([
            JobArtifactRecord {
                kind: "consistency_report".into(),
                path: consistency_path.clone(),
                sha256: Some(consistency_sha256.clone()),
            },
            JobArtifactRecord {
                kind: "provider_manifest".into(),
                path: provider_manifest_path.clone(),
                sha256: Some(manifest_sha256.clone()),
            },
        ]);
    })?;

    let mut graph = WorkflowGraphV1 {
        schema_version: "1".into(),
        workflow: "topdown-keyframes@2.0.0".into(),
        job_id: job_id.into(),
        parent_job_id: record.parent_job_id.clone(),
        nodes: graph_nodes,
    };
    append_keyframe_prepack_graph(
        &mut graph,
        &actions,
        &frame_groups,
        &consistency_path,
        &consistency_sha256,
    )?;
    write_workflow_graph(&record.job_dir.join(WORKFLOW_GRAPH_FILE), &graph)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;

    let consistency_review_required = consistency.verdict == ConsistencyVerdict::AwaitingReview;
    if matches!(
        consistency.verdict,
        ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
    ) {
        return store
            .update_record(job_id, |record| {
                record.state = JobState::QualityChecked;
                record.lifecycle_state = JobLifecycleState::Failed;
                record.progress = 1.0;
                record.worker_pid = None;
                record.recoverable = true;
                record.error_code = Some("keyframe_regeneration_required".into());
                record.next_actions = vec!["job_report".into(), "retry_frame".into()];
            })
            .map_err(Into::into);
    }

    let mut prepare = PrepareCharacterPackRequest {
        schema_version: "2".into(),
        metadata: request.metadata.clone(),
        workflow: request.workflow.clone(),
        animations: actions
            .iter()
            .map(|(action, fps)| CharacterAnimationRecipe {
                name: (*action).into(),
                input: AssetInput::PngSequence {
                    paths: frame_groups
                        .get(*action)
                        .expect("generated keyframe group")
                        .clone(),
                },
                fps: *fps,
                loop_animation: true,
                matting: MattingRecipe::PreserveAlpha,
            })
            .collect(),
        normalize: request.normalize,
        sheet: request.sheet,
        quality: request.quality.clone(),
    };
    // A gray consistency result may be exported only as a candidate for an explicit
    // human review. The hard consistency verdicts returned above never reach export.
    if consistency_review_required {
        prepare.quality.require_game_ready = false;
    }
    let completed = run_prepare_character_pack(store, job_id, &prepare)?;
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .cloned();
    if let Some(pack) = &pack {
        append_keyframe_postprocess_graph(&mut graph, &record.job_dir, &actions)?;
        let pack_key = compute_cache_key(
            "pack",
            "gsfpack@2.0.0",
            None,
            None,
            &serde_json::json!({"workflow": graph.workflow}),
            &pack.sha256.clone().into_iter().collect::<Vec<_>>(),
        )
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        graph.nodes.push(WorkflowNodeV1 {
            id: "pack".into(),
            stage: "pack".into(),
            item: None,
            frame: None,
            implementation_version: "gsfpack@2.0.0".into(),
            depends_on: vec!["shared_normalize".into()],
            invalidates: vec!["godot_install".into()],
            inputs: vec![WorkflowArtifactV1 {
                sha256: consistency_sha256.clone(),
                path: consistency_path.clone(),
            }],
            outputs: vec![WorkflowArtifactV1 {
                sha256: pack
                    .sha256
                    .clone()
                    .unwrap_or_else(|| hash_directory(&pack.path).unwrap_or_default()),
                path: pack.path.clone(),
            }],
            cache_key: pack_key,
            provider_request: false,
            cache_hit: false,
            provider_id: None,
            model: None,
        });
        write_workflow_graph(&record.job_dir.join(WORKFLOW_GRAPH_FILE), &graph)
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        let graph_sha256 = hash_file(&record.job_dir.join(WORKFLOW_GRAPH_FILE))?;
        store.update_record(job_id, |record| {
            record
                .artifacts
                .retain(|artifact| artifact.kind != "workflow_graph");
            record.artifacts.push(JobArtifactRecord {
                kind: "workflow_graph".into(),
                path: record.job_dir.join(WORKFLOW_GRAPH_FILE),
                sha256: Some(graph_sha256.clone()),
            });
        })?;
        if !consistency_review_required {
            if let (Some(project_root), Some(asset_id), Some(pack_sha256)) = (
                request.project_path.as_ref(),
                request.asset_id.as_ref(),
                pack.sha256.as_ref(),
            ) {
                let catalog_path = register_catalog_asset(
                    project_root,
                    ProjectCatalogEntryV1 {
                        asset_id: asset_id.clone(),
                        name: request.metadata.name.clone(),
                        kind: "character".into(),
                        pack_path: pack.path.clone(),
                        pack_sha256: pack_sha256.clone(),
                        source_job_id: job_id.into(),
                        parent_job_id: record.parent_job_id.clone(),
                        style: Some(CatalogStyleRefV1 {
                            revision: style.revision.clone(),
                        }),
                        subject: Some(CatalogSubjectRefV1 {
                            id: subject.id.clone(),
                            revision: subject.revision.clone(),
                        }),
                        workflow: "topdown-keyframes@2.0.0".into(),
                        provider: Some(CatalogProviderRefV1 {
                            provider_id: request.provider_id.clone(),
                            profile_id: request.profile_id.clone(),
                            model: Some(image_model.clone()),
                        }),
                        installed: None,
                        created_at: Utc::now(),
                    },
                )
                .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
                let catalog_sha256 = hash_file(&catalog_path)?;
                store.update_record(job_id, |record| {
                    record.artifacts.push(JobArtifactRecord {
                        kind: "project_catalog".into(),
                        path: catalog_path.clone(),
                        sha256: Some(catalog_sha256.clone()),
                    });
                })?;
            }
        }
    }
    if consistency_review_required {
        return store
            .update_record(job_id, |record| {
                if let Some(artifact) = record
                    .artifacts
                    .iter_mut()
                    .find(|artifact| artifact.kind == "gsfpack")
                {
                    artifact.kind = "candidate_gsfpack".into();
                }
                record.state = JobState::QualityChecked;
                record.lifecycle_state = JobLifecycleState::AwaitingReview;
                record.progress = 1.0;
                record.worker_pid = None;
                record.recoverable = true;
                record.error_code = Some("keyframe_review_required".into());
                record.error_summary =
                    Some("keyframe consistency is in the reviewable gray range".into());
                record.next_actions = vec![
                    "job_report".into(),
                    "review_candidate".into(),
                    "retry_frame".into(),
                ];
            })
            .map_err(Into::into);
    }
    store.read_record(job_id).map_err(Into::into)
}

fn append_keyframe_prepack_graph(
    graph: &mut WorkflowGraphV1,
    actions: &[(&str, f32)],
    frame_groups: &BTreeMap<String, Vec<PathBuf>>,
    consistency_path: &Path,
    consistency_sha256: &str,
) -> Result<(), AutomationRunError> {
    let mut alignment_nodes = Vec::new();
    for (action, _) in actions {
        let paths = frame_groups.get(*action).ok_or_else(|| {
            AutomationRunError::Processing(format!("missing frames for {action}"))
        })?;
        let artifacts = workflow_artifacts(paths)?;
        let input_sha256 = artifacts
            .iter()
            .map(|artifact| artifact.sha256.clone())
            .collect::<Vec<_>>();
        let frame_nodes = (0..paths.len())
            .map(|frame| format!("frame_image:{action}:{frame}"))
            .collect::<Vec<_>>();
        let matting_id = format!("matting:{action}");
        let matting_key = compute_cache_key(
            "matting",
            "preserve-alpha@1.0.0",
            None,
            None,
            &serde_json::json!({"action": action}),
            &input_sha256,
        )
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        graph.nodes.push(WorkflowNodeV1 {
            id: matting_id.clone(),
            stage: "matting".into(),
            item: Some((*action).into()),
            frame: None,
            implementation_version: "preserve-alpha@1.0.0".into(),
            depends_on: frame_nodes,
            invalidates: vec![
                format!("provisional_align:{action}"),
                "collection_consistency".into(),
                format!("loop_quality:{action}"),
                "shared_normalize".into(),
                "pack".into(),
                "godot_install".into(),
            ],
            inputs: artifacts.clone(),
            outputs: artifacts.clone(),
            cache_key: matting_key,
            provider_request: false,
            cache_hit: false,
            provider_id: None,
            model: None,
        });
        let alignment_id = format!("provisional_align:{action}");
        let alignment_key = compute_cache_key(
            "provisional_align",
            "foot-anchor@1.0.0",
            None,
            None,
            &serde_json::json!({"action": action}),
            &input_sha256,
        )
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        graph.nodes.push(WorkflowNodeV1 {
            id: alignment_id.clone(),
            stage: "provisional_align".into(),
            item: Some((*action).into()),
            frame: None,
            implementation_version: "foot-anchor@1.0.0".into(),
            depends_on: vec![matting_id],
            invalidates: vec![
                "collection_consistency".into(),
                format!("loop_quality:{action}"),
                "shared_normalize".into(),
                "pack".into(),
                "godot_install".into(),
            ],
            inputs: artifacts.clone(),
            outputs: artifacts,
            cache_key: alignment_key,
            provider_request: false,
            cache_hit: false,
            provider_id: None,
            model: None,
        });
        alignment_nodes.push(alignment_id);
    }
    let consistency_inputs = graph
        .nodes
        .iter()
        .filter(|node| node.stage == "provisional_align")
        .flat_map(|node| node.outputs.iter().map(|output| output.sha256.clone()))
        .collect::<Vec<_>>();
    let consistency_key = compute_cache_key(
        "collection_consistency",
        CONSISTENCY_PROFILE,
        None,
        None,
        &serde_json::json!({"assetType": "character"}),
        &consistency_inputs,
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    graph.nodes.push(WorkflowNodeV1 {
        id: "collection_consistency".into(),
        stage: "collection_consistency".into(),
        item: None,
        frame: None,
        implementation_version: CONSISTENCY_PROFILE.into(),
        depends_on: alignment_nodes,
        invalidates: vec![
            "loop_quality:idle".into(),
            "loop_quality:walk_up".into(),
            "loop_quality:walk_right".into(),
            "loop_quality:walk_down".into(),
            "shared_normalize".into(),
            "pack".into(),
            "godot_install".into(),
        ],
        inputs: Vec::new(),
        outputs: vec![WorkflowArtifactV1 {
            sha256: consistency_sha256.into(),
            path: consistency_path.into(),
        }],
        cache_key: consistency_key,
        provider_request: false,
        cache_hit: false,
        provider_id: None,
        model: None,
    });
    Ok(())
}

fn append_keyframe_postprocess_graph(
    graph: &mut WorkflowGraphV1,
    job_dir: &Path,
    actions: &[(&str, f32)],
) -> Result<(), AutomationRunError> {
    let quality_path = job_dir.join("animation-quality-report.json");
    let loop_path = job_dir.join("loop-selection-report.json");
    let quality_artifacts = workflow_artifacts(&[quality_path, loop_path])?;
    let quality_sha256 = quality_artifacts
        .iter()
        .map(|artifact| artifact.sha256.clone())
        .collect::<Vec<_>>();
    let mut loop_nodes = Vec::new();
    for (action, _) in actions {
        let id = format!("loop_quality:{action}");
        let cache_key = compute_cache_key(
            "loop_quality",
            LOOP_SELECTION_PROFILE,
            None,
            None,
            &serde_json::json!({"action": action, "workflow": graph.workflow}),
            &quality_sha256,
        )
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        graph.nodes.push(WorkflowNodeV1 {
            id: id.clone(),
            stage: "loop_quality".into(),
            item: Some((*action).into()),
            frame: None,
            implementation_version: LOOP_SELECTION_PROFILE.into(),
            depends_on: vec![
                format!("provisional_align:{action}"),
                "collection_consistency".into(),
            ],
            invalidates: vec![
                "shared_normalize".into(),
                "pack".into(),
                "godot_install".into(),
            ],
            inputs: Vec::new(),
            outputs: quality_artifacts.clone(),
            cache_key,
            provider_request: false,
            cache_hit: false,
            provider_id: None,
            model: None,
        });
        loop_nodes.push(id);
    }
    let normalized_root = job_dir.join("processed/normalized");
    let mut normalized_paths = fs::read_dir(&normalized_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("png"))
        .collect::<Vec<_>>();
    normalized_paths.sort();
    if normalized_paths.len() != 32 {
        return Err(AutomationRunError::Processing(format!(
            "keyframe shared normalization produced {} frames instead of 32",
            normalized_paths.len()
        )));
    }
    let normalized_artifacts = workflow_artifacts(&normalized_paths)?;
    let normalized_sha256 = normalized_artifacts
        .iter()
        .map(|artifact| artifact.sha256.clone())
        .collect::<Vec<_>>();
    let normalize_key = compute_cache_key(
        "shared_normalize",
        "shared-normalize@1.0.0",
        None,
        None,
        &serde_json::json!({"frameCount": 32}),
        &normalized_sha256,
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    graph.nodes.push(WorkflowNodeV1 {
        id: "shared_normalize".into(),
        stage: "shared_normalize".into(),
        item: None,
        frame: None,
        implementation_version: "shared-normalize@1.0.0".into(),
        depends_on: loop_nodes,
        invalidates: vec!["pack".into(), "godot_install".into()],
        inputs: quality_artifacts,
        outputs: normalized_artifacts,
        cache_key: normalize_key,
        provider_request: false,
        cache_hit: false,
        provider_id: None,
        model: None,
    });
    Ok(())
}

fn workflow_artifacts(paths: &[PathBuf]) -> Result<Vec<WorkflowArtifactV1>, AutomationRunError> {
    paths
        .iter()
        .map(|path| {
            Ok(WorkflowArtifactV1 {
                sha256: hash_file(path)?,
                path: path.clone(),
            })
        })
        .collect()
}

fn aggregate_keyframe_consistency_verdict(items: &[ConsistencyItemReport]) -> ConsistencyVerdict {
    if items
        .iter()
        .any(|item| item.verdict == ConsistencyVerdict::Blocked)
    {
        ConsistencyVerdict::Blocked
    } else if items
        .iter()
        .any(|item| item.verdict == ConsistencyVerdict::Regenerate)
    {
        ConsistencyVerdict::Regenerate
    } else if items
        .iter()
        .any(|item| item.verdict == ConsistencyVerdict::AwaitingReview)
    {
        ConsistencyVerdict::AwaitingReview
    } else {
        ConsistencyVerdict::GameReady
    }
}

fn keyframe_prompt(action: &str, frame: u8, attempt: u8) -> String {
    let repair = if attempt > 1 {
        " This is a corrective retry: preserve identity more strictly and remove clipping or extra subjects."
    } else {
        ""
    };
    format!(
        "Generate exactly one transparent-background 2D game sprite frame. Reference 1 is the immutable subject identity, reference 2 is the immutable style, and reference 3 is pose structure only. Preserve face, hair, proportions, clothing, colors, equipment, camera, scale, and lighting. Action {action}; Forge frame phase {frame}/8. Follow the pose phase without copying guide colors. One complete centered subject, fixed top-down orthographic camera, feet aligned, no text, no UI, no scene, no extra object.{repair}"
    )
}

fn write_pose_guide(
    path: &Path,
    action: &str,
    frame: u8,
    canvas: u32,
) -> Result<(), AutomationRunError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut image: RgbaImage = ImageBuffer::from_pixel(canvas, canvas, Rgba([255, 255, 255, 255]));
    let center = (canvas / 2) as i32;
    let scale = (canvas as f32 / 256.0).max(0.25);
    let phase = frame as f32 / 8.0 * std::f32::consts::TAU;
    let swing = (phase.sin() * 18.0 * scale).round() as i32;
    let bob = if action == "idle" {
        (phase.sin() * 3.0 * scale).round() as i32
    } else {
        (phase.cos().abs() * 4.0 * scale).round() as i32
    };
    let head_y = (canvas as f32 * 0.29) as i32 + bob;
    let shoulder_y = (canvas as f32 * 0.43) as i32 + bob;
    let hip_y = (canvas as f32 * 0.62) as i32 + bob;
    let foot_y = (canvas as f32 * 0.84) as i32;
    let ink = match action {
        "walk_up" => Rgba([45, 90, 220, 255]),
        "walk_right" => Rgba([220, 90, 45, 255]),
        "walk_down" => Rgba([45, 170, 90, 255]),
        _ => Rgba([145, 65, 200, 255]),
    };
    draw_disc(&mut image, center, head_y, (18.0 * scale) as i32, ink);
    draw_line(
        &mut image,
        center,
        head_y + 18,
        center,
        hip_y,
        ink,
        (7.0 * scale) as i32,
    );
    draw_line(
        &mut image,
        center - (28.0 * scale) as i32,
        shoulder_y + swing / 3,
        center + (28.0 * scale) as i32,
        shoulder_y - swing / 3,
        ink,
        (6.0 * scale) as i32,
    );
    let leg_swing = if action == "idle" { 0 } else { swing };
    draw_line(
        &mut image,
        center,
        hip_y,
        center - (14.0 * scale) as i32 + leg_swing,
        foot_y,
        ink,
        (7.0 * scale) as i32,
    );
    draw_line(
        &mut image,
        center,
        hip_y,
        center + (14.0 * scale) as i32 - leg_swing,
        foot_y,
        ink,
        (7.0 * scale) as i32,
    );
    image.save(path)?;
    Ok(())
}

fn draw_line(
    image: &mut RgbaImage,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
    color: Rgba<u8>,
    radius: i32,
) {
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        draw_disc(image, x0, y0, radius.max(1), color);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let twice = 2 * error;
        if twice >= dy {
            error += dy;
            x0 += sx;
        }
        if twice <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn draw_disc(image: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            if x >= 0
                && y >= 0
                && x < image.width() as i32
                && y < image.height() as i32
                && (x - cx).pow(2) + (y - cy).pow(2) <= radius.pow(2)
            {
                image.put_pixel(x as u32, y as u32, color);
            }
        }
    }
}

fn run_generate_character_pack(
    store: &JobStore,
    job_id: &str,
    request: &GenerateCharacterPackRequest,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    if request.workflow.id == "topdown-keyframes" && request.workflow.version == "2.0.0" {
        return run_generate_keyframe_character_pack(store, job_id, request, provider);
    }
    let provider_usage_baseline = provider.usage();
    if provider.id() != request.provider_id {
        return Err(AutomationRunError::Processing(format!(
            "resolved provider {} does not match plan provider {}",
            provider.id(),
            request.provider_id
        )));
    }
    require_provider_capabilities(
        provider,
        &[
            ProviderCapability::EditImage,
            ProviderCapability::ImageToVideo,
        ],
    )?;
    let style = request
        .style_lock_path
        .as_deref()
        .map(read_style_lock)
        .transpose()
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let health = provider.health_check();
    if !health.available || !health.authenticated {
        return Err(AutomationRunError::Processing(
            health
                .message
                .unwrap_or_else(|| format!("provider {} is not authenticated", provider.id())),
        ));
    }

    let record = store.read_record(job_id)?;
    let provider_root = record.job_dir.join("source/provider");
    fs::create_dir_all(&provider_root)?;
    let reused_manifest = request
        .reuse_from_job_dir
        .as_deref()
        .map(read_provider_manifest)
        .transpose()?;
    step(store, job_id, "provider:reference", "running", 0.04, None)?;
    let reference_path = provider_root.join("reference/reference.png");
    let reference_media = if let (Some(source_job_dir), Some(manifest)) =
        (&request.reuse_from_job_dir, &reused_manifest)
    {
        validate_retry_source_file(source_job_dir, &manifest.reference.path)?;
        let actual_hash = hash_file(&manifest.reference.path)?;
        if actual_hash != manifest.reference.sha256 {
            return Err(AutomationRunError::Processing(
                "retry source reference hash does not match its provider manifest".into(),
            ));
        }
        if let Some(parent) = reference_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&manifest.reference.path, &reference_path)?;
        ProviderMedia {
            path: reference_path.clone(),
            mime_type: "image/png".into(),
            provider_asset_id: manifest.reference.asset_id.clone(),
            revised_prompt: None,
        }
    } else if let Some(style) = &style {
        let mut references =
            vec![
                ProviderImageReference::from_path(ReferenceRole::Style, style.board_path.clone())
                    .map_err(provider_error)?,
            ];
        if let Some(source) = &request.character.reference_image_path {
            references.push(
                ProviderImageReference::from_path(ReferenceRole::SubjectIdentity, source.clone())
                    .map_err(provider_error)?,
            );
        }
        provider
            .edit_image(
                &EditImageRequest {
                    prompt: canonical_reference_prompt(&request.character.prompt),
                    model: request
                        .generation
                        .image_model
                        .clone()
                        .or_else(|| style.image_model.clone()),
                    references,
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                },
                &reference_path,
            )
            .map_err(provider_error)?
    } else if let Some(source) = &request.character.reference_image_path {
        if let Some(parent) = reference_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, &reference_path)?;
        ProviderMedia {
            path: reference_path.clone(),
            mime_type: "image/png".into(),
            provider_asset_id: None,
            revised_prompt: None,
        }
    } else {
        provider
            .generate_image(
                &GenerateImageRequest {
                    prompt: canonical_reference_prompt(&request.character.prompt),
                    model: request.generation.image_model.clone(),
                    aspect_ratio: "1:1".into(),
                    resolution: "1k".into(),
                },
                &reference_path,
            )
            .map_err(provider_error)?
    };
    let reference_sha256 = validate_provider_image(&provider_root, &reference_media)?;
    store.update_record(job_id, |record| {
        record.artifacts.push(JobArtifactRecord {
            kind: "provider_reference".into(),
            path: reference_media.path.clone(),
            sha256: Some(reference_sha256.clone()),
        });
    })?;
    step(store, job_id, "provider:reference", "succeeded", 0.1, None)?;

    let workflow = [
        (
            "idle",
            8.0,
            "facing down in one relaxed neutral pose",
            "facing down, calm breathing idle loop",
        ),
        (
            "walk_up",
            12.0,
            "facing directly up in one neutral mid-stride keyframe",
            "facing up, walking in place loop",
        ),
        (
            "walk_right",
            12.0,
            "facing directly right in one neutral mid-stride keyframe",
            "facing right, walking in place loop",
        ),
        (
            "walk_down",
            12.0,
            "facing directly down in one neutral mid-stride keyframe",
            "facing down, walking in place loop",
        ),
    ];
    let mut attempts = HashMap::<String, u8>::new();
    let mut generated = BTreeMap::<String, GeneratedMediaRecord>::new();
    let retry_animations = request
        .retry_animations
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let local_only_retry = request.retry_stages.values().any(|stage| {
        matches!(
            stage,
            CharacterRetryStage::Loop
                | CharacterRetryStage::Matting
                | CharacterRetryStage::Consistency
        )
    });
    for (index, (name, _, still_pose, video_action)) in workflow.iter().enumerate() {
        let media = if let (Some(source_job_dir), Some(manifest)) =
            (&request.reuse_from_job_dir, &reused_manifest)
        {
            let source_media = manifest.animations.get(*name).ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "retry source provider manifest has no animation {name}"
                ))
            })?;
            if !retry_animations.contains(*name) {
                let reused = copy_reused_character_media(
                    source_job_dir,
                    &provider_root,
                    name,
                    source_media,
                )?;
                record_reused_character_media(store, job_id, name, &reused)?;
                attempts.insert((*name).into(), reused.attempt);
                reused
            } else {
                let stage = request
                    .retry_stages
                    .get(*name)
                    .copied()
                    .unwrap_or(CharacterRetryStage::Auto);
                match stage {
                    CharacterRetryStage::Auto | CharacterRetryStage::Still => {
                        generate_animation_with_retry(
                            store,
                            job_id,
                            request,
                            provider,
                            &reference_media.path,
                            name,
                            still_pose,
                            video_action,
                            &mut attempts,
                        )?
                    }
                    CharacterRetryStage::Video => {
                        let reused = copy_reused_character_media(
                            source_job_dir,
                            &provider_root,
                            name,
                            source_media,
                        )?;
                        record_reused_character_media(store, job_id, name, &reused)?;
                        let attempt = reused.video_attempt.saturating_add(1);
                        let repaired = repair_animation_video_once(
                            store,
                            job_id,
                            request,
                            provider,
                            &reused,
                            name,
                            video_action,
                            attempt,
                        )?;
                        attempts.insert((*name).into(), repaired.attempt);
                        repaired
                    }
                    CharacterRetryStage::Loop
                    | CharacterRetryStage::Matting
                    | CharacterRetryStage::Consistency => {
                        let mut reused = copy_reused_character_media(
                            source_job_dir,
                            &provider_root,
                            name,
                            source_media,
                        )?;
                        reused.retry_method = match stage {
                            CharacterRetryStage::Loop => "loop_reprocess",
                            CharacterRetryStage::Matting => "matting_reprocess",
                            CharacterRetryStage::Consistency => "consistency_reprocess",
                            _ => unreachable!(),
                        }
                        .into();
                        record_reused_character_media(store, job_id, name, &reused)?;
                        attempts.insert((*name).into(), reused.attempt);
                        reused
                    }
                    CharacterRetryStage::Frame => {
                        return Err(AutomationRunError::Processing(
                            "frame retry is only valid for topdown-keyframes@2.0.0".into(),
                        ));
                    }
                }
            }
        } else {
            generate_animation_with_retry(
                store,
                job_id,
                request,
                provider,
                &reference_media.path,
                name,
                still_pose,
                video_action,
                &mut attempts,
            )?
        };
        generated.insert((*name).to_string(), media);
        let progress = 0.1 + (index + 1) as f32 / workflow.len() as f32 * 0.3;
        step(
            store,
            job_id,
            &format!("provider:{name}"),
            "succeeded",
            progress,
            None,
        )?;
    }
    if let Some(style) = &style {
        let mut consistency = evaluate_character_consistency(
            &record.job_dir,
            style,
            &reference_media.path,
            &generated,
        )?;
        let retry_names = consistency
            .items
            .iter()
            .filter(|item| {
                item.verdict != ConsistencyVerdict::GameReady
                    && attempts.get(&item.id).copied().unwrap_or_default()
                        < request.generation.max_attempts_per_animation
            })
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        for name in retry_names {
            let (_, _, still_pose, video_action) = workflow
                .iter()
                .find(|(candidate, _, _, _)| *candidate == name)
                .ok_or_else(|| {
                    AutomationRunError::Processing(format!("unknown animation {name}"))
                })?;
            let media = generate_animation_once(
                store,
                job_id,
                request,
                provider,
                &reference_media.path,
                &name,
                still_pose,
                video_action,
                attempts.get(&name).copied().unwrap_or_default() + 1,
            )?;
            attempts.insert(name.clone(), media.attempt);
            generated.insert(name, media);
        }
        if !generated.is_empty() {
            consistency = evaluate_character_consistency(
                &record.job_dir,
                style,
                &reference_media.path,
                &generated,
            )?;
        }
        persist_consistency_artifacts(
            store,
            job_id,
            &consistency,
            &record.job_dir.join("normalized/character-stills"),
        )?;
        if consistency.verdict != ConsistencyVerdict::GameReady {
            write_provider_manifest(
                store,
                job_id,
                request,
                provider,
                &provider_usage_baseline,
                &reference_media,
                &reference_sha256,
                &generated,
            )?;
            if consistency.verdict == ConsistencyVerdict::AwaitingReview {
                let candidate_request = generated_pack_request(request, &generated, &workflow);
                let candidate = run_prepare_character_pack(store, job_id, &candidate_request)?;
                let candidate = attach_character_consistency(&candidate, Some(style))?;
                return mark_character_review_candidate(store, &candidate.job_id);
            }
            return store
                .update_record(job_id, |record| {
                    record.lifecycle_state = JobLifecycleState::AwaitingReview;
                    record.progress = 1.0;
                    record.worker_pid = None;
                    record.recoverable = true;
                    record.error_code = Some("consistency_review_required".into());
                    record.error_summary = Some(format!(
                        "character directions did not clear {CONSISTENCY_PROFILE}"
                    ));
                    record.next_actions = vec![
                        "job_report".into(),
                        "retry_direction".into(),
                        "review_job".into(),
                    ];
                })
                .map_err(Into::into);
        }
    }
    write_provider_manifest(
        store,
        job_id,
        request,
        provider,
        &provider_usage_baseline,
        &reference_media,
        &reference_sha256,
        &generated,
    )?;

    let mut pack_request = generated_pack_request(request, &generated, &workflow);
    let first = run_prepare_character_pack(store, job_id, &pack_request)?;
    if first.lifecycle_state != JobLifecycleState::AwaitingReview
        || request.generation.max_attempts_per_animation < 2
        || local_only_retry
    {
        return finalize_video_character_result(store, request, &first, style.as_ref());
    }

    let report: CharacterQualityReport = serde_json::from_slice(&fs::read(
        first.job_dir.join("animation-quality-report.json"),
    )?)?;
    let failing = report
        .animations
        .iter()
        .filter(|entry| entry.report.verdict != QualityVerdict::GameReady)
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    if failing.is_empty()
        || failing.iter().any(|name| {
            attempts.get(name).copied().unwrap_or_default()
                >= request.generation.max_attempts_per_animation
        })
    {
        return finalize_video_character_result(store, request, &first, style.as_ref());
    }

    reset_generated_pack_processing(store, job_id)?;
    for name in &failing {
        let (_, _, _, video_action) = workflow
            .iter()
            .find(|(candidate, _, _, _)| *candidate == name)
            .ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "quality report contains an unknown animation: {name}"
                ))
            })?;
        let source_media = generated.get(name).cloned().ok_or_else(|| {
            AutomationRunError::Processing(format!(
                "quality retry has no generated media for {name}"
            ))
        })?;
        let media = repair_animation_video_once(
            store,
            job_id,
            request,
            provider,
            &source_media,
            name,
            video_action,
            attempts.get(name).copied().unwrap_or_default() + 1,
        )?;
        attempts.insert(name.clone(), media.attempt);
        generated.insert(name.clone(), media);
        step(
            store,
            job_id,
            &format!("provider:{name}"),
            "succeeded",
            0.42,
            Some("quality retry completed".into()),
        )?;
    }
    write_provider_manifest(
        store,
        job_id,
        request,
        provider,
        &provider_usage_baseline,
        &reference_media,
        &reference_sha256,
        &generated,
    )?;
    if let Some(style) = &style {
        let consistency = evaluate_character_consistency(
            &record.job_dir,
            style,
            &reference_media.path,
            &generated,
        )?;
        persist_consistency_artifacts(
            store,
            job_id,
            &consistency,
            &record.job_dir.join("normalized/character-stills"),
        )?;
        if consistency.verdict != ConsistencyVerdict::GameReady {
            if consistency.verdict == ConsistencyVerdict::AwaitingReview {
                let candidate_request = generated_pack_request(request, &generated, &workflow);
                let candidate = run_prepare_character_pack(store, job_id, &candidate_request)?;
                let candidate = attach_character_consistency(&candidate, Some(style))?;
                return mark_character_review_candidate(store, &candidate.job_id);
            }
            return store
                .update_record(job_id, |record| {
                    record.lifecycle_state = JobLifecycleState::AwaitingReview;
                    record.progress = 1.0;
                    record.worker_pid = None;
                    record.recoverable = true;
                    record.error_code = Some("consistency_review_required".into());
                    record.next_actions = vec!["job_report".into(), "retry_direction".into()];
                })
                .map_err(Into::into);
        }
    }
    pack_request = generated_pack_request(request, &generated, &workflow);
    let completed = run_prepare_character_pack(store, job_id, &pack_request)?;
    finalize_video_character_result(store, request, &completed, style.as_ref())
}

fn finalize_video_character_result(
    store: &JobStore,
    request: &GenerateCharacterPackRequest,
    record: &JobRecord,
    style: Option<&crate::asset_project::StyleLockV1>,
) -> Result<JobRecord, AutomationRunError> {
    let attached = attach_character_consistency(record, style)?;
    if attached.lifecycle_state != JobLifecycleState::Succeeded {
        return Ok(attached);
    }
    let (Some(project_root), Some(asset_id)) =
        (request.project_path.as_ref(), request.asset_id.as_ref())
    else {
        return Ok(attached);
    };
    let Some(pack) = attached
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
    else {
        return Ok(attached);
    };
    let pack_sha256 = pack
        .sha256
        .clone()
        .unwrap_or_else(|| hash_directory(&pack.path).unwrap_or_default());
    let catalog_path = register_catalog_asset(
        project_root,
        ProjectCatalogEntryV1 {
            asset_id: asset_id.clone(),
            name: request.metadata.name.clone(),
            kind: "character".into(),
            pack_path: pack.path.clone(),
            pack_sha256,
            source_job_id: attached.job_id.clone(),
            parent_job_id: attached.parent_job_id.clone(),
            style: style.map(|style| CatalogStyleRefV1 {
                revision: style.revision.clone(),
            }),
            subject: None,
            workflow: format!("{}@{}", request.workflow.id, request.workflow.version),
            provider: Some(CatalogProviderRefV1 {
                provider_id: request.provider_id.clone(),
                profile_id: request.profile_id.clone(),
                model: request.generation.image_model.clone(),
            }),
            installed: None,
            created_at: Utc::now(),
        },
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let sha256 = hash_file(&catalog_path)?;
    store.update_record(&attached.job_id, |record| {
        record
            .artifacts
            .retain(|artifact| artifact.kind != "project_catalog");
        record.artifacts.push(JobArtifactRecord {
            kind: "project_catalog".into(),
            path: catalog_path.clone(),
            sha256: Some(sha256.clone()),
        });
    })?;
    store.read_record(&attached.job_id).map_err(Into::into)
}

fn evaluate_character_consistency(
    job_dir: &Path,
    style: &crate::asset_project::StyleLockV1,
    reference_path: &Path,
    generated: &BTreeMap<String, GeneratedMediaRecord>,
) -> Result<ConsistencyReportV1, AutomationRunError> {
    let normalized_root = job_dir.join("normalized/character-stills");
    fs::create_dir_all(&normalized_root)?;
    let normalized_reference = normalize_static_image(
        reference_path,
        &job_dir.join("normalized/character-reference.png"),
        style.character_canvas_size,
        true,
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let edge_reference = image_signature(&normalized_reference);
    let mut character_style = style.clone();
    character_style.baseline.palette = edge_reference.palette.clone();
    let mut reports = Vec::new();
    for (name, media) in generated {
        let normalized = normalize_static_image(
            &media.still_path,
            &normalized_root.join(format!("{name}.png")),
            style.character_canvas_size,
            true,
        )
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        reports.push(assess_consistency(
            name,
            media.attempt,
            &image_signature(&normalized),
            &character_style,
            None,
            Some(&edge_reference),
            style.character_canvas_size,
        ));
    }
    let verdict = aggregate_consistency_verdict(&reports, reports.len() == generated.len());
    Ok(ConsistencyReportV1 {
        schema_version: "1".into(),
        profile: CONSISTENCY_PROFILE.into(),
        asset_type: "character".into(),
        style_revision: style.revision.clone(),
        style_baseline_profile: Some(style.baseline_profile.clone()),
        verdict,
        items: reports,
    })
}

fn persist_consistency_artifacts(
    store: &JobStore,
    job_id: &str,
    report: &ConsistencyReportV1,
    images_dir: &Path,
) -> Result<(), AutomationRunError> {
    let record = store.read_record(job_id)?;
    let report_path = record.job_dir.join("consistency-report.json");
    write_json_atomic(&report_path, report)?;
    let mut images = fs::read_dir(images_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("png"))
        .collect::<Vec<_>>();
    images.sort();
    let contact_path = record.job_dir.join("contact-sheet.png");
    if !images.is_empty() {
        write_contact_sheet(&images, &contact_path, 256)
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    }
    store.update_record(job_id, |record| {
        record.artifacts.retain(|artifact| {
            artifact.kind != "consistency_report" && artifact.kind != "contact_sheet"
        });
        record.artifacts.push(JobArtifactRecord {
            kind: "consistency_report".into(),
            path: report_path.clone(),
            sha256: hash_asset_file(&report_path).ok(),
        });
        if contact_path.is_file() {
            record.artifacts.push(JobArtifactRecord {
                kind: "contact_sheet".into(),
                path: contact_path.clone(),
                sha256: hash_asset_file(&contact_path).ok(),
            });
        }
    })?;
    Ok(())
}

fn attach_character_consistency(
    record: &JobRecord,
    style: Option<&crate::asset_project::StyleLockV1>,
) -> Result<JobRecord, AutomationRunError> {
    let Some(style) = style else {
        return Ok(record.clone());
    };
    let Some(pack) = record
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .map(|artifact| artifact.path.clone())
    else {
        return Ok(record.clone());
    };
    let report_source = record.job_dir.join("consistency-report.json");
    if !report_source.is_file() {
        return Ok(record.clone());
    }
    fs::copy(&report_source, pack.join("consistency-report.json"))?;
    let forgepack_path = pack.join("forgepack.json");
    let mut forgepack: serde_json::Value = serde_json::from_slice(&fs::read(&forgepack_path)?)?;
    forgepack["schemaVersion"] = serde_json::json!("2.0.0");
    forgepack["assetType"] = serde_json::json!("character");
    forgepack["assets"]["consistencyReport"] = serde_json::json!("consistency-report.json");
    forgepack["source"]["metadata"]["styleRevision"] = serde_json::json!(style.revision);
    forgepack["source"]["metadata"]["styleBaselineProfile"] =
        serde_json::json!(style.baseline_profile);
    forgepack["source"]["metadata"]["consistencyProfile"] = serde_json::json!(CONSISTENCY_PROFILE);
    fs::write(&forgepack_path, serde_json::to_vec_pretty(&forgepack)?)?;
    let manifest_path = pack.join("assets/manifest.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&fs::read(&manifest_path)?)?;
    manifest["assetType"] = serde_json::json!("character");
    fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest)?)?;
    forge_pack::validate_pack_layout(&pack)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    Ok(record.clone())
}

fn mark_character_review_candidate(
    store: &JobStore,
    job_id: &str,
) -> Result<JobRecord, AutomationRunError> {
    store
        .update_record(job_id, |record| {
            for artifact in &mut record.artifacts {
                if artifact.kind == "gsfpack" {
                    artifact.kind = "candidate_gsfpack".into();
                }
            }
            record.lifecycle_state = JobLifecycleState::AwaitingReview;
            record.progress = 1.0;
            record.worker_pid = None;
            record.recoverable = true;
            record.error_code = Some("consistency_review_required".into());
            record.error_summary = Some(
                "character candidate is complete but requires explicit consistency review".into(),
            );
            record.next_actions = vec!["job_report".into(), "review_job".into()];
        })
        .map_err(Into::into)
}

#[allow(clippy::too_many_arguments)]
fn generate_animation_with_retry(
    store: &JobStore,
    job_id: &str,
    request: &GenerateCharacterPackRequest,
    provider: &dyn MediaGenerationProvider,
    reference_path: &Path,
    name: &str,
    still_pose: &str,
    video_action: &str,
    attempts: &mut HashMap<String, u8>,
) -> Result<GeneratedMediaRecord, AutomationRunError> {
    let mut last_error = None;
    for attempt in 1..=request.generation.max_attempts_per_animation {
        attempts.insert(name.to_string(), attempt);
        match generate_animation_once(
            store,
            job_id,
            request,
            provider,
            reference_path,
            name,
            still_pose,
            video_action,
            attempt,
        ) {
            Ok(media) => return Ok(media),
            Err(AutomationRunError::Cancelled) => return Err(AutomationRunError::Cancelled),
            Err(error) => {
                let message = error.to_string();
                step(
                    store,
                    job_id,
                    &format!("provider:{name}"),
                    "running",
                    0.1,
                    Some(format!(
                        "attempt {attempt} failed; retrying if allowed: {message}"
                    )),
                )?;
                last_error = Some(error);
            }
        }
    }
    Err(last_error.unwrap_or_else(|| {
        AutomationRunError::Processing(format!("provider did not generate animation {name}"))
    }))
}

#[allow(clippy::too_many_arguments)]
fn generate_animation_once(
    store: &JobStore,
    job_id: &str,
    request: &GenerateCharacterPackRequest,
    provider: &dyn MediaGenerationProvider,
    reference_path: &Path,
    name: &str,
    still_pose: &str,
    video_action: &str,
    attempt: u8,
) -> Result<GeneratedMediaRecord, AutomationRunError> {
    check_cancelled(store, job_id)?;
    step(
        store,
        job_id,
        &format!("provider:{name}"),
        "running",
        0.1,
        Some(format!("generation attempt {attempt}")),
    )?;
    let record = store.read_record(job_id)?;
    let provider_root = record.job_dir.join("source/provider");
    let attempt_root = provider_root.join(name).join(format!("attempt-{attempt}"));
    fs::create_dir_all(&attempt_root)?;
    let mut still_references = vec![ProviderImageReference::from_path(
        ReferenceRole::SubjectIdentity,
        reference_path.to_path_buf(),
    )
    .map_err(provider_error)?];
    if let Some(path) = &request.style_lock_path {
        let style = read_style_lock(path)
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        still_references.push(
            ProviderImageReference::from_path(ReferenceRole::Style, style.board_path)
                .map_err(provider_error)?,
        );
    }
    let still = provider
        .edit_image(
            &EditImageRequest {
                prompt: direction_still_prompt(&request.character.prompt, still_pose, attempt),
                model: request.generation.image_model.clone(),
                references: still_references,
                aspect_ratio: "1:1".into(),
                resolution: "1k".into(),
            },
            &attempt_root.join("direction.png"),
        )
        .map_err(provider_error)?;
    let still_sha256 = validate_provider_image(&provider_root, &still)?;
    let ticket = provider
        .generate_video(&GenerateVideoRequest {
            prompt: animation_video_prompt(video_action),
            model: request.generation.video_model.clone(),
            mode: VideoGenerationMode::ImageToVideo {
                image: still.path.clone(),
            },
            duration_seconds: request.generation.video_duration_seconds,
            aspect_ratio: "1:1".into(),
            resolution: "720p".into(),
        })
        .map_err(provider_error)?;
    let video = poll_generated_video(
        store,
        job_id,
        provider,
        &ticket,
        &attempt_root.join("animation.mp4"),
    )?;
    let video_sha256 = validate_provider_video(&provider_root, &video)?;
    let media = GeneratedMediaRecord {
        animation: name.into(),
        attempt,
        still_attempt: attempt,
        video_attempt: attempt,
        retry_method: default_retry_method(),
        still_path: still.path.clone(),
        still_sha256: still_sha256.clone(),
        video_path: video.path.clone(),
        video_sha256: video_sha256.clone(),
        still_asset_id: still.provider_asset_id.clone(),
        video_asset_id: video.provider_asset_id.clone(),
    };
    store.update_record(job_id, |record| {
        record.artifacts.push(JobArtifactRecord {
            kind: format!("provider_still_{name}_attempt_{attempt}"),
            path: still.path.clone(),
            sha256: Some(still_sha256.clone()),
        });
        record.artifacts.push(JobArtifactRecord {
            kind: format!("provider_video_{name}_attempt_{attempt}"),
            path: video.path.clone(),
            sha256: Some(video_sha256.clone()),
        });
    })?;
    Ok(media)
}

#[allow(clippy::too_many_arguments)]
fn repair_animation_video_once(
    store: &JobStore,
    job_id: &str,
    request: &GenerateCharacterPackRequest,
    provider: &dyn MediaGenerationProvider,
    source: &GeneratedMediaRecord,
    name: &str,
    action: &str,
    attempt: u8,
) -> Result<GeneratedMediaRecord, AutomationRunError> {
    check_cancelled(store, job_id)?;
    step(
        store,
        job_id,
        &format!("provider:{name}"),
        "running",
        0.4,
        Some(format!("video-only quality repair attempt {attempt}")),
    )?;
    let record = store.read_record(job_id)?;
    let provider_root = record.job_dir.join("source/provider");
    let attempt_root = provider_root.join(name).join(format!("attempt-{attempt}"));
    fs::create_dir_all(&attempt_root)?;
    let output_path = attempt_root.join("animation.mp4");
    let edit_prompt = animation_video_repair_prompt(action);
    let (ticket, retry_method) = if provider
        .capabilities()
        .contains(&ProviderCapability::EditVideo)
    {
        match provider.edit_video(&EditVideoRequest {
            prompt: edit_prompt,
            model: request.generation.video_model.clone(),
            video: ProviderInputRef {
                path: source.video_path.clone(),
                sha256: source.video_sha256.clone(),
                provider_asset_id: source.video_asset_id.clone(),
            },
        }) {
            Ok(ticket) => (ticket, "video_edit".to_string()),
            Err(crate::provider::ProviderError::Unavailable(message)) => {
                step(
                    store,
                    job_id,
                    &format!("provider:{name}"),
                    "running",
                    0.4,
                    Some(format!(
                        "video edit unavailable; using recorded image-to-video fallback: {message}"
                    )),
                )?;
                (
                    provider
                        .generate_video(&GenerateVideoRequest {
                            prompt: animation_video_prompt(action),
                            model: request.generation.video_model.clone(),
                            mode: VideoGenerationMode::ImageToVideo {
                                image: source.still_path.clone(),
                            },
                            duration_seconds: request.generation.video_duration_seconds,
                            aspect_ratio: "1:1".into(),
                            resolution: "720p".into(),
                        })
                        .map_err(provider_error)?,
                    "image_to_video_fallback".to_string(),
                )
            }
            Err(error) => return Err(provider_error(error)),
        }
    } else {
        (
            provider
                .generate_video(&GenerateVideoRequest {
                    prompt: animation_video_prompt(action),
                    model: request.generation.video_model.clone(),
                    mode: VideoGenerationMode::ImageToVideo {
                        image: source.still_path.clone(),
                    },
                    duration_seconds: request.generation.video_duration_seconds,
                    aspect_ratio: "1:1".into(),
                    resolution: "720p".into(),
                })
                .map_err(provider_error)?,
            "image_to_video_fallback".to_string(),
        )
    };
    let video = poll_generated_video(store, job_id, provider, &ticket, &output_path)?;
    let video_sha256 = validate_provider_video(&provider_root, &video)?;
    let media = GeneratedMediaRecord {
        animation: name.into(),
        attempt,
        still_attempt: source.still_attempt.max(source.attempt),
        video_attempt: attempt,
        retry_method: retry_method.clone(),
        still_path: source.still_path.clone(),
        still_sha256: source.still_sha256.clone(),
        video_path: video.path.clone(),
        video_sha256: video_sha256.clone(),
        still_asset_id: source.still_asset_id.clone(),
        video_asset_id: video.provider_asset_id.clone(),
    };
    store.update_record(job_id, |record| {
        record.artifacts.extend([
            JobArtifactRecord {
                kind: format!("provider_still_{name}_attempt_{attempt}"),
                path: source.still_path.clone(),
                sha256: Some(source.still_sha256.clone()),
            },
            JobArtifactRecord {
                kind: format!("provider_video_{name}_attempt_{attempt}"),
                path: video.path.clone(),
                sha256: Some(video_sha256.clone()),
            },
        ]);
    })?;
    Ok(media)
}

fn poll_generated_video(
    store: &JobStore,
    job_id: &str,
    provider: &dyn MediaGenerationProvider,
    ticket: &ProviderTicket,
    output_path: &Path,
) -> Result<ProviderMedia, AutomationRunError> {
    let started = Instant::now();
    loop {
        if check_cancelled(store, job_id).is_err() {
            let _ = provider.cancel(ticket);
            return Err(AutomationRunError::Cancelled);
        }
        if started.elapsed() > Duration::from_secs(15 * 60) {
            let _ = provider.cancel(ticket);
            return Err(AutomationRunError::Processing(
                "provider video generation timed out after 15 minutes".into(),
            ));
        }
        match provider.poll(ticket, output_path).map_err(provider_error)? {
            ProviderPoll::Pending { progress } => {
                if let Some(progress) = progress {
                    store.update_record(job_id, |record| {
                        record.error_summary = None;
                        if let Some(step) = record.steps.iter_mut().find(|step| {
                            step.name.starts_with("provider:") && step.state == "running"
                        }) {
                            step.message = Some(format!("provider progress: {progress}%"));
                        }
                    })?;
                }
                thread::sleep(Duration::from_secs(2));
            }
            ProviderPoll::Succeeded(media) => return Ok(media),
            ProviderPoll::Failed { code, message } => {
                return Err(AutomationRunError::Processing(format!(
                    "provider video failed ({code}): {message}"
                )))
            }
        }
    }
}

fn generated_pack_request(
    request: &GenerateCharacterPackRequest,
    generated: &BTreeMap<String, GeneratedMediaRecord>,
    workflow: &[(&str, f32, &str, &str)],
) -> PrepareCharacterPackRequest {
    PrepareCharacterPackRequest {
        schema_version: "2".into(),
        metadata: request.metadata.clone(),
        workflow: request.workflow.clone(),
        animations: workflow
            .iter()
            .map(|(name, fps, _, _)| CharacterAnimationRecipe {
                name: (*name).into(),
                input: AssetInput::VideoClip {
                    path: generated[*name].video_path.clone(),
                    start_time_ms: 0,
                    end_time_ms: None,
                    target_frame_count: request.generation.target_frame_count,
                },
                fps: *fps,
                loop_animation: true,
                matting: MattingRecipe::AutoCorners {
                    parameters: ChromaParameters::default(),
                },
            })
            .collect(),
        normalize: request.normalize,
        sheet: request.sheet,
        quality: request.quality.clone(),
    }
}

fn reset_generated_pack_processing(
    store: &JobStore,
    job_id: &str,
) -> Result<(), AutomationRunError> {
    let record = store.read_record(job_id)?;
    for path in [
        record.job_dir.join("animations"),
        record.job_dir.join("processed"),
    ] {
        match fs::remove_dir_all(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    for path in [
        record.job_dir.join("animation-quality-report.json"),
        record.job_dir.join("normalized-frames.json"),
    ] {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    store.update_record(job_id, |record| {
        record.lifecycle_state = JobLifecycleState::Running;
        record.state = JobState::Created;
        record.progress = 0.4;
        record.worker_pid = Some(std::process::id());
        record.recoverable = false;
        record.artifacts.retain(|artifact| {
            artifact.kind.starts_with("provider_") || artifact.kind == "provider_manifest"
        });
        record.next_actions = vec!["poll_job".into(), "cancel_job".into()];
        for step in &mut record.steps {
            if !step.name.starts_with("provider:") {
                step.state = "pending".into();
                step.message = None;
            }
        }
    })?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn write_provider_manifest(
    store: &JobStore,
    job_id: &str,
    request: &GenerateCharacterPackRequest,
    provider: &dyn MediaGenerationProvider,
    provider_usage_baseline: &ProviderUsage,
    reference: &ProviderMedia,
    reference_sha256: &str,
    generated: &BTreeMap<String, GeneratedMediaRecord>,
) -> Result<(), AutomationRunError> {
    let record = store.read_record(job_id)?;
    let path = record.job_dir.join("source/provider-manifest.json");
    let value = serde_json::json!({
        "schemaVersion": "1",
        "providerId": request.provider_id,
        "profileId": request.profile_id,
        "workflow": format!("{}@{}", request.workflow.id, request.workflow.version),
        "imageModel": request.generation.image_model,
        "videoModel": request.generation.video_model,
        "reference": {
            "path": reference.path,
            "sha256": reference_sha256,
            "assetId": reference.provider_asset_id,
        },
        "animations": generated,
        "retrySourceJob": request.reuse_from_job_dir.as_ref().and_then(|path| path.file_name()),
        "retryAnimations": request.retry_animations,
        "retryStages": request.retry_stages,
        "usage": provider_usage_delta(&provider.usage(), provider_usage_baseline),
    });
    write_json_atomic(&path, &value)?;
    let sha256 = hash_file(&path)?;
    store.update_record(job_id, |record| {
        record
            .artifacts
            .retain(|artifact| artifact.kind != "provider_manifest");
        record.artifacts.push(JobArtifactRecord {
            kind: "provider_manifest".into(),
            path: path.clone(),
            sha256: Some(sha256.clone()),
        });
    })?;
    Ok(())
}

fn write_character_stage_manifest(
    store: &JobStore,
    job_id: &str,
    request: &PrepareCharacterPackRequest,
    pack_path: Option<&Path>,
) -> Result<PathBuf, AutomationRunError> {
    let record = store.read_record(job_id)?;
    let provider_manifest = fs::read(record.job_dir.join("source/provider-manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let mut stages = Vec::<serde_json::Value>::new();
    if let Some(reference) = provider_manifest
        .as_ref()
        .and_then(|value| value.get("reference"))
    {
        let provider_request = record
            .recipe
            .as_ref()
            .and_then(|recipe| recipe.pointer("/request/reuseFromJobDir"))
            .is_none();
        stages.push(serde_json::json!({
            "stage": "subject_reference",
            "implementationVersion": "provider-media@1.0.0",
            "inputSha256": [record.recipe_hash],
            "outputSha256": reference.get("sha256").into_iter().collect::<Vec<_>>(),
            "invalidates": ["direction_still", "animation_video", "candidate_extract", "matting", "provisional_align", "loop_select", "shared_normalize", "quality", "pack", "godot_install"],
            "providerRequest": provider_request,
        }));
    }
    for animation in &request.animations {
        let provider_animation = provider_manifest
            .as_ref()
            .and_then(|value| value.get("animations"))
            .and_then(|value| value.get(&animation.name));
        if let Some(media) = provider_animation {
            let still_reused = media
                .get("stillPath")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| path.contains("/reused/"));
            let video_reused = media
                .get("videoPath")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|path| path.contains("/reused/"));
            stages.push(serde_json::json!({
                "stage": "direction_still",
                "item": animation.name,
                "implementationVersion": "provider-image-edit@1.0.0",
                "inputSha256": provider_manifest.as_ref().and_then(|value| value.pointer("/reference/sha256")).into_iter().collect::<Vec<_>>(),
                "outputSha256": media.get("stillSha256").into_iter().collect::<Vec<_>>(),
                "invalidates": ["animation_video", "candidate_extract", "matting", "provisional_align", "loop_select", "shared_normalize", "quality", "pack", "godot_install"],
                "providerRequest": !still_reused,
                "attempt": media.get("stillAttempt"),
            }));
            stages.push(serde_json::json!({
                "stage": "animation_video",
                "item": animation.name,
                "implementationVersion": "provider-video@1.0.0",
                "inputSha256": media.get("stillSha256").into_iter().collect::<Vec<_>>(),
                "outputSha256": media.get("videoSha256").into_iter().collect::<Vec<_>>(),
                "invalidates": ["candidate_extract", "matting", "provisional_align", "loop_select", "shared_normalize", "quality", "pack", "godot_install"],
                "providerRequest": !video_reused,
                "attempt": media.get("videoAttempt"),
                "retryMethod": media.get("retryMethod"),
            }));
        }
        let workspace = record.job_dir.join("animations").join(&animation.name);
        let candidates_hash = stage_path_hash(&workspace.join("candidates"))?;
        let matted_hash = stage_path_hash(&workspace.join("processed/matted"))?;
        let provisional_hash = stage_path_hash(&workspace.join("processed/provisional-aligned"))?;
        let selected_hash = stage_path_hash(&workspace.join("processed/loop-selected"))?;
        let loop_report_hash = stage_path_hash(&workspace.join("loop-selection-report.json"))?;
        let loop_output_hashes = [selected_hash, loop_report_hash]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let source_hash = match &animation.input {
            AssetInput::VideoClip { path, .. } => stage_path_hash(path)?,
            _ => None,
        };
        stages.extend([
            serde_json::json!({
                "stage": "candidate_extract", "item": animation.name,
                "implementationVersion": "ffmpeg-candidates@1.0.0",
                "inputSha256": source_hash.into_iter().collect::<Vec<_>>(),
                "outputSha256": candidates_hash.clone().into_iter().collect::<Vec<_>>(),
                "invalidates": ["matting", "provisional_align", "loop_select", "shared_normalize", "quality", "pack", "godot_install"],
                "providerRequest": false,
            }),
            serde_json::json!({
                "stage": "matting", "item": animation.name,
                "implementationVersion": "matting@1.0.0",
                "inputSha256": candidates_hash.into_iter().collect::<Vec<_>>(),
                "outputSha256": matted_hash.clone().into_iter().collect::<Vec<_>>(),
                "invalidates": ["provisional_align", "loop_select", "shared_normalize", "quality", "pack", "godot_install"],
                "providerRequest": false,
            }),
            serde_json::json!({
                "stage": "provisional_align", "item": animation.name,
                "implementationVersion": "normalize@1.0.0",
                "inputSha256": matted_hash.into_iter().collect::<Vec<_>>(),
                "outputSha256": provisional_hash.clone().into_iter().collect::<Vec<_>>(),
                "invalidates": ["loop_select", "shared_normalize", "quality", "pack", "godot_install"],
                "providerRequest": false,
            }),
            serde_json::json!({
                "stage": "loop_select", "item": animation.name,
                "implementationVersion": LOOP_SELECTION_PROFILE,
                "inputSha256": provisional_hash.into_iter().collect::<Vec<_>>(),
                "outputSha256": loop_output_hashes,
                "invalidates": ["shared_normalize", "quality", "pack", "godot_install"],
                "providerRequest": false,
            }),
        ]);
    }
    let normalized_hash = stage_path_hash(&record.job_dir.join("processed/normalized"))?;
    let quality_hash = stage_path_hash(&record.job_dir.join("animation-quality-report.json"))?;
    stages.push(serde_json::json!({
        "stage": "shared_normalize",
        "implementationVersion": "normalize@1.0.0",
        "inputSha256": request.animations.iter().filter_map(|animation| stage_path_hash(&record.job_dir.join("animations").join(&animation.name).join("processed/loop-selected")).ok().flatten()).collect::<Vec<_>>(),
        "outputSha256": normalized_hash.clone().into_iter().collect::<Vec<_>>(),
        "invalidates": ["quality", "pack", "godot_install"],
        "providerRequest": false,
    }));
    stages.push(serde_json::json!({
        "stage": "quality",
        "implementationVersion": "animation-quality@2.0.0",
        "inputSha256": normalized_hash.into_iter().collect::<Vec<_>>(),
        "outputSha256": quality_hash.clone().into_iter().collect::<Vec<_>>(),
        "invalidates": ["pack", "godot_install"],
        "providerRequest": false,
    }));
    if let Some(pack_path) = pack_path {
        stages.push(serde_json::json!({
            "stage": "pack",
            "implementationVersion": "gsfpack@2.0.0",
            "inputSha256": quality_hash.into_iter().collect::<Vec<_>>(),
            "outputSha256": stage_path_hash(pack_path)?.into_iter().collect::<Vec<_>>(),
            "invalidates": ["godot_install"],
            "providerRequest": false,
        }));
    }
    let path = record.job_dir.join("workflow-stage-manifest.json");
    let workflow = format!("{}@{}", request.workflow.id, request.workflow.version);
    let manifest = serde_json::json!({
        "schemaVersion": "1",
        "workflow": workflow,
        "stages": stages,
    });
    write_json_atomic(&path, &manifest)?;
    let sha256 = hash_file(&path)?;
    let graph_path = record.job_dir.join(WORKFLOW_GRAPH_FILE);
    let graph = workflow_graph_from_stage_manifest(&record, &workflow, &stages)?;
    write_workflow_graph(&graph_path, &graph)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let graph_sha256 = hash_file(&graph_path)?;
    store.update_record(job_id, |record| {
        record.artifacts.retain(|artifact| {
            !matches!(
                artifact.kind.as_str(),
                "workflow_stage_manifest" | "workflow_graph"
            )
        });
        record.artifacts.extend([
            JobArtifactRecord {
                kind: "workflow_stage_manifest".into(),
                path: path.clone(),
                sha256: Some(sha256.clone()),
            },
            JobArtifactRecord {
                kind: "workflow_graph".into(),
                path: graph_path.clone(),
                sha256: Some(graph_sha256.clone()),
            },
        ]);
    })?;
    Ok(path)
}

fn workflow_graph_from_stage_manifest(
    record: &JobRecord,
    workflow: &str,
    stages: &[serde_json::Value],
) -> Result<WorkflowGraphV1, AutomationRunError> {
    let mut nodes = Vec::<WorkflowNodeV1>::new();
    let mut last_by_item = BTreeMap::<String, String>::new();
    let mut last_shared = None::<String>;
    for stage in stages {
        let stage_name = stage
            .get("stage")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let item = stage
            .get("item")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let id = item
            .as_ref()
            .map(|item| format!("{stage_name}:{item}"))
            .unwrap_or_else(|| stage_name.to_string());
        let depends_on = if let Some(item) = &item {
            last_by_item
                .get(item)
                .cloned()
                .into_iter()
                .collect::<Vec<_>>()
        } else if stage_name == "shared_normalize" {
            last_by_item.values().cloned().collect()
        } else {
            last_shared.clone().into_iter().collect()
        };
        let input_sha256 = json_string_array(stage.get("inputSha256"));
        let output_sha256 = json_string_array(stage.get("outputSha256"));
        let implementation_version = stage
            .get("implementationVersion")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown@1.0.0")
            .to_string();
        let cache_key = compute_cache_key(
            stage_name,
            &implementation_version,
            None,
            None,
            stage,
            &input_sha256,
        )
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        let hash_path = |sha256: String| WorkflowArtifactV1 {
            path: record.job_dir.join(".forge-hash").join(&sha256),
            sha256,
        };
        nodes.push(WorkflowNodeV1 {
            id: id.clone(),
            stage: stage_name.into(),
            item: item.clone(),
            frame: None,
            implementation_version,
            depends_on,
            invalidates: stage
                .get("invalidates")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect(),
            inputs: input_sha256.into_iter().map(&hash_path).collect(),
            outputs: output_sha256.into_iter().map(hash_path).collect(),
            cache_key,
            provider_request: stage
                .get("providerRequest")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            cache_hit: false,
            provider_id: None,
            model: None,
        });
        if let Some(item) = item {
            last_by_item.insert(item, id);
        } else {
            last_shared = Some(id);
        }
    }
    Ok(WorkflowGraphV1 {
        schema_version: "1".into(),
        workflow: workflow.into(),
        job_id: record.job_id.clone(),
        parent_job_id: record.parent_job_id.clone(),
        nodes,
    })
}

fn json_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|value| value.len() == 64)
        .collect()
}

fn stage_path_hash(path: &Path) -> Result<Option<String>, AutomationRunError> {
    if path.is_file() {
        Ok(Some(hash_file(path)?))
    } else if path.is_dir() {
        Ok(Some(hash_directory(path)?))
    } else {
        Ok(None)
    }
}

fn validate_provider_image(
    provider_root: &Path,
    media: &ProviderMedia,
) -> Result<String, AutomationRunError> {
    validate_provider_local_file(provider_root, &media.path)?;
    let _ = image::open(&media.path)
        .map_err(|error| {
            AutomationRunError::Processing(format!("provider returned malformed image: {error}"))
        })?
        .to_rgba8();
    hash_file(&media.path).map_err(Into::into)
}

fn validate_provider_video(
    provider_root: &Path,
    media: &ProviderMedia,
) -> Result<String, AutomationRunError> {
    validate_provider_local_file(provider_root, &media.path)?;
    let probe = probe_video(&ProbeVideoParams {
        input_path: media.path.clone(),
        configured_ffprobe_path: None,
        bundled_resource_path: None,
    })
    .map_err(|error| {
        AutomationRunError::Processing(format!("provider returned malformed video: {error}"))
    })?;
    if probe.duration_seconds <= 0.0 {
        return Err(AutomationRunError::Processing(
            "provider video has zero duration".into(),
        ));
    }
    hash_file(&media.path).map_err(Into::into)
}

fn validate_provider_local_file(
    provider_root: &Path,
    path: &Path,
) -> Result<(), AutomationRunError> {
    if !path.is_file() {
        return Err(AutomationRunError::Processing(format!(
            "provider output was not materialized: {}",
            path.display()
        )));
    }
    let canonical_root = fs::canonicalize(provider_root)?;
    let canonical_path = fs::canonicalize(path)?;
    if !canonical_path.starts_with(canonical_root) {
        return Err(AutomationRunError::Processing(
            "provider output escaped the Forge job directory".into(),
        ));
    }
    let size = fs::metadata(&canonical_path)?.len();
    if size == 0 || size > 512 * 1024 * 1024 {
        return Err(AutomationRunError::Processing(format!(
            "provider output size is invalid: {size} bytes"
        )));
    }
    Ok(())
}

fn canonical_reference_prompt(character: &str) -> String {
    format!(
        "{character}. Single full-body top-down 2D game character, centered, fixed orthographic camera, consistent silhouette and costume, solid chroma green background, no shadow, no text, no UI, one character only."
    )
}

fn direction_still_prompt(character: &str, pose: &str, attempt: u8) -> String {
    let correction = if attempt > 1 {
        " The previous result was rejected. Return exactly one character once; do not repeat the character anywhere in the image."
    } else {
        ""
    };
    format!(
        "Preserve the exact identity, proportions, costume, palette, and camera of the identity reference. {character}. Pose: {pose}. This is one single animation keyframe, not an animation sequence. Full body centered, fixed top-down orthographic camera, solid chroma green background, no shadow, no text. Exactly one character, one pose, one view, one panel. No sprite sheet, turnaround, collage, contact sheet, duplicate, inset, alternate pose, or multiple frames.{correction}"
    )
}

fn animation_video_prompt(action: &str) -> String {
    format!(
        "Animate only the character: {action}. One seamless repeating game animation cycle. Keep identity, costume, scale, framing, camera, and solid chroma green background unchanged. Character stays centered with no scene translation, no camera motion, no text, no cuts."
    )
}

fn animation_video_repair_prompt(action: &str) -> String {
    format!(
        "Repair this existing game animation without redesigning it. Keep exactly one character, the same identity, costume, palette, top-down orthographic camera, scale, framing, and solid chroma green background. Preserve the action ({action}) but make it exactly one complete in-place cycle whose ending pose and motion flow seamlessly back to the starting pose. No camera motion, translation, cuts, text, extra subjects, new equipment, or effects."
    )
}

fn provider_error(error: crate::provider::ProviderError) -> AutomationRunError {
    match error {
        crate::provider::ProviderError::Cancelled => AutomationRunError::Cancelled,
        other => AutomationRunError::Provider(other),
    }
}

struct CharacterAnimationRun {
    recipe: CharacterAnimationRecipe,
    frame_paths: Vec<PathBuf>,
    quality_report: QualityReport,
    loop_selection: Option<LoopSelectionReport>,
}

struct ProcessedAnimationGroup {
    recipe: CharacterAnimationRecipe,
    frame_paths: Vec<PathBuf>,
    loop_selection: Option<LoopSelectionReport>,
}

#[derive(Debug, Clone, Copy)]
struct LoopCandidateTiming {
    sample_fps: f32,
    duration_ms: u64,
}

fn run_prepare_character_pack(
    store: &JobStore,
    job_id: &str,
    request: &PrepareCharacterPackRequest,
) -> Result<JobRecord, AutomationRunError> {
    let record = store.read_record(job_id)?;
    let animations_root = record.job_dir.join("animations");
    fs::create_dir_all(&animations_root)?;
    let mut processed_groups = Vec::with_capacity(request.animations.len());
    let total_phases = (request.animations.len() * 3 + 2) as f32;
    let mut completed_phases = 0f32;

    for animation in &request.animations {
        check_cancelled(store, job_id)?;
        let workspace = animations_root.join(&animation.name);
        fs::create_dir_all(&workspace)?;
        let ingest_step = format!("{}:ingest", animation.name);
        step(
            store,
            job_id,
            &ingest_step,
            "running",
            completed_phases / total_phases,
            None,
        )?;
        let (raw_frames, loop_timing) = if record.operation_kind
            == JobOperationKind::GenerateCharacterPack
            && animation.loop_animation
        {
            ingest_generated_loop_candidates(&workspace, &animation.input)?
        } else {
            (ingest_frames(&workspace, &animation.input)?, None)
        };
        if raw_frames.len() < 2 {
            return Err(AutomationRunError::Processing(format!(
                "animation {} requires at least two frames",
                animation.name
            )));
        }
        completed_phases += 1.0;
        step(
            store,
            job_id,
            &ingest_step,
            "succeeded",
            completed_phases / total_phases,
            None,
        )?;

        let matting_step = format!("{}:matting", animation.name);
        step(
            store,
            job_id,
            &matting_step,
            "running",
            completed_phases / total_phases,
            None,
        )?;
        let (processed, loop_selection) = if let Some(timing) = loop_timing {
            let target_frame_count = match &animation.input {
                AssetInput::VideoClip {
                    target_frame_count, ..
                } => *target_frame_count,
                _ => unreachable!("loop candidate ingestion is video-only"),
            };
            let selected = select_generated_animation_loop(
                &workspace,
                &animation.name,
                &raw_frames,
                &animation.matting,
                request.normalize,
                timing,
                target_frame_count,
            )?;
            (selected.0, Some(selected.1))
        } else {
            (
                apply_matting(&workspace, &raw_frames, &animation.matting)?,
                None,
            )
        };
        completed_phases += 1.0;
        step(
            store,
            job_id,
            &matting_step,
            "succeeded",
            completed_phases / total_phases,
            None,
        )?;
        processed_groups.push(ProcessedAnimationGroup {
            recipe: animation.clone(),
            frame_paths: processed,
            loop_selection,
        });
    }

    check_cancelled(store, job_id)?;
    step(
        store,
        job_id,
        "shared:normalize",
        "running",
        completed_phases / total_phases,
        None,
    )?;
    let all_processed = processed_groups
        .iter()
        .flat_map(|group| group.frame_paths.iter().cloned())
        .collect::<Vec<_>>();
    let (normalized_paths, bboxes, sizes, anchor, summaries) =
        normalize_and_save(&record.job_dir, &all_processed, request.normalize)?;
    fs::write(
        record.job_dir.join("normalized-frames.json"),
        serde_json::to_vec_pretty(&summaries)?,
    )?;
    completed_phases += 1.0;
    step(
        store,
        job_id,
        "shared:normalize",
        "succeeded",
        completed_phases / total_phases,
        None,
    )?;

    let mut runs = Vec::with_capacity(processed_groups.len());
    let mut offset = 0usize;
    for group in processed_groups {
        let animation = group.recipe;
        let processed = group.frame_paths;
        check_cancelled(store, job_id)?;
        let quality_step = format!("{}:quality", animation.name);
        step(
            store,
            job_id,
            &quality_step,
            "running",
            completed_phases / total_phases,
            None,
        )?;
        let end = offset + processed.len();
        let mut quality_report = compute_quality_report_for_animation(
            &bboxes[offset..end],
            &sizes[offset..end],
            animation.loop_animation,
        );
        if let Some(loop_selection) = &group.loop_selection {
            apply_loop_selection_quality(&mut quality_report, loop_selection);
        }
        runs.push(CharacterAnimationRun {
            recipe: animation,
            frame_paths: normalized_paths[offset..end].to_vec(),
            quality_report,
            loop_selection: group.loop_selection,
        });
        offset = end;
        completed_phases += 1.0;
        step(
            store,
            job_id,
            &quality_step,
            "succeeded",
            completed_phases / total_phases,
            None,
        )?;
    }

    let aggregate_quality = aggregate_character_quality(&runs);
    let character_quality = CharacterQualityReport {
        quality_profile: "animation-quality@2.0.0".into(),
        verdict: aggregate_quality.verdict,
        default_animation: request.metadata.default_animation.clone(),
        frame_count: normalized_paths.len(),
        animations: runs
            .iter()
            .map(|run| AnimationQualityEntry {
                name: run.recipe.name.clone(),
                report: run.quality_report.clone(),
                loop_selection_report: run
                    .loop_selection
                    .as_ref()
                    .map(|_| format!("animations/{}/loop-selection-report.json", run.recipe.name)),
                loop_selection: run.loop_selection.clone(),
            })
            .collect(),
    };
    let job_quality_path = record.job_dir.join("animation-quality-report.json");
    let job_loop_path = record.job_dir.join("loop-selection-report.json");
    fs::write(
        &job_quality_path,
        serde_json::to_vec_pretty(&character_quality)?,
    )?;
    fs::write(
        &job_loop_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "profile": LOOP_SELECTION_PROFILE,
            "animations": runs.iter().filter_map(|run| {
                run.loop_selection.as_ref().map(|report| serde_json::json!({
                    "name": run.recipe.name,
                    "report": report,
                }))
            }).collect::<Vec<_>>(),
        }))?,
    )?;
    write_character_stage_manifest(store, job_id, request, None)?;
    let repair_artifact =
        write_repair_comparison(&record, character_quality_snapshot(&character_quality))
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;

    if request.quality.require_game_ready
        && runs
            .iter()
            .any(|run| run.quality_report.verdict != QualityVerdict::GameReady)
    {
        return store
            .update_record(job_id, |record| {
                record.state = JobState::QualityChecked;
                record.lifecycle_state = JobLifecycleState::AwaitingReview;
                record.progress = 1.0;
                record.worker_pid = None;
                record.recoverable = true;
                record.artifacts.push(JobArtifactRecord {
                    kind: "animation_quality_report".into(),
                    path: job_quality_path.clone(),
                    sha256: None,
                });
                record.artifacts.push(JobArtifactRecord {
                    kind: "loop_selection_report".into(),
                    path: job_loop_path.clone(),
                    sha256: None,
                });
                if let Some(artifact) = repair_artifact.clone() {
                    record.artifacts.push(artifact);
                }
                record.next_actions = vec![
                    "analyze_repair".into(),
                    "plan_repair_job".into(),
                    "open_job".into(),
                ];
                if let Some(step) = record
                    .steps
                    .iter_mut()
                    .find(|step| step.name == "pack:export")
                {
                    step.state = "blocked".into();
                    step.message = Some("every animation must be game_ready".into());
                }
            })
            .map_err(Into::into);
    }

    check_cancelled(store, job_id)?;
    step(
        store,
        job_id,
        "pack:export",
        "running",
        completed_phases / total_phases,
        None,
    )?;
    let asset_id = record
        .asset_id
        .clone()
        .unwrap_or_else(|| job_id.to_string());
    let animation_names = runs
        .iter()
        .map(|run| run.recipe.name.clone())
        .collect::<Vec<_>>();
    let loop_provenance = runs
        .iter()
        .filter_map(|run| {
            run.loop_selection.as_ref().map(|loop_report| {
                serde_json::json!({
                    "animation": run.recipe.name,
                    "profile": loop_report.profile,
                    "selectedStartFrame": loop_report.selected_start_frame,
                    "selectedEndBoundaryFrame": loop_report.selected_end_boundary_frame,
                    "selectedDurationMs": loop_report.selected_duration_ms,
                    "outputFrameIndices": loop_report.output_frame_indices,
                })
            })
        })
        .collect::<Vec<_>>();
    let provider_retry_methods = fs::read(record.job_dir.join("source/provider-manifest.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
        .and_then(|value| value.get("animations").cloned())
        .and_then(|animations| animations.as_object().cloned())
        .map(|animations| {
            animations
                .into_iter()
                .filter_map(|(name, media)| {
                    media
                        .get("retryMethod")
                        .cloned()
                        .map(|method| (name, method))
                })
                .collect::<serde_json::Map<_, _>>()
        })
        .unwrap_or_default();
    let subject_provenance = record
        .recipe
        .as_ref()
        .and_then(|recipe| recipe.pointer("/request/subjectLockPath"))
        .and_then(serde_json::Value::as_str)
        .and_then(|path| read_subject_lock(Path::new(path)).ok())
        .map(|subject| {
            serde_json::json!({
                "id": subject.id,
                "revision": subject.revision,
                "canonicalSha256": subject.canonical_sha256,
                "profile": subject.profile,
            })
        });
    let style_provenance = record
        .recipe
        .as_ref()
        .and_then(|recipe| recipe.pointer("/request/styleLockPath"))
        .and_then(serde_json::Value::as_str)
        .and_then(|path| read_style_lock(Path::new(path)).ok())
        .map(|style| {
            serde_json::json!({
                "revision": style.revision,
                "boardSha256": style.board_sha256,
                "baselineProfile": style.baseline_profile,
            })
        });
    let output = export_character_pack(CharacterPackExportParams {
        exports_dir: record.job_dir.join("exports"),
        export_id: asset_id.clone(),
        animations: runs
            .into_iter()
            .map(|run| CharacterAnimationExport {
                name: run.recipe.name,
                frame_paths: run.frame_paths,
                fps: run.recipe.fps,
                loop_animation: run.recipe.loop_animation,
                quality_report: run.quality_report,
                loop_selection: run.loop_selection,
            })
            .collect(),
        sheet: request.sheet,
        metadata: CharacterPackMetadataParams {
            id: asset_id,
            name: request.metadata.name.clone(),
            version: "0.1.0".into(),
            creator_name: request.metadata.creator.clone(),
            license_type: request.metadata.license.clone(),
            source_kind: if record.operation_kind == JobOperationKind::GenerateCharacterPack {
                "provider_generation".into()
            } else {
                "import_frames".into()
            },
            source_name: Some(
                if record.operation_kind == JobOperationKind::GenerateCharacterPack {
                    "character_pack_v3"
                } else {
                    "character_pack_v2"
                }
                .into(),
            ),
            source_metadata: Some(serde_json::json!({
                "automationSchemaVersion": if record.operation_kind == JobOperationKind::GenerateCharacterPack { "3" } else { "2" },
                "profile": "godot-pixel-art@1.0.0",
                "characterWorkflow": {
                    "id": request.workflow.id,
                    "version": request.workflow.version,
                },
                "defaultAnimation": request.metadata.default_animation,
                "animations": animation_names,
                "inputFingerprint": record.input_hash,
                "recipeHash": record.recipe_hash,
                "provider": record.recipe.as_ref().and_then(|recipe| recipe.pointer("/request/providerId")),
                "providerProfile": record.recipe.as_ref().and_then(|recipe| recipe.pointer("/request/profileId")),
                "providerManifest": if record.operation_kind == JobOperationKind::GenerateCharacterPack {
                    Some(if request.workflow.id == "topdown-keyframes" {
                        "source/keyframe-provider-manifest.json"
                    } else {
                        "source/provider-manifest.json"
                    })
                } else { None },
                "subject": subject_provenance,
                "style": style_provenance,
                "loopSelectionProfile": LOOP_SELECTION_PROFILE,
                "loopSelection": loop_provenance,
                "providerRetryMethods": provider_retry_methods,
            })),
            default_animation: request.metadata.default_animation.clone(),
            anchor,
            quality_report: aggregate_quality,
        },
    })
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;

    write_character_stage_manifest(store, job_id, request, Some(&output.pack_dir))?;
    let pack_hash = hash_directory(&output.pack_dir)?;
    let mut artifacts = vec![
        JobArtifactRecord {
            kind: "gsfpack".into(),
            path: output.pack_dir,
            sha256: Some(pack_hash),
        },
        JobArtifactRecord {
            kind: "preview_gif".into(),
            path: output.preview_gif_path,
            sha256: None,
        },
        JobArtifactRecord {
            kind: "animation_quality_report".into(),
            path: output.animation_quality_report_path,
            sha256: None,
        },
        JobArtifactRecord {
            kind: "loop_selection_report".into(),
            path: output.loop_selection_report_path,
            sha256: None,
        },
    ];
    if let Some(artifact) = repair_artifact {
        artifacts.push(artifact);
    }
    for (name, path) in output.animation_preview_paths {
        artifacts.push(JobArtifactRecord {
            kind: format!("preview_{name}"),
            path,
            sha256: None,
        });
    }
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record.artifacts.extend(artifacts);
            record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
            if let Some(step) = record
                .steps
                .iter_mut()
                .find(|step| step.name == "pack:export")
            {
                step.state = "succeeded".into();
            }
        })
        .map_err(Into::into)
}

fn aggregate_character_quality(runs: &[CharacterAnimationRun]) -> QualityReport {
    let total_frames = runs.iter().map(|run| run.frame_paths.len()).sum::<usize>();
    let total_weight = total_frames.max(1) as f32;
    let mut metrics = QualityMetrics {
        bbox_bottom_drift_px: 0.0,
        bbox_center_x_drift_px: 0.0,
        bbox_center_y_drift_px: 0.0,
        bbox_width_variation_px: 0.0,
        alpha_coverage_avg: 0.0,
        loop_match_score: 1.0,
        frame_count: total_frames,
        frame_size_consistent: true,
        cell_boundary_safe: true,
    };
    let mut verdict = QualityVerdict::GameReady;
    let mut recommendations = Vec::new();
    let mut notes = Vec::new();
    for run in runs {
        let report = &run.quality_report;
        let current = report.metrics;
        metrics.bbox_bottom_drift_px = metrics
            .bbox_bottom_drift_px
            .max(current.bbox_bottom_drift_px);
        metrics.bbox_center_x_drift_px = metrics
            .bbox_center_x_drift_px
            .max(current.bbox_center_x_drift_px);
        metrics.bbox_center_y_drift_px = metrics
            .bbox_center_y_drift_px
            .max(current.bbox_center_y_drift_px);
        metrics.bbox_width_variation_px = metrics
            .bbox_width_variation_px
            .max(current.bbox_width_variation_px);
        metrics.alpha_coverage_avg +=
            current.alpha_coverage_avg * run.frame_paths.len() as f32 / total_weight;
        metrics.loop_match_score = metrics.loop_match_score.min(current.loop_match_score);
        metrics.frame_size_consistent &= current.frame_size_consistent;
        metrics.cell_boundary_safe &= current.cell_boundary_safe;
        if verdict_rank(report.verdict) > verdict_rank(verdict) {
            verdict = report.verdict;
        }
        for recommendation in &report.recommendations {
            if !recommendations.contains(recommendation) {
                recommendations.push(*recommendation);
            }
        }
        notes.extend(
            report
                .notes
                .iter()
                .map(|note| format!("{}:{note}", run.recipe.name)),
        );
    }
    QualityReport {
        verdict,
        metrics,
        recommendations,
        notes,
    }
}

fn verdict_rank(verdict: QualityVerdict) -> u8 {
    match verdict {
        QualityVerdict::GameReady => 0,
        QualityVerdict::NeedsCleanup => 1,
        QualityVerdict::PrototypeUsable => 2,
        QualityVerdict::Blocked => 3,
    }
}

fn run_prepare_asset(
    store: &JobStore,
    job_id: &str,
    request: &PrepareAssetRequest,
) -> Result<JobRecord, AutomationRunError> {
    check_cancelled(store, job_id)?;
    step(store, job_id, "ingest", "running", 0.08, None)?;
    let record = store.read_record(job_id)?;

    if let AssetInput::Gsfpack { path } = &request.input {
        forge_pack::validate_pack_layout(path)
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        let target = record
            .job_dir
            .join("exports")
            .join(path.file_name().unwrap_or_default());
        copy_directory(path, &target)?;
        let artifact = JobArtifactRecord {
            kind: "gsfpack".into(),
            sha256: Some(hash_directory(&target)?),
            path: target,
        };
        return store
            .update_record(job_id, |record| {
                record.state = JobState::Exported;
                record.lifecycle_state = JobLifecycleState::Succeeded;
                record.progress = 1.0;
                record.worker_pid = None;
                record
                    .steps
                    .iter_mut()
                    .for_each(|step| step.state = "succeeded".into());
                record.artifacts.push(artifact);
                record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
            })
            .map_err(Into::into);
    }

    let raw_frames = ingest_frames(&record.job_dir, &request.input)?;
    if raw_frames.len() < 2 {
        return Err(AutomationRunError::Processing(
            "at least two frames are required".into(),
        ));
    }
    step(store, job_id, "ingest", "succeeded", 0.2, None)?;
    check_cancelled(store, job_id)?;

    step(store, job_id, "matting", "running", 0.25, None)?;
    let processed_frames = apply_matting(&record.job_dir, &raw_frames, &request.matting)?;
    step(store, job_id, "matting", "succeeded", 0.4, None)?;
    check_cancelled(store, job_id)?;

    step(store, job_id, "normalize", "running", 0.45, None)?;
    let (normalized_paths, bboxes, sizes, anchor, summaries) =
        normalize_and_save(&record.job_dir, &processed_frames, request.normalize)?;
    step(store, job_id, "normalize", "succeeded", 0.6, None)?;

    step(store, job_id, "quality", "running", 0.65, None)?;
    let quality_report = compute_quality_report(&bboxes, &sizes);
    fs::write(
        record.job_dir.join("quality-report.json"),
        serde_json::to_vec_pretty(&quality_report)?,
    )?;
    fs::write(
        record.job_dir.join("normalized-frames.json"),
        serde_json::to_vec_pretty(&summaries)?,
    )?;
    step(store, job_id, "quality", "succeeded", 0.72, None)?;
    let repair_artifact = write_repair_comparison(
        &record,
        single_quality_snapshot(&request.metadata.animation, quality_report.clone()),
    )
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;

    if request.quality.require_game_ready && quality_report.verdict != QualityVerdict::GameReady {
        return store
            .update_record(job_id, |record| {
                record.state = JobState::QualityChecked;
                record.lifecycle_state = JobLifecycleState::AwaitingReview;
                record.progress = 1.0;
                record.worker_pid = None;
                record.recoverable = true;
                record.artifacts.push(JobArtifactRecord {
                    kind: "quality_report".into(),
                    path: record.job_dir.join("quality-report.json"),
                    sha256: None,
                });
                if let Some(artifact) = repair_artifact.clone() {
                    record.artifacts.push(artifact);
                }
                record.next_actions = vec![
                    "analyze_repair".into(),
                    "plan_repair_job".into(),
                    "open_job".into(),
                ];
                if let Some(export_step) =
                    record.steps.iter_mut().find(|step| step.name == "export")
                {
                    export_step.state = "blocked".into();
                    export_step.message = Some(format!(
                        "quality verdict {:?}; GameReady is required",
                        quality_report.verdict
                    ));
                }
            })
            .map_err(Into::into);
    }

    check_cancelled(store, job_id)?;
    step(store, job_id, "export", "running", 0.78, None)?;
    let asset_id = record
        .asset_id
        .clone()
        .unwrap_or_else(|| job_id.to_string());
    let source_kind = source_kind_for_input(&request.input).as_str().to_string();
    let source_name = input_display_name(&request.input);
    let export = export_pack(ExportPackParams {
        exports_dir: record.job_dir.join("exports"),
        export_id: asset_id.clone(),
        frame_paths: normalized_paths,
        sheet: request.sheet,
        gif: PreviewGifParameters {
            fps: request.metadata.fps,
            loop_animation: request.metadata.loop_animation,
            background: GifBackground::Transparent,
        },
        metadata: PackMetadataParams {
            id: asset_id,
            name: request.metadata.name.clone(),
            version: "0.1.0".into(),
            creator_name: request.metadata.creator.clone(),
            license_type: request.metadata.license.clone(),
            source_kind,
            source_name,
            source_metadata: Some(serde_json::json!({
                "automationSchemaVersion": "1",
                "profile": "godot-pixel-art@1.0.0",
                "inputFingerprint": record.input_hash,
                "recipeHash": record.recipe_hash,
            })),
            animation_name: request.metadata.animation.clone(),
            animation_frames: None,
            fps: request.metadata.fps,
            loop_animation: request.metadata.loop_animation,
            anchor,
            quality_report,
        },
    })
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let pack_hash = hash_directory(&export.pack_dir)?;
    let mut artifacts = vec![
        JobArtifactRecord {
            kind: "gsfpack".into(),
            path: export.pack_dir,
            sha256: Some(pack_hash),
        },
        JobArtifactRecord {
            kind: "preview_gif".into(),
            path: export.preview_gif_path,
            sha256: None,
        },
        JobArtifactRecord {
            kind: "quality_report".into(),
            path: export.quality_report_path,
            sha256: None,
        },
    ];
    if let Some(artifact) = repair_artifact {
        artifacts.push(artifact);
    }
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record.artifacts.extend(artifacts);
            record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
            if let Some(export_step) = record.steps.iter_mut().find(|step| step.name == "export") {
                export_step.state = "succeeded".into();
            }
        })
        .map_err(Into::into)
}

fn run_install_godot(
    store: &JobStore,
    job_id: &str,
    request: &GodotInstallRequest,
) -> Result<JobRecord, AutomationRunError> {
    check_cancelled(store, job_id)?;
    forge_pack::validate_pack_layout(&request.pack_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let pack_summary = forge_pack::inspect_pack(&request.pack_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let provider_refs =
        effective_provider_refs(&request.pack_path, &pack_summary, &request.provider_refs)?;
    let pack_sha256 = hash_directory(&request.pack_path)?;
    let record = store.read_record(job_id)?;
    let target = request.project_path.join(&request.target);
    ensure_target_inside_project(&request.project_path, &target)?;
    let backup = record.job_dir.join("backups/godot-target");
    step(store, job_id, "validate", "succeeded", 0.18, None)?;

    if target.exists() {
        if !target.join(OWNERSHIP_MARKER).is_file() {
            return Err(AutomationRunError::Processing(format!(
                "refusing to replace non-Forge-owned directory: {}",
                target.display()
            )));
        }
        if backup.exists() {
            fs::remove_dir_all(&backup)?;
        }
        copy_directory(&target, &backup)?;
        fs::remove_dir_all(&target)?;
    }
    fs::create_dir_all(&target)?;
    copy_godot_pack_sources(&request.pack_path, &target, &pack_summary.asset_type)?;
    step(store, job_id, "backup", "succeeded", 0.3, None)?;

    let script = record.job_dir.join("tools/install_forge_pack.gd");
    fs::write(&script, GODOT_INSTALL_SCRIPT)?;
    let godot = locate_godot()
        .ok_or_else(|| AutomationRunError::Processing("Godot 4 executable was not found".into()))?;
    require_godot_46(&godot)?;
    let import_output = Command::new(&godot)
        .arg("--headless")
        .arg("--import")
        .arg("--path")
        .arg(&request.project_path)
        .output()?;
    fs::write(
        record.job_dir.join("logs/godot.import.stdout.log"),
        &import_output.stdout,
    )?;
    fs::write(
        record.job_dir.join("logs/godot.import.stderr.log"),
        &import_output.stderr,
    )?;
    if !import_output.status.success() {
        let _ = fs::remove_dir_all(&target);
        if backup.exists() {
            copy_directory(&backup, &target)?;
        }
        return Err(AutomationRunError::Processing(format!(
            "Godot asset import failed with status {}",
            import_output.status
        )));
    }
    let output = Command::new(&godot)
        .arg("--headless")
        .arg("--path")
        .arg(&request.project_path)
        .arg("--script")
        .arg(&script)
        .arg("--")
        .arg(&request.pack_path)
        .arg(&request.target)
        .output()?;
    fs::write(record.job_dir.join("logs/godot.stdout.log"), &output.stdout)?;
    fs::write(record.job_dir.join("logs/godot.stderr.log"), &output.stderr)?;

    if !output.status.success() {
        let _ = fs::remove_dir_all(&target);
        if backup.exists() {
            copy_directory(&backup, &target)?;
        }
        return Err(AutomationRunError::Processing(format!(
            "Godot import failed with status {}",
            output.status
        )));
    }

    let (scene_path, frames_path) = match pack_summary.asset_type.as_str() {
        "icon_set" => (target.join("items"), target.join("items")),
        "prop_set" => (target.join("scenes"), target.join("items")),
        "terrain_set" => (
            target.join("forge_terrain_preview.tscn"),
            target.join("forge_terrain_set.tres"),
        ),
        "building_kit" => (
            target.join("scenes"),
            target.join("forge_building_kit.tres"),
        ),
        "map" => (
            target.join("forge_world.tscn"),
            target.join("forge_terrain_set.tres"),
        ),
        _ => (
            target.join("forge_animated_sprite.tscn"),
            target.join("forge_sprite_frames.tres"),
        ),
    };
    if !scene_path.exists() || !frames_path.exists() {
        let _ = fs::remove_dir_all(&target);
        if backup.exists() {
            copy_directory(&backup, &target)?;
        }
        return Err(AutomationRunError::Processing(
            "Godot exited successfully but required scene resources are missing".into(),
        ));
    }
    verify_external_godot_resources(&target)?;

    let asset_key = request
        .asset_key
        .clone()
        .or_else(|| {
            request
                .target
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
        .ok_or_else(|| AutomationRunError::Processing("Godot target has no asset key".into()))?;
    let (scene_relative, frames_relative) = match pack_summary.asset_type.as_str() {
        "icon_set" => (request.target.join("items"), request.target.join("items")),
        "prop_set" => (request.target.join("scenes"), request.target.join("items")),
        "terrain_set" => (
            request.target.join("forge_terrain_preview.tscn"),
            request.target.join("forge_terrain_set.tres"),
        ),
        "building_kit" => (
            request.target.join("scenes"),
            request.target.join("forge_building_kit.tres"),
        ),
        "map" => (
            request.target.join("forge_world.tscn"),
            request.target.join("forge_terrain_set.tres"),
        ),
        _ => (
            request.target.join("forge_animated_sprite.tscn"),
            request.target.join("forge_sprite_frames.tres"),
        ),
    };
    let usage_relative = request.target.join("forge_usage.json");
    let usage_path = request.project_path.join(&usage_relative);
    let loop_selection = fs::read(request.pack_path.join("quality/loops.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let pack_provenance = fs::read(request.pack_path.join("forgepack.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
    let provider_retry_methods = pack_provenance
        .as_ref()
        .and_then(|value| value.pointer("/source/metadata/providerRetryMethods"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let has_topdown_directions =
        ["idle", "walk_up", "walk_right", "walk_down"]
            .iter()
            .all(|required| {
                pack_summary
                    .animations
                    .iter()
                    .any(|animation| animation.name == *required)
            });
    let usage = serde_json::json!({
        "schemaVersion": "1",
        "assetKey": &asset_key,
        "assetId": &pack_summary.id,
        "packSha256": &pack_sha256,
        "kind": &pack_summary.asset_type,
        "scenePath": format!("res://{}", scene_relative.display()),
        "spriteFramesPath": format!("res://{}", frames_relative.display()),
        "defaultAnimation": &pack_summary.default_animation,
        "animations": &pack_summary.animations,
        "items": &pack_summary.items,
        "directionalPlayback": if has_topdown_directions {
            Some(serde_json::json!({
                "up": { "animation": "walk_up", "flipH": false },
                "right": { "animation": "walk_right", "flipH": false },
                "down": { "animation": "walk_down", "flipH": false },
                "left": { "animation": "walk_right", "flipH": true },
                "idle": { "animation": "idle", "flipH": false }
            }))
        } else {
            None
        },
        "providerProvenance": &provider_refs,
        "loopSelection": loop_selection,
        "providerRetryMethods": provider_retry_methods,
        "nodeType": match pack_summary.asset_type.as_str() {
            "icon_set" => "Texture2D",
            "prop_set" => "Sprite2D",
            "terrain_set" => "TileSet",
            "building_kit" => "Node2D",
            "map" => "TileMapLayer",
            _ => "AnimatedSprite2D",
        },
        "worldGeneration": pack_provenance.as_ref().and_then(|value| {
            matches!(pack_summary.asset_type.as_str(), "terrain_set" | "building_kit" | "map")
                .then(|| value.pointer("/source/metadata").cloned())
                .flatten()
        }),
        "gameplayControllerIncluded": false,
    });
    fs::write(&usage_path, serde_json::to_vec_pretty(&usage)?)?;

    let marker = serde_json::json!({
        "schemaVersion": "1",
        "owner": "Game Sprite Forge",
        "jobId": job_id,
        "packPath": request.pack_path,
        "assetKey": &asset_key,
        "projectManifest": PROJECT_MANIFEST_RELATIVE,
    });
    fs::write(
        target.join(OWNERSHIP_MARKER),
        serde_json::to_vec_pretty(&marker)?,
    )?;
    let manifest_path = match register_project_asset(RegisterProjectAsset {
        project_path: &request.project_path,
        asset_key: &asset_key,
        pack_path: &request.pack_path,
        pack_sha256: &pack_sha256,
        godot_target: &request.target,
        scene_path: &scene_relative,
        sprite_frames_path: &frames_relative,
        usage_path: &usage_relative,
        pack: &pack_summary,
        provider_refs: &provider_refs,
        job_id,
    }) {
        Ok(path) => path,
        Err(error) => {
            let _ = fs::remove_dir_all(&target);
            if backup.exists() {
                copy_directory(&backup, &target)?;
            }
            return Err(AutomationRunError::Processing(error.to_string()));
        }
    };
    let catalog_link = if let Some(catalog_project) = &request.catalog_project_path {
        Some(
            link_catalog_install(
                catalog_project,
                &pack_summary.id,
                request.project_path.clone(),
                request.target.clone(),
            )
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?,
        )
    } else {
        None
    };
    let mut artifacts = vec![
        JobArtifactRecord {
            kind: "godot_scene".into(),
            path: scene_path,
            sha256: None,
        },
        JobArtifactRecord {
            kind: "godot_usage".into(),
            path: usage_path,
            sha256: None,
        },
        JobArtifactRecord {
            kind: "project_manifest".into(),
            path: manifest_path,
            sha256: None,
        },
    ];
    if let Some(path) = catalog_link {
        artifacts.push(JobArtifactRecord {
            kind: "project_catalog".into(),
            path,
            sha256: None,
        });
    }
    store
        .update_record(job_id, |record| {
            record.state = JobState::Exported;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.progress = 1.0;
            record.worker_pid = None;
            record
                .steps
                .iter_mut()
                .for_each(|step| step.state = "succeeded".into());
            record.artifacts.extend(artifacts);
            record.next_actions = vec![
                "inspect_project".into(),
                "open_godot_project".into(),
                "open_job".into(),
            ];
        })
        .map_err(Into::into)
}

fn effective_provider_refs(
    pack_path: &Path,
    pack: &forge_pack::PackInspectSummary,
    requested: &[ProviderAssetRef],
) -> Result<Vec<ProviderAssetRef>, AutomationRunError> {
    if !requested.is_empty() {
        return Ok(requested.to_vec());
    }
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    let provider_generated = forgepack
        .pointer("/source/kind")
        .and_then(serde_json::Value::as_str)
        == Some("provider_generation");
    let provider = forgepack
        .pointer("/source/metadata/provider")
        .or_else(|| {
            provider_generated
                .then(|| forgepack.pointer("/source/name"))
                .flatten()
        })
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|provider| {
            !provider.is_empty() && provider.len() <= 64 && !provider.chars().any(char::is_control)
        });
    Ok(provider
        .map(|provider| {
            vec![ProviderAssetRef {
                provider: provider.into(),
                asset_id: None,
                label: Some(pack.name.clone()),
            }]
        })
        .unwrap_or_default())
}

fn copy_godot_pack_sources(
    pack: &Path,
    target: &Path,
    asset_type: &str,
) -> Result<(), AutomationRunError> {
    let helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json"))?)?;
    if matches!(asset_type, "terrain_set" | "building_kit" | "map") {
        let mut textures = Vec::new();
        match asset_type {
            "terrain_set" | "building_kit" => {
                let relative = helper
                    .get("atlas")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        AutomationRunError::Processing(format!(
                            "{asset_type} Godot helper has no atlas"
                        ))
                    })?;
                textures.push(relative.to_string());
            }
            "map" => {
                for field in ["terrainAtlas", "buildingAtlas"] {
                    let relative = helper
                        .get(field)
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            AutomationRunError::Processing(format!(
                                "map Godot helper has no {field}"
                            ))
                        })?;
                    textures.push(relative.to_string());
                }
                for entry in helper
                    .get("propTextures")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let relative = entry
                        .get("texture")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            AutomationRunError::Processing(
                                "map prop texture entry has no texture".into(),
                            )
                        })?;
                    textures.push(relative.to_string());
                }
            }
            _ => unreachable!(),
        }
        for relative in textures {
            if relative.contains("..") || Path::new(&relative).is_absolute() {
                return Err(AutomationRunError::Processing(
                    "world texture path may not escape the Pack".into(),
                ));
            }
            let source = pack.join(&relative);
            if !source.is_file() {
                return Err(AutomationRunError::Processing(format!(
                    "world texture is missing: {relative}"
                )));
            }
            let file_name = Path::new(&relative).file_name().ok_or_else(|| {
                AutomationRunError::Processing("world texture has no filename".into())
            })?;
            fs::copy(source, target.join(file_name))?;
        }
        return Ok(());
    }
    if matches!(asset_type, "icon_set" | "prop_set") {
        let items = helper
            .get("items")
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                AutomationRunError::Processing("static Godot helper has no items".into())
            })?;
        let item_target = target.join("items");
        fs::create_dir_all(&item_target)?;
        for item in items {
            let id = item
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    AutomationRunError::Processing("static Godot item has no id".into())
                })?;
            let source = item
                .get("texture")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    AutomationRunError::Processing("static Godot item has no texture".into())
                })?;
            let source_path = pack.join(source);
            if !source_path.is_file() || source.contains("..") {
                return Err(AutomationRunError::Processing(format!(
                    "static item texture is invalid: {source}"
                )));
            }
            fs::copy(source_path, item_target.join(format!("{id}.png")))?;
        }
        return Ok(());
    }
    let textures = helper
        .pointer("/spriteFrames/textures")
        .and_then(|value| value.as_array())
        .ok_or_else(|| AutomationRunError::Processing("Godot helper has no textures".into()))?;
    for texture in textures {
        let relative = texture.as_str().ok_or_else(|| {
            AutomationRunError::Processing("Godot texture entry must be a string".into())
        })?;
        if relative.contains("..") {
            return Err(AutomationRunError::Processing(
                "Godot texture path may not traverse parents".into(),
            ));
        }
        let source = pack.join(relative);
        let file_name = Path::new(relative).file_name().ok_or_else(|| {
            AutomationRunError::Processing("Godot texture path has no filename".into())
        })?;
        fs::copy(source, target.join(file_name))?;
    }
    Ok(())
}

fn verify_external_godot_resources(target: &Path) -> Result<(), AutomationRunError> {
    fn visit(directory: &Path) -> Result<(), AutomationRunError> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                visit(&path)?;
                continue;
            }
            if !matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("tres" | "tscn")
            ) {
                continue;
            }
            let metadata = fs::metadata(&path)?;
            if metadata.len() >= 1024 * 1024 {
                return Err(AutomationRunError::Processing(format!(
                    "Godot text resource exceeds 1 MiB: {}",
                    path.display()
                )));
            }
            let text = fs::read_to_string(&path)?;
            if text.contains("sub_resource type=\"Image\"")
                || text.contains("sub_resource type=\"ImageTexture\"")
                || text.contains("ImageTexture.create_from_image")
                || text
                    .lines()
                    .any(|line| line.trim_start().starts_with("data = PackedByteArray"))
            {
                return Err(AutomationRunError::Processing(format!(
                    "Godot resource embeds image pixels instead of referencing an external texture: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }
    visit(target)
}

fn ingest_generated_loop_candidates(
    job_dir: &Path,
    input: &AssetInput,
) -> Result<(Vec<PathBuf>, Option<LoopCandidateTiming>), AutomationRunError> {
    let AssetInput::VideoClip {
        path,
        start_time_ms,
        end_time_ms,
        target_frame_count,
    } = input
    else {
        return Ok((ingest_frames(job_dir, input)?, None));
    };
    let probe = probe_video(&ProbeVideoParams {
        input_path: path.clone(),
        configured_ffprobe_path: None,
        bundled_resource_path: None,
    })
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let resolved_end =
        end_time_ms.unwrap_or_else(|| (probe.duration_seconds * 1000.0).round().max(0.0) as u64);
    let result = extract_candidate_frames(&ExtractCandidateFramesParams {
        input_path: path.clone(),
        start_time_ms: *start_time_ms,
        end_time_ms: resolved_end,
        maximum_fps: 12.0,
        maximum_frame_count: 96,
        output_directory: job_dir.join("candidates"),
        configured_ffmpeg_path: None,
        bundled_resource_path: None,
    })
    .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    if result.frames.len() <= *target_frame_count as usize {
        return Err(AutomationRunError::Processing(format!(
            "legacy_artifact_missing: generated video produced {} candidates, but loop selection needs more than targetFrameCount {}",
            result.frames.len(), target_frame_count
        )));
    }
    Ok((
        result.frames,
        Some(LoopCandidateTiming {
            sample_fps: result.sample_fps,
            duration_ms: result.duration_ms,
        }),
    ))
}

fn select_generated_animation_loop(
    workspace: &Path,
    animation: &str,
    raw_paths: &[PathBuf],
    matting: &MattingRecipe,
    normalize: crate::frames::NormalizeOptions,
    timing: LoopCandidateTiming,
    target_frame_count: u32,
) -> Result<(Vec<PathBuf>, LoopSelectionReport), AutomationRunError> {
    let analysis_workspace = workspace.join("loop-analysis");
    let analysis_raw_dir = analysis_workspace.join("raw");
    fs::create_dir_all(&analysis_raw_dir)?;
    let mut analysis_raw_paths = Vec::with_capacity(raw_paths.len());
    for (index, path) in raw_paths.iter().enumerate() {
        let frame = image::open(path)?.to_rgba8();
        let target = analysis_raw_dir.join(format!("frame_{:05}.png", index + 1));
        loop_analysis_image(&frame).save(&target)?;
        analysis_raw_paths.push(target);
    }
    let analysis_matted = apply_matting(&analysis_workspace, &analysis_raw_paths, matting)?;
    let processed_images = analysis_matted
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let source_signatures = processed_images
        .iter()
        .map(loop_analysis_image)
        .map(|image| image_signature(&image))
        .collect::<Vec<_>>();
    let provisional = normalize_frames(&processed_images, normalize)
        .into_iter()
        .map(|frame| loop_analysis_image(&frame.image))
        .collect::<Vec<_>>();
    let provisional_dir = workspace.join("processed/provisional-aligned");
    fs::create_dir_all(&provisional_dir)?;
    for (index, frame) in provisional.iter().enumerate() {
        frame.save(provisional_dir.join(format!("frame_{:05}.png", index + 1)))?;
    }
    let policy = LoopSelectionPolicy::for_animation(
        animation,
        target_frame_count,
        timing.sample_fps,
        timing.duration_ms,
    );
    let mut selected = select_loop_frames(&provisional, policy)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    if source_signatures
        .iter()
        .any(|signature| signature.subject_count > 1)
    {
        selected.report.verdict = LoopSelectionVerdict::Blocked;
        selected.report.reasons.push("multiple_subjects".into());
    }
    if source_signatures
        .iter()
        .any(|signature| signature.foreground_scale > 0.0 && !signature.cell_boundary_safe)
    {
        selected.report.verdict = LoopSelectionVerdict::Blocked;
        selected.report.reasons.push("candidate_cropped".into());
    }
    let selected_dir = workspace.join("processed/loop-selected");
    fs::create_dir_all(&selected_dir)?;
    let selected_raw_paths = selected
        .report
        .output_frame_indices
        .iter()
        .map(|source_index| {
            raw_paths.get(*source_index).cloned().ok_or_else(|| {
                AutomationRunError::Processing(format!(
                    "loop selection referenced missing candidate frame {source_index}"
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let selected_matted = apply_matting(workspace, &selected_raw_paths, matting)?;
    let mut selected_paths = Vec::with_capacity(selected.report.output_frame_indices.len());
    for (output_index, source) in selected_matted.iter().enumerate() {
        let target = selected_dir.join(format!("frame_{:05}.png", output_index + 1));
        fs::copy(source, &target)?;
        selected_paths.push(target);
    }
    selected.report.output_frame_sha256 = selected_paths
        .iter()
        .map(|path| hash_file(path))
        .collect::<Result<Vec<_>, _>>()?;
    let report_path = workspace.join("loop-selection-report.json");
    fs::write(&report_path, serde_json::to_vec_pretty(&selected.report)?)?;
    Ok((selected_paths, selected.report))
}

fn loop_analysis_image(image: &image::RgbaImage) -> image::RgbaImage {
    const LOOP_ANALYSIS_CANVAS: u32 = 256;
    let longest = image.width().max(image.height()).max(1);
    if longest <= LOOP_ANALYSIS_CANVAS {
        return image.clone();
    }
    let width = (image.width() * LOOP_ANALYSIS_CANVAS / longest).max(1);
    let height = (image.height() * LOOP_ANALYSIS_CANVAS / longest).max(1);
    image::imageops::resize(image, width, height, image::imageops::FilterType::Nearest)
}

fn apply_loop_selection_quality(quality: &mut QualityReport, selection: &LoopSelectionReport) {
    quality.metrics.loop_match_score = selection.composite_score;
    quality.notes.push(LOOP_SELECTION_PROFILE.into());
    quality.notes.push(format!(
        "loop_selected:{}..{}",
        selection.selected_start_frame, selection.selected_end_boundary_frame
    ));
    if selection.verdict != LoopSelectionVerdict::GameReady
        && !quality
            .recommendations
            .contains(&QualityRecommendationId::TrimLoopRange)
    {
        quality
            .recommendations
            .push(QualityRecommendationId::TrimLoopRange);
    }
    quality.verdict = match selection.verdict {
        LoopSelectionVerdict::GameReady => quality.verdict,
        LoopSelectionVerdict::AwaitingReview => {
            if quality.verdict == QualityVerdict::GameReady {
                QualityVerdict::NeedsCleanup
            } else {
                quality.verdict
            }
        }
        LoopSelectionVerdict::Regenerate => {
            if matches!(
                quality.verdict,
                QualityVerdict::GameReady | QualityVerdict::NeedsCleanup
            ) {
                QualityVerdict::PrototypeUsable
            } else {
                quality.verdict
            }
        }
        LoopSelectionVerdict::Blocked => QualityVerdict::Blocked,
    };
}

fn ingest_frames(job_dir: &Path, input: &AssetInput) -> Result<Vec<PathBuf>, AutomationRunError> {
    match input {
        AssetInput::PngSequence { paths } => {
            let raw = job_dir.join("raw");
            fs::create_dir_all(&raw)?;
            let mut outputs = Vec::with_capacity(paths.len());
            for (index, path) in paths.iter().enumerate() {
                let target = raw.join(format!("frame_{:05}.png", index + 1));
                fs::copy(path, &target)?;
                outputs.push(target);
            }
            Ok(outputs)
        }
        AssetInput::SpriteSheet { path, split } => {
            let result = match split {
                SpriteSheetSplit::FixedGrid(grid) => {
                    slice_sprite_sheet_grid(&SliceSpriteSheetParams {
                        sheet_path: path.clone(),
                        output_directory: job_dir.to_path_buf(),
                        frame_width: grid.frame_width,
                        frame_height: grid.frame_height,
                        columns: grid.columns,
                        rows: grid.rows,
                    })
                }
                SpriteSheetSplit::TransparentGutters {
                    alpha_threshold,
                    min_gap_px,
                } => slice_sprite_sheet_transparent(&SliceSpriteSheetTransparentParams {
                    sheet_path: path.clone(),
                    output_directory: job_dir.to_path_buf(),
                    alpha_threshold: *alpha_threshold,
                    min_gap_px: *min_gap_px,
                }),
            }
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
            Ok(result.frames)
        }
        AssetInput::Gsfpack { .. } => unreachable!("handled before frame ingestion"),
        AssetInput::VideoClip {
            path,
            start_time_ms,
            end_time_ms,
            target_frame_count,
        } => {
            let probe = probe_video(&ProbeVideoParams {
                input_path: path.clone(),
                configured_ffprobe_path: None,
                bundled_resource_path: None,
            })
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
            let resolved_end = end_time_ms
                .unwrap_or_else(|| (probe.duration_seconds * 1000.0).round().max(0.0) as u64);
            let result = extract_sampled_frames(&SampleVideoFramesParams {
                input_path: path.clone(),
                start_time_ms: *start_time_ms,
                end_time_ms: Some(resolved_end),
                target_frame_count: *target_frame_count,
                output_directory: job_dir.to_path_buf(),
                configured_ffmpeg_path: None,
                bundled_resource_path: None,
            })
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
            Ok(result.frames)
        }
    }
}

fn apply_matting(
    job_dir: &Path,
    raw_frames: &[PathBuf],
    recipe: &MattingRecipe,
) -> Result<Vec<PathBuf>, AutomationRunError> {
    let processed = job_dir.join("processed/matted");
    fs::create_dir_all(&processed)?;
    match recipe {
        MattingRecipe::PreserveAlpha => {
            let mut outputs = Vec::with_capacity(raw_frames.len());
            for (index, path) in raw_frames.iter().enumerate() {
                let target = processed.join(format!("frame_{:05}.png", index + 1));
                fs::copy(path, &target)?;
                outputs.push(target);
            }
            Ok(outputs)
        }
        MattingRecipe::AutoCorners { parameters } => {
            let mut parameters = parameters.clone();
            parameters.key_mode = ChromaKeyMode::AutoCorners;
            let output = process_chroma_batch(raw_frames, &processed, &parameters)
                .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
            Ok(output
                .frames
                .iter()
                .map(|frame| output.processed_dir.join(&frame.frame))
                .collect())
        }
        MattingRecipe::ManualColor { color, parameters } => {
            let mut parameters: ChromaParameters = parameters.clone();
            parameters.key_mode = ChromaKeyMode::Manual;
            parameters.manual_key_color = color.clone();
            let output = process_chroma_batch(raw_frames, &processed, &parameters)
                .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
            Ok(output
                .frames
                .iter()
                .map(|frame| output.processed_dir.join(&frame.frame))
                .collect())
        }
    }
}

type NormalizeOutput = (
    Vec<PathBuf>,
    Vec<crate::frames::FrameBbox>,
    Vec<FrameSize>,
    crate::frames::FootAnchor,
    Vec<serde_json::Value>,
);

fn normalize_and_save(
    job_dir: &Path,
    paths: &[PathBuf],
    options: crate::frames::NormalizeOptions,
) -> Result<NormalizeOutput, AutomationRunError> {
    let images = paths
        .iter()
        .map(|path| Ok(image::open(path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()?;
    let normalized = normalize_frames(&images, options);
    let target_dir = job_dir.join("processed/normalized");
    fs::create_dir_all(&target_dir)?;
    let mut paths = Vec::with_capacity(normalized.len());
    let mut bboxes = Vec::with_capacity(normalized.len());
    let mut sizes = Vec::with_capacity(normalized.len());
    let mut summaries = Vec::with_capacity(normalized.len());
    for (index, frame) in normalized.iter().enumerate() {
        let path = target_dir.join(format!("frame_{:05}.png", index + 1));
        frame.image.save(&path)?;
        let bbox = bbox_from_image(&frame.image, options.alpha_threshold);
        paths.push(path);
        bboxes.push(bbox);
        sizes.push(frame.size);
        summaries.push(serde_json::json!({
            "index": index,
            "bbox": bbox,
            "size": frame.size,
            "anchor": frame.anchor,
            "offsetX": frame.offset_x,
            "offsetY": frame.offset_y,
            "warnings": frame.warnings,
        }));
    }
    let anchor = normalized
        .first()
        .map(|frame| frame.anchor)
        .ok_or_else(|| AutomationRunError::Processing("normalization produced no frames".into()))?;
    Ok((paths, bboxes, sizes, anchor, summaries))
}

fn step(
    store: &JobStore,
    job_id: &str,
    name: &str,
    state: &str,
    progress: f32,
    message: Option<String>,
) -> Result<(), AutomationRunError> {
    store.update_record(job_id, |record| {
        record.progress = progress;
        if let Some(step) = record.steps.iter_mut().find(|step| step.name == name) {
            step.state = state.to_string();
            step.message = message;
        }
    })?;
    Ok(())
}

fn check_cancelled(store: &JobStore, job_id: &str) -> Result<(), AutomationRunError> {
    if store.read_record(job_id)?.cancellation_requested {
        Err(AutomationRunError::Cancelled)
    } else {
        Ok(())
    }
}

fn source_kind_for_input(input: &AssetInput) -> SourceKind {
    match input {
        AssetInput::PngSequence { .. } => SourceKind::ImportFrames,
        AssetInput::SpriteSheet { .. } => SourceKind::ImportSpriteSheet,
        AssetInput::Gsfpack { .. } => SourceKind::ImportGsfpack,
        AssetInput::VideoClip { .. } => SourceKind::ImportVideo,
    }
}

fn input_display_name(input: &AssetInput) -> Option<String> {
    match input {
        AssetInput::PngSequence { paths } => paths
            .first()
            .and_then(|path| path.parent())
            .map(|path| path.display().to_string()),
        AssetInput::SpriteSheet { path, .. }
        | AssetInput::Gsfpack { path }
        | AssetInput::VideoClip { path, .. } => Some(path.display().to_string()),
    }
}

fn steps_for_operation(operation: &AutomationOperation) -> Vec<JobStepRecord> {
    let names = match operation {
        AutomationOperation::PrepareAsset(_) => vec![
            "ingest".to_string(),
            "matting".to_string(),
            "normalize".to_string(),
            "quality".to_string(),
            "export".to_string(),
        ],
        AutomationOperation::PrepareCharacterPack(request) => {
            let mut names = Vec::with_capacity(request.animations.len() * 3 + 2);
            for animation in &request.animations {
                names.push(format!("{}:ingest", animation.name));
                names.push(format!("{}:matting", animation.name));
            }
            names.push("shared:normalize".to_string());
            for animation in &request.animations {
                names.push(format!("{}:quality", animation.name));
            }
            names.push("pack:export".to_string());
            names
        }
        AutomationOperation::GenerateCharacterPack(request)
            if request.workflow.id == "topdown-keyframes"
                && request.workflow.version == "2.0.0" =>
        {
            let mut names = Vec::with_capacity(32 + 14);
            for animation in ["idle", "walk_up", "walk_right", "walk_down"] {
                for frame in 0..8 {
                    names.push(format!("provider:{animation}:frame-{frame:02}"));
                }
                names.push(format!("{animation}:ingest"));
                names.push(format!("{animation}:matting"));
            }
            names.push("shared:normalize".into());
            for animation in ["idle", "walk_up", "walk_right", "walk_down"] {
                names.push(format!("{animation}:quality"));
            }
            names.push("pack:export".into());
            names
        }
        AutomationOperation::GenerateCharacterPack(_) => {
            let mut names = vec!["provider:reference".to_string()];
            for animation in ["idle", "walk_up", "walk_right", "walk_down"] {
                names.push(format!("provider:{animation}"));
                names.push(format!("{animation}:ingest"));
                names.push(format!("{animation}:matting"));
            }
            names.push("shared:normalize".to_string());
            for animation in ["idle", "walk_up", "walk_right", "walk_down"] {
                names.push(format!("{animation}:quality"));
            }
            names.push("pack:export".to_string());
            names
        }
        AutomationOperation::CreateStyleLock(_) => {
            vec!["style:materialize".into(), "style:lock".into()]
        }
        AutomationOperation::CreateSubjectLock(_) => {
            vec!["subject:materialize".into(), "subject:lock".into()]
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            let mut names = request
                .asset
                .items
                .iter()
                .map(|item| format!("item:{}", item.id))
                .collect::<Vec<_>>();
            names.push("consistency:report".into());
            names.push("pack:export".into());
            names
        }
        AutomationOperation::CreateEnvironmentLock(_) => {
            vec!["environment:materialize".into(), "environment:lock".into()]
        }
        AutomationOperation::GenerateTerrainSet(_) => vec![
            "terrain:materials".into(),
            "terrain:compose".into(),
            "terrain:validate".into(),
            "pack:export".into(),
        ],
        AutomationOperation::GenerateBuildingKit(_) => vec![
            "building:materials".into(),
            "building:compose".into(),
            "building:validate".into(),
            "pack:export".into(),
        ],
        AutomationOperation::CompileMap(_) => vec![
            "map:validate_spec".into(),
            "map:compile".into(),
            "map:validate".into(),
            "pack:export".into(),
        ],
        AutomationOperation::InstallGodot(_) => ["validate", "backup", "install", "verify"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        AutomationOperation::BuildProject(_) => [
            "validate_manifest",
            "diff_catalog",
            "run_child_builds",
            "update_catalog",
            "summarize",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
    };
    names
        .into_iter()
        .map(|name| JobStepRecord {
            name,
            state: "pending".into(),
            message: None,
        })
        .collect()
}

fn locate_godot() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FORGE_GODOT_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    let candidates = [
        PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot"),
        PathBuf::from("/Applications/Godot_mono.app/Contents/MacOS/Godot"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .or_else(|| ["godot4", "godot"].into_iter().find_map(which))
}

fn require_godot_46(godot: &Path) -> Result<(), AutomationRunError> {
    let output = Command::new(godot).arg("--version").output()?;
    if !output.status.success() {
        return Err(AutomationRunError::Processing(
            "Godot version check failed".into(),
        ));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if !version.trim().starts_with("4.6.") {
        return Err(AutomationRunError::Processing(format!(
            "Forge v0.2 requires Godot 4.6.x; found {}",
            version.trim()
        )));
    }
    Ok(())
}

fn ensure_target_inside_project(project: &Path, target: &Path) -> Result<(), AutomationRunError> {
    let canonical_project = fs::canonicalize(project)?;
    let mut existing = target;
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            AutomationRunError::Processing("Godot target has no existing project ancestor".into())
        })?;
    }
    let metadata = fs::symlink_metadata(existing)?;
    if metadata.file_type().is_symlink() {
        return Err(AutomationRunError::Processing(
            "Godot target may not traverse a symbolic link".into(),
        ));
    }
    let canonical_existing = fs::canonicalize(existing)?;
    if !canonical_existing.starts_with(&canonical_project) {
        return Err(AutomationRunError::Processing(
            "Godot target resolves outside the project".into(),
        ));
    }
    if target.exists() {
        let metadata = fs::symlink_metadata(target)?;
        if metadata.file_type().is_symlink()
            || !fs::canonicalize(target)?.starts_with(canonical_project)
        {
            return Err(AutomationRunError::Processing(
                "Godot target is not a regular directory inside the project".into(),
            ));
        }
    }
    Ok(())
}

fn which(name: &str) -> Option<PathBuf> {
    let output = Command::new("/usr/bin/which").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8(output.stdout).ok()?;
    let path = PathBuf::from(path.trim());
    path.is_file().then_some(path)
}

fn copy_directory(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination)?;
        }
    }
    Ok(())
}

fn hash_directory(root: &Path) -> Result<String, std::io::Error> {
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("directory root is a symbolic link: {}", root.display()),
        ));
    }
    let mut paths = Vec::new();
    collect_paths(root, root, &mut paths)?;
    paths.sort();
    let mut hasher = Sha256::new();
    hasher.update(b"forge-directory-hash-v2\0");
    for relative in paths {
        let relative_text = relative.to_string_lossy();
        let contents = fs::read(root.join(&relative))?;
        hasher.update(b"file\0");
        hasher.update((relative_text.len() as u64).to_le_bytes());
        hasher.update(relative_text.as_bytes());
        hasher.update((contents.len() as u64).to_le_bytes());
        hasher.update(contents);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn collect_paths(
    root: &Path,
    directory: &Path,
    paths: &mut Vec<PathBuf>,
) -> Result<(), std::io::Error> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "directory contains symbolic link: {}",
                    entry.path().display()
                ),
            ));
        } else if file_type.is_dir() {
            collect_paths(root, &entry.path(), paths)?;
        } else if file_type.is_file() {
            paths.push(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(&entry.path())
                    .to_path_buf(),
            );
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "directory contains unsupported entry: {}",
                    entry.path().display()
                ),
            ));
        }
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), AutomationRunError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}
