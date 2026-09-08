use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use forge_core::{
    asset_project::{
        export_static_pack, normalize_static_image, ConsistencyReportV1, ConsistencyVerdict,
        SamplingMode, StaticAssetKind, StaticAssetSetSpecV1, StaticPackItem, StyleLockV1,
    },
    automation::{
        run_operation, stage_plan_job, AutomationOperation, GodotInstallRequest, PlanStore,
    },
    job::{JobLifecycleState, JobStore},
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};

fn read_json(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

fn write_json(path: impl AsRef<Path>, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}

fn fixture_pack(root: &Path, kind: StaticAssetKind, sampling: SamplingMode) -> PathBuf {
    fs::create_dir_all(root).unwrap();
    let source = root.join("source.png");
    let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
    for y in 8..56 {
        for x in 16..48 {
            image.put_pixel(x, y, Rgba([180, 100, 60, 255]));
        }
    }
    image.save(&source).unwrap();
    let normalized_path = root.join("normalized.png");
    let normalized = normalize_static_image(
        &source,
        &normalized_path,
        64,
        kind == StaticAssetKind::PropSet,
    )
    .unwrap();
    if kind == StaticAssetKind::PropSet {
        let bottom = normalized
            .enumerate_pixels()
            .filter(|(_, _, p)| p[3] > 16)
            .map(|(_, y, _)| y + 1)
            .max()
            .unwrap();
        assert_eq!(
            bottom, 60,
            "normalization ground line must equal the exported anchor"
        );
    }
    let style: StyleLockV1 = serde_json::from_value(json!({
        "schemaVersion":"1", "revision":"fixture-style", "providerId":"fixture",
        "profileId":"default", "prompt":"fixture", "perspective":"topdown",
        "lighting":"upper_left", "outline":"soft", "background":"transparent",
        "sampling":sampling, "characterCanvasSize":64, "iconCanvasSize":64,
        "propCanvasSize":64, "boardPath":source, "boardSha256":"0".repeat(64),
        "referenceSha256":[], "baseline":{"palette":[], "edgeDensity":0.0,
        "foregroundScale":0.82, "perceptualHash":"0"}
    }))
    .unwrap();
    let asset: StaticAssetSetSpecV1 = serde_json::from_value(json!({
        "schemaVersion":"1", "kind":kind, "id":"static-fixture", "name":"Static fixture",
        "license":"CC0-1.0", "items":[{"id":"stone", "name":"Stone", "prompt":"fixture"}]
    }))
    .unwrap();
    let report = ConsistencyReportV1 {
        schema_version:"1".into(), profile:"fixture".into(), asset_type:kind.as_str().into(),
        style_revision:style.revision.clone(), style_baseline_profile:None,
        verdict:ConsistencyVerdict::GameReady,
        items:vec![serde_json::from_value(json!({"id":"stone","attempt":1,
            "metrics":{"paletteOverlap":1.0,"foregroundScaleRatio":1.0,"edgeDensityRatio":1.0,
                "anchorDriftPx":0.0,"canvasMatches":true,"alphaPresent":true,"cellBoundarySafe":true},
            "verdict":"game_ready","reasons":[]})).unwrap()],
    };
    export_static_pack(
        &root.join("exports"),
        &asset,
        &style,
        "fixture",
        &[StaticPackItem {
            id: "stone".into(),
            name: "Stone".into(),
            image_path: normalized_path,
        }],
        &report,
    )
    .unwrap()
    .pack_dir
}

fn make_legacy(pack: &Path) {
    let mut manifest = read_json(pack.join("assets/manifest.json"));
    manifest.as_object_mut().unwrap().remove("rendering");
    manifest["anchor"]["y"] = json!(64.0);
    write_json(pack.join("assets/manifest.json"), &manifest);
    let mut helper = read_json(pack.join("assets/godot_import.json"));
    for key in ["rendering", "anchor", "frameWidth", "frameHeight"] {
        helper.as_object_mut().unwrap().remove(key);
    }
    write_json(pack.join("assets/godot_import.json"), &helper);
    let mut metadata = read_json(pack.join("forgepack.json"));
    for key in ["rendering", "anchor"] {
        metadata["source"]["metadata"]
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    write_json(pack.join("forgepack.json"), &metadata);
}

#[test]
fn static_pack_preserves_sampling_and_ground_anchor() {
    for kind in [StaticAssetKind::IconSet, StaticAssetKind::PropSet] {
        for sampling in [SamplingMode::Nearest, SamplingMode::Linear] {
            let temp = tempfile::tempdir().unwrap();
            let pack = fixture_pack(temp.path(), kind, sampling.clone());
            forge_pack::validate_pack_layout(&pack).unwrap();
            let helper = read_json(pack.join("assets/godot_import.json"));
            assert_eq!(helper["rendering"]["textureFilter"], json!(sampling));
            assert_eq!(helper["anchor"]["x"], 32.0);
            assert_eq!(
                helper["anchor"]["y"],
                if kind == StaticAssetKind::IconSet {
                    32.0
                } else {
                    60.0
                }
            );
            assert_eq!(helper["frameWidth"], 64);
        }
    }
}

#[test]
fn static_pack_rejects_drifting_delivery_metadata() {
    for field in ["anchor", "rendering", "frameWidth", "items"] {
        let temp = tempfile::tempdir().unwrap();
        let pack = fixture_pack(temp.path(), StaticAssetKind::PropSet, SamplingMode::Linear);
        let mut helper = read_json(pack.join("assets/godot_import.json"));
        helper.as_object_mut().unwrap().remove(field);
        write_json(pack.join("assets/godot_import.json"), &helper);
        assert!(
            forge_pack::validate_pack_layout(&pack).is_err(),
            "missing {field} must fail"
        );
    }
}

#[test]
fn legacy_static_pack_remains_valid() {
    let temp = tempfile::tempdir().unwrap();
    let pack = fixture_pack(temp.path(), StaticAssetKind::PropSet, SamplingMode::Nearest);
    make_legacy(&pack);
    forge_pack::validate_pack_layout(&pack).unwrap();
}

#[test]
#[ignore = "requires a real Godot 4.6.x executable; run explicitly for delivery changes"]
fn static_pack_installs_in_real_godot_with_legacy_compatibility() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let project = root.join("game");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("project.godot"), "config_version=5\n[application]\nconfig/name=\"Static delivery test\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n").unwrap();
    let jobs = JobStore::new(root.join("jobs")).unwrap();
    let plans = PlanStore::new(root.join("plans")).unwrap();
    for (name, kind, sampling, legacy) in [
        (
            "linear",
            StaticAssetKind::PropSet,
            SamplingMode::Linear,
            false,
        ),
        (
            "nearest",
            StaticAssetKind::PropSet,
            SamplingMode::Nearest,
            false,
        ),
        (
            "icons",
            StaticAssetKind::IconSet,
            SamplingMode::Linear,
            false,
        ),
        (
            "legacy",
            StaticAssetKind::PropSet,
            SamplingMode::Nearest,
            true,
        ),
    ] {
        let pack = fixture_pack(&root.join(name), kind, sampling);
        if legacy {
            make_legacy(&pack);
        }
        let target = PathBuf::from("addons/forge_assets").join(name);
        let operation = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack,
            project_path: project.clone(),
            catalog_project_path: None,
            target: target.clone(),
            asset_key: Some(name.into()),
            provider_refs: vec![],
        });
        let prepared = plans.prepare(operation).unwrap();
        let plan = plans.claim(&prepared.token).unwrap();
        let queued = stage_plan_job(&jobs, &plan).unwrap();
        let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
        assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
        let usage = read_json(project.join(target).join("forge_usage.json"));
        assert_eq!(
            usage["texturePaths"]["stone"],
            format!("res://addons/forge_assets/{name}/items/stone.png")
        );
        if !legacy {
            assert_eq!(
                usage["rendering"]["textureFilter"],
                if name == "nearest" {
                    "nearest"
                } else {
                    "linear"
                }
            );
            assert_eq!(
                usage["anchor"]["y"],
                if name == "icons" { 32.0 } else { 60.0 }
            );
        }
    }
    fs::write(project.join("verify.gd"), r#"extends SceneTree
func _initialize() -> void:
	for name in ["linear", "nearest", "legacy"]:
		var packed = load("res://addons/forge_assets/%s/scenes/stone.tscn" % name)
		assert(packed is PackedScene)
		var root = packed.instantiate()
		var sprite = root.get_node("Sprite2D")
		if name == "legacy":
			assert(sprite.centered and sprite.position == Vector2.ZERO)
			assert(sprite.texture_filter == CanvasItem.TEXTURE_FILTER_PARENT_NODE)
		else:
			assert(not sprite.centered and sprite.position == Vector2(-32, -60))
			assert(sprite.texture_filter == (CanvasItem.TEXTURE_FILTER_LINEAR if name == "linear" else CanvasItem.TEXTURE_FILTER_NEAREST))
		root.free()
	print("PASS static Godot delivery: linear, nearest, icons, legacy")
	quit(0)
"#).unwrap();
    let godot = std::env::var("GODOT_BIN")
        .unwrap_or_else(|_| "/Applications/Godot.app/Contents/MacOS/Godot".into());
    let output = Command::new(godot)
        .args(["--headless", "--path"])
        .arg(&project)
        .args(["--script", "res://verify.gd", "--quit-after", "120"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("PASS static Godot delivery"),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
