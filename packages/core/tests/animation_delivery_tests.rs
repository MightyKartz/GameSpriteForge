use forge_core::{
    automation::{
        run_operation, stage_plan_job, AutomationOperation, PlanStore, PrepareAssetRequest,
        PrepareCharacterPackRequest,
    },
    frames::{manual_anchor, normalize_frames, CanvasMode, NormalizeOptions},
    job::{JobLifecycleState, JobStore},
};
use image::{Rgba, RgbaImage};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn fixtures(root: &Path) -> Vec<PathBuf> {
    (0..3)
        .map(|index| {
            let mut frame = RgbaImage::new(64, 64);
            for y in 20..52 {
                for x in 22..38 {
                    frame.put_pixel(x, y, Rgba([200, 70, 90, 255]));
                }
            }
            // Moving sleeve changes bbox center; the torso must remain at source coordinates.
            for x in 38..(43 + index * 2) {
                frame.put_pixel(x, 30, Rgba([80, 150, 210, 128]));
            }
            let path = root.join(format!("{index}.png"));
            frame.save(&path).unwrap();
            path
        })
        .collect()
}
fn recipe(paths: &[PathBuf]) -> Value {
    json!({"schemaVersion":"1", "input":{"kind":"png_sequence","paths":paths},
        "metadata":{"name":"Local animation", "animation":"idle", "fps":10.0,"frameDurationsMs":[70,150,230]},
        "rendering":{"textureFilter":"linear","pixelSnap":false},
        "normalize":{"mode":"preserve_source","margin":0,"marginBottom":0,"alphaThreshold":0,
            "manualAnchor":{"x":32.5,"y":52.25,"lockedByUser":true}},
        "quality":{"requireGameReady":false}})
}
fn run(root: &Path, operation: AutomationOperation) -> (PathBuf, PathBuf) {
    let plans = PlanStore::new(root.join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(root.join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = completed
        .artifacts
        .iter()
        .find(|a| a.kind == "gsfpack")
        .unwrap()
        .path
        .clone();
    forge_pack::validate_pack_layout(&pack).unwrap();
    (pack, completed.job_dir)
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn preserve_source_keeps_pixels_and_shared_anchor_despite_changing_bbox() {
    let temp = tempfile::tempdir().unwrap();
    let frames: Vec<_> = fixtures(temp.path())
        .iter()
        .map(|p| image::open(p).unwrap().to_rgba8())
        .collect();
    let normalized = normalize_frames(
        &frames,
        NormalizeOptions {
            mode: CanvasMode::PreserveSource,
            manual_anchor: Some(manual_anchor(32.5, 52.25)),
            ..Default::default()
        },
    );
    for (input, output) in frames.iter().zip(normalized) {
        assert_eq!(input, &output.image);
        assert_eq!((output.offset_x, output.offset_y), (0, 0));
        assert_eq!(output.anchor, manual_anchor(32.5, 52.25));
    }
}
#[test]
fn local_asset_pack_preserves_coordinate_rendering_and_timing_contract() {
    let temp = tempfile::tempdir().unwrap();
    let paths = fixtures(temp.path());
    let (pack, job) = run(
        temp.path(),
        AutomationOperation::PrepareAsset(serde_json::from_value(recipe(&paths)).unwrap()),
    );
    for (index, source) in paths.iter().enumerate() {
        assert_eq!(
            image::open(source).unwrap().to_rgba8(),
            image::open(pack.join(format!("assets/frames/frame_{:03}.png", index + 1)))
                .unwrap()
                .to_rgba8()
        );
    }
    let manifest = read(pack.join("assets/manifest.json"));
    let helper = read(pack.join("assets/godot_import.json"));
    let forgepack = read(pack.join("forgepack.json"));
    assert_eq!(
        manifest["rendering"],
        json!({"profile":"godot-sprite-rendering@1.0.0","textureFilter":"linear","pixelSnap":false,"mirrorPolicy":"auto"})
    );
    assert_eq!(helper["spriteFrames"]["rendering"], manifest["rendering"]);
    assert_eq!(
        manifest["animations"][0]["frameDurationsMs"],
        json!([70, 150, 230])
    );
    assert_eq!(forgepack["animations"], manifest["animations"]);
    assert_eq!(helper["spriteFrames"]["animations"], manifest["animations"]);
    assert_eq!(manifest["anchor"]["x"], 32.5);
    for frame in read(job.join("normalized-frames.json")).as_array().unwrap() {
        assert_eq!(frame["offsetX"], 0);
        assert_eq!(frame["offsetY"], 0);
    }
}
#[test]
fn character_actions_share_canvas_and_timing_survives_default_action_reordering() {
    let temp = tempfile::tempdir().unwrap();
    let paths = fixtures(temp.path());
    let mut value = recipe(&paths);
    value["schemaVersion"] = json!("2");
    value["metadata"] = json!({"name":"Two actions", "defaultAnimation":"attack"});
    let input = value.as_object_mut().unwrap().remove("input").unwrap();
    value["animations"] = json!([
        {"name":"idle","input":input,"fps":10,"frameDurationsMs":[70,150,230]},
        {"name":"attack","input":input,"fps":12,"loop":false,"frameDurationsMs":[60,240,100]}
    ]);
    let (pack, job) = run(
        temp.path(),
        AutomationOperation::PrepareCharacterPack(serde_json::from_value(value).unwrap()),
    );
    let manifest = read(pack.join("assets/manifest.json"));
    assert_eq!(manifest["animations"][0]["name"], "attack");
    assert_eq!(
        manifest["animations"][0]["frameDurationsMs"],
        json!([60, 240, 100])
    );
    assert_eq!(
        manifest["animations"][1]["frameDurationsMs"],
        json!([70, 150, 230])
    );
    assert_eq!(manifest["sheet"]["frameWidth"], 64);
    assert_eq!(manifest["sheet"]["frameHeight"], 64);
    for frame in read(job.join("normalized-frames.json")).as_array().unwrap() {
        assert_eq!(frame["anchor"]["x"], 32.5);
        assert_eq!(frame["anchor"]["y"], 52.25);
        assert_eq!(frame["offsetX"], 0);
        assert_eq!(frame["offsetY"], 0);
    }
}
#[test]
fn invalid_local_animation_parameters_are_rejected_before_job_execution() {
    let temp = tempfile::tempdir().unwrap();
    let paths = fixtures(temp.path());
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    for (pointer, bad, message) in [
        (
            "/metadata/frameDurationsMs",
            json!([70, 150]),
            "one positive integer per frame",
        ),
        (
            "/metadata/frameDurationsMs",
            json!([70, 0, 230]),
            "one positive integer per frame",
        ),
        ("/normalize/margin", json!(1), "requires margin"),
        (
            "/normalize/manualAnchor/x",
            json!(65),
            "inside the preserved source canvas",
        ),
        ("/rendering/pixelSnap", json!(true), "integer manualAnchor"),
    ] {
        let mut value = recipe(&paths);
        *value.pointer_mut(pointer).unwrap() = bad;
        let request: PrepareAssetRequest = serde_json::from_value(value).unwrap();
        let error = plans
            .prepare(AutomationOperation::PrepareAsset(request))
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
    for (pointer, bad) in [
        ("/rendering/textureFilter", json!("cubic")),
        ("/metadata/frameDurationsMs", json!([70, -1, 230])),
    ] {
        let mut value = recipe(&paths);
        *value.pointer_mut(pointer).unwrap() = bad;
        assert!(serde_json::from_value::<PrepareAssetRequest>(value).is_err());
    }
    let mut no_rendering = recipe(&paths);
    no_rendering.as_object_mut().unwrap().remove("rendering");
    let error = plans
        .prepare(AutomationOperation::PrepareAsset(
            serde_json::from_value(no_rendering).unwrap(),
        ))
        .unwrap_err();
    assert!(error.to_string().contains("integer manualAnchor"));
    let mut unsupported = recipe(&paths);
    unsupported["rendering"]["profile"] = json!("future");
    assert!(serde_json::from_value::<PrepareAssetRequest>(unsupported).is_err());
    RgbaImage::new(65, 64).save(&paths[1]).unwrap();
    let request: PrepareAssetRequest = serde_json::from_value(recipe(&paths)).unwrap();
    let error = plans
        .prepare(AutomationOperation::PrepareAsset(request))
        .unwrap_err();
    assert!(error.to_string().contains("identical source canvas"));
}
#[test]
fn omitted_new_options_keep_legacy_defaults_and_payload_shape() {
    let temp = tempfile::tempdir().unwrap();
    let paths = fixtures(temp.path());
    let value = json!({"input":{"kind":"png_sequence","paths":paths},"metadata":{"name":"Legacy"}});
    let request: PrepareAssetRequest = serde_json::from_value(value).unwrap();
    assert!(request.rendering.is_none());
    assert!(request.metadata.frame_durations_ms.is_none());
    assert_eq!(request.normalize.mode, CanvasMode::SquareBottom);
    let serialized = serde_json::to_value(&request).unwrap();
    assert!(serialized.get("rendering").is_none());
    assert!(serialized["metadata"].get("frameDurationsMs").is_none());
    let character: PrepareCharacterPackRequest = serde_json::from_value(
        json!({"metadata":{"name":"Legacy","defaultAnimation":"idle"},"animations":[]}),
    )
    .unwrap();
    assert!(character.rendering.is_none());
}
