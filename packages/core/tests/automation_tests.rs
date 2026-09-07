use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;

use forge_core::automation::{
    automation_profile, run_operation, stage_plan_job, AssetInput, AssetMetadata,
    AutomationOperation, CharacterAnimationRecipe, CharacterPackMetadata,
    CharacterWorkflowSelection, FixedGridSplit, GodotInstallRequest, MattingRecipe, PlanStore,
    PrepareAssetRequest, PrepareCharacterPackRequest, QualityPolicy, SpriteSheetSplit,
};
use forge_core::character_camera::CharacterCameraProfileV1;
use forge_core::export::{CharacterMirrorPolicyV1, GodotRenderingContractV1, GodotTextureFilterV1};
use forge_core::job::{JobLifecycleState, JobOperationKind, JobStore};
use image::{Rgba, RgbaImage};
use tempfile::tempdir;

#[test]
fn bundled_profile_deserializes_with_locked_defaults() {
    let profile = automation_profile();

    assert_eq!(profile.id, "godot-pixel-art");
    assert_eq!(profile.version, "1.0.0");
    assert_eq!(profile.normalize.margin_bottom, 16);
    assert_eq!(profile.sheet.padding_px, 2);
    assert!(profile.quality.require_game_ready);
}

#[test]
fn plan_token_is_single_use() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let paths = write_identical_frames(&input);
    let store = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = store
        .prepare(AutomationOperation::PrepareAsset(request(paths)))
        .unwrap();

    let claimed = store.claim(&prepared.token).unwrap();
    assert_eq!(claimed.token, prepared.token);
    assert!(store.claim(&prepared.token).is_err());
}

#[test]
fn changed_input_invalidates_prepared_plan() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let paths = write_identical_frames(&input);
    let store = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = store
        .prepare(AutomationOperation::PrepareAsset(request(paths.clone())))
        .unwrap();
    RgbaImage::from_pixel(16, 16, Rgba([0, 0, 255, 255]))
        .save(&paths[0])
        .unwrap();

    let error = store.claim(&prepared.token).unwrap_err();
    assert!(error.to_string().contains("input changed"));
}

#[test]
fn tampered_pending_plan_operation_estimate_or_effects_cannot_be_claimed() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let paths = write_identical_frames(&input);
    let store = PlanStore::new(temp.path().join("plans")).unwrap();

    for mutation in ["operation", "estimate", "effects"] {
        let prepared = store
            .prepare(AutomationOperation::PrepareAsset(request(paths.clone())))
            .unwrap();
        let pending_path = store
            .root()
            .join(format!("{}.pending.json", prepared.token));
        let mut pending: serde_json::Value =
            serde_json::from_slice(&fs::read(&pending_path).unwrap()).unwrap();
        match mutation {
            "operation" => pending["operation"]["request"]["metadata"]["name"] = "tampered".into(),
            "estimate" => pending["estimate"]["maximumProviderRequests"] = 99.into(),
            "effects" => pending["effects"] = serde_json::json!(["tampered effect"]),
            _ => unreachable!(),
        }
        fs::write(&pending_path, serde_json::to_vec_pretty(&pending).unwrap()).unwrap();
        let error = store.claim(&prepared.token).unwrap_err();
        assert!(
            error.to_string().contains("input changed"),
            "{mutation}: {error}"
        );
    }
}

#[test]
fn prepare_asset_job_exports_valid_pack() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let operation = AutomationOperation::PrepareAsset(request(write_identical_frames(&input)));
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();

    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();

    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap();
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(
        forgepack["source"]["metadata"]["profile"],
        "godot-pixel-art@1.0.0"
    );
    assert!(forgepack["source"]["metadata"]["recipeHash"].is_string());
}

