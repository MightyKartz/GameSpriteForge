use std::fs;
use std::path::{Path, PathBuf};

use forge_core::asset_project::{
    hash_file, init_project, read_project, ForgeProjectV1, SamplingMode, StaticAssetKind,
    StyleSpecV1, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::{
    run_operation, run_operation_with_provider, stage_plan_job, AutomationOperation,
    CreateCollectionLockRequest, CreateStyleLockRequest, CreateSubjectLockRequest,
    GenerateStaticAssetSetRequest, GodotInstallRequest, PlanStore,
};
use forge_core::catalog::{read_project_catalog, CatalogReviewStatusV1};
use forge_core::collection::{
    collection_lock_path, read_static_collection_spec, CollectionRevisionRefV1,
};
use forge_core::job::{JobLifecycleState, JobRecord, JobState, JobStore};
use forge_core::portrait::{
    approve_portrait_base, PortraitGenerationPhaseV1, PortraitNeutralReferencePolicyV1,
    PORTRAIT_BASE_APPROVAL_FILE, PORTRAIT_BASE_LOCK_FILE,
};
use forge_core::project_audit::{audit_project, ProjectAuditScopeV1};
use forge_core::provider::MediaGenerationProvider;
use forge_core::subject::{subject_lock_path, SubjectSpecV1};
use forge_providers::fixture::FixtureProvider;
use image::{ImageBuffer, Rgba};

struct Context {
    root: PathBuf,
    plans: PlanStore,
    jobs: JobStore,
    provider: FixtureProvider,
    style_lock: PathBuf,
}

#[test]
fn fixture_delivers_stage3_static_types_and_zero_cost_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let context = setup(temp.path());
    let subject_spec_path = context.root.join("specs/subject.json");
    fs::write(
        &subject_spec_path,
        serde_json::to_vec_pretty(&SubjectSpecV1 {
            schema_version: "1".into(),
            id: "hero".into(),
            name: "Hero".into(),
            prompt: "a compact purple ranger".into(),
            reference_images: vec![],
            image_model: None,
            license: "MIT".into(),
        })
        .unwrap(),
    )
    .unwrap();
    let subject_job = run(
        &context,
        AutomationOperation::CreateSubjectLock(CreateSubjectLockRequest {
            schema_version: "1".into(),
            project_path: context.root.clone(),
            spec_path: subject_spec_path,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            canonical_import_path: None,
            import_approval_note: None,
        }),
        true,
    );
    assert_eq!(subject_job.lifecycle_state, JobLifecycleState::Succeeded);
    let subject_revision = subject_job
        .asset_id
        .as_deref()
        .unwrap()
        .split_once('@')
        .unwrap()
        .1
        .to_string();

    let icon_collection = create_collection(&context, StaticAssetKind::IconSet, "inventory", 128);
    let portrait_collection =
        create_collection(&context, StaticAssetKind::PortraitSet, "portraits", 128);
    let equipment_collection =
        create_collection(&context, StaticAssetKind::EquipmentSet, "equipment", 256);
    let decal_collection = create_collection(&context, StaticAssetKind::DecalSet, "decals", 256);

    let icon_spec = context.root.join("specs/icons-v2.json");
    fs::write(
        &icon_spec,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "2", "kind": "icon_set", "id": "inventory-icons-v2",
            "name": "Inventory Icons V2", "collection": icon_collection,
            "items": [
                {"id":"potion", "name":"Potion", "prompt":"a purple potion"},
                {"id":"key", "name":"Key", "prompt":"a purple key"},
                {"id":"orb", "name":"Orb", "prompt":"[fixture:outlier_then_success] a purple orb"}
            ],
            "license": "MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let icons = generate(&context, &icon_spec, StaticAssetKind::IconSet, None);

    let portrait_spec = context.root.join("specs/portraits.json");
    fs::write(
        &portrait_spec,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "2", "kind": "portrait_set", "id": "hero-portraits",
            "name": "Hero Portraits", "collection": portrait_collection,
            "subject": {"id":"hero", "revision": subject_revision},
            "framingProfile": "dialogue_bust@1.0.0",
            "expressions": [
                {"id":"neutral", "name":"Neutral", "prompt":"neutral expression"},
                {"id":"happy", "name":"Happy", "prompt":"happy expression"},
                {"id":"angry", "name":"Angry", "prompt":"angry expression"},
                {"id":"hurt", "name":"Hurt", "prompt":"hurt expression"},
                {"id":"surprised", "name":"Surprised", "prompt":"surprised expression"}
            ],
            "license": "MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let portraits = generate(
        &context,
        &portrait_spec,
        StaticAssetKind::PortraitSet,
        Some(subject_lock_path(&context.root, "hero", &subject_revision)),
    );

    let equipment_spec = context.root.join("specs/equipment.json");
    fs::write(
        &equipment_spec,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1", "kind": "equipment_set", "id": "ranger-equipment",
            "name": "Ranger Equipment", "collection": equipment_collection,
            "items": [{"id":"bow", "name":"Bow", "prompt":"a purple ranger bow", "equippedPreview":true}],
            "license": "MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let equipment = generate(
        &context,
        &equipment_spec,
        StaticAssetKind::EquipmentSet,
        None,
    );

    let decal_spec = context.root.join("specs/decals.json");
    fs::write(
        &decal_spec,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1", "kind": "decal_set", "id": "forest-decals",
            "name": "Forest Decals", "collection": decal_collection,
            "items": [{"id":"leaves", "name":"Leaves", "prompt":"a purple leaf scatter", "footprint":{"width":2,"height":1}, "blend":"mix"}],
            "license": "MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let decals = generate(&context, &decal_spec, StaticAssetKind::DecalSet, None);

    for (record, kind, count) in [
        (&icons, "icon_set", 3usize),
        (&portraits, "portrait_set", 5),
        (&equipment, "equipment_set", 3),
        (&decals, "decal_set", 1),
    ] {
        assert_eq!(
            record.lifecycle_state,
            JobLifecycleState::Succeeded,
            "{kind} failed: {}\nportrait-local: {}",
            fs::read_to_string(record.job_dir.join("consistency-report.json"))
                .unwrap_or_else(|_| "missing consistency report".into()),
            fs::read_to_string(record.job_dir.join("portrait-consistency-report.json"))
                .unwrap_or_else(|_| "missing portrait consistency report".into())
        );
        let pack = pack_path(record);
        forge_pack::validate_pack_layout(&pack).unwrap();
        let inspected = forge_pack::inspect_pack(&pack).unwrap();
        assert_eq!(inspected.asset_type, kind);
        assert_eq!(inspected.items.len(), count);
        if kind == "portrait_set" {
            assert!(pack.join("portrait-base-lock.json").is_file());
            assert!(pack.join("portrait-consistency-report.json").is_file());
            assert!(!pack.join("collection-consistency-report.json").exists());
        } else {
            assert!(pack.join("collection-consistency-report.json").is_file());
        }
        assert!(pack.join("collection-lock-ref.json").is_file());
        let lock_ref = fs::read_to_string(pack.join("collection-lock-ref.json")).unwrap();
        assert!(!lock_ref.contains(context.root.to_string_lossy().as_ref()));
    }
    assert!(icons
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "provider_item_orb_attempt_2"));
    let portrait_pack = pack_path(&portraits);
    let portrait_provenance: serde_json::Value =
        serde_json::from_slice(&fs::read(portrait_pack.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(
        portrait_provenance["source"]["metadata"]["assetGeometryProfile"],
        "dialogue_bust@1.0.0"
    );
    assert_eq!(
        portrait_provenance["source"]["metadata"]["consistencyReportSha256"],
        hash_file(&portrait_pack.join("consistency-report.json")).unwrap()
    );
    assert_eq!(
        portrait_provenance["source"]["metadata"]["portraitBaseProfile"],
        "portrait-base-lock@1.0.0"
    );
    assert_eq!(
        portrait_provenance["source"]["metadata"]["portraitLocalProfile"],
        "portrait-local@1.1.0"
    );
    let packed_lock: serde_json::Value =
        serde_json::from_slice(&fs::read(portrait_pack.join("portrait-base-lock.json")).unwrap())
            .unwrap();
    assert!(packed_lock.get("imagePath").is_none());
    assert!(packed_lock.get("providerEditSourcePath").is_none());
    let job_lock: serde_json::Value = serde_json::from_slice(
        &fs::read(
            portraits
                .job_dir
                .join("portrait-base/portrait-base-lock.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let observations = context.provider.edit_observations();
    let neutral = observations
        .iter()
        .find(|observation| observation.authorization_target.as_deref() == Some("neutral"))
        .unwrap();
    assert_eq!(
        neutral.reference_roles,
        vec![
            forge_core::provider::ReferenceRole::Style,
            forge_core::provider::ReferenceRole::EditTarget,
            forge_core::provider::ReferenceRole::SubjectIdentity,
        ]
    );
    for expression in ["happy", "angry", "hurt", "surprised"] {
        let observation = observations
            .iter()
            .find(|observation| observation.authorization_target.as_deref() == Some(expression))
            .unwrap();
        assert_eq!(
            observation.reference_roles,
            vec![forge_core::provider::ReferenceRole::EditTarget]
        );
        assert_eq!(
            observation.reference_sha256,
            vec![job_lock["providerEditSourceSha256"]
                .as_str()
                .unwrap()
                .to_string()]
        );
    }

    if locate_godot().is_some() {
        let godot = temp.path().join("godot-project");
        fs::create_dir(&godot).unwrap();
        fs::write(
            godot.join("project.godot"),
            "[application]\nconfig/name=\"Forge Stage 3 Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        for record in [&icons, &portraits, &equipment, &decals] {
            let target = PathBuf::from(format!(
                "addons/forge_assets/{}",
                record.asset_id.as_deref().unwrap()
            ));
            let prepared = context
                .plans
                .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
                    schema_version: "1".into(),
                    pack_path: pack_path(record),
                    project_path: godot.clone(),
                    catalog_project_path: Some(context.root.clone()),
                    target: target.clone(),
                    asset_key: record.asset_id.clone(),
                    provider_refs: vec![],
                }))
                .unwrap();
            let claimed = context.plans.claim(&prepared.token).unwrap();
            let job = stage_plan_job(&context.jobs, &claimed).unwrap();
            let installed = run_operation(&context.jobs, &job.job_id, &claimed.operation).unwrap();
            assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
            let installed_root = godot.join(&target);
            assert!(installed_root.join("forge_usage.json").is_file());
            if record.asset_id.as_deref() == Some("hero-portraits") {
                let usage: serde_json::Value = serde_json::from_slice(
                    &fs::read(installed_root.join("forge_usage.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(usage["assetGeometry"]["profile"], "dialogue_bust@1.0.0");
                assert_eq!(
                    usage["portraitConsistency"]["baseProfile"],
                    "portrait-base-lock@1.0.0"
                );
                assert_eq!(
                    usage["portraitConsistency"]["localProfile"],
                    "portrait-local@1.1.0"
                );
            }
            assert_no_embedded_godot_images(&installed_root);
        }
        assert!(godot
            .join("addons/forge_assets/ranger-equipment/scenes/bow__world.tscn")
            .is_file());
        assert!(godot
            .join("addons/forge_assets/forest-decals/scenes/leaves.tscn")
            .is_file());
    }

    let usage_before = context.provider.usage();
    let invalid_replacement = context.root.join("specs/invalid-potion.png");
    let mut invalid = ImageBuffer::from_pixel(128, 128, Rgba([0u8, 0, 0, 0]));
    for y in 28..100 {
        for x in 42..86 {
            invalid.put_pixel(x, y, Rgba([20, 240, 30, 255]));
        }
    }
    invalid.save(&invalid_replacement).unwrap();
    let mut quarantine_request = recipe(&icons);
    quarantine_request.reuse_from_job_dir = Some(icons.job_dir.clone());
    quarantine_request.retry_item_ids = vec!["potion".into()];
    quarantine_request.replacement_item_paths = [("potion".into(), invalid_replacement)]
        .into_iter()
        .collect();
    quarantine_request.consistency_recheck_only = true;
    let prepared = context
        .plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(
            quarantine_request,
        ))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&context.jobs, &claimed).unwrap();
    let quarantined =
        run_operation_with_provider(&context.jobs, &job.job_id, &claimed.operation, None).unwrap();
    assert_eq!(
        quarantined.lifecycle_state,
        JobLifecycleState::AwaitingReview
    );
    assert!(quarantined
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "project_catalog_quarantine"));
    let catalog = read_project_catalog(&context.root).unwrap();
    let quarantined_entry = &catalog.assets["inventory-icons-v2"];
    assert_eq!(quarantined_entry.game_ready, Some(false));
    assert_eq!(
        quarantined_entry.quality_verdict.as_deref(),
        Some("consistency_recheck_failed")
    );
    assert_eq!(
        quarantined_entry
            .review
            .as_ref()
            .map(|review| review.status),
        Some(CatalogReviewStatusV1::Quarantined)
    );
    assert_eq!(context.provider.usage(), usage_before);
    let plan_only_godot = temp.path().join("quarantine-plan-godot");
    fs::create_dir(&plan_only_godot).unwrap();
    fs::write(plan_only_godot.join("project.godot"), "[application]\n").unwrap();
    let blocked_install = context
        .plans
        .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack_path(&icons),
            project_path: plan_only_godot,
            catalog_project_path: Some(context.root.clone()),
            target: PathBuf::from("addons/forge_assets/inventory-icons-v2"),
            asset_key: Some("inventory-icons-v2".into()),
            provider_refs: vec![],
        }))
        .unwrap_err();
    assert!(blocked_install.to_string().contains("quarantined"));

    let replacement = icons.job_dir.join("normalized/static/potion.png");
    let mut request = recipe(&icons);
    request.reuse_from_job_dir = Some(icons.job_dir.clone());
    request.retry_item_ids = vec!["potion".into()];
    request.replacement_item_paths = [("potion".into(), replacement)].into_iter().collect();
    request.consistency_recheck_only = true;
    let prepared = context
        .plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(request))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&context.jobs, &claimed).unwrap();
    let replaced =
        run_operation_with_provider(&context.jobs, &job.job_id, &claimed.operation, None).unwrap();
    assert_eq!(replaced.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(
        replaced.parent_job_id.as_deref(),
        Some(icons.job_id.as_str())
    );
    assert_eq!(context.provider.usage(), usage_before);
    assert!(replaced
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "replacement_item_potion"));
    let replaced_pack = pack_path(&replaced);
    let replaced_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(replaced_pack.join("assets/manifest.json")).unwrap())
            .unwrap();
    assert_eq!(
        replaced_manifest["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["id"] == "potion")
            .unwrap()["provenance"]["providerId"],
        "manual_replacement"
    );

    if locate_godot().is_some() {
        let godot = temp.path().join("godot-project");
        let stale = audit_project(&context.root, ProjectAuditScopeV1::All).unwrap();
        assert!(stale
            .findings
            .iter()
            .any(|finding| finding.code == "orphan_godot_asset"));
        let target = PathBuf::from("addons/forge_assets/inventory-icons-v2");
        let prepared = context
            .plans
            .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
                schema_version: "1".into(),
                pack_path: replaced_pack,
                project_path: godot,
                catalog_project_path: Some(context.root.clone()),
                target,
                asset_key: Some("inventory-icons-v2".into()),
                provider_refs: vec![],
            }))
            .unwrap();
        let claimed = context.plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&context.jobs, &claimed).unwrap();
        let installed = run_operation(&context.jobs, &job.job_id, &claimed.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
    }

    let audit = audit_project(&context.root, ProjectAuditScopeV1::All).unwrap();
    assert_eq!(audit.summary.error_count, 0, "{:#?}", audit.findings);
}

