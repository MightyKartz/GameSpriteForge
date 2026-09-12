use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    sync::{Arc, Barrier},
    thread,
};

use forge_core::{
    automation::{
        run_operation, stage_plan_job, AutomationOperation, PlanStore, PrepareStaticRequest,
    },
    catalog::{
        publish_catalog_asset, read_project_catalog, register_catalog_asset_v2,
        ProjectCatalogEntryV2, PROJECT_CATALOG_RELATIVE,
    },
    delivery::directory_sha256,
    job::JobStore,
    library::{self, AssetRecord},
};
use image::{Rgba, RgbaImage};
use serde_json::json;

fn entry(root: &Path) -> ProjectCatalogEntryV2 {
    fs::create_dir_all(root).unwrap();
    let source = root.join("source.png");
    let mut image = RgbaImage::new(16, 16);
    for y in 3..13 {
        for x in 4..12 {
            image.put_pixel(x, y, Rgba([250, 70, 20, 255]));
        }
    }
    image.save(&source).unwrap();
    let request: PrepareStaticRequest = serde_json::from_value(json!({
        "schemaVersion":"1", "kind":"prop_set", "id":"props", "name":"Props", "license":"private", "sampling":"linear", "canvasSize":64,
        "items":[{"id":"gem", "name":"Gem", "path":source}]
    })).unwrap();
    let plans = PlanStore::new(root.join("plans")).unwrap();
    let plan = plans
        .prepare(AutomationOperation::PrepareStatic(request))
        .unwrap();
    let plan = plans.claim(&plan.token).unwrap();
    let jobs = JobStore::new(root.join("jobs")).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let job = run_operation(&jobs, &job.job_id, &plan.operation).unwrap();
    let pack = job.artifacts.iter().find(|a| a.kind == "gsfpack").unwrap();
    serde_json::from_value(json!({
        "assetId":"props", "name":"Props", "kind":"prop_set", "packPath":pack.path,
        "packSha256":directory_sha256(&pack.path).unwrap(), "sourceJobId":job.job_id,
        "workflow":"static-set@1.0.0", "createdAt":"2026-01-01T00:00:00Z",
        "license":"private", "reviewedAt":"2026-01-02T00:00:00Z", "specSha256":"b".repeat(64)
    }))
    .unwrap()
}

#[test]
fn initialization_and_queries_are_provider_neutral_and_nonoverwriting() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("assets");
    library::initialize(&root, "Local assets").unwrap();
    assert!(!root.join("forge-project.json").exists());
    let before = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    assert!(read_project_catalog(&root).unwrap().assets.is_empty());
    assert!(library::initialize(&root, "Other").is_err());
    assert_eq!(
        before,
        fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap()
    );
    assert!(root.join(".forge/library/.gitignore").is_file());
}

#[test]
fn migration_preserves_original_bytes_and_rejects_stale_preview() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = entry(root);
    register_catalog_asset_v2(root, old.clone()).unwrap();
    let before = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    let preview = library::migration_preview(root).unwrap();
    assert_eq!(
        before,
        fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap()
    );
    assert!(!root.join(".forge/library").exists());
    assert!(preview.assets[0].issues.is_empty());
    assert!(library::migrate(root, "Resources", &"0".repeat(64)).is_err());
    library::migrate(root, "Resources", &preview.expected_sha256).unwrap();
    let backup = root.join(format!(
        ".forge/library/backups/{}.json",
        preview.catalog_sha256
    ));
    assert_eq!(fs::read(backup).unwrap(), before);
    let view = read_project_catalog(root).unwrap();
    assert_eq!(
        fs::canonicalize(&view.assets["props"].pack_path).unwrap(),
        fs::canonicalize(&old.pack_path).unwrap()
    );
    assert_eq!(view.assets["props"].reviewed_at, old.reviewed_at);
    assert_eq!(view.assets["props"].license, old.license);
    assert!(forge_core::catalog::write_project_catalog(root, &view).is_err());
}

#[test]
fn migration_binds_resource_bytes_and_retains_unavailable_legacy_assertions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut old = entry(root);
    register_catalog_asset_v2(root, old.clone()).unwrap();
    let preview = library::migration_preview(root).unwrap();
    fs::write(old.pack_path.join("extra.txt"), b"drift").unwrap();
    assert!(library::migrate(root, "Resources", &preview.expected_sha256).is_err());
    old.pack_path = root.join("missing-pack");
    register_catalog_asset_v2(root, old).unwrap();
    let preview = library::migration_preview(root).unwrap();
    assert!(preview.assets[0].content_sha256.is_none());
    let catalog = library::migrate(root, "Resources", &preview.expected_sha256).unwrap();
    let asset = library::read_asset(root, &catalog, "props").unwrap();
    let revision = library::read_revision(root, &asset, &asset.revisions[0]).unwrap();
    assert!(revision.content.is_none());
    assert_eq!(revision.legacy.unwrap().license.as_deref(), Some("private"));
}

