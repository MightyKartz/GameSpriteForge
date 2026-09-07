use super::*;
use crate::automation::{
    automation_profile, run_operation, stage_plan_job, AssetInput, AssetMetadata,
    AutomationOperation, MattingRecipe, PlanStore, PrepareAssetRequest, QualityPolicy,
};
use std::os::unix::fs::PermissionsExt;

struct Fixture {
    _temp: tempfile::TempDir,
    jobs: JobStore,
    job: JobRecord,
    request: GodotInstallRequest,
    godot: PathBuf,
    original_manifest: Vec<u8>,
    original_target_hash: Option<String>,
}

fn setup(mode: &str, existing: bool) -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("godot");
    fs::create_dir_all(project.join(".forge")).unwrap();
    fs::write(project.join("project.godot"), "[application]\n").unwrap();
    let original_manifest =
        b"{\"schemaVersion\":\"1\",\"updatedAt\":\"2026-08-01T00:00:00Z\",\"assets\":{}}\n"
            .to_vec();
    fs::write(project.join(PROJECT_MANIFEST_RELATIVE), &original_manifest).unwrap();
    let target = project.join("addons/forge_assets/test");
    let original_target_hash = existing.then(|| {
        fs::create_dir_all(target.join("nested")).unwrap();
        fs::write(
            target.join(OWNERSHIP_MARKER),
            "{\"owner\":\"Game Sprite Forge\"}",
        )
        .unwrap();
        fs::write(
            target.join("nested/previous.txt"),
            "previous installed asset",
        )
        .unwrap();
        hash_directory(&target).unwrap()
    });
    let mut frame = image::RgbaImage::new(16, 16);
    for y in 4..12 {
        for x in 4..12 {
            frame.put_pixel(x, y, image::Rgba([200, 40, 30, 255]));
        }
    }
    let paths = (0..4)
        .map(|index| {
            let path = temp.path().join(format!("frame-{index}.png"));
            frame.save(&path).unwrap();
            path
        })
        .collect();
    let profile = automation_profile();
    let plans = PlanStore::new(temp.path().join("plans")).unwrap();
    let jobs = JobStore::new(temp.path().join("jobs")).unwrap();
    let prepared = plans
        .prepare(AutomationOperation::PrepareAsset(PrepareAssetRequest {
            schema_version: "1".into(),
            input: AssetInput::PngSequence { paths },
            metadata: AssetMetadata {
                name: "Rollback fixture".into(),
                animation: "idle".into(),
                fps: 8.0,
                loop_animation: true,
                creator: "Forge".into(),
                license: "private".into(),
            },
            matting: MattingRecipe::PreserveAlpha,
            normalize: profile.normalize,
            sheet: profile.sheet,
            quality: QualityPolicy::default(),
        }))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let queued = stage_plan_job(&jobs, &plan).unwrap();
    let completed = run_operation(&jobs, &queued.job_id, &plan.operation).unwrap();
    let pack = completed
        .artifacts
        .iter()
        .find(|artifact| artifact.kind == "gsfpack")
        .unwrap();
    let catalog_project_path =
        matches!(mode, "registration_failure" | "job_commit_failure").then(|| {
            let catalog = temp.path().join("catalog-project");
            crate::asset_project::init_project(&catalog, "Install rollback").unwrap();
            crate::catalog::write_project_catalog(&catalog, &Default::default()).unwrap();
            {
                let summary = forge_pack::inspect_pack(&pack.path).unwrap();
                crate::catalog::register_catalog_asset(
                    &catalog,
                    crate::catalog::ProjectCatalogEntryV1 {
                        asset_id: summary.id,
                        name: summary.name,
                        kind: summary.asset_type,
                        pack_path: pack.path.clone(),
                        pack_sha256: hash_directory(&pack.path).unwrap(),
                        source_job_id: completed.job_id.clone(),
                        parent_job_id: None,
                        style: None,
                        subject: None,
                        workflow: "test-install@1.0.0".into(),
                        provider: None,
                        installed: None,
                        created_at: chrono::Utc::now(),
                    },
                )
                .unwrap();
            }
            catalog
        });
    let request = GodotInstallRequest {
        schema_version: "1".into(),
        pack_path: pack.path.clone(),
        project_path: project,
        catalog_project_path,
        target: "addons/forge_assets/test".into(),
        asset_key: Some("test".into()),
        provider_refs: vec![],
    };
    let prepared = plans
        .prepare(AutomationOperation::InstallGodot(request.clone()))
        .unwrap();
    let plan = plans.claim(&prepared.token).unwrap();
    let job = stage_plan_job(&jobs, &plan).unwrap();
    if mode == "log_failure" {
        fs::create_dir(job.job_dir.join("logs/godot.import.stdout.log")).unwrap();
    }
    if mode == "source_copy_failure" {
        fs::write(job.job_dir.join("staged-target"), "not a directory").unwrap();
    }
    let godot = temp.path().join("fake-godot");
    // A process boundary fake exercises spawn/status/log/resource validation
    // failures without environment mutation or launching the user's editor.
    let script = r##"#!/bin/sh