#[test]
fn full_body_portrait_contract_blocks_a_bust_before_pack_export() {
    let temp = tempfile::tempdir().unwrap();
    let context = setup(temp.path());
    let subject_spec_path = context.root.join("specs/full-body-subject.json");
    fs::write(
        &subject_spec_path,
        serde_json::to_vec_pretty(&SubjectSpecV1 {
            schema_version: "1".into(),
            id: "full-body-hero".into(),
            name: "Full Body Hero".into(),
            prompt: "a compact purple ranger".into(),
            reference_images: vec![],
            image_model: None,
            license: "MIT".into(),
        })
        .unwrap(),
    )
    .unwrap();
    let subject_job = run(
        &context,
        AutomationOperation::CreateSubjectLock(CreateSubjectLockRequest {
            schema_version: "1".into(),
            project_path: context.root.clone(),
            spec_path: subject_spec_path,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            canonical_import_path: None,
            import_approval_note: None,
        }),
        true,
    );
    let subject_revision = subject_job
        .asset_id
        .as_deref()
        .unwrap()
        .split_once('@')
        .unwrap()
        .1
        .to_string();

    let anchor_path = context.root.join("specs/full-body-anchor.png");
    let mut anchor = ImageBuffer::from_pixel(128, 128, Rgba([0u8, 0, 0, 0]));
    for (left, right, top, bottom) in [
        (48, 80, 8, 32),
        (40, 88, 32, 78),
        (42, 58, 78, 116),
        (70, 86, 78, 116),
    ] {
        for y in top..bottom {
            for x in left..right {
                anchor.put_pixel(x, y, Rgba([130, 60, 210, 255]));
            }
        }
    }
    anchor.save(&anchor_path).unwrap();
    let collection_spec = context.root.join("specs/full-body-collection.json");
    fs::write(
        &collection_spec,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion":"1",
            "id":"full-body-portraits",
            "name":"Full Body Portraits",
            "assetKind":"portrait_set",
            "prompt":"complete full-body portraits",
            "materials":[],
            "scale":"consistent",
            "perspective":"inherit_style",
            "grounding":"feet",
            "framingProfile":"full_body@1.0.0",
            "canvasSize":128,
            "anchorImage":anchor_path,
            "license":"MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let collection_job = run(
        &context,
        AutomationOperation::CreateCollectionLock(CreateCollectionLockRequest {
            schema_version: "1".into(),
            project_path: context.root.clone(),
            spec_path: collection_spec,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
        }),
        false,
    );
    assert_eq!(collection_job.lifecycle_state, JobLifecycleState::Succeeded);
    let collection_revision = collection_job
        .asset_id
        .as_deref()
        .unwrap()
        .split_once('@')
        .unwrap()
        .1;

    let portrait_spec = context.root.join("specs/full-body-portraits.json");
    fs::write(
        &portrait_spec,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion":"2",
            "kind":"portrait_set",
            "id":"hero-full-body-portraits",
            "name":"Hero Full Body Portraits",
            "collection":{"id":"full-body-portraits","revision":collection_revision},
            "subject":{"id":"full-body-hero","revision":subject_revision},
            "framingProfile":"full_body@1.0.0",
            "expressions":[
                {"id":"neutral","name":"Neutral","prompt":"neutral expression"},
                {"id":"happy","name":"Happy","prompt":"[fixture:bust] happy expression"},
                {"id":"angry","name":"Angry","prompt":"angry expression"},
                {"id":"hurt","name":"Hurt","prompt":"hurt expression"},
                {"id":"surprised","name":"Surprised","prompt":"surprised expression"}
            ],
            "license":"MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let record = generate(
        &context,
        &portrait_spec,
        StaticAssetKind::PortraitSet,
        Some(subject_lock_path(
            &context.root,
            "full-body-hero",
            &subject_revision,
        )),
    );
    assert_eq!(record.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert!(!record
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(record.job_dir.join("consistency-report.json")).unwrap())
            .unwrap();
    let happy = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["id"] == "happy")
        .unwrap();
    assert_eq!(happy["verdict"], "blocked");
    assert_eq!(happy["geometry"]["geometryProfile"], "full_body@1.0.0");
    assert!(happy["reasons"]
        .as_array()
        .unwrap()
        .iter()
        .any(|reason| reason == "full_body_lower_body_missing"));
}

