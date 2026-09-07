#![cfg(feature = "grid-generation")]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use forge_core::asset_project::{
    hash_file, init_project, read_project, CharacterEquipmentKindV1, SamplingMode, StyleSpecV1,
    FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::hash_directory;
use forge_core::automation::{
    automation_profile, run_operation, run_operation_with_provider, stage_plan_job,
    AutomationOperation, CharacterPackMetadata, CharacterRetryStage, CharacterWorkflowSelection,
    CreateStyleLockRequest, CreateSubjectLockRequest, ExternalDirectionGridFilesV1,
    GenerateCharacterPackRequest, GeneratedCharacterSpec, GenerationPolicy, GodotInstallRequest,
    ImportDirectionGridRequestV1, PlanStore, QualityPolicy,
};
use forge_core::character_camera::CharacterCameraProfileV1;
use forge_core::character_cycle::CharacterScaleLockV1;
use forge_core::character_direction_motion::{
    CharacterMotionProfileV1, DirectionMotionGenerationStageV1,
};
use forge_core::character_grid::{
    CapeHemConsistencyReportV1, CapeHemContractV1, DirectionGridAppearanceReportV1,
    DirectionGridApprovalV1, DirectionGridLockV1, GridKeyframeActionReportV1,
    GridKeyframeGenerationMethodV1, GridPoseGuidanceV1, CAPE_HEM_CONSISTENCY_FILE,
    DIRECTION_GRID_APPEARANCE_FILE, DIRECTION_GRID_IMPORT_LOCK_PROFILE, GRID_APPROVAL_FILE,
    GRID_APPROVAL_PROFILE,
};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::project::ProviderAssetRef;
use forge_core::provider::{
    EditImageRequest, EditVideoRequest, GenerateImageRequest, GenerateVideoRequest,
    MediaGenerationProvider, ProviderError, ProviderHealth, ProviderMedia, ProviderPoll,
    ProviderTicket, ProviderUsage, ReferenceRole,
};
use forge_core::subject::{read_subject_lock, SubjectSpecV1};
use forge_core::workflow_graph::{compute_artifact_cache_key, read_workflow_graph};
use forge_providers::fixture::{FixtureProvider, FixtureRequestKind};
use image::{Rgba, RgbaImage};
use sha2::{Digest, Sha256};

/// Deliberately presents as a real xAI provider with a matching durable grant,
/// while retaining the trait's fail-closed default scope validator. This proves
/// Core cannot treat a matching authorization id as sufficient V9.3 proof.
struct UnvalidatedXaiProvider {
    fixture: FixtureProvider,
    authorization_id: String,
    authorization_manifest_sha256: String,
}

/// Offline proof for the narrowly enabled V10 real-provider boundary. It
/// behaves like Fixture for media bytes but presents the exact durable xAI
/// scope that Core must validate before the first request.
struct AuthorizedV10XaiProvider {
    fixture: FixtureProvider,
    authorization_id: String,
    authorization_manifest_sha256: String,
    expected_lineage: String,
    expected_recipe_hash: String,
    expected_input_fingerprint: String,
    scope_checks: Mutex<u32>,
}

/// Offline proof for the Image 2.0 candidate boundary. Media bytes still come
/// from Fixture, while Core must prove the exact real xAI 1/1 scope first.
struct AuthorizedImage2XaiProvider {
    fixture: FixtureProvider,
    authorization_id: String,
    authorization_manifest_sha256: String,
    expected_lineage: String,
    expected_recipe_hash: String,
    expected_input_fingerprint: String,
    scope_checks: Mutex<u32>,
}

impl MediaGenerationProvider for AuthorizedImage2XaiProvider {
    fn id(&self) -> &'static str {
        "xai"
    }

    fn capabilities(&self) -> Vec<forge_core::provider::ProviderCapability> {
        self.fixture.capabilities()
    }

    fn health_check(&self) -> ProviderHealth {
        let mut health = self.fixture.health_check();
        health.provider_id = self.id().into();
        health
    }

    fn durable_authorization_id(&self) -> Result<Option<String>, ProviderError> {
        Ok(Some(self.authorization_id.clone()))
    }

    fn durable_authorization_manifest_sha256(&self) -> Result<Option<String>, ProviderError> {
        Ok(Some(self.authorization_manifest_sha256.clone()))
    }

    fn validate_durable_authorization_scope(
        &self,
        provider_id: &str,
        profile_id: &str,
        model: &str,
        target: &str,
        max_requests: u32,
        max_operations: u32,
        max_cost_ticks: u64,
        lineage_root_job_id: &str,
        recipe_hash: &str,
        input_fingerprint: &str,
    ) -> Result<(), ProviderError> {
        let exact = provider_id == "xai"
            && profile_id == "default"
            && model == forge_core::automation::XAI_IMAGE_2_CANDIDATE_MODEL
            && target == "direction_grid"
            && max_requests == 1
            && max_operations == 1
            && max_cost_ticks == forge_core::automation::XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS
            && lineage_root_job_id == self.expected_lineage
            && recipe_hash == self.expected_recipe_hash
            && input_fingerprint == self.expected_input_fingerprint;
        if !exact {
            return Err(ProviderError::RealProviderNotAccepted(
                "Image 2.0 exact authorization scope mismatch".into(),
            ));
        }
        *self.scope_checks.lock().unwrap() += 1;
        Ok(())
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        Some(
            requested
                .unwrap_or(forge_core::automation::XAI_IMAGE_2_CANDIDATE_MODEL)
                .to_string(),
        )
    }

    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_video_model(requested)
    }

    fn resolved_video_edit_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_video_edit_model(requested)
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.fixture.generate_image(request, output_path)
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.fixture.edit_image(request, output_path)
    }

    fn generate_video(
        &self,
        request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        self.fixture.generate_video(request)
    }

    fn edit_video(&self, request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        self.fixture.edit_video(request)
    }

    fn poll(
        &self,
        ticket: &ProviderTicket,
        output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError> {
        self.fixture.poll(ticket, output_path)
    }

    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError> {
        self.fixture.cancel(ticket)
    }

    fn usage(&self) -> ProviderUsage {
        self.fixture.usage()
    }
}

impl MediaGenerationProvider for AuthorizedV10XaiProvider {
    fn id(&self) -> &'static str {
        "xai"
    }

    fn capabilities(&self) -> Vec<forge_core::provider::ProviderCapability> {
        self.fixture.capabilities()
    }

    fn health_check(&self) -> ProviderHealth {
        let mut health = self.fixture.health_check();
        health.provider_id = self.id().into();
        health
    }

    fn durable_authorization_id(&self) -> Result<Option<String>, ProviderError> {
        Ok(Some(self.authorization_id.clone()))
    }

    fn durable_authorization_manifest_sha256(&self) -> Result<Option<String>, ProviderError> {
        Ok(Some(self.authorization_manifest_sha256.clone()))
    }

    fn validate_durable_authorization_scope(
        &self,
        provider_id: &str,
        profile_id: &str,
        model: &str,
        target: &str,
        max_requests: u32,
        max_operations: u32,
        max_cost_ticks: u64,
        lineage_root_job_id: &str,
        recipe_hash: &str,
        input_fingerprint: &str,
    ) -> Result<(), ProviderError> {
        let exact = provider_id == "xai"
            && profile_id == "default"
            && model == "grok-imagine-video-1.5"
            && target == "walk_down:video"
            && max_requests == 1
            && max_operations == 1
            && max_cost_ticks == 3_300_000_000
            && lineage_root_job_id == self.expected_lineage
            && recipe_hash == self.expected_recipe_hash
            && input_fingerprint == self.expected_input_fingerprint;
        if !exact {
            return Err(ProviderError::RealProviderNotAccepted(
                "V10 exact authorization scope mismatch".into(),
            ));
        }
        *self.scope_checks.lock().unwrap() += 1;
        Ok(())
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_image_model(requested)
    }

    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        Some(requested.unwrap_or("grok-imagine-video-1.5").to_string())
    }

    fn resolved_video_edit_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_video_edit_model(requested)
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.fixture.generate_image(request, output_path)
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.fixture.edit_image(request, output_path)
    }

    fn generate_video(
        &self,
        request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        self.fixture.generate_video(request)
    }

    fn edit_video(&self, request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        self.fixture.edit_video(request)
    }

    fn poll(
        &self,
        ticket: &ProviderTicket,
        output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError> {
        self.fixture.poll(ticket, output_path)
    }

    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError> {
        self.fixture.cancel(ticket)
    }

    fn usage(&self) -> ProviderUsage {
        self.fixture.usage()
    }
}

impl UnvalidatedXaiProvider {
    fn new(
        authorization_id: impl Into<String>,
        authorization_manifest_sha256: impl Into<String>,
    ) -> Self {
        Self {
            fixture: FixtureProvider::default(),
            authorization_id: authorization_id.into(),
            authorization_manifest_sha256: authorization_manifest_sha256.into(),
        }
    }
}

impl MediaGenerationProvider for UnvalidatedXaiProvider {
    fn id(&self) -> &'static str {
        "xai"
    }

    fn capabilities(&self) -> Vec<forge_core::provider::ProviderCapability> {
        self.fixture.capabilities()
    }

    fn health_check(&self) -> ProviderHealth {
        let mut health = self.fixture.health_check();
        health.provider_id = self.id().into();
        health
    }

    fn durable_authorization_id(&self) -> Result<Option<String>, ProviderError> {
        Ok(Some(self.authorization_id.clone()))
    }

    fn durable_authorization_manifest_sha256(&self) -> Result<Option<String>, ProviderError> {
        Ok(Some(self.authorization_manifest_sha256.clone()))
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_image_model(requested)
    }

    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_video_model(requested)
    }

    fn resolved_video_edit_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_video_edit_model(requested)
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.fixture.generate_image(request, output_path)
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.fixture.edit_image(request, output_path)
    }

    fn generate_video(
        &self,
        request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        self.fixture.generate_video(request)
    }

    fn edit_video(&self, request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        self.fixture.edit_video(request)
    }

    fn poll(
        &self,
        ticket: &ProviderTicket,
        output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError> {
        self.fixture.poll(ticket, output_path)
    }

    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError> {
        self.fixture.cancel(ticket)
    }

    fn usage(&self) -> ProviderUsage {
        self.fixture.usage()
    }
}

/// Models a provider-side time-of-check/time-of-use race: it accepts the
/// immutable reference hashes sent by Core, materializes a valid fixture
/// response, and only then changes the locally-addressed input. Core must
/// never persist those changed bytes as if they were the bytes sent upstream.
struct PostSendReferenceMutatingProvider {
    fixture: FixtureProvider,
    mutate_role: ReferenceRole,
    observed_reference_sha256: Mutex<Vec<String>>,
}

impl PostSendReferenceMutatingProvider {
    fn new(mutate_role: ReferenceRole) -> Self {
        Self {
            fixture: FixtureProvider::default(),
            mutate_role,
            observed_reference_sha256: Mutex::new(Vec::new()),
        }
    }

    fn observed_reference_sha256(&self) -> Vec<String> {
        self.observed_reference_sha256.lock().unwrap().clone()
    }
}

impl MediaGenerationProvider for PostSendReferenceMutatingProvider {
    fn id(&self) -> &'static str {
        self.fixture.id()
    }

    fn capabilities(&self) -> Vec<forge_core::provider::ProviderCapability> {
        self.fixture.capabilities()
    }

    fn health_check(&self) -> ProviderHealth {
        self.fixture.health_check()
    }

    fn resolved_image_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_image_model(requested)
    }

    fn resolved_video_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_video_model(requested)
    }

    fn resolved_video_edit_model(&self, requested: Option<&str>) -> Option<String> {
        self.fixture.resolved_video_edit_model(requested)
    }

    fn generate_image(
        &self,
        request: &GenerateImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        self.fixture.generate_image(request, output_path)
    }

    fn edit_image(
        &self,
        request: &EditImageRequest,
        output_path: &Path,
    ) -> Result<ProviderMedia, ProviderError> {
        let reference = request
            .references
            .iter()
            .find(|reference| reference.role == self.mutate_role)
            .expect("V9.3 request includes the targeted immutable reference");
        self.observed_reference_sha256
            .lock()
            .unwrap()
            .push(reference.sha256.clone());
        let reference_path = reference.path.clone();

        let media = self.fixture.edit_image(request, output_path)?;
        let mut mutated = fs::read(&reference_path).map_err(ProviderError::Io)?;
        mutated.push(0);
        fs::write(&reference_path, mutated).map_err(ProviderError::Io)?;
        Ok(media)
    }

    fn generate_video(
        &self,
        request: &GenerateVideoRequest,
    ) -> Result<ProviderTicket, ProviderError> {
        self.fixture.generate_video(request)
    }

    fn edit_video(&self, request: &EditVideoRequest) -> Result<ProviderTicket, ProviderError> {
        self.fixture.edit_video(request)
    }

    fn poll(
        &self,
        ticket: &ProviderTicket,
        output_path: &Path,
    ) -> Result<ProviderPoll, ProviderError> {
        self.fixture.poll(ticket, output_path)
    }

    fn cancel(&self, ticket: &ProviderTicket) -> Result<(), ProviderError> {
        self.fixture.cancel(ticket)
    }

    fn usage(&self) -> ProviderUsage {
        self.fixture.usage()
    }
}

fn setup() -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    PathBuf,
    PlanStore,
    JobStore,
) {
    setup_with_subject_prompt("compact purple ranger")
}

fn setup_with_subject_prompt(
    subject_prompt: &str,
) -> (
    tempfile::TempDir,
    PathBuf,
    PathBuf,
    PathBuf,
    PlanStore,
    JobStore,
) {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "Grid Generation Contract").unwrap();
    project.provider.id = "fixture".into();
    fs::write(
        project_root.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    let style_spec = temp.path().join("style.json");
    fs::write(
        &style_spec,
        serde_json::to_vec_pretty(&StyleSpecV1 {
            schema_version: "1".into(),
            prompt: "compact purple game art".into(),
            reference_images: vec![],
            perspective: "top_down".into(),
            lighting: "soft".into(),
            outline: "clean".into(),
            background: "transparent".into(),
            sampling: SamplingMode::Nearest,
            character_canvas_size: 256,
            icon_canvas_size: 128,
            prop_canvas_size: 256,
            image_model: None,
        })
        .unwrap(),
    )
    .unwrap();
    let subject_spec = temp.path().join("subject.json");
    fs::write(
        &subject_spec,
        serde_json::to_vec_pretty(&SubjectSpecV1 {
            schema_version: "1".into(),
            id: "grid-ranger".into(),
            name: "Grid Ranger".into(),
            prompt: subject_prompt.into(),
            reference_images: vec![],
            image_model: None,
            license: "MIT".into(),
        })
        .unwrap(),
    )
    .unwrap();
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    for operation in [
        AutomationOperation::CreateStyleLock(CreateStyleLockRequest {
            schema_version: "1".into(),
            project_path: project_root.clone(),
            spec_path: style_spec,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
        }),
        AutomationOperation::CreateSubjectLock(CreateSubjectLockRequest {
            schema_version: "1".into(),
            project_path: project_root.clone(),
            spec_path: subject_spec,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            canonical_import_path: None,
            import_approval_note: None,
        }),
    ] {
        let plan = plans.prepare(operation).unwrap();
        let plan = plans.claim(&plan.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        let completed =
            run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider))
                .unwrap();
        assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    }
    let style_revision = read_project(&project_root)
        .unwrap()
        .current_style_revision
        .unwrap();
    let style_path = project_root
        .join(".forge/styles")
        .join(style_revision)
        .join(STYLE_LOCK_FILE);
    let subject_path = project_root
        .join(".forge/subjects/grid-ranger")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("subject-lock.json");
    (temp, project_root, style_path, subject_path, plans, jobs)
}

