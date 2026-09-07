use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use forge_core::asset_project::{
    init_project, read_project, CharacterEquipmentKindV1, SamplingMode, StyleSpecV1,
    FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::{
    automation_profile, run_operation, run_operation_with_provider, stage_plan_job,
    AutomationOperation, CharacterPackMetadata, CharacterRetryStage, CharacterWorkflowSelection,
    CreateStyleLockRequest, CreateSubjectLockRequest, GenerateCharacterPackRequest,
    GeneratedCharacterSpec, GenerationPolicy, GodotInstallRequest, PlanStore, QualityPolicy,
};
use forge_core::catalog::read_project_catalog;
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::project::ProviderAssetRef;
use forge_core::provider::{MediaGenerationProvider, ReferenceRole};
use forge_core::subject::{read_subject_lock, SubjectSpecV1};
use forge_core::workflow_graph::{read_workflow_graph, WORKFLOW_GRAPH_FILE};
use forge_providers::fixture::FixtureProvider;

#[test]
fn fixture_keyframes_create_pack_graph_catalog_and_single_frame_retry() {
    let temp = tempfile::tempdir().unwrap();
    let cache_root = temp.path().join("cache");
    temp_env::with_var("FORGE_CACHE_STORE", Some(&cache_root), || {
        run_fixture_keyframes_contract(&temp)
    });
}

fn run_fixture_keyframes_contract(temp: &tempfile::TempDir) {
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "Keyframe Contract").unwrap();
    project.provider.id = "fixture".into();
    fs::write(
        project_root.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    let style_spec = temp.path().join("style.json");
    fs::write(
        &style_spec,
        serde_json::to_vec_pretty(&StyleSpecV1 {
            schema_version: "1".into(),
            prompt: "compact purple pixel art".into(),
            reference_images: vec![],
            perspective: "top_down".into(),
            lighting: "soft".into(),
            outline: "clean".into(),
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
    let subject_spec = temp.path().join("subject.json");
    fs::write(
        &subject_spec,
        serde_json::to_vec_pretty(&SubjectSpecV1 {
            schema_version: "1".into(),
            id: "fixture-ranger".into(),
            name: "Fixture Ranger".into(),
            prompt: "compact purple ranger".into(),
            reference_images: vec![],
            image_model: None,
            license: "MIT".into(),
        })
        .unwrap(),
    )
    .unwrap();
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();

    run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::CreateStyleLock(CreateStyleLockRequest {
            schema_version: "1".into(),
            project_path: project_root.clone(),
            spec_path: style_spec,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
        }),
    );
    let style_revision = read_project(&project_root)
        .unwrap()
        .current_style_revision
        .unwrap();
    run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::CreateSubjectLock(CreateSubjectLockRequest {
            schema_version: "1".into(),
            project_path: project_root.clone(),
            spec_path: subject_spec,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            canonical_import_path: None,
            import_approval_note: None,
        }),
    );
    let subject_lock_path = project_root
        .join(".forge/subjects/fixture-ranger")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("subject-lock.json");
    let subject = read_subject_lock(&subject_lock_path).unwrap();
    assert_eq!(subject.image_model.as_deref(), Some("fixture-image"));
    let profile = automation_profile();
    let fixture_model = format!(
        "fixture-image-{}",
        temp.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("run")
    );
    let request = GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: Some(project_root.clone()),
        asset_id: Some("fixture-ranger-keyframes".into()),
        character: GeneratedCharacterSpec {
            prompt: subject.prompt.clone(),
            reference_image_path: Some(subject.canonical_path.clone()),
        },
        camera_profile: Default::default(),
        equipment: Default::default(),
        equipment_explicit: true,
        style_lock_path: Some(
            project_root
                .join(".forge/styles")
                .join(style_revision)
                .join(STYLE_LOCK_FILE),
        ),
        subject_lock_path: Some(subject_lock_path),
        reuse_from_job_dir: None,
        retry_animations: vec![],
        retry_stages: BTreeMap::new(),
        retry_frames: BTreeMap::new(),
        direction_grid_cape_contract: None,
        validation_only: false,
        validation_animations: vec![],
        motion_profile: Default::default(),
        direction_motion_stage: Default::default(),
        metadata: CharacterPackMetadata {
            name: "Fixture Ranger Keyframes".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-keyframes".into(),
            version: "2.3.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 8,
            video_duration_seconds: 4,
            image_model: Some(fixture_model.clone()),
            video_model: None,
            pose_guidance: Default::default(),
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    };
    let mut valid_staff = request.clone();
    valid_staff.equipment.kind = CharacterEquipmentKindV1::StaffLike;
    valid_staff.equipment.reference_image = Some(subject.canonical_path.clone());
    plans
        .prepare(AutomationOperation::GenerateCharacterPack(valid_staff))
        .expect("V2.3 staff lock with a clean PNG reference must plan");

    let mut implicit_equipment = request.clone();
    implicit_equipment.equipment_explicit = false;
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            implicit_equipment
        ))
        .unwrap_err()
        .to_string()
        .contains("explicit equipment declaration"));

    let mut none_with_reference = request.clone();
    none_with_reference.equipment.reference_image = Some(subject.canonical_path.clone());
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            none_with_reference
        ))
        .unwrap_err()
        .to_string()
        .contains("forbidden when equipment.kind is none"));

    let mut staff_without_reference = request.clone();
    staff_without_reference.equipment.kind = CharacterEquipmentKindV1::StaffLike;
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            staff_without_reference
        ))
        .unwrap_err()
        .to_string()
        .contains("staff_like equipment requires a clean referenceImage"));

    let mut legacy_with_equipment = request.clone();
    legacy_with_equipment.workflow.version = "2.1.0".into();
    legacy_with_equipment.equipment.kind = CharacterEquipmentKindV1::StaffLike;
    legacy_with_equipment.equipment.reference_image = Some(subject.canonical_path.clone());
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            legacy_with_equipment
        ))
        .unwrap_err()
        .to_string()
        .contains("explicit Character equipment requires topdown-keyframes@2.2.0 or @2.3.0"));

    let mut legacy_validation = request.clone();
    legacy_validation.workflow.version = "2.1.0".into();
    legacy_validation.validation_only = true;
    legacy_validation.validation_animations = vec!["walk_right".into()];
    assert!(plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            legacy_validation
        ))
        .unwrap_err()
        .to_string()
        .contains("validationOnly requires topdown-keyframes@2.2.0 or @2.3.0"));

    let completed = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(request.clone()),
    );
    let consistency_details = fs::read_to_string(completed.job_dir.join("consistency-report.json"))
        .unwrap_or_else(|_| "no consistency report".into());
    let equipment_details =
        fs::read_to_string(completed.job_dir.join("hand-equipment-contact-report.json"))
            .unwrap_or_else(|_| "no equipment report".into());
    let quality_details =
        fs::read_to_string(completed.job_dir.join("animation-quality-report.json"))
            .unwrap_or_else(|_| "no animation quality report".into());
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "error_code={:?} error_summary={:?} consistency={} equipment={} quality={}",
        completed.error_code,
        completed.error_summary,
        consistency_details,
        equipment_details,
        quality_details,
    );
    let graph = read_workflow_graph(&completed.job_dir.join(WORKFLOW_GRAPH_FILE)).unwrap();
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "frame_image")
            .count(),
        32
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "provider_image")
            .count(),
        32
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "background_cleanup")
            .count(),
        32
    );
    assert!(graph
        .nodes
        .iter()
        .filter(|node| node.stage == "background_cleanup" || node.stage == "frame_image")
        .all(|node| !node.provider_request));
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "matting")
            .count(),
        4
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "provisional_align")
            .count(),
        4
    );
    assert_eq!(
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "loop_quality")
            .count(),
        4
    );
    let consistency_node = graph
        .nodes
        .iter()
        .find(|node| node.id == "collection_consistency")
        .expect("collection consistency node");
    assert_eq!(consistency_node.implementation_version, "consistency@1.6.0");
    assert!(graph.nodes.iter().any(|node| node.id == "shared_normalize"));
    assert!(graph
        .nodes
        .iter()
        .filter(|node| node.provider_request)
        .all(|node| node.model.as_deref() == Some(fixture_model.as_str())));
    let completed_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(completed_manifest["model"], fixture_model);
    assert_eq!(completed_manifest["workflow"], "topdown-keyframes@2.3.0");
    assert_eq!(completed_manifest["poseProfile"], "topdown-poses@1.2.0");
    assert_eq!(
        completed_manifest["directionGuideProfile"],
        "topdown-direction-guides@1.0.0"
    );
    assert_eq!(
        completed_manifest["directionAnchorProfile"],
        "direction-anchor@1.2.0"
    );
    assert_eq!(
        completed_manifest["backgroundCleanupProfile"],
        "keyframe-background-cleanup@1.3.0"
    );
    assert!(completed_manifest["directionLock"]["sha256"]
        .as_str()
        .is_some_and(|sha256| sha256.len() == 64));
    let direction_lock: serde_json::Value = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(direction_lock["profile"], "direction-lock@1.0.0");
    assert_eq!(direction_lock["entries"].as_array().unwrap().len(), 3);
    assert!(direction_lock["entries"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["assessment"]["verdict"] == "game_ready"));
    assert_eq!(
        completed_manifest["styleReferenceMode"],
        "prompt_descriptor"
    );
    assert!(completed_manifest["styleDescriptor"]["sha256"]
        .as_str()
        .is_some_and(|sha256| sha256.len() == 64));
    assert_eq!(
        completed_manifest["hardGateProfile"],
        "keyframe-hard-defects@1.0.0"
    );
    let inbetween = completed_manifest["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| frame["animation"] == "walk_right" && frame["frame"] == 3)
        .expect("walk_right in-between frame");
    assert_eq!(
        inbetween["roles"],
        serde_json::json!(["start_keyframe", "end_keyframe", "pose_structure"])
    );
    let anchor = completed_manifest["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| frame["animation"] == "walk_right" && frame["frame"] == 2)
        .expect("walk_right anchor frame");
    assert_eq!(
        anchor["roles"],
        serde_json::json!(["edit_target", "pose_structure"])
    );
    let keyframe_observations = provider
        .edit_observations()
        .into_iter()
        .filter(|observation| {
            observation
                .authorization_target
                .as_deref()
                .is_some_and(|target| target.contains(":frame:"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        keyframe_observations.len(),
        graph
            .nodes
            .iter()
            .filter(|node| node.stage == "provider_image" && node.provider_request)
            .count()
    );
    assert!(!keyframe_observations.is_empty());
    assert!(keyframe_observations
        .iter()
        .all(|observation| !observation.reference_roles.contains(&ReferenceRole::Style)));
    let anchor_node = graph
        .nodes
        .iter()
        .find(|node| node.id == "provider_image:walk_right:2")
        .expect("walk_right anchor workflow node");
    assert!(anchor_node.inputs.iter().any(|input| {
        input.path.file_name().and_then(|name| name.to_str())
            == Some("character-style-descriptor@1.0.0.json")
    }));
    assert!(!anchor_node.inputs.iter().any(|input| {
        input.path.file_name().and_then(|name| name.to_str()) == Some("style-board.png")
    }));
    let pose = image::open(
        completed
            .job_dir
            .join("source/pose-guides/topdown-poses@1.2.0/walk_right-frame-02.png"),
    )
    .unwrap()
    .to_rgba8();
    assert_eq!(pose.get_pixel(0, 0)[3], 0);
    assert_eq!(pose.get_pixel(pose.width() - 1, pose.height() - 1)[3], 0);
    assert!(
        pose.pixels().filter(|pixel| pixel[3] > 0).count()
            < (pose.width() * pose.height() / 5) as usize
    );
    let inbetween_node = graph
        .nodes
        .iter()
        .find(|node| node.id == "provider_image:walk_right:3")
        .expect("in-between workflow node");
    assert_eq!(
        inbetween_node.depends_on,
        vec![
            "frame_image:walk_right:2".to_string(),
            "frame_image:walk_right:4".to_string()
        ]
    );
    let normalized_inbetween = graph
        .nodes
        .iter()
        .find(|node| node.id == "frame_image:walk_right:3")
        .expect("normalized in-between node");
    assert_eq!(
        normalized_inbetween.depends_on,
        vec!["background_cleanup:walk_right:3".to_string()]
    );
    let cleaned_frame = completed_manifest["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| frame["animation"] == "walk_right" && frame["frame"] == 3)
        .unwrap();
    assert_ne!(cleaned_frame["rawSha256"], cleaned_frame["cleanedSha256"]);
    assert_eq!(
        cleaned_frame["backgroundCleanupProfile"],
        "keyframe-background-cleanup@1.3.0"
    );
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("keyframe gsfpack");
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let packed_direction_lock: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("character-direction-lock.json")).unwrap())
            .unwrap();
    assert!(packed_direction_lock["entries"]
        .as_array()
        .unwrap()
        .iter()
        .all(|entry| entry["path"]
            .as_str()
            .is_some_and(|path| path.starts_with("assets/frames/frame_"))));
    assert!(
        !fs::read_to_string(pack.path.join("character-direction-lock.json"))
            .unwrap()
            .contains(temp.path().to_string_lossy().as_ref())
    );
    assert!(read_project_catalog(&project_root)
        .unwrap()
        .assets
        .contains_key("fixture-ranger-keyframes"));

    if let Some(godot) = locate_godot() {
        let godot_project = temp.path().join("godot-project");
        fs::create_dir(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge Keyframe Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack.path.clone(),
            project_path: godot_project.clone(),
            catalog_project_path: Some(project_root.clone()),
            target: PathBuf::from("addons/forge_assets/fixture_ranger_keyframes"),
            asset_key: Some("fixture_ranger_keyframes".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("fixture-ranger-keyframes".into()),
                label: Some("Fixture Ranger Keyframes".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let target = godot_project.join("addons/forge_assets/fixture_ranger_keyframes");
        let usage: serde_json::Value =
            serde_json::from_slice(&fs::read(target.join("forge_usage.json")).unwrap()).unwrap();
        assert_eq!(
            usage["characterDirectionLock"]["profile"],
            "direction-lock@1.0.0"
        );
        assert_eq!(
            usage["keyframeBackgroundCleanupProfile"],
            "keyframe-background-cleanup@1.3.0"
        );
        for resource in [
            target.join("forge_sprite_frames.tres"),
            target.join("forge_animated_sprite.tscn"),
        ] {
            assert!(fs::metadata(&resource).unwrap().len() < 1024 * 1024);
            let text = fs::read_to_string(resource).unwrap();
            assert!(!text.contains("PackedByteArray"));
            assert!(!text.contains("sub_resource type=\"Image\""));
        }
        let output = Command::new(godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot keyframe project load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        eprintln!("skipping keyframe Godot installation because Godot 4 is unavailable");
    }

    let mut validation = request.clone();
    validation.asset_id = Some("fixture-ranger-walk-right-validation".into());
    validation.validation_only = true;
    validation.validation_animations = vec!["walk_right".into()];
    validation.metadata.default_animation = "walk_right".into();
    validation.generation.image_model = Some(format!(
        "fixture-walk-right-validation-{}",
        temp.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("run")
    ));
    let validation_observation_start = provider.edit_observations().len();
    let validation = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(validation),
    );
    assert_eq!(validation.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(!validation
        .artifacts
        .iter()
        .any(|artifact| { matches!(artifact.kind.as_str(), "gsfpack" | "candidate_gsfpack") }));
    let validation_graph =
        read_workflow_graph(&validation.job_dir.join(WORKFLOW_GRAPH_FILE)).unwrap();
    assert_eq!(
        validation_graph
            .nodes
            .iter()
            .filter(|node| node.stage == "frame_image")
            .count(),
        8
    );
    assert!(validation_graph
        .nodes
        .iter()
        .filter_map(|node| node.item.as_deref())
        .all(|item| item == "walk_right"));
    let validation_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            validation
                .job_dir
                .join("source/keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(validation_manifest["validationOnly"], true);
    assert_eq!(
        validation_manifest["validationAnimations"],
        serde_json::json!(["walk_right"])
    );
    assert_eq!(validation_manifest["usage"]["requests"], 8);
    let validation_observations = provider.edit_observations();
    let validation_observations = &validation_observations[validation_observation_start..];
    assert_eq!(validation_observations.len(), 8);
    assert!(validation_observations
        .iter()
        .all(|observation| !observation.reference_roles.contains(&ReferenceRole::Style)));
    let walk_right_anchor_observation = validation_observations
        .iter()
        .find(|observation| {
            observation.authorization_target.as_deref() == Some("walk_right:frame:2")
        })
        .expect("walk_right frame 2 provider observation");
    assert_eq!(
        walk_right_anchor_observation.reference_roles,
        vec![ReferenceRole::EditTarget, ReferenceRole::PoseStructure]
    );

    let mut keypose_v24 = request_from_job(&completed);
    keypose_v24.asset_id = Some("fixture-ranger-keyposes-v24".into());
    keypose_v24.workflow.id = "topdown-keyposes".into();
    keypose_v24.workflow.version = "2.4.0".into();
    keypose_v24.generation.target_frame_count = 4;
    keypose_v24.validation_only = false;
    keypose_v24.validation_animations.clear();
    keypose_v24.metadata.default_animation = "idle".into();
    keypose_v24.quality.require_game_ready = true;
    keypose_v24.generation.image_model = Some(format!(
        "fixture-keyposes-v24-{}",
        temp.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("run")
    ));
    let keypose_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            keypose_v24.clone(),
        ))
        .unwrap();
    assert_eq!(keypose_plan.estimate.provider_request_estimate, 16);
    assert_eq!(keypose_plan.estimate.maximum_provider_requests, 32);
    let keypose_v24 = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(keypose_v24),
    );
    let keypose_motion_path = keypose_v24
        .job_dir
        .join("character-motion-semantics-report.json");
    assert!(
        keypose_motion_path.is_file(),
        "V2.4 result={keypose_v24:#?}"
    );
    let keypose_motion: serde_json::Value =
        serde_json::from_slice(&fs::read(keypose_motion_path).unwrap()).unwrap();
    assert_eq!(keypose_motion["profile"], "motion-semantics@1.5.0");
    assert_eq!(
        keypose_v24.lifecycle_state,
        JobLifecycleState::Succeeded,
        "motion={keypose_motion:#?} consistency={} silhouette={} source_silhouette={} error={:?}",
        fs::read_to_string(keypose_v24.job_dir.join("consistency-report.json"))
            .unwrap_or_else(|_| "missing".into()),
        fs::read_to_string(
            keypose_v24
                .job_dir
                .join("character-silhouette-temporal-report.json")
        )
        .unwrap_or_else(|_| "missing".into()),
        fs::read_to_string(
            keypose_v24
                .job_dir
                .join("character-silhouette-temporal-source-report.json")
        )
        .unwrap_or_else(|_| "missing".into()),
        keypose_v24.error_summary,
    );
    assert_eq!(keypose_motion["verdict"], "game_ready");
    assert_eq!(keypose_motion["animations"].as_array().unwrap().len(), 4);
    let keypose_graph =
        read_workflow_graph(&keypose_v24.job_dir.join(WORKFLOW_GRAPH_FILE)).unwrap();
    assert_eq!(
        keypose_graph
            .nodes
            .iter()
            .filter(|node| node.stage == "provider_image")
            .count(),
        16
    );
    let keypose_frame_2 = keypose_graph
        .nodes
        .iter()
        .find(|node| node.id == "provider_image:walk_right:2")
        .unwrap();
    assert_eq!(
        keypose_frame_2.depends_on,
        vec![
            "frame_image:walk_right:0".to_string(),
            "frame_image:walk_right:1".to_string(),
        ]
    );
    let keypose_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            keypose_v24
                .job_dir
                .join("source/keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(keypose_manifest["workflow"], "topdown-keyposes@2.4.0");
    assert_eq!(keypose_manifest["poseProfile"], "topdown-keyposes@2.0.0");
    let keypose_frame_2_manifest = keypose_manifest["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| frame["animation"] == "walk_right" && frame["frame"] == 2)
        .unwrap();
    assert_eq!(
        keypose_frame_2_manifest["roles"],
        serde_json::json!(["edit_target", "start_keyframe", "pose_structure"])
    );
    let keypose_pack = keypose_v24
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("V2.4 keypose pack");
    forge_pack::validate_pack_layout(&keypose_pack.path).unwrap();
    let keypose_forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(keypose_pack.path.join("forgepack.json")).unwrap())
            .unwrap();
    assert_eq!(
        keypose_forgepack["assets"]["characterMotionSemanticsReport"],
        "character-motion-semantics-report.json"
    );
    assert_eq!(
        keypose_forgepack["source"]["metadata"]["characterMotionSemantics"]["profile"],
        "motion-semantics@1.5.0"
    );
    assert!(keypose_pack
        .path
        .join("character-motion-semantics-report.json")
        .is_file());

    if locate_godot().is_some() {
        let godot_project = temp.path().join("godot-keyposes-v24");
        fs::create_dir(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge Keyposes V2.4\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: keypose_pack.path.clone(),
            project_path: godot_project,
            catalog_project_path: Some(project_root.clone()),
            target: PathBuf::from("addons/forge_assets/fixture_ranger_keyposes_v24"),
            asset_key: Some("fixture_ranger_keyposes_v24".into()),
            provider_refs: vec![],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let usage: serde_json::Value = serde_json::from_slice(
            &fs::read(
                installed
                    .job_dir
                    .join("staged-target/forge_usage.json"),
            )
            .or_else(|_| {
                fs::read(
                    temp.path()
                        .join("godot-keyposes-v24/addons/forge_assets/fixture_ranger_keyposes_v24/forge_usage.json"),
                )
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            usage["characterMotionSemantics"]["profile"],
            "motion-semantics@1.5.0"
        );
    }

    let keypose_v25_observation_start = provider.edit_observations().len();
    let mut keypose_v25 = request_from_job(&completed);
    keypose_v25.asset_id = Some("fixture-ranger-keyposes-v25".into());
    keypose_v25.workflow.id = "topdown-keyposes".into();
    keypose_v25.workflow.version = "2.5.0".into();
    keypose_v25.generation.target_frame_count = 4;
    keypose_v25.validation_only = false;
    keypose_v25.validation_animations.clear();
    keypose_v25.metadata.default_animation = "idle".into();
    keypose_v25.quality.require_game_ready = true;
    keypose_v25.generation.image_model = Some(format!(
        "fixture-keyposes-v25-{}",
        temp.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("run")
    ));
    let keypose_v25_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            keypose_v25.clone(),
        ))
        .unwrap();
    assert_eq!(keypose_v25_plan.estimate.provider_request_estimate, 16);
    assert_eq!(keypose_v25_plan.estimate.maximum_provider_requests, 32);
    let keypose_v25 = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(keypose_v25),
    );
    assert_eq!(keypose_v25.lifecycle_state, JobLifecycleState::Succeeded);
    let keypose_v25_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            keypose_v25
                .job_dir
                .join("source/keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(keypose_v25_manifest["workflow"], "topdown-keyposes@2.5.0");
    assert_eq!(
        keypose_v25_manifest["poseProfile"],
        "topdown-keyposes@2.1.0"
    );
    assert_eq!(
        keypose_v25_manifest["referenceIsolation"],
        "direction_lock_plus_pose_only"
    );
    let v25_frame_2 = keypose_v25_manifest["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| frame["animation"] == "walk_right" && frame["frame"] == 2)
        .unwrap();
    assert_eq!(
        v25_frame_2["roles"],
        serde_json::json!(["edit_target", "pose_structure"])
    );
    assert_eq!(
        v25_frame_2["referenceIsolation"],
        "direction_lock_plus_pose_only"
    );
    let keypose_v25_graph =
        read_workflow_graph(&keypose_v25.job_dir.join(WORKFLOW_GRAPH_FILE)).unwrap();
    let v25_graph_frame_2 = keypose_v25_graph
        .nodes
        .iter()
        .find(|node| node.id == "provider_image:walk_right:2")
        .unwrap();
    assert_eq!(
        v25_graph_frame_2.depends_on,
        vec!["frame_image:walk_right:0".to_string()]
    );
    let keypose_v25_observations = provider.edit_observations();
    let keypose_v25_observations = &keypose_v25_observations[keypose_v25_observation_start..];
    assert_eq!(keypose_v25_observations.len(), 16);
    assert!(keypose_v25_observations.iter().all(|observation| {
        let is_followup = observation
            .authorization_target
            .as_deref()
            .and_then(|target| target.rsplit(':').next())
            != Some("0");
        !is_followup
            || observation.reference_roles
                == vec![ReferenceRole::EditTarget, ReferenceRole::PoseStructure]
    }));

    let mut blocked_keyposes = request_from_job(&completed);
    blocked_keyposes.asset_id = Some("fixture-ranger-keyposes-v25-blocked".into());
    blocked_keyposes.workflow.id = "topdown-keyposes".into();
    blocked_keyposes.workflow.version = "2.5.0".into();
    blocked_keyposes.generation.target_frame_count = 4;
    blocked_keyposes.generation.image_model = Some(format!(
        "fixture-keyposes-v25-blocked-{}",
        temp.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("run")
    ));
    let blocked_provider = FixtureProvider::default()
        .with_review_keyframes()
        .with_static_keyposes();
    let blocked = run(
        &plans,
        &jobs,
        &blocked_provider,
        AutomationOperation::GenerateCharacterPack(blocked_keyposes),
    );
    assert_eq!(blocked.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        blocked.error_code.as_deref(),
        Some("character_motion_semantics_failed")
    );
    assert!(!blocked
        .next_actions
        .iter()
        .any(|action| action.contains("review")));
    assert!(!blocked
        .artifacts
        .iter()
        .any(|artifact| matches!(artifact.kind.as_str(), "gsfpack" | "candidate_gsfpack")));
    let blocked_motion: serde_json::Value = serde_json::from_slice(
        &fs::read(
            blocked
                .job_dir
                .join("character-motion-semantics-report.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(blocked_motion["verdict"], "blocked");
    assert!(blocked_motion["animations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|animation| animation["name"].as_str().unwrap().starts_with("walk"))
        .all(|animation| !animation["recommendedRetryFrames"]
            .as_array()
            .unwrap()
            .is_empty()));

    let mut legacy_v22 = request_from_job(&completed);
    legacy_v22.asset_id = Some("fixture-ranger-v22-compat".into());
    legacy_v22.workflow.version = "2.2.0".into();
    legacy_v22.validation_only = true;
    legacy_v22.validation_animations = vec!["walk_right".into()];
    legacy_v22.metadata.default_animation = "walk_right".into();
    legacy_v22.generation.image_model = Some(format!(
        "fixture-v22-compat-{}",
        temp.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("run")
    ));
    let legacy_v22 = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(legacy_v22),
    );
    assert_eq!(legacy_v22.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(!legacy_v22
        .job_dir
        .join("source/direction-lock.json")
        .exists());
    let legacy_graph = read_workflow_graph(&legacy_v22.job_dir.join(WORKFLOW_GRAPH_FILE)).unwrap();
    assert_eq!(
        legacy_graph
            .nodes
            .iter()
            .filter(|node| node.stage == "frame_image" && node.provider_request)
            .count(),
        8
    );
    assert!(!legacy_graph
        .nodes
        .iter()
        .any(|node| node.stage == "background_cleanup"));

    let retry_observation_start = provider.edit_observations().len();
    let mut retry = request;
    retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    retry.retry_animations = vec!["walk_right".into()];
    retry
        .retry_stages
        .insert("walk_right".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_right".into(), vec![3]);
    let retried = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(retry),
    );
    assert_eq!(
        retried.parent_job_id.as_deref(),
        Some(completed.job_id.as_str())
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            retried
                .job_dir
                .join("source/keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["usage"]["requests"], 1);
    let retry_observations = provider.edit_observations();
    let retry_observations = &retry_observations[retry_observation_start..];
    assert_eq!(retry_observations.len(), 1);
    let cleaned_start = completed_manifest["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| frame["animation"] == "walk_right" && frame["frame"] == 2)
        .unwrap()["outputSha256"]
        .as_str()
        .unwrap();
    let cleaned_end = completed_manifest["frames"]
        .as_array()
        .unwrap()
        .iter()
        .find(|frame| frame["animation"] == "walk_right" && frame["frame"] == 4)
        .unwrap()["outputSha256"]
        .as_str()
        .unwrap();
    assert!(retry_observations[0]
        .reference_sha256
        .iter()
        .any(|sha256| sha256 == cleaned_start));
    assert!(retry_observations[0]
        .reference_sha256
        .iter()
        .any(|sha256| sha256 == cleaned_end));

    let mut local_request = request_from_job(&completed);
    local_request.asset_id = Some("fixture-ranger-local-replay".into());
    local_request.reuse_from_job_dir = Some(completed.job_dir.clone());
    local_request.retry_animations = vec![
        "idle".into(),
        "walk_up".into(),
        "walk_right".into(),
        "walk_down".into(),
    ];
    local_request.retry_stages = local_request
        .retry_animations
        .iter()
        .map(|animation| (animation.clone(), CharacterRetryStage::Consistency))
        .collect();
    let local = run(
        &plans,
        &jobs,
        &provider,
        AutomationOperation::GenerateCharacterPack(local_request),
    );
    let local_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(local.job_dir.join("source/keyframe-provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(local_manifest["usage"]["requests"], 0);

    let mut wrong_direction_request = request_from_job(&completed);
    wrong_direction_request.asset_id = Some("fixture-ranger-wrong-right".into());
    wrong_direction_request.validation_only = true;
    wrong_direction_request.validation_animations = vec!["walk_right".into()];
    wrong_direction_request.metadata.default_animation = "walk_right".into();
    wrong_direction_request.generation.image_model = Some(format!(
        "fixture-wrong-right-{}",
        temp.path()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("run")
    ));
    let wrong_direction_provider = FixtureProvider::default().with_wrong_right_direction();
    let wrong_direction = run(
        &plans,
        &jobs,
        &wrong_direction_provider,
        AutomationOperation::GenerateCharacterPack(wrong_direction_request),
    );
    assert_eq!(wrong_direction.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(
        wrong_direction.error_code.as_deref(),
        Some("direction_lock_required")
    );
    assert_eq!(wrong_direction_provider.usage().requests, 2);
    let wrong_lock: serde_json::Value = serde_json::from_slice(
        &fs::read(wrong_direction.job_dir.join("source/direction-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(wrong_lock["entries"].as_array().unwrap().len(), 1);
    assert_eq!(wrong_lock["entries"][0]["assessment"]["verdict"], "blocked");
    let wrong_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            wrong_direction
                .job_dir
                .join("source/keyframe-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(wrong_manifest["partial"], true);
    assert_eq!(wrong_manifest["failureCode"], "direction_lock_required");
    assert_eq!(wrong_manifest["usage"]["requests"], 2);
    assert!(wrong_direction
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "workflow_graph"));
    assert!(wrong_direction
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind.starts_with("keyframe_walk_right_"))
        .all(|artifact| artifact.kind == "keyframe_walk_right_00"));

    let mut review_request = request_from_job(&completed);
    review_request.asset_id = Some("fixture-ranger-review".into());
    review_request.project_path = Some(project_root.clone());
    review_request.generation.image_model = Some("fixture-review".into());
    let review_provider = FixtureProvider::default().with_review_keyframes();
    let review = run(
        &plans,
        &jobs,
        &review_provider,
        AutomationOperation::GenerateCharacterPack(review_request),
    );
    assert_eq!(review.lifecycle_state, JobLifecycleState::AwaitingReview);
    assert!(review
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "candidate_gsfpack"));
    assert!(!review
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));
}

fn request_from_job(completed: &forge_core::job::JobRecord) -> GenerateCharacterPackRequest {
    let operation: AutomationOperation =
        serde_json::from_value(completed.recipe.clone().expect("immutable recipe")).unwrap();
    let AutomationOperation::GenerateCharacterPack(mut request) = operation else {
        panic!("expected Character recipe");
    };
    request.reuse_from_job_dir = None;
    request.retry_animations.clear();
    request.retry_stages.clear();
    request.retry_frames.clear();
    request
}

fn run(
    plans: &PlanStore,
    jobs: &JobStore,
    provider: &dyn MediaGenerationProvider,
    operation: AutomationOperation,
) -> forge_core::job::JobRecord {
    let prepared = plans.prepare(operation).unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(jobs, &claimed).unwrap();
    run_operation_with_provider(jobs, &job.job_id, &claimed.operation, Some(provider)).unwrap()
}

fn locate_godot() -> Option<PathBuf> {
    [
        PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot"),
        PathBuf::from("/Applications/Godot_mono.app/Contents/MacOS/Godot"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .or_else(|| {
        ["godot4", "godot"].into_iter().find_map(|name| {
            let output = Command::new("/usr/bin/which").arg(name).output().ok()?;
            output
                .status
                .success()
                .then(|| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_string()))
        })
    })
}
