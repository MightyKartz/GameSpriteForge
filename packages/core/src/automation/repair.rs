use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::export::CharacterQualityReport;
use crate::frames::CanvasMode;
use crate::job::{
    JobArtifactRecord, JobLifecycleState, JobRecord, JobStore, JobStoreError,
    RepairAnimationQuality, RepairChange, RepairContext, RepairQualitySnapshot,
};
use crate::quality::{QualityRecommendationId, QualityReport, QualityVerdict};

use super::{
    AutomationOperation, MattingRecipe, PlanStore, PlanStoreError, PrepareAssetRequest,
    PrepareCharacterPackRequest, PreparedPlan,
};

pub const MAX_REPAIR_ATTEMPTS: u32 = 3;

#[derive(Debug, Error)]
pub enum RepairError {
    #[error(transparent)]
    Job(#[from] JobStoreError),
    #[error(transparent)]
    Plan(#[from] PlanStoreError),
    #[error("repair is available only for awaiting_review prepare jobs")]
    UnsupportedJob,
    #[error("repair attempt limit reached ({MAX_REPAIR_ATTEMPTS})")]
    AttemptLimit,
    #[error("source job already has an active repair job: {0}")]
    AlreadyRepaired(String),
    #[error("job has no reusable automation recipe")]
    MissingRecipe,
    #[error("job automation recipe is invalid: {0}")]
    InvalidRecipe(String),
    #[error("quality evidence is invalid: {0}")]
    InvalidQuality(#[from] serde_json::Error),
    #[error("could not read quality evidence at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("no safe automatic repair is available; use manualActions from analyze_repair")]
    NoAutomaticChanges,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairAnalysis {
    pub schema_version: String,
    pub source_job_id: String,
    pub attempt: u32,
    pub can_auto_repair: bool,
    pub quality: RepairQualitySnapshot,
    pub changes: Vec<RepairChange>,
    pub manual_actions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed_operation: Option<AutomationOperation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairComparison {
    pub schema_version: String,
    pub source_job_id: String,
    pub repair_job_id: String,
    pub attempt: u32,
    pub improved: bool,
    pub before: RepairQualitySnapshot,
    pub after: RepairQualitySnapshot,
    pub changes: Vec<RepairChange>,
}

pub fn analyze_repair(store: &JobStore, job_id: &str) -> Result<RepairAnalysis, RepairError> {
    let source = store.read_record(job_id)?;
    if source.lifecycle_state != JobLifecycleState::AwaitingReview {
        return Err(RepairError::UnsupportedJob);
    }
    if let Some(child) = active_repair_child(store, &source.job_id)? {
        return Err(RepairError::AlreadyRepaired(child.job_id));
    }
    let attempt = source
        .repair
        .as_ref()
        .map(|repair| repair.attempt.saturating_add(1))
        .unwrap_or(1);
    if attempt > MAX_REPAIR_ATTEMPTS {
        return Err(RepairError::AttemptLimit);
    }
    let operation: AutomationOperation =
        serde_json::from_value(source.recipe.clone().ok_or(RepairError::MissingRecipe)?)
            .map_err(|error| RepairError::InvalidRecipe(error.to_string()))?;
    let quality = read_quality_snapshot(&source, &operation)?;
    let mut proposed = operation.clone();
    let mut changes = Vec::new();
    let mut manual_actions = BTreeSet::new();
    apply_safe_repairs(&quality, &mut proposed, &mut changes, &mut manual_actions);
    let can_auto_repair = !changes.is_empty();
    Ok(RepairAnalysis {
        schema_version: "1".into(),
        source_job_id: source.job_id,
        attempt,
        can_auto_repair,
        quality,
        changes,
        manual_actions: manual_actions.into_iter().collect(),
        proposed_operation: can_auto_repair.then_some(proposed),
    })
}

pub(crate) fn active_repair_child(
    store: &JobStore,
    source_job_id: &str,
) -> Result<Option<JobRecord>, JobStoreError> {
    Ok(store.list_records()?.into_iter().find(|record| {
        record
            .repair
            .as_ref()
            .is_some_and(|repair| repair.source_job_id == source_job_id)
            && !matches!(
                record.lifecycle_state,
                JobLifecycleState::Failed | JobLifecycleState::Cancelled
            )
    }))
}

pub fn prepare_repair_plan(
    plans: &PlanStore,
    jobs: &JobStore,
    job_id: &str,
) -> Result<PreparedPlan, RepairError> {
    let analysis = analyze_repair(jobs, job_id)?;
    let operation = analysis
        .proposed_operation
        .ok_or(RepairError::NoAutomaticChanges)?;
    let context = RepairContext {
        source_job_id: analysis.source_job_id,
        attempt: analysis.attempt,
        changes: analysis.changes,
        baseline: analysis.quality,
    };
    Ok(plans.prepare_with_repair_context(operation, Some(context))?)
}

pub fn single_quality_snapshot(name: &str, report: QualityReport) -> RepairQualitySnapshot {
    RepairQualitySnapshot {
        verdict: report.verdict,
        animations: vec![RepairAnimationQuality {
            name: name.to_string(),
            report,
        }],
    }
}

pub fn character_quality_snapshot(report: &CharacterQualityReport) -> RepairQualitySnapshot {
    RepairQualitySnapshot {
        verdict: report.verdict,
        animations: report
            .animations
            .iter()
            .map(|entry| RepairAnimationQuality {
                name: entry.name.clone(),
                report: entry.report.clone(),
            })
            .collect(),
    }
}

pub fn write_repair_comparison(
    record: &JobRecord,
    after: RepairQualitySnapshot,
) -> Result<Option<JobArtifactRecord>, RepairError> {
    let Some(context) = record.repair.clone() else {
        return Ok(None);
    };
    let comparison = RepairComparison {
        schema_version: "1".into(),
        source_job_id: context.source_job_id,
        repair_job_id: record.job_id.clone(),
        attempt: context.attempt,
        improved: quality_score(&after) > quality_score(&context.baseline),
        before: context.baseline,
        after,
        changes: context.changes,
    };
    let path = record.job_dir.join("repair-comparison.json");
    fs::write(&path, serde_json::to_vec_pretty(&comparison)?).map_err(|source| {
        RepairError::Io {
            path: path.clone(),
            source,
        }
    })?;
    Ok(Some(JobArtifactRecord {
        kind: "repair_comparison".into(),
        path,
        sha256: None,
    }))
}

fn read_quality_snapshot(
    source: &JobRecord,
    operation: &AutomationOperation,
) -> Result<RepairQualitySnapshot, RepairError> {
    match operation {
        AutomationOperation::PrepareAsset(request) => {
            let path = source.job_dir.join("quality-report.json");
            let report: QualityReport = read_json(&path)?;
            Ok(single_quality_snapshot(&request.metadata.animation, report))
        }
        AutomationOperation::PrepareCharacterPack(_) => {
            let path = source.job_dir.join("animation-quality-report.json");
            let report: CharacterQualityReport = read_json(&path)?;
            Ok(character_quality_snapshot(&report))
        }
        AutomationOperation::GenerateCharacterPack(_) => {
            let path = source.job_dir.join("animation-quality-report.json");
            let report: CharacterQualityReport = read_json(&path)?;
            Ok(character_quality_snapshot(&report))
        }
        AutomationOperation::CreateStyleLock(_)
        | AutomationOperation::CreateSubjectLock(_)
        | AutomationOperation::GenerateStaticAssetSet(_)
        | AutomationOperation::CreateEnvironmentLock(_)
        | AutomationOperation::GenerateTerrainSet(_)
        | AutomationOperation::GenerateBuildingKit(_)
        | AutomationOperation::CompileMap(_)
        | AutomationOperation::InstallGodot(_)
        | AutomationOperation::BuildProject(_) => Err(RepairError::UnsupportedJob),
    }
}

fn read_json<T: serde::de::DeserializeOwned>(path: &PathBuf) -> Result<T, RepairError> {
    let bytes = fs::read(path).map_err(|source| RepairError::Io {
        path: path.clone(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(RepairError::InvalidQuality)
}

fn apply_safe_repairs(
    quality: &RepairQualitySnapshot,
    operation: &mut AutomationOperation,
    changes: &mut Vec<RepairChange>,
    manual: &mut BTreeSet<String>,
) {
    match operation {
        AutomationOperation::PrepareAsset(request) => {
            if let Some(animation) = quality.animations.first() {
                apply_animation_recommendations(
                    "asset",
                    &animation.report,
                    &mut request.matting,
                    changes,
                    manual,
                );
            }
            apply_shared_recommendations(
                quality.animations.iter().map(|item| &item.report),
                request,
                changes,
                manual,
            );
        }
        AutomationOperation::PrepareCharacterPack(request) => {
            for evidence in &quality.animations {
                if let Some(animation) = request
                    .animations
                    .iter_mut()
                    .find(|animation| animation.name == evidence.name)
                {
                    apply_animation_recommendations(
                        &format!("animation:{}", evidence.name),
                        &evidence.report,
                        &mut animation.matting,
                        changes,
                        manual,
                    );
                }
            }
            apply_character_shared_recommendations(
                quality.animations.iter().map(|item| &item.report),
                request,
                changes,
                manual,
            );
        }
        AutomationOperation::GenerateCharacterPack(_) => {
            manual.insert(
                "generation jobs retry failed animation media internally; create a new V3 plan to change the prompt or generation policy"
                    .into(),
            );
        }
        AutomationOperation::CreateStyleLock(_)
        | AutomationOperation::CreateSubjectLock(_)
        | AutomationOperation::GenerateStaticAssetSet(_)
        | AutomationOperation::CreateEnvironmentLock(_)
        | AutomationOperation::GenerateTerrainSet(_)
        | AutomationOperation::GenerateBuildingKit(_)
        | AutomationOperation::CompileMap(_)
        | AutomationOperation::InstallGodot(_)
        | AutomationOperation::BuildProject(_) => {}
    }
}

fn apply_animation_recommendations(
    scope: &str,
    report: &QualityReport,
    matting: &mut MattingRecipe,
    changes: &mut Vec<RepairChange>,
    manual: &mut BTreeSet<String>,
) {
    for recommendation in &report.recommendations {
        match recommendation {
            QualityRecommendationId::ReduceChromaThreshold => {
                adjust_chroma_threshold(scope, matting, -12, changes, manual)
            }
            QualityRecommendationId::IncreaseChromaThreshold => {
                adjust_chroma_threshold(scope, matting, 12, changes, manual)
            }
            QualityRecommendationId::TrimLoopRange => {
                manual.insert(format!("{scope}:review_loop_range"));
            }
            QualityRecommendationId::UseShorterClip => {
                manual.insert(format!("{scope}:choose_shorter_frame_range"));
            }
            QualityRecommendationId::AdjustAnchor
            | QualityRecommendationId::IncreaseCanvasMargin => {}
        }
    }
    if report
        .notes
        .iter()
        .any(|note| note == "frame_size_inconsistent")
    {
        manual.insert(format!("{scope}:verify_source_frame_dimensions"));
    }
}

fn adjust_chroma_threshold(
    scope: &str,
    matting: &mut MattingRecipe,
    delta: i16,
    changes: &mut Vec<RepairChange>,
    manual: &mut BTreeSet<String>,
) {
    let parameters = match matting {
        MattingRecipe::AutoCorners { parameters }
        | MattingRecipe::ManualColor { parameters, .. } => parameters,
        MattingRecipe::PreserveAlpha => {
            manual.insert(format!("{scope}:choose_background_matting"));
            return;
        }
    };
    let before = parameters.threshold;
    let after = (before as i16 + delta).clamp(0, 255) as u8;
    if before == after {
        manual.insert(format!("{scope}:chroma_threshold_at_limit"));
        return;
    }
    parameters.threshold = after;
    let direction = if delta < 0 { "reduce" } else { "increase" };
    changes.push(RepairChange {
        id: format!("{scope}:chroma-threshold-{direction}"),
        scope: scope.into(),
        parameter: "matting.threshold".into(),
        before: before.into(),
        after: after.into(),
        reason: if delta < 0 {
            "foreground coverage is too low or missing".into()
        } else {
            "alpha coverage suggests background residue".into()
        },
        confidence: "high".into(),
    });
}

fn apply_shared_recommendations<'a>(
    reports: impl Iterator<Item = &'a QualityReport>,
    request: &mut PrepareAssetRequest,
    changes: &mut Vec<RepairChange>,
    manual: &mut BTreeSet<String>,
) {
    let recommendations = collect_recommendations(reports);
    apply_normalize_recommendations(&recommendations, &mut request.normalize, changes, manual);
}

fn apply_character_shared_recommendations<'a>(
    reports: impl Iterator<Item = &'a QualityReport>,
    request: &mut PrepareCharacterPackRequest,
    changes: &mut Vec<RepairChange>,
    manual: &mut BTreeSet<String>,
) {
    let recommendations = collect_recommendations(reports);
    apply_normalize_recommendations(&recommendations, &mut request.normalize, changes, manual);
}

fn collect_recommendations<'a>(
    reports: impl Iterator<Item = &'a QualityReport>,
) -> BTreeSet<String> {
    reports
        .flat_map(|report| report.recommendations.iter())
        .map(|recommendation| format!("{recommendation:?}"))
        .collect()
}

fn apply_normalize_recommendations(
    recommendations: &BTreeSet<String>,
    normalize: &mut crate::frames::NormalizeOptions,
    changes: &mut Vec<RepairChange>,
    manual: &mut BTreeSet<String>,
) {
    if recommendations.contains("IncreaseCanvasMargin") {
        let before = normalize.margin;
        let after = before.saturating_add(4).min(128);
        if after > before {
            normalize.margin = after;
            changes.push(RepairChange {
                id: "shared:increase-canvas-margin".into(),
                scope: "shared".into(),
                parameter: "normalize.margin".into(),
                before: before.into(),
                after: after.into(),
                reason: "foreground touches a normalized cell boundary".into(),
                confidence: "high".into(),
            });
        }
    }
    if recommendations.contains("AdjustAnchor") {
        if normalize.mode != CanvasMode::SquareBottom {
            let before = serde_json::to_value(normalize.mode).unwrap_or_default();
            normalize.mode = CanvasMode::SquareBottom;
            changes.push(RepairChange {
                id: "shared:use-bottom-anchor".into(),
                scope: "shared".into(),
                parameter: "normalize.mode".into(),
                before,
                after: serde_json::to_value(normalize.mode).unwrap_or_default(),
                reason: "bottom or horizontal anchor drift exceeds the quality threshold".into(),
                confidence: "medium".into(),
            });
        } else {
            manual.insert("shared:inspect_anchor_drift".into());
        }
    }
}

fn quality_score(snapshot: &RepairQualitySnapshot) -> f32 {
    let verdict = match snapshot.verdict {
        QualityVerdict::Blocked => 0.0,
        QualityVerdict::PrototypeUsable => 1.0,
        QualityVerdict::NeedsCleanup => 2.0,
        QualityVerdict::GameReady => 3.0,
    };
    let detail = if snapshot.animations.is_empty() {
        0.0
    } else {
        snapshot
            .animations
            .iter()
            .map(|animation| {
                let metrics = animation.report.metrics;
                metrics.loop_match_score
                    - metrics.bbox_bottom_drift_px / 100.0
                    - metrics.bbox_center_x_drift_px / 200.0
            })
            .sum::<f32>()
            / snapshot.animations.len() as f32
    };
    verdict * 10.0 + detail
}
