//! Stage 2 project build orchestrator: executes a [`ProjectBuildPlanV1`] by
//! dispatching one automation child job per `build`/`rebuild` action, in
//! topological order, and registering each produced pack into the V2 project
//! catalog with full game-art provenance.
//!
//! Design notes:
//!
//! - The parent build job makes **zero** provider calls itself. Every child
//!   runs through [`run_operation_with_provider`], so the real-provider cost
//!   guard (`FORGE_REAL_PROVIDER_*`, enforced inside the provider) stays in
//!   force for exactly the operations that spend money.
//! - `build-state.json` in the parent job directory is the crash-recovery
//!   record: it is rewritten atomically after every status transition, and a
//!   resumed run skips entries already marked `succeeded` for the same
//!   `specSha256` without starting a new child job.
//! - Cancellation is cooperative: the parent's `cancellation_requested` flag
//!   is checked before each child starts, and a child that lands in
//!   `Cancelled` stops the loop. [`crate::job::JobStore::request_cancellation_cascade`]
//!   propagates the flag to already-running children.
//! - [`reconcile_interrupted_builds`] marks build jobs whose recorded worker
//!   process is gone as failed/recoverable (`worker_lost`) so a crashed
//!   worker never leaves a job `Running` forever.
//!
//! [`ProjectBuildPlanV1`]: super::ProjectBuildPlanV1

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::diff::{hash_pack, reasons};
use super::plan::{
    PlanActionKindV1, ProjectBuildPlanV1, ProjectPlanActionV1, ProviderCapabilityInput,
};
use super::types::{AssetKind, GameArtError, GameArtManifestV1};
use super::{compute_build_plan, compute_project_diff};
use crate::asset_project::{
    resolve_relative, CharacterAssetSpecV1, StaticAssetSetSpecV1, STYLE_LOCK_FILE,
};
use crate::automation::{
    run_operation_with_provider, stage_plan_job, AutomationOperation, AutomationRunError,
    BuildProjectRequestV1, GenerateCharacterPackRequest, GenerateStaticAssetSetRequest, PlanStore,
};
use crate::catalog::{
    read_project_catalog, register_catalog_asset_v2, CatalogDependencyRefV1,
    CatalogLockRevisionsV1, CatalogProviderRefV1, CatalogStyleRefV1, ProjectCatalogEntryV2,
};
use crate::job::{JobLifecycleState, JobOperationKind, JobRecord, JobState, JobStore};
use crate::provider::{MediaGenerationProvider, ProviderError, ProviderUsage};

/// On-disk discriminator stored in the report `kind` field.
pub const PROJECT_BUILD_REPORT_KIND: &str = "project_build_report";
/// Only schema version emitted by this stage.
pub const PROJECT_BUILD_REPORT_SCHEMA_VERSION: &str = "1";
/// Report artifact file written into the parent job directory.
pub const PROJECT_BUILD_REPORT_FILE: &str = "project-build-report.json";
/// Crash-recovery state file rewritten after every asset transition.
pub const BUILD_STATE_FILE: &str = "build-state.json";
/// Only schema version emitted for [`ProjectBuildStateV1`].
pub const BUILD_STATE_SCHEMA_VERSION: &str = "1";
/// Artifact kind used for the report on the parent job record.
pub const PROJECT_BUILD_REPORT_ARTIFACT_KIND: &str = "project_build_report";

/// Lifecycle status of one asset entry inside `build-state.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildAssetStatusV1 {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

/// Final per-asset outcome recorded in [`ProjectBuildReportV1`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildResultStatusV1 {
    /// Catalog entry was fresh; no provider work happened.
    Reused,
    /// A child job finished `Succeeded` (or a resumed build-state entry was
    /// carried over) and the pack is catalog-registered.
    Succeeded,
    /// The child job did not succeed (provider/processing error or a
    /// consistency gate that left the job awaiting review).
    Failed,
    /// Never attempted: a dependency failed or the build was cancelled.
    Skipped,
}

/// One asset entry in `build-state.json`. `packPath`/`packSha256` are filled
/// on success so a resumed run can re-register the catalog entry without
/// re-running the child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildStateAssetV1 {
    pub asset_id: String,
    pub action: PlanActionKindV1,
    pub spec_sha256: String,
    pub status: BuildAssetStatusV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Crash-recovery record persisted in the parent job directory. Rewritten
/// atomically after every transition; reused when the same parent job is
/// retried with the same manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBuildStateV1 {
    pub schema_version: String,
    pub manifest_sha256: String,
    pub plan_sha256: String,
    #[serde(default)]
    pub assets: Vec<BuildStateAssetV1>,
}

/// Per-asset build outcome inside [`ProjectBuildReportV1`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBuildAssetResultV1 {
    pub asset_id: String,
    pub kind: AssetKind,
    pub action: PlanActionKindV1,
    pub status: BuildResultStatusV1,
    /// Diff reason codes for build/rebuild actions; skip reasons
    /// (`dependency_failed`, `cancelled`) for skipped outcomes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregate counters of one build run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBuildSummaryV1 {
    pub built: u32,
    pub reused: u32,
    pub failed: u32,
    pub skipped: u32,
    pub orphans: u32,
}