#[test]
fn static_transport_failure_resumes_only_incomplete_items_in_a_child_job() {
    let temp = tempfile::tempdir().unwrap();
    let mut context = setup(temp.path());
    context.provider = FixtureProvider::default().with_edit_failure_once("[fixture:fail-once]");
    let collection = create_collection(&context, StaticAssetKind::PropSet, "resume-props", 256);
    let spec_path = context.root.join("specs/resume-props.json");
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion":"2", "kind":"prop_set", "id":"resume-props",
            "name":"Resume Props", "collection":collection,
            "items":[
                {"id":"first", "name":"First", "prompt":"first purple prop"},
                {"id":"second", "name":"Second", "prompt":"[fixture:fail-once] second purple prop"},
                {"id":"third", "name":"Third", "prompt":"third purple prop"}
            ],
            "license":"MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let resolved = read_static_collection_spec(&spec_path, StaticAssetKind::PropSet).unwrap();
    let request = GenerateStaticAssetSetRequest {
        schema_version: "4".into(),
        project_path: context.root.clone(),
        style_lock_path: context.style_lock.clone(),
        collection_lock_path: Some(collection_lock_path(
            &context.root,
            &resolved.collection.id,
            &resolved.collection.revision,
        )),
        subject_lock_path: None,
        source_spec_path: Some(spec_path),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        framing_profile: resolved.framing_profile,
        asset: resolved.asset,
        item_metadata: resolved.item_metadata,
        max_attempts_per_item: 2,
        image_model: None,
        reuse_from_job_dir: None,
        retry_item_ids: vec![],
        replacement_item_paths: Default::default(),
        consistency_recheck_only: false,
        resume_incomplete_static: false,
        portrait_phase: Default::default(),
        neutral_reference_policy: Default::default(),
        portrait_base_parent_job_id: None,
    };
    let prepared = context
        .plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(request.clone()))
        .unwrap();
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let parent = stage_plan_job(&context.jobs, &claimed).unwrap();
    let error = run_operation_with_provider(
        &context.jobs,
        &parent.job_id,
        &claimed.operation,
        Some(&context.provider),
    )
    .unwrap_err();
    assert_eq!(error.code(), "provider_request_failed");
    let parent = context.jobs.read_record(&parent.job_id).unwrap();
    assert_eq!(parent.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(context.provider.usage().requests, 2);
    assert!(parent.job_dir.join("normalized/static/first.png").is_file());
    assert!(!parent.job_dir.join("consistency-report.json").is_file());

    let mut resume = request;
    resume.reuse_from_job_dir = Some(parent.job_dir.clone());
    resume.retry_item_ids = vec!["second".into(), "third".into()];
    resume.resume_incomplete_static = true;
    let prepared = context
        .plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(resume))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 2);
    assert_eq!(prepared.estimate.maximum_provider_requests, 4);
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let child = stage_plan_job(&context.jobs, &claimed).unwrap();
    let child = run_operation_with_provider(
        &context.jobs,
        &child.job_id,
        &claimed.operation,
        Some(&context.provider),
    )
    .unwrap();
    assert_eq!(child.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(child.parent_job_id.as_deref(), Some(parent.job_id.as_str()));
    assert_eq!(context.provider.usage().requests, 4);
    assert!(child
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "reused_item_first"));
    assert_eq!(
        context
            .jobs
            .read_record(&parent.job_id)
            .unwrap()
            .lifecycle_state,
        JobLifecycleState::Failed
    );
}

