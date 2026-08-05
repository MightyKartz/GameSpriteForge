use std::fs;
use std::path::{Path, PathBuf};

use forge_core::automation::{
    automation_profile, run_operation, stage_plan_job, AssetInput, AssetMetadata,
    AutomationOperation, CharacterAnimationRecipe, CharacterPackMetadata,
    CharacterWorkflowSelection, FixedGridSplit, GodotInstallRequest, MattingRecipe, PlanStore,
    PrepareAssetRequest, PrepareCharacterPackRequest, QualityPolicy, SpriteSheetSplit,
};
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
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
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
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    };

    let error = PlanStore::new(temp.path().join("plans"))
        .unwrap()
        .prepare(AutomationOperation::PrepareCharacterPack(request))
        .unwrap_err();

    assert!(error.to_string().contains("requires animations: jump"));
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

    fs::write(
        &manifest,
        "{\"schemaVersion\":\"1\",\"name\":\"changed\"}\n",
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
    let manifest = root.join("game-art.json");
    fs::write(&manifest, "{\"schemaVersion\":\"1\"}\n").unwrap();
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