/// Final project build report written to the parent job directory and
/// appended to the parent job record as a `project_build_report` artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBuildReportV1 {
    pub schema_version: String,
    pub kind: String,
    pub manifest_sha256: String,
    pub plan_sha256: String,
    pub plan: ProjectBuildPlanV1,
    pub results: Vec<ProjectBuildAssetResultV1>,
    pub summary: ProjectBuildSummaryV1,
    /// Provider usage aggregated from every child's `provider-usage.json`.
    pub provider_usage: ProviderUsage,
}

/// Execute the stage 2 project build for the parent job `parent_job_id`.
///
/// Loads and validates the manifest, computes the diff and build plan, then
/// runs one automation child job per build/rebuild action in topological
/// order and registers each pack into the project catalog. Returns the
/// persisted report. Errors:
///
/// - [`AutomationRunError::GameArt`] for manifest/diff/plan level failures;
/// - [`AutomationRunError::Cancelled`] when the parent cancellation flag is
///   observed (the runner's cancelled path marks the parent `Cancelled`);
/// - [`AutomationRunError::ProjectBuildFailed`] when any *required* asset
///   failed or was skipped — the report is still written first.
pub fn run_build_project(
    store: &JobStore,
    plans: &PlanStore,
    parent_job_id: &str,
    request: &BuildProjectRequestV1,
    provider: Option<&dyn MediaGenerationProvider>,
) -> Result<ProjectBuildReportV1, AutomationRunError> {
    // A crashed worker must never leave sibling build jobs `Running` forever.
    // The current parent survives: its worker pid is this live process (or
    // unset when called directly), and reconciliation skips both cases.
    reconcile_interrupted_builds(store)?;

    let project_root = request.project_path.clone();
    step(store, parent_job_id, "validate_manifest", "running", 0.05)?;
    let validated = GameArtManifestV1::load_validated(&request.manifest_path)?;
    step(store, parent_job_id, "validate_manifest", "succeeded", 0.1)?;

    step(store, parent_job_id, "diff_catalog", "running", 0.12)?;
    let diff = compute_project_diff(&project_root, &validated)?;
    let capabilities = provider_capability_input(provider);
    let plan = compute_build_plan(&project_root, &validated, &diff, &capabilities)?;
    let manifest_sha256 = plan.manifest_sha256.clone();
    let plan_sha256 = plan.plan_sha256();
    step(store, parent_job_id, "diff_catalog", "succeeded", 0.18)?;

    // Child requests need a Style Lock to build against; the diff resolved
    // the effective revision (manifest pin or project current).
    let style_revision = diff.style_revision.clone();
    let build_required = plan
        .actions
        .iter()
        .any(|action| action.action != PlanActionKindV1::Reuse);
    if build_required && provider.is_none() {
        return Err(AutomationRunError::Processing(format!(
            "provider {} must be resolved before running this plan",
            validated.manifest.provider.id
        )));
    }
    let style_lock_path = if build_required {
        let revision = style_revision.as_deref().ok_or_else(|| {
            GameArtError::InvalidManifest(
                "project has no style revision; run `forge style create` before building".into(),
            )
        })?;
        Some(
            project_root
                .join(".forge/styles")
                .join(revision)
                .join(STYLE_LOCK_FILE),
        )
    } else {
        None
    };

    // (b) Build-state: resume a matching record, otherwise start fresh.
    // Entries carried over keep their terminal info only while the spec hash
    // still matches; anything else goes back to `pending`.
    let parent = store.read_record(parent_job_id)?;
    let state_path = parent.job_dir.join(BUILD_STATE_FILE);
    let mut state = resume_or_fresh_state(&state_path, &plan, &manifest_sha256, &plan_sha256);
    write_build_state(&state_path, &state)?;

    let catalog = read_project_catalog(&project_root).map_err(|error| {
        AutomationRunError::Processing(format!("invalid project catalog: {error}"))
    })?;
    let required: BTreeSet<&str> = validated
        .manifest
        .assets
        .iter()
        .filter(|asset| asset.required)
        .map(|asset| asset.id.as_str())
        .collect();

    let mut results: Vec<ProjectBuildAssetResultV1> = Vec::with_capacity(plan.actions.len());
    let mut poisoned: BTreeSet<String> = BTreeSet::new();
    let mut cancelled = false;
    let build_total = plan
        .actions
        .iter()
        .filter(|action| action.action != PlanActionKindV1::Reuse)
        .count();
    let mut build_done = 0usize;

    step(store, parent_job_id, "run_child_builds", "running", 0.2)?;
    for action in &plan.actions {
        let progress = 0.2 + 0.65 * (build_done as f32 / build_total.max(1) as f32);
        if action.action == PlanActionKindV1::Reuse {
            let (pack_path, pack_sha256) = catalog
                .assets
                .get(&action.asset_id)
                .map(|entry| {
                    (
                        Some(entry.pack_path.clone()),
                        Some(entry.pack_sha256.clone()),
                    )
                })
                .unwrap_or((None, None));
            results.push(ProjectBuildAssetResultV1 {
                asset_id: action.asset_id.clone(),
                kind: action.kind,
                action: action.action,
                status: BuildResultStatusV1::Reused,
                reasons: Vec::new(),
                child_job_id: None,
                pack_path,
                pack_sha256,
                error: None,
            });
            continue;
        }

        // Resume case: a prior run of THIS parent already built the asset for
        // the same spec hash and the pack is still intact — skip the child.
        // Never skip a `dependency_rebuilt` rebuild: the dependency's new
        // output must flow into a fresh build of this asset.
        if let Some(entry) = state
            .assets
            .iter()
            .find(|entry| entry.asset_id == action.asset_id)
        {
            let dependency_rebuilt = action
                .reasons
                .iter()
                .any(|reason| reason == reasons::DEPENDENCY_REBUILT);
            if !dependency_rebuilt
                && entry.status == BuildAssetStatusV1::Succeeded
                && entry.spec_sha256 == action.spec_sha256
                && pack_intact(entry)
            {
                let meta = parse_spec_meta(action)?;
                register_built_asset(
                    &project_root,
                    &validated,
                    action,
                    &meta,
                    &style_revision,
                    entry.pack_path.as_ref().expect("intact pack has a path"),
                    entry
                        .pack_sha256
                        .as_deref()
                        .expect("intact pack has a hash"),
                    entry.child_job_id.as_deref().unwrap_or(parent_job_id),
                    parent_job_id,
                    child_quality(store, entry.child_job_id.as_deref()),
                    provider.expect("provider presence checked when builds are required"),
                )?;
                results.push(ProjectBuildAssetResultV1 {
                    asset_id: action.asset_id.clone(),
                    kind: action.kind,
                    action: action.action,
                    status: BuildResultStatusV1::Succeeded,
                    reasons: action.reasons.clone(),
                    child_job_id: entry.child_job_id.clone(),
                    pack_path: entry.pack_path.clone(),
                    pack_sha256: entry.pack_sha256.clone(),
                    error: None,
                });
                build_done += 1;
                continue;
            }
        }

        // A failed or skipped dependency poisons its transitive dependents;
        // topological order guarantees the dependency outcome is already known.
        if action
            .depends_on_assets
            .iter()
            .any(|dependency| poisoned.contains(dependency))
        {
            transition(
                &state_path,
                &mut state,
                &action.asset_id,
                BuildAssetStatusV1::Skipped,
                None,
                Some("dependency_failed".to_string()),
            )?;
            poisoned.insert(action.asset_id.clone());
            results.push(ProjectBuildAssetResultV1 {
                asset_id: action.asset_id.clone(),
                kind: action.kind,
                action: action.action,
                status: BuildResultStatusV1::Skipped,
                reasons: vec!["dependency_failed".to_string()],
                child_job_id: None,
                pack_path: None,
                pack_sha256: None,
                error: Some("dependency_failed".to_string()),
            });
            build_done += 1;
            continue;
        }

        // Cooperative cancellation: check the parent flag before each child.
        if store.read_record(parent_job_id)?.cancellation_requested {
            cancelled = true;
        }
        if cancelled {
            transition(
                &state_path,
                &mut state,
                &action.asset_id,
                BuildAssetStatusV1::Skipped,
                None,
                Some("cancelled".to_string()),
            )?;
            poisoned.insert(action.asset_id.clone());
            results.push(ProjectBuildAssetResultV1 {
                asset_id: action.asset_id.clone(),
                kind: action.kind,
                action: action.action,
                status: BuildResultStatusV1::Skipped,
                reasons: vec!["cancelled".to_string()],
                child_job_id: None,
                pack_path: None,
                pack_sha256: None,
                error: Some("cancelled".to_string()),
            });
            build_done += 1;
            continue;
        }

        let provider = provider.expect("provider presence checked when builds are required");
        let style_lock_path = style_lock_path
            .clone()
            .expect("style lock path exists when builds are required");
        let meta = parse_spec_meta(action)?;
        let operation = child_operation(
            &project_root,
            &validated.manifest,
            action,
            &meta,
            &style_lock_path,
            provider,
        )?;

        step(
            store,
            parent_job_id,
            "run_child_builds",
            "running",
            progress,
        )?;
        build_done += 1;
        let child_record = match execute_child(
            store,
            plans,
            parent_job_id,
            &state_path,
            &mut state,
            action,
            operation,
            provider,
        ) {
            Ok(record) => record,
            Err(
                AutomationRunError::Cancelled
                | AutomationRunError::Provider(ProviderError::Cancelled),
            ) => {
                cancelled = true;
                let child_id = state_child_id(&state, &action.asset_id);
                transition(
                    &state_path,
                    &mut state,
                    &action.asset_id,
                    BuildAssetStatusV1::Skipped,
                    None,
                    Some("cancelled".to_string()),
                )?;
                poisoned.insert(action.asset_id.clone());
                results.push(ProjectBuildAssetResultV1 {
                    asset_id: action.asset_id.clone(),
                    kind: action.kind,
                    action: action.action,
                    status: BuildResultStatusV1::Skipped,
                    reasons: vec!["cancelled".to_string()],
                    child_job_id: child_id,
                    pack_path: None,
                    pack_sha256: None,
                    error: Some("cancelled".to_string()),
                });
                continue;
            }
            Err(error) => {
                // Hard child failure (plan/store/provider error): record the
                // asset as failed and keep building unrelated assets instead
                // of aborting the whole project.
                let error_text = format!("{}: {error}", error.code());
                let child_id = state_child_id(&state, &action.asset_id);
                transition(
                    &state_path,
                    &mut state,
                    &action.asset_id,
                    BuildAssetStatusV1::Failed,
                    None,
                    Some(error_text.clone()),
                )?;
                poisoned.insert(action.asset_id.clone());
                results.push(ProjectBuildAssetResultV1 {
                    asset_id: action.asset_id.clone(),
                    kind: action.kind,
                    action: action.action,
                    status: BuildResultStatusV1::Failed,
                    reasons: action.reasons.clone(),
                    child_job_id: child_id,
                    pack_path: None,
                    pack_sha256: None,
                    error: Some(error_text),
                });
                continue;
            }
        };
        match child_record.lifecycle_state {
            JobLifecycleState::Succeeded => {
                let pack = child_record
                    .artifacts
                    .iter()
                    .find(|artifact| artifact.kind == "gsfpack")
                    .ok_or_else(|| {
                        AutomationRunError::Processing(format!(
                            "child job {} succeeded without a pack artifact",
                            child_record.job_id
                        ))
                    })?;
                let pack_path = pack.path.clone();
                let pack_sha256 = hash_pack(&pack_path, true)?;
                let quality = child_quality(store, Some(child_record.job_id.as_str()));
                register_built_asset(
                    &project_root,
                    &validated,
                    action,
                    &meta,
                    &style_revision,
                    &pack_path,
                    &pack_sha256,
                    &child_record.job_id,
                    parent_job_id,
                    quality,
                    provider,
                )?;
                let entry = state
                    .assets
                    .iter_mut()
                    .find(|entry| entry.asset_id == action.asset_id)
                    .expect("state covers every plan action");
                entry.status = BuildAssetStatusV1::Succeeded;
                entry.child_job_id = Some(child_record.job_id.clone());
                entry.pack_path = Some(pack_path.clone());
                entry.pack_sha256 = Some(pack_sha256.clone());
                entry.error = None;
                write_build_state(&state_path, &state)?;
                results.push(ProjectBuildAssetResultV1 {
                    asset_id: action.asset_id.clone(),
                    kind: action.kind,
                    action: action.action,
                    status: BuildResultStatusV1::Succeeded,
                    reasons: action.reasons.clone(),
                    child_job_id: Some(child_record.job_id.clone()),
                    pack_path: Some(pack_path),
                    pack_sha256: Some(pack_sha256),
                    error: None,
                });
            }
            JobLifecycleState::Cancelled => {
                cancelled = true;
                transition(
                    &state_path,
                    &mut state,
                    &action.asset_id,
                    BuildAssetStatusV1::Skipped,
                    Some(child_record.job_id.clone()),
                    Some("cancelled".to_string()),
                )?;
                poisoned.insert(action.asset_id.clone());
                results.push(ProjectBuildAssetResultV1 {
                    asset_id: action.asset_id.clone(),
                    kind: action.kind,
                    action: action.action,
                    status: BuildResultStatusV1::Skipped,
                    reasons: vec!["cancelled".to_string()],
                    child_job_id: Some(child_record.job_id.clone()),
                    pack_path: None,
                    pack_sha256: None,
                    error: Some("cancelled".to_string()),
                });
            }
            _ => {
                let error = child_error(&child_record);
                transition(
                    &state_path,
                    &mut state,
                    &action.asset_id,
                    BuildAssetStatusV1::Failed,
                    Some(child_record.job_id.clone()),
                    Some(error.clone()),
                )?;
                poisoned.insert(action.asset_id.clone());
                results.push(ProjectBuildAssetResultV1 {
                    asset_id: action.asset_id.clone(),
                    kind: action.kind,
                    action: action.action,
                    status: BuildResultStatusV1::Failed,
                    reasons: action.reasons.clone(),
                    child_job_id: Some(child_record.job_id.clone()),
                    pack_path: None,
                    pack_sha256: None,
                    error: Some(error),
                });
            }
        }
    }
    step(store, parent_job_id, "run_child_builds", "succeeded", 0.85)?;
    step(store, parent_job_id, "update_catalog", "succeeded", 0.9)?;

    // (e) Final report: always written, even when the build failed or was
    // cancelled, so callers have the full per-asset evidence.
    step(store, parent_job_id, "summarize", "running", 0.95)?;
    let mut summary = ProjectBuildSummaryV1 {
        orphans: plan.delete_candidates.len() as u32,
        ..ProjectBuildSummaryV1::default()
    };
    for result in &results {
        match result.status {
            BuildResultStatusV1::Reused => summary.reused += 1,
            BuildResultStatusV1::Succeeded => summary.built += 1,
            BuildResultStatusV1::Failed => summary.failed += 1,
            BuildResultStatusV1::Skipped => summary.skipped += 1,
        }
    }
    let report = ProjectBuildReportV1 {
        schema_version: PROJECT_BUILD_REPORT_SCHEMA_VERSION.to_string(),
        kind: PROJECT_BUILD_REPORT_KIND.to_string(),
        manifest_sha256,
        plan_sha256,
        plan,
        results,
        summary,
        provider_usage: aggregate_provider_usage(store, &state),
    };
    let report_path = parent.job_dir.join(PROJECT_BUILD_REPORT_FILE);
    write_json_atomic(&report_path, &report)?;
    let report_sha256 = format!("{:x}", Sha256::digest(fs::read(&report_path)?));
    store.update_record(parent_job_id, |record| {
        record
            .artifacts
            .retain(|artifact| artifact.kind != PROJECT_BUILD_REPORT_ARTIFACT_KIND);
        record.artifacts.push(crate::job::JobArtifactRecord {
            kind: PROJECT_BUILD_REPORT_ARTIFACT_KIND.into(),
            path: report_path.clone(),
            sha256: Some(report_sha256.clone()),
        });
    })?;
    step(store, parent_job_id, "summarize", "succeeded", 0.98)?;

    if cancelled {
        return Err(AutomationRunError::Cancelled);
    }
    let required_failure = report
        .results
        .iter()
        .filter(|result| {
            matches!(
                result.status,
                BuildResultStatusV1::Failed | BuildResultStatusV1::Skipped
            ) && required.contains(result.asset_id.as_str())
        })
        .map(|result| result.asset_id.clone())
        .collect::<Vec<_>>();
    if !required_failure.is_empty() {
        return Err(AutomationRunError::ProjectBuildFailed(format!(
            "required assets failed or were skipped: {}",
            required_failure.join(", ")
        )));
    }
    store.update_record(parent_job_id, |record| {
        record.state = JobState::Exported;
        record.lifecycle_state = JobLifecycleState::Succeeded;
        record.progress = 1.0;
        record.worker_pid = None;
        record.next_actions = vec!["job_report".into()];
    })?;
    Ok(report)
}