#[test]
fn portrait_v2_two_phase_approval_locks_policy_budget_and_expression_parent() {
    let temp = tempfile::tempdir().unwrap();
    let context = setup(temp.path());
    let subject_spec_path = context.root.join("specs/two-phase-subject.json");
    fs::write(
        &subject_spec_path,
        serde_json::to_vec_pretty(&SubjectSpecV1 {
            schema_version: "1".into(),
            id: "two-phase-hero".into(),
            name: "Two Phase Hero".into(),
            prompt: "a compact purple ranger canonical identity image".into(),
            reference_images: vec![],
            image_model: None,
            license: "MIT".into(),
        })
        .unwrap(),
    )
    .unwrap();
    let subject_job = run(
        &context,
        AutomationOperation::CreateSubjectLock(CreateSubjectLockRequest {
            schema_version: "1".into(),
            project_path: context.root.clone(),
            spec_path: subject_spec_path,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            canonical_import_path: None,
            import_approval_note: None,
        }),
        true,
    );
    let subject_revision = subject_job
        .asset_id
        .as_deref()
        .unwrap()
        .split_once('@')
        .unwrap()
        .1
        .to_string();
    let collection = create_collection(
        &context,
        StaticAssetKind::PortraitSet,
        "two-phase-portraits",
        128,
    );
    let spec_path = context.root.join("specs/two-phase-portraits.json");
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion":"2", "kind":"portrait_set", "id":"two-phase-hero-portraits",
            "name":"Two Phase Hero Portraits", "collection":collection,
            "subject":{"id":"two-phase-hero", "revision":subject_revision},
            "framingProfile":"dialogue_bust@1.0.0",
            "expressions":[
                {"id":"neutral","name":"Neutral","prompt":"neutral expression"},
                {"id":"happy","name":"Happy","prompt":"happy expression"},
                {"id":"angry","name":"Angry","prompt":"angry expression"},
                {"id":"hurt","name":"Hurt","prompt":"hurt expression"},
                {"id":"surprised","name":"Surprised","prompt":"surprised expression"}
            ],
            "license":"MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let subject_lock = subject_lock_path(&context.root, "two-phase-hero", &subject_revision);
    let mut template = static_request(
        &context,
        &spec_path,
        StaticAssetKind::PortraitSet,
        Some(subject_lock),
    );
    template.schema_version = "5".into();
    let policies = [
        (
            PortraitNeutralReferencePolicyV1::SubjectStyle,
            vec![
                forge_core::provider::ReferenceRole::SubjectIdentity,
                forge_core::provider::ReferenceRole::Style,
            ],
        ),
        (
            PortraitNeutralReferencePolicyV1::SubjectEdit,
            vec![forge_core::provider::ReferenceRole::EditTarget],
        ),
        (
            PortraitNeutralReferencePolicyV1::LegacyThreeReference,
            vec![
                forge_core::provider::ReferenceRole::Style,
                forge_core::provider::ReferenceRole::EditTarget,
                forge_core::provider::ReferenceRole::SubjectIdentity,
            ],
        ),
    ];
    let mut approved_source = None;
    for (policy, expected_roles) in policies {
        let mut request = template.clone();
        request.portrait_phase = PortraitGenerationPhaseV1::BaseOnly;
        request.neutral_reference_policy = policy;
        let observation_start = context.provider.edit_observations().len();
        let prepared = context
            .plans
            .prepare(AutomationOperation::GenerateStaticAssetSet(request.clone()))
            .unwrap();
        assert_eq!(prepared.estimate.provider_request_estimate, 1);
        assert_eq!(prepared.estimate.maximum_provider_requests, 2);
        assert_eq!(
            prepared.estimate.workflow.as_deref(),
            Some("portrait-base@1.0.0")
        );
        let claimed = context.plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&context.jobs, &claimed).unwrap();
        let base = run_operation_with_provider(
            &context.jobs,
            &job.job_id,
            &claimed.operation,
            Some(&context.provider),
        )
        .unwrap();
        assert_eq!(base.lifecycle_state, JobLifecycleState::AwaitingReview);
        assert_eq!(
            base.error_code.as_deref(),
            Some("portrait_base_review_required")
        );
        assert!(base.job_dir.join("normalized/static/neutral.png").is_file());
        assert!(!base.job_dir.join("normalized/static/happy.png").exists());
        assert!(!base
            .artifacts
            .iter()
            .any(|artifact| artifact.kind == "gsfpack"));
        let observations = context.provider.edit_observations();
        assert_eq!(
            observations[observation_start].reference_roles,
            expected_roles
        );
        let lock: serde_json::Value = serde_json::from_slice(
            &fs::read(
                base.job_dir
                    .join("portrait-base")
                    .join(PORTRAIT_BASE_LOCK_FILE),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            lock["neutralReferencePolicy"],
            serde_json::to_value(policy).unwrap()
        );
        if policy == PortraitNeutralReferencePolicyV1::SubjectStyle {
            approved_source = Some((request, base));
        }
    }

    let (base_request, base) = approved_source.unwrap();
    let (approval_path, _approval) = approve_portrait_base(
        &base.job_id,
        &base.job_dir,
        "fixture neutral identity and equipment approved",
        chrono::Utc::now(),
    )
    .unwrap();
    context
        .jobs
        .update_record(&base.job_id, |record| {
            record.state = JobState::QualityChecked;
            record.lifecycle_state = JobLifecycleState::Succeeded;
            record.error_code = None;
            record.error_summary = None;
        })
        .unwrap();
    let mut expressions = base_request;
    expressions.portrait_phase = PortraitGenerationPhaseV1::Expressions;
    expressions.reuse_from_job_dir = Some(base.job_dir.clone());
    expressions.retry_item_ids = ["happy", "angry", "hurt", "surprised"]
        .into_iter()
        .map(str::to_string)
        .collect();
    expressions.portrait_base_parent_job_id = Some(base.job_id.clone());
    let prepared = context
        .plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(
            expressions.clone(),
        ))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 4);
    assert_eq!(prepared.estimate.maximum_provider_requests, 8);
    assert_eq!(
        prepared.estimate.workflow.as_deref(),
        Some("portrait-expressions@1.0.0")
    );
    let usage_before = context.provider.usage();
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&context.jobs, &claimed).unwrap();
    let result = run_operation_with_provider(
        &context.jobs,
        &job.job_id,
        &claimed.operation,
        Some(&context.provider),
    )
    .unwrap();
    assert_eq!(
        result.lifecycle_state,
        JobLifecycleState::Succeeded,
        "{}",
        fs::read_to_string(result.job_dir.join("portrait-consistency-report.json"))
            .unwrap_or_else(|_| "missing portrait consistency report".into())
    );
    assert_eq!(context.provider.usage().requests - usage_before.requests, 4);
    let pack = pack_path(&result);
    forge_pack::validate_pack_layout(&pack).unwrap();
    assert!(pack.join(PORTRAIT_BASE_APPROVAL_FILE).is_file());
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(
        forgepack["source"]["metadata"]["portraitNeutralReferencePolicy"],
        "subject-style@1.0.0"
    );
    assert_eq!(
        forgepack["source"]["metadata"]["portraitBaseApprovalSourceJobId"],
        base.job_id
    );

    if locate_godot().is_some() {
        let godot = temp.path().join("two-phase-godot");
        fs::create_dir(&godot).unwrap();
        fs::write(
            godot.join("project.godot"),
            "[application]\nconfig/name=\"Forge Two Phase Portrait\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let target = PathBuf::from("addons/forge_assets/two-phase-hero-portraits");
        let prepared = context
            .plans
            .prepare(AutomationOperation::InstallGodot(GodotInstallRequest {
                schema_version: "1".into(),
                pack_path: pack.clone(),
                project_path: godot.clone(),
                catalog_project_path: Some(context.root.clone()),
                target: target.clone(),
                asset_key: Some("two-phase-hero-portraits".into()),
                provider_refs: vec![],
            }))
            .unwrap();
        let claimed = context.plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&context.jobs, &claimed).unwrap();
        let installed = run_operation(&context.jobs, &job.job_id, &claimed.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let installed_root = godot.join(&target);
        let usage: serde_json::Value =
            serde_json::from_slice(&fs::read(installed_root.join("forge_usage.json")).unwrap())
                .unwrap();
        assert_eq!(
            usage["portraitConsistency"]["neutralReferencePolicy"],
            "subject-style@1.0.0"
        );
        assert_eq!(
            usage["portraitConsistency"]["approvalSourceJobId"],
            base.job_id
        );
        assert_no_embedded_godot_images(&installed_root);
    }

    let original_approval = fs::read(&approval_path).unwrap();
    let mut tampered: serde_json::Value = serde_json::from_slice(&original_approval).unwrap();
    tampered["neutralSha256"] = serde_json::json!("0".repeat(64));
    fs::write(
        &approval_path,
        serde_json::to_vec_pretty(&tampered).unwrap(),
    )
    .unwrap();
    let usage_before_rejection = context.provider.usage();
    let error = context
        .plans
        .prepare(AutomationOperation::GenerateStaticAssetSet(expressions))
        .unwrap_err();
    assert!(error.to_string().contains("approval no longer matches"));
    assert_eq!(context.provider.usage(), usage_before_rejection);
    fs::write(approval_path, original_approval).unwrap();
}

