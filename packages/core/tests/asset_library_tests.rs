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
    entry_with_binding(root, None)
}

fn entry_with_binding(
    root: &Path,
    binding: Option<library::finalize::ProjectBinding>,
) -> ProjectCatalogEntryV2 {
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
        "assetProject":binding, "schemaVersion":"1", "kind":"prop_set", "id":"props", "name":"Props", "license":"private", "sampling":"linear", "canvasSize":64,
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
fn portable_legacy_projection_does_not_require_old_machine_install_or_spec_roots() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let mut old = entry(&root);
    old.spec_path = Some(temp.path().join("external-spec.json"));
    old.installed = Some(forge_core::catalog::CatalogInstallRefV1 {
        godot_project: temp.path().join("old-game"),
        target: "addons/forge_assets/props".into(),
        installed_at: chrono::Utc::now(),
    });
    register_catalog_asset_v2(&root, old.clone()).unwrap();
    let preview = library::migration_preview(&root).unwrap();
    library::migrate(&root, "Portable resources", &preview.expected_sha256).unwrap();
    let catalog = library::read_catalog(&root).unwrap();
    let asset = library::read_asset(&root, &catalog, "props").unwrap();
    library::delivery::retain(
        &root,
        &library::delivery::VersionRef {
            asset_id: "props".into(),
            revision: asset.revisions[0].clone(),
        },
    )
    .unwrap();
    fs::remove_dir_all(root.join("jobs")).unwrap();
    fs::remove_file(root.join(".forge/library/local.json")).unwrap();
    let view = library::catalog_view(&root).unwrap();
    assert_eq!(view.assets["props"].pack_sha256, old.pack_sha256);
    assert!(view.assets["props"].pack_path.is_dir());
    assert!(view.assets["props"].spec_path.is_none());
    assert!(view.assets["props"].installed.is_none());
    let catalog = library::read_catalog(&root).unwrap();
    let asset = library::read_asset(&root, &catalog, "props").unwrap();
    assert_eq!(asset.installations.len(), 1);
    assert_eq!(
        asset.installations[0].evidence,
        "legacy_installation_assertion"
    );
    assert!(asset.installations[0].snapshot_sha256.is_none());
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
    let barrier = Arc::new(Barrier::new(8));
    let workers: Vec<_> = (0..8)
        .map(|index| {
            let root = root.to_path_buf();
            let mut new = old.clone();
            let barrier = barrier.clone();
            new.source_job_id = format!("job-{}", index / 2);
            thread::spawn(move || {
                barrier.wait();
                publish_catalog_asset(&root, new.clone()).unwrap();
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

#[test]
fn intake_scan_registration_conflicts_and_unavailable_history() {
    use library::intake::{self, SearchFilter};
    let temp = tempfile::tempdir().unwrap();
    let media = temp.path().join("media");
    let root = temp.path().join("library");
    fs::create_dir_all(media.join(".godot")).unwrap();
    fs::write(media.join(".godot/ignored.bin"), b"cache").unwrap();
    fs::write(media.join("external.bin"), b"external data").unwrap();
    RgbaImage::from_pixel(2, 2, Rgba([1, 2, 3, 255]))
        .save(media.join("art.png"))
        .unwrap();
    let mut wav = b"RIFF".to_vec();
    wav.extend(196_u32.to_le_bytes());
    wav.extend(b"WAVEfmt ");
    wav.extend(16_u32.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(1_u16.to_le_bytes());
    wav.extend(8000_u32.to_le_bytes());
    wav.extend(16000_u32.to_le_bytes());
    wav.extend(2_u16.to_le_bytes());
    wav.extend(16_u16.to_le_bytes());
    wav.extend(b"data");
    wav.extend(160_u32.to_le_bytes());
    wav.extend([0_u8; 160]);
    fs::write(media.join("sound.wav"), wav).unwrap();
    library::initialize(&root, "Local").unwrap();
    let before = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    let mut scan = intake::scan(&media).unwrap();
    assert_eq!(scan.batch.items.len(), 3);
    assert_eq!(
        fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap(),
        before
    );
    for item in &mut scan.batch.items {
        item.tags = vec!["battle".into()];
    }
    let registered = intake::register(&root, &scan.batch).unwrap();
    assert_eq!(registered.len(), 3);
    let head = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    assert!(intake::register(&root, &scan.batch)
        .unwrap()
        .iter()
        .all(|i| i.outcome == "existing"));
    assert_eq!(fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap(), head);
    let filter = SearchFilter {
        kind: Some("audio".into()),
        tag: Some("battle".into()),
        limit: 20,
        ..Default::default()
    };
    assert_eq!(intake::search(&root, &filter).unwrap().total, 1);
    let original = scan
        .batch
        .items
        .iter()
        .find(|i| i.kind == "file")
        .unwrap()
        .clone();
    fs::write(&original.path, b"changed").unwrap();
    assert!(intake::register(&root, &scan.batch).is_err());
    assert_eq!(fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap(), head);
    let mut changed = original.clone();
    changed.expected_content = intake::content_at(&changed.path).unwrap();
    let mut batch = intake::IntakeBatch {
        schema_version: "1".into(),
        items: vec![changed],
    };
    assert!(intake::register(&root, &batch)
        .unwrap_err()
        .to_string()
        .contains("ID conflict"));
    batch.items[0].new_revision = true;
    intake::register(&root, &batch).unwrap();
    assert_eq!(intake::history(&root, &original.asset_id).unwrap().len(), 2);
    fs::remove_file(&original.path).unwrap();
    assert!(intake::history(&root, &original.asset_id)
        .unwrap()
        .iter()
        .all(|i| i.status == "unavailable"));
    assert!(read_project_catalog(&root).unwrap().assets.is_empty()); // legacy shape unchanged
}

#[test]
fn intake_pack_members_duplicate_locations_and_batch_atomicity() {
    use library::intake::{self, IntakeBatch, SearchFilter};
    let temp = tempfile::tempdir().unwrap();
    let produced = entry(&temp.path().join("production"));
    let root = temp.path().join("library");
    library::initialize(&root, "Local").unwrap();
    let scan = intake::scan(&produced.pack_path).unwrap();
    assert_eq!(scan.batch.items.len(), 1);
    intake::register(&root, &scan.batch).unwrap();
    let found = intake::search(
        &root,
        &SearchFilter {
            query: Some("gem".into()),
            limit: 1,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(found.total, 1);
    assert_eq!(found.items[0].members, vec!["gem"]);
    let sources = temp.path().join("sources");
    fs::create_dir_all(sources.join("nested")).unwrap();
    fs::write(sources.join("same.bin"), b"same").unwrap();
    fs::write(sources.join("nested/same.bin"), b"same").unwrap();
    let mut duplicates = intake::scan(&sources).unwrap();
    assert_eq!(duplicates.issues.len(), 2);
    let id = duplicates.batch.items[0].asset_id.clone();
    duplicates.batch.items[1].asset_id = id.clone();
    intake::register(&root, &duplicates.batch).unwrap();
    let catalog = library::read_catalog(&root).unwrap();
    let asset = library::read_asset(&root, &catalog, &id).unwrap();
    assert_eq!(asset.revisions.len(), 1);
    assert_eq!(asset.additional_locations.values().next().unwrap().len(), 1);
    fs::remove_file(&duplicates.batch.items[0].path).unwrap();
    assert_eq!(intake::history(&root, &id).unwrap()[0].status, "available");
    let head = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    let mut valid = duplicates.batch.items[1].clone();
    valid.asset_id = "new-id".into();
    let missing = duplicates.batch.items[0].clone();
    assert!(intake::register(
        &root,
        &IntakeBatch {
            schema_version: "1".into(),
            items: vec![valid, missing]
        }
    )
    .is_err());
    assert_eq!(head, fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap());
}

#[test]
fn bound_local_job_publishes_once_and_pending_output_recovers_without_execution() {
    use library::finalize::{self, ProjectBinding};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    library::initialize(&root, "Local").unwrap();
    let binding = ProjectBinding {
        project_path: root.clone(),
        asset_id: "local-props".into(),
    };
    let produced = entry_with_binding(&temp.path().join("production"), Some(binding.clone()));
    let catalog = library::read_catalog(&root).unwrap();
    let asset = library::read_asset(&root, &catalog, "local-props").unwrap();
    assert_eq!(asset.revisions.len(), 1);
    let revision = library::read_revision(&root, &asset, &asset.revisions[0]).unwrap();
    assert_eq!(revision.source["sourceJobId"], produced.source_job_id);
    assert!(revision.legacy.is_none());
    let head = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    let pending = fs::read_dir(produced.pack_path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.extension().is_some_and(|e| e == "json")
                && p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .contains("publication-")
        })
        .unwrap();
    finalize::recover(&pending).unwrap();
    assert_eq!(head, fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap());
    // A completed Pack can survive registration failure. Recovery consumes only
    // the publication request and Pack, with no Plan, worker or Provider path.
    fs::write(root.join(PROJECT_CATALOG_RELATIVE), b"broken").unwrap();
    let source = BTreeMap::from([
        ("executionId".into(), json!("second-execution")),
        ("method".into(), json!("test_fixture")),
    ]);
    let error = finalize::local_output(&binding, &produced.pack_path, source).unwrap_err();
    assert!(error.to_string().contains("publication pending"));
    fs::write(root.join(PROJECT_CATALOG_RELATIVE), &head).unwrap();
    // The second execution keeps its own durable request; it cannot replace
    // the first execution's recovery evidence.
    let second = fs::read_dir(produced.pack_path.parent().unwrap())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            fs::read(p)
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .is_some_and(|v| v["source"]["executionId"] == "second-execution")
        })
        .unwrap();
    assert_ne!(pending, second);
    finalize::recover(&second).unwrap();
    assert_eq!(
        library::intake::history(&root, "local-props")
            .unwrap()
            .len(),
        2
    );
    fs::write(produced.pack_path.join("changed.bin"), b"tampered").unwrap();
    assert!(finalize::recover(&pending).is_err());
}

#[test]
fn retained_versions_survive_sources_and_consumer_locks_do_not_follow_new_revisions() {
    use library::delivery::{self, VersionRef};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    library::initialize(&root, "Local").unwrap();
    let source = temp.path().join("media");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("tone.wav"), b"synthetic external bytes").unwrap();
    let mut scan = library::intake::scan(&source).unwrap();
    let registered = library::intake::register(&root, &scan.batch).unwrap();
    let reference = VersionRef {
        asset_id: registered[0].asset_id.clone(),
        revision: registered[0].revision.clone(),
    };
    let retained = delivery::retain(&root, &reference).unwrap();
    assert!(retained.retained);
    let head = fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap();
    delivery::retain(&root, &reference).unwrap();
    assert_eq!(head, fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap());
    delivery::select(&root, &reference).unwrap();
    let lock = temp.path().join("consumer/.forge/resources.lock.json");
    delivery::write_lock(&root, &reference, &lock).unwrap();
    let locked_bytes = fs::read(&lock).unwrap();
    fs::write(source.join("tone.wav"), b"new revision").unwrap();
    scan.batch.items[0].expected_content =
        library::intake::content_at(&source.join("tone.wav")).unwrap();
    scan.batch.items[0].new_revision = true;
    library::intake::register(&root, &scan.batch).unwrap();
    fs::remove_dir_all(&source).unwrap();
    assert_eq!(
        delivery::locked_reference(&root, &reference.asset_id, &lock).unwrap(),
        reference
    );
    assert_eq!(fs::read(&lock).unwrap(), locked_bytes);
    let asset = library::read_asset(
        &root,
        &library::read_catalog(&root).unwrap(),
        &reference.asset_id,
    )
    .unwrap();
    assert_eq!(asset.selected_revision.as_ref(), Some(&reference.revision));
    assert_eq!(asset.revisions.len(), 2);
    assert_eq!(
        delivery::resolve(&root, &reference).unwrap().path,
        retained.path
    );
    fs::write(&retained.path, b"tampered").unwrap();
    assert!(delivery::resolve(&root, &reference).is_err());
    assert!(delivery::locked_reference(&root, &reference.asset_id, &lock).is_err());
}

#[test]
fn exact_install_plan_binds_consumer_lock_and_discloses_pack_members() {
    use library::delivery::{self, VersionRef};
    let temp = tempfile::tempdir().unwrap();
    let produced = entry(&temp.path().join("producer"));
    let root = temp.path().join("library");
    library::initialize(&root, "Delivery").unwrap();
    let mut scan = library::intake::scan(&produced.pack_path).unwrap();
    scan.batch.items[0].asset_id = "alias".into();
    let registered = library::intake::register(&root, &scan.batch).unwrap();
    let reference = VersionRef {
        asset_id: "alias".into(),
        revision: registered[0].revision.clone(),
    };
    let retained = delivery::retain(&root, &reference).unwrap();
    let game = temp.path().join("game");
    fs::create_dir(&game).unwrap();
    fs::write(game.join("project.godot"), "config_version=5\n").unwrap();
    let lock = game.join(".forge/resources.lock.json");
    delivery::write_lock(&root, &reference, &lock).unwrap();
    let operation: AutomationOperation = AutomationOperation::InstallGodot(serde_json::from_value(json!({
        "packPath":retained.path,"projectPath":game,"catalogProjectPath":root,"catalogRevision":reference,
        "resourceLockPath":lock,"target":"addons/forge_assets/alias","assetKey":"alias"
    })).unwrap());
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let rendered = serde_json::to_string(&prepared).unwrap();
    assert!(rendered.contains("whole_pack") && rendered.contains("gem"));
    fs::write(&lock, b"changed").unwrap();
    assert!(plans
        .claim(&prepared.token)
        .unwrap_err()
        .to_string()
        .contains("input changed"));
}

#[test]
fn duplicate_intake_restores_missing_bindings_without_changing_shared_history() {
    use forge_core::library::intake;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let sources = temp.path().join("sources");
    fs::create_dir(&sources).unwrap();
    fs::write(sources.join("a.bin"), "A").unwrap();
    fs::write(sources.join("b.bin"), "B").unwrap();
    library::initialize(&root, "Bindings").unwrap();
    let mut batch = intake::scan(&sources).unwrap().batch;
    let registered = intake::register(&root, &batch).unwrap();
    let head = fs::read(root.join(".forge/catalog.json")).unwrap();
    let local_path = root.join(".forge/library/local.json");
    fs::remove_file(&local_path).unwrap();
    let restored = intake::register(&root, &batch).unwrap();
    assert!(restored.iter().all(|r| r.outcome == "existing"));
    assert_eq!(fs::read(root.join(".forge/catalog.json")).unwrap(), head);
    for (before, after) in registered.iter().zip(&restored) {
        assert_eq!(before.revision, after.revision);
        assert_eq!(
            intake::history(&root, &after.asset_id).unwrap()[0].status,
            "available"
        );
    }
    let catalog = library::read_catalog(&root).unwrap();
    let first = library::read_asset(&root, &catalog, &registered[0].asset_id).unwrap();
    let missing = &first.locations[&registered[0].revision].root_id;
    let mut local: serde_json::Value =
        serde_json::from_slice(&fs::read(&local_path).unwrap()).unwrap();
    local["roots"].as_object_mut().unwrap().remove(missing);
    let remaining = local["roots"].clone();
    fs::write(&local_path, serde_json::to_vec(&local).unwrap()).unwrap();
    batch.items.truncate(1);
    let again = intake::register(&root, &batch).unwrap();
    assert_eq!(again[0].revision, registered[0].revision);
    assert_eq!(
        intake::history(&root, &again[0].asset_id).unwrap()[0].status,
        "available"
    );
    let recovered: serde_json::Value =
        serde_json::from_slice(&fs::read(&local_path).unwrap()).unwrap();
    for (key, value) in remaining.as_object().unwrap() {
        assert_eq!(&recovered["roots"][key], value);
    }
    assert_eq!(fs::read(root.join(".forge/catalog.json")).unwrap(), head);
}

#[test]
fn oversized_batch_metadata_does_not_publish_partial_intake() {
    use forge_core::library::intake;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("library");
    let source = temp.path().join("source.bin");
    fs::write(&source, "Original").unwrap();
    library::initialize(&root, "Batch limits").unwrap();
    let mut batch = intake::scan(&source).unwrap().batch;
    let registered = intake::register(&root, &batch).unwrap();
    let head = fs::read(root.join(".forge/catalog.json")).unwrap();
    let local = fs::read(root.join(".forge/library/local.json")).unwrap();
    batch.items[0].asset_id = "new-valid".into();
    let mut oversized = batch.items[0].clone();
    oversized.asset_id = "new-oversized".into();
    oversized.name = "x".repeat(17 * 1024 * 1024);
    batch.items.push(oversized);
    assert!(intake::register(&root, &batch).is_err());
    assert_eq!(fs::read(root.join(".forge/catalog.json")).unwrap(), head);
    assert_eq!(
        fs::read(root.join(".forge/library/local.json")).unwrap(),
        local
    );
    assert_eq!(library::read_catalog(&root).unwrap().assets.len(), 1);
    assert_eq!(
        intake::history(&root, &registered[0].asset_id).unwrap()[0].status,
        "available"
    );
}
