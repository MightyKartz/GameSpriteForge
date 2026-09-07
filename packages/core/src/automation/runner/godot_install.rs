//! Godot delivery owns one rollback boundary for resources and their registrations.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{check_cancelled, copy_directory, hash_directory, step, which, AutomationRunError};
use crate::automation::GodotInstallRequest;
use crate::catalog::{link_catalog_install_unlocked, lock_catalog, PROJECT_CATALOG_RELATIVE};
use crate::job::{JobArtifactRecord, JobLifecycleState, JobRecord, JobState, JobStore};
use crate::project::{
    lock_project_manifest, register_project_asset_unlocked, ProviderAssetRef, RegisterProjectAsset,
    PROJECT_MANIFEST_RELATIVE,
};

const GODOT_INSTALL_SCRIPT: &str =
    include_str!("../../../../../scripts/godot/install_forge_pack.gd");
const OWNERSHIP_MARKER: &str = ".forge-owned.json";

pub(super) fn run_install_godot(
    store: &JobStore,
    job_id: &str,
    request: &GodotInstallRequest,
) -> Result<JobRecord, AutomationRunError> {
    let godot = locate_godot()
        .ok_or_else(|| AutomationRunError::Processing("Godot 4 executable was not found".into()))?;
    run_install_godot_with_executable(store, job_id, request, &godot)
}

