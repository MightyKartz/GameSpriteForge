use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::asset_project::{
    image_signature, init_project, ConsistencyReportV1, ConsistencyVerdict, ForgeProjectV1,
    StyleSpecV1, FORGE_PROJECT_FILE,
};
use crate::automation::{
    automation_profile, run_operation_with_provider, stage_plan_job, AutomationOperation,
    CharacterPackMetadata, CharacterWorkflowSelection, CreateStyleLockRequest,
    CreateSubjectLockRequest, GenerateCharacterPackRequest, GeneratedCharacterSpec,
    GenerationPolicy, GodotInstallRequest, PlanStore,
};
use crate::job::{JobLifecycleState, JobRecord, JobStore};
use crate::provider::MediaGenerationProvider;
use crate::subject::SubjectSpecV1;

pub const CHARACTER_BENCHMARK_PROFILE: &str = "character-v2-benchmark@1.0.0";

#[derive(Debug, Error)]
pub enum BenchmarkError {
    #[error("invalid benchmark: {0}")]
    Invalid(String),
    #[error("benchmark execution failed: {0}")]
    Execution(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterBenchmarkManifestV1 {
    pub schema_version: String,
    pub id: String,
    pub styles: Vec<CharacterBenchmarkStyleV1>,
    pub cases: Vec<CharacterBenchmarkCaseV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterBenchmarkStyleV1 {
    pub id: String,
    pub spec: StyleSpecV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterBenchmarkCaseV1 {
    pub id: String,
    pub style_id: String,
    pub subject: SubjectSpecV1,
    #[serde(default)]
    pub expected_hard_defect: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkWorkflow {
    Video,
    Keyframes,
}

impl BenchmarkWorkflow {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Video => "video",
            Self::Keyframes => "keyframes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterBenchmarkCaseResultV1 {
    pub case_id: String,
    pub style_id: String,
    pub workflow: BenchmarkWorkflow,
    pub job_id: String,
    pub lifecycle_state: String,
    pub game_ready: bool,
    pub pack_exported: bool,
    pub pack_valid: bool,
    #[serde(default)]
    pub godot_validation_attempted: bool,
    pub godot_loaded: bool,
    pub hard_defect_expected: bool,
    pub hard_defect_detected: bool,
    pub identity_pass_count: u32,
    pub identity_sample_count: u32,
    pub provider_requests: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterBenchmarkRunV1 {
    pub schema_version: String,
    pub profile: String,
    pub benchmark_id: String,
    pub provider_id: String,
    pub profile_id: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub cases: Vec<CharacterBenchmarkCaseResultV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkRequestEstimateV1 {
    pub normal: u64,
    pub maximum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterBenchmarkPlanV1 {
    pub schema_version: String,
    pub profile: String,
    pub benchmark_id: String,
    pub provider_id: String,
    pub profile_id: String,
    pub style_count: usize,
    pub case_count: usize,
    pub full_frozen_scope: bool,
    pub shared_setup_requests: BenchmarkRequestEstimateV1,
    pub video_requests: BenchmarkRequestEstimateV1,
    pub keyframe_requests: BenchmarkRequestEstimateV1,
    pub both_workflows_requests: BenchmarkRequestEstimateV1,
    pub hard_defect_labels: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterBenchmarkExecutionOptions {
    pub output_root: PathBuf,
    pub provider_id: String,
    pub profile_id: String,
    pub workflows: Vec<BenchmarkWorkflow>,
    pub limit: Option<usize>,
    pub godot_project: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BenchmarkGateVerdict {
    Pass,
    Fail,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowBenchmarkMetricsV1 {
    pub case_count: usize,
    pub successful_pack_count: usize,
    pub automated_pack_success_rate: f32,
    pub godot_validation_count: usize,
    pub godot_loaded_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub godot_load_rate: Option<f32>,
    pub identity_pass_count: u64,
    pub identity_sample_count: u64,
    pub identity_pass_rate: f32,
    pub median_provider_requests: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterBenchmarkGatesV1 {
    pub frozen_scope: BenchmarkGateVerdict,
    pub keyframe_pack_success: BenchmarkGateVerdict,
    pub hard_defect_interception: BenchmarkGateVerdict,
    pub zero_erroneous_pack_exports: BenchmarkGateVerdict,
    pub identity_improvement: BenchmarkGateVerdict,
    pub provider_request_budget: BenchmarkGateVerdict,
    pub godot_validation: BenchmarkGateVerdict,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterBenchmarkSummaryV1 {
    pub schema_version: String,
    pub profile: String,
    pub benchmark_id: String,
    pub provider_id: String,
    pub distinct_case_count: usize,
    pub distinct_style_count: usize,
    pub workflows: BTreeMap<String, WorkflowBenchmarkMetricsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity_improvement_percentage_points: Option<f32>,
    pub expected_hard_defect_count: usize,
    pub intercepted_hard_defect_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hard_defect_interception_rate: Option<f32>,
    pub erroneous_pack_export_count: usize,
    pub gates: CharacterBenchmarkGatesV1,
    pub upgrade_recommended: bool,
}

impl CharacterBenchmarkManifestV1 {
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if self.schema_version != "1" || self.id.trim().is_empty() {
            return Err(BenchmarkError::Invalid(
                "schemaVersion 1 and benchmark id are required".into(),
            ));
        }
        let style_ids = self
            .styles
            .iter()
            .map(|style| style.id.as_str())
            .collect::<BTreeSet<_>>();
        if style_ids.len() != self.styles.len() || style_ids.iter().any(|id| id.trim().is_empty()) {
            return Err(BenchmarkError::Invalid(
                "style ids must be non-empty and unique".into(),
            ));
        }
        let case_ids = self
            .cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<BTreeSet<_>>();
        if case_ids.len() != self.cases.len() || case_ids.iter().any(|id| id.trim().is_empty()) {
            return Err(BenchmarkError::Invalid(
                "case ids must be non-empty and unique".into(),
            ));
        }
        if self
            .cases
            .iter()
            .any(|case| !style_ids.contains(case.style_id.as_str()))
        {
            return Err(BenchmarkError::Invalid(
                "every case must reference a declared style".into(),
            ));
        }
        for style in &self.styles {
            if style.spec.schema_version != "1"
                || style.spec.prompt.trim().is_empty()
                || style.spec.prompt.len() > 4_000
                || style.spec.reference_images.len() > 3
            {
                return Err(BenchmarkError::Invalid(format!(
                    "style {} has an invalid StyleSpecV1",
                    style.id
                )));
            }
            for size in [
                style.spec.character_canvas_size,
                style.spec.icon_canvas_size,
                style.spec.prop_canvas_size,
            ] {
                if !(64..=512).contains(&size) || !size.is_power_of_two() {
                    return Err(BenchmarkError::Invalid(format!(
                        "style {} uses an unsupported canvas size {size}",
                        style.id
                    )));
                }
            }
        }
        for case in &self.cases {
            if case.subject.schema_version != "1"
                || case.subject.id.trim().is_empty()
                || !case
                    .subject
                    .id
                    .chars()
                    .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_'))
                || case.subject.name.trim().is_empty()
                || case.subject.prompt.trim().is_empty()
                || case.subject.prompt.len() > 4_000
                || case.subject.reference_images.len() > 2
                || case.subject.license.trim().is_empty()
            {
                return Err(BenchmarkError::Invalid(format!(
                    "case {} has an invalid SubjectSpecV1",
                    case.id
                )));
            }
        }
        Ok(())
    }
}

pub fn plan_character_benchmark(
    manifest: &CharacterBenchmarkManifestV1,
    provider_id: &str,
    profile_id: &str,
) -> Result<CharacterBenchmarkPlanV1, BenchmarkError> {
    manifest.validate()?;
    if provider_id.trim().is_empty() || profile_id.trim().is_empty() {
        return Err(BenchmarkError::Invalid(
            "providerId and profileId are required".into(),
        ));
    }
    let style_count = manifest.styles.len() as u64;
    let case_count = manifest.cases.len() as u64;
    // Style and Subject Locks are shared by the video and keyframe comparisons.
    let shared = BenchmarkRequestEstimateV1 {
        normal: style_count + case_count,
        maximum: style_count + case_count,
    };
    // These values deliberately mirror PlanStore's per-Character estimates.
    let video = BenchmarkRequestEstimateV1 {
        normal: case_count.saturating_mul(9),
        maximum: case_count.saturating_mul(13),
    };
    let keyframes = BenchmarkRequestEstimateV1 {
        normal: case_count.saturating_mul(32),
        maximum: case_count.saturating_mul(64),
    };
    let both = BenchmarkRequestEstimateV1 {
        normal: shared
            .normal
            .saturating_add(video.normal)
            .saturating_add(keyframes.normal),
        maximum: shared
            .maximum
            .saturating_add(video.maximum)
            .saturating_add(keyframes.maximum),
    };
    let hard_defect_labels = manifest
        .cases
        .iter()
        .filter(|case| case.expected_hard_defect)
        .count();
    let mut notes = vec![
        "Style and Subject Lock requests are shared across both workflows.".into(),
        "The estimate counts Provider requests, not price; the Provider remains responsible for pricing.".into(),
    ];
    if hard_defect_labels == 0 {
        notes.push(
            "No hard-defect labels are present; interception gates will remain not_evaluated."
                .into(),
        );
    }
    Ok(CharacterBenchmarkPlanV1 {
        schema_version: "1".into(),
        profile: CHARACTER_BENCHMARK_PROFILE.into(),
        benchmark_id: manifest.id.clone(),
        provider_id: provider_id.into(),
        profile_id: profile_id.into(),
        style_count: manifest.styles.len(),
        case_count: manifest.cases.len(),
        full_frozen_scope: manifest.styles.len() >= 5 && manifest.cases.len() >= 20,
        shared_setup_requests: shared,
        video_requests: video,
        keyframe_requests: keyframes,
        both_workflows_requests: both,
        hard_defect_labels,
        notes,
    })
}

pub fn run_character_benchmark(
    manifest: &CharacterBenchmarkManifestV1,
    manifest_path: &Path,
    options: &CharacterBenchmarkExecutionOptions,
    provider: &dyn MediaGenerationProvider,
) -> Result<(CharacterBenchmarkRunV1, CharacterBenchmarkSummaryV1), BenchmarkError> {
    manifest.validate()?;
    if provider.id() != options.provider_id {
        return Err(BenchmarkError::Invalid(format!(
            "resolved provider {} does not match benchmark provider {}",
            provider.id(),
            options.provider_id
        )));
    }
    if options.workflows.is_empty() {
        return Err(BenchmarkError::Invalid(
            "at least one benchmark workflow is required".into(),
        ));
    }
    fs::create_dir_all(&options.output_root).map_err(execution_error)?;
    let plans = PlanStore::new(options.output_root.join("plans")).map_err(execution_error)?;
    let jobs = JobStore::new(options.output_root.join("jobs")).map_err(execution_error)?;
    let specs_root = options.output_root.join("specs");
    fs::create_dir_all(&specs_root).map_err(execution_error)?;
    if let Some(project) = &options.godot_project {
        fs::create_dir_all(project).map_err(execution_error)?;
        let descriptor = project.join("project.godot");
        if !descriptor.is_file() {
            fs::write(
                descriptor,
                "[application]\nconfig/name=\"Forge Character Benchmark\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
            )
            .map_err(execution_error)?;
        }
    }

    let started_at = Utc::now();
    let manifest_root = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let selected_cases = manifest
        .cases
        .iter()
        .take(options.limit.unwrap_or(usize::MAX))
        .collect::<Vec<_>>();
    let selected_styles = selected_cases
        .iter()
        .map(|case| case.style_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut style_locks = BTreeMap::<String, PathBuf>::new();
    let mut style_failures = BTreeMap::<String, (String, String)>::new();

    for style in manifest
        .styles
        .iter()
        .filter(|style| selected_styles.contains(style.id.as_str()))
    {
        let project_root = options.output_root.join("projects").join(&style.id);
        let mut project = init_project(&project_root, &format!("Benchmark {}", style.id))
            .map_err(execution_error)?;
        project.provider.id = options.provider_id.clone();
        project.provider.profile_id = options.profile_id.clone();
        write_project(&project_root, &project)?;
        let mut spec = style.spec.clone();
        spec.reference_images = spec
            .reference_images
            .iter()
            .map(|path| manifest_root.join(path))
            .collect();
        let spec_path = specs_root.join(format!("style-{}.json", style.id));
        write_json(&spec_path, &spec)?;
        let operation = AutomationOperation::CreateStyleLock(CreateStyleLockRequest {
            schema_version: "1".into(),
            project_path: project_root,
            spec_path,
            provider_id: options.provider_id.clone(),
            profile_id: options.profile_id.clone(),
        });
        let outcome = execute_benchmark_operation(&plans, &jobs, operation, Some(provider))?;
        if let Some(lock) = outcome
            .record
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "style_lock")
        {
            style_locks.insert(style.id.clone(), lock.path.clone());
        } else {
            style_failures.insert(
                style.id.clone(),
                (
                    outcome.record.job_id,
                    outcome.error.unwrap_or_else(|| "style_lock_missing".into()),
                ),
            );
        }
    }

    let mut results = Vec::new();
    let profile = automation_profile();
    for case in selected_cases {
        let project_root = options.output_root.join("projects").join(&case.style_id);
        let Some(style_lock_path) = style_locks.get(&case.style_id) else {
            let (job_id, message) = style_failures
                .get(&case.style_id)
                .cloned()
                .unwrap_or_else(|| ("style-unavailable".into(), "style unavailable".into()));
            for workflow in &options.workflows {
                results.push(failed_case_result(
                    case,
                    *workflow,
                    &job_id,
                    "style_lock_failed",
                    &message,
                ));
            }
            persist_run(
                &options.output_root,
                build_run(manifest, options, started_at, results.clone()),
            )?;
            continue;
        };

        let mut subject = case.subject.clone();
        subject.reference_images = subject
            .reference_images
            .iter()
            .map(|path| manifest_root.join(path))
            .collect();
        let subject_spec_path = specs_root.join(format!("subject-{}.json", case.id));
        write_json(&subject_spec_path, &subject)?;
        let subject_outcome = execute_benchmark_operation(
            &plans,
            &jobs,
            AutomationOperation::CreateSubjectLock(CreateSubjectLockRequest {
                schema_version: "1".into(),
                project_path: project_root.clone(),
                spec_path: subject_spec_path,
                provider_id: options.provider_id.clone(),
                profile_id: options.profile_id.clone(),
            }),
            Some(provider),
        )?;
        let subject_lock_path = subject_outcome
            .record
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "subject_lock")
            .map(|artifact| artifact.path.clone());
        let canonical_path = subject_outcome
            .record
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "subject_canonical")
            .map(|artifact| artifact.path.clone());
        let (Some(subject_lock_path), Some(canonical_path)) = (subject_lock_path, canonical_path)
        else {
            let message = subject_outcome
                .error
                .unwrap_or_else(|| "subject lock artifacts are missing".into());
            for workflow in &options.workflows {
                results.push(failed_case_result(
                    case,
                    *workflow,
                    &subject_outcome.record.job_id,
                    "subject_lock_failed",
                    &message,
                ));
            }
            persist_run(
                &options.output_root,
                build_run(manifest, options, started_at, results.clone()),
            )?;
            continue;
        };

        for workflow in &options.workflows {
            let selection = match workflow {
                BenchmarkWorkflow::Video => CharacterWorkflowSelection {
                    id: "topdown".into(),
                    version: "1.0.0".into(),
                },
                BenchmarkWorkflow::Keyframes => CharacterWorkflowSelection {
                    id: "topdown-keyframes".into(),
                    version: "2.0.0".into(),
                },
            };
            let request = GenerateCharacterPackRequest {
                schema_version: "3".into(),
                provider_id: options.provider_id.clone(),
                profile_id: options.profile_id.clone(),
                project_path: Some(project_root.clone()),
                asset_id: Some(format!("{}-{}", case.id, workflow.as_str())),
                character: GeneratedCharacterSpec {
                    prompt: case.subject.prompt.clone(),
                    reference_image_path: Some(canonical_path.clone()),
                },
                style_lock_path: Some(style_lock_path.clone()),
                subject_lock_path: (*workflow == BenchmarkWorkflow::Keyframes)
                    .then(|| subject_lock_path.clone()),
                reuse_from_job_dir: None,
                retry_animations: Vec::new(),
                retry_stages: BTreeMap::new(),
                retry_frames: BTreeMap::new(),
                metadata: CharacterPackMetadata {
                    name: format!("{} {} benchmark", case.subject.name, workflow.as_str()),
                    default_animation: "idle".into(),
                    creator: "Game Sprite Forge benchmark".into(),
                    license: case.subject.license.clone(),
                },
                workflow: selection,
                generation: GenerationPolicy {
                    image_model: case.subject.image_model.clone(),
                    ..GenerationPolicy::default()
                },
                normalize: profile.normalize,
                sheet: profile.sheet,
                quality: profile.quality.clone(),
            };
            let usage_before = provider.usage();
            let outcome = execute_benchmark_operation(
                &plans,
                &jobs,
                AutomationOperation::GenerateCharacterPack(request),
                Some(provider),
            )?;
            let provider_requests = provider
                .usage()
                .requests
                .saturating_sub(usage_before.requests) as u64;
            let mut result = inspect_case_result(
                case,
                *workflow,
                &outcome.record,
                provider_requests,
                outcome.error.as_deref(),
            );
            if result.pack_valid {
                if let (Some(godot_project), Some(pack)) = (
                    options.godot_project.as_ref(),
                    outcome
                        .record
                        .artifacts
                        .iter()
                        .find(|artifact| artifact.kind == "gsfpack"),
                ) {
                    result.godot_validation_attempted = true;
                    let install = execute_benchmark_operation(
                        &plans,
                        &jobs,
                        AutomationOperation::InstallGodot(GodotInstallRequest {
                            schema_version: "1".into(),
                            pack_path: pack.path.clone(),
                            project_path: godot_project.clone(),
                            catalog_project_path: Some(project_root.clone()),
                            target: PathBuf::from(format!(
                                "addons/forge_assets/benchmark/{}/{}",
                                workflow.as_str(),
                                case.id
                            )),
                            asset_key: Some(format!("{}-{}", case.id, workflow.as_str())),
                            provider_refs: Vec::new(),
                        }),
                        None,
                    )?;
                    result.godot_loaded = install.error.is_none()
                        && install.record.lifecycle_state == JobLifecycleState::Succeeded;
                    if install.error.is_some() && result.error_code.is_none() {
                        result.error_code = Some("godot_install_failed".into());
                    }
                }
            }
            results.push(result);
            persist_run(
                &options.output_root,
                build_run(manifest, options, started_at, results.clone()),
            )?;
        }
    }
    let run = build_run(manifest, options, started_at, results);
    persist_run(&options.output_root, run.clone())?;
    let summary = summarize_character_benchmark(&run)?;
    write_json(
        &options.output_root.join("benchmark-summary.json"),
        &summary,
    )?;
    Ok((run, summary))
}

struct BenchmarkOperationOutcome {
    record: JobRecord,
    error: Option<String>,
}

fn execute_benchmark_operation(
    plans: &PlanStore,
    jobs: &JobStore,
    operation: AutomationOperation,
    provider: Option<&dyn MediaGenerationProvider>,
) -> Result<BenchmarkOperationOutcome, BenchmarkError> {
    let prepared = plans.prepare(operation).map_err(execution_error)?;
    let claimed = plans.claim(&prepared.token).map_err(execution_error)?;
    let staged = stage_plan_job(jobs, &claimed).map_err(execution_error)?;
    match run_operation_with_provider(jobs, &staged.job_id, &claimed.operation, provider) {
        Ok(record) => Ok(BenchmarkOperationOutcome {
            record,
            error: None,
        }),
        Err(error) => Ok(BenchmarkOperationOutcome {
            record: jobs.read_record(&staged.job_id).map_err(execution_error)?,
            error: Some(error.to_string()),
        }),
    }
}

fn inspect_case_result(
    case: &CharacterBenchmarkCaseV1,
    workflow: BenchmarkWorkflow,
    record: &JobRecord,
    provider_requests: u64,
    execution_error: Option<&str>,
) -> CharacterBenchmarkCaseResultV1 {
    let pack = record
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack");
    let pack_valid =
        pack.is_some_and(|artifact| forge_pack::validate_pack_layout(&artifact.path).is_ok());
    let report = record
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "consistency_report")
        .and_then(|artifact| fs::read(&artifact.path).ok())
        .and_then(|bytes| serde_json::from_slice::<ConsistencyReportV1>(&bytes).ok())
        .or_else(|| {
            fs::read(record.job_dir.join("consistency-report.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<ConsistencyReportV1>(&bytes).ok())
        });
    let mut identity_pass_count = report
        .as_ref()
        .map(|report| {
            report
                .items
                .iter()
                .filter(|item| {
                    item.metrics
                        .perceptual_similarity
                        .is_some_and(|score| score >= 0.70)
                })
                .count() as u32
        })
        .unwrap_or_default();
    let mut identity_sample_count = report
        .as_ref()
        .map(|report| {
            report
                .items
                .iter()
                .filter(|item| item.metrics.perceptual_similarity.is_some())
                .count() as u32
        })
        .unwrap_or_default();
    if identity_sample_count == 0 && workflow == BenchmarkWorkflow::Video {
        (identity_pass_count, identity_sample_count) = video_identity_counts(record);
    }
    let hard_defect_detected = report.as_ref().is_some_and(|report| {
        matches!(
            report.verdict,
            ConsistencyVerdict::Regenerate | ConsistencyVerdict::Blocked
        ) || report.items.iter().any(|item| {
            !item.metrics.canvas_matches
                || !item.metrics.alpha_present
                || !item.metrics.cell_boundary_safe
                || item.metrics.subject_count != 1
        })
    });
    let game_ready = report
        .as_ref()
        .is_some_and(|report| report.verdict == ConsistencyVerdict::GameReady)
        && record.lifecycle_state == JobLifecycleState::Succeeded;
    CharacterBenchmarkCaseResultV1 {
        case_id: case.id.clone(),
        style_id: case.style_id.clone(),
        workflow,
        job_id: record.job_id.clone(),
        lifecycle_state: serde_json::to_value(record.lifecycle_state)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".into()),
        game_ready,
        pack_exported: pack.is_some(),
        pack_valid,
        godot_validation_attempted: false,
        godot_loaded: false,
        hard_defect_expected: case.expected_hard_defect,
        hard_defect_detected,
        identity_pass_count,
        identity_sample_count,
        provider_requests,
        error_code: record
            .error_code
            .clone()
            .or_else(|| execution_error.map(|_| "benchmark_operation_failed".into())),
    }
}

fn video_identity_counts(record: &JobRecord) -> (u32, u32) {
    let reference_path = record.job_dir.join("normalized/character-reference.png");
    let stills_root = record.job_dir.join("normalized/character-stills");
    let Ok(reference) = image::open(reference_path) else {
        return (0, 0);
    };
    let reference = image_signature(&reference.to_rgba8());
    let Ok(entries) = fs::read_dir(stills_root) else {
        return (0, 0);
    };
    let mut passes = 0_u32;
    let mut samples = 0_u32;
    for path in entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("png"))
    {
        let Ok(candidate) = image::open(path) else {
            continue;
        };
        let candidate = image_signature(&candidate.to_rgba8());
        let similarity = 1.0
            - (candidate.perceptual_hash ^ reference.perceptual_hash).count_ones() as f32 / 64.0;
        samples += 1;
        if similarity >= 0.70 {
            passes += 1;
        }
    }
    (passes, samples)
}

fn failed_case_result(
    case: &CharacterBenchmarkCaseV1,
    workflow: BenchmarkWorkflow,
    job_id: &str,
    error_code: &str,
    message: &str,
) -> CharacterBenchmarkCaseResultV1 {
    let normalized = message.to_ascii_lowercase();
    let hard_defect_detected = [
        "alpha",
        "clipping",
        "single-subject",
        "multiple_subject",
        "multiple subject",
        "subject_missing",
        "subject missing",
        "malformed image",
    ]
    .iter()
    .any(|indicator| normalized.contains(indicator));
    CharacterBenchmarkCaseResultV1 {
        case_id: case.id.clone(),
        style_id: case.style_id.clone(),
        workflow,
        job_id: job_id.into(),
        lifecycle_state: "failed".into(),
        game_ready: false,
        pack_exported: false,
        pack_valid: false,
        godot_validation_attempted: false,
        godot_loaded: false,
        hard_defect_expected: case.expected_hard_defect,
        hard_defect_detected,
        identity_pass_count: 0,
        identity_sample_count: 0,
        provider_requests: 0,
        error_code: Some(error_code.into()),
    }
}

fn build_run(
    manifest: &CharacterBenchmarkManifestV1,
    options: &CharacterBenchmarkExecutionOptions,
    started_at: DateTime<Utc>,
    cases: Vec<CharacterBenchmarkCaseResultV1>,
) -> CharacterBenchmarkRunV1 {
    CharacterBenchmarkRunV1 {
        schema_version: "1".into(),
        profile: CHARACTER_BENCHMARK_PROFILE.into(),
        benchmark_id: manifest.id.clone(),
        provider_id: options.provider_id.clone(),
        profile_id: options.profile_id.clone(),
        started_at,
        finished_at: Utc::now(),
        cases,
    }
}

fn persist_run(output_root: &Path, run: CharacterBenchmarkRunV1) -> Result<(), BenchmarkError> {
    write_json(&output_root.join("benchmark-run.json"), &run)
}

fn write_project(root: &Path, project: &ForgeProjectV1) -> Result<(), BenchmarkError> {
    write_json(&root.join(FORGE_PROJECT_FILE), project)
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), BenchmarkError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(execution_error)?;
    }
    let pending = path.with_extension("json.pending");
    fs::write(
        &pending,
        serde_json::to_vec_pretty(value).map_err(execution_error)?,
    )
    .map_err(execution_error)?;
    fs::rename(&pending, path).map_err(execution_error)?;
    Ok(())
}

fn execution_error(error: impl std::fmt::Display) -> BenchmarkError {
    BenchmarkError::Execution(error.to_string())
}

pub fn summarize_character_benchmark(
    run: &CharacterBenchmarkRunV1,
) -> Result<CharacterBenchmarkSummaryV1, BenchmarkError> {
    if run.schema_version != "1"
        || run.profile != CHARACTER_BENCHMARK_PROFILE
        || run.benchmark_id.trim().is_empty()
        || run.provider_id.trim().is_empty()
    {
        return Err(BenchmarkError::Invalid(
            "benchmark run identity or profile is invalid".into(),
        ));
    }
    let distinct_cases = run
        .cases
        .iter()
        .map(|case| case.case_id.as_str())
        .collect::<BTreeSet<_>>();
    let distinct_styles = run
        .cases
        .iter()
        .map(|case| case.style_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut workflows = BTreeMap::new();
    for workflow in [BenchmarkWorkflow::Video, BenchmarkWorkflow::Keyframes] {
        let cases = run
            .cases
            .iter()
            .filter(|case| case.workflow == workflow)
            .collect::<Vec<_>>();
        let successful = cases
            .iter()
            .filter(|case| case.game_ready && case.pack_exported && case.pack_valid)
            .count();
        let godot_validation_count = cases
            .iter()
            .filter(|case| case.godot_validation_attempted)
            .count();
        let godot_loaded_count = cases.iter().filter(|case| case.godot_loaded).count();
        let identity_pass_count = cases
            .iter()
            .map(|case| case.identity_pass_count as u64)
            .sum::<u64>();
        let identity_sample_count = cases
            .iter()
            .map(|case| case.identity_sample_count as u64)
            .sum::<u64>();
        let mut requests = cases
            .iter()
            .map(|case| case.provider_requests)
            .collect::<Vec<_>>();
        requests.sort_unstable();
        let metrics = WorkflowBenchmarkMetricsV1 {
            case_count: cases.len(),
            successful_pack_count: successful,
            automated_pack_success_rate: ratio(successful as u64, cases.len() as u64),
            godot_validation_count,
            godot_loaded_count,
            godot_load_rate: (godot_validation_count > 0).then_some(ratio(
                godot_loaded_count as u64,
                godot_validation_count as u64,
            )),
            identity_pass_count,
            identity_sample_count,
            identity_pass_rate: ratio(identity_pass_count, identity_sample_count),
            median_provider_requests: median(&requests),
        };
        workflows.insert(workflow.as_str().into(), metrics);
    }
    let video = workflows.get("video").expect("video metrics exist");
    let keyframes = workflows.get("keyframes").expect("keyframe metrics exist");
    let identity_improvement = (video.identity_sample_count > 0
        && keyframes.identity_sample_count > 0)
        .then_some((keyframes.identity_pass_rate - video.identity_pass_rate) * 100.0);
    let expected_hard = run
        .cases
        .iter()
        .filter(|case| case.hard_defect_expected)
        .count();
    let intercepted_hard = run
        .cases
        .iter()
        .filter(|case| {
            case.hard_defect_expected && case.hard_defect_detected && !case.pack_exported
        })
        .count();
    let hard_rate =
        (expected_hard > 0).then_some(ratio(intercepted_hard as u64, expected_hard as u64));
    let erroneous_exports = run
        .cases
        .iter()
        .filter(|case| {
            (case.hard_defect_expected || case.hard_defect_detected) && case.pack_exported
        })
        .count();
    let gates = CharacterBenchmarkGatesV1 {
        frozen_scope: pass_fail(distinct_cases.len() >= 20 && distinct_styles.len() >= 5),
        keyframe_pack_success: evaluated(
            keyframes.case_count > 0,
            keyframes.automated_pack_success_rate >= 0.90,
        ),
        hard_defect_interception: hard_rate
            .map(|rate| pass_fail(rate >= 1.0))
            .unwrap_or(BenchmarkGateVerdict::NotEvaluated),
        zero_erroneous_pack_exports: evaluated(expected_hard > 0, erroneous_exports == 0),
        identity_improvement: identity_improvement
            .map(|improvement| pass_fail(improvement >= 10.0))
            .unwrap_or(BenchmarkGateVerdict::NotEvaluated),
        provider_request_budget: evaluated(
            keyframes.case_count > 0,
            keyframes.median_provider_requests <= 40,
        ),
        godot_validation: {
            let successful = workflows
                .values()
                .map(|metrics| metrics.successful_pack_count)
                .sum::<usize>();
            let attempted = workflows
                .values()
                .map(|metrics| metrics.godot_validation_count)
                .sum::<usize>();
            let loaded = workflows
                .values()
                .map(|metrics| metrics.godot_loaded_count)
                .sum::<usize>();
            evaluated(
                successful > 0 && attempted > 0,
                attempted == successful && loaded == successful,
            )
        },
    };
    let upgrade_recommended = [
        gates.frozen_scope,
        gates.keyframe_pack_success,
        gates.hard_defect_interception,
        gates.zero_erroneous_pack_exports,
        gates.identity_improvement,
        gates.provider_request_budget,
        gates.godot_validation,
    ]
    .into_iter()
    .all(|verdict| verdict == BenchmarkGateVerdict::Pass);
    Ok(CharacterBenchmarkSummaryV1 {
        schema_version: "1".into(),
        profile: CHARACTER_BENCHMARK_PROFILE.into(),
        benchmark_id: run.benchmark_id.clone(),
        provider_id: run.provider_id.clone(),
        distinct_case_count: distinct_cases.len(),
        distinct_style_count: distinct_styles.len(),
        workflows,
        identity_improvement_percentage_points: identity_improvement,
        expected_hard_defect_count: expected_hard,
        intercepted_hard_defect_count: intercepted_hard,
        hard_defect_interception_rate: hard_rate,
        erroneous_pack_export_count: erroneous_exports,
        gates,
        upgrade_recommended,
    })
}

fn ratio(numerator: u64, denominator: u64) -> f32 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f32 / denominator as f32
    }
}

fn median(values: &[u64]) -> u64 {
    if values.is_empty() {
        0
    } else if values.len() % 2 == 1 {
        values[values.len() / 2]
    } else {
        let right = values.len() / 2;
        (values[right - 1] + values[right]) / 2
    }
}

fn pass_fail(value: bool) -> BenchmarkGateVerdict {
    if value {
        BenchmarkGateVerdict::Pass
    } else {
        BenchmarkGateVerdict::Fail
    }
}

fn evaluated(evaluated: bool, value: bool) -> BenchmarkGateVerdict {
    if evaluated {
        pass_fail(value)
    } else {
        BenchmarkGateVerdict::NotEvaluated
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(
        index: usize,
        workflow: BenchmarkWorkflow,
        identity_pass: u32,
        requests: u64,
    ) -> CharacterBenchmarkCaseResultV1 {
        CharacterBenchmarkCaseResultV1 {
            case_id: format!("case-{index:02}"),
            style_id: format!("style-{}", index % 5),
            workflow,
            job_id: format!("job-{index:02}-{}", workflow.as_str()),
            lifecycle_state: "succeeded".into(),
            game_ready: true,
            pack_exported: true,
            pack_valid: true,
            godot_validation_attempted: true,
            godot_loaded: true,
            hard_defect_expected: false,
            hard_defect_detected: false,
            identity_pass_count: identity_pass,
            identity_sample_count: 10,
            provider_requests: requests,
            error_code: None,
        }
    }

    #[test]
    fn summary_enforces_scope_identity_and_request_gates_without_faking_labels() {
        let mut cases = Vec::new();
        for index in 0..20 {
            cases.push(result(index, BenchmarkWorkflow::Video, 7, 9));
            cases.push(result(index, BenchmarkWorkflow::Keyframes, 9, 32));
        }
        let run = CharacterBenchmarkRunV1 {
            schema_version: "1".into(),
            profile: CHARACTER_BENCHMARK_PROFILE.into(),
            benchmark_id: "frozen-20x5".into(),
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            cases,
        };
        let summary = summarize_character_benchmark(&run).unwrap();
        assert_eq!(summary.distinct_case_count, 20);
        assert_eq!(summary.distinct_style_count, 5);
        assert_eq!(
            summary.gates.identity_improvement,
            BenchmarkGateVerdict::Pass
        );
        assert_eq!(
            summary.gates.provider_request_budget,
            BenchmarkGateVerdict::Pass
        );
        assert_eq!(
            summary.gates.hard_defect_interception,
            BenchmarkGateVerdict::NotEvaluated
        );
        assert!(!summary.upgrade_recommended);
    }

    #[test]
    fn hard_defect_export_blocks_release_recommendation() {
        let mut cases = (0..20)
            .map(|index| result(index, BenchmarkWorkflow::Keyframes, 9, 32))
            .collect::<Vec<_>>();
        cases[0].hard_defect_expected = true;
        cases[0].hard_defect_detected = true;
        let run = CharacterBenchmarkRunV1 {
            schema_version: "1".into(),
            profile: CHARACTER_BENCHMARK_PROFILE.into(),
            benchmark_id: "hard-defect".into(),
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            started_at: Utc::now(),
            finished_at: Utc::now(),
            cases,
        };
        let summary = summarize_character_benchmark(&run).unwrap();
        assert_eq!(summary.erroneous_pack_export_count, 1);
        assert_eq!(
            summary.gates.zero_erroneous_pack_exports,
            BenchmarkGateVerdict::Fail
        );
    }

    #[test]
    fn full_plan_counts_shared_setup_once_and_exposes_unlabeled_defect_gate() {
        let styles = (0..5)
            .map(|index| CharacterBenchmarkStyleV1 {
                id: format!("style-{index}"),
                spec: serde_json::from_value(serde_json::json!({
                    "schemaVersion": "1",
                    "prompt": format!("style {index}"),
                    "referenceImages": [],
                    "perspective": "topdown",
                    "lighting": "upper_left",
                    "outline": "clean",
                    "background": "transparent",
                    "sampling": "nearest",
                    "characterCanvasSize": 256,
                    "iconCanvasSize": 128,
                    "propCanvasSize": 256
                }))
                .unwrap(),
            })
            .collect::<Vec<_>>();
        let cases = (0..20)
            .map(|index| CharacterBenchmarkCaseV1 {
                id: format!("case-{index}"),
                style_id: format!("style-{}", index % 5),
                subject: SubjectSpecV1 {
                    schema_version: "1".into(),
                    id: format!("subject-{index}"),
                    name: format!("Subject {index}"),
                    prompt: "an original game character".into(),
                    reference_images: Vec::new(),
                    image_model: None,
                    license: "MIT".into(),
                },
                expected_hard_defect: false,
            })
            .collect::<Vec<_>>();
        let plan = plan_character_benchmark(
            &CharacterBenchmarkManifestV1 {
                schema_version: "1".into(),
                id: "frozen".into(),
                styles,
                cases,
            },
            "xai",
            "default",
        )
        .unwrap();
        assert!(plan.full_frozen_scope);
        assert_eq!(plan.shared_setup_requests.normal, 25);
        assert_eq!(plan.video_requests.normal, 180);
        assert_eq!(plan.keyframe_requests.normal, 640);
        assert_eq!(plan.both_workflows_requests.normal, 845);
        assert_eq!(plan.both_workflows_requests.maximum, 1_565);
        assert_eq!(plan.hard_defect_labels, 0);
    }
}
