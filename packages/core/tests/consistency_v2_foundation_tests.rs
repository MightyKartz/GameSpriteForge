use std::fs;

use forge_core::catalog::{read_project_catalog, register_catalog_asset, ProjectCatalogEntryV1};
use forge_core::component::{
    FixtureVisionComponent, VisionComponent, VisionComponentRequestV1, VisionInputV1,
    VisionOperation, VISION_COMPONENT_PROTOCOL,
};
use forge_core::workflow_graph::{
    compute_artifact_cache_key, compute_cache_key, ContentCache, WorkflowArtifactV1,
    WorkflowGraphV1, WorkflowNodeV1,
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
fn artifact_cache_key_binds_ordered_paths_as_well_as_hashes() {
    let parameters = serde_json::json!({
        "action": "walk_down",
        "frame": 0,
        "method": "byte_reuse",
    });
    let make_key = |inputs: &[WorkflowArtifactV1]| {
        compute_artifact_cache_key(
            "independent_keyframe",
            "grid-byte-reuse@1.1.0",
            None,
            None,
            &parameters,
            inputs,
        )
        .unwrap()
    };
    let first = WorkflowArtifactV1 {
        sha256: "a".repeat(64),
        path: "source/frame-00.png".into(),
    };
    let second = WorkflowArtifactV1 {
        sha256: "b".repeat(64),
        path: "source/anchor.png".into(),
    };
    let baseline = make_key(&[first.clone(), second.clone()]);
    let mut changed_path = first.clone();
    changed_path.path = "other/frame-00.png".into();
    let mut changed_hash = first.clone();
    changed_hash.sha256 = "c".repeat(64);

    assert_ne!(baseline, make_key(&[changed_path, second.clone()]));
    assert_ne!(baseline, make_key(&[changed_hash, second.clone()]));
    assert_ne!(baseline, make_key(&[second, first]));
}

#[test]
fn byte_reuse_graph_validator_rejects_provenance_tampering() {
    let inputs = vec![
        WorkflowArtifactV1 {
            sha256: "a".repeat(64),
            path: "source/frame-00.png".into(),
        },
        WorkflowArtifactV1 {
            sha256: "b".repeat(64),
            path: "source/anchor.png".into(),
        },
    ];
    let cache_key = compute_artifact_cache_key(
        "independent_keyframe",
        "grid-byte-reuse@1.1.0",
        None,
        None,
        &serde_json::json!({
            "action": "walk_down",
            "frame": 0,
            "method": "byte_reuse",
        }),
        &inputs,
    )
    .unwrap();
    let node = WorkflowNodeV1 {
        id: "independent_keyframe:walk_down:0".into(),
        stage: "independent_keyframe".into(),
        item: Some("walk_down".into()),
        frame: Some(0),
        implementation_version: "grid-byte-reuse@1.1.0".into(),
        depends_on: vec![],
        invalidates: vec![],
        inputs,
        outputs: vec![WorkflowArtifactV1 {
            sha256: "a".repeat(64),
            path: "child/frame-00.png".into(),
        }],
        cache_key,
        provider_request: false,
        cache_hit: true,
        provider_id: None,
        model: None,
    };
    let graph = |node| WorkflowGraphV1 {
        schema_version: "1".into(),
        workflow: "topdown-grid@9.3.0".into(),
        job_id: "child".into(),
        parent_job_id: Some("source".into()),
        nodes: vec![node],
    };
    assert!(graph(node.clone()).validate().is_ok());

    for mutation in [
        "path", "hash", "order", "key", "provider", "cache", "output",
    ] {
        let mut tampered = node.clone();
        match mutation {
            "path" => tampered.inputs[0].path = "tampered/frame.png".into(),
            "hash" => tampered.inputs[0].sha256 = "c".repeat(64),
            "order" => tampered.inputs.swap(0, 1),
            "key" => tampered.cache_key = "0".repeat(64),
            "provider" => tampered.provider_id = Some("fixture".into()),
            "cache" => tampered.cache_hit = false,
            "output" => tampered.outputs[0].sha256 = "d".repeat(64),
            _ => unreachable!(),
        }
        assert!(graph(tampered).validate().is_err(), "{mutation}");
    }
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
