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
use forge_core::job::{JobLifecycleState, JobStore};
use forge_core::project::ProviderAssetRef;
use forge_core::provider::{
    GenerateVideoRequest, MediaGenerationProvider, ProviderPoll, VideoGenerationMode,
};
use forge_core::subject::{read_subject_lock, SubjectSpecV1};
use forge_providers::fixture::{FixtureProvider, FixtureRequestKind};
use image::{ImageBuffer, Rgba};

const V6_MARKER: &str = "Forge topdown video cycle v6";
const V7_MARKER: &str = "Forge topdown direction pose v7";

#[test]
fn fixture_v6_marker_records_a_720p_generation_master_request() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("generation-master.png");
    ImageBuffer::from_pixel(512, 512, Rgba([0_u8, 255, 0, 255]))
        .save(&input)
        .unwrap();
    let provider = FixtureProvider::default();
    let ticket = provider
        .generate_video(&GenerateVideoRequest {
            prompt: format!("{V6_MARKER}. Perform three identical ordinary in-place walk cycles."),
            model: Some("fixture-video".into()),
            mode: VideoGenerationMode::ImageToVideo {
                image: input.clone(),
            },
            duration_seconds: 4,
            aspect_ratio: "1:1".into(),
            resolution: "720p".into(),
            authorization_target: Some("walk_right".into()),
        })
        .unwrap();
    let output = temp.path().join("animation.mp4");
    let ProviderPoll::Succeeded(media) = provider.poll(&ticket, &output).unwrap() else {
        panic!("fixture video must complete immediately");
    };

    let observation = provider.request_observations().pop().unwrap();
    assert_eq!(observation.kind, FixtureRequestKind::GenerateVideo);
    assert!(observation.prompt.contains(V6_MARKER));
    assert_eq!(observation.resolution.as_deref(), Some("720p"));
    assert_eq!(
        observation.video_input_path.as_deref(),
        Some(input.as_path())
    );
    assert_eq!(observation.video_input_dimensions, Some((512, 512)));

    let file = fs::File::open(media.path).unwrap();
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = decoder.read_info(file).unwrap();
    let mut frames = 0;
    while reader.read_next_frame().unwrap().is_some() {
        frames += 1;
    }
    assert_eq!(
        frames, 36,
        "V6 fixture emits three 12-sample source cycles with exact contact and passing phases"
    );
}

