#![cfg(unix)]

use forge_core::{
    automation::{
        run_operation, stage_plan_job, AutomationOperation, GodotInstallRequest, PlanStore,
    },
    delivery::directory_sha256,
    job::{JobLifecycleState, JobStore},
};
use image::{Rgba, RgbaImage};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

fn fixture(root: &Path) -> (JobStore, PlanStore, PathBuf, PathBuf) {
    let jobs = JobStore::new(root.join("jobs")).unwrap();
    let plans = PlanStore::new(root.join("plans")).unwrap();
    let source = root.join("source.png");
    let mut image = RgbaImage::new(64, 64);
    for y in 12..52 {
        for x in 16..48 {
            image.put_pixel(x, y, Rgba([200, 140, 40, 255]));
        }
    }
    image.save(&source).unwrap();
    let request = serde_json::from_value(json!({
        "schemaVersion":"1", "kind":"prop_set", "id":"fixture", "name":"Fixture", "license":"CC0-1.0",
        "sampling":"nearest", "canvasSize":64, "items":[{"id":"stone", "name":"Stone", "path":source}]
    })).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareStatic(request))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let complete = run_operation(&jobs, &job.job_id, &plan.operation).unwrap();
    let pack = complete
        .artifacts
        .iter()
        .find(|a| a.kind == "gsfpack")
        .unwrap()
        .path
        .clone();
    let project = root.join("game");
    fs::create_dir_all(project.join(".forge")).unwrap();
    fs::write(project.join("project.godot"), "config_version=5\n[application]\nconfig/name=\"Install transaction fixture\"\n[rendering]\nrenderer/rendering_method=\"gl_compatibility\"\n").unwrap();
    (jobs, plans, pack, project)
}

fn install_operation(pack: &Path, project: &Path) -> AutomationOperation {
    AutomationOperation::InstallGodot(GodotInstallRequest {
        schema_version: "1".into(),
        pack_path: pack.into(),
        project_path: project.into(),
        catalog_project_path: None,
        target: PathBuf::from("addons/forge_assets/fixture"),
        asset_key: Some("fixture".into()),
        provider_refs: vec![],
    })
}

fn stub(path: &Path, mode: &str) {
    let python = format!(
        r#"#!/usr/bin/env python3
import json, pathlib, sys, time
mode = {mode:?}
if '--version' in sys.argv:
    print('4.5.0.fixture' if mode == 'version' else '4.6.3.fixture')
    if mode == 'spawn': pathlib.Path(__file__).unlink()
    sys.exit(0)
if '--import' in sys.argv:
    if mode == 'cancel':
        pathlib.Path(__file__).with_suffix('.running').write_text('running')
        time.sleep(30)
    if mode == 'import_exit': sys.exit(3)
    if mode == 'import_error': print('SCRIPT ERROR: injected parse error', file=sys.stderr)
    sys.exit(0)
if mode == 'protocol': sys.exit(0)
project = pathlib.Path(sys.argv[sys.argv.index('--path')+1])
args = sys.argv[sys.argv.index('--')+1:]
target = project / args[1]
phase = 'verify' if '--verify' in args else 'install'
if mode == 'registry' and phase == 'verify':
    (project/'.forge/assets.json').write_text('injected registry failure after backup')
if mode == 'verify_error' and phase == 'verify':
    print('ERROR: injected native load error', file=sys.stderr)
(target/'scenes').mkdir(exist_ok=True)
(target/'scenes/stone.tscn').write_text('data = PackedByteArray(1,2)' if mode == 'embedded' else '[gd_scene format=3]')
print('FORGE_INSTALL_RESULT ' + json.dumps(dict(schemaVersion='1',status='succeeded',phase=phase,target='res://'+args[1],assetType='prop_set')))
"#
    );
    fs::write(path, python).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn all_install_failures_preserve_target_and_registry() {
    for mode in [
        "version",
        "spawn",
        "log_io",
        "import_exit",
        "import_error",
        "protocol",
        "verify_error",
        "embedded",
        "registry",
        "cancel",
    ] {
        let root = tempfile::tempdir().unwrap();
        let (jobs, plans, pack, project) = fixture(root.path());
        let target = project.join("addons/forge_assets/fixture");
        fs::create_dir_all(&target).unwrap();
        fs::write(
            target.join(".forge-owned.json"),
            "{\"owner\":\"Game Sprite Forge\",\"assetKey\":\"fixture\"}",
        )
        .unwrap();
        fs::write(target.join("original.png"), "approved original bytes").unwrap();
        fs::write(
            target.join("original.tscn"),
            "approved original native scene",
        )
        .unwrap();
        let manifest = project.join(".forge/assets.json");
        fs::write(
            &manifest,
            "{\"schemaVersion\":\"1\",\"updatedAt\":\"2026-09-12T00:00:00Z\",\"assets\":{}}",
        )
        .unwrap();
        let before = directory_sha256(&target).unwrap();
        let before_manifest = fs::read(&manifest).unwrap();
        let prepared = plans.prepare(install_operation(&pack, &project)).unwrap();
        let plan = plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        if mode == "log_io" {
            fs::create_dir(job.job_dir.join("logs/godot.import.stdout.log")).unwrap();
        }
        let executable = root.path().join("godot-fixture.py");
        stub(&executable, mode);
        let canceller = if mode == "cancel" {
            let store_path = jobs.root().to_path_buf();
            let job_id = job.job_id.clone();
            let signal = executable.with_extension("running");
            Some(thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10);
                while !signal.exists() {
                    assert!(Instant::now() < deadline);
                    thread::sleep(Duration::from_millis(10));
                }
                JobStore::new(store_path)
                    .unwrap()
                    .update_record(&job_id, |record| record.cancellation_requested = true)
                    .unwrap();
            }))
        } else {
            None
        };
        temp_env::with_var("FORGE_GODOT_PATH", Some(executable.as_os_str()), || {
            let result = run_operation(&jobs, &job.job_id, &plan.operation);
            if mode == "cancel" {
                assert_eq!(
                    result.unwrap().lifecycle_state,
                    JobLifecycleState::Cancelled
                );
            } else {
                assert!(result.is_err(), "{mode} should fail");
            }
        });
        if let Some(canceller) = canceller {
            canceller.join().unwrap();
        }
        assert_eq!(
            directory_sha256(&target).unwrap(),
            before,
            "{mode} changed installed bytes"
        );
        assert_eq!(
            fs::read(&manifest).unwrap(),
            before_manifest,
            "{mode} changed registry"
        );
    }
}