#[test]
fn concurrent_workers_execute_a_staged_job_exactly_once() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let operation = AutomationOperation::PrepareAsset(request(write_identical_frames(&input)));
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();

    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let jobs = jobs.clone();
        let job_id = queued.job_id.clone();
        let operation = plan.operation.clone();
        let barrier = barrier.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            run_operation(&jobs, &job_id, &operation)
        }));
    }
    barrier.wait();
    let results = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    assert!(results
        .iter()
        .filter_map(|result| result.as_ref().err())
        .all(|error| error.to_string().contains("job_execution_not_queued")));
    let completed = jobs.read_record(&queued.job_id).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(
        completed
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == "gsfpack")
            .count(),
        1
    );
}

#[test]
fn staged_job_rejects_a_different_operation_before_running() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let operation = AutomationOperation::PrepareAsset(request(write_identical_frames(&input)));
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let mut different_operation = plan.operation.clone();
    let AutomationOperation::PrepareAsset(request) = &mut different_operation else {
        unreachable!("test fixture must stage PrepareAsset");
    };
    request.metadata.name = "Operation substitution".into();

    let error = run_operation(&jobs, &queued.job_id, &different_operation).unwrap_err();

    assert!(error.to_string().contains("job_execution_not_queued"));
    let record = jobs.read_record(&queued.job_id).unwrap();
    assert_eq!(record.lifecycle_state, JobLifecycleState::Queued);
    assert!(record.artifacts.is_empty());
}

#[test]
fn queued_cancellation_prevents_operation_execution() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let operation = AutomationOperation::PrepareAsset(request(write_identical_frames(&input)));
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    jobs.request_cancellation(&queued.job_id).unwrap();

    let error = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap_err();

    assert!(error.to_string().contains("job_execution_not_queued"));
    let record = jobs.read_record(&queued.job_id).unwrap();
    assert_eq!(record.lifecycle_state, JobLifecycleState::Queued);
    assert!(record.cancellation_requested);
    assert!(record.artifacts.is_empty());
}

