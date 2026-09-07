use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use forge_core::asset_project::{
    hash_file, init_project, read_project, SamplingMode, StyleSpecV1, FORGE_PROJECT_FILE,
    STYLE_LOCK_FILE,
};
use forge_core::automation::{
    automation_profile, run_operation, run_operation_with_provider, stage_plan_job,
    AutomationOperation, CharacterPackMetadata, CharacterWorkflowSelection, CreateStyleLockRequest,
    CreateSubjectLockRequest, GenerateCharacterPackRequest, GeneratedCharacterSpec,
    GenerationPolicy, GodotInstallRequest, PlanStore, QualityPolicy,
};
use forge_core::character_camera::CharacterCameraProfileV1;
use forge_core::character_direction_motion::{
    CharacterMotionProfileV1, DirectionMotionApprovalV1, DirectionMotionGenerationStageV1,
    DirectionMotionLockV1, DIRECTION_MOTION_APPROVAL_FILE, DIRECTION_MOTION_APPROVAL_PROFILE,
};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::project::ProviderAssetRef;
use forge_core::subject::{read_subject_lock, SubjectSpecV1};
use forge_providers::fixture::{FixtureProvider, FixtureRequestKind};
use forge_providers::LocalReplayProvider;

fn setup() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    std::path::PathBuf,
    PlanStore,
    JobStore,
    FixtureProvider,
) {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "V8 Direction Motion Contract").unwrap();
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
            id: "v8-subject".into(),
            name: "V8 Subject".into(),
            prompt: "one complete compact purple fantasy subject".into(),
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
        let prepared = plans.prepare(operation).unwrap();
        let claimed = plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&jobs, &claimed).unwrap();
        let completed =
            run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
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
        .join(".forge/subjects/v8-subject")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("subject-lock.json");
    (
        temp,
        project_root,
        style_path,
        subject_path,
        plans,
        jobs,
        provider,
    )
}

fn request(
    project_root: std::path::PathBuf,
    style_path: std::path::PathBuf,
    subject_path: std::path::PathBuf,
    stage: DirectionMotionGenerationStageV1,
) -> GenerateCharacterPackRequest {
    let subject = read_subject_lock(&subject_path).unwrap();
    let profile = automation_profile();
    GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: Some(project_root),
        asset_id: Some("v8-direction-motion".into()),
        character: GeneratedCharacterSpec {
            prompt: subject.prompt,
            reference_image_path: None,
        },
        camera_profile: Some(CharacterCameraProfileV1::TopdownThreeQuarter),
        equipment: Default::default(),
        equipment_explicit: false,
        style_lock_path: Some(style_path),
        subject_lock_path: Some(subject_path),
        reuse_from_job_dir: None,
        retry_animations: vec![],
        retry_stages: BTreeMap::new(),
        retry_frames: BTreeMap::new(),
        direction_grid_cape_contract: None,
        validation_only: false,
        validation_animations: vec![],
        motion_profile: CharacterMotionProfileV1::Freeform,
        direction_motion_stage: stage,
        metadata: CharacterPackMetadata {
            name: "V8 Direction Motion".into(),
            default_animation: "idle_down".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-direction-motion".into(),
            version: "8.0.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 12,
            video_duration_seconds: 4,
            image_model: Some("fixture-image".into()),
            video_model: Some("fixture-video".into()),
            pose_guidance: Default::default(),
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    }
}

fn approve_direction_motion_lock(jobs: &JobStore, job_id: &str) -> std::path::PathBuf {
    let record = jobs.read_record(job_id).unwrap();
    let lock_path = record.job_dir.join("source/direction-motion-lock.json");
    let lock: DirectionMotionLockV1 =
        serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    let approval = DirectionMotionApprovalV1 {
        schema_version: "1".into(),
        profile: DIRECTION_MOTION_APPROVAL_PROFILE.into(),
        source_job_id: job_id.into(),
        lock_sha256: hash_file(&lock_path).unwrap(),
        node_sha256: lock
            .nodes
            .iter()
            .map(|node| (node.node_id.clone(), node.sha256.clone()))
            .collect(),
        accepted: true,
        reason: "fixture contract approval".into(),
        reviewed_at: chrono::Utc::now(),
    };
    fs::write(
        record.job_dir.join(DIRECTION_MOTION_APPROVAL_FILE),
        serde_json::to_vec_pretty(&approval).unwrap(),
    )
    .unwrap();
    jobs.update_record(job_id, |record| {
        record.lifecycle_state = JobLifecycleState::Succeeded;
    })
    .unwrap();
    record.job_dir
}

