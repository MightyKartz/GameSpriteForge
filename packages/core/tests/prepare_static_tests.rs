use std::{fs, path::Path};

use forge_core::{
    asset_project::{hash_file, SamplingMode, StaticAssetKind, StaticCanvasPolicy},
    automation::{
        run_operation, stage_plan_job, AutomationOperation, PlanStore, PrepareStaticItem,
        PrepareStaticRequest,
    },
    job::{JobLifecycleState, JobOperationKind, JobStore},
};
use image::{Rgba, RgbaImage};
use serde_json::Value;

fn request(root: &Path) -> PrepareStaticRequest {
    let path = root.join("jade.png");
    let mut image = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
    for y in 8..56 {
        for x in 20..44 {
            image.put_pixel(x, y, Rgba([0, 200, 80, if x == 20 { 48 } else { 255 }]));
        }
    }
    image.save(&path).unwrap();
    PrepareStaticRequest {
        source_locks: vec![],
        schema_version: "1".into(),
        kind: StaticAssetKind::PropSet,
        id: "jade-props".into(),
        name: "Jade props".into(),
        license: "CC0-1.0".into(),
        sampling: SamplingMode::Linear,
        canvas_size: Some(64),
        canvas_policy: StaticCanvasPolicy::Normalize,
        foreground_alpha_threshold: 1,
        edge_padding_px: 0,
        items: vec![PrepareStaticItem {
            id: "jade".into(),
            name: "Jade".into(),
            path,
        }],
    }
}

#[test]
fn local_static_import_preserves_green_alpha_and_source_provenance() {
    for kind in [StaticAssetKind::PropSet, StaticAssetKind::IconSet] {
        for sampling in [SamplingMode::Nearest, SamplingMode::Linear] {
            let temp = tempfile::tempdir().unwrap();
            let mut request = request(temp.path());
            request.kind = kind;
            request.sampling = sampling.clone();
            let source_hash = hash_file(&request.items[0].path).unwrap();
            let plans = PlanStore::new(temp.path().join("plans")).unwrap();
            let prepared = plans
                .prepare(AutomationOperation::PrepareStatic(request.clone()))
                .unwrap();
            let plan = plans.claim(&prepared.token).unwrap();
            let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
            let queued = stage_plan_job(&jobs, &plan).unwrap();
            assert_eq!(queued.operation_kind, JobOperationKind::PrepareStatic);
            let done = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
            assert_eq!(done.lifecycle_state, JobLifecycleState::Succeeded);
            let pack = &done
                .artifacts
                .iter()
                .find(|a| a.kind == "gsfpack")
                .unwrap()
                .path;
            forge_pack::validate_pack_layout(pack).unwrap();
            let metadata: Value =
                serde_json::from_slice(&fs::read(pack.join("forgepack.json")).unwrap()).unwrap();
            assert_eq!(metadata["assetType"], kind.as_str());
            assert_eq!(metadata["source"]["kind"], "import_frames");
            assert_eq!(metadata["license"]["type"], "CC0-1.0");
            assert_eq!(metadata["items"][0]["provenance"]["sha256"], source_hash);
            assert!(metadata["source"]["metadata"].get("provider").is_none());
            assert_eq!(metadata["source"]["metadata"]["providerRequestCount"], 0);
            assert_eq!(
                metadata["source"]["metadata"]["rendering"]["textureFilter"],
                serde_json::json!(sampling)
            );
            let image = image::open(pack.join("assets/items/jade.png"))
                .unwrap()
                .to_rgba8();
            assert!(
                image.pixels().any(|p| p[1] > 150 && p[3] > 200),
                "green foreground must survive without chroma key"
            );
            assert!(
                image.pixels().any(|p| p[3] > 0 && p[3] < 100),
                "soft alpha must survive"
            );
            assert_eq!(
                hash_file(&done.job_dir.join("source/static/jade.png")).unwrap(),
                source_hash
            );
        }
    }
}