fn freeze_v92_failure_as_legacy_source(
    jobs: &JobStore,
    source: &forge_core::job::JobRecord,
) -> GridKeyframeActionReportV1 {
    let report_path = source
        .job_dir
        .join("source/grid-keyframe-actions/walk_down.json");
    let mut report: GridKeyframeActionReportV1 =
        serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
    report.profile = "grid-keyframe-action-report@1.1.0".into();
    report.frames[2].generation_method = GridKeyframeGenerationMethodV1::MotionDiagnosticEdit;
    report.frames[2].input_frame_sha256 = Some(report.frames[2].sha256.clone());
    fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let report_sha256 = hash_file(&report_path).unwrap();

    let manifest_path = source
        .job_dir
        .join("source/grid-keyframe-provider-manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["actions"][0]["frames"][2]["generationMethod"] =
        serde_json::json!("motion_diagnostic_edit");
    manifest["actions"][0]["frames"][2]["inputFrameSha256"] =
        serde_json::json!(report.frames[2].input_frame_sha256);
    manifest["actions"][0]["actionReportProfile"] = serde_json::json!(report.profile);
    manifest["actions"][0]["actionReportPath"] = serde_json::json!(report_path);
    manifest["actions"][0]["actionReportSha256"] = serde_json::json!(report_sha256);
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();
    let manifest_sha256 = hash_file(&manifest_path).unwrap();

    let graph_path = source.job_dir.join("workflow-graph.json");
    let mut graph = read_workflow_graph(&graph_path).unwrap();
    let frame_node = graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "independent_keyframe:walk_down:2")
        .unwrap();
    frame_node.implementation_version = "grid-structured-keyframe@1.0.0".into();
    frame_node.inputs.insert(
        0,
        forge_core::workflow_graph::WorkflowArtifactV1 {
            sha256: report.frames[2].input_frame_sha256.clone().unwrap(),
            path: report.frames[2].path.clone(),
        },
    );
    let quality_node = graph
        .nodes
        .iter_mut()
        .find(|node| node.id == "action_quality:walk_down")
        .unwrap();
    quality_node.outputs[0].sha256 = report_sha256.clone();
    forge_core::workflow_graph::write_workflow_graph(&graph_path, &graph).unwrap();
    let graph_sha256 = hash_file(&graph_path).unwrap();

    jobs.update_record(&source.job_id, |record| {
        for (kind, sha256) in [
            ("grid_keyframe_action_report_walk_down", &report_sha256),
            ("provider_manifest", &manifest_sha256),
            ("workflow_graph", &graph_sha256),
        ] {
            record
                .artifacts
                .iter_mut()
                .find(|artifact| artifact.kind == kind)
                .unwrap()
                .sha256 = Some(sha256.clone());
        }
    })
    .unwrap();
    report
}

#[test]
fn fixture_grid_v91_validation_uses_four_independent_direction_anchor_edits() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.1.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Disabled;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 4);
    assert_eq!(plan.estimate.maximum_provider_requests, 8);
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();

    assert_eq!(provider.usage().requests, 4);
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(
        completed.error_code.as_deref(),
        Some("grid_keyframe_validation_review_required")
    );
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    let observations = provider
        .request_observations()
        .into_iter()
        .filter(|observation| {
            observation
                .authorization_target
                .as_deref()
                .is_some_and(|target| target.starts_with("walk_down:frame:"))
        })
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 4);
    for (index, observation) in observations.iter().enumerate() {
        assert_eq!(
            observation.authorization_target.as_deref(),
            Some(format!("walk_down:frame:{index}").as_str())
        );
        assert_eq!(
            observation.reference_roles,
            vec![ReferenceRole::DirectionAnchor]
        );
        assert!(observation
            .prompt
            .contains(&format!("Forge frame phase {index}/4")));
        assert!(observation.prompt.contains("not a sheet"));
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["workflow"], "topdown-grid@9.1.0");
    assert_eq!(manifest["stage"], "keyframe_validation");
    assert_eq!(manifest["usage"]["requests"], 4);
    let graph = read_workflow_graph(&completed.job_dir.join("workflow-graph.json")).unwrap();
    assert_eq!(graph.workflow, "topdown-grid@9.1.0");
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "independent_keyframe")
            .count(),
        4
    );
}

#[test]
fn fixture_image2_candidate_is_one_direction_grid_edit_and_stops_for_review() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style,
            subject,
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source =
        promote_approved_direction_grid_source_to_xai(&jobs, &project, &direction_job.job_id);
    let source_hash_before = hash_directory(&approved_source).unwrap();
    let source_record = jobs.read_record(&direction_job.job_id).unwrap();
    let mut candidate: AutomationOperation =
        serde_json::from_value(source_record.recipe.clone().unwrap()).unwrap();
    let AutomationOperation::GenerateCharacterPack(candidate_request) = &mut candidate else {
        unreachable!()
    };
    candidate_request.reuse_from_job_dir = Some(approved_source.clone());
    candidate_request.retry_animations = vec!["direction_grid".into()];
    candidate_request.retry_stages =
        BTreeMap::from([("direction_grid".into(), CharacterRetryStage::Still)]);
    candidate_request.retry_frames.clear();
    candidate_request.generation.image_model =
        Some(forge_core::automation::XAI_IMAGE_2_CANDIDATE_MODEL.into());
    candidate_request.generation.max_attempts_per_animation = 1;

    let mut expanded = candidate.clone();
    let AutomationOperation::GenerateCharacterPack(expanded_request) = &mut expanded else {
        unreachable!()
    };
    expanded_request.generation.max_attempts_per_animation = 2;
    assert!(plans
        .prepare(expanded)
        .unwrap_err()
        .to_string()
        .contains("xai_image2_candidate_scope_forbidden"));

    let plan = plans.prepare(candidate.clone()).unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 1);
    assert_eq!(plan.estimate.maximum_provider_requests, 1);
    assert_eq!(
        plan.estimate.model.as_deref(),
        Some(forge_core::automation::XAI_IMAGE_2_CANDIDATE_MODEL)
    );
    assert!(plan.effects.iter().any(|effect| {
        effect.contains("exactly one grok-imagine-image-2.0 direction_grid edit")
    }));

    let unvalidated_plan = plans.prepare(candidate).unwrap();
    let unvalidated_plan = plans.claim(&unvalidated_plan.token).unwrap();
    let unvalidated_job = stage_plan_job(&jobs, &unvalidated_plan).unwrap();
    jobs.update_record(&unvalidated_job.job_id, |record| {
        record.authorization_id = Some("image2-unvalidated".into());
        record.authorization_manifest_sha256 = Some("c".repeat(64));
    })
    .unwrap();
    let unvalidated = UnvalidatedXaiProvider::new("image2-unvalidated", "c".repeat(64));
    let error = run_operation_with_provider(
        &jobs,
        &unvalidated_job.job_id,
        &unvalidated_plan.operation,
        Some(&unvalidated),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("authorization scope validation is not implemented"));
    assert_eq!(unvalidated.usage().requests, 0);

    let claimed = plans.claim(&plan.token).unwrap();
    let staged = stage_plan_job(&jobs, &claimed).unwrap();
    assert_eq!(
        staged.parent_job_id.as_deref(),
        Some(direction_job.job_id.as_str())
    );
    assert_eq!(
        staged.lineage_root_job_id,
        source_record.lineage_root_job_id
    );
    assert!(staged.authorization_id.is_none());
    assert!(staged.authorization_manifest_sha256.is_none());

    let authorization_id = "image2-candidate-fixture";
    let manifest_sha256 = "b".repeat(64);
    jobs.update_record(&staged.job_id, |record| {
        record.authorization_id = Some(authorization_id.into());
        record.authorization_manifest_sha256 = Some(manifest_sha256.clone());
    })
    .unwrap();
    let staged = jobs.read_record(&staged.job_id).unwrap();
    let provider = AuthorizedImage2XaiProvider {
        fixture: FixtureProvider::default(),
        authorization_id: authorization_id.into(),
        authorization_manifest_sha256: manifest_sha256,
        expected_lineage: staged.lineage_root_job_id.clone().unwrap(),
        expected_recipe_hash: staged.recipe_hash.clone().unwrap(),
        expected_input_fingerprint: staged.input_hash.clone().unwrap(),
        scope_checks: Mutex::new(0),
    };
    let completed =
        run_operation_with_provider(&jobs, &staged.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    assert_eq!(provider.usage().requests, 1);
    assert_eq!(*provider.scope_checks.lock().unwrap(), 1);
    let observations = provider.fixture.request_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].kind, FixtureRequestKind::EditImage);
    assert_eq!(
        observations[0].authorization_target.as_deref(),
        Some("direction_grid")
    );
    assert!(observations[0].prompt.contains("front_hand_state_drift"));
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(
        completed.error_code.as_deref(),
        Some("direction_grid_review_required"),
        "{}",
        completed.error_summary.as_deref().unwrap_or_default()
    );
    assert!(completed
        .artifacts
        .iter()
        .all(|artifact| artifact.kind != "gsfpack"));
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["imageModel"],
        forge_core::automation::XAI_IMAGE_2_CANDIDATE_MODEL
    );
    let evidence: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/direction-grid-retry-evidence.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(evidence["comparisonOnly"], true);
    assert_eq!(evidence["evaluationTag"], "candidate_model_comparison");
    assert_eq!(
        evidence["correctionCodes"],
        serde_json::json!(["front_hand_state_drift"])
    );
    assert_eq!(evidence["sourceImageModel"], "grok-imagine-image-quality");
    assert_eq!(
        evidence["targetImageModel"],
        forge_core::automation::XAI_IMAGE_2_CANDIDATE_MODEL
    );
    assert_eq!(
        hash_directory(&approved_source).unwrap(),
        source_hash_before
    );
}

#[test]
fn external_direction_grid_import_mattes_checkerboard_with_zero_provider_requests() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project,
            style,
            subject,
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);
    let source_hash_before = hash_directory(&approved_source).unwrap();
    let external_sheet = _temp.path().join("external-checkerboard-grid.png");
    write_checkerboard_from_direction_lock(&approved_source, &external_sheet);

    let operation = AutomationOperation::ImportDirectionGrid(ImportDirectionGridRequestV1 {
        schema_version: "1".into(),
        source_job_dir: approved_source.clone(),
        sheet_path: Some(external_sheet.clone()),
        direction_paths: None,
        cape_hem_contract: None,
        generator: "codex_builtin_image_gen".into(),
        note: "fixture external candidate".into(),
    });
    let plan = plans.prepare(operation).unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 0);
    assert_eq!(plan.estimate.maximum_provider_requests, 0);
    assert!(plan.estimate.provider_id.is_none());
    assert!(plan
        .effects
        .iter()
        .any(|effect| effect.contains("zero Provider requests")));
    let plan = plans.claim(&plan.token).unwrap();
    let staged = stage_plan_job(&jobs, &plan).unwrap();
    assert_eq!(
        staged.parent_job_id.as_deref(),
        Some(direction_job.job_id.as_str())
    );
    assert!(staged.authorization_id.is_none());
    let completed = run_operation(&jobs, &staged.job_id, &plan.operation).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    let import_consistency = fs::read_to_string(
        completed
            .job_dir
            .join("direction-grid-import-consistency-report.json"),
    )
    .unwrap();
    assert!(
        matches!(
            completed.error_code.as_deref(),
            Some("direction_grid_review_required" | "direction_grid_regeneration_required")
        ),
        "{}\n{}",
        completed.error_summary.as_deref().unwrap_or_default(),
        import_consistency
    );
    assert!(completed
        .steps
        .iter()
        .filter(|step| step.name.starts_with("import:"))
        .all(|step| step.state == "succeeded"));
    assert!(completed.artifacts.iter().any(|artifact| {
        artifact.kind == "direction_grid_import_evidence"
            && artifact
                .sha256
                .as_deref()
                .is_some_and(|sha| sha.len() == 64)
    }));
    assert!(completed
        .artifacts
        .iter()
        .all(|artifact| artifact.kind != "gsfpack"));
    let usage: serde_json::Value =
        serde_json::from_slice(&fs::read(completed.job_dir.join("provider-usage.json")).unwrap())
            .unwrap();
    assert_eq!(usage["usage"]["requests"], 0);
    let matting: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/imported-direction-grid/checkerboard-matting-report.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(matting["profile"], "checkerboard-sheet-matting@1.1.0");
    assert_eq!(matting["verdict"], "game_ready");
    assert_eq!(matting["significantComponentCount"], 4);
    assert_eq!(matting["neutralEdgeResidualPixels"], 0);
    assert_eq!(
        matting["transparencyOrigin"],
        "deterministic_checkerboard_postprocess"
    );
    let matted = image::open(
        completed
            .job_dir
            .join("source/imported-direction-grid/sheet-matted.png"),
    )
    .unwrap()
    .to_rgba8();
    assert_eq!(matted.get_pixel(0, 0), &Rgba([0, 0, 0, 0]));
    let alpha_edge: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/imported-direction-grid/alpha-edge-halo-report.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(alpha_edge["profile"], "alpha-edge-halo@1.0.0");
    assert_eq!(alpha_edge["verdict"], "game_ready");
    let authority_alignment: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/imported-direction-grid/authority-alignment-report.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        authority_alignment["profile"],
        "direction-grid-authority-alignment@1.0.0"
    );
    assert_eq!(authority_alignment["verdict"], "game_ready");
    assert!(authority_alignment["frames"]
        .as_array()
        .unwrap()
        .iter()
        .all(|frame| {
            (0.90..=1.10).contains(&frame["widthRatio"].as_f64().unwrap())
                && (0.90..=1.10).contains(&frame["heightRatio"].as_f64().unwrap())
                && frame["centerDriftPx"].as_f64().unwrap() <= 2.0
                && frame["footBaselineDriftPx"].as_f64().unwrap() <= 2.0
        }));
    assert_eq!(
        hash_directory(&approved_source).unwrap(),
        source_hash_before
    );

    let tamper_plan = plans
        .prepare(AutomationOperation::ImportDirectionGrid(
            ImportDirectionGridRequestV1 {
                schema_version: "1".into(),
                source_job_dir: approved_source.clone(),
                sheet_path: Some(external_sheet.clone()),
                direction_paths: None,
                cape_hem_contract: None,
                generator: "codex_builtin_image_gen".into(),
                note: "tamper regression".into(),
            },
        ))
        .unwrap();
    let tamper_plan = plans.claim(&tamper_plan.token).unwrap();
    let tamper_job = stage_plan_job(&jobs, &tamper_plan).unwrap();
    let mut changed = image::open(&external_sheet).unwrap().to_rgba8();
    changed.put_pixel(0, 0, Rgba([10, 20, 30, 255]));
    changed.save(&external_sheet).unwrap();
    let error = run_operation(&jobs, &tamper_job.job_id, &tamper_plan.operation).unwrap_err();
    assert!(error.to_string().contains("inputs changed after staging"));

    write_checkerboard_from_direction_lock(&approved_source, &external_sheet);
    let source_tamper_plan = plans
        .prepare(AutomationOperation::ImportDirectionGrid(
            ImportDirectionGridRequestV1 {
                schema_version: "1".into(),
                source_job_dir: approved_source.clone(),
                sheet_path: Some(external_sheet.clone()),
                direction_paths: None,
                cape_hem_contract: None,
                generator: "codex_builtin_image_gen".into(),
                note: "source tamper regression".into(),
            },
        ))
        .unwrap();
    let source_tamper_plan = plans.claim(&source_tamper_plan.token).unwrap();
    let source_tamper_job = stage_plan_job(&jobs, &source_tamper_plan).unwrap();
    let source_lock_path = approved_source.join("source/direction-grid-lock.json");
    let original_lock = fs::read(&source_lock_path).unwrap();
    let mut changed_lock: serde_json::Value = serde_json::from_slice(&original_lock).unwrap();
    changed_lock["imageModel"] = "tampered-model".into();
    fs::write(
        &source_lock_path,
        serde_json::to_vec_pretty(&changed_lock).unwrap(),
    )
    .unwrap();
    let error = run_operation(
        &jobs,
        &source_tamper_job.job_id,
        &source_tamper_plan.operation,
    )
    .unwrap_err();
    assert!(error.to_string().contains("inputs changed after staging"));
    fs::write(&source_lock_path, original_lock).unwrap();

    let missing_direction_sheet = _temp.path().join("missing-direction.png");
    write_checkerboard_from_direction_lock(&approved_source, &missing_direction_sheet);
    let mut missing = image::open(&missing_direction_sheet).unwrap().to_rgba8();
    let half = missing.width() / 2;
    for y in half..missing.height() {
        for x in half..missing.width() {
            let tone = if (x / 32 + y / 32) % 2 == 0 { 236 } else { 254 };
            missing.put_pixel(x, y, Rgba([tone, tone, tone, 255]));
        }
    }
    missing.save(&missing_direction_sheet).unwrap();
    let missing_plan = plans
        .prepare(AutomationOperation::ImportDirectionGrid(
            ImportDirectionGridRequestV1 {
                schema_version: "1".into(),
                source_job_dir: approved_source,
                sheet_path: Some(missing_direction_sheet),
                direction_paths: None,
                cape_hem_contract: None,
                generator: "codex_builtin_image_gen".into(),
                note: "missing direction regression".into(),
            },
        ))
        .unwrap();
    let missing_plan = plans.claim(&missing_plan.token).unwrap();
    let missing_job = stage_plan_job(&jobs, &missing_plan).unwrap();
    let error = run_operation(&jobs, &missing_job.job_id, &missing_plan.operation).unwrap_err();
    assert!(error
        .to_string()
        .contains("checkerboard_sheet_subject_count_invalid"));
}

