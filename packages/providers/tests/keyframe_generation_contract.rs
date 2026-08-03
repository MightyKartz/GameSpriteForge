use std::collections::BTreeMap;
use std::fs;

use forge_core::asset_project::{
    init_project, read_project, SamplingMode, StyleSpecV1, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::{
    automation_profile, run_operation_with_provider, stage_plan_job, AutomationOperation,
    CharacterPackMetadata, CharacterRetryStage, CharacterWorkflowSelection, CreateStyleLockRequest,
    CreateSubjectLockRequest, GenerateCharacterPackRequest, GeneratedCharacterSpec,
    GenerationPolicy, PlanStore, QualityPolicy,
};
use forge_core::catalog::read_project_catalog;
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::provider::MediaGenerationProvider;
use forge_core::subject::{read_subject_lock, SubjectSpecV1};
use forge_core::workflow_graph::{read_workflow_graph, WORKFLOW_GRAPH_FILE};
use forge_providers::fixture::FixtureProvider;

#[test]
fn fixture_keyframes_create_pack_graph_catalog_and_single_frame_retry() {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "Keyframe Contract").unwrap();
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
            prompt: "compact purple pixel art".into(),
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
            id: "fixture-ranger".into(),
            name: "Fixture Ranger".into(),
            prompt: "compact purple ranger".into(),
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

    run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::CreateStyleLock(CreateStyleLockRequest {
            schema_version: "1".into(),
            project_path: project_root.clone(),
            spec_path: style_spec,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
        }),
    );
    let style_revision = read_project(&project_root)
        .unwrap()
        .current_style_revision
        .unwrap();
    run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::CreateSubjectLock(CreateSubjectLockRequest {
            schema_version: "1".into(),
            project_path: project_root.clone(),
            spec_path: subject_spec,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
        }),
    );
    let subject_lock_path = project_root
        .join(".forge/subjects/fixture-ranger")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("subject-lock.json");
    let subject = read_subject_lock(&subject_lock_path).unwrap();
    let profile = automation_profile();
    let request = GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: Some(project_root.clone()),
        asset_id: Some("fixture-ranger-keyframes".into()),
        character: GeneratedCharacterSpec {
            prompt: subject.prompt.clone(),
            reference_image_path: Some(subject.canonical_path.clone()),
        },
        style_lock_path: Some(
            project_root
                .join(".forge/styles")
                .join(style_revision)
                .join(STYLE_LOCK_FILE),
        ),
        subject_lock_path: Some(subject_lock_path),
        reuse_from_job_dir: None,
        retry_animations: vec![],
        retry_stages: BTreeMap::new(),
        retry_frames: BTreeMap::new(),
        metadata: CharacterPackMetadata {
            name: "Fixture Ranger Keyframes".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-keyframes".into(),
            version: "2.0.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 8,
            video_duration_seconds: 4,
            image_model: None,
            video_model: None,
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    };
    let completed = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(request.clone()),
    );
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let graph = read_workflow_graph(&completed.job_dir.join(WORKFLOW_GRAPH_FILE)).unwrap();
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "frame_image")
            .count(),
        32
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "matting")
            .count(),
        4
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "provisional_align")
            .count(),
        4
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "loop_quality")
            .count(),
        4
    );
    assert!(graph
        .nodes
        .iter()
        .any(|node| node.id == "collection_consistency"));
    assert!(graph.nodes.iter().any(|node| node.id == "shared_normalize"));
    assert!(completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    assert!(read_project_catalog(&project_root)
        .unwrap()
        .assets
        .contains_key("fixture-ranger-keyframes"));

    let mut retry = request;
    retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    retry.retry_animations = vec!["walk_right".into()];
    retry
        .retry_stages
        .insert("walk_right".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_right".into(), vec![3]);
    let retried = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(retry),
    );
    assert_eq!(
        retried.parent_job_id.as_deref(),
        Some(completed.job_id.as_str())
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            retried
                .job_dir
                .join("source/keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["usage"]["requests"], 1);

    let mut local_request = request_from_job(&completed);
    local_request.asset_id = Some("fixture-ranger-local-replay".into());
    local_request.reuse_from_job_dir = Some(completed.job_dir.clone());
    local_request.retry_animations = vec![
        "idle".into(),
        "walk_up".into(),
        "walk_right".into(),
        "walk_down".into(),
    ];
    local_request.retry_stages = local_request
        .retry_animations
        .iter()
        .map(|animation| (animation.clone(), CharacterRetryStage::Consistency))
        .collect();
    let local = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(local_request),
    );
    let local_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(local.job_dir.join("source/keyframe-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(local_manifest["usage"]["requests"], 0);

    let mut review_request = request_from_job(&completed);
    review_request.asset_id = Some("fixture-ranger-review".into());
    review_request.project_path = Some(project_root.clone());
    review_request.generation.image_model = Some("fixture-review".into());
    let review_provider = FixtureProvider::default().with_review_keyframes();
    let review = run(
        &plans,
        &jobs,
        &review_provider,
        AutomationOperation::GenerateCharacterPack(review_request),
    );
    assert_eq!(review.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert!(review
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "candidate_gsfpack"));
    assert!(!review
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
}

fn request_from_job(completed: &forge_core::job::JobRecord) -> GenerateCharacterPackRequest {
    let operation: AutomationOperation =
        serde_json::from_value(completed.recipe.clone().expect("immutable recipe")).unwrap();
    let AutomationOperation::GenerateCharacterPack(mut request) = operation else {
        panic!("expected Character recipe");
    };
    request.reuse_from_job_dir = None;
    request.retry_animations.clear();
    request.retry_stages.clear();
    request.retry_frames.clear();
    request
}

fn run(
    plans: &PlanStore,
    jobs: &JobStore,
    provider: &dyn MediaGenerationProvider,
    operation: AutomationOperation,
) -> forge_core::job::JobRecord {
    let prepared = plans.prepare(operation).unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(jobs, &claimed).unwrap();
    run_operation_with_provider(jobs, &job.job_id, &claimed.operation, Some(provider)).unwrap()
}
