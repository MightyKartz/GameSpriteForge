use std::fs;

use forge_core::catalog::{read_project_catalog, register_catalog_asset, ProjectCatalogEntryV1};
use forge_core::component::{
    FixtureVisionComponent, VisionComponent, VisionComponentRequestV1, VisionInputV1,
    VisionOperation, VISION_COMPONENT_PROTOCOL,
};
use forge_core::workflow_graph::{
    compute_cache_key, ContentCache, WorkflowGraphV1, WorkflowNodeV1,
};
use sha2::{Digest, Sha256};

#[test]
fn workflow_graph_rejects_cycles_and_cache_rejects_corruption() {
    let key = compute_cache_key(
        "frame_image",
        "provider-image-edit@2.0.0",
        Some("fixture"),
        Some("fixture-image"),
        &serde_json::json!({"frame": 0}),
        &["a".repeat(64)],
    )
    .unwrap();
    let node = |id: &str, depends_on: Vec<String>| WorkflowNodeV1 {
        id: id.into(),
        stage: "frame_image".into(),
        item: Some("idle".into()),
        frame: Some(0),
        implementation_version: "provider-image-edit@2.0.0".into(),
        depends_on,
        invalidates: vec![],
        inputs: vec![],
        outputs: vec![],
        cache_key: key.clone(),
        provider_request: true,
        cache_hit: false,
        provider_id: Some("fixture".into()),
        model: Some("fixture-image".into()),
    };
    let graph = WorkflowGraphV1 {
        schema_version: "1".into(),
        workflow: "topdown-keyframes@2.0.0".into(),
        job_id: "job".into(),
        parent_job_id: None,
        nodes: vec![node("a", vec!["b".into()]), node("b", vec!["a".into()])],
    };
    assert!(graph.validate().is_err());

    let temp = tempfile::tempdir().unwrap();
    let cache = ContentCache::new(temp.path().join("cache")).unwrap();
    let source = temp.path().join("source.png");
    fs::write(&source, b"verified bytes").unwrap();
    let output_sha = cache.put_file(&key, &source).unwrap();
    let target = temp.path().join("target.png");
    assert!(cache.materialize_file(&key, &output_sha, &target).unwrap());
    assert_eq!(fs::read(&target).unwrap(), b"verified bytes");
    let object = cache.root().join(&key[..2]).join(&key[2..]);
    fs::write(object, b"corrupt").unwrap();
    assert!(cache.materialize_file(&key, &output_sha, &target).is_err());
}

#[test]
fn project_catalog_and_fixture_component_are_deterministic() {
    let temp = tempfile::tempdir().unwrap();
    register_catalog_asset(
        temp.path(),
        ProjectCatalogEntryV1 {
            asset_id: "hero".into(),
            name: "Hero".into(),
            kind: "character".into(),
            pack_path: temp.path().join("hero.gsfpack"),
            pack_sha256: "1".repeat(64),
            source_job_id: "job-1".into(),
            parent_job_id: None,
            style: None,
            subject: None,
            workflow: "topdown-keyframes@2.0.0".into(),
            provider: None,
            installed: None,
            created_at: chrono::Utc::now(),
        },
    )
    .unwrap();
    assert_eq!(read_project_catalog(temp.path()).unwrap().assets.len(), 1);

    let input = temp.path().join("input.png");
    fs::write(&input, b"fixture vision input").unwrap();
    let sha256 = format!("{:x}", Sha256::digest(fs::read(&input).unwrap()));
    let request = VisionComponentRequestV1 {
        schema_version: "1".into(),
        request_id: "request-1".into(),
        operation: VisionOperation::IdentityEmbedding,
        inputs: vec![VisionInputV1 {
            path: input,
            sha256,
        }],
        parameters: serde_json::Value::Null,
    };
    let response = FixtureVisionComponent.invoke(&request).unwrap();
    assert!(response.ok);
    assert_eq!(response.schema_version, "1");
    let health = FixtureVisionComponent
        .invoke(&VisionComponentRequestV1 {
            schema_version: "1".into(),
            request_id: "health".into(),
            operation: VisionOperation::Health,
            inputs: vec![],
            parameters: serde_json::Value::Null,
        })
        .unwrap();
    assert_eq!(
        health.result.unwrap()["protocol"],
        VISION_COMPONENT_PROTOCOL
    );
}