#[test]
fn external_direction_grid_import_accepts_four_named_files_with_ordered_hash_closure() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project,
            style,
            subject,
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);
    let source_hash_before = hash_directory(&approved_source).unwrap();
    let source_lock: DirectionGridLockV1 = serde_json::from_slice(
        &fs::read(approved_source.join("source/direction-grid-lock.json")).unwrap(),
    )
    .unwrap();
    let external_root = _temp.path().join("four-directions");
    fs::create_dir_all(&external_root).unwrap();
    let copy_node = |node_id: &str, name: &str| {
        let path = external_root.join(format!("{name}.png"));
        fs::copy(&source_lock.node(node_id).unwrap().path, &path).unwrap();
        path
    };
    let paths = ExternalDirectionGridFilesV1 {
        front: copy_node("front_idle", "front"),
        rear: copy_node("back_idle", "rear"),
        right: copy_node("right_idle", "right"),
        left: copy_node("left_idle", "left"),
    };
    let tamper_paths = paths.clone();
    let operation = AutomationOperation::ImportDirectionGrid(ImportDirectionGridRequestV1 {
        schema_version: "1".into(),
        source_job_dir: approved_source.clone(),
        sheet_path: None,
        direction_paths: Some(paths),
        cape_hem_contract: None,
        generator: "codex_builtin_image_gen".into(),
        note: "four named direction fixture".into(),
    });
    let plan = plans.prepare(operation).unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 0);
    assert_eq!(plan.estimate.maximum_provider_requests, 0);
    let plan = plans.claim(&plan.token).unwrap();
    let staged = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &staged.job_id, &plan.operation).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_ne!(
        completed.error_code.as_deref(),
        Some("direction_grid_regeneration_required"),
        "{}",
        completed.error_summary.as_deref().unwrap_or_default()
    );
    let evidence: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/direction-grid-import-evidence.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(evidence["profile"], "direction-grid-import@1.1.0");
    assert_eq!(evidence["inputMode"], "four_files");
    assert_eq!(
        evidence["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|input| input["role"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["front", "rear", "right", "left"]
    );
    let imported_lock: DirectionGridLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-grid-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(imported_lock.profile, DIRECTION_GRID_IMPORT_LOCK_PROFILE);
    assert_eq!(
        imported_lock.producer.as_ref().unwrap().generator,
        "codex_builtin_image_gen"
    );
    assert!(
        !imported_lock
            .producer
            .as_ref()
            .unwrap()
            .provider_request_occurred
    );
    assert_eq!(
        imported_lock
            .downstream_binding
            .as_ref()
            .unwrap()
            .provider_id,
        source_lock.provider_id
    );
    assert!(completed.artifacts.iter().any(|artifact| {
        artifact.kind == "direction_grid_native_review_package"
            && artifact.path.ends_with("native-review-package.png")
    }));
    let authority_alignment: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/imported-direction-grid/authority-alignment-report.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(authority_alignment["exactByteReuse"], true);
    assert_eq!(authority_alignment["sharedScale"], 1.0);
    assert_eq!(
        hash_directory(&approved_source).unwrap(),
        source_hash_before
    );

    let tamper_operation = AutomationOperation::ImportDirectionGrid(ImportDirectionGridRequestV1 {
        schema_version: "1".into(),
        source_job_dir: approved_source.clone(),
        sheet_path: None,
        direction_paths: Some(tamper_paths.clone()),
        cape_hem_contract: None,
        generator: "codex_builtin_image_gen".into(),
        note: "four named direction tamper regression".into(),
    });
    let tamper_plan = plans.prepare(tamper_operation).unwrap();
    let tamper_plan = plans.claim(&tamper_plan.token).unwrap();
    let tamper_job = stage_plan_job(&jobs, &tamper_plan).unwrap();
    let mut changed = image::open(&tamper_paths.right).unwrap().to_rgba8();
    changed.put_pixel(0, 0, Rgba([255, 0, 0, 255]));
    changed.save(&tamper_paths.right).unwrap();
    let error = run_operation(&jobs, &tamper_job.job_id, &tamper_plan.operation).unwrap_err();
    assert!(error.to_string().contains("inputs changed after staging"));
    assert_eq!(
        hash_directory(&approved_source).unwrap(),
        source_hash_before
    );
}

#[test]
fn fixture_topdown_cycle_v10_reuses_approved_v9_and_selects_one_scaled_walk_cycle() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);
    let source_lock_path = approved_source.join("source/direction-grid-lock.json");
    let source_lock_sha256 = hash_file(&source_lock_path).unwrap();

    let mut cycle = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    cycle.workflow = CharacterWorkflowSelection {
        id: "topdown-cycle".into(),
        version: "10.0.0".into(),
    };
    cycle.reuse_from_job_dir = Some(approved_source.clone());
    cycle.motion_profile = CharacterMotionProfileV1::BipedWalk;
    cycle.validation_only = true;
    cycle.validation_animations = vec!["walk_down".into()];
    cycle.metadata.default_animation = "walk_down".into();
    cycle.generation = GenerationPolicy {
        max_attempts_per_animation: 1,
        target_frame_count: 12,
        video_duration_seconds: 4,
        image_model: Some("fixture-grid".into()),
        video_model: Some("fixture-video".into()),
        pose_guidance: GridPoseGuidanceV1::Disabled,
    };
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(cycle))
        .unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 1);
    assert_eq!(plan.estimate.maximum_provider_requests, 1);
    assert_eq!(plan.estimate.model.as_deref(), Some("fixture-video"));
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        for relative in [
            "animation-quality-report.json",
            "character-gait-cycle-report.json",
            "character-motion-semantics-report.json",
            "character-silhouette-temporal-report.json",
            "character-scale-lock.json",
        ] {
            let path = completed.job_dir.join(relative);
            if path.is_file() {
                eprintln!("--- {relative} ---\n{}", fs::read_to_string(path).unwrap());
            }
        }
    }
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "code={:?} summary={:?} job={}",
        completed.error_code,
        completed.error_summary,
        completed.job_dir.display()
    );
    assert_eq!(provider.usage().requests, 1);
    assert_eq!(provider.usage().generated_videos, 1);
    assert_eq!(hash_file(&source_lock_path).unwrap(), source_lock_sha256);
    let sampling: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("animations/walk_down/source-cycle-sampling-report.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        sampling["outputFrameCount"].as_u64().unwrap(),
        8 | 10 | 12
    ));
    let scale_lock: CharacterScaleLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("character-scale-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        scale_lock.verdict,
        forge_core::asset_project::ConsistencyVerdict::GameReady,
        "{:?}",
        scale_lock.reasons
    );
    assert_eq!(scale_lock.direction_grid_lock_sha256, source_lock_sha256);
    assert!(completed
        .artifacts
        .iter()
        .all(|artifact| artifact.kind != "gsfpack"));
    assert!(completed.job_dir.join("workflow-graph.json").is_file());
}

#[test]
fn fixture_topdown_cycle_v10_real_boundary_is_exact_one_video_and_awaits_review() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    // Make the otherwise identical offline source carry xAI provenance and
    // rebind its approval. This remains a local fixture; no network is used.
    let lock_path = approved_source.join("source/direction-grid-lock.json");
    let mut lock: DirectionGridLockV1 =
        serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    lock.provider_id = "xai".into();
    lock.profile_id = "default".into();
    lock.image_model = "grok-imagine-image-quality".into();
    fs::write(&lock_path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
    let approval_path = approved_source.join(GRID_APPROVAL_FILE);
    let mut approval: DirectionGridApprovalV1 =
        serde_json::from_slice(&fs::read(&approval_path).unwrap()).unwrap();
    approval.lock_sha256 = hash_file(&lock_path).unwrap();
    fs::write(
        &approval_path,
        serde_json::to_vec_pretty(&approval).unwrap(),
    )
    .unwrap();
    let mut xai_project = read_project(&project).unwrap();
    xai_project.provider.id = "xai".into();
    xai_project.provider.profile_id = "default".into();
    fs::write(
        project.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&xai_project).unwrap(),
    )
    .unwrap();

    let mut cycle = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    cycle.provider_id = "xai".into();
    cycle.workflow = CharacterWorkflowSelection {
        id: "topdown-cycle".into(),
        version: "10.0.0".into(),
    };
    cycle.reuse_from_job_dir = Some(approved_source);
    cycle.motion_profile = CharacterMotionProfileV1::BipedWalk;
    cycle.validation_only = true;
    cycle.validation_animations = vec!["walk_down".into()];
    cycle.metadata.default_animation = "walk_down".into();
    cycle.generation = GenerationPolicy {
        max_attempts_per_animation: 1,
        target_frame_count: 12,
        video_duration_seconds: 4,
        image_model: Some("grok-imagine-image-quality".into()),
        video_model: Some("grok-imagine-video-1.5".into()),
        pose_guidance: GridPoseGuidanceV1::Disabled,
    };

    let mut full = cycle.clone();
    full.validation_only = false;
    full.validation_animations.clear();
    full.metadata.default_animation = "idle_down".into();
    let full_error = plans
        .prepare(AutomationOperation::GenerateCharacterPack(full))
        .unwrap_err();
    assert!(
        full_error
            .to_string()
            .contains("topdown_cycle_v10_real_scope_forbidden"),
        "{full_error}"
    );
    let mut long = cycle.clone();
    long.generation.video_duration_seconds = 5;
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(long))
        .unwrap_err()
        .to_string()
        .contains("topdown_cycle_v10_real_scope_forbidden"));

    let unvalidated_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(cycle.clone()))
        .unwrap();
    let unvalidated_plan = plans.claim(&unvalidated_plan.token).unwrap();
    let mut unvalidated_job = stage_plan_job(&jobs, &unvalidated_plan).unwrap();
    unvalidated_job = jobs
        .update_record(&unvalidated_job.job_id, |record| {
            record.authorization_id = Some("offline-v10-unvalidated".into());
            record.authorization_manifest_sha256 = Some("b".repeat(64));
        })
        .unwrap();
    let unvalidated = UnvalidatedXaiProvider::new(
        "offline-v10-unvalidated",
        unvalidated_job
            .authorization_manifest_sha256
            .clone()
            .unwrap(),
    );
    let error = run_operation_with_provider(
        &jobs,
        &unvalidated_job.job_id,
        &unvalidated_plan.operation,
        Some(&unvalidated),
    )
    .unwrap_err();
    assert!(error.to_string().contains("authorization"), "{error}");
    assert_eq!(unvalidated.usage().requests, 0);

    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(cycle))
        .unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 1);
    assert_eq!(plan.estimate.maximum_provider_requests, 1);
    assert_eq!(
        plan.estimate.model.as_deref(),
        Some("grok-imagine-video-1.5")
    );
    let plan = plans.claim(&plan.token).unwrap();
    let mut job = stage_plan_job(&jobs, &plan).unwrap();
    assert!(job.authorization_id.is_none());
    let authorization_id = "offline-v10-exact-auth";
    let manifest_sha256 = "a".repeat(64);
    job = jobs
        .update_record(&job.job_id, |record| {
            record.authorization_id = Some(authorization_id.into());
            record.authorization_manifest_sha256 = Some(manifest_sha256.clone());
        })
        .unwrap();
    let provider = AuthorizedV10XaiProvider {
        fixture: FixtureProvider::default(),
        authorization_id: authorization_id.into(),
        authorization_manifest_sha256: manifest_sha256,
        expected_lineage: job.lineage_root_job_id.clone().unwrap(),
        expected_recipe_hash: plan.recipe_hash.clone(),
        expected_input_fingerprint: plan.input_fingerprint.clone(),
        scope_checks: Mutex::new(0),
    };
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    assert_eq!(*provider.scope_checks.lock().unwrap(), 1);
    assert_eq!(provider.usage().requests, 1);
    assert_eq!(provider.usage().generated_videos, 1);
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(
        completed.error_code.as_deref(),
        Some("topdown_cycle_v10_native_review_required")
    );
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    let observations = provider.fixture.request_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(observations[0].kind, FixtureRequestKind::GenerateVideo);
    assert_eq!(
        observations[0].authorization_target.as_deref(),
        Some("walk_down:video")
    );
}

#[test]
fn fixture_topdown_cycle_v10_plan_rejects_tampered_v9_anchor_before_video_request() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);
    let lock: DirectionGridLockV1 = serde_json::from_slice(
        &fs::read(approved_source.join("source/direction-grid-lock.json")).unwrap(),
    )
    .unwrap();
    let mut tampered = fs::read(&lock.node("front_idle").unwrap().path).unwrap();
    tampered.push(0);
    fs::write(&lock.node("front_idle").unwrap().path, tampered).unwrap();

    let mut cycle = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    cycle.workflow = CharacterWorkflowSelection {
        id: "topdown-cycle".into(),
        version: "10.0.0".into(),
    };
    cycle.reuse_from_job_dir = Some(approved_source);
    cycle.motion_profile = CharacterMotionProfileV1::BipedWalk;
    cycle.validation_only = true;
    cycle.validation_animations = vec!["walk_down".into()];
    cycle.metadata.default_animation = "walk_down".into();
    cycle.generation = GenerationPolicy {
        max_attempts_per_animation: 1,
        target_frame_count: 12,
        video_duration_seconds: 4,
        image_model: Some("fixture-grid".into()),
        video_model: Some("fixture-video".into()),
        pose_guidance: GridPoseGuidanceV1::Disabled,
    };
    let error = plans
        .prepare(AutomationOperation::GenerateCharacterPack(cycle))
        .unwrap_err();
    assert!(error.to_string().contains("hash changed after approval"));
}

