use forge_core::asset_project::{
    init_project, read_project, ForgeProjectV1, SamplingMode, StyleSpecV1, FORGE_PROJECT_FILE,
    STYLE_LOCK_FILE,
};
use forge_core::automation::{
    automation_profile, run_operation, run_operation_with_provider, stage_plan_job,
    AutomationOperation, CharacterPackMetadata, CharacterRetryStage, CharacterWorkflowSelection,
    CreateStyleLockRequest, GenerateCharacterPackRequest, GeneratedCharacterSpec, GenerationPolicy,
    GodotInstallRequest, PlanStore, QualityPolicy,
};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::project::ProviderAssetRef;
use forge_core::provider::MediaGenerationProvider;
use forge_providers::fixture::FixtureProvider;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::process::Command;

#[test]
fn fixture_provider_generates_a_game_ready_topdown_pack() {
    if forge_core::video::ffmpeg::find_in_path("ffmpeg").is_none()
        || forge_core::video::ffmpeg::find_in_path("ffprobe").is_none()
    {
        eprintln!("skipping provider contract test because ffmpeg/ffprobe are unavailable");
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let profile = automation_profile();
    let operation = AutomationOperation::GenerateCharacterPack(GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: None,
        asset_id: None,
        character: GeneratedCharacterSpec {
            prompt: "A compact red adventurer with a blue hood".into(),
            reference_image_path: None,
        },
        style_lock_path: None,
        subject_lock_path: None,
        reuse_from_job_dir: None,
        retry_animations: vec![],
        retry_stages: Default::default(),
        retry_frames: Default::default(),
        metadata: CharacterPackMetadata {
            name: "Fixture Hero".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "private".into(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown".into(),
            version: "1.0.0".into(),
        },
        generation: GenerationPolicy::default(),
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    });
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let provider = FixtureProvider::default();

    let completed = run_operation_with_provider(
        &jobs,
        &queued.job_id,
        &plan.operation,
        Some(&provider as &dyn MediaGenerationProvider),
    )
    .unwrap();

    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        eprintln!(
            "{}",
            std::fs::read_to_string(completed.job_dir.join("loop-selection-report.json")).unwrap()
        );
    }
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("generated pack artifact");
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    let provider_manifest = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "provider_manifest")
        .expect("provider manifest");
    let manifest_text = std::fs::read_to_string(&provider_manifest.path).unwrap();
    assert!(manifest_text.contains("\"providerId\": \"fixture\""));
    assert!(!manifest_text.contains("access_token"));
    assert!(!manifest_text.contains("refresh_token"));
    let loop_report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(pack.path.join("quality/loops.json")).unwrap())
            .unwrap();
    let loops = loop_report["animations"].as_array().unwrap();
    assert_eq!(loops.len(), 4);
    for animation in loops {
        let report = &animation["report"];
        let output_indices = report["outputFrameIndices"].as_array().unwrap();
        let output_hashes = report["outputFrameSha256"].as_array().unwrap();
        assert_eq!(output_indices.len(), 8);
        assert_eq!(output_hashes.len(), 8);
        assert!(!output_indices.contains(&report["selectedEndBoundaryFrame"]));
        let name = animation["name"].as_str().unwrap();
        for (output_index, expected_hash) in output_hashes.iter().enumerate() {
            let selected = completed
                .job_dir
                .join("animations")
                .join(name)
                .join("processed/loop-selected")
                .join(format!("frame_{:05}.png", output_index + 1));
            let selected_bytes = std::fs::read(&selected).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(&selected_bytes)),
                expected_hash.as_str().unwrap()
            );
            let source = completed
                .job_dir
                .join("animations")
                .join(name)
                .join("processed/matted")
                .join(format!("frame_{:05}.png", output_index + 1));
            assert_eq!(selected_bytes, std::fs::read(source).unwrap());
        }
    }
    let exported_frame_count = std::fs::read_dir(pack.path.join("assets/frames"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("png"))
        .count();
    assert_eq!(exported_frame_count, 32);
    let provider_usage = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "provider_usage")
        .expect("provider usage artifact");
    let usage: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&provider_usage.path).unwrap()).unwrap();
    assert_eq!(usage["providerId"], "fixture");
    assert!(usage["usage"]["requests"].as_u64().unwrap() > 0);

    std::fs::remove_file(&provider_manifest.path).unwrap();
    let AutomationOperation::GenerateCharacterPack(mut retry_request) = plan.operation.clone()
    else {
        panic!("expected generated character operation");
    };
    retry_request.reuse_from_job_dir = Some(completed.job_dir.clone());
    retry_request.retry_animations = vec!["walk_right".into()];
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry_request))
        .unwrap();
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let retried = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&provider as &dyn MediaGenerationProvider),
    )
    .unwrap();
    assert_eq!(retried.lifecycle_state, JobLifecycleState::Succeeded);
    assert!(retried
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "reused_video_idle"));
    assert!(retried
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "provider_manifest"));

    let AutomationOperation::GenerateCharacterPack(mut loop_retry_request) = plan.operation.clone()
    else {
        panic!("expected generated character operation");
    };
    loop_retry_request.reuse_from_job_dir = Some(retried.job_dir.clone());
    loop_retry_request.retry_animations = vec!["walk_right".into()];
    loop_retry_request
        .retry_stages
        .insert("walk_right".into(), CharacterRetryStage::Loop);
    let loop_retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            loop_retry_request,
        ))
        .unwrap();
    let loop_retry_plan = plans.claim(&loop_retry_plan.token).unwrap();
    let loop_retry_job = stage_plan_job(&jobs, &loop_retry_plan).unwrap();
    let loop_retried = run_operation_with_provider(
        &jobs,
        &loop_retry_job.job_id,
        &loop_retry_plan.operation,
        Some(&provider as &dyn MediaGenerationProvider),
    )
    .unwrap();
    assert_eq!(loop_retried.lifecycle_state, JobLifecycleState::Succeeded);
    let loop_manifest_path = loop_retried
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "provider_manifest")
        .unwrap();
    let loop_manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&loop_manifest_path.path).unwrap()).unwrap();
    assert_eq!(loop_manifest["usage"]["requests"], 0);
    assert_eq!(
        loop_manifest["animations"]["walk_right"]["retryMethod"],
        "loop_reprocess"
    );
    assert!(loop_retried
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "workflow_stage_manifest"));

    let AutomationOperation::GenerateCharacterPack(mut video_retry_request) =
        plan.operation.clone()
    else {
        panic!("expected generated character operation");
    };
    video_retry_request.reuse_from_job_dir = Some(loop_retried.job_dir.clone());
    video_retry_request.retry_animations = vec!["walk_right".into()];
    video_retry_request
        .retry_stages
        .insert("walk_right".into(), CharacterRetryStage::Video);
    let video_retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            video_retry_request,
        ))
        .unwrap();
    let video_retry_plan = plans.claim(&video_retry_plan.token).unwrap();
    let video_retry_job = stage_plan_job(&jobs, &video_retry_plan).unwrap();
    let video_retried = run_operation_with_provider(
        &jobs,
        &video_retry_job.job_id,
        &video_retry_plan.operation,
        Some(&provider as &dyn MediaGenerationProvider),
    )
    .unwrap();
    assert_eq!(video_retried.lifecycle_state, JobLifecycleState::Succeeded);
    let video_manifest_path = video_retried
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "provider_manifest")
        .unwrap();
    let video_manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&video_manifest_path.path).unwrap()).unwrap();
    assert_eq!(video_manifest["usage"]["generatedImages"], 0);
    assert_eq!(video_manifest["usage"]["editedVideos"], 1);
    assert_eq!(
        video_manifest["animations"]["walk_right"]["retryMethod"],
        "video_edit"
    );

    let AutomationOperation::GenerateCharacterPack(mut fallback_request) = plan.operation.clone()
    else {
        panic!("expected generated character operation");
    };
    fallback_request.reuse_from_job_dir = Some(loop_retried.job_dir.clone());
    fallback_request.retry_animations = vec!["walk_right".into()];
    fallback_request
        .retry_stages
        .insert("walk_right".into(), CharacterRetryStage::Video);
    let fallback_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(fallback_request))
        .unwrap();
    let fallback_plan = plans.claim(&fallback_plan.token).unwrap();
    let fallback_job = stage_plan_job(&jobs, &fallback_plan).unwrap();
    let fallback_provider = FixtureProvider::default().without_video_edit();
    let fallback_result = run_operation_with_provider(
        &jobs,
        &fallback_job.job_id,
        &fallback_plan.operation,
        Some(&fallback_provider as &dyn MediaGenerationProvider),
    )
    .unwrap();
    assert_eq!(
        fallback_result.lifecycle_state,
        JobLifecycleState::Succeeded
    );
    let fallback_manifest_path = fallback_result
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "provider_manifest")
        .unwrap();
    let fallback_manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fallback_manifest_path.path).unwrap()).unwrap();
    assert_eq!(fallback_manifest["usage"]["generatedImages"], 0);
    assert_eq!(fallback_manifest["usage"]["editedVideos"], 0);
    assert_eq!(fallback_manifest["usage"]["generatedVideos"], 1);
    assert_eq!(
        fallback_manifest["animations"]["walk_right"]["retryMethod"],
        "image_to_video_fallback"
    );

    let forgepack: serde_json::Value =
        serde_json::from_slice(&std::fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(
        forgepack["source"]["metadata"]["automationSchemaVersion"],
        "3"
    );
    assert_eq!(forgepack["source"]["metadata"]["provider"], "fixture");

    let Some(godot) = locate_godot() else {
        eprintln!("skipping Godot installation portion because Godot 4 is unavailable");
        return;
    };
    let project = temp.path().join("godot-project");
    std::fs::create_dir(&project).unwrap();
    std::fs::write(
        project.join("project.godot"),
        "[application]\nconfig/name=\"Forge Provider Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
    )
    .unwrap();
    let install = AutomationOperation::InstallGodot(GodotInstallRequest {
        schema_version: "1".into(),
        pack_path: pack.path.clone(),
        project_path: project.clone(),
        catalog_project_path: None,
        target: PathBuf::from("addons/forge_assets/fixture_hero"),
        asset_key: Some("fixture_hero".into()),
        provider_refs: vec![ProviderAssetRef {
            provider: "fixture".into(),
            asset_id: Some("fixture-character-contract".into()),
            label: Some("Fixture Hero".into()),
        }],
    });
    let install_plan = plans.prepare(install).unwrap();
    let install_plan = plans.claim(&install_plan.token).unwrap();
    let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
    let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
    assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
    let usage_path = project.join("addons/forge_assets/fixture_hero/forge_usage.json");
    let usage: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&usage_path).unwrap()).unwrap();
    assert_eq!(
        usage["directionalPlayback"]["left"]["animation"],
        "walk_right"
    );
    assert_eq!(usage["directionalPlayback"]["left"]["flipH"], true);
    assert_eq!(usage["providerProvenance"][0]["provider"], "fixture");
    let frames_resource = project.join("addons/forge_assets/fixture_hero/forge_sprite_frames.tres");
    let scene_resource =
        project.join("addons/forge_assets/fixture_hero/forge_animated_sprite.tscn");
    for resource in [&frames_resource, &scene_resource] {
        assert!(std::fs::metadata(resource).unwrap().len() < 1024 * 1024);
        let text = std::fs::read_to_string(resource).unwrap();
        assert!(!text.contains("PackedByteArray"));
        assert!(!text.contains("sub_resource type=\"Image\""));
    }

    let output = Command::new(godot)
        .args(["--headless", "--editor", "--quit", "--path"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "Godot project load failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fixture_bad_loops_are_repaired_by_video_edit_without_regenerating_stills() {
    if forge_core::video::ffmpeg::find_in_path("ffmpeg").is_none()
        || forge_core::video::ffmpeg::find_in_path("ffprobe").is_none()
    {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let profile = automation_profile();
    let operation = AutomationOperation::GenerateCharacterPack(GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: None,
        asset_id: None,
        character: GeneratedCharacterSpec {
            prompt: "A compact red adventurer with a blue hood".into(),
            reference_image_path: None,
        },
        style_lock_path: None,
        subject_lock_path: None,
        reuse_from_job_dir: None,
        retry_animations: vec![],
        retry_stages: Default::default(),
        retry_frames: Default::default(),
        metadata: CharacterPackMetadata {
            name: "Repaired Fixture Hero".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "private".into(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown".into(),
            version: "1.0.0".into(),
        },
        generation: GenerationPolicy::default(),
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    });
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let provider = FixtureProvider::default().with_bad_loop_before_edit();
    let completed = run_operation_with_provider(
        &jobs,
        &job.job_id,
        &claimed.operation,
        Some(&provider as &dyn MediaGenerationProvider),
    )
    .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let manifest_path = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "provider_manifest")
        .unwrap();
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path.path).unwrap()).unwrap();
    assert_eq!(manifest["usage"]["generatedImages"], 5);
    assert_eq!(manifest["usage"]["editedVideos"], 4);
    for name in ["idle", "walk_up", "walk_right", "walk_down"] {
        assert_eq!(manifest["animations"][name]["stillAttempt"], 1);
        assert_eq!(manifest["animations"][name]["videoAttempt"], 2);
        assert_eq!(manifest["animations"][name]["retryMethod"], "video_edit");
    }
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