#[test]
fn install_project_lock_serializes_and_waiting_job_can_cancel() {
    let root = tempfile::tempdir().unwrap();
    let (jobs, plans, pack, project) = fixture(root.path());
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(project.join(".forge/install.lock"))
        .unwrap();
    lock.lock().unwrap();
    let prepared = plans.prepare(install_operation(&pack, &project)).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let executable = root.path().join("godot-fixture.py");
    stub(&executable, "protocol");
    temp_env::with_var("FORGE_GODOT_PATH", Some(executable.as_os_str()), || {
        thread::scope(|scope| {
            let worker = scope.spawn(|| run_operation(&jobs, &job.job_id, &plan.operation));
            thread::sleep(Duration::from_millis(250));
            assert!(
                !worker.is_finished(),
                "install did not wait for project lock"
            );
            assert!(!project.join("addons/forge_assets/fixture").exists());
            jobs.update_record(&job.job_id, |record| record.cancellation_requested = true)
                .unwrap();
            assert_eq!(
                worker.join().unwrap().unwrap().lifecycle_state,
                JobLifecycleState::Cancelled
            );
        });
    });
}

#[test]
#[ignore = "requires real Godot 4.6.x; validates native install, update and failed update rollback"]
fn real_godot_install_and_failed_update_preserve_approved_resources() {
    let root = tempfile::tempdir().unwrap();
    let (jobs, plans, pack, project) = fixture(root.path());
    let executable = std::env::var_os("FORGE_GODOT_PATH")
        .unwrap_or_else(|| "/Applications/Godot.app/Contents/MacOS/Godot".into());
    temp_env::with_var("FORGE_GODOT_PATH", Some(&executable), || {
        for _ in 0..2 {
            let prepared = plans.prepare(install_operation(&pack, &project)).unwrap();
            let plan = plans.claim(&prepared.token).unwrap();
            let job = stage_plan_job(&jobs, &plan).unwrap();
            let result = run_operation(&jobs, &job.job_id, &plan.operation);
            if result.is_err() {
                eprintln!("{:?}", result);
                for name in ["godot.import", "godot", "godot.verify"] {
                    eprintln!(
                        "{}",
                        fs::read_to_string(job.job_dir.join(format!("logs/{name}.stderr.log")))
                            .unwrap_or_default()
                    );
                }
            }
            assert_eq!(
                result.unwrap().lifecycle_state,
                JobLifecycleState::Succeeded
            );
            let verify =
                fs::read_to_string(job.job_dir.join("logs/godot.verify.stdout.log")).unwrap();
            assert!(verify.contains("FORGE_INSTALL_RESULT"));
            let registry: serde_json::Value =
                serde_json::from_slice(&fs::read(project.join(".forge/assets.json")).unwrap())
                    .unwrap();
            assert_eq!(
                registry["assets"]["fixture"]["installSnapshotSha256"],
                forge_core::delivery::hash_file(
                    &project.join("addons/forge_assets/fixture/.forge-install.json")
                )
                .unwrap()
            );
        }
    });
    let target = project.join("addons/forge_assets/fixture");
    let before = directory_sha256(&target).unwrap();
    let manifest = fs::read(project.join(".forge/assets.json")).unwrap();
    let broken_godot = root.path().join("unsupported.py");
    stub(&broken_godot, "version");
    let prepared = plans.prepare(install_operation(&pack, &project)).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    temp_env::with_var("FORGE_GODOT_PATH", Some(broken_godot.as_os_str()), || {
        assert!(run_operation(&jobs, &job.job_id, &plan.operation).is_err());
    });
    assert_eq!(directory_sha256(&target).unwrap(), before);
    assert_eq!(
        fs::read(project.join(".forge/assets.json")).unwrap(),
        manifest
    );
}