#[test]
fn local_static_plan_rejects_changed_source() {
    let temp = tempfile::tempdir().unwrap();
    let request = request(temp.path());
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareStatic(request.clone()))
        .unwrap();
    let mut image = image::open(&request.items[0].path).unwrap().to_rgba8();
    image.put_pixel(30, 30, Rgba([200, 0, 0, 255]));
    image.save(&request.items[0].path).unwrap();
    assert!(plans
        .claim(&prepared.token)
        .unwrap_err()
        .to_string()
        .contains("input changed"));
}

#[test]
fn local_static_plan_rejects_invalid_inputs_before_creating_jobs() {
    for invalid in [
        "duplicate",
        "unsafe_id",
        "canvas",
        "license",
        "opaque",
        "corrupt",
        "alpha_threshold",
        "edge_padding",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let mut request = request(temp.path());
        match invalid {
            "duplicate" => {
                let mut item = request.items[0].clone();
                item.id = "JADE".into();
                request.items.push(item);
            }
            "unsafe_id" => request.items[0].id = "../jade".into(),
            "canvas" => request.canvas_size = Some(100),
            "license" => request.license = String::new(),
            "opaque" => RgbaImage::from_pixel(16, 16, Rgba([0, 200, 80, 255]))
                .save(&request.items[0].path)
                .unwrap(),
            "corrupt" => fs::write(&request.items[0].path, "not a PNG").unwrap(),
            "alpha_threshold" => request.foreground_alpha_threshold = 0,
            "edge_padding" => request.edge_padding_px = 65,
            _ => unreachable!(),
        }
        let plans = PlanStore::new(temp.path().join("plans")).unwrap();
        assert!(
            plans
                .prepare(AutomationOperation::PrepareStatic(request))
                .is_err(),
            "{invalid} must fail"
        );
    }
}

