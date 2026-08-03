use std::fs;
use std::path::PathBuf;

use forge_core::asset_project::{
    init_project, read_project, ForgeProjectV1, SamplingMode, StaticAssetItemSpecV1,
    StaticAssetKind, StaticAssetSetSpecV1, StyleSpecV1, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::{
    run_operation, run_operation_with_provider, stage_plan_job, AutomationOperation,
    CreateStyleLockRequest, GenerateStaticAssetSetRequest, GodotInstallRequest, PlanStore,
};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::provider::MediaGenerationProvider;
use forge_providers::fixture::FixtureProvider;

#[test]
fn fixture_generates_consistent_icon_set_pack() {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("game-assets");
    init_project(&project_root, "Game Assets").unwrap();
    let mut project: ForgeProjectV1 = read_project(&project_root).unwrap();
    project.provider.id = "fixture".into();
    fs::write(
        project_root.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    let style_spec_path = project_root.join("specs/style.json");
    let style_spec = StyleSpecV1 {
        schema_version: "1".into(),
        prompt: "compact jewel-tone pixel art".into(),
        reference_images: vec![],
        perspective: "topdown".into(),
        lighting: "upper_left".into(),
        outline: "dark".into(),
        background: "transparent".into(),
        sampling: SamplingMode::Nearest,
        character_canvas_size: 256,
        icon_canvas_size: 128,
        prop_canvas_size: 256,
        image_model: None,
    };
    fs::write(
        &style_spec_path,
        serde_json::to_vec_pretty(&style_spec).unwrap(),
    )
    .unwrap();
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::CreateStyleLock(
            CreateStyleLockRequest {
                schema_version: "1".into(),
                project_path: project_root.clone(),
                spec_path: style_spec_path,
                provider_id: "fixture".into(),
                profile_id: "default".into(),
            },
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let style_job = stage_plan_job(&jobs, &claimed).unwrap();
    let style_result = run_operation_with_provider(
        &jobs,
        &style_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(style_result.lifecycle_state, JobLifecycleState::Succeeded);
    let project = read_project(&project_root).unwrap();
    let revision = project.current_style_revision.unwrap();
    let style_lock_path = project_root
        .join(".forge/styles")
        .join(revision)
        .join(STYLE_LOCK_FILE);

    let request = GenerateStaticAssetSetRequest {
        schema_version: "4".into(),
        project_path: project_root.clone(),
        style_lock_path: style_lock_path.clone(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        asset: StaticAssetSetSpecV1 {
            schema_version: "1".into(),
            kind: StaticAssetKind::IconSet,
            id: "inventory-icons".into(),
            name: "Inventory Icons".into(),
            items: vec![
                StaticAssetItemSpecV1 {
                    id: "potion".into(),
                    name: "Potion".into(),
                    prompt: "a red healing potion".into(),
                    reference_image: None,
                },
                StaticAssetItemSpecV1 {
                    id: "key".into(),
                    name: "Key".into(),
                    prompt: "a small brass key".into(),
                    reference_image: None,
                },
            ],
            license: "private".into(),
        },
        max_attempts_per_item: 2,
        image_model: None,
        reuse_from_job_dir: None,
        retry_item_ids: vec![],
        consistency_recheck_only: false,
    };
    let prepared = plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(request.clone()))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let generation_job = stage_plan_job(&jobs, &claimed).unwrap();
    let result = run_operation_with_provider(
        &jobs,
        &generation_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    if result.lifecycle_state != JobLifecycleState::Succeeded {
        eprintln!(
            "{}",
            fs::read_to_string(result.job_dir.join("consistency-report.json")).unwrap()
        );
    }
    assert_eq!(result.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = result
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .map(|artifact| artifact.path.clone())
        .unwrap_or_else(|| PathBuf::from("missing"));
    forge_pack::validate_pack_layout(&pack).unwrap();
    let inspected = forge_pack::inspect_pack(&pack).unwrap();
    assert_eq!(inspected.asset_type, "icon_set");
    assert_eq!(inspected.items.len(), 2);
    assert!(pack.join("consistency-report.json").is_file());

    let usage_before_recheck = provider.usage();
    let recheck_request = GenerateStaticAssetSetRequest {
        reuse_from_job_dir: Some(result.job_dir.clone()),
        retry_item_ids: request
            .asset
            .items
            .iter()
            .map(|item| item.id.clone())
            .collect(),
        consistency_recheck_only: true,
        ..request.clone()
    };
    let prepared = plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(recheck_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let recheck_job = stage_plan_job(&jobs, &claimed).unwrap();
    let rechecked = run_operation_with_provider(
        &jobs,
        &recheck_job.job_id,
        &claimed.operation,
        Some(&provider),
    )
    .unwrap();
    assert_eq!(rechecked.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(provider.usage(), usage_before_recheck);
    assert_eq!(
        rechecked
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind.starts_with("rechecked_item_"))
            .count(),
        2
    );
    let recheck_report: serde_json::Value = serde_json::from_slice(
        &fs::read(rechecked.job_dir.join("consistency-report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(recheck_report["profile"], "consistency@1.3.0");
    assert_eq!(
        recheck_report["styleBaselineProfile"],
        "style-baseline@2.3.0"
    );

    let Some(_godot) = locate_godot() else {
        eprintln!("skipping static Godot contract because Godot 4 is unavailable");
        return;
    };
    let godot_project = temp.path().join("godot-project");
    fs::create_dir(&godot_project).unwrap();
    fs::write(
        godot_project.join("project.godot"),
        "[application]\nconfig/name=\"Forge Static Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
    )
    .unwrap();
    let prepared = plans
        .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack,
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/inventory_icons"),
            asset_key: Some("inventory_icons".into()),
            provider_refs: vec![],
        }))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let install_job = stage_plan_job(&jobs, &claimed).unwrap();
    let installed = run_operation(&jobs, &install_job.job_id, &claimed.operation).unwrap();
    assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(godot_project
        .join("addons/forge_assets/inventory_icons/items/potion.png")
        .is_file());
    let usage: serde_json::Value = serde_json::from_slice(
        &fs::read(godot_project.join("addons/forge_assets/inventory_icons/forge_usage.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(usage["kind"], "icon_set");
    assert_eq!(usage["nodeType"], "Texture2D");
    assert_eq!(usage["items"].as_array().unwrap().len(), 2);
    assert_eq!(usage["providerProvenance"][0]["provider"], "fixture");
    assert!(usage["providerProvenance"][0].get("assetId").is_none());

    let prop_request = GenerateStaticAssetSetRequest {
        schema_version: "4".into(),
        project_path: project_root,
        style_lock_path,
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        asset: StaticAssetSetSpecV1 {
            schema_version: "1".into(),
            kind: StaticAssetKind::PropSet,
            id: "forest-props".into(),
            name: "Forest Props".into(),
            items: vec![StaticAssetItemSpecV1 {
                id: "chest".into(),
                name: "Chest".into(),
                prompt: "a compact wooden chest".into(),
                reference_image: None,
            }],
            license: "private".into(),
        },
        max_attempts_per_item: 2,
        image_model: None,
        reuse_from_job_dir: None,
        retry_item_ids: vec![],
        consistency_recheck_only: false,
    };
    let prepared = plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(prop_request))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let prop_job = stage_plan_job(&jobs, &claimed).unwrap();
    let prop_result =
        run_operation_with_provider(&jobs, &prop_job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    assert_eq!(prop_result.lifecycle_state, JobLifecycleState::Succeeded);
    let prop_pack = prop_result
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap()
        .path
        .clone();
    let prepared = plans
        .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: prop_pack,
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/forest_props"),
            asset_key: Some("forest_props".into()),
            provider_refs: vec![],
        }))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let install_job = stage_plan_job(&jobs, &claimed).unwrap();
    let installed = run_operation(&jobs, &install_job.job_id, &claimed.operation).unwrap();
    assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
    let prop_scene = godot_project.join("addons/forge_assets/forest_props/scenes/chest.tscn");
    assert!(prop_scene.is_file());
    assert!(fs::metadata(&prop_scene).unwrap().len() < 1024 * 1024);
    assert!(!fs::read_to_string(prop_scene)
        .unwrap()
        .contains("PackedByteArray"));
}

fn locate_godot() -> Option<PathBuf> {
    [
        PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot"),
        PathBuf::from("/Applications/Godot_mono.app/Contents/MacOS/Godot"),
    ]
    .into_iter()
    .find(|path| path.is_file())
}