#[test]
#[ignore = "requires real Godot 4.6.x; validates provider-neutral V3 catalog installation"]
fn real_godot_install_with_v3_catalog() {
    let root = tempfile::tempdir().unwrap();
    let (jobs, plans, pack, project) = fixture(root.path());
    let library = root.path().join("library");
    forge_core::library::initialize(&library, "Local resources").unwrap();
    let entry = serde_json::from_value(json!({
        "assetId":"fixture","name":"Fixture","kind":"prop_set","packPath":pack,
        "packSha256":directory_sha256(&pack).unwrap(),"sourceJobId":"fixture-source",
        "workflow":"static-set@1.0.0","createdAt":"2026-01-01T00:00:00Z"
    }))
    .unwrap();
    forge_core::catalog::publish_catalog_asset(&library, entry).unwrap();
    assert!(!library.join("forge-project.json").exists());
    let mut operation = install_operation(&pack, &project);
    if let AutomationOperation::InstallGodot(request) = &mut operation {
        request.catalog_project_path = Some(library.clone());
    }
    let executable = std::env::var_os("FORGE_GODOT_PATH")
        .unwrap_or_else(|| "/Applications/Godot.app/Contents/MacOS/Godot".into());
    temp_env::with_var("FORGE_GODOT_PATH", Some(executable), || {
        let plan = plans.prepare(operation).unwrap();
        let plan = plans.claim(&plan.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        let result = run_operation(&jobs, &job.job_id, &plan.operation).unwrap();
        assert_eq!(result.lifecycle_state, JobLifecycleState::Succeeded);
    });
    let catalog = forge_core::library::read_catalog(&library).unwrap();
    let asset = forge_core::library::read_asset(&library, &catalog, "fixture").unwrap();
    assert_eq!(asset.installations.len(), 1);
    assert_eq!(asset.installations[0].revision, asset.revisions[0]);
    assert_eq!(asset.installations[0].evidence, "installation_transaction");
    forge_core::delivery::verify_install(&project, "fixture", None).unwrap();
}

#[test]
#[ignore = "requires real Godot 4.6.x; checks native animation timing and explicit failure propagation"]
fn real_godot_animation_verification_and_script_failure_protocol() {
    let root = tempfile::tempdir().unwrap();
    let (jobs, plans, _, project) = fixture(root.path());
    let source = root.path().join("source.png");
    let request = serde_json::from_value(json!({
        "schemaVersion":"1", "input":{"kind":"png_sequence","paths":[source,source]},
        "metadata":{"name":"Burst", "animation":"burst", "fps":8.0,"loop":false,"frameDurationsMs":[80,240]},
        "rendering":{"textureFilter":"linear","pixelSnap":false},
        "normalize":{"mode":"preserve_source","margin":0,"marginBottom":0,"alphaThreshold":0,
            "manualAnchor":{"x":32.5,"y":52.25,"lockedByUser":true}},
        "quality":{"requireGameReady":false}
    })).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareAsset(request))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let complete = run_operation(&jobs, &job.job_id, &plan.operation).unwrap();
    let pack = complete
        .artifacts
        .iter()
        .find(|a| a.kind == "gsfpack")
        .unwrap()
        .path
        .clone();
    let executable = std::env::var_os("FORGE_GODOT_PATH")
        .unwrap_or_else(|| "/Applications/Godot.app/Contents/MacOS/Godot".into());
    temp_env::with_var("FORGE_GODOT_PATH", Some(&executable), || {
        let prepared = plans.prepare(install_operation(&pack, &project)).unwrap();
        let plan = plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        let complete = run_operation(&jobs, &job.job_id, &plan.operation);
        if complete.is_err() {
            eprintln!(
                "{:?}\n{}",
                complete,
                fs::read_to_string(job.job_dir.join("logs/godot.verify.stderr.log"))
                    .unwrap_or_default()
            );
        }
        let complete = complete.unwrap();
        let report = complete
            .artifacts
            .iter()
            .find(|a| a.kind == "godot_install_verification")
            .unwrap();
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report.path).unwrap()).unwrap();
        assert_eq!(report["nativeLoadVerified"], true);
        assert_eq!(report["visualApproval"], false);
        let script = job.job_dir.join("tools/install_forge_pack.gd");
        let frames = project.join("addons/forge_assets/fixture/forge_sprite_frames.tres");
        let contents = fs::read_to_string(&frames).unwrap();
        fs::write(
            &frames,
            contents.replace("\"speed\": 8.0", "\"speed\": 99.0"),
        )
        .unwrap();
        assert!(fs::read_to_string(&frames)
            .unwrap()
            .contains("\"speed\": 99.0"));
        let output = std::process::Command::new(&executable)
            .arg("--headless")
            .arg("--path")
            .arg(&project)
            .arg("--script")
            .arg(&script)
            .arg("--")
            .arg(&pack)
            .arg("addons/forge_assets/fixture")
            .arg("--verify")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!String::from_utf8_lossy(&output.stdout).contains("FORGE_INSTALL_RESULT"));
        assert!(String::from_utf8_lossy(&output.stdout).contains("timing differs"));
        // A missing static texture previously called quit(1), then continued to PASS/quit(0).
        let broken_pack = root.path().join("broken.gsfpack");
        fs::create_dir_all(broken_pack.join("assets")).unwrap();
        fs::write(
            broken_pack.join("assets/godot_import.json"),
            serde_json::to_vec(&json!({"assetType":"prop_set","items":[{"id":"missing"}]}))
                .unwrap(),
        )
        .unwrap();
        let output = std::process::Command::new(&executable)
            .arg("--headless")
            .arg("--path")
            .arg(&project)
            .arg("--script")
            .arg(&script)
            .arg("--")
            .arg(&broken_pack)
            .arg("addons/forge_assets/broken")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("FAIL Forge Godot install"));
        assert!(!stdout.contains("PASS Forge"));
        assert!(!stdout.contains("FORGE_INSTALL_RESULT"));
    });
}