mode='@MODE@'
if [ "$1" = '--version' ]; then
  if [ "$mode" = 'version_failure' ]; then echo '4.5.0'; else echo '4.6.3'; fi
  if [ "$mode" = 'spawn_failure' ]; then rm "$0"; fi
  exit 0
fi
import=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --path) project="$2"; shift 2 ;;
    --import) import=1; shift ;;
    --script) install_script="$2"; shift 2 ;;
    --) shift; pack="$1"; target="$2"; break ;;
    *) shift ;;
  esac
done
if [ "$import" = 1 ]; then
  if [ "$mode" = 'import_failure' ]; then exit 17; fi
  exit 0
fi
if [ "$mode" = 'script_failure' ]; then exit 18; fi
if [ "$mode" = 'missing_resources' ]; then exit 0; fi
mkdir -p "$project/$target"
echo '[gd_scene format=3]' > "$project/$target/forge_animated_sprite.tscn"
echo '[gd_resource type="SpriteFrames" format=3]' > "$project/$target/forge_sprite_frames.tres"
if [ "$mode" = 'embedded_pixels' ]; then
  echo '[sub_resource type="Image" id="bad"]' >> "$project/$target/forge_sprite_frames.tres"
fi
if [ "$mode" = 'job_commit_failure' ]; then
  job="$(dirname "$(dirname "$install_script")")"
  rm "$job/job.json"
fi
if [ "$mode" = 'registration_failure' ]; then
  root="$(dirname "$project")"
  echo '{"schemaVersion":"2","updatedAt":"2026-08-01T00:00:00Z","assets":{}}' > "$root/catalog-project/.forge/catalog.json"
fi
"##
    .replace("@MODE@", mode);
    fs::write(&godot, script).unwrap();
    fs::set_permissions(&godot, fs::Permissions::from_mode(0o755)).unwrap();
    Fixture {
        _temp: temp,
        jobs,
        job,
        request,
        godot,
        original_manifest,
        original_target_hash,
    }
}

#[test]
fn godot_install_errors_restore_previous_resources_and_registration() {
    for existing in [false, true] {
        for (mode, expected) in [
            ("version_failure", "requires Godot 4.6"),
            ("source_copy_failure", "io error"),
            ("spawn_failure", "io error"),
            ("import_failure", "asset import failed"),
            ("script_failure", "Godot import failed"),
            ("log_failure", "io error"),
            ("missing_resources", "required scene resources are missing"),
            ("embedded_pixels", "embeds image pixels"),
            ("registration_failure", "catalog asset not found"),
            ("job_commit_failure", "job error"),
        ] {
            let fixture = setup(mode, existing);
            let original_pack = hash_directory(&fixture.request.pack_path).unwrap();
            let original_catalog = fixture
                .request
                .catalog_project_path
                .as_ref()
                .map(|root| fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap());
            let error = run_install_godot_with_executable(
                &fixture.jobs,
                &fixture.job.job_id,
                &fixture.request,
                &fixture.godot,
            )
            .unwrap_err();
            assert!(error.to_string().contains(expected), "{mode}: {error}");
            let target = fixture.request.project_path.join(&fixture.request.target);
            if let Some(hash) = fixture.original_target_hash {
                assert_eq!(hash_directory(&target).unwrap(), hash, "{mode}");
            } else {
                assert!(!target.exists(), "{mode} left a partial installation");
            }
            assert_eq!(
                fs::read(fixture.request.project_path.join(PROJECT_MANIFEST_RELATIVE)).unwrap(),
                fixture.original_manifest,
                "{mode}"
            );
            assert_eq!(
                hash_directory(&fixture.request.pack_path).unwrap(),
                original_pack,
                "{mode} changed the source Pack"
            );
            if let Some(root) = &fixture.request.catalog_project_path {
                assert_eq!(
                    fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap(),
                    original_catalog.unwrap(),
                    "{mode}"
                );
            }
        }
    }
}