#[test]
fn character_pack_uses_shared_canvas_and_exports_multiple_animations() {
    let temp = tempdir().unwrap();
    let idle = temp.path().join("idle");
    let attack = temp.path().join("attack");
    fs::create_dir(&idle).unwrap();
    fs::create_dir(&attack).unwrap();
    let profile = automation_profile();
    let request = PrepareCharacterPackRequest {
        schema_version: "2".into(),
        metadata: CharacterPackMetadata {
            name: "Automation Knight".into(),
            default_animation: "attack".into(),
            creator: "Game Sprite Forge".into(),
            license: "private".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection::default(),
        animations: vec![
            CharacterAnimationRecipe {
                name: "idle".into(),
                input: AssetInput::PngSequence {
                    paths: write_identical_frames(&idle),
                },
                fps: 8.0,
                loop_animation: true,
                matting: MattingRecipe::PreserveAlpha,
            },
            CharacterAnimationRecipe {
                name: "attack".into(),
                input: AssetInput::PngSequence {
                    paths: write_identical_frames(&attack),
                },
                fps: 12.0,
                loop_animation: false,
                matting: MattingRecipe::PreserveAlpha,
            },
        ],
        character_prompt: None,
        camera_profile: None,
        equipment: Default::default(),
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
        source_cycle_sampling_profile: None,
        source_cycle_sampling_preview: false,
    };
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();

    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();

    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap();
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("assets/manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["animations"][0]["name"], "attack");
    assert_eq!(
        manifest["animations"][0]["frames"],
        serde_json::json!([0, 1, 2])
    );
    assert_eq!(manifest["animations"][0]["loop"], false);
    assert_eq!(manifest["animations"][1]["name"], "idle");
    assert_eq!(
        manifest["animations"][1]["frames"],
        serde_json::json!([3, 4, 5])
    );
    assert!(pack.path.join("previews/attack.gif").is_file());
    assert!(pack.path.join("previews/idle.gif").is_file());
    assert!(pack.path.join("quality/animations.json").is_file());

    let frames = fs::read_dir(pack.path.join("assets/frames"))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| image::open(entry.path()).unwrap().to_rgba8())
        .collect::<Vec<_>>();
    assert_eq!(frames.len(), 6);
    assert!(frames
        .iter()
        .all(|frame| frame.dimensions() == frames[0].dimensions()));
}

#[test]
fn fixed_grid_recipe_round_trips() {
    let value = serde_json::json!({
        "schemaVersion": "1",
        "input": {
            "kind": "sprite_sheet",
            "path": "/tmp/sheet.png",
            "split": {
                "mode": "fixed_grid",
                "frameWidth": 32,
                "frameHeight": 32,
                "columns": 4,
                "rows": 2
            }
        },
        "metadata": { "name": "Knight" }
    });
    let request: PrepareAssetRequest = serde_json::from_value(value).unwrap();
    assert!(matches!(
        request.input,
        AssetInput::SpriteSheet {
            split: SpriteSheetSplit::FixedGrid(FixedGridSplit { columns: 4, .. }),
            ..
        }
    ));
}

#[test]
fn character_pack_fields_are_rejected_in_v1() {
    let value = serde_json::json!({
        "schemaVersion": "1",
        "input": {
            "kind": "png_sequence",
            "paths": ["/tmp/idle_1.png", "/tmp/idle_2.png"]
        },
        "metadata": { "name": "Knight" },
        "animations": [
            { "name": "idle", "paths": ["/tmp/idle_1.png", "/tmp/idle_2.png"] },
            { "name": "walk", "paths": ["/tmp/walk_1.png", "/tmp/walk_2.png"] }
        ]
    });

    let error = serde_json::from_value::<PrepareAssetRequest>(value).unwrap_err();

    assert!(error.to_string().contains("unknown field `animations`"));
}

#[test]
fn godot_plan_rejects_targets_outside_forge_namespace() {
    let temp = tempdir().unwrap();
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("project.godot"), "[application]\n").unwrap();
    let input = temp.path().join("input");
    fs::create_dir(&input).unwrap();
    let operation = AutomationOperation::PrepareAsset(request(write_identical_frames(&input)));
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap();

    let error = plans
        .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack.path.clone(),
            project_path: project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("../outside"),
            asset_key: None,
            provider_refs: Vec::new(),
        }))
        .unwrap_err();

    assert!(error.to_string().contains("addons/forge_assets"));

    let unowned = project.join("addons/forge_assets/unowned");
    fs::create_dir_all(&unowned).unwrap();
    fs::write(unowned.join("user-file.txt"), "keep me").unwrap();
    let error = plans
        .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack.path.clone(),
            project_path: project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/unowned"),
            asset_key: None,
            provider_refs: Vec::new(),
        }))
        .unwrap_err();
    assert!(error.to_string().contains("not Forge-owned"));
    assert_eq!(
        fs::read_to_string(unowned.join("user-file.txt")).unwrap(),
        "keep me"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let outside = temp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        fs::remove_dir_all(project.join("addons/forge_assets")).unwrap();
        symlink(&outside, project.join("addons/forge_assets")).unwrap();

        let error = plans
            .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
                schema_version: "1".into(),
                pack_path: pack.path.clone(),
                project_path: project,
                catalog_project_path: None,
                target: PathBuf::from("addons/forge_assets/escaped"),
                asset_key: None,
                provider_refs: Vec::new(),
            }))
            .unwrap_err();

        assert!(error.to_string().contains("symbolic link"));
        assert!(!outside.join("escaped").exists());
    }
}