fn setup(root: &Path) -> Context {
    let project_root = root.join("game-assets");
    init_project(&project_root, "Game Assets").unwrap();
    let mut project: ForgeProjectV1 = read_project(&project_root).unwrap();
    project.provider.id = "fixture".into();
    fs::write(
        project_root.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    let style_spec_path = project_root.join("specs/style.json");
    fs::write(
        &style_spec_path,
        serde_json::to_vec_pretty(&StyleSpecV1 {
            schema_version: "1".into(),
            prompt: "compact purple pixel art".into(),
            reference_images: vec![],
            perspective: "topdown".into(),
            lighting: "upper_left".into(),
            outline: "dark".into(),
            background: "transparent".into(),
            sampling: SamplingMode::Nearest,
            character_canvas_size: 256,
            icon_canvas_size: 128,
            prop_canvas_size: 256,
            image_model: None,
        })
        .unwrap(),
    )
    .unwrap();
    let context = Context {
        root: project_root,
        plans: PlanStore::new(root.join("plans")).unwrap(),
        jobs: JobStore::new(root.join("jobs")).unwrap(),
        provider: FixtureProvider::default(),
        style_lock: PathBuf::new(),
    };
    let style = run(
        &context,
        AutomationOperation::CreateStyleLock(CreateStyleLockRequest {
            schema_version: "1".into(),
            project_path: context.root.clone(),
            spec_path: style_spec_path,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
        }),
        true,
    );
    assert_eq!(style.lifecycle_state, JobLifecycleState::Succeeded);
    let revision = read_project(&context.root)
        .unwrap()
        .current_style_revision
        .unwrap();
    Context {
        style_lock: context
            .root
            .join(".forge/styles")
            .join(revision)
            .join(STYLE_LOCK_FILE),
        ..context
    }
}

fn create_collection(
    context: &Context,
    kind: StaticAssetKind,
    id: &str,
    canvas: u32,
) -> CollectionRevisionRefV1 {
    let anchor = context.root.join("specs").join(format!("{id}-anchor.png"));
    let mut image = ImageBuffer::from_pixel(canvas, canvas, Rgba([0u8, 0, 0, 0]));
    let (left, right, top, bottom) = if kind == StaticAssetKind::PortraitSet {
        (
            canvas * 11 / 32,
            canvas * 21 / 32,
            canvas * 3 / 16,
            canvas * 13 / 16,
        )
    } else {
        (canvas / 3, canvas * 2 / 3, canvas / 4, canvas * 3 / 4)
    };
    for y in top..bottom {
        for x in left..right {
            image.put_pixel(x, y, Rgba([130, 60, 210, 255]));
        }
    }
    image.save(&anchor).unwrap();
    let spec_path = context
        .root
        .join("specs")
        .join(format!("{id}-collection.json"));
    fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion":"1", "id":id, "name":format!("{id} collection"),
            "assetKind":kind.as_str(), "prompt":"cohesive purple game art", "materials":["painted metal"],
            "scale":"consistent", "perspective":"inherit_style", "canvasSize":canvas,
            "anchorImage":anchor, "license":"MIT"
        }))
        .unwrap(),
    )
    .unwrap();
    let prepared = context
        .plans
        .prepare(AutomationOperation::CreateCollectionLock(
            CreateCollectionLockRequest {
                schema_version: "1".into(),
                project_path: context.root.clone(),
                spec_path,
                provider_id: "fixture".into(),
                profile_id: "default".into(),
            },
        ))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&context.jobs, &claimed).unwrap();
    let result =
        run_operation_with_provider(&context.jobs, &job.job_id, &claimed.operation, None).unwrap();
    assert_eq!(result.lifecycle_state, JobLifecycleState::Succeeded);
    let revision = result
        .asset_id
        .unwrap()
        .split_once('@')
        .unwrap()
        .1
        .to_string();
    CollectionRevisionRefV1 {
        id: id.into(),
        revision,
    }
}