/// Mark every `build_project` job stuck in `Running`/`Queued` whose recorded
/// worker process no longer exists as failed/recoverable with the stable
/// `worker_lost` code. Idempotent; safe to call before every build. Returns
/// the number of jobs reconciled.
pub fn reconcile_interrupted_builds(store: &JobStore) -> Result<usize, AutomationRunError> {
    let mut reconciled = 0;
    for record in store.list_records()? {
        if record.operation_kind != JobOperationKind::BuildProject {
            continue;
        }
        if !matches!(
            record.lifecycle_state,
            JobLifecycleState::Running | JobLifecycleState::Queued
        ) {
            continue;
        }
        let Some(pid) = record.worker_pid else {
            continue;
        };
        if pid == std::process::id() || pid_alive(pid) {
            continue;
        }
        store.update_record(&record.job_id, |record| {
            record.state = JobState::Failed;
            record.lifecycle_state = JobLifecycleState::Failed;
            record.worker_pid = None;
            record.recoverable = true;
            record.error_code = Some("worker_lost".into());
            record.error_summary = Some(format!("build worker process {pid} is no longer running"));
            record.next_actions = vec!["prepare_new_plan".into(), "job_report".into()];
        })?;
        reconciled += 1;
    }
    Ok(reconciled)
}

/// Process liveness probe. Core has no libc dependency, so on Unix this
/// shells out to `kill -0`; a failed probe errs on the side of "alive" so a
/// probe problem never kills a healthy build. Non-Unix platforms assume alive
/// (stage 2 ships macOS-first).
fn pid_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        true
    }
}