#[test]
fn guided_character_workflow_requires_its_core_animations() {
    let temp = tempdir().unwrap();
    let idle = temp.path().join("idle");
    let walk = temp.path().join("walk");
    fs::create_dir(&idle).unwrap();
    fs::create_dir(&walk).unwrap();
    let profile = automation_profile();
    let request = PrepareCharacterPackRequest {
        schema_version: "2".into(),
        metadata: CharacterPackMetadata {
            name: "Incomplete Platformer".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "private".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "platformer".into(),
            version: "1.0.0".into(),
        },
        animations: vec![
            CharacterAnimationRecipe {
                name: "idle".into(),
                input: AssetInput::PngSequence {
                    paths: write_identical_frames(&idle),
                },
                fps: 8.0,
                loop_animation: true,
                matting: MattingRecipe::PreserveAlpha,
            },
            CharacterAnimationRecipe {
                name: "walk".into(),
                input: AssetInput::PngSequence {
                    paths: write_identical_frames(&walk),
                },
                fps: 12.0,
                loop_animation: true,
                matting: MattingRecipe::PreserveAlpha,
            },
        ],
        character_prompt: None,
        camera_profile: None,
        equipment: Default::default(),
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
        source_cycle_sampling_profile: None,
        source_cycle_sampling_preview: false,
    };

    let error = PlanStore::new(temp.path().join("plans"))
        .unwrap()
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap_err();

    assert!(error.to_string().contains("requires animations: jump"));
}

#[test]
fn external_keyframes_plan_accepts_independent_transparent_pngs_without_provider_budget() {
    let temp = tempdir().unwrap();
    let request =
        external_keyframe_request(temp.path(), CharacterMirrorPolicyV1::MirrorRightToLeft);
    let prepared = PlanStore::new(temp.path().join("plans"))
        .unwrap()
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap();

    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
    assert_eq!(
        prepared.estimate.workflow.as_deref(),
        Some("topdown-external-keyframes@11.0.0")
    );
}

#[test]
fn external_keyframes_plan_accepts_explicit_right_only_contract() {
    let temp = tempdir().unwrap();
    let request = external_keyframe_request(temp.path(), CharacterMirrorPolicyV1::RightOnly);
    let prepared = PlanStore::new(temp.path().join("plans"))
        .unwrap()
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap();

    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
}

#[test]
fn external_keyframes_right_only_rejects_missing_right_animation() {
    let temp = tempdir().unwrap();
    let mut request = external_keyframe_request(temp.path(), CharacterMirrorPolicyV1::RightOnly);
    request
        .animations
        .retain(|animation| animation.name != "walk_right");
    let error = PlanStore::new(temp.path().join("plans"))
        .unwrap()
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("requires animations: walk_right"));
}

#[test]
fn external_keyframes_right_only_rejects_left_animations() {
    let temp = tempdir().unwrap();
    let mut request = external_keyframe_request(temp.path(), CharacterMirrorPolicyV1::RightOnly);
    request.animations.extend([
        CharacterAnimationRecipe {
            name: "idle_left".into(),
            input: AssetInput::PngSequence {
                paths: write_external_frames(temp.path(), "idle_left", 1),
            },
            fps: 1.0,
            loop_animation: false,
            matting: MattingRecipe::PreserveAlpha,
        },
        CharacterAnimationRecipe {
            name: "walk_left".into(),
            input: AssetInput::PngSequence {
                paths: write_external_frames(temp.path(), "walk_left", 4),
            },
            fps: 4.0,
            loop_animation: true,
            matting: MattingRecipe::PreserveAlpha,
        },
    ]);
    let error = PlanStore::new(temp.path().join("plans"))
        .unwrap()
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("external_keyframes_left_forbidden"));
}

#[test]
fn external_keyframes_plan_requires_explicit_mirror_policy() {
    let temp = tempdir().unwrap();
    let request = external_keyframe_request(temp.path(), CharacterMirrorPolicyV1::Auto);
    let error = PlanStore::new(temp.path().join("plans"))
        .unwrap()
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap_err();

    assert!(error
        .to_string()
        .contains("external_keyframes_mirror_policy_required"));
}

#[test]
fn build_project_plan_round_trips_with_zero_provider_estimate() {
    let temp = tempdir().unwrap();
    let (project, manifest) = build_project_fixture(temp.path());
    let store = PlanStore::new(temp.path().join("plans")).unwrap();

    let prepared = store
        .prepare(build_project_operation(&project, &manifest))
        .unwrap();

    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
    assert_eq!(prepared.estimate.cache_hit_count, 0);
    assert!(prepared.estimate.provider_id.is_none());
    assert!(prepared.estimate.profile_id.is_none());
    assert_eq!(prepared.effects.len(), 1);
    assert!(prepared.effects[0].contains("build project from manifest"));

    let claimed = store.claim(&prepared.token).unwrap();
    assert!(matches!(
        claimed.operation,
        AutomationOperation::BuildProject(_)
    ));
}