fn generate(
    context: &Context,
    spec_path: &Path,
    kind: StaticAssetKind,
    subject_lock: Option<PathBuf>,
) -> JobRecord {
    let request = static_request(context, spec_path, kind, subject_lock);
    run(
        context,
        AutomationOperation::GenerateStaticAssetSet(request),
        true,
    )
}

fn static_request(
    context: &Context,
    spec_path: &Path,
    kind: StaticAssetKind,
    subject_lock: Option<PathBuf>,
) -> GenerateStaticAssetSetRequest {
    let resolved = read_static_collection_spec(spec_path, kind).unwrap();
    let collection = collection_lock_path(
        &context.root,
        &resolved.collection.id,
        &resolved.collection.revision,
    );
    GenerateStaticAssetSetRequest {
        schema_version: "4".into(),
        project_path: context.root.clone(),
        style_lock_path: context.style_lock.clone(),
        collection_lock_path: Some(collection),
        subject_lock_path: subject_lock,
        source_spec_path: Some(spec_path.to_path_buf()),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        framing_profile: resolved.framing_profile,
        asset: resolved.asset,
        item_metadata: resolved.item_metadata,
        max_attempts_per_item: 2,
        image_model: None,
        reuse_from_job_dir: None,
        retry_item_ids: vec![],
        replacement_item_paths: Default::default(),
        consistency_recheck_only: false,
        resume_incomplete_static: false,
        portrait_phase: Default::default(),
        neutral_reference_policy: Default::default(),
        portrait_base_parent_job_id: None,
    }
}