#[test]
fn fixture_style_lock_gates_character_identity_before_pack_export() {
    if forge_core::video::ffmpeg::find_in_path("ffmpeg").is_none()
        || forge_core::video::ffmpeg::find_in_path("ffprobe").is_none()
    {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("assets");
    init_project(&project_root, "Styled Game").unwrap();
    let mut project: ForgeProjectV1 = read_project(&project_root).unwrap();
    project.provider.id = "fixture".into();
    std::fs::write(
        project_root.join(FORGE_PROJECT_FILE),
        serde_json::to_vec_pretty(&project).unwrap(),
    )
    .unwrap();
    let spec_path = project_root.join("specs/style.json");
    std::fs::write(
        &spec_path,
        serde_json::to_vec_pretty(&StyleSpecV1 {
            schema_version: "1".into(),
            prompt: "compact jewel-tone sprite art".into(),
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
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let provider = FixtureProvider::default();
    let prepared = plans
        .prepare(AutomationOperation::CreateStyleLock(
            CreateStyleLockRequest {
                schema_version: "1".into(),
                project_path: project_root.clone(),
                spec_path,
                provider_id: "fixture".into(),
                profile_id: "default".into(),
            },
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider)).unwrap();
    let revision = read_project(&project_root)
        .unwrap()
        .current_style_revision
        .unwrap();
    let style_lock = project_root
        .join(".forge/styles")
        .join(revision)
        .join(STYLE_LOCK_FILE);
    let profile = automation_profile();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            GenerateCharacterPackRequest {
                schema_version: "3".into(),
                provider_id: "fixture".into(),
                profile_id: "default".into(),
                project_path: Some(project_root.clone()),
                asset_id: Some("styled-fixture-hero".into()),
                character: GeneratedCharacterSpec {
                    prompt: "a compact red adventurer with a blue hood".into(),
                    reference_image_path: None,
                },
                style_lock_path: Some(style_lock),
                subject_lock_path: None,
                reuse_from_job_dir: None,
                retry_animations: vec![],
                retry_stages: Default::default(),
                retry_frames: Default::default(),
                metadata: CharacterPackMetadata {
                    name: "Styled Fixture Hero".into(),
                    default_animation: "idle".into(),
                    creator: "Game Sprite Forge".into(),
                    license: "private".into(),
                },
                workflow: CharacterWorkflowSelection {
                    id: "topdown".into(),
                    version: "1.0.0".into(),
                },
                generation: GenerationPolicy::default(),
                normalize: profile.normalize,
                sheet: profile.sheet,
                quality: QualityPolicy::default(),
            },
        ))
        .unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let result =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    if result.lifecycle_state != JobLifecycleState::Succeeded {
        eprintln!(
            "{}",
            std::fs::read_to_string(result.job_dir.join("consistency-report.json")).unwrap()
        );
    }
    assert_eq!(result.lifecycle_state, JobLifecycleState::Succeeded);
    let pack = result
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap();
    let forgepack: serde_json::Value =
        serde_json::from_slice(&std::fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    assert_eq!(forgepack["schemaVersion"], "2.0.0");
    assert_eq!(forgepack["assetType"], "character");
    assert!(pack.path.join("consistency-report.json").is_file());
}