fn run_install_godot_with_executable(
    store: &JobStore,
    job_id: &str,
    request: &GodotInstallRequest,
    godot: &Path,
) -> Result<JobRecord, AutomationRunError> {
    check_cancelled(store, job_id)?;
    forge_pack::validate_pack_layout(&request.pack_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let pack_summary = forge_pack::inspect_pack(&request.pack_path)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    let provider_refs =
        effective_provider_refs(&request.pack_path, &pack_summary, &request.provider_refs)?;
    let pack_sha256 = hash_directory(&request.pack_path)?;
    let record = store.read_record(job_id)?;
    let target = request.project_path.join(&request.target);
    ensure_target_inside_project(&request.project_path, &target)?;
    require_godot_46(godot)?;
    let mut transaction = InstallTransaction::new(request, &record.job_dir)?;
    let result = (|| {
        step(store, job_id, "validate", "succeeded", 0.18, None)?;

        // Materialize sources before touching a previously installed asset.
        let staged = record.job_dir.join("staged-target");
        fs::create_dir_all(&staged)?;
        copy_godot_pack_sources(&request.pack_path, &staged, &pack_summary.asset_type)?;
        let script = record.job_dir.join("tools/install_forge_pack.gd");
        fs::write(&script, GODOT_INSTALL_SCRIPT)?;
        check_cancelled(store, job_id)?;
        transaction.replace_target(&staged)?;
        step(store, job_id, "backup", "succeeded", 0.3, None)?;

        let import_output = Command::new(godot)
            .arg("--headless")
            .arg("--import")
            .arg("--path")
            .arg(&request.project_path)
            .output()?;
        fs::write(
            record.job_dir.join("logs/godot.import.stdout.log"),
            &import_output.stdout,
        )?;
        fs::write(
            record.job_dir.join("logs/godot.import.stderr.log"),
            &import_output.stderr,
        )?;
        if !import_output.status.success() {
            return Err(AutomationRunError::Processing(format!(
                "Godot asset import failed with status {}",
                import_output.status
            )));
        }
        let output = Command::new(godot)
            .arg("--headless")
            .arg("--path")
            .arg(&request.project_path)
            .arg("--script")
            .arg(&script)
            .arg("--")
            .arg(&request.pack_path)
            .arg(&request.target)
            .output()?;
        fs::write(record.job_dir.join("logs/godot.stdout.log"), &output.stdout)?;
        fs::write(record.job_dir.join("logs/godot.stderr.log"), &output.stderr)?;

        if !output.status.success() {
            return Err(AutomationRunError::Processing(format!(
                "Godot import failed with status {}",
                output.status
            )));
        }

        let (scene_path, frames_path) = match pack_summary.asset_type.as_str() {
            "icon_set" | "portrait_set" => (target.join("items"), target.join("items")),
            "prop_set" | "equipment_set" | "decal_set" => {
                (target.join("scenes"), target.join("items"))
            }
            "terrain_set" => (
                target.join("forge_terrain_preview.tscn"),
                target.join("forge_terrain_set.tres"),
            ),
            "building_kit" => (
                target.join("scenes"),
                target.join("forge_building_kit.tres"),
            ),
            "map" => (
                target.join("forge_world.tscn"),
                target.join("forge_terrain_set.tres"),
            ),
            _ => (
                target.join("forge_animated_sprite.tscn"),
                target.join("forge_sprite_frames.tres"),
            ),
        };
        if !scene_path.exists() || !frames_path.exists() {
            return Err(AutomationRunError::Processing(
                "Godot exited successfully but required scene resources are missing".into(),
            ));
        }
        verify_external_godot_resources(&target)?;

        let asset_key = request
            .asset_key
            .clone()
            .or_else(|| {
                request
                    .target
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(str::to_string)
            })
            .ok_or_else(|| {
                AutomationRunError::Processing("Godot target has no asset key".into())
            })?;
        let (scene_relative, frames_relative) = match pack_summary.asset_type.as_str() {
            "icon_set" | "portrait_set" => {
                (request.target.join("items"), request.target.join("items"))
            }
            "prop_set" | "equipment_set" | "decal_set" => {
                (request.target.join("scenes"), request.target.join("items"))
            }
            "terrain_set" => (
                request.target.join("forge_terrain_preview.tscn"),
                request.target.join("forge_terrain_set.tres"),
            ),
            "building_kit" => (
                request.target.join("scenes"),
                request.target.join("forge_building_kit.tres"),
            ),
            "map" => (
                request.target.join("forge_world.tscn"),
                request.target.join("forge_terrain_set.tres"),
            ),
            _ => (
                request.target.join("forge_animated_sprite.tscn"),
                request.target.join("forge_sprite_frames.tres"),
            ),
        };
        let usage_relative = request.target.join("forge_usage.json");
        let usage_path = request.project_path.join(&usage_relative);
        let loop_selection = fs::read(request.pack_path.join("quality/loops.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let pack_provenance = fs::read(request.pack_path.join("forgepack.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let godot_helper = fs::read(request.pack_path.join("assets/godot_import.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
        let rendering = godot_helper
            .as_ref()
            .and_then(|value| value.pointer("/spriteFrames/rendering"))
            .cloned();
        let mirror_policy = rendering
            .as_ref()
            .and_then(|value| value.get("mirrorPolicy"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("auto");
        let provider_retry_methods = pack_provenance
            .as_ref()
            .and_then(|value| value.pointer("/source/metadata/providerRetryMethods"))
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let asset_geometry = pack_provenance.as_ref().and_then(|value| {
            let metadata = value.pointer("/source/metadata")?;
            metadata.get("assetGeometryProfile")?;
            Some(serde_json::json!({
                "profile": metadata.get("assetGeometryProfile"),
                "reportProfile": metadata.get("assetGeometryReportProfile"),
                "consistencyReportSha256": metadata.get("consistencyReportSha256"),
            }))
        });
        let portrait_consistency = pack_provenance.as_ref().and_then(|value| {
            let metadata = value.pointer("/source/metadata")?;
            metadata.get("portraitBaseProfile")?;
            Some(serde_json::json!({
                "baseProfile": metadata.get("portraitBaseProfile"),
                "baseItemId": metadata.get("portraitBaseItemId"),
                "baseSha256": metadata.get("portraitBaseSha256"),
                "neutralReferencePolicy": metadata.get("portraitNeutralReferencePolicy"),
                "neutralReferenceRoles": metadata.get("portraitNeutralReferenceRoles"),
                "neutralReferenceSha256": metadata.get("portraitNeutralReferenceSha256"),
                "approvalProfile": metadata.get("portraitBaseApprovalProfile"),
                "approvalSourceJobId": metadata.get("portraitBaseApprovalSourceJobId"),
                "approvalSha256": metadata.get("portraitBaseApprovalSha256"),
                "localProfile": metadata.get("portraitLocalProfile"),
                "reportSha256": metadata.get("portraitConsistencyReportSha256"),
            }))
        });
        let character_semantic_quality = pack_provenance.as_ref().and_then(|value| {
            value
                .pointer("/source/metadata/characterSemanticQuality")
                .cloned()
        });
        let character_camera_profile = pack_provenance
            .as_ref()
            .and_then(|value| value.pointer("/source/metadata/cameraProfile"))
            .cloned();
        let character_silhouette_temporal = pack_provenance.as_ref().and_then(|value| {
            value
                .pointer("/source/metadata/characterSilhouetteTemporal")
                .cloned()
        });
        let character_motion_semantics = pack_provenance.as_ref().and_then(|value| {
            value
                .pointer("/source/metadata/characterMotionSemantics")
                .cloned()
        });
        let character_assembly = pack_provenance
            .as_ref()
            .and_then(|value| value.pointer("/source/metadata/characterAssembly"))
            .cloned();
        let character_direction_lock = pack_provenance
            .as_ref()
            .and_then(|value| value.pointer("/source/metadata/directionLock"))
            .cloned();
        let character_direction_motion_lock = pack_provenance
            .as_ref()
            .and_then(|value| value.pointer("/source/metadata/directionMotionLock"))
            .cloned();
        let keyframe_background_cleanup_profile = pack_provenance
            .as_ref()
            .and_then(|value| value.pointer("/source/metadata/backgroundCleanupProfile"))
            .cloned();
        let keyframe_loop_trim = pack_provenance
            .as_ref()
            .and_then(|value| value.pointer("/source/metadata/keyframeLoopTrim"))
            .cloned();
        let animation_names = pack_summary
            .animations
            .iter()
            .map(|animation| animation.name.clone())
            .collect::<HashSet<_>>();
        let directional_playback = directional_playback_contract(&animation_names, mirror_policy);
        let usage = serde_json::json!({
            "schemaVersion": "1",
            "assetKey": &asset_key,
            "assetId": &pack_summary.id,
            "packSha256": &pack_sha256,
            "kind": &pack_summary.asset_type,
            "scenePath": format!("res://{}", scene_relative.display()),
            "spriteFramesPath": format!("res://{}", frames_relative.display()),
            "defaultAnimation": &pack_summary.default_animation,
            "animations": &pack_summary.animations,
            "items": &pack_summary.items,
            "directionalPlayback": directional_playback,
            "providerProvenance": &provider_refs,
            "rendering": rendering,
            "loopSelection": loop_selection,
            "providerRetryMethods": provider_retry_methods,
            "characterSemanticQuality": character_semantic_quality,
            "characterCameraProfile": character_camera_profile,
            "characterSilhouetteTemporal": character_silhouette_temporal,
            "characterMotionSemantics": character_motion_semantics,
            "characterAssembly": character_assembly,
            "characterDirectionLock": character_direction_lock,
            "characterDirectionMotionLock": character_direction_motion_lock,
            "keyframeBackgroundCleanupProfile": keyframe_background_cleanup_profile,
            "keyframeLoopTrim": keyframe_loop_trim,
            "assetGeometry": asset_geometry,
            "portraitConsistency": portrait_consistency,
            "nodeType": match pack_summary.asset_type.as_str() {
                "icon_set" | "portrait_set" => "Texture2D",
                "prop_set" | "equipment_set" | "decal_set" => "Sprite2D",
                "terrain_set" => "TileSet",
                "building_kit" => "Node2D",
                "map" => "TileMapLayer",
                _ => "AnimatedSprite2D",
            },
            "worldGeneration": pack_provenance.as_ref().and_then(|value| {
                matches!(pack_summary.asset_type.as_str(), "terrain_set" | "building_kit" | "map")
                    .then(|| value.pointer("/source/metadata").cloned())
                    .flatten()
            }),
            "gameplayControllerIncluded": false,
        });
        fs::write(&usage_path, serde_json::to_vec_pretty(&usage)?)?;

        let marker = serde_json::json!({
            "schemaVersion": "1",
            "owner": "Game Sprite Forge",
            "jobId": job_id,
            "packPath": request.pack_path,
            "assetKey": &asset_key,
            "projectManifest": PROJECT_MANIFEST_RELATIVE,
        });
        fs::write(
            target.join(OWNERSHIP_MARKER),
            serde_json::to_vec_pretty(&marker)?,
        )?;
        let manifest_path = register_project_asset_unlocked(RegisterProjectAsset {
            project_path: &request.project_path,
            asset_key: &asset_key,
            pack_path: &request.pack_path,
            pack_sha256: &pack_sha256,
            godot_target: &request.target,
            scene_path: &scene_relative,
            sprite_frames_path: &frames_relative,
            usage_path: &usage_relative,
            pack: &pack_summary,
            provider_refs: &provider_refs,
            job_id,
        })
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        let catalog_link = if let Some(catalog_project) = &request.catalog_project_path {
            Some(
                link_catalog_install_unlocked(
                    catalog_project,
                    &pack_summary.id,
                    request.project_path.clone(),
                    request.target.clone(),
                )
                .map_err(|error| AutomationRunError::Processing(error.to_string()))?,
            )
        } else {
            None
        };
        let mut artifacts = vec![
            JobArtifactRecord {
                kind: "godot_scene".into(),
                path: scene_path,
                sha256: None,
            },
            JobArtifactRecord {
                kind: "godot_usage".into(),
                path: usage_path,
                sha256: None,
            },
            JobArtifactRecord {
                kind: "project_manifest".into(),
                path: manifest_path,
                sha256: None,
            },
        ];
        if let Some(path) = catalog_link {
            artifacts.push(JobArtifactRecord {
                kind: "project_catalog".into(),
                path,
                sha256: None,
            });
        }
        store
            .update_record(job_id, |record| {
                record.state = JobState::Exported;
                record.lifecycle_state = JobLifecycleState::Succeeded;
                record.progress = 1.0;
                record.worker_pid = None;
                record
                    .steps
                    .iter_mut()
                    .for_each(|step| step.state = "succeeded".into());
                record.artifacts.extend(artifacts);
                record.next_actions = vec![
                    "inspect_project".into(),
                    "open_godot_project".into(),
                    "open_job".into(),
                ];
            })
            .map_err(Into::into)
    })();
    transaction.finish(result)
}

/// Both metadata locks remain held until resources and registrations commit or
/// roll back. Snapshots are kept in the Job even when recovery itself fails.
struct InstallTransaction {
    target: PathBuf,
    backup: PathBuf,
    had_target: bool,
    target_changed: bool,
    snapshots: Vec<FileSnapshot>,
    finished: bool,
    _project_lock: fs::File,
    _catalog_lock: Option<fs::File>,
}

impl InstallTransaction {
    fn new(request: &GodotInstallRequest, job_dir: &Path) -> Result<Self, AutomationRunError> {
        let project_lock = lock_project_manifest(&request.project_path)
            .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
        let catalog_lock = request
            .catalog_project_path
            .as_ref()
            .map(|project| {
                lock_catalog(project)
                    .map_err(|error| AutomationRunError::Processing(error.to_string()))
            })
            .transpose()?;
        let target = request.project_path.join(&request.target);
        ensure_target_inside_project(&request.project_path, &target)?;
        let had_target = target.exists();
        if had_target && !target.join(OWNERSHIP_MARKER).is_file() {
            return Err(AutomationRunError::Processing(format!(
                "refusing to replace non-Forge-owned directory: {}",
                target.display()
            )));
        }
        let backups = job_dir.join("backups");
        fs::create_dir_all(&backups)?;
        let mut snapshots = vec![FileSnapshot::capture(
            request.project_path.join(PROJECT_MANIFEST_RELATIVE),
            &backups.join("godot-project-manifest.json"),
        )?];
        if let Some(project) = &request.catalog_project_path {
            snapshots.push(FileSnapshot::capture(
                project.join(PROJECT_CATALOG_RELATIVE),
                &backups.join("godot-project-catalog.json"),
            )?);
        }
        Ok(Self {
            target,
            backup: backups.join("godot-target"),
            had_target,
            target_changed: false,
            snapshots,
            finished: false,
            _project_lock: project_lock,
            _catalog_lock: catalog_lock,
        })
    }

    fn replace_target(&mut self, staged: &Path) -> Result<(), AutomationRunError> {
        if self.had_target {
            // The shared directory copier intentionally ignores symlinks. Reject
            // them before replacement instead of silently omitting user files.
            require_regular_tree(&self.target)?;
            if self.backup.exists() {
                return Err(AutomationRunError::Processing(
                    "Godot install backup already exists; refusing to overwrite recovery evidence"
                        .into(),
                ));
            }
            copy_directory(&self.target, &self.backup)?;
        }
        self.target_changed = true;
        if self.had_target {
            fs::remove_dir_all(&self.target)?;
        }
        copy_directory(staged, &self.target)?;
        Ok(())
    }

    fn finish<T>(
        &mut self,
        result: Result<T, AutomationRunError>,
    ) -> Result<T, AutomationRunError> {
        let outcome = match result {
            Ok(value) => Ok(value),
            Err(error) => match self.rollback() {
                Ok(()) => Err(error),
                Err(recovery) => Err(AutomationRunError::Processing(format!(
                    "{error}; Godot rollback failed: {recovery}; recovery backup: {}",
                    self.backup.display()
                ))),
            },
        };
        self.finished = true;
        outcome
    }

    fn rollback(&mut self) -> Result<(), String> {
        if !self.target_changed {
            return Ok(());
        }
        let mut errors = Vec::new();
        let resources = (|| -> std::io::Result<()> {
            if self.target.exists() {
                fs::remove_dir_all(&self.target)?;
            }
            if self.had_target {
                copy_directory(&self.backup, &self.target)?;
            }
            Ok(())
        })();
        if let Err(error) = resources {
            errors.push(format!("{}: {error}", self.target.display()));
        }
        for snapshot in &self.snapshots {
            if let Err(error) = snapshot.restore() {
                errors.push(format!("{}: {error}", snapshot.path.display()));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

impl Drop for InstallTransaction {
    fn drop(&mut self) {
        if !self.finished {
            // Also cover Rust unwinding; ordinary errors use finish() so a
            // recovery failure is reported instead of being silently discarded.
            let _ = self.rollback();
        }
    }
}

struct FileSnapshot {
    path: PathBuf,
    original: Option<(Vec<u8>, fs::Permissions)>,
}

impl FileSnapshot {
    fn capture(path: PathBuf, backup: &Path) -> Result<Self, AutomationRunError> {
        let original = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {
                let bytes = fs::read(&path)?;
                fs::write(backup, &bytes)?;
                Some((bytes, metadata.permissions()))
            }
            Ok(_) => {
                return Err(AutomationRunError::Processing(format!(
                    "Godot registration must be a regular file: {}",
                    path.display()
                )))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        Ok(Self { path, original })
    }

    fn restore(&self) -> std::io::Result<()> {
        if let Some((bytes, permissions)) = &self.original {
            use std::io::Write;
            let mut temporary = tempfile::NamedTempFile::new_in(
                self.path.parent().expect("registration has a parent"),
            )?;
            temporary.write_all(bytes)?;
            temporary.as_file().set_permissions(permissions.clone())?;
            temporary.as_file().sync_all()?;
            temporary.persist(&self.path).map_err(|error| error.error)?;
        } else {
            match fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

fn require_regular_tree(directory: &Path) -> Result<(), AutomationRunError> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            require_regular_tree(&entry.path())?;
        } else if !kind.is_file() {
            return Err(AutomationRunError::Processing(format!(
                "Godot install cannot safely back up a non-regular entry: {}",
                entry.path().display()
            )));
        }
    }
    Ok(())
}

fn directional_playback_contract(
    animation_names: &HashSet<String>,
    mirror_policy: &str,
) -> Option<serde_json::Value> {
    let has = |required: &[&str]| {
        required
            .iter()
            .all(|animation| animation_names.contains(*animation))
    };
    if has(&[
        "idle_down",
        "idle_up",
        "idle_right",
        "idle_left",
        "walk_down",
        "walk_up",
        "walk_right",
        "walk_left",
    ]) && mirror_policy != "mirror_right_to_left"
    {
        return Some(serde_json::json!({
            "profile": "explicit-four-direction@1.0.0",
            "up": { "animation": "walk_up", "idleAnimation": "idle_up", "flipH": false },
            "right": { "animation": "walk_right", "idleAnimation": "idle_right", "flipH": false },
            "down": { "animation": "walk_down", "idleAnimation": "idle_down", "flipH": false },
            "left": { "animation": "walk_left", "idleAnimation": "idle_left", "flipH": false },
            "idle": { "animation": "idle_down", "flipH": false }
        }));
    }
    if mirror_policy == "mirror_right_to_left"
        && has(&[
            "idle_down",
            "idle_up",
            "idle_right",
            "walk_down",
            "walk_up",
            "walk_right",
        ])
    {
        return Some(serde_json::json!({
            "profile": "right-mirror-left@1.0.0",
            "up": { "animation": "walk_up", "idleAnimation": "idle_up", "flipH": false },
            "right": { "animation": "walk_right", "idleAnimation": "idle_right", "flipH": false },
            "down": { "animation": "walk_down", "idleAnimation": "idle_down", "flipH": false },
            "left": { "animation": "walk_right", "idleAnimation": "idle_right", "flipH": true },
            "idle": { "animation": "idle_down", "flipH": false }
        }));
    }
    if mirror_policy == "right_only"
        && has(&[
            "idle_down",
            "idle_up",
            "idle_right",
            "walk_down",
            "walk_up",
            "walk_right",
        ])
    {
        return Some(serde_json::json!({
            "profile": "explicit-three-direction-no-left@1.0.0",
            "up": { "animation": "walk_up", "idleAnimation": "idle_up", "flipH": false },
            "right": { "animation": "walk_right", "idleAnimation": "idle_right", "flipH": false },
            "down": { "animation": "walk_down", "idleAnimation": "idle_down", "flipH": false },
            "idle": { "animation": "idle_down", "flipH": false },
            "leftAuthorized": false
        }));
    }
    if mirror_policy == "right_only" && has(&["idle_right", "walk_right"]) {
        return Some(serde_json::json!({
            "profile": "right-only@1.0.0",
            "right": { "animation": "walk_right", "idleAnimation": "idle_right", "flipH": false },
            "idle": { "animation": "idle_right", "flipH": false },
            "leftAuthorized": false
        }));
    }
    has(&["idle", "walk_up", "walk_right", "walk_down"]).then(|| {
        serde_json::json!({
            "profile": "legacy-right-mirror-left@1.0.0",
            "up": { "animation": "walk_up", "flipH": false },
            "right": { "animation": "walk_right", "flipH": false },
            "down": { "animation": "walk_down", "flipH": false },
            "left": { "animation": "walk_right", "flipH": true },
            "idle": { "animation": "idle", "flipH": false }
        })
    })
}

fn effective_provider_refs(
    pack_path: &Path,
    pack: &forge_pack::PackInspectSummary,
    requested: &[ProviderAssetRef],
) -> Result<Vec<ProviderAssetRef>, AutomationRunError> {
    if !requested.is_empty() {
        return Ok(requested.to_vec());
    }
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    let provider_generated = forgepack
        .pointer("/source/kind")
        .and_then(serde_json::Value::as_str)
        == Some("provider_generation");
    let provider = forgepack
        .pointer("/source/metadata/provider")
        .or_else(|| {
            provider_generated
                .then(|| forgepack.pointer("/source/name"))
                .flatten()
        })
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|provider| {
            !provider.is_empty() && provider.len() <= 64 && !provider.chars().any(char::is_control)
        });
    Ok(provider
        .map(|provider| {
            vec![ProviderAssetRef {
                provider: provider.into(),
                asset_id: None,
                label: Some(pack.name.clone()),
            }]
        })
        .unwrap_or_default())
}

fn copy_godot_pack_sources(
    pack: &Path,
    target: &Path,
    asset_type: &str,
) -> Result<(), AutomationRunError> {
    let helper: serde_json::Value =
        serde_json::from_slice(&fs::read(pack.join("assets/godot_import.json"))?)?;
    if matches!(asset_type, "terrain_set" | "building_kit" | "map") {
        let mut textures = Vec::new();
        match asset_type {
            "terrain_set" | "building_kit" => {
                let relative = helper
                    .get("atlas")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        AutomationRunError::Processing(format!(
                            "{asset_type} Godot helper has no atlas"
                        ))
                    })?;
                textures.push(relative.to_string());
            }
            "map" => {
                for field in ["terrainAtlas", "buildingAtlas"] {
                    let relative = helper
                        .get(field)
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            AutomationRunError::Processing(format!(
                                "map Godot helper has no {field}"
                            ))
                        })?;
                    textures.push(relative.to_string());
                }
                for entry in helper
                    .get("propTextures")
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                {
                    let relative = entry
                        .get("texture")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            AutomationRunError::Processing(
                                "map prop texture entry has no texture".into(),
                            )
                        })?;
                    textures.push(relative.to_string());
                }
            }
            _ => unreachable!(),
        }
        for relative in textures {
            if relative.contains("..") || Path::new(&relative).is_absolute() {
                return Err(AutomationRunError::Processing(
                    "world texture path may not escape the Pack".into(),
                ));
            }
            let source = pack.join(&relative);
            if !source.is_file() {
                return Err(AutomationRunError::Processing(format!(
                    "world texture is missing: {relative}"
                )));
            }
            let file_name = Path::new(&relative).file_name().ok_or_else(|| {
                AutomationRunError::Processing("world texture has no filename".into())
            })?;
            fs::copy(source, target.join(file_name))?;
        }
        return Ok(());
    }
    if matches!(
        asset_type,
        "icon_set" | "prop_set" | "portrait_set" | "equipment_set" | "decal_set"
    ) {
        let items = helper
            .get("items")
            .and_then(|value| value.as_array())
            .ok_or_else(|| {
                AutomationRunError::Processing("static Godot helper has no items".into())
            })?;
        let item_target = target.join("items");
        fs::create_dir_all(&item_target)?;
        for item in items {
            let id = item
                .get("id")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    AutomationRunError::Processing("static Godot item has no id".into())
                })?;
            let source = item
                .get("texture")
                .and_then(|value| value.as_str())
                .ok_or_else(|| {
                    AutomationRunError::Processing("static Godot item has no texture".into())
                })?;
            let source_path = pack.join(source);
            if !source_path.is_file() || source.contains("..") {
                return Err(AutomationRunError::Processing(format!(
                    "static item texture is invalid: {source}"
                )));
            }
            fs::copy(source_path, item_target.join(format!("{id}.png")))?;
        }
        return Ok(());
    }
    let textures = helper
        .pointer("/spriteFrames/textures")
        .and_then(|value| value.as_array())
        .ok_or_else(|| AutomationRunError::Processing("Godot helper has no textures".into()))?;
    for texture in textures {
        let relative = texture.as_str().ok_or_else(|| {
            AutomationRunError::Processing("Godot texture entry must be a string".into())
        })?;
        if relative.contains("..") {
            return Err(AutomationRunError::Processing(
                "Godot texture path may not traverse parents".into(),
            ));
        }
        let source = pack.join(relative);
        let file_name = Path::new(relative).file_name().ok_or_else(|| {
            AutomationRunError::Processing("Godot texture path has no filename".into())
        })?;
        fs::copy(source, target.join(file_name))?;
    }
    Ok(())
}

fn verify_external_godot_resources(target: &Path) -> Result<(), AutomationRunError> {
    fn visit(directory: &Path) -> Result<(), AutomationRunError> {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                visit(&path)?;
                continue;
            }
            if !matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("tres" | "tscn")
            ) {
                continue;
            }
            let metadata = fs::metadata(&path)?;
            if metadata.len() >= 1024 * 1024 {
                return Err(AutomationRunError::Processing(format!(
                    "Godot text resource exceeds 1 MiB: {}",
                    path.display()
                )));
            }
            let text = fs::read_to_string(&path)?;
            if text.contains("sub_resource type=\"Image\"")
                || text.contains("sub_resource type=\"ImageTexture\"")
                || text.contains("ImageTexture.create_from_image")
                || text
                    .lines()
                    .any(|line| line.trim_start().starts_with("data = PackedByteArray"))
            {
                return Err(AutomationRunError::Processing(format!(
                    "Godot resource embeds image pixels instead of referencing an external texture: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }
    visit(target)
}

fn locate_godot() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("FORGE_GODOT_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    let candidates = [
        PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot"),
        PathBuf::from("/Applications/Godot_mono.app/Contents/MacOS/Godot"),
    ];
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .or_else(|| ["godot4", "godot"].into_iter().find_map(which))
}

fn require_godot_46(godot: &Path) -> Result<(), AutomationRunError> {
    let output = Command::new(godot).arg("--version").output()?;
    if !output.status.success() {
        return Err(AutomationRunError::Processing(
            "Godot version check failed".into(),
        ));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if !version.trim().starts_with("4.6.") {
        return Err(AutomationRunError::Processing(format!(
            "Forge v0.2 requires Godot 4.6.x; found {}",
            version.trim()
        )));
    }
    Ok(())
}

fn ensure_target_inside_project(project: &Path, target: &Path) -> Result<(), AutomationRunError> {
    let canonical_project = fs::canonicalize(project)?;
    let mut existing = target;
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| {
            AutomationRunError::Processing("Godot target has no existing project ancestor".into())
        })?;
    }
    let metadata = fs::symlink_metadata(existing)?;
    if metadata.file_type().is_symlink() {
        return Err(AutomationRunError::Processing(
            "Godot target may not traverse a symbolic link".into(),
        ));
    }
    let canonical_existing = fs::canonicalize(existing)?;
    if !canonical_existing.starts_with(&canonical_project) {
        return Err(AutomationRunError::Processing(
            "Godot target resolves outside the project".into(),
        ));
    }
    if target.exists() {
        let metadata = fs::symlink_metadata(target)?;
        if metadata.file_type().is_symlink()
            || !fs::canonicalize(target)?.starts_with(canonical_project)
        {
            return Err(AutomationRunError::Processing(
                "Godot target is not a regular directory inside the project".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    mod rollback;
    #[test]
    fn external_directional_playback_uses_declared_right_to_left_mirroring() {
        let names = [
            "idle_down",
            "idle_up",
            "idle_right",
            "walk_down",
            "walk_up",
            "walk_right",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<HashSet<_>>();
        let contract = directional_playback_contract(&names, "mirror_right_to_left").unwrap();

        assert_eq!(contract["profile"], "right-mirror-left@1.0.0");
        assert_eq!(contract["left"]["animation"], "walk_right");
        assert_eq!(contract["left"]["idleAnimation"], "idle_right");
        assert_eq!(contract["left"]["flipH"], true);
    }

    #[test]
    fn explicit_directional_playback_never_flips_left() {
        let names = [
            "idle_down",
            "idle_up",
            "idle_right",
            "idle_left",
            "walk_down",
            "walk_up",
            "walk_right",
            "walk_left",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<HashSet<_>>();
        let contract = directional_playback_contract(&names, "explicit_left").unwrap();

        assert_eq!(contract["profile"], "explicit-four-direction@1.0.0");
        assert_eq!(contract["left"]["animation"], "walk_left");
        assert_eq!(contract["left"]["flipH"], false);
    }

    #[test]
    fn right_only_playback_exposes_no_left_mapping() {
        let names = ["idle_right", "walk_right"]
            .into_iter()
            .map(str::to_string)
            .collect::<HashSet<_>>();
        let contract = directional_playback_contract(&names, "right_only").unwrap();

        assert_eq!(contract["profile"], "right-only@1.0.0");
        assert_eq!(contract["right"]["animation"], "walk_right");
        assert_eq!(contract["leftAuthorized"], false);
        assert!(contract.get("left").is_none());
    }

    #[test]
    fn right_only_playback_preserves_declared_vertical_directions() {
        let names = [
            "idle_down",
            "idle_up",
            "idle_right",
            "walk_down",
            "walk_up",
            "walk_right",
        ]
        .into_iter()
        .map(str::to_string)
        .collect::<HashSet<_>>();
        let contract = directional_playback_contract(&names, "right_only").unwrap();

        assert_eq!(
            contract["profile"],
            "explicit-three-direction-no-left@1.0.0"
        );
        assert_eq!(contract["up"]["animation"], "walk_up");
        assert_eq!(contract["down"]["animation"], "walk_down");
        assert_eq!(contract["leftAuthorized"], false);
        assert!(contract.get("left").is_none());
    }
}