#[test]
fn preserves_native_rgb_and_rgba_rectangles_bytes_coordinates_and_godot_origin() {
    for rgba in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let mut request = request(temp.path());
        request.canvas_policy = StaticCanvasPolicy::PreserveSource;
        request.canvas_size = None;
        let path = &request.items[0].path;
        if rgba {
            let mut image = RgbaImage::from_pixel(37, 19, Rgba([23, 45, 67, 0]));
            image.put_pixel(31, 2, Rgba([120, 90, 50, 1]));
            image.put_pixel(4, 17, Rgba([10, 200, 90, 255]));
            image.save(path).unwrap();
        } else {
            let mut image = image::RgbImage::from_pixel(37, 19, image::Rgb([220, 210, 200]));
            image.put_pixel(31, 2, image::Rgb([10, 20, 30]));
            image.save(path).unwrap();
        }
        let original = fs::read(path).unwrap();
        let plans = PlanStore::new(temp.path().join("plans")).unwrap();
        let prepared = plans
            .prepare(AutomationOperation::PrepareStatic(request))
            .unwrap();
        let plan = plans.claim(&prepared.token).unwrap();
        let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
        let queued = stage_plan_job(&jobs, &plan).unwrap();
        let done = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
        let pack = &done
            .artifacts
            .iter()
            .find(|a| a.kind == "gsfpack")
            .unwrap()
            .path;
        forge_pack::validate_pack_layout(pack).unwrap();
        assert_eq!(
            fs::read(pack.join("assets/items/jade.png")).unwrap(),
            original
        );
        assert_eq!(
            fs::read(pack.join("assets/frames/frame_001.png")).unwrap(),
            original
        );
        let helper: Value =
            serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json")).unwrap())
                .unwrap();
        assert_eq!(helper["frameWidth"], 37);
        assert_eq!(helper["frameHeight"], 19);
        assert_eq!(
            helper["anchor"],
            serde_json::json!({"type":"custom", "x":0.0,"y":0.0})
        );
        let manifest: Value =
            serde_json::from_slice(&fs::read(pack.join("assets/manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest["sheet"]["frameHeight"], 19);
        let report: Value = serde_json::from_slice(
            &fs::read(done.job_dir.join("local-import-report.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            report["items"][0]["cropBounds"],
            serde_json::json!([0, 0, 37, 19])
        );
        assert_eq!(report["items"][0]["sourceBytesPreserved"], true);
        assert_eq!(report["checks"]["transparentBackground"], rgba);
    }
}

#[test]
fn native_policy_rejects_ambiguous_resizing_and_mixed_canvases() {
    for invalid in ["canvas", "padding", "mixed"] {
        let temp = tempfile::tempdir().unwrap();
        let mut request = request(temp.path());
        request.canvas_policy = StaticCanvasPolicy::PreserveSource;
        request.canvas_size = None;
        match invalid {
            "canvas" => request.canvas_size = Some(64),
            "padding" => request.edge_padding_px = 2,
            "mixed" => {
                let second = temp.path().join("other.png");
                RgbaImage::from_pixel(65, 32, Rgba([2, 3, 4, 255]))
                    .save(&second)
                    .unwrap();
                request.items.push(PrepareStaticItem {
                    id: "other".into(),
                    name: "Other".into(),
                    path: second,
                });
            }
            _ => unreachable!(),
        }
        let plans = PlanStore::new(temp.path().join("plans")).unwrap();
        assert!(
            plans
                .prepare(AutomationOperation::PrepareStatic(request))
                .is_err(),
            "accepted {invalid}"
        );
    }
}

#[test]
fn legacy_request_json_and_serialized_fingerprints_keep_normalize_default() {
    let temp = tempfile::tempdir().unwrap();
    let old = serde_json::to_value(request(temp.path())).unwrap();
    assert_eq!(old["canvasSize"], 64);
    assert!(old.get("canvasPolicy").is_none());
    let parsed: PrepareStaticRequest = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(parsed.canvas_policy, StaticCanvasPolicy::Normalize);
    assert_eq!(serde_json::to_value(parsed).unwrap(), old);
    let mut native = old;
    native.as_object_mut().unwrap().remove("canvasSize");
    native["canvasPolicy"] = serde_json::json!("preserve_source");
    let parsed: PrepareStaticRequest = serde_json::from_value(native).unwrap();
    assert_eq!(parsed.canvas_size, None);
}

#[test]
fn explicit_alpha_bounds_ignore_distant_residue_and_keep_padded_soft_edges() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = request(temp.path());
    request.sampling = SamplingMode::Nearest;
    request.foreground_alpha_threshold = 16;
    request.edge_padding_px = 2;
    let path = &request.items[0].path;
    let mut image = image::open(path).unwrap().to_rgba8();
    image.put_pixel(0, 0, Rgba([200, 0, 0, 2]));
    image.put_pixel(18, 24, Rgba([0, 200, 80, 8]));
    image.save(path).unwrap();
    let source_hash = hash_file(path).unwrap();
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareStatic(request))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let done = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    let report_path = &done
        .artifacts
        .iter()
        .find(|a| a.kind == "quality_report")
        .unwrap()
        .path;
    let report: Value = serde_json::from_slice(&fs::read(report_path).unwrap()).unwrap();
    assert_eq!(
        report["items"][0]["foregroundBounds"],
        serde_json::json!([20, 8, 44, 56])
    );
    assert_eq!(
        report["items"][0]["cropBounds"],
        serde_json::json!([18, 6, 46, 58])
    );
    let output = image::open(done.job_dir.join("normalized/static/jade.png"))
        .unwrap()
        .to_rgba8();
    assert!(
        output.pixels().any(|p| p[3] == 8 && p[1] == 200),
        "padding retains original soft alpha"
    );
    assert!(
        !output.pixels().any(|p| p[0] == 200 && p[3] == 2),
        "distant residue stays outside the crop"
    );
    assert_eq!(
        hash_file(&done.job_dir.join("source/static/jade.png")).unwrap(),
        source_hash
    );
}
