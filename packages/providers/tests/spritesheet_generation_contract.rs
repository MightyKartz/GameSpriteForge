use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use forge_core::asset_project::{
    init_project, read_project, SamplingMode, StyleSpecV1, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::{
    automation_profile, run_operation, run_operation_with_provider, stage_plan_job,
    AutomationOperation, CharacterPackMetadata, CharacterWorkflowSelection, CreateStyleLockRequest,
    CreateSubjectLockRequest, GenerateCharacterPackRequest, GeneratedCharacterSpec,
    GenerationPolicy, GodotInstallRequest, PlanStore, QualityPolicy,
};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::project::ProviderAssetRef;
use forge_core::provider::{MediaGenerationProvider, ReferenceRole};
use forge_core::subject::{read_subject_lock, SubjectSpecV1};
use forge_providers::fixture::FixtureProvider;

#[test]
fn fixture_spritesheets_create_four_request_pack_and_native_godot_animation() {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "Sprite Sheet Contract").unwrap();
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
            id: "sheet-ranger".into(),
            name: "Sheet Ranger".into(),
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
            canonical_import_path: None,
            import_approval_note: None,
        }),
    );
    let subject_lock_path = project_root
        .join(".forge/subjects/sheet-ranger")
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
        asset_id: Some("sheet-ranger-character".into()),
        character: GeneratedCharacterSpec {
            prompt: subject.prompt.clone(),
            reference_image_path: Some(subject.canonical_path.clone()),
        },
        camera_profile: Default::default(),
        equipment: Default::default(),
        equipment_explicit: true,
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
        direction_grid_cape_contract: None,
        validation_only: false,
        validation_animations: vec![],
        motion_profile: Default::default(),
        direction_motion_stage: Default::default(),
        metadata: CharacterPackMetadata {
            name: "Sheet Ranger Character".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-spritesheet".into(),
            version: "3.0.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 4,
            video_duration_seconds: 4,
            image_model: Some("fixture-spritesheet".into()),
            video_model: None,
            pose_guidance: Default::default(),
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    };
    let request_template = request.clone();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 4);
    assert_eq!(prepared.estimate.maximum_provider_requests, 8);
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    let diagnostic = |name: &str| {
        fs::read_to_string(completed.job_dir.join(name)).unwrap_or_else(|_| "missing".into())
    };
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "error={:?} summary={:?}\nsheets={}\nmotion={}\nquality={}",
        completed.error_code,
        completed.error_summary,
        diagnostic("animation-sheet-reports.json"),
        diagnostic("character-motion-semantics-report.json"),
        diagnostic("animation-quality-report.json"),
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/spritesheet-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["workflow"], "topdown-spritesheet@3.0.0");
    assert_eq!(manifest["usage"]["requests"], 4);
    assert_eq!(manifest["actions"].as_array().unwrap().len(), 4);
    let sheet_observations = provider
        .edit_observations()
        .into_iter()
        .filter(|observation| {
            matches!(
                observation.authorization_target.as_deref(),
                Some("idle" | "walk_up" | "walk_right" | "walk_down")
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(sheet_observations.len(), 4);
    assert!(sheet_observations.iter().all(|observation| {
        observation.reference_roles == vec![ReferenceRole::SubjectIdentity, ReferenceRole::Style]
    }));

    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("spritesheet Character Pack");
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(
        forgepack["source"]["metadata"]["animationSheet"]["layout"],
        "2x2_row_major"
    );
    assert!(pack.path.join("animation-sheet-report.json").is_file());
    assert!(
        !fs::read_to_string(pack.path.join("animation-sheet-report.json"))
            .unwrap()
            .contains(temp.path().to_string_lossy().as_ref())
    );

    let mut action_retry = request_template.clone();
    action_retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    action_retry.retry_animations = vec!["walk_right".into()];
    action_retry.retry_stages.insert(
        "walk_right".into(),
        forge_core::automation::CharacterRetryStage::Still,
    );
    let action_retry_provider = FixtureProvider::default();
    let action_retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(action_retry))
        .unwrap();
    assert_eq!(action_retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(action_retry_plan.estimate.maximum_provider_requests, 2);
    let action_retry_plan = plans.claim(&action_retry_plan.token).unwrap();
    let action_retry_job = stage_plan_job(&jobs, &action_retry_plan).unwrap();
    let action_retry_result = run_operation_with_provider(
        &jobs,
        &action_retry_job.job_id,
        &action_retry_plan.operation,
        Some(&action_retry_provider),
    )
    .unwrap();
    assert_eq!(
        action_retry_result.lifecycle_state,
        JobLifecycleState::Succeeded
    );
    assert_eq!(action_retry_provider.usage().requests, 1);
    let action_retry_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            action_retry_result
                .job_dir
                .join("source/spritesheet-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(action_retry_manifest["usage"]["requests"], 1);
    assert_eq!(
        action_retry_manifest["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["reused"] == true)
            .count(),
        3
    );

    let mut local_replay = request_template.clone();
    local_replay.reuse_from_job_dir = Some(action_retry_result.job_dir.clone());
    local_replay.retry_animations = vec!["walk_right".into()];
    local_replay.retry_stages.insert(
        "walk_right".into(),
        forge_core::automation::CharacterRetryStage::Consistency,
    );
    let local_provider = FixtureProvider::default();
    let local_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(local_replay))
        .unwrap();
    assert_eq!(local_plan.estimate.provider_request_estimate, 0);
    assert_eq!(local_plan.estimate.maximum_provider_requests, 0);
    let local_plan = plans.claim(&local_plan.token).unwrap();
    let local_job = stage_plan_job(&jobs, &local_plan).unwrap();
    let local_result = run_operation_with_provider(
        &jobs,
        &local_job.job_id,
        &local_plan.operation,
        Some(&local_provider),
    )
    .unwrap();
    assert_eq!(local_result.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(local_provider.usage().requests, 0);

    let mut retry_validation = request_template.clone();
    retry_validation.asset_id = Some("sheet-ranger-retry-validation".into());
    retry_validation
        .character
        .prompt
        .push_str(" [fixture:retry_sheet_once]");
    retry_validation.validation_only = true;
    retry_validation.validation_animations = vec!["walk_right".into()];
    retry_validation.metadata.default_animation = "walk_right".into();
    let retry_provider = FixtureProvider::default().with_edit_failure_once("retry_sheet_once");
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry_validation))
        .unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 2);
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let retried = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&retry_provider),
    )
    .unwrap();
    assert_eq!(retried.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(!retried
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    let retry_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            retried
                .job_dir
                .join("source/spritesheet-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(retry_manifest["usage"]["requests"], 2);
    assert_eq!(retry_manifest["actions"].as_array().unwrap().len(), 1);

    let mut duplicate_validation = request_template;
    duplicate_validation.asset_id = Some("sheet-ranger-duplicate-validation".into());
    duplicate_validation
        .character
        .prompt
        .push_str(" [fixture:sheet_duplicate]");
    duplicate_validation.validation_only = true;
    duplicate_validation.validation_animations = vec!["walk_right".into()];
    duplicate_validation.metadata.default_animation = "walk_right".into();
    let duplicate_provider = FixtureProvider::default();
    let duplicate = run(
        &plans,
        &jobs,
        &duplicate_provider,
        AutomationOperation::GenerateCharacterPack(duplicate_validation),
    );
    assert_eq!(duplicate.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        duplicate.error_code.as_deref(),
        Some("animation_sheet_regeneration_required")
    );
    assert_eq!(duplicate_provider.usage().requests, 2);
    assert!(!duplicate
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));

    if let Some(godot) = locate_godot() {
        let godot_project = temp.path().join("godot-project");
        fs::create_dir(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge Sprite Sheet Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack.path.clone(),
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/sheet_ranger"),
            asset_key: Some("sheet_ranger".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("sheet-ranger-character".into()),
                label: Some("Sheet Ranger".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let target = godot_project.join("addons/forge_assets/sheet_ranger");
        let frames_text = fs::read_to_string(target.join("forge_sprite_frames.tres")).unwrap();
        assert!(frames_text.contains("filter_clip = true"));
        assert!(!frames_text.contains("PackedByteArray"));
        assert!(
            fs::metadata(target.join("forge_sprite_frames.tres"))
                .unwrap()
                .len()
                < 1024 * 1024
        );
        assert!(
            fs::metadata(target.join("forge_animated_sprite.tscn"))
                .unwrap()
                .len()
                < 1024 * 1024
        );
        let output = Command::new(godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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

fn locate_godot() -> Option<PathBuf> {
    [
        PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot"),
        PathBuf::from("/Applications/Godot_mono.app/Contents/MacOS/Godot"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .or_else(|| {
        ["godot4", "godot"].into_iter().find_map(|name| {
            let output = Command::new("/usr/bin/which").arg(name).output().ok()?;
            output
                .status
                .success()
                .then(|| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_string()))
        })
    })
}
