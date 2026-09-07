use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use forge_core::asset_project::{
    init_project, read_project, SamplingMode, StyleSpecV1, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use forge_core::automation::{
    automation_profile, run_operation, run_operation_with_provider, stage_plan_job,
    AutomationOperation, CharacterPackMetadata, CharacterRetryStage, CharacterWorkflowSelection,
    CreateStyleLockRequest, CreateSubjectLockRequest, GenerateCharacterPackRequest,
    GeneratedCharacterSpec, GenerationPolicy, GodotInstallRequest, PlanStore, QualityPolicy,
};
use forge_core::character_camera::CharacterCameraProfileV1;
use forge_core::character_direction::{DirectionLockV1, DirectionViewV1};
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::project::ProviderAssetRef;
use forge_core::provider::{EditImageRequest, MediaGenerationProvider, ReferenceRole};
use forge_core::subject::{read_subject_lock, SubjectSpecV1};
use forge_providers::fixture::FixtureProvider;

#[test]
fn fixture_locked_frame_repairs_preserve_sheet_pose_phases() {
    let temp = tempfile::tempdir().unwrap();
    let provider = FixtureProvider::default();
    for (action, sheet_direction, repair_direction) in [
        ("idle", "front/down DirectionAnchor", "front/down view"),
        (
            "walk_up",
            "rear/up DirectionAnchor",
            "rear/up view with no visible face",
        ),
        (
            "walk_right",
            "strict right-profile DirectionAnchor",
            "strict right profile moving to screen right",
        ),
        ("walk_down", "front/down DirectionAnchor", "front/down view"),
    ] {
        let sheet_path = temp.path().join(format!("{action}-sheet.png"));
        let mut request = EditImageRequest {
            prompt: format!(
                "Forge locked animation frames 2x2. Action {action}; {sheet_direction}."
            ),
            model: None,
            references: vec![],
            aspect_ratio: "1:1".into(),
            resolution: "1k".into(),
            authorization_target: Some(action.into()),
        };
        provider.edit_image(&request, &sheet_path).unwrap();
        let sheet = image::open(&sheet_path).unwrap().to_rgba8();
        assert_eq!(sheet.dimensions(), (192, 192));
        let frames = (0..4)
            .map(|frame| {
                image::imageops::crop_imm(&sheet, (frame % 2) * 96, (frame / 2) * 96, 96, 96)
                    .to_image()
            })
            .collect::<Vec<_>>();
        assert!(
            frames[0] != frames[if action == "idle" { 1 } else { 2 }],
            "{action} must preserve visible motion between authored phases"
        );
        for (frame, expected) in frames.iter().enumerate() {
            request.prompt = format!(
                "Forge locked frame repair. Forge frame phase {frame}/4. Action {action}; {repair_direction}."
            );
            request.authorization_target = Some(format!("{action}:frame:{frame}"));
            let repaired_path = temp.path().join(format!("{action}-{frame}.png"));
            provider.edit_image(&request, &repaired_path).unwrap();
            let repaired = image::open(&repaired_path).unwrap().to_rgba8();
            assert!(
                expected == &repaired,
                "{action} phase {frame} must use the same pose in sheet generation and frame repair"
            );
        }
    }
    assert_eq!(provider.usage().requests, 20);
}

#[test]
fn fixture_locked_frames_builds_direction_lock_action_timeline_and_targeted_retry() {
    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "Locked Frames Contract").unwrap();
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
            id: "locked-ranger".into(),
            name: "Locked Ranger".into(),
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
        .join(".forge/subjects/locked-ranger")
        .read_dir()
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("subject-lock.json");
    let subject = read_subject_lock(&subject_lock_path).unwrap();
    let profile = automation_profile();
    let request = GenerateCharacterPackRequest {
        schema_version: "3".into(),
        provider_id: "fixture".into(),
        profile_id: "default".into(),
        project_path: Some(project_root.clone()),
        asset_id: Some("locked-ranger-character".into()),
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
            name: "Locked Ranger Character".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-frames".into(),
            version: "4.0.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 4,
            video_duration_seconds: 4,
            image_model: Some("fixture-locked-frames".into()),
            video_model: None,
            pose_guidance: Default::default(),
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    };
    let request_template = request.clone();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 5);
    assert_eq!(prepared.estimate.maximum_provider_requests, 10);
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "error={:?} summary={:?} direction={} sheets={} stages={} consistency={} quality={} motion={} silhouette={} source_silhouette={}",
        completed.error_code,
        completed.error_summary,
        read_or_missing(completed.job_dir.join("direction-sheet-reports.json")),
        read_or_missing(completed.job_dir.join("animation-sheet-reports.json")),
        read_or_missing(
            completed
                .job_dir
                .join("source/animation-sheet-stage-manifest.json")
        ),
        read_or_missing(completed.job_dir.join("consistency-report.json")),
        read_or_missing(completed.job_dir.join("animation-quality-report.json")),
        read_or_missing(
            completed
                .job_dir
                .join("character-motion-semantics-report.json")
        ),
        read_or_missing(
            completed
                .job_dir
                .join("character-silhouette-temporal-report.json")
        ),
        read_or_missing(
            completed
                .job_dir
                .join("character-silhouette-temporal-source-report.json")
        ),
    );
    assert_eq!(provider.usage().requests, 7); // style + subject + five Character requests
    let observations = provider.edit_observations();
    let direction = observations
        .iter()
        .find(|observation| observation.authorization_target.as_deref() == Some("direction_lock"))
        .expect("DirectionLock request");
    assert_eq!(
        direction.reference_roles,
        vec![ReferenceRole::SubjectIdentity]
    );
    let actions = observations
        .iter()
        .filter(|observation| {
            matches!(
                observation.authorization_target.as_deref(),
                Some("idle" | "walk_up" | "walk_right" | "walk_down")
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(actions.len(), 4);
    assert!(actions.iter().all(|observation| {
        observation.reference_roles == vec![ReferenceRole::DirectionAnchor]
            && !observation.reference_roles.contains(&ReferenceRole::Style)
    }));
    let direction_lock: DirectionLockV1 = serde_json::from_slice(
        &fs::read(completed.job_dir.join("source/direction-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(direction_lock.entries.len(), 4);
    assert_eq!(
        direction_lock
            .entries
            .iter()
            .map(|entry| entry.direction)
            .collect::<Vec<_>>(),
        vec![
            DirectionViewV1::Front,
            DirectionViewV1::Rear,
            DirectionViewV1::Right,
            DirectionViewV1::Left,
        ]
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("source/spritesheet-provider-manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["workflow"], "topdown-frames@4.0.0");
    assert_eq!(manifest["usage"]["requests"], 5);
    assert_eq!(
        manifest["referencePolicy"]["styleImageSentToActions"],
        false
    );
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("locked frames Character Pack");
    forge_pack::validate_pack_layout(&pack.path).unwrap();
    assert!(pack.path.join("character-direction-lock.json").is_file());
    assert!(pack.path.join("assets/direction-lock/left.png").is_file());
    let pack_text = fs::read_to_string(pack.path.join("character-direction-lock.json")).unwrap();
    assert!(!pack_text.contains(temp.path().to_string_lossy().as_ref()));

    let edits_before_locked_video = provider.edit_observations().len();
    let requests_before_locked_video = provider.usage().requests;
    let mut locked_video_request = request_template.clone();
    locked_video_request.asset_id = Some("locked-ranger-video-character".into());
    locked_video_request.workflow = CharacterWorkflowSelection {
        id: "topdown-video-locked".into(),
        version: "5.0.0".into(),
    };
    locked_video_request.camera_profile = Some(CharacterCameraProfileV1::TopdownThreeQuarter);
    locked_video_request.generation.target_frame_count = 8;
    locked_video_request.generation.video_model = Some("fixture-video".into());
    locked_video_request.reuse_from_job_dir = Some(completed.job_dir.clone());
    locked_video_request.retry_animations = vec![
        "idle".into(),
        "walk_up".into(),
        "walk_right".into(),
        "walk_down".into(),
    ];
    locked_video_request.retry_stages = locked_video_request
        .retry_animations
        .iter()
        .map(|animation| (animation.clone(), CharacterRetryStage::Video))
        .collect();
    let locked_video_template = locked_video_request.clone();
    let locked_video_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            locked_video_request,
        ))
        .unwrap();
    assert_eq!(locked_video_plan.estimate.provider_request_estimate, 4);
    assert_eq!(locked_video_plan.estimate.maximum_provider_requests, 8);
    let locked_video_plan = plans.claim(&locked_video_plan.token).unwrap();
    let locked_video_job = stage_plan_job(&jobs, &locked_video_plan).unwrap();
    let locked_video = run_operation_with_provider(
        &jobs,
        &locked_video_job.job_id,
        &locked_video_plan.operation,
        Some(&provider),
    )
    .unwrap();
    let semantic_debug = read_or_missing(
        locked_video
            .job_dir
            .join("character-semantic-quality-report.json"),
    );
    let motion_debug = read_or_missing(
        locked_video
            .job_dir
            .join("character-motion-semantics-report.json"),
    );
    let loop_debug = read_or_missing(locked_video.job_dir.join("animation-quality-report.json"));
    let consistency_debug = read_or_missing(locked_video.job_dir.join("consistency-report.json"));
    assert_eq!(
        locked_video.lifecycle_state,
        JobLifecycleState::Succeeded,
        "locked-video fixture failed: code={:?}, summary={:?}, next={:?}, job={}, semantic={}, motion={}, loop={}, consistency={}",
        locked_video.error_code,
        locked_video.error_summary,
        locked_video.next_actions,
        locked_video.job_dir.display(),
        semantic_debug,
        motion_debug,
        loop_debug,
        consistency_debug,
    );
    assert_eq!(
        provider.edit_observations().len(),
        edits_before_locked_video
    );
    assert_eq!(
        provider.usage().requests - requests_before_locked_video,
        4,
        "V5 must spend one video request per action and no image request when reusing DirectionLock"
    );
    let locked_video_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(locked_video.job_dir.join("source/provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        locked_video_manifest["workflow"],
        "topdown-video-locked@5.0.0"
    );
    assert_eq!(locked_video_manifest["usage"]["requests"], 4);
    assert!(locked_video_manifest["animations"]
        .as_object()
        .unwrap()
        .values()
        .all(|animation| animation["retryMethod"] == "direction_lock_image_to_video"));
    let upgraded_lock: DirectionLockV1 = serde_json::from_slice(
        &fs::read(locked_video.job_dir.join("source/direction-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(upgraded_lock.workflow, "topdown-video-locked@5.0.0");
    let anchor = image::open(
        locked_video
            .job_dir
            .join("source/provider/walk_up/attempt-1/direction.png"),
    )
    .unwrap()
    .to_rgba8();
    assert_eq!(anchor.get_pixel(0, 0).0, [0, 255, 0, 255]);
    let action_consistency_bytes =
        fs::read(locked_video.job_dir.join("consistency-report.json")).unwrap();
    let action_consistency: serde_json::Value =
        serde_json::from_slice(&action_consistency_bytes).unwrap();
    assert_eq!(action_consistency["profile"], "consistency@1.7.0");
    assert_eq!(action_consistency["items"].as_array().unwrap().len(), 4);
    assert!(action_consistency["items"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| {
            item["reasons"]
                .as_array()
                .unwrap()
                .iter()
                .any(|reason| reason == "action_frame_consistency")
                && item["semanticAnchorSimilarity"].is_number()
                && item["semanticAnchorGeometrySimilarity"].is_number()
        }));
    let locked_video_pack = locked_video
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("locked-video Character Pack");
    let locked_video_pack_path = locked_video_pack.path.clone();
    forge_pack::validate_pack_layout(&locked_video_pack.path).unwrap();
    let locked_video_forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(locked_video_pack.path.join("forgepack.json")).unwrap())
            .unwrap();
    for animation in locked_video_forgepack["animations"].as_array().unwrap() {
        let name = animation["name"].as_str().unwrap();
        let expected_fps = if name == "idle" { 6.0 } else { 8.0 };
        assert_eq!(animation["fps"].as_f64(), Some(expected_fps));
        assert!(animation["frameDurationsMs"]
            .as_array()
            .is_none_or(Vec::is_empty));
    }
    let portable_locked_video_lock: DirectionLockV1 = serde_json::from_slice(
        &fs::read(locked_video_pack.path.join("character-direction-lock.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        portable_locked_video_lock.workflow,
        "topdown-video-locked@5.0.0"
    );
    assert_eq!(
        fs::read_dir(locked_video_pack.path.join("assets/frames"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(
                |entry| entry.path().extension().and_then(|value| value.to_str()) == Some("png")
            )
            .count(),
        32
    );

    let requests_before_local_replay = provider.usage().requests;
    let mut local_replay_request = locked_video_template;
    local_replay_request.asset_id = Some("locked-ranger-video-local-replay".into());
    local_replay_request.reuse_from_job_dir = Some(locked_video.job_dir.clone());
    local_replay_request.retry_animations = vec![
        "idle".into(),
        "walk_up".into(),
        "walk_right".into(),
        "walk_down".into(),
    ];
    local_replay_request.retry_stages = local_replay_request
        .retry_animations
        .iter()
        .map(|animation| (animation.clone(), CharacterRetryStage::Consistency))
        .collect();
    let local_replay_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            local_replay_request,
        ))
        .unwrap();
    assert_eq!(local_replay_plan.estimate.provider_request_estimate, 0);
    assert_eq!(local_replay_plan.estimate.maximum_provider_requests, 0);
    let local_replay_plan = plans.claim(&local_replay_plan.token).unwrap();
    let local_replay_job = stage_plan_job(&jobs, &local_replay_plan).unwrap();
    let unauthenticated_replay_provider = FixtureProvider::default().without_authentication();
    let local_replay = run_operation_with_provider(
        &jobs,
        &local_replay_job.job_id,
        &local_replay_plan.operation,
        Some(&unauthenticated_replay_provider),
    )
    .unwrap();
    assert_eq!(local_replay.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(provider.usage().requests, requests_before_local_replay);
    assert_eq!(unauthenticated_replay_provider.usage().requests, 0);
    assert_eq!(
        fs::read(local_replay.job_dir.join("consistency-report.json")).unwrap(),
        action_consistency_bytes,
        "unchanged local replay must deterministically recompute the same action-frame report"
    );
    let local_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(local_replay.job_dir.join("source/provider-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(local_manifest["usage"]["requests"], 0);
    assert!(local_manifest["animations"]
        .as_object()
        .unwrap()
        .values()
        .all(|animation| animation["retryMethod"] == "consistency_reprocess"));

    let source_hashes = (0..4)
        .map(|frame| {
            sha256(&completed.job_dir.join(format!(
                "animation-sheet-frames/walk_right/frame-{frame:02}.png"
            )))
        })
        .collect::<Vec<_>>();
    let mut blocked_direction = request_template.clone();
    blocked_direction.asset_id = Some("blocked-direction-lock".into());
    blocked_direction
        .character
        .prompt
        .push_str(" [fixture:direction_sheet_bleed]");
    blocked_direction.validation_only = true;
    blocked_direction.validation_animations = vec!["walk_right".into()];
    blocked_direction.metadata.default_animation = "walk_right".into();
    let blocked_provider = FixtureProvider::default();
    let blocked_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(
            blocked_direction,
        ))
        .unwrap();
    let blocked_plan = plans.claim(&blocked_plan.token).unwrap();
    let blocked_job = stage_plan_job(&jobs, &blocked_plan).unwrap();
    let blocked_error = run_operation_with_provider(
        &jobs,
        &blocked_job.job_id,
        &blocked_plan.operation,
        Some(&blocked_provider),
    )
    .unwrap_err();
    assert!(blocked_error
        .to_string()
        .contains("direction_lock_regeneration_required"));
    let blocked = jobs.read_record(&blocked_job.job_id).unwrap();
    assert_eq!(blocked.lifecycle_state, JobLifecycleState::Failed);
    assert_eq!(blocked_provider.usage().requests, 2);
    assert!(blocked_provider
        .edit_observations()
        .iter()
        .all(|observation| {
            observation.authorization_target.as_deref() == Some("direction_lock")
        }));
    assert!(!blocked
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack"));

    let mut retry = request_template;
    retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    retry.retry_animations = vec!["walk_right".into()];
    retry
        .retry_stages
        .insert("walk_right".into(), CharacterRetryStage::Frame);
    retry.retry_frames.insert("walk_right".into(), vec![2]);
    let retry_provider = FixtureProvider::default();
    let retry_plan = plans
        .prepare(AutomationOperation::GenerateCharacterPack(retry))
        .unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 1);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 2);
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let retried = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&retry_provider),
    )
    .unwrap();
    assert_eq!(
        retried.lifecycle_state,
        JobLifecycleState::Succeeded,
        "error={:?} summary={:?} stages={} onionSkin={} consistency={}",
        retried.error_code,
        retried.error_summary,
        read_or_missing(
            retried
                .job_dir
                .join("source/animation-sheet-stage-manifest.json")
        ),
        read_or_missing(retried.job_dir.join(
            "source/provider-animation-sheets/walk_right/frame-retry-aligned/onion-skin-report.json"
        )),
        read_or_missing(retried.job_dir.join("consistency-report.json")),
    );
    assert_eq!(retry_provider.usage().requests, 1);
    let retry_observation = retry_provider
        .edit_observations()
        .into_iter()
        .next()
        .expect("targeted frame request");
    assert_eq!(
        retry_observation.authorization_target.as_deref(),
        Some("walk_right:frame:2")
    );
    assert_eq!(
        retry_observation.reference_roles,
        vec![ReferenceRole::DirectionAnchor, ReferenceRole::StartKeyframe]
    );
    for (frame, source_hash) in source_hashes.iter().enumerate() {
        let relative_path = format!("animation-sheet-frames/walk_right/frame-{frame:02}.png");
        assert_eq!(
            source_hash,
            &sha256(&completed.job_dir.join(&relative_path)),
            "retry must leave source frame {frame} unchanged"
        );
        if frame != 2 {
            assert_eq!(
                source_hash,
                &sha256(&retried.job_dir.join(&relative_path)),
                "unselected frame {frame} must be byte-for-byte reused"
            );
        }
    }

    if let Some(godot) = locate_godot() {
        let godot_project = temp.path().join("godot-project");
        fs::create_dir(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge Locked Frames Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let locked_video_install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: locked_video_pack_path,
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/locked_video_ranger"),
            asset_key: Some("locked_video_ranger".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("locked-video-ranger-character".into()),
                label: Some("Locked Video Ranger".into()),
            }],
        });
        let locked_video_install_plan = plans.prepare(locked_video_install).unwrap();
        let locked_video_install_plan = plans.claim(&locked_video_install_plan.token).unwrap();
        let locked_video_install_job = stage_plan_job(&jobs, &locked_video_install_plan).unwrap();
        let locked_video_installed = run_operation(
            &jobs,
            &locked_video_install_job.job_id,
            &locked_video_install_plan.operation,
        )
        .unwrap();
        assert_eq!(
            locked_video_installed.lifecycle_state,
            JobLifecycleState::Succeeded
        );
        let locked_video_target = godot_project.join("addons/forge_assets/locked_video_ranger");
        let locked_video_frames =
            fs::read_to_string(locked_video_target.join("forge_sprite_frames.tres")).unwrap();
        for animation in ["idle", "walk_up", "walk_right", "walk_down"] {
            assert!(locked_video_frames.contains(animation));
        }
        assert!(!locked_video_frames.contains("PackedByteArray"));
        assert!(
            fs::metadata(locked_video_target.join("forge_sprite_frames.tres"))
                .unwrap()
                .len()
                < 1024 * 1024
        );
        let locked_video_usage: serde_json::Value = serde_json::from_slice(
            &fs::read(locked_video_target.join("forge_usage.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            locked_video_usage["characterDirectionLock"]["profile"],
            "direction-lock@1.0.0"
        );
        let retried_pack = retried
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "gsfpack")
            .unwrap();
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: retried_pack.path.clone(),
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/locked_ranger"),
            asset_key: Some("locked_ranger".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("locked-ranger-character".into()),
                label: Some("Locked Ranger".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let target = godot_project.join("addons/forge_assets/locked_ranger");
        let frames_text = fs::read_to_string(target.join("forge_sprite_frames.tres")).unwrap();
        assert!(frames_text.contains("idle"));
        assert!(frames_text.contains("walk_up"));
        assert!(frames_text.contains("walk_right"));
        assert!(frames_text.contains("walk_down"));
        assert!(!frames_text.contains("PackedByteArray"));
        assert!(
            fs::metadata(target.join("forge_sprite_frames.tres"))
                .unwrap()
                .len()
                < 1024 * 1024
        );
        let output = Command::new(godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
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

fn read_or_missing(path: PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| "missing".into())
}

fn sha256(path: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
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