/// Facts parsed out of an asset spec file, needed to build the child
/// operation and the catalog entry. Specs are parsed once per asset with
/// their kind's `deny_unknown_fields` struct, and every embedded relative
/// reference image path is resolved against the spec's directory (mirroring
/// the CLI's `resolve_relative` handling).
enum SpecMeta {
    Character {
        name: String,
        license: String,
        prompt: String,
        reference_image: Option<PathBuf>,
    },
    StaticSet {
        spec: StaticAssetSetSpecV1,
    },
}

impl SpecMeta {
    fn name(&self) -> &str {
        match self {
            Self::Character { name, .. } => name,
            Self::StaticSet { spec } => &spec.name,
        }
    }

    fn license(&self) -> &str {
        match self {
            Self::Character { license, .. } => license,
            Self::StaticSet { spec } => &spec.license,
        }
    }
}

/// Parse the action's spec file and cross-check the embedded id/kind against
/// the manifest declaration, so a spec swapped for a different asset fails
/// with `invalid_manifest` instead of producing a mismatched child job.
fn parse_spec_meta(action: &ProjectPlanActionV1) -> Result<SpecMeta, AutomationRunError> {
    let bytes = fs::read(&action.canonical_spec_path)?;
    let spec_dir = action
        .canonical_spec_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let id_mismatch = |found: &str| {
        GameArtError::InvalidManifest(format!(
            "spec {} id \"{found}\" does not match manifest asset \"{}\"",
            action.canonical_spec_path.display(),
            action.asset_id
        ))
    };
    let kind_mismatch = |found: &str| {
        GameArtError::InvalidManifest(format!(
            "spec {} declares kind \"{found}\", manifest declares \"{}\"",
            action.canonical_spec_path.display(),
            action.kind
        ))
    };
    match action.kind {
        AssetKind::Character => {
            let spec: CharacterAssetSpecV1 = serde_json::from_slice(&bytes).map_err(|error| {
                GameArtError::InvalidJson(format!(
                    "character spec {}: {error}",
                    action.canonical_spec_path.display()
                ))
            })?;
            if spec.kind != AssetKind::Character.as_str() {
                return Err(kind_mismatch(&spec.kind).into());
            }
            if spec.id != action.asset_id {
                return Err(id_mismatch(&spec.id).into());
            }
            Ok(SpecMeta::Character {
                name: spec.name,
                license: spec.license,
                prompt: spec.prompt,
                reference_image: spec
                    .reference_image
                    .map(|path| resolve_relative(&spec_dir, &path)),
            })
        }
        AssetKind::IconSet | AssetKind::PropSet => {
            let mut spec: StaticAssetSetSpecV1 =
                serde_json::from_slice(&bytes).map_err(|error| {
                    GameArtError::InvalidJson(format!(
                        "static asset set spec {}: {error}",
                        action.canonical_spec_path.display()
                    ))
                })?;
            if spec.kind.as_str() != action.kind.as_str() {
                return Err(kind_mismatch(spec.kind.as_str()).into());
            }
            if spec.id != action.asset_id {
                return Err(id_mismatch(&spec.id).into());
            }
            for item in &mut spec.items {
                if let Some(reference) = item.reference_image.take() {
                    item.reference_image = Some(resolve_relative(&spec_dir, &reference));
                }
            }
            Ok(SpecMeta::StaticSet { spec })
        }
    }
}