#[test]
fn fixture_topdown_cycle_v10_full_pack_has_eight_animations_and_godot_contract() {
    let (temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);
    let source_lock_path = approved_source.join("source/direction-grid-lock.json");
    let source_lock_sha256 = hash_file(&source_lock_path).unwrap();

    let mut cycle = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    cycle.workflow = CharacterWorkflowSelection {
        id: "topdown-cycle".into(),
        version: "10.0.0".into(),
    };
    cycle.reuse_from_job_dir = Some(approved_source);
    cycle.motion_profile = CharacterMotionProfileV1::BipedWalk;
    cycle.metadata.default_animation = "idle_down".into();
    cycle.generation = GenerationPolicy {
        max_attempts_per_animation: 1,
        target_frame_count: 12,
        video_duration_seconds: 4,
        image_model: Some("fixture-grid".into()),
        video_model: Some("fixture-video".into()),
        pose_guidance: GridPoseGuidanceV1::Disabled,
    };
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(cycle))
        .unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 4);
    assert_eq!(plan.estimate.maximum_provider_requests, 4);
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        for relative in [
            "animation-quality-report.json",
            "character-semantic-quality-report.json",
            "character-motion-semantics-report.json",
            "character-silhouette-temporal-report.json",
            "character-scale-lock.json",
        ] {
            let path = completed.job_dir.join(relative);
            if path.is_file() {
                eprintln!("--- {relative} ---\n{}", fs::read_to_string(path).unwrap());
            }
        }
    }
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "code={:?} summary={:?} job={}",
        completed.error_code,
        completed.error_summary,
        completed.job_dir.display()
    );
    assert_eq!(provider.usage().requests, 4);
    assert_eq!(provider.usage().generated_videos, 4);
    assert_eq!(hash_file(&source_lock_path).unwrap(), source_lock_sha256);
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("complete V10 fixture exports a Pack");
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    let animations = forgepack["animations"].as_array().unwrap();
    assert_eq!(animations.len(), 8);
    assert_eq!(
        animations
            .iter()
            .map(|animation| animation["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "idle_down",
            "idle_up",
            "idle_right",
            "idle_left",
            "walk_down",
            "walk_up",
            "walk_right",
            "walk_left",
        ]
    );
    assert!(pack.path.join("character-direction-lock.json").is_file());
    assert!(pack.path.join("character-scale-lock.json").is_file());
    assert_no_secret_or_jobstore_leak(&pack.path);

    let godot = PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot");
    if godot.is_file() {
        let godot_project = temp.path().join("godot-cycle-v10");
        fs::create_dir_all(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge Cycle V10 Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let validated_pack = temp.path().join("validated-cycle-v10-pack.gsfpack");
        copy_directory(&pack.path, &validated_pack);
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: validated_pack,
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/cycle_v10_character"),
            asset_key: Some("cycle_v10_character".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("cycle-v10-ranger".into()),
                label: Some("Cycle V10 Ranger".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let output = std::process::Command::new(godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot V10 load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_no_secret_or_jobstore_leak(&godot_project);
    }
}

#[test]
fn fixture_grid_v92_validation_binds_grayscale_structure_and_laterality() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.2.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 4);
    assert_eq!(plan.estimate.maximum_provider_requests, 8);
    assert!(plan
        .effects
        .iter()
        .any(|effect| effect.contains("no other direction, video, Pack")));
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();

    if completed.lifecycle_state != JobLifecycleState::AwaitingReview {
        eprintln!(
            "V9.2 fixture failed: dir={} code={:?} summary={:?}",
            completed.job_dir.display(),
            completed.error_code,
            completed.error_summary
        );
        for relative in [
            "source/grid-keyframe-actions/walk_down.json",
            "source/grid-keyframe-actions/walk_down-motion.json",
            "source/grid-keyframe-actions/walk_down-laterality.json",
            "consistency-report.json",
        ] {
            let path = completed.job_dir.join(relative);
            if path.is_file() {
                eprintln!("--- {relative} ---\n{}", fs::read_to_string(path).unwrap());
            }
        }
    }
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    let observations = provider
        .request_observations()
        .into_iter()
        .filter(|observation| {
            observation
                .authorization_target
                .as_deref()
                .is_some_and(|target| target.starts_with("walk_down:frame:"))
        })
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 4);
    assert!(observations.iter().all(|observation| {
        observation.reference_roles
            == vec![ReferenceRole::DirectionAnchor, ReferenceRole::PoseStructure]
            && observation
                .prompt
                .contains("Forge structured gait keyframe V9.2")
            && observation.prompt.contains("SCREEN-")
    }));
    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report.profile, "grid-keyframe-action-report@1.2.0");
    assert_eq!(
        report.laterality_verdict,
        Some(forge_core::asset_project::ConsistencyVerdict::GameReady)
    );
    assert!(report.frames.iter().all(|frame| {
        frame.pose_structure_path.is_some()
            && frame
                .pose_structure_sha256
                .as_deref()
                .is_some_and(|hash| hash.len() == 64)
    }));
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["workflow"], "topdown-grid@9.2.0");
    assert_eq!(manifest["actions"][0]["lateralityVerdict"], "game_ready");
    let graph = read_workflow_graph(&completed.job_dir.join("workflow-graph.json")).unwrap();
    assert_eq!(graph.workflow, "topdown-grid@9.2.0");
    assert!(graph
        .nodes
        .iter()
        .filter(|node| node.stage == "independent_keyframe")
        .all(|node| {
            node.inputs.len() == 2
                && node.implementation_version == "grid-structured-keyframe@1.1.0"
        }));
}

#[test]
fn fixture_grid_v92_same_side_collapse_retries_only_opposite_half_and_preserves_evidence() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.2.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    let maximum = plan.estimate.maximum_provider_requests;
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default().with_same_side_keyposes();
    let failed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();

    assert_eq!(failed.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        failed.error_code.as_deref(),
        Some("walk_laterality_not_alternating")
    );
    assert_eq!(provider.usage().requests, 6);
    assert!(provider.usage().requests <= maximum);
    let observations = provider
        .request_observations()
        .into_iter()
        .filter(|observation| {
            observation
                .authorization_target
                .as_deref()
                .is_some_and(|target| target.starts_with("walk_down:frame:"))
        })
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 6);
    assert_eq!(
        observations
            .iter()
            .filter_map(|observation| observation.authorization_target.as_deref())
            .collect::<Vec<_>>(),
        [
            "walk_down:frame:0",
            "walk_down:frame:1",
            "walk_down:frame:2",
            "walk_down:frame:3",
            "walk_down:frame:2",
            "walk_down:frame:3",
        ]
    );
    assert!(observations[..4].iter().all(|observation| {
        observation.reference_roles
            == vec![ReferenceRole::DirectionAnchor, ReferenceRole::PoseStructure]
    }));
    assert!(observations[4..].iter().all(|observation| {
        observation.reference_roles
            == vec![ReferenceRole::DirectionAnchor, ReferenceRole::PoseStructure]
            && observation.prompt.contains("laterality-only fresh retry")
            && observation
                .prompt
                .contains("rejected frame is intentionally not provided")
    }));
    let laterality: serde_json::Value = serde_json::from_slice(
        &fs::read(
            failed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down-laterality.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(laterality["verdict"], "blocked");
    assert!(laterality["reasons"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("walk_laterality_not_alternating")));
    assert_eq!(
        laterality["recommendedRetryFrames"],
        serde_json::json!([2, 3])
    );
    let action: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            failed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for frame in action.frames.iter().filter(|frame| frame.frame_index >= 2) {
        assert_eq!(
            frame.generation_method,
            forge_core::character_grid::GridKeyframeGenerationMethodV1::LateralityFreshRetry
        );
        assert!(frame.input_frame_sha256.is_none());
        assert_eq!(frame.attempt, 2);
    }
    assert!(failed
        .job_dir
        .join("source/grid-keyframe-provider-manifest.json")
        .is_file());
    assert!(failed.job_dir.join("workflow-graph.json").is_file());
    let graph = read_workflow_graph(&failed.job_dir.join("workflow-graph.json")).unwrap();
    for frame in [2_u8, 3] {
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == format!("independent_keyframe:walk_down:{frame}"))
            .unwrap();
        assert_eq!(node.inputs.len(), 2);
        assert!(node.inputs[0].path.ends_with("front_idle.png"));
        assert!(node.inputs[1].path.ends_with("pose-structure.png"));
        assert!(node
            .inputs
            .iter()
            .all(|input| !input.path.ends_with("edit-target.png")));
        assert_eq!(
            node.implementation_version,
            "grid-structured-keyframe@1.1.0"
        );
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            failed
                .job_dir
                .join("source/grid-keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for frame in manifest["actions"][0]["frames"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|frame| frame["frameIndex"].as_u64().unwrap() >= 2)
    {
        assert_eq!(frame["generationMethod"], "laterality_fresh_retry");
        assert!(frame.get("inputFrameSha256").is_none() || frame["inputFrameSha256"].is_null());
    }
    assert!(!failed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
}

#[test]
fn fixture_grid_v92_child_validation_retries_only_recommended_frame_fresh_once() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.2.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let source_job = stage_plan_job(&jobs, &plan).unwrap();
    let source_provider = FixtureProvider::default().with_wrong_right_contact_keypose();
    let source = run_operation_with_provider(
        &jobs,
        &source_job.job_id,
        &plan.operation,
        Some(&source_provider),
    )
    .unwrap();
    assert_eq!(source.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        source.error_code.as_deref(),
        Some("walk_laterality_not_alternating")
    );
    let source_report_path = source
        .job_dir
        .join("source/grid-keyframe-actions/walk_down.json");
    let mut source_report: GridKeyframeActionReportV1 =
        serde_json::from_slice(&fs::read(&source_report_path).unwrap()).unwrap();
    assert_eq!(source_report.recommended_retry_frames, vec![2]);

    // Freeze the source in the exact legacy shape produced by the real V9.2
    // probe before laterality fresh retry existed. That Job used a diagnostic
    // EditTarget for frame 2, so its one fresh allowance remains unconsumed.
    source_report = freeze_v92_failure_as_legacy_source(&jobs, &source);

    let AutomationOperation::GenerateCharacterPack(mut retry) = plan.operation else {
        unreachable!()
    };
    retry.reuse_from_job_dir = Some(source.job_dir.clone());
    retry.retry_animations = vec!["walk_down".into()];
    retry
        .retry_stages
        .insert("walk_down".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_down".into(), vec![2]);

    let mut wrong_frame = retry.clone();
    wrong_frame.retry_frames.insert("walk_down".into(), vec![3]);
    let wrong_frame_error = plans
        .prepare(AutomationOperation::GenerateCharacterPack(wrong_frame))
        .unwrap_err()
        .to_string();
    assert!(wrong_frame_error.contains("immutable laterality recommendation"));
    let mut broad_retry = retry.clone();
    broad_retry
        .retry_frames
        .insert("walk_down".into(), vec![2, 3]);
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(broad_retry))
        .unwrap_err()
        .to_string()
        .contains("exactly one frame"));

    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry.clone()))
        .unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 1);
    assert!(retry_plan.effects.iter().any(|effect| {
        effect.contains("no other direction, video, Pack")
            || effect.contains("no other direction, video, Pack, catalog entry")
    }));
    assert!(retry_plan.effects.iter().any(|effect| {
        effect.contains("fresh-retry only immutable laterality-recommended walk_down frame 2")
            && effect.contains("byte-reuse the other three frames")
            && effect.contains("no second request")
    }));
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let retry_provider = FixtureProvider::default();
    let completed = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&retry_provider),
    )
    .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(retry_provider.usage().requests, 1);
    let observations = retry_provider.request_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(
        observations[0].authorization_target.as_deref(),
        Some("walk_down:frame:2")
    );
    assert_eq!(
        observations[0].reference_roles,
        vec![ReferenceRole::DirectionAnchor, ReferenceRole::PoseStructure]
    );
    assert!(observations[0]
        .prompt
        .contains("laterality-only fresh retry"));
    assert!(!completed
        .job_dir
        .join("source/provider/walk_down/frame-02/attempt-3/edit-target.png")
        .exists());

    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report.profile, "grid-keyframe-action-report@1.2.0");
    assert_eq!(
        report.laterality_verdict,
        Some(forge_core::asset_project::ConsistencyVerdict::GameReady)
    );
    for index in [0_usize, 1, 3] {
        assert_eq!(
            report.frames[index].generation_method,
            GridKeyframeGenerationMethodV1::ByteReuse
        );
        assert_eq!(
            report.frames[index].sha256,
            source_report.frames[index].sha256
        );
        assert!(report.frames[index].reused);
        assert!(!report.frames[index].provider_request_occurred);
        assert!(report.frames[index]
            .pose_structure_path
            .as_ref()
            .unwrap()
            .starts_with(&completed.job_dir));
    }
    assert_eq!(
        report.frames[2].generation_method,
        GridKeyframeGenerationMethodV1::LateralityFreshRetry
    );
    assert_eq!(report.frames[2].attempt, 3);
    assert!(report.frames[2].input_frame_sha256.is_none());
    assert!(report.frames[2].replaces_frame_sha256.is_none());
    assert_ne!(report.frames[2].sha256, source_report.frames[2].sha256);
    assert_eq!(report.retried_frames, vec![2]);

    let graph = read_workflow_graph(&completed.job_dir.join("workflow-graph.json")).unwrap();
    let selected_node = graph
        .nodes
        .iter()
        .find(|node| node.id == "independent_keyframe:walk_down:2")
        .unwrap();
    assert_eq!(selected_node.inputs.len(), 2);
    assert!(selected_node.inputs[0].path.ends_with("front_idle.png"));
    assert!(selected_node.inputs[1].path.ends_with("pose-structure.png"));
    assert!(selected_node
        .inputs
        .iter()
        .all(|input| input.sha256 != source_report.frames[2].sha256));
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));

    let failed_retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    assert_eq!(failed_retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(failed_retry_plan.estimate.maximum_provider_requests, 1);
    let failed_retry_plan = plans.claim(&failed_retry_plan.token).unwrap();
    let failed_retry_job = stage_plan_job(&jobs, &failed_retry_plan).unwrap();
    let invalid_provider = FixtureProvider::default().with_invalid_laterality_fresh_retry();
    let failed_retry = run_operation_with_provider(
        &jobs,
        &failed_retry_job.job_id,
        &failed_retry_plan.operation,
        Some(&invalid_provider),
    )
    .unwrap();
    assert_eq!(invalid_provider.usage().requests, 1);
    assert_eq!(failed_retry.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        failed_retry.error_code.as_deref(),
        Some("grid_keyframe_action_quality_failed")
    );
    assert!(failed_retry
        .error_summary
        .as_deref()
        .unwrap()
        .contains("walk_down"));
    assert!(failed_retry
        .job_dir
        .join("source/grid-keyframe-actions/walk_down.json")
        .is_file());
    assert!(failed_retry
        .job_dir
        .join("source/grid-keyframe-provider-manifest.json")
        .is_file());
    assert!(failed_retry.job_dir.join("workflow-graph.json").is_file());
    assert!(!failed_retry
        .job_dir
        .join("source/provider/walk_down/frame-02/attempt-3/edit-target.png")
        .exists());
    assert!(!failed_retry
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));

    let AutomationOperation::GenerateCharacterPack(mut chained_retry) = failed_retry_plan.operation
    else {
        unreachable!()
    };
    chained_retry.reuse_from_job_dir = Some(failed_retry.job_dir.clone());
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(chained_retry))
        .unwrap_err()
        .to_string()
        .contains("not a stable laterality failure"));
}