#[test]
fn install_targets_require_an_asset_directory_and_matching_owner_identity() {
    let root = tempfile::tempdir().unwrap();
    let (_, plans, pack, project) = fixture(root.path());
    let AutomationOperation::InstallGodot(mut request) = install_operation(&pack, &project) else {
        unreachable!()
    };
    request.target = "addons/forge_assets".into();
    assert!(plans
        .prepare(AutomationOperation::InstallGodot(request))
        .unwrap_err()
        .to_string()
        .contains("directory below"));
    let target = project.join("addons/forge_assets/fixture");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("keep.txt"), "unrelated content").unwrap();
    for marker in [
        json!({"owner":"Other tool","assetKey":"fixture"}),
        json!({"owner":"Game Sprite Forge","assetKey":"another"}),
        json!({"owner":"Game Sprite Forge"}),
    ] {
        fs::write(
            target.join(".forge-owned.json"),
            serde_json::to_vec(&marker).unwrap(),
        )
        .unwrap();
        let before = directory_sha256(&target).unwrap();
        assert!(plans.prepare(install_operation(&pack, &project)).is_err());
        assert_eq!(directory_sha256(&target).unwrap(), before);
    }
}

#[test]
fn legacy_marker_needs_registry_identity_and_other_keys_cannot_reuse_or_overlap_target() {
    use forge_core::project::{register_project_asset, RegisterProjectAsset};
    let root = tempfile::tempdir().unwrap();
    let (_, plans, pack, project) = fixture(root.path());
    let target = Path::new("addons/forge_assets/fixture");
    fs::create_dir_all(project.join(target)).unwrap();
    fs::write(
        project.join(target).join(".forge-owned.json"),
        "{\"owner\":\"Game Sprite Forge\"}",
    )
    .unwrap();
    let summary = forge_pack::inspect_pack(&pack).unwrap();
    let pack_sha256 = directory_sha256(&pack).unwrap();
    register_project_asset(RegisterProjectAsset {
        project_path: &project,
        asset_key: "fixture",
        pack_path: &pack,
        pack_sha256: &pack_sha256,
        godot_target: target,
        scene_path: &target.join("scenes"),
        sprite_frames_path: &target.join("items"),
        usage_path: &target.join("forge_usage.json"),
        pack: &summary,
        provider_refs: &[],
        job_id: "existing-install",
    })
    .unwrap();
    // A real legacy marker can be associated with exactly the same registered asset.
    plans.prepare(install_operation(&pack, &project)).unwrap();
    for path in [
        "addons/forge_assets/fixture",
        "addons/forge_assets/fixture/child",
    ] {
        let AutomationOperation::InstallGodot(mut request) = install_operation(&pack, &project)
        else {
            unreachable!()
        };
        request.asset_key = Some("another".into());
        request.target = path.into();
        assert!(plans
            .prepare(AutomationOperation::InstallGodot(request))
            .unwrap_err()
            .to_string()
            .contains("overlaps asset key fixture"));
    }
    // Registry ownership still protects a target if a stale marker claims a different key.
    fs::write(
        project.join(target).join(".forge-owned.json"),
        "{\"owner\":\"Game Sprite Forge\",\"assetKey\":\"another\"}",
    )
    .unwrap();
    let AutomationOperation::InstallGodot(mut request) = install_operation(&pack, &project) else {
        unreachable!()
    };
    request.asset_key = Some("another".into());
    assert!(plans
        .prepare(AutomationOperation::InstallGodot(request))
        .unwrap_err()
        .to_string()
        .contains("overlaps asset key fixture"));
}