/// Construct the child automation operation for one build/rebuild action,
/// mirroring the CLI's request construction in `main.rs` field-for-field:
/// characters go through `GenerateCharacterPackRequest` (schemaVersion "3",
/// V1 spec, video workflow `topdown@1.0.0`, no `subjectLockPath`), icon/prop
/// sets through `GenerateStaticAssetSetRequest` (schemaVersion "4"). Model
/// pins are resolved from the provider, matching `resolve_operation_models`.
fn child_operation(
    project_root: &Path,
    manifest: &GameArtManifestV1,
    action: &ProjectPlanActionV1,
    meta: &SpecMeta,
    style_lock_path: &Path,
    provider: &dyn MediaGenerationProvider,
) -> Result<AutomationOperation, AutomationRunError> {
    match meta {
        SpecMeta::Character {
            name,
            license,
            prompt,
            reference_image,
        } => {
            let request: GenerateCharacterPackRequest =
                serde_json::from_value(serde_json::json!({
                    "schemaVersion": "3",
                    "providerId": manifest.provider.id,
                    "profileId": manifest.provider.profile_id,
                    "projectPath": project_root,
                    "assetId": action.asset_id,
                    "styleLockPath": style_lock_path,
                    "character": {
                        "prompt": prompt,
                        "referenceImagePath": reference_image,
                    },
                    "metadata": {
                        "name": name,
                        "defaultAnimation": "idle",
                        "creator": "Game Sprite Forge",
                        "license": license,
                    },
                    "workflow": { "id": "topdown", "version": "1.0.0" },
                    "generation": {
                        "maxAttemptsPerAnimation": 2,
                        "targetFrameCount": 8,
                        "videoDurationSeconds": 4,
                        "imageModel": provider.resolved_image_model(None),
                        "videoModel": provider.resolved_video_model(None),
                    },
                    "quality": { "requireGameReady": true }
                }))?;
            Ok(AutomationOperation::GenerateCharacterPack(request))
        }
        SpecMeta::StaticSet { spec } => {
            let request = GenerateStaticAssetSetRequest {
                schema_version: "4".into(),
                project_path: project_root.to_path_buf(),
                style_lock_path: style_lock_path.to_path_buf(),
                provider_id: manifest.provider.id.clone(),
                profile_id: manifest.provider.profile_id.clone(),
                asset: spec.clone(),
                max_attempts_per_item: 2,
                image_model: provider.resolved_image_model(None),
                reuse_from_job_dir: None,
                retry_item_ids: vec![],
                consistency_recheck_only: false,
            };
            Ok(AutomationOperation::GenerateStaticAssetSet(request))
        }
    }
}