#[test]
fn fixture_grid_v93_child_uses_asymmetric_guide_for_only_recommended_frame() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.2.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let source_job = stage_plan_job(&jobs, &plan).unwrap();
    let source = run_operation_with_provider(
        &jobs,
        &source_job.job_id,
        &plan.operation,
        Some(&FixtureProvider::default().with_wrong_right_contact_keypose()),
    )
    .unwrap();
    assert_eq!(source.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        source.error_code.as_deref(),
        Some("walk_laterality_not_alternating")
    );
    let source_report_path = source
        .job_dir
        .join("source/grid-keyframe-actions/walk_down.json");
    let mut source_report: GridKeyframeActionReportV1 =
        serde_json::from_slice(&fs::read(&source_report_path).unwrap()).unwrap();
    assert_eq!(source_report.recommended_retry_frames, vec![2]);
    source_report = freeze_v92_failure_as_legacy_source(&jobs, &source);

    let AutomationOperation::GenerateCharacterPack(mut retry) = plan.operation else {
        unreachable!()
    };
    retry.workflow.version = "9.3.0".into();
    retry.reuse_from_job_dir = Some(source.job_dir.clone());
    retry.retry_animations = vec!["walk_down".into()];
    retry
        .retry_stages
        .insert("walk_down".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_down".into(), vec![2]);
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry.clone()))
        .unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 1);
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let provider = FixtureProvider::default();
    let completed = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(provider.usage().requests, 1);
    let observations = provider.request_observations();
    assert_eq!(observations.len(), 1);
    let prompt = &observations[0].prompt;
    assert!(prompt.contains("Forge asymmetric gait keyframe V9.3"));
    assert!(prompt.contains("screen_right_contact"));
    assert!(prompt.contains("wide horizontal sole"));
    assert!(prompt.contains("never the character's anatomical left/right"));
    assert!(!prompt.contains("(right contact)"));
    assert_eq!(
        observations[0].reference_roles,
        vec![ReferenceRole::DirectionAnchor, ReferenceRole::PoseStructure]
    );
    let edits = provider.edit_observations();
    assert_eq!(edits.len(), 1);

    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report.profile, "grid-keyframe-action-report@1.3.0");
    assert_eq!(
        report.frames[2].generation_method,
        GridKeyframeGenerationMethodV1::AsymmetricGuideFreshRetry
    );
    assert_eq!(
        report.frames[2].pose_structure_profile.as_deref(),
        Some("grid-pose-structure@1.1.0")
    );
    for index in [0_usize, 1, 3] {
        assert_eq!(
            report.frames[index].pose_structure_profile.as_deref(),
            Some("grid-pose-structure@1.0.0")
        );
        assert_eq!(
            report.frames[index].sha256,
            source_report.frames[index].sha256
        );
        assert_eq!(
            report.frames[index].pose_structure_sha256,
            source_report.frames[index].pose_structure_sha256
        );
    }
    assert_ne!(
        report.frames[2].pose_structure_sha256,
        source_report.frames[2].pose_structure_sha256
    );
    assert_eq!(edits[0].reference_sha256[0], report.direction_anchor_sha256);
    assert_eq!(
        edits[0].reference_sha256[1],
        report.frames[2].pose_structure_sha256.clone().unwrap()
    );
    let graph = read_workflow_graph(&completed.job_dir.join("workflow-graph.json")).unwrap();
    assert_eq!(graph.workflow, "topdown-grid@9.3.0");
    let selected = graph
        .nodes
        .iter()
        .find(|node| node.id == "independent_keyframe:walk_down:2")
        .unwrap();
    assert_eq!(
        selected.implementation_version,
        "grid-structured-keyframe@1.2.0"
    );
    assert!(selected.provider_request);
    assert!(!selected.cache_hit);
    assert_eq!(selected.inputs[0].sha256, report.direction_anchor_sha256);
    assert_eq!(
        selected.inputs[1].sha256,
        report.frames[2].pose_structure_sha256.clone().unwrap()
    );
    for frame in [0_u8, 1, 3] {
        let reused = graph
            .nodes
            .iter()
            .find(|node| node.id == format!("independent_keyframe:walk_down:{frame}"))
            .unwrap();
        assert_eq!(reused.implementation_version, "grid-byte-reuse@1.1.0");
        assert!(reused.provider_id.is_none());
        assert!(reused.model.is_none());
        assert!(!reused.provider_request);
        assert!(reused.cache_hit);
        assert_eq!(reused.inputs.len(), 3);
        assert_eq!(
            reused.inputs[0].sha256,
            source_report.frames[frame as usize].sha256
        );
        assert_eq!(
            reused.inputs[0].path,
            source_report.frames[frame as usize].path
        );
        assert_eq!(reused.inputs[1].sha256, report.direction_anchor_sha256);
        assert_eq!(
            reused.inputs[2].sha256,
            report.frames[frame as usize]
                .pose_structure_sha256
                .clone()
                .unwrap()
        );
        assert_eq!(
            reused.outputs[0].sha256,
            source_report.frames[frame as usize].sha256
        );
        assert_eq!(reused.outputs[0].path, report.frames[frame as usize].path);
        assert_eq!(
            reused.cache_key,
            compute_artifact_cache_key(
                "independent_keyframe",
                "grid-byte-reuse@1.1.0",
                None,
                None,
                &serde_json::json!({
                    "action": "walk_down",
                    "frame": frame,
                    "method": "byte_reuse",
                }),
                &reused.inputs,
            )
            .unwrap()
        );
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["workflow"], "topdown-grid@9.3.0");
    assert_eq!(
        manifest["actions"][0]["actionReportProfile"],
        "grid-keyframe-action-report@1.3.0"
    );
    assert_eq!(
        manifest["actions"][0]["actionReportSha256"],
        hash_file(
            &completed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json")
        )
        .unwrap()
    );
    assert_eq!(manifest["actions"][0]["frames"][0]["reused"], true);
    assert_eq!(manifest["actions"][0]["frames"][2]["reused"], false);
    assert_eq!(
        manifest["actions"][0]["frames"][2]["poseStructureProfile"],
        "grid-pose-structure@1.1.0"
    );
    assert_eq!(manifest["usage"]["requests"], 1);
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));

    let retry_for_pose_tamper = retry.clone();
    let sibling_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    let sibling_plan = plans.claim(&sibling_plan.token).unwrap();
    let sibling_error = stage_plan_job(&jobs, &sibling_plan).unwrap_err();
    assert!(sibling_error
        .to_string()
        .contains("asymmetric_gait_source_already_consumed"));
    assert_eq!(jobs.list_children(&source.job_id).unwrap().len(), 1);

    let source_pose_path = source_report.frames[2]
        .pose_structure_path
        .as_ref()
        .unwrap();
    let source_pose_bytes = fs::read(source_pose_path).unwrap();
    let mut tampered_pose_bytes = source_pose_bytes.clone();
    tampered_pose_bytes.push(0);
    fs::write(source_pose_path, tampered_pose_bytes).unwrap();
    let pose_tamper_error = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            retry_for_pose_tamper,
        ))
        .unwrap_err();
    assert!(pose_tamper_error
        .to_string()
        .contains("structured PoseStructure 2 changed after assessment"));
    fs::write(source_pose_path, source_pose_bytes).unwrap();
}

#[test]
fn fixture_grid_v94_child_replaces_only_platform_leaked_frame_with_v12_guide() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut v92 = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    v92.workflow.version = "9.2.0".into();
    v92.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    v92.reuse_from_job_dir = Some(approved_source);
    v92.validation_only = true;
    v92.validation_animations = vec!["walk_down".into()];
    v92.metadata.default_animation = "walk_down".into();
    let v92_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(v92))
        .unwrap();
    let v92_plan = plans.claim(&v92_plan.token).unwrap();
    let v92_job = stage_plan_job(&jobs, &v92_plan).unwrap();
    let v92_source = run_operation_with_provider(
        &jobs,
        &v92_job.job_id,
        &v92_plan.operation,
        Some(&FixtureProvider::default().with_wrong_right_contact_keypose()),
    )
    .unwrap();
    freeze_v92_failure_as_legacy_source(&jobs, &v92_source);

    let AutomationOperation::GenerateCharacterPack(mut v93) = v92_plan.operation else {
        unreachable!()
    };
    v93.workflow.version = "9.3.0".into();
    v93.reuse_from_job_dir = Some(v92_source.job_dir.clone());
    v93.retry_animations = vec!["walk_down".into()];
    v93.retry_stages
        .insert("walk_down".into(), CharacterRetryStage::Frame);
    v93.retry_frames.insert("walk_down".into(), vec![2]);
    let v93_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(v93))
        .unwrap();
    let v93_plan = plans.claim(&v93_plan.token).unwrap();
    let v93_job = stage_plan_job(&jobs, &v93_plan).unwrap();
    let v93_source = run_operation_with_provider(
        &jobs,
        &v93_job.job_id,
        &v93_plan.operation,
        Some(&FixtureProvider::default().with_platform_sole_leak()),
    )
    .unwrap();
    assert_eq!(
        v93_source.lifecycle_state,
        JobLifecycleState::AwaitingReview,
        "code={:?} summary={:?}",
        v93_source.error_code,
        v93_source.error_summary
    );
    let v93_report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            v93_source
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        forge_core::footwear_platform::assess_footwear_platform(
            "walk_down",
            2,
            &image::open(&v93_report.frames[2].path).unwrap().to_rgba8(),
        )
        .verdict,
        forge_core::asset_project::ConsistencyVerdict::Blocked
    );

    let AutomationOperation::GenerateCharacterPack(mut v94) = v93_plan.operation else {
        unreachable!()
    };
    v94.workflow.version = "9.4.0".into();
    v94.reuse_from_job_dir = Some(v93_source.job_dir.clone());
    let v94_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(v94))
        .unwrap();
    assert_eq!(v94_plan.estimate.provider_request_estimate, 1);
    assert_eq!(v94_plan.estimate.maximum_provider_requests, 1);
    let v94_plan = plans.claim(&v94_plan.token).unwrap();
    let v94_job = stage_plan_job(&jobs, &v94_plan).unwrap();
    let provider = FixtureProvider::default();
    let completed =
        run_operation_with_provider(&jobs, &v94_job.job_id, &v94_plan.operation, Some(&provider))
            .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(provider.usage().requests, 1);
    let observation = provider.request_observations().pop().unwrap();
    assert_eq!(
        observation.reference_roles,
        vec![ReferenceRole::DirectionAnchor, ReferenceRole::PoseStructure]
    );
    assert!(observation
        .prompt
        .contains("Forge platform-safe gait keyframe V9.4"));
    assert!(observation.prompt.contains("no shoe sole"));
    assert!(observation
        .prompt
        .contains("Never invent or copy a platform"));
    assert!(!observation.prompt.contains("wide horizontal sole means"));

    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report.profile, "grid-keyframe-action-report@1.4.0");
    assert_eq!(
        report.frames[2].generation_method,
        GridKeyframeGenerationMethodV1::PlatformSafeGuideFreshRetry
    );
    assert_eq!(
        report.frames[2].pose_structure_profile.as_deref(),
        Some("grid-pose-structure@1.2.0")
    );
    assert_eq!(
        report.footwear_verdict,
        Some(forge_core::asset_project::ConsistencyVerdict::GameReady)
    );
    for index in [0_usize, 1, 3] {
        assert_eq!(report.frames[index].sha256, v93_report.frames[index].sha256);
        assert_eq!(
            report.frames[index].generation_method,
            GridKeyframeGenerationMethodV1::ByteReuse
        );
    }
    assert_ne!(report.frames[2].sha256, v93_report.frames[2].sha256);
    let pose = image::open(report.frames[2].pose_structure_path.as_ref().unwrap())
        .unwrap()
        .to_rgba8();
    forge_core::grid_pose_structure::validate_walk_down_pose_structure_v12(&pose, 2).unwrap();
    assert_eq!(
        forge_core::footwear_platform::assess_footwear_platform(
            "walk_down",
            2,
            &image::open(&report.frames[2].path).unwrap().to_rgba8(),
        )
        .verdict,
        forge_core::asset_project::ConsistencyVerdict::GameReady
    );
    let graph = read_workflow_graph(&completed.job_dir.join("workflow-graph.json")).unwrap();
    assert_eq!(graph.workflow, "topdown-grid@9.4.0");
    assert_eq!(
        graph
            .nodes
            .iter()
            .find(|node| node.id == "independent_keyframe:walk_down:2")
            .unwrap()
            .implementation_version,
        "grid-structured-keyframe@1.3.0"
    );
    assert!(completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "grid_keyframe_footwear_walk_down"));
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
}

fn staged_v93_laterality_retry() -> (
    tempfile::TempDir,
    PlanStore,
    JobStore,
    forge_core::job::JobRecord,
    forge_core::automation::AutomationPlan,
) {
    let (temp, project, style, subject, plans, jobs) = setup();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.2.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let source_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    let source_plan = plans.claim(&source_plan.token).unwrap();
    let source_job = stage_plan_job(&jobs, &source_plan).unwrap();
    let source = run_operation_with_provider(
        &jobs,
        &source_job.job_id,
        &source_plan.operation,
        Some(&FixtureProvider::default().with_wrong_right_contact_keypose()),
    )
    .unwrap();
    assert_eq!(source.lifecycle_state, JobLifecycleState::Failed);
    freeze_v92_failure_as_legacy_source(&jobs, &source);

    let AutomationOperation::GenerateCharacterPack(mut retry) = source_plan.operation else {
        unreachable!()
    };
    retry.workflow.version = "9.3.0".into();
    retry.reuse_from_job_dir = Some(source.job_dir.clone());
    retry.retry_animations = vec!["walk_down".into()];
    retry
        .retry_stages
        .insert("walk_down".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_down".into(), vec![2]);
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    let retry_plan = plans.claim(&retry_plan.token).unwrap();

    (temp, plans, jobs, source, retry_plan)
}

#[test]
fn fixture_grid_v93_rejects_xai_provider_without_explicit_scope_proof_before_request() {
    let (_temp, plans, jobs, _source, retry_plan) = staged_v93_laterality_retry();
    let AutomationOperation::GenerateCharacterPack(mut retry) = retry_plan.operation else {
        unreachable!()
    };
    retry.provider_id = "xai".into();
    let project_path = retry.project_path.as_ref().unwrap();
    let mut project = read_project(project_path).unwrap();
    project.provider.id = "xai".into();
    fs::write(
        project_path.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    for lock_path in [
        retry.style_lock_path.as_ref().unwrap(),
        retry.subject_lock_path.as_ref().unwrap(),
    ] {
        let mut lock: serde_json::Value =
            serde_json::from_slice(&fs::read(lock_path).unwrap()).unwrap();
        lock["providerId"] = serde_json::json!("xai");
        fs::write(lock_path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
    }
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let manifest_sha256 = "7a".repeat(32);
    jobs.update_record(&retry_job.job_id, |record| {
        record.authorization_id = Some("matching-durable-authorization".into());
        record.authorization_manifest_sha256 = Some(manifest_sha256.clone());
    })
    .unwrap();

    let provider = UnvalidatedXaiProvider::new("matching-durable-authorization", manifest_sha256);
    let error = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&provider),
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("durable authorization scope validation is not implemented"),
        "{error}"
    );
    assert_eq!(provider.usage().requests, 0);
}

#[test]
fn fixture_grid_v93_staged_child_rejects_tampered_source_frame_before_provider_request() {
    let (_temp, _plans, jobs, source, retry_plan) = staged_v93_laterality_retry();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            source
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let frame_path = report.frames[0].path.clone();
    let frame_bytes = fs::read(&frame_path).unwrap();
    let mut tampered = frame_bytes.clone();
    tampered.push(0);
    fs::write(&frame_path, tampered).unwrap();

    let provider = FixtureProvider::default();
    let error = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&provider),
    )
    .unwrap_err();

    assert!(
        error.to_string().contains("inputs changed after staging"),
        "{error}"
    );
    assert_eq!(provider.usage().requests, 0);
    fs::write(frame_path, frame_bytes).unwrap();
}

#[test]
fn fixture_grid_v93_staged_child_rejects_tampered_pose_before_provider_request() {
    let (_temp, _plans, jobs, source, retry_plan) = staged_v93_laterality_retry();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            source
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let pose_path = report.frames[0].pose_structure_path.clone().unwrap();
    let pose_bytes = fs::read(&pose_path).unwrap();
    let mut tampered = pose_bytes.clone();
    tampered.push(0);
    fs::write(&pose_path, tampered).unwrap();

    let provider = FixtureProvider::default();
    let error = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&provider),
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("structured PoseStructure 2 changed after assessment"));
    assert_eq!(provider.usage().requests, 0);
    fs::write(pose_path, pose_bytes).unwrap();
}

#[test]
fn fixture_grid_v93_rejects_pose_structure_changed_after_provider_request() {
    let (_temp, _plans, jobs, _source, retry_plan) = staged_v93_laterality_retry();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let provider = PostSendReferenceMutatingProvider::new(ReferenceRole::PoseStructure);

    let error = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&provider),
    )
    .unwrap_err();

    assert!(
        error.to_string().contains(
            "grid_keyframe_reference_changed_during_provider: walk_down frame 2 role PoseStructure"
        ),
        "{error}"
    );
    assert_eq!(provider.usage().requests, 1);
    let observed = provider.observed_reference_sha256();
    assert_eq!(observed.len(), 1);

    let record = jobs.read_record(&retry_job.job_id).unwrap();
    let report_path = record
        .job_dir
        .join("source/grid-keyframe-actions/walk_down.json");
    assert!(
        !report_path.exists(),
        "a post-send reference mutation must not produce an Action report"
    );
    let graph_path = record.job_dir.join("workflow-graph.json");
    assert!(
        !graph_path.exists(),
        "a post-send reference mutation must not produce a Workflow Graph"
    );
}