#[test]
fn publication_is_immutable_idempotent_and_does_not_select_new_versions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = entry(root);
    library::initialize(root, "Resources").unwrap();
    publish_catalog_asset(root, old.clone()).unwrap();
    let before = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    let first = library::read_catalog(root).unwrap();
    let first_asset = library::read_asset(root, &first, "props").unwrap();
    let first_revision = first_asset.revisions[0].clone();
    publish_catalog_asset(root, old.clone()).unwrap();
    assert_eq!(
        before,
        fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap()
    );
    let mut new = old;
    new.source_job_id = "new-execution".into();
    new.reviewed_at = None;
    publish_catalog_asset(root, new).unwrap();
    let catalog = library::read_catalog(root).unwrap();
    let asset = library::read_asset(root, &catalog, "props").unwrap();
    assert_eq!(asset.revisions.len(), 2);
    assert!(asset.selected_revision.is_none());
    assert_ne!(
        asset.build_revision.as_deref(),
        Some(first_revision.as_str())
    );
    assert!(library::read_revision(root, &asset, &first_revision)
        .unwrap()
        .legacy
        .unwrap()
        .reviewed_at
        .is_some());
    assert!(read_project_catalog(root).unwrap().assets["props"]
        .reviewed_at
        .is_none());
    assert_eq!(
        library::read_asset(root, &first, "props")
            .unwrap()
            .revisions
            .len(),
        1
    );
}

#[test]
fn immutable_objects_and_foreign_head_overwrites_are_detected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = entry(root);
    library::initialize(root, "Resources").unwrap();
    publish_catalog_asset(root, old).unwrap();
    let catalog = library::read_catalog(root).unwrap();
    let object = root.join(format!(
        ".forge/library/objects/{}.json",
        catalog.assets["props"]
    ));
    fs::write(&object, b"{}").unwrap();
    assert!(library::read_object::<AssetRecord>(root, &catalog.assets["props"]).is_err());
    assert!(library::snapshot_sha256(root).is_err());
    fs::write(
        root.join(PROJECT_CATALOG_RELATIVE),
        br#"{"schemaVersion":"2","updatedAt":"2026-01-01T00:00:00Z","assets":{}}"#,
    )
    .unwrap();
    assert!(read_project_catalog(root).is_err());
}

#[test]
fn library_install_references_reject_changed_pack_bytes_before_linking() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = entry(root);
    library::initialize(root, "Resources").unwrap();
    publish_catalog_asset(root, old.clone()).unwrap();
    library::validate_current_pack(root, &old.pack_path).unwrap();
    let before = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    fs::write(old.pack_path.join("added.txt"), b"changed inventory").unwrap();
    assert!(library::validate_current_pack(root, &old.pack_path).is_err());
    assert!(forge_core::catalog::link_catalog_install(
        root,
        "props",
        root.join("game"),
        "addons/props".into()
    )
    .is_err());
    assert_eq!(
        fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap(),
        before
    );
}

#[test]
fn concurrent_publication_and_head_restore_keep_complete_histories() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = entry(root);
    library::initialize(root, "Resources").unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|index| {
            let root = root.to_path_buf();
            let mut new = old.clone();
            let barrier = barrier.clone();
            new.source_job_id = format!("job-{index}");
            thread::spawn(move || {
                barrier.wait();
                publish_catalog_asset(&root, new).unwrap();
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let catalog = library::read_catalog(root).unwrap();
    assert_eq!(
        library::read_asset(root, &catalog, "props")
            .unwrap()
            .revisions
            .len(),
        4
    );
    let before = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    forge_core::catalog::link_catalog_install(
        root,
        "props",
        root.join("game"),
        "addons/props".into(),
    )
    .unwrap();
    assert_ne!(
        before,
        fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap()
    );
    fs::write(root.join(PROJECT_CATALOG_RELATIVE), &before).unwrap();
    let catalog = library::read_catalog(root).unwrap();
    assert!(library::read_asset(root, &catalog, "props")
        .unwrap()
        .installations
        .is_empty());
    library::snapshot_sha256(root).unwrap();
}

#[test]
fn v1_migration_keeps_unknown_fields_in_backup_without_inventing_history() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join(".forge")).unwrap();
    let bytes = serde_json::to_vec(&json!({"schemaVersion":"1","updatedAt":"2026-01-01T00:00:00Z","assets":BTreeMap::<String, String>::new(),"historicalNote":"retain me"})).unwrap();
    fs::write(root.join(PROJECT_CATALOG_RELATIVE), &bytes).unwrap();
    let preview = library::migration_preview(root).unwrap();
    let catalog = library::migrate(root, "Old project", &preview.expected_sha256).unwrap();
    assert!(catalog.assets.is_empty());
    assert_eq!(
        fs::read(root.join(format!(
            ".forge/library/backups/{}.json",
            preview.catalog_sha256
        )))
        .unwrap(),
        bytes
    );
}

#[test]
fn interrupted_migration_repeats_with_original_identity_and_input_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let old = entry(root);
    register_catalog_asset_v2(root, old).unwrap();
    let preview = library::migration_preview(root).unwrap();
    fs::create_dir_all(root.join(".forge/library/objects")).unwrap();
    fs::write(root.join(".forge/library/identity.json"), serde_json::to_vec(&json!({
        "projectId":"interrupted-project", "name":"Resources", "migratedFromSha256":preview.catalog_sha256
    })).unwrap()).unwrap();
    assert!(read_project_catalog(root).is_err());
    let catalog = library::migrate(root, "Resources", &preview.expected_sha256).unwrap();
    assert_eq!(catalog.project_id, "interrupted-project");
    assert_eq!(
        library::read_asset(root, &catalog, "props")
            .unwrap()
            .revisions
            .len(),
        1
    );
}

#[cfg(unix)]
#[test]
fn library_rejects_redirected_storage_and_lock_files() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    symlink(&outside, root.join(".forge")).unwrap();
    assert!(library::initialize(&root, "Resources").is_err());
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
    fs::remove_file(root.join(".forge")).unwrap();
    fs::create_dir(root.join(".forge")).unwrap();
    symlink(outside.join("lock"), root.join(".forge/catalog.lock")).unwrap();
    assert!(library::initialize(&root, "Resources").is_err());
    assert!(!outside.join("lock").exists());
}