/// Quality evidence for the catalog entry, read back from the child's
/// `consistency-report.json` when one exists: `(profile, verdict, game_ready)`.
fn child_quality(
    store: &JobStore,
    child_job_id: Option<&str>,
) -> (Option<String>, Option<String>, Option<bool>) {
    let Some(child_job_id) = child_job_id else {
        return (None, None, None);
    };
    let Ok(record) = store.read_record(child_job_id) else {
        return (None, None, None);
    };
    let report_path = record.job_dir.join("consistency-report.json");
    let Ok(bytes) = fs::read(&report_path) else {
        return (None, None, None);
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return (None, None, None);
    };
    let profile = value
        .get("profile")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let verdict = value
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let game_ready = verdict.as_deref().map(|verdict| verdict == "game_ready");
    (profile, verdict, game_ready)
}

/// Human-readable error for a child job that did not succeed.
fn child_error(record: &JobRecord) -> String {
    let summary = record
        .error_summary
        .clone()
        .unwrap_or_else(|| format!("child job ended in {:?}", record.lifecycle_state));
    match &record.error_code {
        Some(code) => format!("{code}: {summary}"),
        None => summary,
    }
}

/// The child job id currently recorded for an asset in the build-state, if
/// any (used to keep failure results pointing at the staged child).
fn state_child_id(state: &ProjectBuildStateV1, asset_id: &str) -> Option<String> {
    state
        .assets
        .iter()
        .find(|entry| entry.asset_id == asset_id)
        .and_then(|entry| entry.child_job_id.clone())
}