#[test]
fn fixture_v8_image_stage_builds_an_explicit_eight_node_lock_before_video() {
    let (_temp, project, style, subject, plans, jobs, provider) = setup();
    let request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let observations_before = provider.request_observations().len();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 8);
    assert_eq!(prepared.estimate.maximum_provider_requests, 16);
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert_eq!(
        completed.error_code.as_deref(),
        Some("direction_motion_lock_review_required")
    );
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));

    let lock: DirectionMotionLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-motion-lock.json")).unwrap(),
    )
    .unwrap();
    let ids = lock
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "front_idle",
            "back_idle",
            "right_idle",
            "left_idle",
            "front_walk",
            "back_walk",
            "right_walk",
            "left_walk",
        ]
    );
    for animation in ["walk_down", "walk_up", "walk_right", "walk_left"] {
        assert_eq!(
            lock.video_input(animation).unwrap().animation.as_deref(),
            Some(animation)
        );
    }

    let observations = provider.request_observations();
    let observations = &observations[observations_before..];
    assert_eq!(observations.len(), 8);
    assert_eq!(observations[0].kind, FixtureRequestKind::GenerateImage);
    assert_eq!(
        observations[0].authorization_target.as_deref(),
        Some("front_idle")
    );
    assert!(observations[0].reference_roles.is_empty());
    assert!(observations[1..]
        .iter()
        .all(|observation| observation.kind == FixtureRequestKind::EditImage));
    assert!(observations
        .iter()
        .all(|observation| observation.kind != FixtureRequestKind::GenerateVideo));
}

#[test]
fn fixture_v8_image_lock_retry_invalidates_only_dependent_nodes() {
    let (_temp, project, style, subject, plans, jobs, provider) = setup();
    let image_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(image_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let image_job = stage_plan_job(&jobs, &claimed).unwrap();
    let source = run_operation_with_provider(
        &jobs,
        &image_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(source.lifecycle_state, JobLifecycleState::AwaitingReview);
    let source_lock_path = source.job_dir.join("source/direction-motion-lock.json");
    let source_lock_sha = hash_file(&source_lock_path).unwrap();
    let source_lock: DirectionMotionLockV1 =
        serde_json::from_slice(&fs::read(&source_lock_path).unwrap()).unwrap();
    fs::write(
        source.job_dir.join("review-decision.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1",
            "accepted": false,
            "reason": "back_idle loses the permanent shoulder covering from front_idle",
            "reviewedAt": chrono::Utc::now(),
        }))
        .unwrap(),
    )
    .unwrap();

    let mut retry = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    retry.asset_id = Some("v8-back-idle-targeted-retry".into());
    retry.reuse_from_job_dir = Some(source.job_dir.clone());
    retry.retry_animations = vec!["idle_up".into()];
    retry.retry_stages.insert(
        "idle_up".into(),
        forge_core::automation::CharacterRetryStage::Still,
    );
    let observations_before = provider.request_observations().len();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 2);
    assert_eq!(prepared.estimate.maximum_provider_requests, 4);
    let claimed = plans.claim(&prepared.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    assert_eq!(hash_file(&source_lock_path).unwrap(), source_lock_sha);

    let observations = provider.request_observations();
    let observations = &observations[observations_before..];
    assert_eq!(
        observations
            .iter()
            .filter_map(|observation| observation.authorization_target.as_deref())
            .collect::<Vec<_>>(),
        ["back_idle", "back_walk"]
    );
    assert_eq!(observations.len(), 2);
    assert!(observations
        .iter()
        .all(|observation| observation.kind == FixtureRequestKind::EditImage));
    assert_eq!(
        observations
            .iter()
            .filter_map(|observation| observation.authorization_target.as_deref())
            .collect::<Vec<_>>(),
        ["back_idle", "back_walk"]
    );
    assert!(observations[0]
        .prompt
        .contains("back_idle loses the permanent shoulder covering"));
    assert_eq!(
        observations[0].reference_roles,
        vec![
            forge_core::provider::ReferenceRole::EditTarget,
            forge_core::provider::ReferenceRole::PoseStructure,
        ]
    );
    assert_eq!(
        observations[1].reference_roles,
        vec![
            forge_core::provider::ReferenceRole::EditTarget,
            forge_core::provider::ReferenceRole::SubjectIdentity,
            forge_core::provider::ReferenceRole::PoseStructure,
        ]
    );

    let retried_lock: DirectionMotionLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-motion-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        retried_lock.node("front_idle").unwrap().sha256,
        source_lock.node("front_idle").unwrap().sha256
    );
    assert_eq!(
        retried_lock.node("right_idle").unwrap().sha256,
        source_lock.node("right_idle").unwrap().sha256
    );
    assert_eq!(
        retried_lock
            .node("back_walk")
            .unwrap()
            .parent_node_id
            .as_deref(),
        Some("back_idle")
    );
}