#[test]
fn fixture_grid_v93_ignored_asymmetric_guide_fails_on_an_independent_source() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let direction_plan = plans.claim(&direction_plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &direction_plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &direction_plan.operation,
        Some(&FixtureProvider::default()),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.2.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let source_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    let source_plan = plans.claim(&source_plan.token).unwrap();
    let source_job = stage_plan_job(&jobs, &source_plan).unwrap();
    let source = run_operation_with_provider(
        &jobs,
        &source_job.job_id,
        &source_plan.operation,
        Some(&FixtureProvider::default().with_wrong_right_contact_keypose()),
    )
    .unwrap();
    assert_eq!(source.lifecycle_state, JobLifecycleState::Failed);
    freeze_v92_failure_as_legacy_source(&jobs, &source);

    let AutomationOperation::GenerateCharacterPack(mut retry) = source_plan.operation else {
        unreachable!()
    };
    retry.workflow.version = "9.3.0".into();
    retry.reuse_from_job_dir = Some(source.job_dir.clone());
    retry.retry_animations = vec!["walk_down".into()];
    retry
        .retry_stages
        .insert("walk_down".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_down".into(), vec![2]);
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    // Ignoring the asymmetric guide must retain the source failure: frame 2
    // still chooses screen-left contact. The two fixture switches model those
    // independent facts explicitly instead of relying on the ordinary
    // keyframe fallback, which correctly chooses screen-right contact.
    let provider = FixtureProvider::default()
        .with_wrong_right_contact_keypose()
        .ignoring_asymmetric_pose_structure();
    let failed = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(provider.usage().requests, 1);
    assert_eq!(failed.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        failed.error_code.as_deref(),
        Some("walk_laterality_not_alternating")
    );
    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            failed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(report.retried_frames, vec![2]);
    assert_eq!(
        report.frames[2].generation_method,
        GridKeyframeGenerationMethodV1::AsymmetricGuideFreshRetry
    );
    let AutomationOperation::GenerateCharacterPack(mut chained) = retry_plan.operation else {
        unreachable!()
    };
    chained.reuse_from_job_dir = Some(failed.job_dir.clone());
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(chained))
        .unwrap_err()
        .to_string()
        .contains("already consumed its one laterality fresh retry"));
}

#[test]
fn fixture_grid_v92_plan_rejects_scope_and_guidance_expansion() {
    let (_temp, project, style, subject, plans, _jobs) = setup();
    let source = _temp.path().join("approved-v9-source");
    fs::create_dir_all(source.join("source")).unwrap();
    fs::write(source.join("job.json"), "{}").unwrap();
    fs::write(source.join("source/direction-grid-lock.json"), "{}").unwrap();
    fs::write(source.join(GRID_APPROVAL_FILE), "{}").unwrap();
    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.2.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Grayscale;
    probe.reuse_from_job_dir = Some(source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();

    let mut full = probe.clone();
    full.validation_only = false;
    full.validation_animations.clear();
    full.metadata.default_animation = "idle_down".into();
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(full))
        .unwrap_err()
        .to_string()
        .contains("validation-only"));

    let mut wrong_direction = probe.clone();
    wrong_direction.validation_animations = vec!["walk_up".into()];
    wrong_direction.metadata.default_animation = "walk_up".into();
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(wrong_direction))
        .unwrap_err()
        .to_string()
        .contains("exactly walk_down"));

    let mut legacy_grayscale = probe.clone();
    legacy_grayscale.workflow.version = "9.0.0".into();
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(legacy_grayscale))
        .unwrap_err()
        .to_string()
        .contains("grayscale is reserved"));

    let mut no_guide = probe;
    no_guide.generation.pose_guidance = GridPoseGuidanceV1::Disabled;
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(no_guide))
        .unwrap_err()
        .to_string()
        .contains("poseGuidance grayscale"));
}

#[test]
fn fixture_grid_v91_child_retry_replaces_only_one_frame_and_reuses_three_bytes() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.1.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Disabled;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let source_job = stage_plan_job(&jobs, &plan).unwrap();
    let source_provider = FixtureProvider::default();
    let source = run_operation_with_provider(
        &jobs,
        &source_job.job_id,
        &plan.operation,
        Some(&source_provider),
    )
    .unwrap();
    let source_report_path = source
        .job_dir
        .join("source/grid-keyframe-actions/walk_down.json");
    let source_report: GridKeyframeActionReportV1 =
        serde_json::from_slice(&fs::read(&source_report_path).unwrap()).unwrap();

    let AutomationOperation::GenerateCharacterPack(mut retry) = plan.operation else {
        unreachable!()
    };
    retry.reuse_from_job_dir = Some(source.job_dir.clone());
    retry.retry_animations = vec!["walk_down".into()];
    retry
        .retry_stages
        .insert("walk_down".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_down".into(), vec![2]);
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 2);
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let retry_provider = FixtureProvider::default();
    let completed = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&retry_provider),
    )
    .unwrap();
    assert_eq!(retry_provider.usage().requests, 1);
    let observations = retry_provider.request_observations();
    assert_eq!(observations.len(), 1);
    assert_eq!(
        observations[0].authorization_target.as_deref(),
        Some("walk_down:frame:2")
    );
    assert_eq!(
        observations[0].reference_roles,
        vec![ReferenceRole::DirectionAnchor]
    );

    let report: GridKeyframeActionReportV1 = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/grid-keyframe-actions/walk_down.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for index in [0_usize, 1, 3] {
        assert_eq!(
            report.frames[index].generation_method,
            GridKeyframeGenerationMethodV1::ByteReuse
        );
        assert_eq!(
            report.frames[index].sha256,
            source_report.frames[index].sha256
        );
        assert!(report.frames[index].reused);
        assert!(!report.frames[index].provider_request_occurred);
    }
    assert_eq!(
        report.frames[2].generation_method,
        GridKeyframeGenerationMethodV1::ChildFrameRetry
    );
    assert_eq!(
        report.frames[2].replaces_frame_sha256.as_deref(),
        Some(source_report.frames[2].sha256.as_str())
    );
    assert!(!report.frames[2].reused);
    assert!(report.frames[2].provider_request_occurred);
    let graph = read_workflow_graph(&completed.job_dir.join("workflow-graph.json")).unwrap();
    let retry_node = graph
        .nodes
        .iter()
        .find(|node| node.id == "independent_keyframe:walk_down:2")
        .unwrap();
    assert!(retry_node.provider_request);
    assert!(retry_node
        .inputs
        .iter()
        .any(|input| input.sha256 == source_report.frames[2].sha256));
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "independent_keyframe" && node.cache_hit)
            .count(),
        3
    );
}

#[test]
fn fixture_grid_v91_full_pack_carries_portable_evidence_and_godot_contract() {
    let (temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    request.workflow.version = "9.1.0".into();
    request.generation.pose_guidance = GridPoseGuidanceV1::Disabled;
    request.reuse_from_job_dir = Some(approved_source);
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request))
        .unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 16);
    assert_eq!(plan.estimate.maximum_provider_requests, 32);
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        for relative in [
            "animation-quality-report.json",
            "character-motion-semantics-report.json",
            "character-silhouette-temporal-report.json",
            "character-semantic-quality-report.json",
            "hand-equipment-contact-report.json",
        ] {
            let path = completed.job_dir.join(relative);
            if path.is_file() {
                eprintln!("--- {relative} ---\n{}", fs::read_to_string(path).unwrap());
            }
        }
    }
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "code={:?} summary={:?} job={}",
        completed.error_code,
        completed.error_summary,
        completed.job_dir.display()
    );
    assert_eq!(provider.usage().requests, 16);
    assert_eq!(provider.usage().generated_videos, 0);
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("V9.1 full workflow exports a Pack");
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    assert_no_secret_or_jobstore_leak(&pack.path);

    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(
        forgepack["assets"]["characterDirectionGridLock"],
        "provenance/grid-keyframes/direction-grid-lock.json"
    );
    assert_eq!(
        forgepack["assets"]["gridKeyframeProviderManifest"],
        "provenance/grid-keyframes/provider-manifest.json"
    );
    assert_eq!(
        forgepack["assets"]["gridKeyframeActionReports"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    let portable_lock: DirectionGridLockV1 = serde_json::from_slice(
        &fs::read(
            pack.path
                .join("provenance/grid-keyframes/direction-grid-lock.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(portable_lock.nodes.iter().all(|node| {
        node.path.is_relative()
            && node
                .path
                .to_string_lossy()
                .starts_with("provenance/grid-keyframes/direction-grid/nodes/")
            && hash_file(&pack.path.join(&node.path)).unwrap() == node.sha256
    }));
    let graph = read_workflow_graph(&completed.job_dir.join("workflow-graph.json")).unwrap();
    assert_eq!(graph.workflow, "topdown-grid@9.1.0");
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "independent_keyframe")
            .count(),
        16
    );
    let pack_node = graph
        .nodes
        .iter()
        .find(|node| node.stage == "pack")
        .unwrap();
    assert_eq!(
        pack_node.outputs[0].sha256,
        hash_directory(&pack.path).unwrap()
    );
    assert_eq!(
        pack_node
            .depends_on
            .iter()
            .filter(|dependency| dependency.starts_with("action_quality:"))
            .count(),
        4
    );
    let tampered_pack = temp.path().join("tampered-grid-v91-pack.gsfpack");
    copy_directory(&pack.path, &tampered_pack);
    let tamper_target =
        tampered_pack.join("provenance/grid-keyframes/actions/walk_down/frame-00.png");
    let mut tampered = image::open(&tamper_target).unwrap().to_rgba8();
    tampered.put_pixel(0, 0, image::Rgba([1, 2, 3, 4]));
    tampered.save(&tamper_target).unwrap();
    let tamper_error = forge_pack::validate_pack_layout(&tampered_pack).unwrap_err();
    assert!(tamper_error.to_string().contains("SHA-256"));

    let godot = PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot");
    if godot.is_file() {
        let godot_project = temp.path().join("godot-grid-v91");
        fs::create_dir_all(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge Grid V9.1 Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let validated_pack = temp.path().join("validated-grid-v91-pack.gsfpack");
        copy_directory(&pack.path, &validated_pack);
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: validated_pack,
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/grid_v91_character"),
            asset_key: Some("grid_v91_character".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("grid-ranger-character".into()),
                label: Some("Grid V9.1 Ranger".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        assert_no_secret_or_jobstore_leak(&godot_project);
        let output = std::process::Command::new(godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot V9.1 load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn fixture_grid_v91_failed_motion_preserves_reports_graph_and_budget_evidence() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let direction_provider = FixtureProvider::default();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request(
            project.clone(),
            style.clone(),
            subject.clone(),
            DirectionMotionGenerationStageV1::ImageLocks,
        )))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &plan.operation,
        Some(&direction_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.workflow.version = "9.1.0".into();
    probe.generation.pose_guidance = GridPoseGuidanceV1::Disabled;
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    let maximum = plan.estimate.maximum_provider_requests;
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default().with_static_keyposes();
    let failed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    assert_eq!(failed.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        failed.error_code.as_deref(),
        Some("grid_keyframe_action_quality_failed")
    );
    assert!(provider.usage().requests <= maximum);
    assert!(failed
        .job_dir
        .join("source/grid-keyframe-actions/walk_down.json")
        .is_file());
    assert!(failed
        .job_dir
        .join("source/grid-keyframe-provider-manifest.json")
        .is_file());
    let graph = read_workflow_graph(&failed.job_dir.join("workflow-graph.json")).unwrap();
    assert_eq!(graph.workflow, "topdown-grid@9.1.0");
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "independent_keyframe")
            .count(),
        4
    );
    let usage: serde_json::Value =
        serde_json::from_slice(&fs::read(failed.job_dir.join("provider-usage.json")).unwrap())
            .unwrap();
    assert_eq!(usage["usage"]["generatedVideos"], 0);
    assert!(usage["usage"]["requests"].as_u64().unwrap() <= u64::from(maximum));
}

#[test]
fn fixture_grid_motion_gate_blocks_a_collapsed_action_grid() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &claimed).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);
    let mut complete_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request
        .character
        .prompt
        .push_str(" [fixture:sheet_duplicate]");
    complete_request.reuse_from_job_dir = Some(approved_source.clone());
    let action_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(complete_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let complete_job = stage_plan_job(&jobs, &claimed).unwrap();
    let error = run_operation_with_provider(
        &jobs,
        &complete_job.job_id,
        &claimed.operation,
        Some(&action_provider),
    )
    .unwrap_err();
    assert!(error.to_string().contains("motion_semantics"));
    assert_eq!(action_provider.usage().requests, 2);
    let failed = jobs.read_record(&complete_job.job_id).unwrap();
    let failed_manifest_path = failed.job_dir.join("source/grid-provider-manifest.json");
    let failed_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&failed_manifest_path).unwrap()).unwrap();
    assert_eq!(failed_manifest["stage"], "action_grid_failed");
    assert_eq!(failed_manifest["failedAction"], "walk_down");
    assert_eq!(failed_manifest["attempts"].as_array().unwrap().len(), 2);
    assert_eq!(failed_manifest["usage"]["requests"], 2);
    assert!(failed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "provider_manifest"));
    assert_eq!(
        failed
            .artifacts
            .iter()
            .filter(|artifact| artifact
                .kind
                .starts_with("grid_action_sheet_walk_down_attempt_"))
            .count(),
        2
    );
}

#[test]
fn fixture_grid_validation_probe_generates_one_direction_without_a_pack() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &claimed).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut probe = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    probe.reuse_from_job_dir = Some(approved_source);
    probe.validation_only = true;
    probe.validation_animations = vec!["walk_down".into()];
    probe.metadata.default_animation = "walk_down".into();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(probe))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 1);
    assert_eq!(prepared.estimate.maximum_provider_requests, 2);
    let claimed = plans.claim(&prepared.token).unwrap();
    let probe_job = stage_plan_job(&jobs, &claimed).unwrap();
    let probe_provider = FixtureProvider::default();
    let completed = run_operation_with_provider(
        &jobs,
        &probe_job.job_id,
        &claimed.operation,
        Some(&probe_provider),
    )
    .unwrap();
    assert_eq!(probe_provider.usage().requests, 1);
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(completed.state, forge_core::job::JobState::QualityChecked);
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    assert!(completed
        .artifacts
        .iter()
        .any(|artifact| { artifact.kind == "grid_action_validation_contact_sheet_walk_down" }));
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["stage"], "action_grid_validation");
    assert_eq!(manifest["validationOnly"], true);
    assert_eq!(
        manifest["validationAnimations"],
        serde_json::json!(["walk_down"])
    );
    assert_eq!(manifest["actions"].as_array().unwrap().len(), 1);
    assert_eq!(manifest["actions"][0]["animation"], "walk_down");
}

