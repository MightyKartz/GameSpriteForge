use forge_core::{
    automation::{
        run_operation, stage_plan_job, AutomationOperation, PlanStore, PrepareStaticRequest,
    },
    delivery::{
        directory_sha256, hash_file, inventory, verify_install, write_install_snapshot,
        INSTALL_SNAPSHOT,
    },
    job::JobStore,
    project::{register_project_asset, RegisterProjectAsset},
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

/// An offline baseline fixture; verification deliberately does not claim native Godot loading.
fn fixture(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let png = root.join("source.png");
    let mut image = RgbaImage::new(16, 16);
    for y in 3..13 {
        for x in 4..12 {
            image.put_pixel(x, y, Rgba([255, 80, 40, 255]));
        }
    }
    image.save(&png).unwrap();
    let request: PrepareStaticRequest = serde_json::from_value(json!({
        "schemaVersion":"1","kind":"prop_set","id":"props","name":"Props","license":"private","sampling":"linear","canvasSize":64,
        "items":[{"id":"gem","name":"Gem","path":png}]
    })).unwrap();
    let plans = PlanStore::new(root.join("plans")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareStatic(request))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(root.join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    let pack = completed
        .artifacts
        .iter()
        .find(|a| a.kind == "gsfpack")
        .unwrap()
        .path
        .clone();
    let project = root.join("project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("project.godot"), "[application]\n").unwrap();
    let relative = Path::new("addons/forge_assets/props");
    let target = project.join(relative);
    fs::create_dir_all(target.join("items")).unwrap();
    fs::create_dir(target.join("scenes")).unwrap();
    fs::copy(
        pack.join("assets/items/gem.png"),
        target.join("items/gem.png"),
    )
    .unwrap();
    fs::write(
        target.join("scenes/gem.tscn"),
        "[gd_scene format=3]\n[node name=\"Gem\" type=\"Sprite2D\"]\n",
    )
    .unwrap();
    let summary = forge_pack::inspect_pack(&pack).unwrap();
    let pack_sha = directory_sha256(&pack).unwrap();
    let helper: Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap()).unwrap();
    let mut usage = json!({"schemaVersion":"1","assetKey":"props","assetId":summary.id,"packSha256":pack_sha,"kind":"prop_set",
        "scenePath":"res://addons/forge_assets/props/scenes","spriteFramesPath":"res://addons/forge_assets/props/items",
        "defaultAnimation":summary.default_animation,"animations":summary.animations,"items":summary.items});
    for key in ["rendering", "anchor", "frameWidth", "frameHeight"] {
        if let Some(value) = helper.get(key) {
            usage[key] = value.clone();
        }
    }
    write_json(&target.join("forge_usage.json"), &usage);
    write_json(
        &target.join(".forge-owned.json"),
        &json!({"schemaVersion":"1","assetKey":"props","jobId":"install-fixture","owner":"Game Sprite Forge","projectManifest":".forge/assets.json"}),
    );
    write_install_snapshot(&project, &target).unwrap();
    register_project_asset(RegisterProjectAsset {
        project_path: &project,
        asset_key: "props",
        pack_path: &pack,
        pack_sha256: &pack_sha,
        godot_target: relative,
        scene_path: &relative.join("scenes"),
        sprite_frames_path: &relative.join("items"),
        usage_path: &relative.join("forge_usage.json"),
        pack: &summary,
        provider_refs: &[],
        job_id: "install-fixture",
    })
    .unwrap();
    (project, target, pack)
}

#[test]
fn audit_is_read_only_and_detects_rewritten_native_resource_even_with_a_replaced_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let (project, target, pack) = fixture(root.path());
    let before = inventory(&project).unwrap();
    let report = verify_install(&project, "props", None).unwrap();
    assert_eq!(report["readOnly"], true);
    assert_eq!(report["nativeLoad"], "not_run");
    assert_eq!(report["verifiedTextures"], 1);
    assert_eq!(before, inventory(&project).unwrap());
    fs::write(target.join("scenes/gem.tscn"), "silently edited resource").unwrap();
    assert!(verify_install(&project, "props", Some(&pack))
        .unwrap_err()
        .to_string()
        .contains("resources differ"));
    write_install_snapshot(&project, &target).unwrap();
    assert!(verify_install(&project, "props", Some(&pack))
        .unwrap_err()
        .to_string()
        .contains("baseline differs"));
}

#[test]
fn cache_audit_rejects_stale_bytes_and_distinguishes_missing_cache_without_rebuilding() {
    let root = tempfile::tempdir().unwrap();
    let (project, target, pack) = fixture(root.path());
    let cache_dir = project.join(".godot/imported");
    fs::create_dir_all(&cache_dir).unwrap();
    let prefix = format!(
        "gem.png-{:x}",
        md5::compute(b"res://addons/forge_assets/props/items/gem.png")
    );
    let cache_relative = Path::new(".godot/imported").join(format!("{prefix}.ctex"));
    let cache = project.join(&cache_relative);
    fs::write(&cache, "original cached pixels").unwrap();
    let sidecar = target.join("items/gem.png.import");
    fs::write(
        &sidecar,
        format!("[remap]\npath=\"res://{}\"\n", cache_relative.display()),
    )
    .unwrap();
    write_install_snapshot(&project, &target).unwrap();
    let registry_path = project.join(".forge/assets.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    registry["assets"]["props"]["installSnapshotSha256"] =
        json!(hash_file(&target.join(INSTALL_SNAPSHOT)).unwrap());
    write_json(&registry_path, &registry);
    let before = inventory(&project).unwrap();
    assert_eq!(
        verify_install(&project, "props", Some(&pack)).unwrap()["cacheCheck"]["status"],
        "verified"
    );
    assert_eq!(before, inventory(&project).unwrap());
    fs::write(&cache, "stale pixels from failed update").unwrap();
    assert!(verify_install(&project, "props", Some(&pack))
        .unwrap_err()
        .to_string()
        .contains("texture cache differs"));
    fs::remove_file(&cache).unwrap();
    let before = inventory(&project).unwrap();
    let result = verify_install(&project, "props", Some(&pack)).unwrap();
    assert_eq!(result["cacheCheck"]["status"], "not_materialized");
    assert_eq!(result["cacheCheck"]["materialized"], false);
    assert_eq!(before, inventory(&project).unwrap());
    fs::write(&sidecar, "changed import metadata").unwrap();
    assert!(verify_install(&project, "props", Some(&pack))
        .unwrap_err()
        .to_string()
        .contains("import settings or routing differ"));
}

#[test]
fn audit_detects_pack_changes_and_texture_changes_even_if_the_local_baseline_is_updated() {
    let root = tempfile::tempdir().unwrap();
    let (project, target, pack) = fixture(root.path());
    let installed = target.join("items/gem.png");
    fs::write(&installed, "altered PNG").unwrap();
    write_install_snapshot(&project, &target).unwrap();
    let registry_path = project.join(".forge/assets.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    registry["assets"]["props"]["installSnapshotSha256"] =
        json!(hash_file(&target.join(INSTALL_SNAPSHOT)).unwrap());
    write_json(&registry_path, &registry);
    assert!(verify_install(&project, "props", Some(&pack))
        .unwrap_err()
        .to_string()
        .contains("texture differs"));
    fs::copy(pack.join("assets/items/gem.png"), &installed).unwrap();
    write_install_snapshot(&project, &target).unwrap();
    registry["assets"]["props"]["installSnapshotSha256"] =
        json!(hash_file(&target.join(INSTALL_SNAPSHOT)).unwrap());
    write_json(&registry_path, &registry);
    fs::write(pack.join("new-file.txt"), "Pack drift").unwrap();
    assert!(verify_install(&project, "props", Some(&pack))
        .unwrap_err()
        .to_string()
        .contains("Pack bytes differ"));
}

#[test]
fn audit_rejects_legacy_unanchored_baseline_and_registry_path_escape() {
    let root = tempfile::tempdir().unwrap();
    let (project, _target, _pack) = fixture(root.path());
    let registry_path = project.join(".forge/assets.json");
    let mut registry: Value = serde_json::from_slice(&fs::read(&registry_path).unwrap()).unwrap();
    let original = registry.clone();
    registry["assets"]["props"]
        .as_object_mut()
        .unwrap()
        .remove("installSnapshotSha256");
    write_json(&registry_path, &registry);
    assert!(verify_install(&project, "props", None)
        .unwrap_err()
        .to_string()
        .contains("no independently registered"));
    registry = original;
    registry["assets"]["props"]["godotTarget"] = json!("addons/forge_assets/../../outside");
    write_json(&registry_path, &registry);
    assert!(verify_install(&project, "props", None)
        .unwrap_err()
        .to_string()
        .contains("below addons/forge_assets"));
}

#[cfg(unix)]
#[test]
fn inventories_and_audits_reject_symbolic_links_including_ignored_import_files() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let (project, target, _pack) = fixture(root.path());
    let outside = root.path().join("outside.txt");
    fs::write(&outside, "untouched").unwrap();
    symlink(&outside, target.join("items/gem.png.import")).unwrap();
    assert!(verify_install(&project, "props", None)
        .unwrap_err()
        .to_string()
        .contains("symbolic link"));
    assert_eq!(fs::read_to_string(outside).unwrap(), "untouched");
}