fn run(context: &Context, operation: AutomationOperation, provider: bool) -> JobRecord {
    let prepared = context.plans.prepare(operation).unwrap();
    let claimed = context.plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&context.jobs, &claimed).unwrap();
    run_operation_with_provider(
        &context.jobs,
        &job.job_id,
        &claimed.operation,
        provider.then_some(&context.provider as &dyn MediaGenerationProvider),
    )
    .unwrap()
}

fn recipe(record: &JobRecord) -> GenerateStaticAssetSetRequest {
    let operation: AutomationOperation =
        serde_json::from_value(record.recipe.clone().unwrap()).unwrap();
    let AutomationOperation::GenerateStaticAssetSet(request) = operation else {
        panic!("expected static recipe")
    };
    request
}

fn pack_path(record: &JobRecord) -> PathBuf {
    record
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap()
        .path
        .clone()
}

fn locate_godot() -> Option<PathBuf> {
    let godot = [
        std::env::var_os("FORGE_GODOT_PATH").map(PathBuf::from),
        Some(PathBuf::from(
            "/Applications/Godot.app/Contents/MacOS/Godot",
        )),
        Some(PathBuf::from("/opt/homebrew/bin/godot")),
        Some(PathBuf::from("/usr/local/bin/godot")),
    ]
    .into_iter()
    .flatten()
    .find(|path| path.is_file());
    if godot.is_none() && std::env::var("FORGE_REQUIRE_GODOT").as_deref() == Ok("1") {
        panic!(
            "FORGE_REQUIRE_GODOT=1 but Godot was not found; set FORGE_GODOT_PATH to a Godot 4.6 executable"
        );
    }
    godot
}

fn assert_no_embedded_godot_images(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                pending.push(entry.path());
                continue;
            }
            let extension = entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_string();
            if matches!(extension.as_str(), "tres" | "tscn") {
                let bytes = fs::read(entry.path()).unwrap();
                assert!(bytes.len() < 1024 * 1024);
                let text = String::from_utf8_lossy(&bytes);
                assert!(!text.contains("PackedByteArray"));
                assert!(!text.contains("ImageTexture.create_from_image"));
            }
        }
    }
}