/// Run one child build through the full single-use plan flow: prepare, claim,
/// stage, link to the parent, then execute via
/// [`run_operation_with_provider`] (keeping the real-provider cost guard in
/// force). Build-state transitions are persisted before each irreversible
/// step so a crash leaves an accurate record. A cancelled or failed child is
/// reported through its [`JobRecord`], not an `Err`.
#[allow(clippy::too_many_arguments)]
fn execute_child(
    store: &JobStore,
    plans: &PlanStore,
    parent_job_id: &str,
    state_path: &Path,
    state: &mut ProjectBuildStateV1,
    action: &ProjectPlanActionV1,
    operation: AutomationOperation,
    provider: &dyn MediaGenerationProvider,
) -> Result<JobRecord, AutomationRunError> {
    transition(
        state_path,
        state,
        &action.asset_id,
        BuildAssetStatusV1::Running,
        None,
        None,
    )?;
    let prepared = plans.prepare(operation)?;
    let claimed = plans.claim(&prepared.token)?;
    let child = stage_plan_job(store, &claimed)?;
    store.update_record(&child.job_id, |record| {
        record.parent_job_id = Some(parent_job_id.to_string());
    })?;
    transition(
        state_path,
        state,
        &action.asset_id,
        BuildAssetStatusV1::Running,
        Some(child.job_id.clone()),
        None,
    )?;
    run_operation_with_provider(store, &child.job_id, &claimed.operation, Some(provider))
}

/// Register (or re-register) the catalog V2 entry for a successfully built
/// asset, with the full stage 2 provenance: spec hash, dependencies, resolved
/// lock refs, workflow pin, provider/profile/model, pack path + content hash,
/// quality evidence and generation timestamp.
#[allow(clippy::too_many_arguments)]
fn register_built_asset(
    project_root: &Path,
    validated: &super::ValidatedManifest,
    action: &ProjectPlanActionV1,
    meta: &SpecMeta,
    style_revision: &Option<String>,
    pack_path: &Path,
    pack_sha256: &str,
    child_job_id: &str,
    parent_job_id: &str,
    quality: (Option<String>, Option<String>, Option<bool>),
    provider: &dyn MediaGenerationProvider,
) -> Result<(), AutomationRunError> {
    let manifest_asset = validated
        .manifest
        .assets
        .iter()
        .find(|asset| asset.id == action.asset_id)
        .ok_or_else(|| {
            GameArtError::InvalidManifest(format!(
                "plan action \"{}\" is not a declared manifest asset",
                action.asset_id
            ))
        })?;
    let mut dependencies: Vec<CatalogDependencyRefV1> = action
        .depends_on_assets
        .iter()
        .map(|id| CatalogDependencyRefV1 {
            id: id.clone(),
            revision: None,
            hash: None,
        })
        .collect();
    dependencies.extend(
        action
            .lock_refs
            .iter()
            .map(|reference| CatalogDependencyRefV1 {
                id: format!("{}:{}", reference.kind, reference.id),
                revision: Some(reference.revision.clone()),
                hash: None,
            }),
    );
    let (workflow_profile, workflow_version) = action
        .workflow
        .split_once('@')
        .map(|(id, version)| (Some(id.to_string()), Some(version.to_string())))
        .unwrap_or((None, None));
    let (quality_profile, quality_verdict, game_ready) = quality;
    let model = provider.resolved_image_model(None);
    let entry = ProjectCatalogEntryV2 {
        asset_id: action.asset_id.clone(),
        name: meta.name().to_string(),
        kind: action.kind.as_str().to_string(),
        pack_path: pack_path.to_path_buf(),
        pack_sha256: pack_sha256.to_string(),
        source_job_id: child_job_id.to_string(),
        parent_job_id: Some(parent_job_id.to_string()),
        style: style_revision.as_ref().map(|revision| CatalogStyleRefV1 {
            revision: revision.clone(),
        }),
        subject: None,
        workflow: action.workflow.clone(),
        provider: Some(CatalogProviderRefV1 {
            provider_id: validated.manifest.provider.id.clone(),
            profile_id: validated.manifest.provider.profile_id.clone(),
            model: model.clone(),
        }),
        installed: None,
        created_at: Utc::now(),
        spec_path: Some(manifest_asset.spec.clone()),
        spec_sha256: Some(action.spec_sha256.clone()),
        dependencies: Some(dependencies),
        locks: Some(CatalogLockRevisionsV1 {
            style: style_revision.clone(),
            ..CatalogLockRevisionsV1::default()
        }),
        workflow_profile,
        workflow_version,
        pack_version: None,
        quality_verdict,
        quality_profile,
        game_ready,
        generated_at: Some(Utc::now()),
        reviewed_at: None,
        license: Some(meta.license().to_string()),
        provenance_summary: Some(format!(
            "{} {} via {}",
            validated.manifest.provider.id,
            model.unwrap_or_else(|| "default-model".into()),
            action.workflow
        )),
    };
    register_catalog_asset_v2(project_root, entry)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    Ok(())
}