#[test]
fn fixture_v7_full_generation_uses_direction_and_pose_edits_before_three_videos() {
    if forge_core::video::ffmpeg::find_in_path("ffmpeg").is_none()
        || forge_core::video::ffmpeg::find_in_path("ffprobe").is_none()
    {
        eprintln!("skipping provider contract test because ffmpeg/ffprobe are unavailable");
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "V7 Direction Pose Contract").unwrap();
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
            id: "pose-ranger".into(),
            name: "Pose Ranger".into(),
            prompt: "compact purple full-body ranger".into(),
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
        .join(".forge/subjects/pose-ranger")
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
        asset_id: Some("pose-ranger-character".into()),
        character: GeneratedCharacterSpec {
            prompt: subject.prompt,
            reference_image_path: Some(subject.canonical_path),
        },
        camera_profile: Some(CharacterCameraProfileV1::TopdownThreeQuarter),
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
            name: "Pose Ranger Character".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-direction-poses".into(),
            version: "7.0.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 8,
            video_duration_seconds: 4,
            image_model: Some("fixture-direction-pose".into()),
            video_model: Some("fixture-video".into()),
            pose_guidance: Default::default(),
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    };
    let observations_before = provider.request_observations().len();
    let prepared = plans
        .prepare(AutomationOperation::GenerateCharacterPack(request))
        .unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 8);
    assert_eq!(prepared.estimate.maximum_provider_requests, 16);
    let claimed = plans.claim(&prepared.token).unwrap();
    let mut validation = claimed.operation.clone();
    let AutomationOperation::GenerateCharacterPack(ref mut validation_request) = validation else {
        unreachable!();
    };
    validation_request.asset_id = Some("pose-ranger-walk-right-validation".into());
    validation_request.validation_only = true;
    validation_request.validation_animations = vec!["walk_right".into()];
    validation_request.metadata.default_animation = "walk_right".into();
    let validation_plan = plans.prepare(validation).unwrap();
    assert_eq!(validation_plan.estimate.provider_request_estimate, 3);
    assert_eq!(validation_plan.estimate.maximum_provider_requests, 6);
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let completed =
        run_operation_with_provider(&jobs, &job.job_id, &claimed.operation, Some(&provider))
            .unwrap();
    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        for report in [
            "character-semantic-quality-report.json",
            "character-motion-semantics-report.json",
            "character-gait-cycle-report.json",
            "animation-quality-report.json",
        ] {
            let path = completed.job_dir.join(report);
            if let Ok(contents) = fs::read_to_string(&path) {
                eprintln!("V7 {report}:\n{contents}");
            }
        }
    }
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "V7 job failed: code={:?}, summary={:?}, dir={}",
        completed.error_code,
        completed.error_summary,
        completed.job_dir.display()
    );
    let semantic: serde_json::Value = serde_json::from_slice(
        &fs::read(
            completed
                .job_dir
                .join("character-semantic-quality-report.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let idle_semantic = semantic["animations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|animation| animation["name"] == "idle")
        .unwrap();
    assert_eq!(idle_semantic["verdict"], "game_ready");
    assert_eq!(idle_semantic["visibleFaceFrames"], 1);
    assert_eq!(idle_semantic["visibleFaceRatio"], 1.0);
    assert_eq!(idle_semantic["directionMatchRatio"], 1.0);
    assert!(idle_semantic["reasons"].as_array().unwrap().is_empty());
    assert!(completed
        .job_dir
        .join("source/direction-lock.json")
        .is_file());
    assert!(completed
        .job_dir
        .join("source/direction-pose-lock.json")
        .is_file());
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("V7 fixture generation exports a Pack");
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    let idle = forgepack["animations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|animation| animation["name"] == "idle")
        .unwrap();
    assert_eq!(idle["frames"].as_array().map(Vec::len), Some(1));
    for name in ["walk_up", "walk_right", "walk_down"] {
        let animation = forgepack["animations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|animation| animation["name"] == name)
            .unwrap();
        assert_eq!(animation["frames"].as_array().map(Vec::len), Some(8));
    }
    assert_eq!(
        forgepack["assets"]["characterDirectionPoseLock"],
        "character-direction-pose-lock.json"
    );

    let godot = PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot");
    if godot.is_file() {
        let godot_project = temp.path().join("godot-v7");
        fs::create_dir_all(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge V7 Direction Pose Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack.path.clone(),
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/pose_ranger"),
            asset_key: Some("pose_ranger".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("pose-ranger-character".into()),
                label: Some("Pose Ranger".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let target = godot_project.join("addons/forge_assets/pose_ranger");
        let frames_text = fs::read_to_string(target.join("forge_sprite_frames.tres")).unwrap();
        for animation in ["idle", "walk_up", "walk_right", "walk_down"] {
            assert!(frames_text.contains(animation));
        }
        assert!(!frames_text.contains("PackedByteArray"));
        let output = Command::new(&godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot V7 load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let generation = &provider.request_observations()[observations_before..];
    let edits = generation
        .iter()
        .filter(|request| request.kind == FixtureRequestKind::EditImage)
        .collect::<Vec<_>>();
    assert_eq!(edits.len(), 5);
    assert_eq!(
        edits
            .iter()
            .filter(|request| request.prompt.contains("direction anchor"))
            .count(),
        2
    );
    assert_eq!(
        edits
            .iter()
            .filter(|request| request.prompt.contains("mid-walk"))
            .count(),
        3
    );
    let videos = generation
        .iter()
        .filter(|request| request.kind == FixtureRequestKind::GenerateVideo)
        .collect::<Vec<_>>();
    assert_eq!(videos.len(), 3);
    assert!(videos.iter().all(|request| {
        request.prompt.contains(V7_MARKER)
            && request.resolution.as_deref() == Some("720p")
            && request
                .video_input_dimensions
                .is_some_and(|(width, height)| width >= 512 && height >= 512)
    }));

    let mut local_retry = claimed.operation.clone();
    let AutomationOperation::GenerateCharacterPack(ref mut retry) = local_retry else {
        unreachable!();
    };
    retry.asset_id = Some("pose-ranger-local-retry".into());
    retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    retry.retry_animations = vec!["walk_right".into()];
    retry.retry_stages = BTreeMap::from([("walk_right".into(), CharacterRetryStage::Consistency)]);
    let retry_plan = plans.prepare(local_retry).unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 0);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 0);
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let local_provider = FixtureProvider::default().without_authentication();
    let replay = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&local_provider),
    )
    .unwrap();
    assert_eq!(replay.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(local_provider.usage().requests, 0);
}

#[test]
fn fixture_v6_full_generation_uses_one_direction_lock_and_four_master_videos() {
    if forge_core::video::ffmpeg::find_in_path("ffmpeg").is_none()
        || forge_core::video::ffmpeg::find_in_path("ffprobe").is_none()
    {
        eprintln!("skipping provider contract test because ffmpeg/ffprobe are unavailable");
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let project_root = temp.path().join("project");
    let mut project = init_project(&project_root, "V6 Cycle Contract").unwrap();
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
            id: "cycle-ranger".into(),
            name: "Cycle Ranger".into(),
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
        .join(".forge/subjects/cycle-ranger")
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
        asset_id: Some("cycle-ranger-character".into()),
        character: GeneratedCharacterSpec {
            prompt: subject.prompt,
            reference_image_path: Some(subject.canonical_path),
        },
        camera_profile: Some(CharacterCameraProfileV1::TopdownThreeQuarter),
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
            name: "Cycle Ranger Character".into(),
            default_animation: "idle".into(),
            creator: "Game Sprite Forge".into(),
            license: "MIT".into(),
            rendering: Default::default(),
        },
        workflow: CharacterWorkflowSelection {
            id: "topdown-video-cycle".into(),
            version: "6.0.0".into(),
        },
        generation: GenerationPolicy {
            max_attempts_per_animation: 2,
            target_frame_count: 8,
            video_duration_seconds: 4,
            image_model: Some("fixture-video-cycle".into()),
            video_model: Some("fixture-video".into()),
            pose_guidance: Default::default(),
        },
        normalize: profile.normalize,
        sheet: profile.sheet,
        quality: QualityPolicy::default(),
    };
    let observations_before = provider.request_observations().len();
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
    if completed.lifecycle_state != JobLifecycleState::Succeeded {
        for report in [
            "character-gait-cycle-report.json",
            "character-motion-semantics-report.json",
            "character-silhouette-temporal-report.json",
            "animation-quality-report.json",
        ] {
            let path = completed.job_dir.join(report);
            if let Ok(contents) = fs::read_to_string(&path) {
                eprintln!("{report}:\n{contents}");
            }
        }
    }
    assert_eq!(
        completed.lifecycle_state,
        JobLifecycleState::Succeeded,
        "job {} failed: code={:?}, summary={:?}, dir={}",
        completed.job_id,
        completed.error_code,
        completed.error_summary,
        completed.job_dir.display()
    );

    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .expect("V6 fixture generation exports a Pack");
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.path.join("forgepack.json")).unwrap()).unwrap();
    let animations = forgepack["animations"].as_array().unwrap();
    for animation in animations {
        let name = animation["name"].as_str().unwrap();
        let durations = animation["frameDurationsMs"].as_array().unwrap();
        assert_eq!(durations.len(), 8);
        let expected = if name == "idle" { 200 } else { 100 };
        assert!(durations
            .iter()
            .all(|duration| duration.as_u64() == Some(expected)));
        assert_eq!(
            animation["fps"].as_f64(),
            Some(if name == "idle" { 5.0 } else { 10.0 })
        );
    }
    assert_eq!(
        forgepack["assets"]["characterGaitCycleReport"],
        "character-gait-cycle-report.json"
    );

    let debug_manifest_path = completed.job_dir.join("previews/debug/manifest.json");
    let debug_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&debug_manifest_path).unwrap()).unwrap();
    let checkerboard_previews = debug_manifest["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["background"] == "checkerboard" && entry["kind"] == "playback")
        .collect::<Vec<_>>();
    assert_eq!(checkerboard_previews.len(), 4);
    for entry in checkerboard_previews {
        let animation = entry["animation"].as_str().unwrap();
        let expected_duration = expected_playback_duration_ms(animation);
        assert_eq!(
            entry["playbackDurationMs"].as_u64(),
            Some(expected_duration)
        );
        let manifest_frame_duration = entry["frameDurationsMs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|duration| duration.as_u64().unwrap())
            .sum::<u64>();
        assert_eq!(manifest_frame_duration, expected_duration);
        let gif_path = debug_manifest_path
            .parent()
            .unwrap()
            .join(entry["path"].as_str().unwrap());
        let decoded_duration = decoded_gif_duration_ms(&gif_path);
        assert!(
            decoded_duration.abs_diff(expected_duration) <= 10,
            "{animation} checkerboard GIF duration was {decoded_duration}ms, expected {expected_duration}ms"
        );
    }

    let godot = PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot");
    if godot.is_file() {
        let godot_project = temp.path().join("godot-project");
        fs::create_dir_all(&godot_project).unwrap();
        fs::write(
            godot_project.join("project.godot"),
            "[application]\nconfig/name=\"Forge V6 Cycle Contract\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n",
        )
        .unwrap();
        let install = AutomationOperation::InstallGodot(GodotInstallRequest {
            schema_version: "1".into(),
            pack_path: pack.path.clone(),
            project_path: godot_project.clone(),
            catalog_project_path: None,
            target: PathBuf::from("addons/forge_assets/cycle_ranger"),
            asset_key: Some("cycle_ranger".into()),
            provider_refs: vec![ProviderAssetRef {
                provider: "fixture".into(),
                asset_id: Some("cycle-ranger-character".into()),
                label: Some("Cycle Ranger".into()),
            }],
        });
        let install_plan = plans.prepare(install).unwrap();
        let install_plan = plans.claim(&install_plan.token).unwrap();
        let install_job = stage_plan_job(&jobs, &install_plan).unwrap();
        let installed = run_operation(&jobs, &install_job.job_id, &install_plan.operation).unwrap();
        assert_eq!(installed.lifecycle_state, JobLifecycleState::Succeeded);
        let target = godot_project.join("addons/forge_assets/cycle_ranger");
        let frames_text = fs::read_to_string(target.join("forge_sprite_frames.tres")).unwrap();
        for animation in ["idle", "walk_up", "walk_right", "walk_down"] {
            assert!(frames_text.contains(animation));
        }
        assert!(!frames_text.contains("PackedByteArray"));
        assert!(
            fs::metadata(target.join("forge_sprite_frames.tres"))
                .unwrap()
                .len()
                < 1024 * 1024
        );
        let output = Command::new(&godot)
            .args(["--headless", "--editor", "--quit", "--path"])
            .arg(&godot_project)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Godot V6 load failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let timing_script = godot_project.join("verify_sprite_frames_timing.gd");
        fs::write(
            &timing_script,
            r#"extends SceneTree

const EXPECTED_DURATION_MS := {
    "idle": 1600.0,
    "walk_up": 800.0,
    "walk_right": 800.0,
    "walk_down": 800.0,
}

func _init() -> void:
    var frames := load("res://addons/forge_assets/cycle_ranger/forge_sprite_frames.tres") as SpriteFrames
    if frames == null:
        printerr("missing SpriteFrames resource")
        quit(1)
        return
    for animation in EXPECTED_DURATION_MS:
        if !frames.has_animation(animation):
            printerr("missing animation: %s" % animation)
            quit(1)
            return
        var fps := frames.get_animation_speed(animation)
        if fps <= 0.0:
            printerr("invalid animation speed: %s" % animation)
            quit(1)
            return
        var duration_ms := 0.0
        for frame_index in range(frames.get_frame_count(animation)):
            duration_ms += frames.get_frame_duration(animation, frame_index) * 1000.0 / fps
        if absf(duration_ms - EXPECTED_DURATION_MS[animation]) > 10.0:
            printerr("duration mismatch for %s: %.3fms" % [animation, duration_ms])
            quit(1)
            return
    quit(0)
"#,
        )
        .unwrap();
        let timing_output = Command::new(&godot)
            .args(["--headless", "--path"])
            .arg(&godot_project)
            .args(["--script"])
            .arg(&timing_script)
            .output()
            .unwrap();
        assert!(
            timing_output.status.success(),
            "Godot SpriteFrames timing failed: {}",
            String::from_utf8_lossy(&timing_output.stderr)
        );
    }

    let observations = provider.request_observations();
    let generation = &observations[observations_before..];
    let direction_locks = generation
        .iter()
        .filter(|request| {
            request.kind == FixtureRequestKind::EditImage
                && request.authorization_target.as_deref() == Some("direction_lock")
        })
        .collect::<Vec<_>>();
    assert_eq!(direction_locks.len(), 1);
    assert!(direction_locks[0].prompt.contains(V6_MARKER));
    assert_eq!(
        generation
            .iter()
            .filter(|request| request.kind == FixtureRequestKind::EditImage)
            .count(),
        1,
        "full V6 generation must not issue SubjectLock or StyleLock image edits"
    );
    let videos = generation
        .iter()
        .filter(|request| request.kind == FixtureRequestKind::GenerateVideo)
        .collect::<Vec<_>>();
    assert_eq!(videos.len(), 4);
    assert!(videos.iter().all(|request| {
        request.prompt.contains(V6_MARKER)
            && request.resolution.as_deref() == Some("720p")
            && request
                .video_input_dimensions
                .is_some_and(|(width, height)| width >= 512 && height >= 512)
    }));
    assert!(generation.iter().all(|request| {
        !matches!(
            request.kind,
            FixtureRequestKind::GenerateImage | FixtureRequestKind::EditVideo
        )
    }));

    let requests_before_local_retry = provider.usage().requests;
    let mut local_retry = claimed.operation.clone();
    let AutomationOperation::GenerateCharacterPack(ref mut retry) = local_retry else {
        unreachable!("the claimed operation is a character pack request");
    };
    retry.asset_id = Some("cycle-ranger-local-retry".into());
    retry.reuse_from_job_dir = Some(completed.job_dir.clone());
    retry.retry_animations = vec!["walk_right".into()];
    retry.retry_stages = BTreeMap::from([("walk_right".into(), CharacterRetryStage::Consistency)]);
    let retry_plan = plans.prepare(local_retry).unwrap();
    assert_eq!(retry_plan.estimate.provider_request_estimate, 0);
    assert_eq!(retry_plan.estimate.maximum_provider_requests, 0);
    let retry_plan = plans.claim(&retry_plan.token).unwrap();
    let retry_job = stage_plan_job(&jobs, &retry_plan).unwrap();
    let local_provider = FixtureProvider::default().without_authentication();
    let local_retry = run_operation_with_provider(
        &jobs,
        &retry_job.job_id,
        &retry_plan.operation,
        Some(&local_provider),
    )
    .unwrap();
    assert_eq!(local_retry.lifecycle_state, JobLifecycleState::Succeeded);
    assert_eq!(provider.usage().requests, requests_before_local_retry);
    assert_eq!(local_provider.usage().requests, 0);
}

fn expected_playback_duration_ms(animation: &str) -> u64 {
    if animation == "idle" {
        1_600
    } else {
        800
    }
}

fn decoded_gif_duration_ms(path: &std::path::Path) -> u64 {
    let file = fs::File::open(path).unwrap();
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut reader = decoder.read_info(file).unwrap();
    let mut duration_ms = 0_u64;
    while let Some(frame) = reader.read_next_frame().unwrap() {
        duration_ms += u64::from(frame.delay) * 10;
    }
    duration_ms
}

#[test]
#[ignore = "requires a retained local real-xAI Job; never performs Provider requests"]
fn frozen_real_v5_job_replays_through_v6_without_credentials_or_media_requests() {
    let default_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
        "../../generated-assets/forge-topdown-video-locked-v5-remediation-real-20260809/jobs/605d08af-c5a3-4a32-98b7-849332dcce7d",
    );
    let source_job_dir = std::env::var("FORGE_V6_FROZEN_SOURCE_JOB")
        .map(PathBuf::from)
        .unwrap_or(default_source)
        .canonicalize()
        .expect("the retained V5 remediation Job must be available for the ignored frozen gate");
    let output_temp = std::env::var("FORGE_V6_FROZEN_OUTPUT")
        .is_err()
        .then(tempfile::tempdir)
        .transpose()
        .unwrap();
    let output_root = std::env::var("FORGE_V6_FROZEN_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| output_temp.as_ref().unwrap().path().to_path_buf());
    let expected = std::env::var("FORGE_V6_FROZEN_EXPECT").unwrap_or_else(|_| "block".into());
    fs::create_dir_all(&output_root).unwrap();

    let source_record: serde_json::Value =
        serde_json::from_slice(&fs::read(source_job_dir.join("job.json")).unwrap()).unwrap();
    let mut operation: AutomationOperation =
        serde_json::from_value(source_record["recipe"].clone()).unwrap();
    let AutomationOperation::GenerateCharacterPack(request) = &mut operation else {
        panic!("frozen source must be a Character generation Job");
    };
    request.workflow = CharacterWorkflowSelection {
        id: "topdown-video-cycle".into(),
        version: "6.0.0".into(),
    };
    request.asset_id = Some(format!(
        "{}-v6-frozen-replay",
        request.asset_id.as_deref().unwrap_or("character")
    ));
    request.reuse_from_job_dir = Some(source_job_dir.clone());
    request.retry_animations = vec![
        "idle".into(),
        "walk_up".into(),
        "walk_right".into(),
        "walk_down".into(),
    ];
    request.retry_stages = request
        .retry_animations
        .iter()
        .cloned()
        .map(|animation| (animation, CharacterRetryStage::Loop))
        .collect();
    request.retry_frames.clear();
    request.validation_only = false;
    request.validation_animations.clear();

    let plans = PlanStore::new(output_root.join("plans")).unwrap();
    let jobs = JobStore::new(output_root.join("jobs")).unwrap();
    let prepared = plans.prepare(operation).unwrap();
    assert_eq!(prepared.estimate.provider_request_estimate, 0);
    assert_eq!(prepared.estimate.maximum_provider_requests, 0);
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &claimed).unwrap();
    let provider = forge_providers::resolve_local_replay_provider("xai").unwrap();
    let completed = run_operation_with_provider(
        &jobs,
        &job.job_id,
        &claimed.operation,
        Some(provider.as_ref()),
    )
    .unwrap();
    let observed_usage = provider.usage();
    assert_eq!(observed_usage.requests, 0);
    assert_eq!(observed_usage.generated_images, 0);
    assert_eq!(observed_usage.generated_videos, 0);

    let gait_document: serde_json::Value = serde_json::from_slice(
        &fs::read(completed.job_dir.join("character-gait-cycle-report.json")).unwrap(),
    )
    .unwrap();
    let gait_verdict = |name: &str| {
        gait_document["animations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|animation| animation["animation"] == name)
            .and_then(|animation| animation["verdict"].as_str())
    };
    assert_eq!(gait_verdict("walk_right"), Some("game_ready"));
    assert_eq!(gait_verdict("walk_down"), Some("game_ready"));
    assert_ne!(gait_verdict("walk_up"), Some("game_ready"));

    let report_paths = [
        "character-gait-cycle-report.json",
        "character-motion-semantics-report.json",
        "animation-quality-report.json",
        "loop-selection-report.json",
        "source/provider-manifest.json",
    ];
    let reports = report_paths
        .into_iter()
        .filter_map(|relative| {
            let path = completed.job_dir.join(relative);
            path.is_file().then(|| {
                serde_json::json!({
                    "path": relative,
                    "sha256": forge_core::asset_project::hash_file(&path).ok(),
                })
            })
        })
        .collect::<Vec<_>>();
    let has_pack = completed
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "gsfpack");
    let summary = serde_json::json!({
        "schemaVersion": "1",
        "profile": "topdown-video-cycle-frozen-replay@1.0.0",
        "sourceJobDir": source_job_dir,
        "replayJobId": completed.job_id,
        "replayJobDir": completed.job_dir,
        "plannedProviderRequests": 0,
        "maximumProviderRequests": 0,
        "observedProviderUsage": observed_usage,
        "lifecycleState": completed.lifecycle_state,
        "errorCode": completed.error_code,
        "errorSummary": completed.error_summary,
        "hasPack": has_pack,
        "reports": reports,
    });
    fs::write(
        output_root.join("frozen-replay-summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();

    match expected.as_str() {
        "pass" => {
            assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
            assert!(has_pack);
        }
        "block" => {
            assert_eq!(completed.lifecycle_state, JobLifecycleState::Failed);
            assert!(!has_pack);
        }
        other => panic!("FORGE_V6_FROZEN_EXPECT must be pass or block, got {other}"),
    }
}

fn run(
    plans: &PlanStore,
    jobs: &JobStore,
    provider: &FixtureProvider,
    operation: AutomationOperation,
) {
    let prepared = plans.prepare(operation).unwrap();
    let claimed = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(jobs, &claimed).unwrap();
    let completed =
        run_operation_with_provider(jobs, &job.job_id, &claimed.operation, Some(provider)).unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
}
