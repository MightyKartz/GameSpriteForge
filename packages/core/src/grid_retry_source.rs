use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::asset_project::ConsistencyVerdict;
use crate::automation::AutomationOperation;
use crate::character_direction_motion::DirectionMotionGenerationStageV1;
use crate::character_grid::{
    GridKeyframeActionReportV1, GridKeyframeGenerationMethodV1, GridKeyframePhaseV1,
    GRID_ACTION_PHASE_PROFILE, GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE,
    GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE,
    GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE_LEGACY,
};
use crate::footwear_platform::{
    assess_footwear_platform_action, FOOTWEAR_GUIDE_LEAK_REASON, FOOTWEAR_PLATFORM_REASON,
};
use crate::gait_laterality::{
    GaitLateralityReportV1, GAIT_LATERALITY_PROFILE, WALK_LATERALITY_NOT_ALTERNATING,
};
use crate::job::{JobLifecycleState, JobRecord};
use crate::workflow_graph::{read_workflow_graph, WorkflowArtifactV1, WorkflowGraphV1};

const SOURCE_WORKFLOW: &str = "topdown-grid@9.2.0";

#[derive(Debug, Error)]
pub enum GridRetrySourceError {
    #[error("grid_structured_retry_source_invalid: {0}")]
    Invalid(String),
    #[error("grid_structured_retry_source_io: {0}")]
    Io(#[from] std::io::Error),
    #[error("grid_structured_retry_source_json: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug)]
pub struct ValidatedGridLateralitySourceV1 {
    pub job: JobRecord,
    pub request: crate::automation::GenerateCharacterPackRequest,
    pub report: GridKeyframeActionReportV1,
    pub graph: WorkflowGraphV1,
}

#[derive(Debug)]
pub struct ValidatedGridPlatformSourceV1 {
    pub job: JobRecord,
    pub request: crate::automation::GenerateCharacterPackRequest,
    pub report: GridKeyframeActionReportV1,
    pub graph: WorkflowGraphV1,
}

#[derive(Debug)]
pub struct ValidatedGridFootwearCleanupSourceV1 {
    pub job: JobRecord,
    pub request: crate::automation::GenerateCharacterPackRequest,
    pub report: GridKeyframeActionReportV1,
    pub graph: WorkflowGraphV1,
    pub material_source_job_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridKeyframeProviderManifestV1 {
    schema_version: String,
    workflow: String,
    stage: String,
    provider_id: String,
    profile_id: String,
    image_model: String,
    validation_only: bool,
    validation_animations: Vec<String>,
    actions: Vec<GridKeyframeManifestActionV1>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridKeyframeManifestActionV1 {
    action_report_profile: String,
    action_report_path: PathBuf,
    action_report_sha256: String,
    animation: String,
    direction_node_id: String,
    direction_anchor_sha256: String,
    phase_profile: String,
    frames: Vec<GridKeyframeManifestFrameV1>,
    motion_verdict: ConsistencyVerdict,
    equipment_verdict: ConsistencyVerdict,
    laterality_verdict: Option<ConsistencyVerdict>,
    laterality_report_sha256: Option<String>,
    #[serde(default)]
    footwear_verdict: Option<ConsistencyVerdict>,
    #[serde(default)]
    footwear_report_sha256: Option<String>,
    #[serde(default)]
    recommended_retry_frames: Vec<u8>,
    #[serde(default)]
    retried_frames: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GridKeyframeManifestFrameV1 {
    frame_index: u8,
    phase: GridKeyframePhaseV1,
    attempt: u8,
    generation_method: GridKeyframeGenerationMethodV1,
    sha256: String,
    provider_request_occurred: bool,
    input_frame_sha256: Option<String>,
    replaces_frame_sha256: Option<String>,
    pose_structure_path: PathBuf,
    pose_structure_sha256: String,
    #[serde(default)]
    pose_structure_profile: Option<String>,
}

pub fn validate_v92_laterality_source_closure(
    source: &Path,
    selected_frames: &[u8],
) -> Result<ValidatedGridLateralitySourceV1, GridRetrySourceError> {
    if selected_frames.len() != 1 {
        return invalid("laterality child validation accepts exactly one frame");
    }
    if selected_frames != [2] {
        return invalid(
            "child frame must exactly match the immutable laterality recommendation [2]",
        );
    }
    let canonical_source = fs::canonicalize(source)?;
    let job_path = source.join("job.json");
    let job: JobRecord = read_json(&job_path)?;
    if fs::canonicalize(&job.job_dir)? != canonical_source
        || job.lifecycle_state != JobLifecycleState::Failed
        || job.error_code.as_deref() != Some(WALK_LATERALITY_NOT_ALTERNATING)
    {
        return invalid("source Job is not a stable laterality failure");
    }

    let recipe_value = job.recipe.clone().ok_or_else(|| {
        GridRetrySourceError::Invalid("source Job has no immutable recipe".into())
    })?;
    let recipe: AutomationOperation = serde_json::from_value(recipe_value)?;
    let recipe_sha256 = hash_serialized(&recipe)?;
    if job.recipe_hash.as_deref() != Some(recipe_sha256.as_str()) {
        return invalid("source Job recipe hash does not match its canonical recipe");
    }
    let AutomationOperation::GenerateCharacterPack(request) = recipe else {
        return invalid("source Job recipe is not generated Character media");
    };
    if request.workflow.id != "topdown-grid"
        || request.workflow.version != "9.2.0"
        || !request.validation_only
        || request.validation_animations != ["walk_down"]
        || request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete
        || !request.retry_frames.is_empty()
    {
        if request.workflow.id == "topdown-grid" && request.workflow.version == "9.3.0" {
            return invalid("selected frame already consumed its one laterality fresh retry");
        }
        return invalid(
            "source recipe is not the original topdown-grid@9.2.0 walk_down validation",
        );
    }

    let action_path = source.join("source/grid-keyframe-actions/walk_down.json");
    let motion_path = source.join("source/grid-keyframe-actions/walk_down-motion.json");
    let equipment_path = source.join("source/grid-keyframe-actions/walk_down-equipment.json");
    let laterality_path = source.join("source/grid-keyframe-actions/walk_down-laterality.json");
    let manifest_path = source.join("source/grid-keyframe-provider-manifest.json");
    let graph_path = source.join("workflow-graph.json");
    for (kind, path) in [
        ("grid_keyframe_action_report_walk_down", &action_path),
        ("grid_keyframe_motion_walk_down", &motion_path),
        ("grid_keyframe_equipment_walk_down", &equipment_path),
        ("grid_keyframe_laterality_walk_down", &laterality_path),
        ("provider_manifest", &manifest_path),
        ("workflow_graph", &graph_path),
    ] {
        validate_bound_artifact(&job, kind, path, &canonical_source)?;
    }

    let report: GridKeyframeActionReportV1 = read_json(&action_path)?;
    if report.profile != GRID_STRUCTURED_GAIT_ACTION_REPORT_PROFILE_LEGACY
        || report.animation != "walk_down"
        || report.direction_node_id != "front_idle"
        || report.phase_profile != GRID_ACTION_PHASE_PROFILE
        || report.motion_report_path != motion_path
        || report.equipment_report_path != equipment_path
        || report.laterality_report_path.as_deref() != Some(laterality_path.as_path())
        || report.motion_report_sha256 != hash_file(&motion_path)?
        || report.equipment_report_sha256 != hash_file(&equipment_path)?
        || report.laterality_report_sha256.as_deref() != Some(hash_file(&laterality_path)?.as_str())
        || report.motion_verdict != ConsistencyVerdict::GameReady
        || report.equipment_verdict != ConsistencyVerdict::GameReady
        || report.laterality_verdict != Some(ConsistencyVerdict::Blocked)
        || report.recommended_retry_frames != selected_frames
        || report.frames.len() != 4
    {
        return invalid("Action Report is not the hash-closed legacy V9.2 laterality failure");
    }
    validate_report_frames(&report, &canonical_source)?;
    if matches!(
        report.frames[2].generation_method,
        GridKeyframeGenerationMethodV1::LateralityFreshRetry
            | GridKeyframeGenerationMethodV1::AsymmetricGuideFreshRetry
    ) {
        return invalid("selected frame already consumed a fresh laterality remediation");
    }

    let laterality: GaitLateralityReportV1 = read_json(&laterality_path)?;
    if laterality.profile != GAIT_LATERALITY_PROFILE
        || laterality.animation != "walk_down"
        || laterality.verdict != ConsistencyVerdict::Blocked
        || !laterality
            .reasons
            .iter()
            .any(|reason| reason == WALK_LATERALITY_NOT_ALTERNATING)
        || laterality.recommended_retry_frames != selected_frames
    {
        return invalid("laterality report does not bind the stable frame-2 recommendation");
    }

    let manifest: GridKeyframeProviderManifestV1 = read_json(&manifest_path)?;
    validate_manifest(&manifest, &request, &report)?;

    let graph = read_workflow_graph(&graph_path)
        .map_err(|error| GridRetrySourceError::Invalid(error.to_string()))?;
    validate_graph(
        &graph,
        &job,
        &report,
        &action_path,
        &motion_path,
        &equipment_path,
        &laterality_path,
        &manifest,
        &canonical_source,
    )?;

    Ok(ValidatedGridLateralitySourceV1 {
        job,
        request,
        report,
        graph,
    })
}

pub fn validate_v93_platform_source_closure(
    source: &Path,
    selected_frames: &[u8],
) -> Result<ValidatedGridPlatformSourceV1, GridRetrySourceError> {
    if selected_frames != [2] {
        return invalid("platform-safe child must select only walk_down frame 2");
    }
    let canonical_source = fs::canonicalize(source)?;
    let job_path = source.join("job.json");
    let job: JobRecord = read_json(&job_path)?;
    if fs::canonicalize(&job.job_dir)? != canonical_source
        || job.lifecycle_state != JobLifecycleState::AwaitingReview
        || job.error_code.as_deref() != Some("grid_keyframe_validation_review_required")
    {
        return invalid("source Job is not the immutable V9.3 native-review candidate");
    }
    let recipe_value = job.recipe.clone().ok_or_else(|| {
        GridRetrySourceError::Invalid("source Job has no immutable recipe".into())
    })?;
    let recipe: AutomationOperation = serde_json::from_value(recipe_value)?;
    if job.recipe_hash.as_deref() != Some(hash_serialized(&recipe)?.as_str()) {
        return invalid("source Job recipe hash does not match its canonical recipe");
    }
    let AutomationOperation::GenerateCharacterPack(request) = recipe else {
        return invalid("source Job recipe is not generated Character media");
    };
    if request.workflow.id != "topdown-grid"
        || request.workflow.version != "9.3.0"
        || !request.validation_only
        || request.validation_animations != ["walk_down"]
        || request.retry_animations != ["walk_down"]
        || request.retry_frames.get("walk_down").map(Vec::as_slice) != Some([2].as_slice())
        || request.retry_stages.get("walk_down")
            != Some(&crate::automation::CharacterRetryStage::Frame)
        || request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete
    {
        return invalid("source recipe is not the sole V9.3 walk_down frame-2 remediation");
    }

    let action_path = source.join("source/grid-keyframe-actions/walk_down.json");
    let motion_path = source.join("source/grid-keyframe-actions/walk_down-motion.json");
    let equipment_path = source.join("source/grid-keyframe-actions/walk_down-equipment.json");
    let laterality_path = source.join("source/grid-keyframe-actions/walk_down-laterality.json");
    let manifest_path = source.join("source/grid-keyframe-provider-manifest.json");
    let graph_path = source.join("workflow-graph.json");
    for (kind, path) in [
        ("grid_keyframe_action_report_walk_down", &action_path),
        ("grid_keyframe_motion_walk_down", &motion_path),
        ("grid_keyframe_equipment_walk_down", &equipment_path),
        ("grid_keyframe_laterality_walk_down", &laterality_path),
        ("provider_manifest", &manifest_path),
        ("workflow_graph", &graph_path),
    ] {
        validate_bound_artifact(&job, kind, path, &canonical_source)?;
    }
    let report: GridKeyframeActionReportV1 = read_json(&action_path)?;
    report
        .validate_for_reuse("walk_down")
        .map_err(GridRetrySourceError::Invalid)?;
    if report.profile != GRID_ASYMMETRIC_GAIT_ACTION_REPORT_PROFILE
        || report.retried_frames != [2]
        || report.frames[2].generation_method
            != GridKeyframeGenerationMethodV1::AsymmetricGuideFreshRetry
        || report.frames[2].pose_structure_profile.as_deref()
            != Some(crate::grid_pose_structure::GRID_POSE_STRUCTURE_PROFILE)
    {
        return invalid("Action Report is not the immutable V9.3 asymmetric-guide candidate");
    }
    for frame in &report.frames {
        validate_file(&frame.path, &frame.sha256, &canonical_source)?;
        validate_file(
            frame.pose_structure_path.as_deref().ok_or_else(|| {
                GridRetrySourceError::Invalid("source frame has no PoseStructure".into())
            })?,
            frame.pose_structure_sha256.as_deref().ok_or_else(|| {
                GridRetrySourceError::Invalid("source frame has no PoseStructure hash".into())
            })?,
            &canonical_source,
        )?;
    }
    let images = report
        .frames
        .iter()
        .map(|frame| Ok(image::open(&frame.path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()
        .map_err(|error| GridRetrySourceError::Invalid(error.to_string()))?;
    let platform = assess_footwear_platform_action("walk_down", &images);
    let blocked = platform
        .frames
        .iter()
        .filter(|frame| frame.verdict == ConsistencyVerdict::Blocked)
        .map(|frame| frame.frame_index)
        .collect::<Vec<_>>();
    if blocked != [2]
        || !platform
            .reasons
            .iter()
            .any(|reason| reason == FOOTWEAR_PLATFORM_REASON)
    {
        return invalid("V9.3 candidate does not reproduce the unique frame-2 platform failure");
    }
    let manifest: GridKeyframeProviderManifestV1 = read_json(&manifest_path)?;
    validate_manifest_for_workflow(&manifest, &request, &report, "topdown-grid@9.3.0")?;
    let graph = read_workflow_graph(&graph_path)
        .map_err(|error| GridRetrySourceError::Invalid(error.to_string()))?;
    validate_v93_platform_graph(&graph, &job, &report, &manifest)?;
    Ok(ValidatedGridPlatformSourceV1 {
        job,
        request,
        report,
        graph,
    })
}

pub fn validate_v94_footwear_cleanup_source_closure(
    source: &Path,
    selected_frames: &[u8],
) -> Result<ValidatedGridFootwearCleanupSourceV1, GridRetrySourceError> {
    if selected_frames != [2] {
        return invalid("footwear cleanup child must select only walk_down frame 2");
    }
    let canonical_source = fs::canonicalize(source)?;
    let job: JobRecord = read_json(&source.join("job.json"))?;
    if fs::canonicalize(&job.job_dir)? != canonical_source
        || job.lifecycle_state != JobLifecycleState::AwaitingReview
        || job.error_code.as_deref() != Some("manual_rejected")
    {
        return invalid("source Job is not the immutable manually rejected V9.4 candidate");
    }
    let review_path = source.join("review-decision.json");
    validate_bound_artifact(&job, "review_decision", &review_path, &canonical_source)?;
    let review: serde_json::Value = read_json(&review_path)?;
    if review.get("accepted").and_then(serde_json::Value::as_bool) != Some(false)
        || !review
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|reason| {
                reason.contains("walk_down frame 2")
                    && reason.contains("gray")
                    && reason.contains("boot")
            })
    {
        return invalid("V9.4 review does not bind the frame-2 gray footwear rejection");
    }
    let recipe_value = job.recipe.clone().ok_or_else(|| {
        GridRetrySourceError::Invalid("source Job has no immutable recipe".into())
    })?;
    let recipe: AutomationOperation = serde_json::from_value(recipe_value)?;
    if job.recipe_hash.as_deref() != Some(hash_serialized(&recipe)?.as_str()) {
        return invalid("source Job recipe hash does not match its canonical recipe");
    }
    let AutomationOperation::GenerateCharacterPack(request) = recipe else {
        return invalid("source Job recipe is not generated Character media");
    };
    if request.workflow.id != "topdown-grid"
        || request.workflow.version != "9.4.0"
        || !request.validation_only
        || request.validation_animations != ["walk_down"]
        || request.retry_animations != ["walk_down"]
        || request.retry_frames.get("walk_down").map(Vec::as_slice) != Some([2].as_slice())
        || request.retry_stages.get("walk_down")
            != Some(&crate::automation::CharacterRetryStage::Frame)
    {
        return invalid("source recipe is not the sole V9.4 walk_down frame-2 remediation");
    }
    let action_path = source.join("source/grid-keyframe-actions/walk_down.json");
    let motion_path = source.join("source/grid-keyframe-actions/walk_down-motion.json");
    let equipment_path = source.join("source/grid-keyframe-actions/walk_down-equipment.json");
    let laterality_path = source.join("source/grid-keyframe-actions/walk_down-laterality.json");
    let footwear_path =
        source.join("source/grid-keyframe-actions/walk_down-footwear-platform.json");
    let manifest_path = source.join("source/grid-keyframe-provider-manifest.json");
    let graph_path = source.join("workflow-graph.json");
    for (kind, path) in [
        ("grid_keyframe_action_report_walk_down", &action_path),
        ("grid_keyframe_motion_walk_down", &motion_path),
        ("grid_keyframe_equipment_walk_down", &equipment_path),
        ("grid_keyframe_laterality_walk_down", &laterality_path),
        ("grid_keyframe_footwear_walk_down", &footwear_path),
        ("provider_manifest", &manifest_path),
        ("workflow_graph", &graph_path),
    ] {
        validate_bound_artifact(&job, kind, path, &canonical_source)?;
    }
    let report: GridKeyframeActionReportV1 = read_json(&action_path)?;
    report
        .validate_for_reuse("walk_down")
        .map_err(GridRetrySourceError::Invalid)?;
    if report.profile != GRID_PLATFORM_SAFE_GAIT_ACTION_REPORT_PROFILE
        || report.retried_frames != [2]
        || report.frames[2].generation_method
            != GridKeyframeGenerationMethodV1::PlatformSafeGuideFreshRetry
    {
        return invalid("Action Report is not the immutable V9.4 platform-safe candidate");
    }
    for frame in &report.frames {
        validate_file(&frame.path, &frame.sha256, &canonical_source)?;
        if let (Some(path), Some(sha256)) = (
            frame.pose_structure_path.as_deref(),
            frame.pose_structure_sha256.as_deref(),
        ) {
            validate_file(path, sha256, &canonical_source)?;
        }
    }
    let images = report
        .frames
        .iter()
        .map(|frame| Ok(image::open(&frame.path)?.to_rgba8()))
        .collect::<Result<Vec<_>, image::ImageError>>()
        .map_err(|error| GridRetrySourceError::Invalid(error.to_string()))?;
    let footwear = assess_footwear_platform_action("walk_down", &images);
    let blocked = footwear
        .frames
        .iter()
        .filter(|frame| frame.verdict == ConsistencyVerdict::Blocked)
        .map(|frame| frame.frame_index)
        .collect::<Vec<_>>();
    if blocked != [2]
        || !footwear
            .reasons
            .iter()
            .any(|reason| reason == FOOTWEAR_GUIDE_LEAK_REASON)
    {
        return invalid("V9.4 candidate does not reproduce the unique frame-2 gray footwear leak");
    }
    let manifest: GridKeyframeProviderManifestV1 = read_json(&manifest_path)?;
    validate_manifest_for_workflow(&manifest, &request, &report, "topdown-grid@9.4.0")?;
    let graph = read_workflow_graph(&graph_path)
        .map_err(|error| GridRetrySourceError::Invalid(error.to_string()))?;
    if graph.workflow != "topdown-grid@9.4.0" || graph.job_id != job.job_id {
        return invalid("V9.4 WorkflowGraph does not match its Job");
    }
    Ok(ValidatedGridFootwearCleanupSourceV1 {
        job,
        request,
        report,
        graph,
        material_source_job_dir: canonical_source,
    })
}

pub fn validate_v95_footwear_cleanup_source_closure(
    source: &Path,
    selected_frames: &[u8],
) -> Result<ValidatedGridFootwearCleanupSourceV1, GridRetrySourceError> {
    validate_v95_footwear_cleanup_source_closure_inner(source, selected_frames, &mut Vec::new())
}

fn validate_v95_footwear_cleanup_source_closure_inner(
    source: &Path,
    selected_frames: &[u8],
    visited: &mut Vec<PathBuf>,
) -> Result<ValidatedGridFootwearCleanupSourceV1, GridRetrySourceError> {
    let canonical_source = fs::canonicalize(source)?;
    if visited.contains(&canonical_source) {
        return invalid("V9.5 zero-output recovery source chain contains a cycle");
    }
    visited.push(canonical_source.clone());
    let job: JobRecord = read_json(&canonical_source.join("job.json"))?;
    if job.lifecycle_state == JobLifecycleState::AwaitingReview
        && job.error_code.as_deref() == Some("manual_rejected")
    {
        return validate_v94_footwear_cleanup_source_closure(&canonical_source, selected_frames);
    }
    if selected_frames != [2]
        || fs::canonicalize(&job.job_dir)? != canonical_source
        || job.lifecycle_state != JobLifecycleState::Failed
        || !matches!(
            job.error_code.as_deref(),
            Some("provider_request_failed" | "automation_failed")
        )
    {
        return invalid("source Job is not an eligible V9.5 zero-output recovery failure");
    }
    let recipe_value = job.recipe.clone().ok_or_else(|| {
        GridRetrySourceError::Invalid("source Job has no immutable recipe".into())
    })?;
    let recipe: AutomationOperation = serde_json::from_value(recipe_value)?;
    if job.recipe_hash.as_deref() != Some(hash_serialized(&recipe)?.as_str()) {
        return invalid("source Job recipe hash does not match its canonical recipe");
    }
    let AutomationOperation::GenerateCharacterPack(request) = recipe else {
        return invalid("source Job recipe is not generated Character media");
    };
    if request.workflow.id != "topdown-grid"
        || request.workflow.version != "9.5.0"
        || !request.validation_only
        || request.validation_animations != ["walk_down"]
        || request.retry_animations != ["walk_down"]
        || request.retry_frames.get("walk_down").map(Vec::as_slice) != Some([2].as_slice())
        || request.retry_stages.get("walk_down")
            != Some(&crate::automation::CharacterRetryStage::Frame)
    {
        return invalid("transport source recipe is not the sole V9.5 frame-2 cleanup");
    }
    let usage_path = canonical_source.join("provider-usage.json");
    validate_bound_artifact(&job, "provider_usage", &usage_path, &canonical_source)?;
    let usage: serde_json::Value = read_json(&usage_path)?;
    let usage = usage.get("usage").ok_or_else(|| {
        GridRetrySourceError::Invalid("transport source has no Provider usage object".into())
    })?;
    for field in [
        "requests",
        "generatedImages",
        "generatedVideos",
        "editedVideos",
        "privateFileUploads",
    ] {
        if usage.get(field).and_then(serde_json::Value::as_u64) != Some(0) {
            return invalid("transport source Provider usage is not exactly zero");
        }
    }
    for forbidden in [
        "source/grid-keyframe-actions/walk_down.json",
        "source/grid-keyframe-provider-manifest.json",
        "workflow-graph.json",
        "grid-keyframe-validation/walk_down-contact-sheet.png",
    ] {
        if canonical_source.join(forbidden).exists() {
            return invalid("transport source contains post-Provider completion evidence");
        }
    }
    let source_png = canonical_source
        .join("source/provider/walk_down/frame-02")
        .join(format!(
            "attempt-{}/source.png",
            request.generation.max_attempts_per_animation + 3
        ));
    if source_png.exists()
        || walk_files(&canonical_source.join("source/provider"))?
            .iter()
            .any(|path| path.file_name().and_then(|name| name.to_str()) == Some("source.png"))
    {
        return invalid("transport source contains a generated Provider image");
    }
    let predecessor = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
        GridRetrySourceError::Invalid("recovery source has no immutable predecessor".into())
    })?;
    let mut validated =
        validate_v95_footwear_cleanup_source_closure_inner(predecessor, selected_frames, visited)?;
    if job.parent_job_id.as_deref() != Some(validated.job.job_id.as_str())
        || job.lineage_root_job_id.as_deref()
            != Some(
                validated
                    .job
                    .lineage_root_job_id
                    .as_deref()
                    .unwrap_or(&validated.job.job_id),
            )
    {
        return invalid("transport source parent/lineage does not match its V9.4 predecessor");
    }
    validated.job = job;
    validated.request = request;
    Ok(validated)
}

fn walk_files(root: &Path) -> Result<Vec<PathBuf>, GridRetrySourceError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn validate_report_frames(
    report: &GridKeyframeActionReportV1,
    canonical_source: &Path,
) -> Result<(), GridRetrySourceError> {
    for (index, frame) in report.frames.iter().enumerate() {
        if frame.frame_index != index as u8
            || GridKeyframePhaseV1::for_index(index as u8) != Some(frame.phase)
            || frame.reused
            || !frame.provider_request_occurred
            || frame.pose_structure_profile.is_some()
        {
            return invalid("legacy source frame structure is inconsistent");
        }
        validate_file(&frame.path, &frame.sha256, canonical_source)?;
        let pose_path = frame.pose_structure_path.as_ref().ok_or_else(|| {
            GridRetrySourceError::Invalid("source frame has no PoseStructure".into())
        })?;
        let pose_sha256 = frame.pose_structure_sha256.as_deref().ok_or_else(|| {
            GridRetrySourceError::Invalid("source frame has no PoseStructure hash".into())
        })?;
        validate_file(pose_path, pose_sha256, canonical_source)?;
    }
    Ok(())
}

fn validate_manifest(
    manifest: &GridKeyframeProviderManifestV1,
    request: &crate::automation::GenerateCharacterPackRequest,
    report: &GridKeyframeActionReportV1,
) -> Result<(), GridRetrySourceError> {
    validate_manifest_for_workflow(manifest, request, report, SOURCE_WORKFLOW)
}

fn validate_manifest_for_workflow(
    manifest: &GridKeyframeProviderManifestV1,
    request: &crate::automation::GenerateCharacterPackRequest,
    report: &GridKeyframeActionReportV1,
    workflow: &str,
) -> Result<(), GridRetrySourceError> {
    if manifest.schema_version != "1"
        || manifest.workflow != workflow
        || manifest.stage != "keyframe_validation"
        || manifest.provider_id != request.provider_id
        || manifest.profile_id != request.profile_id
        || manifest.image_model
            != request
                .generation
                .image_model
                .as_deref()
                .unwrap_or_default()
        || !manifest.validation_only
        || manifest.validation_animations != ["walk_down"]
        || manifest.actions.len() != 1
    {
        return invalid("Provider manifest does not match the source recipe");
    }
    let action = &manifest.actions[0];
    if action.animation != report.animation
        || action.action_report_profile != report.profile
        || action.action_report_path
            != report
                .motion_report_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(format!("{}.json", report.animation))
        || action.action_report_sha256 != hash_file(&action.action_report_path)?
        || action.direction_node_id != report.direction_node_id
        || action.direction_anchor_sha256 != report.direction_anchor_sha256
        || action.phase_profile != report.phase_profile
        || action.motion_verdict != report.motion_verdict
        || action.equipment_verdict != report.equipment_verdict
        || action.laterality_verdict != report.laterality_verdict
        || action.laterality_report_sha256 != report.laterality_report_sha256
        || action.footwear_verdict != report.footwear_verdict
        || action.footwear_report_sha256 != report.footwear_report_sha256
        || action.recommended_retry_frames != report.recommended_retry_frames
        || action.retried_frames != report.retried_frames
        || action.frames.len() != report.frames.len()
    {
        return invalid("Provider manifest action does not match the Action Report");
    }
    for (manifest_frame, frame) in action.frames.iter().zip(&report.frames) {
        if manifest_frame.frame_index != frame.frame_index
            || manifest_frame.phase != frame.phase
            || manifest_frame.attempt != frame.attempt
            || manifest_frame.generation_method != frame.generation_method
            || manifest_frame.sha256 != frame.sha256
            || manifest_frame.provider_request_occurred != frame.provider_request_occurred
            || manifest_frame.input_frame_sha256 != frame.input_frame_sha256
            || manifest_frame.replaces_frame_sha256 != frame.replaces_frame_sha256
            || Some(&manifest_frame.pose_structure_path) != frame.pose_structure_path.as_ref()
            || Some(manifest_frame.pose_structure_sha256.as_str())
                != frame.pose_structure_sha256.as_deref()
            || manifest_frame.pose_structure_profile != frame.pose_structure_profile
        {
            return invalid("Provider manifest frame does not match the Action Report");
        }
    }
    Ok(())
}

fn validate_v93_platform_graph(
    graph: &WorkflowGraphV1,
    job: &JobRecord,
    report: &GridKeyframeActionReportV1,
    manifest: &GridKeyframeProviderManifestV1,
) -> Result<(), GridRetrySourceError> {
    if graph.workflow != "topdown-grid@9.3.0"
        || graph.job_id != job.job_id
        || graph.parent_job_id != job.parent_job_id
    {
        return invalid("V9.3 WorkflowGraph does not match its Job lineage/workflow");
    }
    for frame in &report.frames {
        let id = format!("independent_keyframe:walk_down:{}", frame.frame_index);
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == id)
            .ok_or_else(|| GridRetrySourceError::Invalid(format!("missing graph node {id}")))?;
        let expected_implementation = if frame.frame_index == 2 {
            "grid-structured-keyframe@1.2.0"
        } else {
            "grid-byte-reuse@1.1.0"
        };
        if node.implementation_version != expected_implementation
            || node.provider_request != frame.provider_request_occurred
            || node.cache_hit != frame.reused
            || node.outputs
                != [WorkflowArtifactV1 {
                    sha256: frame.sha256.clone(),
                    path: frame.path.clone(),
                }]
            || node.inputs.last()
                != Some(&WorkflowArtifactV1 {
                    sha256: frame
                        .pose_structure_sha256
                        .clone()
                        .expect("validated pose hash"),
                    path: frame
                        .pose_structure_path
                        .clone()
                        .expect("validated pose path"),
                })
        {
            return invalid("V9.3 WorkflowGraph frame evidence is inconsistent");
        }
        if frame.frame_index == 2
            && (node.provider_id.as_deref() != Some(manifest.provider_id.as_str())
                || node.model.as_deref() != Some(manifest.image_model.as_str())
                || node.inputs.len() != 2
                || node.inputs[0].sha256 != report.direction_anchor_sha256)
        {
            return invalid("V9.3 frame-2 Provider provenance is inconsistent");
        }
    }
    let quality = graph
        .nodes
        .iter()
        .find(|node| node.id == "action_quality:walk_down")
        .ok_or_else(|| GridRetrySourceError::Invalid("missing V9.3 action quality node".into()))?;
    if quality.provider_request
        || quality.inputs
            != report
                .frames
                .iter()
                .map(|frame| WorkflowArtifactV1 {
                    sha256: frame.sha256.clone(),
                    path: frame.path.clone(),
                })
                .collect::<Vec<_>>()
    {
        return invalid("V9.3 action-quality input closure is inconsistent");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_graph(
    graph: &WorkflowGraphV1,
    job: &JobRecord,
    report: &GridKeyframeActionReportV1,
    action_path: &Path,
    motion_path: &Path,
    equipment_path: &Path,
    laterality_path: &Path,
    manifest: &GridKeyframeProviderManifestV1,
    canonical_source: &Path,
) -> Result<(), GridRetrySourceError> {
    if graph.workflow != SOURCE_WORKFLOW
        || graph.job_id != job.job_id
        || graph.parent_job_id != job.parent_job_id
    {
        return invalid("WorkflowGraph does not match the source Job lineage/workflow");
    }
    for frame in &report.frames {
        let node_id = format!("independent_keyframe:walk_down:{}", frame.frame_index);
        let mut nodes = graph.nodes.iter().filter(|node| node.id == node_id);
        let node = nodes.next().ok_or_else(|| {
            GridRetrySourceError::Invalid(format!("missing graph node {node_id}"))
        })?;
        if nodes.next().is_some()
            || node.item.as_deref() != Some("walk_down")
            || node.frame != Some(frame.frame_index)
            || !matches!(
                node.implementation_version.as_str(),
                "grid-structured-keyframe@1.0.0" | "grid-structured-keyframe@1.1.0"
            )
            || node.provider_request != frame.provider_request_occurred
            || node.cache_hit != frame.reused
            || node.outputs
                != [WorkflowArtifactV1 {
                    sha256: frame.sha256.clone(),
                    path: frame.path.clone(),
                }]
        {
            return invalid("WorkflowGraph frame node does not match the Action Report");
        }
        if node.provider_request
            && (node.provider_id.as_deref() != Some(manifest.provider_id.as_str())
                || node.model.as_deref() != Some(manifest.image_model.as_str()))
        {
            return invalid("WorkflowGraph Provider node does not match the Provider manifest");
        }
        for input in &node.inputs {
            validate_file(&input.path, &input.sha256, canonical_source)?;
        }
        let mut expected = Vec::new();
        if let Some(input_sha256) = &frame.input_frame_sha256 {
            let input = node.inputs.get(expected.len()).ok_or_else(|| {
                GridRetrySourceError::Invalid("WorkflowGraph is missing EditTarget input".into())
            })?;
            if &input.sha256 != input_sha256 {
                return invalid("WorkflowGraph EditTarget hash does not match the Action Report");
            }
            expected.push(input.clone());
        }
        let anchor = node.inputs.get(expected.len()).ok_or_else(|| {
            GridRetrySourceError::Invalid("WorkflowGraph is missing DirectionAnchor input".into())
        })?;
        if anchor.sha256 != report.direction_anchor_sha256 {
            return invalid("WorkflowGraph DirectionAnchor hash does not match the Action Report");
        }
        expected.push(anchor.clone());
        if let Some(replaces_sha256) = &frame.replaces_frame_sha256 {
            let replaced = node.inputs.get(expected.len()).ok_or_else(|| {
                GridRetrySourceError::Invalid("WorkflowGraph is missing replacement input".into())
            })?;
            if &replaced.sha256 != replaces_sha256 {
                return invalid("WorkflowGraph replacement hash does not match the Action Report");
            }
            expected.push(replaced.clone());
        }
        expected.push(WorkflowArtifactV1 {
            sha256: frame
                .pose_structure_sha256
                .clone()
                .expect("validated pose hash"),
            path: frame
                .pose_structure_path
                .clone()
                .expect("validated pose path"),
        });
        if node.inputs != expected {
            return invalid("WorkflowGraph frame input order/closure is inconsistent");
        }
    }

    let mut quality_nodes = graph
        .nodes
        .iter()
        .filter(|node| node.id == "action_quality:walk_down");
    let quality = quality_nodes.next().ok_or_else(|| {
        GridRetrySourceError::Invalid("WorkflowGraph is missing action quality node".into())
    })?;
    if quality_nodes.next().is_some()
        || quality.provider_request
        || quality.inputs
            != report
                .frames
                .iter()
                .map(|frame| WorkflowArtifactV1 {
                    sha256: frame.sha256.clone(),
                    path: frame.path.clone(),
                })
                .collect::<Vec<_>>()
        || quality.outputs
            != [
                WorkflowArtifactV1 {
                    sha256: hash_file(action_path)?,
                    path: action_path.to_path_buf(),
                },
                WorkflowArtifactV1 {
                    sha256: report.motion_report_sha256.clone(),
                    path: motion_path.to_path_buf(),
                },
                WorkflowArtifactV1 {
                    sha256: report.equipment_report_sha256.clone(),
                    path: equipment_path.to_path_buf(),
                },
                WorkflowArtifactV1 {
                    sha256: report
                        .laterality_report_sha256
                        .clone()
                        .expect("validated laterality hash"),
                    path: laterality_path.to_path_buf(),
                },
            ]
    {
        return invalid("WorkflowGraph action-quality closure does not match the reports");
    }
    Ok(())
}

fn validate_bound_artifact(
    job: &JobRecord,
    kind: &str,
    path: &Path,
    canonical_source: &Path,
) -> Result<(), GridRetrySourceError> {
    let sha256 = hash_file(path)?;
    if !job.artifacts.iter().any(|artifact| {
        artifact.kind == kind
            && artifact.path == path
            && artifact.sha256.as_deref() == Some(sha256.as_str())
    }) {
        return invalid(format!(
            "Job artifact {kind} is missing or has a stale hash"
        ));
    }
    validate_file(path, &sha256, canonical_source)
}

fn validate_file(
    path: &Path,
    expected_sha256: &str,
    canonical_source: &Path,
) -> Result<(), GridRetrySourceError> {
    let canonical = fs::canonicalize(path)?;
    if !canonical.starts_with(canonical_source) {
        return invalid(format!(
            "source artifact escaped its Job: {}",
            path.display()
        ));
    }
    if hash_file(&canonical)? != expected_sha256 {
        if path.file_name().and_then(|name| name.to_str()) == Some("pose-structure.png") {
            return invalid("structured PoseStructure 2 changed after assessment");
        }
        return invalid(format!("source artifact changed: {}", path.display()));
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, GridRetrySourceError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

fn hash_file(path: &Path) -> Result<String, GridRetrySourceError> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

fn hash_serialized(value: &impl serde::Serialize) -> Result<String, GridRetrySourceError> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(value)?)))
}

fn invalid<T>(message: impl Into<String>) -> Result<T, GridRetrySourceError> {
    Err(GridRetrySourceError::Invalid(message.into()))
}
