use forge_core::library::{
    self, audit,
    delivery::{self, VersionRef},
    intake::{self, OriginAssertion, SearchFilter},
    review::{self, ReviewRequest},
    transfer,
};
use std::fs;

fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
    intake::IntakeBatch,
    VersionRef,
) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source.bin");
    fs::write(&source, b"Synthetic external resource\0v1").unwrap();
    let root = temp.path().join("library");
    library::initialize(&root, "Portable library").unwrap();
    let batch = intake::scan(&source).unwrap().batch;
    let registered = intake::register(&root, &batch).unwrap();
    let reference = VersionRef {
        asset_id: registered[0].asset_id.clone(),
        revision: registered[0].revision.clone(),
    };
    (temp, root, source, batch, reference)
}

#[test]
fn same_bytes_distinct_origin_and_lineage_keep_separate_versions_and_selection() {
    let (_temp, root, _source, mut batch, first) = fixture();
    batch.items[0].origin = Some(OriginAssertion {
        tool: Some("External editor".into()),
        model: None,
        license: Some("User statement, not approval".into()),
        notes: None,
    });
    assert!(intake::register(&root, &batch).is_err());
    batch.items[0].new_revision = true;
    batch.items[0].parent_revisions = vec![first.revision.clone()];
    batch.items[0].variant = Some("combat".into());
    let second = intake::register(&root, &batch).unwrap();
    assert_ne!(first.revision, second[0].revision);
    assert_eq!(
        intake::register(&root, &batch).unwrap()[0].revision,
        second[0].revision
    );
    review::annotate_metadata(
        &root,
        &first.asset_id,
        None,
        None,
        Some(Some("battle".into())),
    )
    .unwrap();
    delivery::select(&root, &first).unwrap();
    let filter = SearchFilter {
        purpose: Some("battle".into()),
        disposition: Some("selected".into()),
        limit: 20,
        ..Default::default()
    };
    assert_eq!(
        intake::search(&root, &filter).unwrap().items[0].revision,
        first.revision
    );
    review::set_disposition(&root, &first, "discarded").unwrap();
    assert_eq!(intake::search(&root, &filter).unwrap().total, 0);
    let history = intake::history(&root, &first.asset_id).unwrap();
    assert_eq!(history.len(), 2);
    assert!(history.iter().all(|h| h.review_states.is_empty()));
    delivery::select(&root, &first).unwrap();
    assert_eq!(intake::search(&root, &filter).unwrap().total, 1);
}

#[test]
fn selected_media_review_and_consumer_lock_survive_source_removal() {
    let (temp, root, source, _batch, reference) = fixture();
    let evidence = temp.path().join("notes.txt");
    fs::write(&evidence, "Original assertion with <untrusted> text").unwrap();
    review::record(
        &root,
        &ReviewRequest {
            reference: reference.clone(),
            domain: "license".into(),
            verdict: "unknown".into(),
            statement: "Retain original statement".into(),
            reviewer: "Fixture".into(),
            evidence,
        },
    )
    .unwrap();
    let consumer_lock = temp.path().join("consumer.lock.json");
    delivery::write_lock(&root, &reference, &consumer_lock).unwrap();
    let before = fs::read(root.join(".forge/catalog.json")).unwrap();
    let bundle = temp.path().join("transfer with spaces");
    let report = transfer::export(
        &root,
        std::slice::from_ref(&reference),
        &bundle,
        serde_json::json!({"executable":"private/path", "build":{"features":[]}}),
        Some(&consumer_lock),
    )
    .unwrap();
    assert!(!report.trusted_baseline_verified);
    assert_eq!(before, fs::read(root.join(".forge/catalog.json")).unwrap());
    assert!(
        !transfer::verify(&bundle, None)
            .unwrap()
            .trusted_baseline_verified
    );
    assert!(transfer::verify(&bundle, Some(&"0".repeat(64))).is_err());
    assert!(!fs::read_to_string(bundle.join("bundle.json"))
        .unwrap()
        .contains("private/path"));
    fs::remove_file(source).unwrap();
    let moved = temp.path().join("different root");
    transfer::import(&bundle, &moved, &report.manifest_sha256).unwrap();
    assert!(!moved.join(".forge/library/local.json").exists());
    assert_eq!(
        fs::read(consumer_lock).unwrap(),
        fs::read(moved.join(".forge/library/consumer-resources.lock.json")).unwrap()
    );
    assert!(audit::verify(&moved).unwrap().complete_media);
    assert!(delivery::resolve(&root, &reference).is_err());
    assert!(delivery::resolve(&moved, &reference).unwrap().retained);
    assert!(transfer::import(&bundle, &moved, &report.manifest_sha256).is_err());
    fs::write(bundle.join("payload/unlisted.txt"), "extra").unwrap();
    assert!(transfer::verify(&bundle, None).is_err());
}