#[test]
fn fixture_v8_complete_stage_preserves_source_cycles_and_exports_eight_animations() {
    if forge_core::video::ffmpeg::find_in_path("ffmpeg").is_none()
        || forge_core::video::ffmpeg::find_in_path("ffprobe").is_none()
    {
        eprintln!("skipping V8 complete contract because ffmpeg/ffprobe are unavailable");
        return;
    }
    let (temp, project, style, subject, plans, jobs, provider) = setup();
    let image_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(image_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let image_job = stage_plan_job(&jobs, &claimed).unwrap();
    let image_completed = run_operation_with_provider(
        &jobs,
        &image_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(
        image_completed.lifecycle_state,
        JobLifecycleState::AwaitingReview
    );
    let approved_source = approve_direction_motion_lock(&jobs, &image_job.job_id);
    let mut complete_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request.reuse_from_job_dir = Some(approved_source);
    let observations_before = provider.request_observations().len();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(complete_request))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 4);
    assert_eq!(prepared.estimate.maximum_provider_requests, 8);
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        for relative in [
            "animation-quality-report.json",
            "loop-selection-report.json",
            "source/direction-motion-provider-manifest.json",
        ] {
            let path = completed.job_dir.join(relative);
            if path.is_file() {
                eprintln!("--- {relative} ---\n{}", fs::read_to_string(path).unwrap());
            }
        }
    }
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let observations = provider.request_observations();
    let observations = &observations[observations_before..];
    assert_eq!(observations.len(), 4);
    assert_eq!(
        observations
            .iter()
            .filter(|observation| matches!(
                observation.kind,
                FixtureRequestKind::GenerateImage | FixtureRequestKind::EditImage
            ))
            .count(),
        0
    );
    assert_eq!(
        observations
            .iter()
            .filter(|observation| observation.kind == FixtureRequestKind::GenerateVideo)
            .count(),
        4
    );
    let completed_lock: DirectionMotionLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-motion-lock.json")).unwrap(),
    )
    .unwrap();
    let videos = observations
        .iter()
        .filter(|observation| observation.kind == FixtureRequestKind::GenerateVideo)
        .collect::<Vec<_>>();
    for animation in ["walk_down", "walk_up", "walk_right", "walk_left"] {
        let node = completed_lock.video_input(animation).unwrap();
        let observation = videos
            .iter()
            .find(|observation| {
                observation.authorization_target.as_deref()
                    == Some(format!("{animation}:video").as_str())
            })
            .unwrap_or_else(|| panic!("missing video observation for {animation}"));
        assert_eq!(
            observation.video_input_sha256.as_deref(),
            Some(node.generation_master_sha256.as_str()),
            "{animation} must receive the exact typed walk-pose master"
        );
        assert_eq!(
            observation.video_input_dimensions,
            Some((512, 512)),
            "{animation} must not be downscaled before image-to-video"
        );
        assert!(
            !observation.prompt.contains("chroma"),
            "{animation} prompt must not ask for a chroma background"
        );
        assert!(
            !observation.prompt.contains("green"),
            "{animation} prompt must not ask for a green background"
        );
        assert!(
            !observation.prompt.contains("background"),
            "{animation} prompt must not restyle the approved background"
        );
    }
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .map(|artifact| artifact.path.clone())
        .expect("V8 complete fixture must export a Pack");
    forge_pack::validate_pack_layout(&pack).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/manifest.json")).unwrap()).unwrap();
    let animation_names = manifest["animations"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|animation| animation["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        animation_names,
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
    for animation in ["walk_down", "walk_up", "walk_right", "walk_left"] {
        let report: serde_json::Value = serde_json::from_slice(
            &fs::read(
                completed
                    .job_dir
                    .join("animations")
                    .join(animation)
                    .join("loop-selection-report.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let indices = report["outputFrameIndices"].as_array().unwrap();
        assert!(indices.len() >= 8 && indices.len() <= 16);
        assert!(indices
            .windows(2)
            .all(|pair| { pair[1].as_u64().unwrap() > pair[0].as_u64().unwrap() }));
        assert!(!indices.contains(&report["selectedEndBoundaryFrame"]));
        assert_eq!(report["timing"]["profile"], "source-cycle-timing@1.0.0");
        assert_eq!(report["timing"]["playbackSpeedRatio"], 1.0);
        let durations = report["timing"]["frameDurationsMs"].as_array().unwrap();
        assert_eq!(durations.len(), indices.len());
        assert!(durations
            .iter()
            .all(|duration| duration.as_u64().unwrap() > 0));
        assert_eq!(
            durations
                .iter()
                .map(|value| value.as_u64().unwrap())
                .sum::<u64>(),
            report["selectedDurationMs"].as_u64().unwrap()
        );
        let sampling: serde_json::Value = serde_json::from_slice(
            &fs::read(
                completed
                    .job_dir
                    .join("animations")
                    .join(animation)
                    .join("source-cycle-sampling-report.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(sampling["profile"], "source-cycle-sampling@1.0.0");
        assert_eq!(sampling["outputFrameIndices"], report["outputFrameIndices"]);
        assert_eq!(sampling["outputFrameCount"], indices.len());
        assert!(!sampling["outputFrameIndices"]
            .as_array()
            .unwrap()
            .contains(&sampling["selectedEndBoundaryFrame"]));
    }
    for relative in [
        "character-direction-motion-lock.json",
        "character-direction-motion-approval.json",
        "provenance/direction-motion-provider-manifest.json",
    ] {
        assert!(pack.join(relative).is_file(), "missing {relative}");
        let text = fs::read_to_string(pack.join(relative)).unwrap();
        assert!(!text.contains(completed.job_dir.to_string_lossy().as_ref()));
        assert!(!text.contains("Authorization"));
        assert!(!text.contains("Bearer "));
    }
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(
        forgepack["source"]["metadata"]["sourceCycleSampling"]["profile"],
        "source-cycle-sampling@1.0.0"
    );
    for animation in ["walk_down", "walk_up", "walk_right", "walk_left"] {
        let report_path = pack
            .join("provenance/source-cycle-sampling")
            .join(format!("{animation}.json"));
        assert!(report_path.is_file());
        assert_eq!(
            hash_file(&report_path).unwrap(),
            forgepack["source"]["metadata"]["sourceCycleSampling"]["reports"][animation]["sha256"]
                .as_str()
                .unwrap()
        );
    }
    let tamper_path = pack.join("provenance/source-cycle-sampling/walk_down.json");
    let original_sampling_report = fs::read(&tamper_path).unwrap();
    let mut tampered_sampling_report: serde_json::Value =
        serde_json::from_slice(&original_sampling_report).unwrap();
    tampered_sampling_report["outputFrameCount"] = serde_json::json!(9);
    fs::write(
        &tamper_path,
        serde_json::to_vec_pretty(&tampered_sampling_report).unwrap(),
    )
    .unwrap();
    assert!(forge_pack::validate_pack_layout(&pack).is_err());
    fs::write(&tamper_path, original_sampling_report).unwrap();
    forge_pack::validate_pack_layout(&pack).unwrap();
    let mut replay_request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    replay_request.reuse_from_job_dir = Some(completed.job_dir.clone());
    replay_request.retry_animations = ["walk_down", "walk_up", "walk_right", "walk_left"]
        .map(str::to_string)
        .into();
    replay_request.retry_stages = replay_request
        .retry_animations
        .iter()
        .map(|animation| {
            (
                animation.clone(),
                forge_core::automation::CharacterRetryStage::Loop,
            )
        })
        .collect();
    let replay_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(replay_request))
        .unwrap();
    assert_eq!(replay_plan.estimate.provider_request_estimate, 0);
    assert_eq!(replay_plan.estimate.maximum_provider_requests, 0);
    let replay_plan = plans.claim(&replay_plan.token).unwrap();
    let replay_job = stage_plan_job(&jobs, &replay_plan).unwrap();
    let local_provider = LocalReplayProvider::new("fixture").unwrap();
    let replayed = run_operation_with_provider(
        &jobs,
        &replay_job.job_id,
        &replay_plan.operation,
        Some(&local_provider),
    )
    .unwrap();
    assert_eq!(replayed.lifecycle_state, JobLifecycleState::Succeeded);
    let replay_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            replayed
                .job_dir
                .join("source/direction-motion-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(replay_manifest["stage"], "local_replay");
    assert_eq!(replay_manifest["usage"]["requests"], 0);
    for animation in ["walk_down", "walk_up", "walk_right", "walk_left"] {
        assert_eq!(
            hash_file(
                &completed
                    .job_dir
                    .join("animations")
                    .join(animation)
                    .join("loop-selection-report.json")
            )
            .unwrap(),
            hash_file(
                &replayed
                    .job_dir
                    .join("animations")
                    .join(animation)
                    .join("loop-selection-report.json")
            )
            .unwrap()
        );
        assert_eq!(
            hash_file(
                &completed
                    .job_dir
                    .join("animations")
                    .join(animation)
                    .join("source-cycle-sampling-report.json")
            )
            .unwrap(),
            hash_file(
                &replayed
                    .job_dir
                    .join("animations")
                    .join(animation)
                    .join("source-cycle-sampling-report.json")
            )
            .unwrap()
        );
    }
    let godot = std::path::PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot");
    if godot.is_file() {
        let godot_project = temp.path().join("godot-v8");
        fs::create_dir_all(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge V8 Direction Motion Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack.clone(),
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: std::path::PathBuf::from("addons/forge_assets/v8_character"),
            asset_key: Some("v8_character".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("v8-direction-motion".into()),
                label: Some("V8 Direction Motion".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let target = godot_project.join("addons/forge_assets/v8_character");
        let frames_text = fs::read_to_string(target.join("forge_sprite_frames.tres")).unwrap();
        for animation in [
            "idle_down",
            "idle_up",
            "idle_right",
            "idle_left",
            "walk_down",
            "walk_up",
            "walk_right",
            "walk_left",
        ] {
            assert!(frames_text.contains(animation));
        }
        assert!(!frames_text.contains("PackedByteArray"));
        assert!(
            fs::metadata(target.join("forge_sprite_frames.tres"))
                .unwrap()
                .len()
                < 1024 * 1024
        );
        let usage: serde_json::Value =
            serde_json::from_slice(&fs::read(target.join("forge_usage.json")).unwrap()).unwrap();
        assert_eq!(
            usage["directionalPlayback"]["left"]["animation"],
            "walk_left"
        );
        assert_eq!(
            usage["directionalPlayback"]["left"]["idleAnimation"],
            "idle_left"
        );
        assert_eq!(usage["directionalPlayback"]["left"]["flipH"], false);
        let output = Command::new(&godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot V8 load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn fixture_v8_complete_validation_requests_only_the_selected_walk_video() {
    if forge_core::video::ffmpeg::find_in_path("ffmpeg").is_none()
        || forge_core::video::ffmpeg::find_in_path("ffprobe").is_none()
    {
        eprintln!("skipping V8 validation contract because ffmpeg/ffprobe are unavailable");
        return;
    }
    let (_temp, project, style, subject, plans, jobs, provider) = setup();
    let image_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(image_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let image_job = stage_plan_job(&jobs, &claimed).unwrap();
    let image_completed = run_operation_with_provider(
        &jobs,
        &image_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(
        image_completed.lifecycle_state,
        JobLifecycleState::AwaitingReview
    );
    let approved_source = approve_direction_motion_lock(&jobs, &image_job.job_id);

    let mut validation_request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    validation_request.asset_id = Some("v8-walk-right-validation".into());
    validation_request.validation_only = true;
    validation_request.validation_animations = vec!["walk_right".into()];
    validation_request.metadata.default_animation = "walk_right".into();
    validation_request.reuse_from_job_dir = Some(approved_source);
    let observations_before = provider.request_observations().len();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            validation_request,
        ))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 1);
    assert_eq!(prepared.estimate.maximum_provider_requests, 2);
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(!completed
        .artifacts
        .iter()
        .any(|artifact| matches!(artifact.kind.as_str(), "gsfpack" | "candidate_gsfpack")));

    let observations = provider.request_observations();
    let observations = &observations[observations_before..];
    assert_eq!(observations.len(), 1);
    let observation = &observations[0];
    assert_eq!(observation.kind, FixtureRequestKind::GenerateVideo);
    assert_eq!(
        observation.authorization_target.as_deref(),
        Some("walk_right:video")
    );
    let lock: DirectionMotionLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-motion-lock.json")).unwrap(),
    )
    .unwrap();
    let node = lock.video_input("walk_right").unwrap();
    assert_eq!(node.node_id, "right_walk");
    assert_eq!(
        observation.video_input_sha256.as_deref(),
        Some(node.generation_master_sha256.as_str())
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/direction-motion-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["stage"], "complete");
    assert_eq!(manifest["validationOnly"], true);
    assert_eq!(
        manifest["validationAnimations"],
        serde_json::json!(["walk_right"])
    );
    assert_eq!(manifest["usage"]["requests"], 1);
    assert_eq!(manifest["usage"]["generatedImages"], 0);
    assert_eq!(manifest["usage"]["generatedVideos"], 1);
    assert!(manifest["animations"].as_object().unwrap().len() == 1);
    assert!(manifest["animations"]["walk_right"].is_object());
}

#[test]
fn fixture_v8_complete_stage_fails_closed_when_the_typed_video_input_changes() {
    let (_temp, project, style, subject, plans, jobs, provider) = setup();
    let image_request = request(
        project.clone(),
        style.clone(),
        subject.clone(),
        DirectionMotionGenerationStageV1::ImageLocks,
    );
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(image_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let image_job = stage_plan_job(&jobs, &claimed).unwrap();
    let image_completed = run_operation_with_provider(
        &jobs,
        &image_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(
        image_completed.lifecycle_state,
        JobLifecycleState::AwaitingReview
    );
    let approved_source = approve_direction_motion_lock(&jobs, &image_job.job_id);

    // Mutate a copy of the approved pixels without updating the immutable Lock.
    // Complete must detect this before any Provider video request.
    let lock_path = approved_source.join("source/direction-motion-lock.json");
    let lock: DirectionMotionLockV1 =
        serde_json::from_slice(&fs::read(&lock_path).unwrap()).unwrap();
    let tampered = approved_source.join("direction-motion-lock/generation-masters/back_walk.png");
    let mut image = image::open(&tampered).unwrap().to_rgba8();
    image.put_pixel(0, 0, image::Rgba([1, 2, 3, 255]));
    image.save(&tampered).unwrap();
    let expected = lock
        .video_input("walk_up")
        .unwrap()
        .generation_master_sha256
        .clone();

    let mut complete_request = request(
        project,
        style,
        subject,
        DirectionMotionGenerationStageV1::Complete,
    );
    complete_request.reuse_from_job_dir = Some(approved_source);
    let observations_before = provider.request_observations().len();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(complete_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let failed =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap_err();
    assert!(failed
        .to_string()
        .contains("direction_motion_source_hash_mismatch: back_walk"));
    assert_eq!(
        provider.request_observations().len(),
        observations_before,
        "tampered input must fail before a paid video request"
    );
    assert_eq!(
        lock.video_input("walk_up")
            .unwrap()
            .generation_master_sha256,
        expected
    );
}