#[test]
fn build_project_fingerprint_tracks_manifest_contents() {
    let temp = tempdir().unwrap();
    let (project, manifest) = build_project_fixture(temp.path());
    let store = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = store
        .prepare(build_project_operation(&project, &manifest))
        .unwrap();

    let mut manifest_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    manifest_json["name"] = "changed".into();
    fs::write(
        &manifest,
        serde_json::to_vec_pretty(&manifest_json).unwrap(),
    )
    .unwrap();

    let error = store.claim(&prepared.token).unwrap_err();
    assert!(error.to_string().contains("input changed"));
}

#[test]
fn build_project_validate_rejects_bad_paths() {
    let temp = tempdir().unwrap();
    let (project, manifest) = build_project_fixture(temp.path());
    let store = PlanStore::new(temp.path().join("plans")).unwrap();

    let error = store
        .prepare(build_project_operation(
            &project,
            Path::new("game-art.json"),
        ))
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("manifestPath must be an absolute path"));

    let error = store
        .prepare(build_project_operation(Path::new("project"), &manifest))
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("projectPath must be an absolute path"));

    let error = store
        .prepare(build_project_operation(
            &project,
            &temp.path().join("missing.json"),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("manifest file does not exist"));

    let error = store
        .prepare(build_project_operation(
            &temp.path().join("missing-dir"),
            &manifest,
        ))
        .unwrap_err();
    assert!(error
        .to_string()
        .contains("project directory does not exist"));

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let link = temp.path().join("linked-manifest.json");
        symlink(&manifest, &link).unwrap();
        let error = store
            .prepare(build_project_operation(&project, &link))
            .unwrap_err();
        assert!(error.to_string().contains("symbolic link"));
    }
}

#[test]
fn build_project_job_stages_steps_and_fails_on_invalid_manifest() {
    let temp = tempdir().unwrap();
    let (project, manifest) = build_project_fixture(temp.path());
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans
        .prepare(build_project_operation(&project, &manifest))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();

    assert_eq!(queued.operation_kind, JobOperationKind::BuildProject);
    let step_names = queued
        .steps
        .iter()
        .map(|step| step.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        step_names,
        [
            "validate_manifest",
            "diff_catalog",
            "run_child_builds",
            "update_catalog",
            "summarize"
        ]
    );

    fs::write(&manifest, "{\"schemaVersion\":\"1\"}\n").unwrap();

    // The fail-closed placeholder is gone: the runner now dispatches to the
    // build orchestrator, which rejects this malformed manifest with a
    // manifest-level error. FORGE_PLAN_STORE keeps the dispatch's child
    // PlanStore inside the tempdir.
    let run = || run_operation(&jobs, &queued.job_id, &plan.operation);
    let error = temp_env::with_var(
        "FORGE_PLAN_STORE",
        Some(temp.path().join("child-plans")),
        run,
    )
    .unwrap_err();
    assert_eq!(error.code(), "invalid_json");
    let record = jobs.read_record(&queued.job_id).unwrap();
    assert_eq!(record.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(record.error_code.as_deref(), Some("invalid_json"));
}

fn build_project_fixture(root: &Path) -> (PathBuf, PathBuf) {
    let project = root.join("project");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("forge-project.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1",
            "projectId": "test-project",
            "name": "Test Project",
            "provider": { "id": "fixture", "profileId": "default" },
            "outputDir": "build"
        }))
        .unwrap(),
    )
    .unwrap();
    let manifest = root.join("game-art.json");
    fs::write(
        &manifest,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1",
            "kind": "game_art_manifest",
            "projectId": "test-project",
            "name": "Test Project",
            "provider": { "id": "fixture", "profileId": "default" },
            "defaults": {
                "outputDirectory": "packs",
                "godotRoot": "addons/forge_assets",
                "license": "private"
            },
            "assets": []
        }))
        .unwrap(),
    )
    .unwrap();
    (project, manifest)
}