#[test]
fn transfer_rejects_duplicate_selection_inventory_and_modified_media() {
    let (temp, root, _source, _batch, reference) = fixture();
    assert!(transfer::export(
        &root,
        &[reference.clone(), reference.clone()],
        &temp.path().join("bad"),
        serde_json::json!({}),
        None
    )
    .is_err());
    let bundle = temp.path().join("bundle");
    transfer::export(&root, &[reference], &bundle, serde_json::json!({}), None).unwrap();
    let manifest_path = bundle.join("bundle.json");
    let original = fs::read(&manifest_path).unwrap();
    let mut manifest: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let duplicate = manifest["payload"]["files"][0].clone();
    manifest["payload"]["files"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(transfer::verify(&bundle, None).is_err());
    fs::write(&manifest_path, &original).unwrap();
    let media = manifest["selections"][0]["mediaPath"].as_str().unwrap();
    fs::write(bundle.join("payload").join(media), "tampered").unwrap();
    assert!(transfer::verify(&bundle, None).is_err());
}

#[test]
fn local_rebinding_changes_availability_without_changing_shared_metadata() {
    let (temp, root, source, _batch, reference) = fixture();
    let catalog = library::read_catalog(&root).unwrap();
    let asset = library::read_asset(&root, &catalog, &reference.asset_id).unwrap();
    let root_id = &asset.locations[&reference.revision].root_id;
    let before = fs::read(root.join(".forge/catalog.json")).unwrap();
    let moved = temp.path().join("moved.bin");
    fs::rename(source, &moved).unwrap();
    assert!(!audit::verify(&root).unwrap().complete_media);
    assert!(
        audit::bind_root(&root, root_id, &moved)
            .unwrap()
            .complete_media
    );
    assert_eq!(before, fs::read(root.join(".forge/catalog.json")).unwrap());
    fs::write(&moved, "different bytes").unwrap();
    assert!(
        !audit::bind_root(&root, root_id, &moved)
            .unwrap()
            .complete_media
    );
    assert!(audit::bind_root(&root, "project", &moved).is_err());
    assert!(audit::bind_root(&root, "unknown", &moved).is_err());
}

#[test]
fn derived_index_is_disposable_and_cannot_hide_missing_objects_or_forge_search_results() {
    let (_temp, root, _source, _batch, reference) = fixture();
    let filter = SearchFilter {
        limit: 20,
        ..Default::default()
    };
    let expected = serde_json::to_value(intake::search(&root, &filter).unwrap()).unwrap();
    let rebuilt = library::index::rebuild(&root).unwrap();
    assert_eq!(rebuilt.assets, 1);
    assert_eq!(
        serde_json::to_value(intake::search(&root, &filter).unwrap()).unwrap(),
        expected
    );
    let mut cache: serde_json::Value =
        serde_json::from_slice(&fs::read(&rebuilt.path).unwrap()).unwrap();
    cache["assets"][&reference.asset_id]["name"] = serde_json::json!("forged metadata");
    let forged = serde_json::to_vec(&cache).unwrap();
    fs::write(&rebuilt.path, &forged).unwrap();
    assert_eq!(
        serde_json::to_value(intake::search(&root, &filter).unwrap()).unwrap(),
        expected
    );
    assert_eq!(fs::read(&rebuilt.path).unwrap(), forged);
    fs::remove_file(&rebuilt.path).unwrap();
    assert_eq!(
        serde_json::to_value(intake::search(&root, &filter).unwrap()).unwrap(),
        expected
    );
    library::index::rebuild(&root).unwrap();
    fs::remove_file(root.join(format!(
        ".forge/library/objects/{}.json",
        reference.revision
    )))
    .unwrap();
    assert!(intake::search(&root, &filter).is_err());
}

#[test]
fn git_merge_preserves_parallel_versions_and_reports_selection_conflicts_without_writes() {
    let (temp, root, source, mut batch, first) = fixture();
    let head = root.join(".forge/catalog.json");
    let base = temp.path().join("base.json");
    let ours = temp.path().join("ours.json");
    let theirs = temp.path().join("theirs.json");
    fs::copy(&head, &base).unwrap();
    fs::write(&source, "branch A").unwrap();
    batch.items[0].expected_content = intake::content_at(&source).unwrap();
    batch.items[0].new_revision = true;
    let left = intake::register(&root, &batch).unwrap();
    fs::copy(&head, &ours).unwrap();
    fs::copy(&base, &head).unwrap();
    fs::write(&source, "branch B").unwrap();
    batch.items[0].expected_content = intake::content_at(&source).unwrap();
    let right = intake::register(&root, &batch).unwrap();
    fs::copy(&head, &theirs).unwrap();
    let conflict_markers = b"<<<<<<< ours\nconflicted catalog\n=======\ntheirs\n>>>>>>> theirs";
    fs::write(&head, conflict_markers).unwrap();
    let preview = library::merge::run(&root, &base, &ours, &theirs, None).unwrap();
    assert!(preview.conflicts.is_empty());
    assert!(!preview.applied);
    assert_eq!(fs::read(&head).unwrap(), conflict_markers);
    assert!(library::merge::run(&root, &base, &ours, &theirs, Some(&"0".repeat(64))).is_err());
    let applied =
        library::merge::run(&root, &base, &ours, &theirs, Some(&preview.expected_sha256)).unwrap();
    assert!(applied.applied);
    let history = intake::history(&root, &first.asset_id).unwrap();
    assert_eq!(history.len(), 3);
    assert!(history.iter().any(|v| v.revision == left[0].revision));
    assert!(history.iter().any(|v| v.revision == right[0].revision));
    fs::copy(&head, &base).unwrap();
    delivery::select(
        &root,
        &VersionRef {
            asset_id: first.asset_id.clone(),
            revision: left[0].revision.clone(),
        },
    )
    .unwrap();
    fs::copy(&head, &ours).unwrap();
    fs::copy(&base, &head).unwrap();
    delivery::select(
        &root,
        &VersionRef {
            asset_id: first.asset_id,
            revision: right[0].revision.clone(),
        },
    )
    .unwrap();
    fs::copy(&head, &theirs).unwrap();
    fs::write(&head, conflict_markers).unwrap();
    let preview = library::merge::run(&root, &base, &ours, &theirs, None).unwrap();
    assert_eq!(preview.conflicts.len(), 1);
    assert!(preview.conflicts[0].ends_with("/selectedRevision"));
    assert!(
        !library::merge::run(&root, &base, &ours, &theirs, Some(&preview.expected_sha256))
            .unwrap()
            .applied
    );
    assert_eq!(fs::read(&head).unwrap(), conflict_markers);
}

#[test]
fn concurrent_review_assertions_require_explicit_resolution() {
    let (temp, root, _source, mut batch, reference) = fixture();
    batch.items[0].purpose = Some(" ".into());
    assert!(intake::register(&root, &batch).is_err());
    batch.items[0].purpose = None;
    batch.items[0].parent_revisions = vec![reference.revision.clone(), reference.revision.clone()];
    assert!(intake::register(&root, &batch).is_err());
    let head = root.join(".forge/catalog.json");
    let base = temp.path().join("base.json");
    let ours = temp.path().join("ours.json");
    let theirs = temp.path().join("theirs.json");
    fs::copy(&head, &base).unwrap();
    let evidence = temp.path().join("notes.txt");
    fs::write(&evidence, "Fixture observation").unwrap();
    let mut request = ReviewRequest {
        reference,
        domain: "technical".into(),
        verdict: "approved".into(),
        statement: "Branch A observation".into(),
        reviewer: "Fixture".into(),
        evidence,
    };
    review::record(&root, &request).unwrap();
    fs::copy(&head, &ours).unwrap();
    fs::copy(&base, &head).unwrap();
    request.verdict = "rejected".into();
    request.statement = "Branch B observation".into();
    review::record(&root, &request).unwrap();
    fs::copy(&head, &theirs).unwrap();
    let original = fs::read(&head).unwrap();
    let preview = library::merge::run(&root, &base, &ours, &theirs, None).unwrap();
    assert_eq!(preview.conflicts.len(), 1);
    assert!(preview.conflicts[0].contains("/reviews/"));
    let result =
        library::merge::run(&root, &base, &ours, &theirs, Some(&preview.expected_sha256)).unwrap();
    assert!(!result.applied);
    assert_eq!(original, fs::read(head).unwrap());
}

#[test]
fn merging_an_independent_review_does_not_revoke_a_later_rejection() {
    let (temp, root, _source, _batch, reference) = fixture();
    let head = root.join(".forge/catalog.json");
    let base = temp.path().join("base.json");
    let ours = temp.path().join("ours.json");
    let theirs = temp.path().join("theirs.json");
    fs::copy(&head, &base).unwrap();
    let evidence = temp.path().join("notes.txt");
    fs::write(&evidence, "review evidence").unwrap();
    let mut request = ReviewRequest {
        reference: reference.clone(),
        domain: "visual".into(),
        verdict: "approved".into(),
        statement: "Initial approval".into(),
        reviewer: "Fixture".into(),
        evidence,
    };
    review::record(&root, &request).unwrap();
    request.verdict = "rejected".into();
    request.statement = "Later observation revokes approval".into();
    review::record(&root, &request).unwrap();
    assert_eq!(
        intake::history(&root, &reference.asset_id).unwrap()[0].review_states["visual"],
        "rejected"
    );
    fs::copy(&head, &ours).unwrap();
    fs::copy(&base, &head).unwrap();
    request.domain = "auditory".into();
    request.verdict = "unknown".into();
    review::record(&root, &request).unwrap();
    fs::copy(&head, &theirs).unwrap();
    for (left, right) in [(&ours, &theirs), (&theirs, &ours)] {
        let preview = library::merge::run(&root, &base, left, right, None).unwrap();
        assert!(preview.conflicts.is_empty());
        let applied =
            library::merge::run(&root, &base, left, right, Some(&preview.expected_sha256)).unwrap();
        assert!(applied.applied);
        let hit = &intake::history(&root, &reference.asset_id).unwrap()[0];
        assert_eq!(hit.review_states["visual"], "rejected");
        assert_eq!(hit.review_states["auditory"], "unknown");
        let catalog = library::read_catalog(&root).unwrap();
        let asset = library::read_asset(&root, &catalog, &reference.asset_id).unwrap();
        assert_eq!(
            review::read_reviews(&root, &asset, &reference.revision)
                .unwrap()
                .len(),
            3
        );
    }
}

#[test]
fn evidence_type_and_content_changes_fail_export_and_bundle_validation() {
    for state in [
        "missing",
        "empty_directory",
        "single_file_directory",
        "changed",
    ] {
        let (temp, root, _source, _batch, reference) = fixture();
        let evidence = temp.path().join("notes.txt");
        fs::write(&evidence, "original evidence").unwrap();
        let record = review::record(
            &root,
            &ReviewRequest {
                reference: reference.clone(),
                domain: "technical".into(),
                verdict: "unknown".into(),
                statement: "Original statement".into(),
                reviewer: "Fixture".into(),
                evidence,
            },
        )
        .unwrap();
        let bundle = temp.path().join("bundle");
        transfer::export(
            &root,
            std::slice::from_ref(&reference),
            &bundle,
            serde_json::json!({}),
            None,
        )
        .unwrap();
        for path in [
            root.join(&record.evidence_path),
            bundle.join("payload").join(&record.evidence_path),
        ] {
            fs::remove_file(&path).unwrap();
            match state {
                "missing" => (),
                "empty_directory" => fs::create_dir(&path).unwrap(),
                "single_file_directory" => {
                    fs::create_dir(&path).unwrap();
                    fs::write(path.join("matching.bin"), "original evidence").unwrap();
                }
                "changed" => fs::write(&path, "changed evidence").unwrap(),
                _ => unreachable!(),
            }
        }
        let audit = audit::verify(&root).unwrap();
        assert!(
            !audit.complete_media && !audit.evidence_issues.is_empty(),
            "{state}"
        );
        let rejected = temp.path().join("rejected");
        assert!(
            transfer::export(&root, &[reference], &rejected, serde_json::json!({}), None).is_err(),
            "{state}"
        );
        assert!(!rejected.exists());
        // Even a self-consistent payload manifest cannot turn a directory or
        // changed bytes into the regular evidence file named by its review.
        let manifest_path = bundle.join("bundle.json");
        let mut manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest["payload"] = serde_json::to_value(
            forge_core::content_digest::directory_inventory(&bundle.join("payload")).unwrap(),
        )
        .unwrap();
        fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(transfer::verify(&bundle, None).is_err(), "{state}");
    }
}

#[test]
fn oversized_origin_rejects_a_new_revision_and_preserves_queryable_history() {
    let (_temp, root, _source, mut batch, reference) = fixture();
    let head = fs::read(root.join(".forge/catalog.json")).unwrap();
    batch.items[0].new_revision = true;
    batch.items[0].origin = Some(OriginAssertion {
        tool: None,
        model: None,
        license: None,
        notes: Some("x".repeat(17 * 1024 * 1024)),
    });
    assert!(intake::register(&root, &batch).is_err());
    assert_eq!(fs::read(root.join(".forge/catalog.json")).unwrap(), head);
    let history = intake::history(&root, &reference.asset_id).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].revision, reference.revision);
    assert_eq!(history[0].status, "available");
}