#[test]
fn waiting_installer_rejects_a_plan_invalidated_by_the_previous_install() {
    let root = tempfile::tempdir().unwrap();
    let (jobs, plans, pack, project) = fixture(root.path());
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(project.join(".forge/install.lock"))
        .unwrap();
    lock.lock().unwrap();
    let prepared = plans.prepare(install_operation(&pack, &project)).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    let executable = root.path().join("godot-fixture.py");
    stub(&executable, "protocol");
    temp_env::with_var("FORGE_GODOT_PATH", Some(executable.as_os_str()), || {
        thread::scope(|scope| {
            let worker = scope.spawn(|| run_operation(&jobs, &job.job_id, &plan.operation));
            let deadline = Instant::now() + Duration::from_secs(10);
            while !job.job_dir.join("logs/godot.version.stdout.log").exists() {
                assert!(Instant::now() < deadline);
                thread::sleep(Duration::from_millis(10));
            }
            assert!(!worker.is_finished());
            // Deterministically model the lock holder publishing a completed target.
            let target = project.join("addons/forge_assets/fixture");
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join(".forge-owned.json"), "{\"owner\":\"Game Sprite Forge\",\"assetKey\":\"fixture\",\"jobId\":\"previous-install\"}").unwrap();
            fs::write(target.join("approved.txt"), "previous installation output").unwrap();
            let before = directory_sha256(&target).unwrap();
            lock.unlock().unwrap();
            let error = worker.join().unwrap().unwrap_err();
            assert!(
                error
                    .to_string()
                    .contains("input changed after plan creation"),
                "{error}"
            );
            assert_eq!(directory_sha256(&target).unwrap(), before);
            assert!(!job.job_dir.join("backups/godot-target").exists());
        });
    });
}