#[test]
fn fixture_grid_diagnostic_retry_edits_the_failed_sheet_and_recovers() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &claimed).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut complete_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request
        .character
        .prompt
        .push_str(" [fixture:sheet_duplicate_once]");
    complete_request.reuse_from_job_dir = Some(approved_source.clone());
    let action_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(complete_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let complete_job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed = run_operation_with_provider(
        &jobs,
        &complete_job.job_id,
        &claimed.operation,
        Some(&action_provider),
    )
    .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(action_provider.usage().requests, 5);

    let observations = action_provider
        .request_observations()
        .into_iter()
        .filter(|observation| {
            observation
                .authorization_target
                .as_deref()
                .is_some_and(|target| target.ends_with(":action_grid"))
        })
        .collect::<Vec<_>>();
    assert_eq!(observations.len(), 5);
    assert_eq!(
        observations[0].reference_roles,
        vec![
            ReferenceRole::SubjectIdentity,
            ReferenceRole::DirectionAnchor,
            ReferenceRole::PoseStructure,
        ]
    );
    assert_eq!(
        observations[1].reference_roles,
        vec![
            ReferenceRole::EditTarget,
            ReferenceRole::DirectionAnchor,
            ReferenceRole::PoseStructure,
        ]
    );
    assert!(observations[1]
        .prompt
        .contains("Forge Action Grid diagnostic retry"));
    assert!(observations[1]
        .prompt
        .contains("walk_pose_diversity_missing"));
    assert!(observations[1]
        .prompt
        .contains("Cells 0 and 2 must have opposite contacts"));
    let edits = action_provider.edit_observations();
    assert_eq!(edits.len(), 5);
    let first_sheet = completed
        .job_dir
        .join("source/provider/walk_down/attempt-1/sheet.png");
    assert_eq!(
        edits[1].reference_sha256[0],
        hash_file(&first_sheet).unwrap()
    );

    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    let walk_down = manifest["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["animation"] == "walk_down")
        .expect("walk_down action report");
    assert_eq!(walk_down["attempt"], 2);
    assert_eq!(walk_down["phaseProfile"], "grid-action-phases@1.0.0");
    assert_eq!(walk_down["generationMethod"], "diagnostic_edit_retry");
    assert!(walk_down["inputSheetSha256"].as_str().is_some());
    assert!(completed
        .job_dir
        .join("source/provider/walk_down/attempt-2/grid-action-retry-evidence.json")
        .is_file());
}

#[test]
fn fixture_grid_failed_manifest_records_pre_quality_sheet_failures() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &claimed).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut complete_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request
        .character
        .prompt
        .push_str(" [fixture:sheet_bleed]");
    complete_request.reuse_from_job_dir = Some(approved_source.clone());
    let action_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(complete_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let complete_job = stage_plan_job(&jobs, &claimed).unwrap();
    run_operation_with_provider(
        &jobs,
        &complete_job.job_id,
        &claimed.operation,
        Some(&action_provider),
    )
    .unwrap_err();
    assert_eq!(action_provider.usage().requests, 2);

    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            complete_job
                .job_dir
                .join("source/grid-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let attempts = manifest["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 2);
    assert!(attempts.iter().all(|attempt| {
        attempt["status"] == "sheet_contract_failed"
            && attempt["sheetSha256"].as_str().is_some()
            && attempt["sheetReportSha256"].as_str().is_some()
    }));

    let mut wrong_path_request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    wrong_path_request
        .character
        .prompt
        .push_str(" [fixture:wrong_output_path]");
    wrong_path_request.reuse_from_job_dir = Some(approved_source);
    let wrong_path_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            wrong_path_request,
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let wrong_path_job = stage_plan_job(&jobs, &claimed).unwrap();
    let error = run_operation_with_provider(
        &jobs,
        &wrong_path_job.job_id,
        &claimed.operation,
        Some(&wrong_path_provider),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("grid_action_provider_output_path_mismatch"));
    assert_eq!(wrong_path_provider.usage().requests, 1);
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            wrong_path_job
                .job_dir
                .join("source/grid-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["attempts"].as_array().unwrap().len(), 1);
    assert_eq!(
        manifest["attempts"][0]["status"],
        "provider_output_path_mismatch"
    );
}

#[test]
fn fixture_grid_materializes_but_blocks_undeclared_equipment() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let mut direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    direction_request
        .character
        .prompt
        .push_str(" [fixture:grid_unexpected_equipment]");
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(
        completed.error_code.as_deref(),
        Some("direction_grid_regeneration_required")
    );
    assert_eq!(provider.usage().requests, 1);
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    let report: DirectionGridAppearanceReportV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join(DIRECTION_GRID_APPEARANCE_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        report.verdict,
        forge_core::asset_project::ConsistencyVerdict::Regenerate
    );
    assert_eq!(
        report.equipment.verdict,
        forge_core::asset_project::ConsistencyVerdict::Blocked
    );

    // Even an API caller that writes an approval directly cannot move the
    // rejected semantic lock into paid action generation.
    let approved_source = approve_direction_grid(&jobs, &completed.job_id);
    let mut complete_request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request.reuse_from_job_dir = Some(approved_source);
    let action_provider = FixtureProvider::default();
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(complete_request))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let error =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&action_provider))
            .unwrap_err();
    assert!(error
        .to_string()
        .contains("direction_grid_appearance_not_approved"));
    assert_eq!(action_provider.usage().requests, 0);
}

#[test]
fn fixture_grid_whole_sheet_retry_is_a_one_request_child_with_local_evidence() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let first_provider = FixtureProvider::default();
    let mut first_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    first_request
        .character
        .prompt
        .push_str(" [fixture:grid_unexpected_equipment]");
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            first_request.clone(),
        ))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let first_job = stage_plan_job(&jobs, &plan).unwrap();
    let first = run_operation_with_provider(
        &jobs,
        &first_job.job_id,
        &plan.operation,
        Some(&first_provider),
    )
    .unwrap();
    assert_eq!(
        first.error_code.as_deref(),
        Some("direction_grid_regeneration_required")
    );

    let mut retry_request = first_request;
    retry_request.reuse_from_job_dir = Some(first.job_dir.clone());
    retry_request.retry_animations = vec!["direction_grid".into()];
    retry_request
        .retry_stages
        .insert("direction_grid".into(), CharacterRetryStage::Still);
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry_request))
        .unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 2);
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    assert_eq!(
        retry_job.parent_job_id.as_deref(),
        Some(first.job_id.as_str())
    );
    let retry_provider = FixtureProvider::default();
    let retry = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&retry_provider),
    )
    .unwrap();
    assert_eq!(retry.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(retry_provider.usage().requests, 1);
    let observation = retry_provider
        .request_observations()
        .into_iter()
        .find(|observation| observation.authorization_target.as_deref() == Some("direction_grid"))
        .unwrap();
    assert!(observation
        .prompt
        .contains("new targeted retry of a rejected grid"));
    assert!(observation
        .prompt
        .contains("direction_grid_equipment_contract_failed"));
    let evidence = retry
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "direction_grid_retry_evidence")
        .unwrap();
    let evidence_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&evidence.path).unwrap()).unwrap();
    assert_eq!(
        evidence_json["sourceJobId"].as_str(),
        Some(first.job_id.as_str())
    );
    assert_eq!(evidence_json["providerRequestOccurred"], false);
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(retry.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["retrySourceJob"].as_str(),
        Some(first.job_id.as_str())
    );
    assert_eq!(
        manifest["retryMethod"].as_str(),
        Some("direction_grid_semantic_retry")
    );
}

#[test]
fn fixture_grid_front_authority_uses_one_anchor_and_one_request() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let source_provider = FixtureProvider::default();
    let source_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            source_request.clone(),
        ))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let source_job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(
        &jobs,
        &source_job.job_id,
        &plan.operation,
        Some(&source_provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &source_job.job_id);
    let source_lock: DirectionGridLockV1 = serde_json::from_slice(
        &fs::read(approved_source.join("source/direction-grid-lock.json")).unwrap(),
    )
    .unwrap();
    let front_sha256 = source_lock.node("front_idle").unwrap().sha256.clone();

    let mut child_request = source_request;
    child_request.reuse_from_job_dir = Some(approved_source);
    child_request.retry_animations = vec!["direction_grid".into()];
    child_request.retry_stages = [("direction_grid".into(), CharacterRetryStage::Still)].into();
    child_request.generation.max_attempts_per_animation = 1;
    child_request.direction_grid_cape_contract =
        Some(CapeHemContractV1::FrontAuthoritativeNoSkirtHem);
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(child_request))
        .unwrap();
    assert_eq!(plan.estimate.provider_request_estimate, 1);
    assert_eq!(plan.estimate.maximum_provider_requests, 1);
    let plan = plans.claim(&plan.token).unwrap();
    let child_job = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default();
    let completed =
        run_operation_with_provider(&jobs, &child_job.job_id, &plan.operation, Some(&provider))
            .unwrap();
    assert_eq!(provider.usage().requests, 1);
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);

    let observation = provider
        .request_observations()
        .into_iter()
        .find(|observation| observation.authorization_target.as_deref() == Some("direction_grid"))
        .unwrap();
    assert_eq!(
        observation.reference_roles,
        vec![ReferenceRole::DirectionAnchor]
    );
    assert!(observation
        .prompt
        .contains("Reference 1 is the sole identity"));
    assert!(observation
        .prompt
        .contains("rotations only, never redesigns"));
    assert!(observation.prompt.contains("no skirt-like cape lower hem"));
    let edit = provider.edit_observations().into_iter().next().unwrap();
    assert_eq!(edit.reference_sha256, vec![front_sha256.clone()]);

    let report: CapeHemConsistencyReportV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join(CAPE_HEM_CONSISTENCY_FILE)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        report.contract,
        CapeHemContractV1::FrontAuthoritativeNoSkirtHem
    );
    assert_eq!(report.nodes.len(), 4);
    let lock: DirectionGridLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-grid-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        lock.cape_hem_contract,
        Some(CapeHemContractV1::FrontAuthoritativeNoSkirtHem)
    );
    let producer = lock.producer.unwrap();
    assert_eq!(producer.input_sha256, front_sha256);
    assert!(producer.provider_request_occurred);
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest["generationMethod"],
        "front_authoritative_direction_grid"
    );
    assert_eq!(manifest["usage"]["requests"], 1);
}

#[test]
fn fixture_grid_retry_is_bound_before_any_provider_request() {
    let (temp, project, style, subject, plans, jobs) = setup();
    let source_provider = FixtureProvider::default();
    let source_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            source_request.clone(),
        ))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let source_job = stage_plan_job(&jobs, &plan).unwrap();
    let source = run_operation_with_provider(
        &jobs,
        &source_job.job_id,
        &plan.operation,
        Some(&source_provider),
    )
    .unwrap();

    let mut mismatched_subject_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&subject).unwrap()).unwrap();
    mismatched_subject_value["id"] = serde_json::json!("retry-other-subject");
    mismatched_subject_value["revision"] = serde_json::json!("retry-other-revision");
    let mismatched_subject = temp.path().join("retry-mismatched-subject-lock.json");
    fs::write(
        &mismatched_subject,
        serde_json::to_vec_pretty(&mismatched_subject_value).unwrap(),
    )
    .unwrap();

    let mut cases = Vec::new();
    let mut subject_case = source_request.clone();
    subject_case.subject_lock_path = Some(mismatched_subject);
    cases.push(("subject", subject_case));

    let mut equipment_case = source_request.clone();
    equipment_case.equipment.kind = CharacterEquipmentKindV1::StaffLike;
    equipment_case.equipment.reference_image =
        Some(read_subject_lock(&subject).unwrap().canonical_path);
    cases.push(("equipment", equipment_case));

    let mut camera_case = source_request.clone();
    camera_case.camera_profile = Some(CharacterCameraProfileV1::TopdownOrthographic);
    cases.push(("camera", camera_case));

    let mut model_case = source_request;
    model_case.generation.image_model = Some("fixture-grid-other".into());
    cases.push(("model", model_case));

    for (label, mut request) in cases {
        request.reuse_from_job_dir = Some(source.job_dir.clone());
        request.retry_animations = vec!["direction_grid".into()];
        request
            .retry_stages
            .insert("direction_grid".into(), CharacterRetryStage::Still);
        let plan = plans
            .prepare(AutomationOperation::GenerateCharacterPack(request))
            .unwrap_or_else(|error| panic!("{label} plan failed too early: {error}"));
        let plan = plans.claim(&plan.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        let retry_provider = FixtureProvider::default();
        let error =
            run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&retry_provider))
                .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("direction_grid_retry_source_mismatch"),
            "{label}: {error}"
        );
        assert_eq!(retry_provider.usage().requests, 0, "{label}");
    }
}

#[test]
fn fixture_grid_flags_a_one_sided_glove_drift_for_explicit_review() {
    let (_temp, project, style, subject, plans, jobs) = setup_with_subject_prompt(
        "compact purple ranger [fixture:grid_hand_baseline] [fixture:grid_one_glove]",
    );
    let provider = FixtureProvider::default();
    let request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(
        completed.error_code.as_deref(),
        Some("direction_grid_review_required")
    );
    let report: DirectionGridAppearanceReportV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join(DIRECTION_GRID_APPEARANCE_FILE)).unwrap(),
    )
    .unwrap();
    assert!(report.hand_state_baseline.available);
    assert!(report.hand_state_nodes.iter().any(|node| {
        node.node_id == "right_idle" && node.reasons.contains(&"mirrored_hand_state_review".into())
    }));
    assert_eq!(provider.usage().requests, 1);
}

#[test]
fn fixture_grid_rejects_subject_bound_to_a_different_style_board() {
    let (_temp, project, style, subject, plans, _jobs) = setup();
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&subject).unwrap()).unwrap();
    value["styleSha256"] = serde_json::json!("f".repeat(64));
    fs::write(&subject, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    let request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let error = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request))
        .unwrap_err();
    assert!(error.to_string().contains("subject_style_hash_mismatch"));
}

#[test]
fn fixture_grid_complete_fails_closed_when_approval_is_tampered() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &claimed).unwrap();
    run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);
    let approval_path = approved_source.join(GRID_APPROVAL_FILE);
    let mut approval: DirectionGridApprovalV1 =
        serde_json::from_slice(&fs::read(&approval_path).unwrap()).unwrap();
    approval
        .node_sha256
        .insert("back_idle".into(), "f".repeat(64));
    fs::write(
        &approval_path,
        serde_json::to_vec_pretty(&approval).unwrap(),
    )
    .unwrap();

    let mut complete_request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request.reuse_from_job_dir = Some(approved_source);
    let action_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(complete_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let complete_job = stage_plan_job(&jobs, &claimed).unwrap();
    let error = run_operation_with_provider(
        &jobs,
        &complete_job.job_id,
        &claimed.operation,
        Some(&action_provider),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("direction_grid_approval_mismatch"));
    assert_eq!(action_provider.usage().requests, 0);
}