fn build_project_operation(project: &Path, manifest: &Path) -> AutomationOperation {
    serde_json::from_value(serde_json::json!({
        "kind": "build_project",
        "request": {
            "schemaVersion": "1",
            "projectPath": project,
            "manifestPath": manifest
        }
    }))
    .unwrap()
}

fn request(paths: Vec<PathBuf>) -> PrepareAssetRequest {
    let profile = automation_profile();
    PrepareAssetRequest {
        schema_version: "1".into(),
        input: AssetInput::PngSequence { paths },
        metadata: AssetMetadata {
            name: "Automation Hero".into(),
            animation: "idle".into(),
            fps: 12.0,
            loop_animation: true,
            creator: "Game Sprite Forge".into(),
            license: "private".into(),
        },
        matting: MattingRecipe::PreserveAlpha,
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    }
}

fn external_keyframe_request(
    root: &Path,
    mirror_policy: CharacterMirrorPolicyV1,
) -> PrepareCharacterPackRequest {
    let profile = automation_profile();
    let definitions = [
        ("idle_down", 1, 1.0, false),
        ("idle_up", 1, 1.0, false),
        ("idle_right", 1, 1.0, false),
        ("walk_down", 4, 4.0, true),
        ("walk_up", 4, 4.0, true),
        ("walk_right", 4, 4.0, true),
    ];
    let animations = definitions
        .into_iter()
        .map(
            |(name, frame_count, fps, loop_animation)| CharacterAnimationRecipe {
                name: name.into(),
                input: AssetInput::PngSequence {
                    paths: write_external_frames(root, name, frame_count),
                },
                fps,
                loop_animation,
                matting: MattingRecipe::PreserveAlpha,
            },
        )
        .collect();
    PrepareCharacterPackRequest {
        schema_version: "2".into(),
        metadata: CharacterPackMetadata {
            name: "External Codex Ranger".into(),
            default_animation: "idle_down".into(),
            creator: "Game Sprite Forge".into(),
            license: "private".into(),
            rendering: GodotRenderingContractV1 {
                texture_filter: GodotTextureFilterV1::Linear,
                pixel_snap: false,
                mirror_policy,
                ..GodotRenderingContractV1::default()
            },
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-external-keyframes".into(),
            version: "11.0.0".into(),
        },
        animations,
        character_prompt: Some("a hooded ranger with an olive cape and orange scarf".into()),
        camera_profile: Some(CharacterCameraProfileV1::TopdownThreeQuarter),
        equipment: Default::default(),
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
        source_cycle_sampling_profile: None,
        source_cycle_sampling_preview: false,
    }
}

fn write_external_frames(root: &Path, animation: &str, frame_count: usize) -> Vec<PathBuf> {
    let directory = root.join(animation);
    fs::create_dir_all(&directory).unwrap();
    let seed = animation.bytes().fold(0_u8, u8::wrapping_add);
    (0..frame_count)
        .map(|index| {
            let path = directory.join(format!("frame_{index}.png"));
            let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
            let offset = index as u32;
            for y in 12..58 {
                for x in (20 + offset)..(44 + offset) {
                    image.put_pixel(
                        x.min(62),
                        y,
                        Rgba([seed, 80_u8.wrapping_add(index as u8), 40, 255]),
                    );
                }
            }
            image.save(&path).unwrap();
            path
        })
        .collect()
}

fn write_identical_frames(directory: &std::path::Path) -> Vec<PathBuf> {
    (0..3)
        .map(|index| {
            let path = directory.join(format!("frame_{index}.png"));
            let mut image = RgbaImage::from_pixel(16, 16, Rgba([0, 0, 0, 0]));
            for y in 4..15 {
                for x in 5..11 {
                    image.put_pixel(x, y, Rgba([255, 80, 40, 255]));
                }
            }
            image.save(&path).unwrap();
            path
        })
        .collect()
}