#[test]
#[ignore = "requires real Godot 4.6.x; protects imported cache pixels across a failed texture update"]
fn real_godot_failed_blue_update_restores_red_native_texture_without_reimport() {
    use forge_core::delivery::{hash_file, inventory};
    let root = tempfile::tempdir().unwrap();
    let (jobs, plans, _, project) = fixture(root.path());
    let source = root.path().join("color.png");
    let make_pack = |name: &str, color: [u8; 4]| {
        let mut image = RgbaImage::new(64, 64);
        for y in 8..56 {
            for x in 16..48 {
                image.put_pixel(x, y, Rgba(color));
            }
        }
        image.save(&source).unwrap();
        let request = serde_json::from_value(json!({
            "schemaVersion":"1","kind":"prop_set","id":"fixture","name":name,"license":"private",
            "sampling":"nearest","canvasSize":64,"items":[{"id":"stone","name":"Stone","path":source}]
        })).unwrap();
        let prepared = plans
            .prepare(AutomationOperation::PrepareStatic(request))
            .unwrap();
        let plan = plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        let completed = run_operation(&jobs, &job.job_id, &plan.operation).unwrap();
        completed
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "gsfpack")
            .unwrap()
            .path
            .clone()
    };
    let red = make_pack("Red", [240, 40, 20, 255]);
    let blue = make_pack("Blue", [20, 40, 240, 255]);
    let godot = std::env::var_os("FORGE_GODOT_PATH")
        .unwrap_or_else(|| "/Applications/Godot.app/Contents/MacOS/Godot".into());
    temp_env::with_var("FORGE_GODOT_PATH", Some(&godot), || {
        let prepared = plans.prepare(install_operation(&red, &project)).unwrap();
        let plan = plans.claim(&prepared.token).unwrap();
        let job = stage_plan_job(&jobs, &plan).unwrap();
        run_operation(&jobs, &job.job_id, &plan.operation).unwrap();
    });
    let texture_reader = root.path().join("read_texture.gd");
    fs::write(&texture_reader, r#"extends SceneTree
func _initialize() -> void:
	var texture := load("res://addons/forge_assets/fixture/items/stone.png") as Texture2D
	if texture == null:
		push_error("Missing installed texture")
		quit(1)
		return
	var image := texture.get_image()
	image.convert(Image.FORMAT_RGBA8)
	var hash := HashingContext.new()
	hash.start(HashingContext.HASH_SHA256)
	hash.update(image.get_data())
	var pixel := image.get_pixel(32,30)
	print("NATIVE_IMAGE " + JSON.stringify({"sha256":hash.finish().hex_encode(),"center":[pixel.r8,pixel.g8,pixel.b8,pixel.a8]}))
	quit(0)
"#).unwrap();
    let native_image = || {
        let output = std::process::Command::new(&godot)
            .args(["--headless", "--path"])
            .arg(&project)
            .arg("--script")
            .arg(&texture_reader)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        serde_json::from_str::<serde_json::Value>(
            stdout
                .lines()
                .find_map(|line| line.strip_prefix("NATIVE_IMAGE "))
                .unwrap(),
        )
        .unwrap()
    };
    let before_native = native_image();
    assert_eq!(before_native["center"], json!([240, 40, 20, 255]));
    let target = project.join("addons/forge_assets/fixture");
    let before_target = inventory(&target).unwrap();
    let before_registry = hash_file(&project.join(".forge/assets.json")).unwrap();
    let before_cache = inventory(&project.join(".godot/imported")).unwrap();
    let wrapper = root.path().join("fail-verify-after-real-import.py");
    fs::write(
        &wrapper,
        format!(
            r#"#!/usr/bin/env python3
import subprocess, sys
if '--verify' in sys.argv:
    print('ERROR: injected failure after real import and resource creation', file=sys.stderr)
    sys.exit(7)
sys.exit(subprocess.call([{godot:?}, *sys.argv[1:]]))
"#,
            godot = godot.to_string_lossy()
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let prepared = plans.prepare(install_operation(&blue, &project)).unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    temp_env::with_var("FORGE_GODOT_PATH", Some(wrapper.as_os_str()), || {
        let error = run_operation(&jobs, &job.job_id, &plan.operation).unwrap_err();
        assert!(error.to_string().contains("Godot verify failed"), "{error}");
    });
    assert_eq!(inventory(&target).unwrap(), before_target);
    assert_eq!(
        hash_file(&project.join(".forge/assets.json")).unwrap(),
        before_registry
    );
    assert_eq!(
        inventory(&project.join(".godot/imported")).unwrap(),
        before_cache
    );
    // A fresh runtime load must use red cache bytes immediately, without an editor import.
    assert_eq!(native_image(), before_native);
}