#[test]
fn fixture_grid_approved_lock_is_bound_to_subject_equipment_camera_and_model() {
    let (temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    run_operation_with_provider(&jobs, &job.job_id, &plan.operation, Some(&provider)).unwrap();
    let approved = approve_direction_grid(&jobs, &job.job_id);

    let mut mismatched_subject_value: serde_json::Value =
        serde_json::from_slice(&fs::read(&subject).unwrap()).unwrap();
    mismatched_subject_value["id"] = serde_json::json!("another-grid-ranger");
    mismatched_subject_value["revision"] = serde_json::json!("another-revision");
    let mismatched_subject = temp.path().join("mismatched-subject-lock.json");
    fs::write(
        &mismatched_subject,
        serde_json::to_vec_pretty(&mismatched_subject_value).unwrap(),
    )
    .unwrap();

    let mut cases = Vec::new();
    let mut subject_case = request(
        project.clone(),
        style.clone(),
        mismatched_subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    subject_case.reuse_from_job_dir = Some(approved.clone());
    cases.push(("subject", subject_case));

    let mut equipment_case = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::Complete,
    );
    equipment_case.reuse_from_job_dir = Some(approved.clone());
    equipment_case.equipment.kind = CharacterEquipmentKindV1::StaffLike;
    equipment_case.equipment.reference_image =
        Some(read_subject_lock(&subject).unwrap().canonical_path);
    cases.push(("equipment", equipment_case));

    let mut camera_case = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::Complete,
    );
    camera_case.reuse_from_job_dir = Some(approved.clone());
    camera_case.camera_profile = Some(CharacterCameraProfileV1::TopdownOrthographic);
    cases.push(("camera", camera_case));

    let mut model_case = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    model_case.reuse_from_job_dir = Some(approved);
    model_case.generation.image_model = Some("fixture-grid-other".into());
    cases.push(("model", model_case));

    for (label, request) in cases {
        let plan = plans
            .prepare(AutomationOperation::GenerateCharacterPack(request))
            .unwrap_or_else(|error| panic!("{label} plan failed too early: {error}"));
        let plan = plans.claim(&plan.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        let action_provider = FixtureProvider::default();
        let error = run_operation_with_provider(
            &jobs,
            &job.job_id,
            &plan.operation,
            Some(&action_provider),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("direction_grid_approval_mismatch")
                || error
                    .to_string()
                    .contains("direction_grid_appearance_not_approved"),
            "{label}: {error}"
        );
        assert_eq!(action_provider.usage().requests, 0, "{label}");
    }
}

fn request(
    project_root: PathBuf,
    style_path: PathBuf,
    subject_path: PathBuf,
    stage: DirectionMotionGenerationStageV1,
) -> GenerateCharacterPackRequest {
    let subject = read_subject_lock(&subject_path).unwrap();
    let profile = automation_profile();
    GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: Some(project_root),
        asset_id: Some("grid-ranger-character".into()),
        character: GeneratedCharacterSpec {
            prompt: subject.prompt.clone(),
            reference_image_path: Some(subject.canonical_path.clone()),
        },
        camera_profile: Some(CharacterCameraProfileV1::TopdownThreeQuarter),
        equipment: Default::default(),
        equipment_explicit: true,
        style_lock_path: Some(style_path),
        subject_lock_path: Some(subject_path),
        reuse_from_job_dir: None,
        retry_animations: vec![],
        retry_stages: BTreeMap::new(),
        retry_frames: BTreeMap::new(),
        direction_grid_cape_contract: None,
        validation_only: false,
        validation_animations: vec![],
        motion_profile: Default::default(),
        direction_motion_stage: stage,
        metadata: CharacterPackMetadata {
            name: "Grid Ranger Character".into(),
            default_animation: "idle_down".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-grid".into(),
            version: "9.0.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 4,
            video_duration_seconds: 4,
            image_model: Some("fixture-grid".into()),
            video_model: None,
            pose_guidance: GridPoseGuidanceV1::Enabled,
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    }
}

fn approve_direction_grid(jobs: &JobStore, job_id: &str) -> PathBuf {
    let record = jobs.read_record(job_id).unwrap();
    let lock_path = record.job_dir.join("source/direction-grid-lock.json");
    let lock: DirectionGridLockV1 = serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    let approval = DirectionGridApprovalV1 {
        schema_version: "1".into(),
        profile: GRID_APPROVAL_PROFILE.into(),
        source_job_id: job_id.into(),
        lock_sha256: hash_file(&lock_path).unwrap(),
        node_sha256: lock.node_hashes(),
        accepted: true,
        reason: "fixture direction grid approval".into(),
        reviewed_at: chrono::Utc::now(),
    };
    fs::write(
        record.job_dir.join(GRID_APPROVAL_FILE),
        serde_json::to_vec_pretty(&approval).unwrap(),
    )
    .unwrap();
    jobs.update_record(job_id, |record| {
        record.lifecycle_state = JobLifecycleState::Succeeded;
    })
    .unwrap();
    record.job_dir
}

fn write_checkerboard_from_direction_lock(source: &Path, output: &Path) {
    let lock: DirectionGridLockV1 =
        serde_json::from_slice(&fs::read(source.join("source/direction-grid-lock.json")).unwrap())
            .unwrap();
    let first = image::open(&lock.nodes[0].path).unwrap().to_rgba8();
    let cell = first.width();
    assert_eq!(first.height(), cell);
    let mut sheet = RgbaImage::new(cell * 2, cell * 2);
    for y in 0..sheet.height() {
        for x in 0..sheet.width() {
            let tone = if (x / 32 + y / 32) % 2 == 0 { 236 } else { 254 };
            sheet.put_pixel(x, y, Rgba([tone, tone, tone, 255]));
        }
    }
    for (index, node) in lock.nodes.iter().enumerate() {
        let frame = image::open(&node.path).unwrap().to_rgba8();
        let x = (index as u32 % 2) * cell;
        let y = (index as u32 / 2) * cell;
        image::imageops::overlay(&mut sheet, &frame, i64::from(x), i64::from(y));
    }
    sheet.save(output).unwrap();
}

fn promote_approved_direction_grid_source_to_xai(
    jobs: &JobStore,
    project_path: &Path,
    job_id: &str,
) -> PathBuf {
    let source = approve_direction_grid(jobs, job_id);
    let source_record = jobs.read_record(job_id).unwrap();
    let source_operation: AutomationOperation =
        serde_json::from_value(source_record.recipe.clone().unwrap()).unwrap();
    let AutomationOperation::GenerateCharacterPack(source_request) = source_operation else {
        unreachable!()
    };
    let style_path = source_request.style_lock_path.unwrap();
    let mut style_lock: forge_core::asset_project::StyleLockV1 =
        serde_json::from_slice(&fs::read(&style_path).unwrap()).unwrap();
    style_lock.provider_id = "xai".into();
    style_lock.profile_id = "default".into();
    style_lock.image_model = Some("grok-imagine-image-quality".into());
    fs::write(&style_path, serde_json::to_vec_pretty(&style_lock).unwrap()).unwrap();
    let subject_path = source_request.subject_lock_path.unwrap();
    let mut subject_lock: forge_core::subject::SubjectLockV1 =
        serde_json::from_slice(&fs::read(&subject_path).unwrap()).unwrap();
    subject_lock.provider_id = "xai".into();
    subject_lock.profile_id = "default".into();
    subject_lock.image_model = Some("grok-imagine-image-quality".into());
    fs::write(
        &subject_path,
        serde_json::to_vec_pretty(&subject_lock).unwrap(),
    )
    .unwrap();
    let lock_path = source.join("source/direction-grid-lock.json");
    let mut lock: DirectionGridLockV1 =
        serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    lock.provider_id = "xai".into();
    lock.profile_id = "default".into();
    lock.image_model = "grok-imagine-image-quality".into();
    fs::write(&lock_path, serde_json::to_vec_pretty(&lock).unwrap()).unwrap();
    let lock_sha256 = hash_file(&lock_path).unwrap();

    let approval_path = source.join(GRID_APPROVAL_FILE);
    let mut approval: DirectionGridApprovalV1 =
        serde_json::from_slice(&fs::read(&approval_path).unwrap()).unwrap();
    approval.lock_sha256 = lock_sha256.clone();
    fs::write(
        &approval_path,
        serde_json::to_vec_pretty(&approval).unwrap(),
    )
    .unwrap();

    let manifest_path = source.join("source/grid-provider-manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["providerId"] = "xai".into();
    manifest["profileId"] = "default".into();
    manifest["imageModel"] = "grok-imagine-image-quality".into();
    manifest["directionGridLock"]["sha256"] = lock_sha256.clone().into();
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    let retry_evidence_path = source.join("source/direction-grid-retry-evidence.json");
    fs::write(
        &retry_evidence_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1",
            "profile": "direction-grid-retry-evidence@1.0.0",
            "sourceJobId": "fixture-incumbent-parent",
            "sourceLockSha256": lock_sha256.clone(),
            "providerRequestOccurred": false,
            "correctionCodes": ["front_hand_state_drift"]
        }))
        .unwrap(),
    )
    .unwrap();
    let retry_evidence_sha256 = hash_file(&retry_evidence_path).unwrap();

    jobs.update_record(job_id, |record| {
        let mut operation: AutomationOperation =
            serde_json::from_value(record.recipe.clone().unwrap()).unwrap();
        let AutomationOperation::GenerateCharacterPack(request) = &mut operation else {
            unreachable!()
        };
        request.provider_id = "xai".into();
        request.profile_id = "default".into();
        request.generation.image_model = Some("grok-imagine-image-quality".into());
        request.reuse_from_job_dir = Some(source.join("fixture-incumbent-parent"));
        request.retry_animations = vec!["direction_grid".into()];
        request.retry_stages =
            BTreeMap::from([("direction_grid".into(), CharacterRetryStage::Still)]);
        let recipe_bytes = serde_json::to_vec(&operation).unwrap();
        record.recipe_hash = Some(format!("{:x}", Sha256::digest(&recipe_bytes)));
        record.recipe = Some(serde_json::to_value(operation).unwrap());
        record.authorization_id = Some("consumed-v9-source-authorization".into());
        record.authorization_manifest_sha256 = Some("a".repeat(64));
        if let Some(artifact) = record
            .artifacts
            .iter_mut()
            .find(|artifact| artifact.kind == "direction_grid_lock")
        {
            artifact.sha256 = Some(lock_sha256.clone());
        }
        record.artifacts.push(forge_core::job::JobArtifactRecord {
            kind: "direction_grid_retry_evidence".into(),
            path: retry_evidence_path.clone(),
            sha256: Some(retry_evidence_sha256.clone()),
        });
    })
    .unwrap();

    let mut project = read_project(project_path).unwrap();
    project.provider.id = "xai".into();
    project.provider.profile_id = "default".into();
    fs::write(
        project_path.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    source
}

fn assert_no_secret_or_jobstore_leak(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                pending.push(entry.unwrap().path());
            }
            continue;
        }
        let extension = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default();
        if !matches!(extension, "json" | "gd" | "tres" | "tscn" | "txt") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("Authorization"), "{path:?}");
        assert!(!text.contains("Bearer "), "{path:?}");
        assert!(!text.contains("access_token"), "{path:?}");
        assert!(!text.contains("refresh_token"), "{path:?}");
        assert!(!text.contains("https://"), "{path:?}");
        assert!(!text.contains("/jobs/"), "{path:?}");
    }
}

fn copy_directory(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_directory(&source_path, &target_path);
        } else {
            fs::copy(&source_path, &target_path).unwrap();
        }
    }
}

#[test]
fn fixture_grid_two_stage_pack_and_cell_retry() {
    let (_temp, project, style, subject, plans, jobs) = setup();
    let provider = FixtureProvider::default();
    let direction_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            direction_request,
        ))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 1);
    assert_eq!(prepared.estimate.maximum_provider_requests, 2);
    let claimed = plans.claim(&prepared.token).unwrap();
    let direction_job = stage_plan_job(&jobs, &claimed).unwrap();
    let direction_completed = run_operation_with_provider(
        &jobs,
        &direction_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(
        direction_completed.lifecycle_state,
        JobLifecycleState::AwaitingReview
    );
    assert!(!direction_completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    let observations = provider.request_observations();
    let direction_observation = observations
        .iter()
        .find(|observation| observation.authorization_target.as_deref() == Some("direction_grid"))
        .expect("direction grid request");
    assert_eq!(direction_observation.kind, FixtureRequestKind::EditImage);
    assert_eq!(
        direction_observation.reference_roles,
        vec![ReferenceRole::SubjectIdentity, ReferenceRole::Style]
    );
    assert!(direction_observation
        .prompt
        .contains("Explicit equipment declaration: NONE"));
    assert!(direction_observation
        .prompt
        .contains("Style reference controls rendering only"));
    assert!(direction_completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "direction_grid_appearance_report"));
    let approved_source = approve_direction_grid(&jobs, &direction_job.job_id);

    let mut complete_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request.reuse_from_job_dir = Some(approved_source);
    let action_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            complete_request.clone(),
        ))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 4);
    assert_eq!(prepared.estimate.maximum_provider_requests, 8);
    let claimed = plans.claim(&prepared.token).unwrap();
    let complete_job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed = run_operation_with_provider(
        &jobs,
        &complete_job.job_id,
        &claimed.operation,
        Some(&action_provider),
    )
    .unwrap();
    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        eprintln!("grid job dir: {}", completed.job_dir.display());
        eprintln!(
            "state={:?} code={:?} summary={:?}",
            completed.lifecycle_state, completed.error_code, completed.error_summary
        );
        for relative in [
            "animation-quality-report.json",
            "consistency-report.json",
            "character-motion-semantics-report.json",
            "character-silhouette-temporal-report.json",
            "character-silhouette-temporal-source-report.json",
        ] {
            let path = completed.job_dir.join(relative);
            if path.is_file() {
                eprintln!("--- {relative} ---\n{}", fs::read_to_string(path).unwrap());
            }
        }
    }
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "code={:?} summary={:?}",
        completed.error_code,
        completed.error_summary
    );
    assert_eq!(action_provider.usage().requests, 4);
    assert_eq!(action_provider.usage().generated_videos, 0);
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["usage"]["requests"], 4);
    assert_eq!(manifest["actions"].as_array().unwrap().len(), 4);
    let action_observations = action_provider
        .request_observations()
        .into_iter()
        .filter(|observation| {
            observation
                .authorization_target
                .as_deref()
                .is_some_and(|target| target.ends_with(":action_grid"))
        })
        .collect::<Vec<_>>();
    assert_eq!(action_observations.len(), 4);
    assert!(action_observations.iter().all(|observation| {
        observation.reference_roles
            == vec![
                ReferenceRole::SubjectIdentity,
                ReferenceRole::DirectionAnchor,
                ReferenceRole::PoseStructure,
            ]
    }));
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("grid workflow exports a Pack");
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    assert_no_secret_or_jobstore_leak(&pack.path);
    let pack_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("assets/manifest.json")).unwrap()).unwrap();
    let names = pack_manifest["animations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|animation| animation["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "idle_down",
            "idle_up",
            "idle_right",
            "idle_left",
            "walk_down",
            "walk_up",
            "walk_right",
            "walk_left",
        ]
    );

    let mut frame_retry = complete_request.clone();
    frame_retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    frame_retry.retry_animations = vec!["walk_right".into()];
    frame_retry.retry_stages.insert(
        "walk_right".into(),
        forge_core::automation::CharacterRetryStage::Frame,
    );
    frame_retry
        .retry_frames
        .insert("walk_right".into(), vec![2]);
    let retry_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(frame_retry))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 1);
    assert_eq!(prepared.estimate.maximum_provider_requests, 2);
    let claimed = plans.claim(&prepared.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &claimed).unwrap();
    let retried = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &claimed.operation,
        Some(&retry_provider),
    )
    .unwrap();
    assert_eq!(retried.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(retry_provider.usage().requests, 1);
    let retry_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(retried.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    let action = retry_manifest["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["animation"] == "walk_right")
        .expect("walk_right retry report");
    let cells = action["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 4);
    assert_eq!(cells[2]["providerRequestOccurred"], true);
    assert_eq!(
        cells
            .iter()
            .enumerate()
            .filter(|(index, cell)| *index != 2 && cell["reused"] == true)
            .count(),
        3
    );

    let mut rejected_frame_retry = complete_request.clone();
    rejected_frame_retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    rejected_frame_retry
        .character
        .prompt
        .push_str(" [fixture:frame_retry_unexpected_equipment]");
    rejected_frame_retry.retry_animations = vec!["walk_right".into()];
    rejected_frame_retry.retry_stages.insert(
        "walk_right".into(),
        forge_core::automation::CharacterRetryStage::Frame,
    );
    rejected_frame_retry
        .retry_frames
        .insert("walk_right".into(), vec![2]);
    let rejected_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            rejected_frame_retry,
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let rejected_job = stage_plan_job(&jobs, &claimed).unwrap();
    let error = run_operation_with_provider(
        &jobs,
        &rejected_job.job_id,
        &claimed.operation,
        Some(&rejected_provider),
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("equipment_failed"),
        "unexpected targeted-frame error: {error}"
    );
    assert_eq!(rejected_provider.usage().requests, 2);
    let rejected_record = jobs.read_record(&rejected_job.job_id).unwrap();
    assert!(rejected_record.artifacts.iter().any(|artifact| artifact
        .kind
        .starts_with("grid_frame_retry_equipment_walk_right_2")));

    let mut disabled_request = complete_request;
    disabled_request.generation.pose_guidance = GridPoseGuidanceV1::Disabled;
    let disabled_provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(disabled_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let disabled_job = stage_plan_job(&jobs, &claimed).unwrap();
    let disabled = run_operation_with_provider(
        &jobs,
        &disabled_job.job_id,
        &claimed.operation,
        Some(&disabled_provider),
    )
    .unwrap();
    assert_eq!(disabled.lifecycle_state, JobLifecycleState::Succeeded);
    let disabled_observations = disabled_provider
        .request_observations()
        .into_iter()
        .filter(|observation| {
            observation
                .authorization_target
                .as_deref()
                .is_some_and(|target| target.ends_with(":action_grid"))
        })
        .collect::<Vec<_>>();
    assert_eq!(disabled_observations.len(), 4);
    assert!(disabled_observations.iter().all(|observation| {
        observation.reference_roles
            == vec![
                ReferenceRole::SubjectIdentity,
                ReferenceRole::DirectionAnchor,
            ]
    }));
    let disabled_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(disabled.job_dir.join("source/grid-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(disabled_manifest["poseGuidance"], "disabled");

    let godot = std::path::PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot");
    if godot.is_file() {
        let godot_project = _temp.path().join("godot-grid");
        fs::create_dir_all(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge Grid Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let validated_pack = _temp.path().join("validated-grid-pack.gsfpack");
        copy_directory(&pack.path, &validated_pack);
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: validated_pack,
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/grid_character"),
            asset_key: Some("grid_character".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("grid-ranger-character".into()),
                label: Some("Grid Ranger".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        assert_no_secret_or_jobstore_leak(&godot_project);
        let output = std::process::Command::new(godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot grid load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