#[test]
fn failed_first_registration_does_not_leave_a_new_manifest() {
    let fixture = setup("registration_failure", false);
    let manifest = fixture.request.project_path.join(PROJECT_MANIFEST_RELATIVE);
    fs::remove_file(&manifest).unwrap();
    let error = run_install_godot_with_executable(
        &fixture.jobs,
        &fixture.job.job_id,
        &fixture.request,
        &fixture.godot,
    )
    .unwrap_err();
    assert!(error.to_string().contains("catalog asset not found"));
    assert!(!manifest.exists());
    assert!(!fixture
        .request
        .project_path
        .join(&fixture.request.target)
        .exists());
}

#[test]
fn successful_install_commits_resources_and_registration() {
    let fixture = setup("success", true);
    let completed = run_install_godot_with_executable(
        &fixture.jobs,
        &fixture.job.job_id,
        &fixture.request,
        &fixture.godot,
    )
    .unwrap();
    assert_eq!(completed.lifecycle_state, JobLifecycleState::Succeeded);
    let target = fixture.request.project_path.join(&fixture.request.target);
    assert!(!target.join("nested/previous.txt").exists());
    assert!(target.join("forge_usage.json").is_file());
    assert!(target.join(OWNERSHIP_MARKER).is_file());
    let manifest = crate::project::read_project_manifest(&fixture.request.project_path).unwrap();
    assert_eq!(
        manifest.assets["test"].pack_sha256,
        hash_directory(&fixture.request.pack_path).unwrap()
    );
    assert_eq!(
        hash_directory(&fixture.job.job_dir.join("backups/godot-target")).unwrap(),
        fixture.original_target_hash.unwrap()
    );
}

#[test]
fn a_waiting_install_commits_after_the_previous_install_rolls_back() {
    use std::sync::mpsc;
    use std::time::Duration;

    let fixture = setup("success", true);
    let mut first = InstallTransaction::new(&fixture.request, &fixture.job.job_dir).unwrap();
    let first_stage = fixture._temp.path().join("first-stage");
    fs::create_dir(&first_stage).unwrap();
    fs::write(first_stage.join(OWNERSHIP_MARKER), "{}").unwrap();
    fs::write(first_stage.join("first.txt"), "uncommitted").unwrap();
    first.replace_target(&first_stage).unwrap();

    let second_job_dir = fixture._temp.path().join("second-job");
    let second_stage = fixture._temp.path().join("second-stage");
    fs::create_dir(&second_stage).unwrap();
    fs::write(second_stage.join(OWNERSHIP_MARKER), "{}").unwrap();
    fs::write(second_stage.join("second.txt"), "committed").unwrap();
    let request = fixture.request.clone();
    let (started_tx, started_rx) = mpsc::channel();
    let (locked_tx, locked_rx) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        let mut second = InstallTransaction::new(&request, &second_job_dir).unwrap();
        locked_tx.send(()).unwrap();
        // The second transaction must snapshot the restored original, never
        // the first install's uncommitted resource tree.
        assert!(second.target.join("nested/previous.txt").is_file());
        assert!(!second.target.join("first.txt").exists());
        let result = second.replace_target(&second_stage);
        second.finish(result).unwrap();
    });
    started_rx.recv().unwrap();
    assert!(matches!(
        locked_rx.recv_timeout(Duration::from_millis(50)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    let failure: Result<(), _> = Err(AutomationRunError::Processing("injected failure".into()));
    assert!(first.finish(failure).is_err());
    drop(first);
    locked_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    worker.join().unwrap();
    let target = fixture.request.project_path.join(&fixture.request.target);
    assert_eq!(
        fs::read_to_string(target.join("second.txt")).unwrap(),
        "committed"
    );
    assert!(!target.join("first.txt").exists());
}
