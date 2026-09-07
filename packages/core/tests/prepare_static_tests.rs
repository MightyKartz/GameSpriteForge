use std::{fs, path::Path};

use forge_core::{
    asset_project::{hash_file, SamplingMode, StaticAssetKind},
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
        schema_version: "1".into(),
        kind: StaticAssetKind::PropSet,
        id: "jade-props".into(),
        name: "Jade props".into(),
        license: "CC0-1.0".into(),
        sampling: SamplingMode::Linear,
        canvas_size: 64,
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
            "canvas" => request.canvas_size = 100,
            "license" => request.license = String::new(),
            "opaque" => RgbaImage::from_pixel(16, 16, Rgba([0, 200, 80, 255]))
                .save(&request.items[0].path)
                .unwrap(),
            "corrupt" => fs::write(&request.items[0].path, "not a PNG").unwrap(),
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