/// Fill the plan layer's provider input from the resolved provider (snake_case
/// capability names, matching the CLI's `provider_capability_input`); an
/// absent provider yields an empty descriptor (unmet capabilities are
/// reported by the plan, never fatal).
fn provider_capability_input(
    provider: Option<&dyn MediaGenerationProvider>,
) -> ProviderCapabilityInput {
    let Some(provider) = provider else {
        return ProviderCapabilityInput::default();
    };
    ProviderCapabilityInput {
        capabilities: provider
            .capabilities()
            .into_iter()
            .filter_map(|capability| {
                serde_json::to_value(capability)
                    .ok()?
                    .as_str()
                    .map(str::to_owned)
            })
            .collect(),
        image_model: provider.resolved_image_model(None),
        video_model: provider.resolved_video_model(None),
    }
}

/// Sum every child job's `provider-usage.json` delta into one total.
fn aggregate_provider_usage(store: &JobStore, state: &ProjectBuildStateV1) -> ProviderUsage {
    #[derive(Deserialize)]
    struct UsageFile {
        usage: ProviderUsage,
    }

    let mut total = ProviderUsage::default();
    let mut seen = BTreeSet::new();
    for entry in &state.assets {
        let Some(child_job_id) = &entry.child_job_id else {
            continue;
        };
        if !seen.insert(child_job_id.clone()) {
            continue;
        }
        let Ok(record) = store.read_record(child_job_id) else {
            continue;
        };
        let Ok(bytes) = fs::read(record.job_dir.join("provider-usage.json")) else {
            continue;
        };
        let Ok(file) = serde_json::from_slice::<UsageFile>(&bytes) else {
            continue;
        };
        total.requests += file.usage.requests;
        total.generated_images += file.usage.generated_images;
        total.generated_videos += file.usage.generated_videos;
        total.edited_videos += file.usage.edited_videos;
        total.private_file_uploads += file.usage.private_file_uploads;
        if let Some(ticks) = file.usage.cost_in_usd_ticks {
            *total.cost_in_usd_ticks.get_or_insert(0) += ticks;
        }
    }
    total
}

/// A resume entry is only trustworthy when the recorded pack still exists and
/// still hashes to the recorded digest.
fn pack_intact(entry: &BuildStateAssetV1) -> bool {
    let (Some(path), Some(recorded)) = (&entry.pack_path, &entry.pack_sha256) else {
        return false;
    };
    path.is_dir() && hash_pack(path, true).is_ok_and(|actual| &actual == recorded)
}

/// Load a prior build-state when it belongs to the same manifest, carrying
/// terminal info forward for entries whose spec hash is unchanged; otherwise
/// start every build/rebuild action `pending`.
fn resume_or_fresh_state(
    state_path: &Path,
    plan: &ProjectBuildPlanV1,
    manifest_sha256: &str,
    plan_sha256: &str,
) -> ProjectBuildStateV1 {
    let prior: Option<ProjectBuildStateV1> = fs::read(state_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .filter(|state: &ProjectBuildStateV1| state.manifest_sha256 == manifest_sha256);
    let mut assets = Vec::new();
    for action in &plan.actions {
        if action.action == PlanActionKindV1::Reuse {
            continue;
        }
        let carried = prior.as_ref().and_then(|state| {
            state.assets.iter().find(|entry| {
                entry.asset_id == action.asset_id && entry.spec_sha256 == action.spec_sha256
            })
        });
        assets.push(match carried {
            Some(entry) => BuildStateAssetV1 {
                asset_id: entry.asset_id.clone(),
                action: action.action,
                spec_sha256: entry.spec_sha256.clone(),
                status: entry.status,
                child_job_id: entry.child_job_id.clone(),
                pack_path: entry.pack_path.clone(),
                pack_sha256: entry.pack_sha256.clone(),
                error: entry.error.clone(),
            },
            None => BuildStateAssetV1 {
                asset_id: action.asset_id.clone(),
                action: action.action,
                spec_sha256: action.spec_sha256.clone(),
                status: BuildAssetStatusV1::Pending,
                child_job_id: None,
                pack_path: None,
                pack_sha256: None,
                error: None,
            },
        });
    }
    ProjectBuildStateV1 {
        schema_version: BUILD_STATE_SCHEMA_VERSION.to_string(),
        manifest_sha256: manifest_sha256.to_string(),
        plan_sha256: plan_sha256.to_string(),
        assets,
    }
}

/// Apply one status transition and persist the build-state atomically.
fn transition(
    state_path: &Path,
    state: &mut ProjectBuildStateV1,
    asset_id: &str,
    status: BuildAssetStatusV1,
    child_job_id: Option<String>,
    error: Option<String>,
) -> Result<(), AutomationRunError> {
    let entry = state
        .assets
        .iter_mut()
        .find(|entry| entry.asset_id == asset_id)
        .expect("state covers every plan action");
    entry.status = status;
    if child_job_id.is_some() {
        entry.child_job_id = child_job_id;
    }
    entry.error = error;
    write_build_state(state_path, state)
}

fn write_build_state(path: &Path, state: &ProjectBuildStateV1) -> Result<(), AutomationRunError> {
    write_json_atomic(path, state)
}

fn step(
    store: &JobStore,
    job_id: &str,
    name: &str,
    state: &str,
    progress: f32,
) -> Result<(), AutomationRunError> {
    store.update_record(job_id, |record| {
        record.progress = progress;
        if let Some(step) = record.steps.iter_mut().find(|step| step.name == name) {
            step.state = state.to_string();
        }
    })?;
    Ok(())
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), AutomationRunError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}
