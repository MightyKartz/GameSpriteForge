//! Installation transaction and subprocess protocol. Kept separate from asset preparation.
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions, Permissions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::Duration;

use super::{check_cancelled, copy_directory, hash_directory, AutomationRunError, JobStore};

pub(super) fn lock_install_project(
    project: &Path,
    store: &JobStore,
    job_id: &str,
) -> Result<File, AutomationRunError> {
    let directory = project.join(".forge");
    if directory.exists() && fs::symlink_metadata(&directory)?.file_type().is_symlink() {
        return Err(AutomationRunError::Processing(
            "project .forge may not be a symbolic link".into(),
        ));
    }
    fs::create_dir_all(&directory)?;
    let path = directory.join("install.lock");
    if path.exists() && fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(AutomationRunError::Processing(
            "install lock may not be a symbolic link".into(),
        ));
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    lock_cancellable(file, store, job_id)
}

pub(super) fn lock_install_catalog(
    project: &Path,
    store: &JobStore,
    job_id: &str,
) -> Result<File, AutomationRunError> {
    let file = crate::catalog::open_catalog_lock(project)
        .map_err(|error| AutomationRunError::Processing(error.to_string()))?;
    lock_cancellable(file, store, job_id)
}

fn lock_cancellable(
    file: File,
    store: &JobStore,
    job_id: &str,
) -> Result<File, AutomationRunError> {
    loop {
        check_cancelled(store, job_id)?;
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(std::fs::TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(50)),
            Err(std::fs::TryLockError::Error(error)) => return Err(error.into()),
        }
    }
}

struct FileSnapshot {
    path: PathBuf,
    previous: Option<(Vec<u8>, Permissions)>,
}

impl FileSnapshot {
    fn read(path: PathBuf) -> io::Result<Self> {
        let previous = match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!(
                            "installation registry is not a regular file: {}",
                            path.display()
                        ),
                    ));
                }
                Some((fs::read(&path)?, metadata.permissions()))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(Self { path, previous })
    }

    fn restore(&self) -> io::Result<()> {
        if let Some((bytes, permissions)) = &self.previous {
            // Atomic replacement also works if the failed operation left a read-only file.
            let parent = self.path.parent().expect("registry path has a parent");
            let temporary = tempfile::NamedTempFile::new_in(parent)?;
            fs::write(temporary.path(), bytes)?;
            fs::set_permissions(temporary.path(), permissions.clone())?;
            temporary.persist(&self.path).map_err(|error| error.error)?;
        } else {
            match fs::remove_file(&self.path) {
                Ok(()) => (),
                Err(error) if error.kind() == io::ErrorKind::NotFound => (),
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

pub(super) struct GodotInstallTransaction {
    target: PathBuf,
    backup: PathBuf,
    had_target: bool,
    registries: Vec<FileSnapshot>,
    project: PathBuf,
    cache_prefixes: BTreeSet<String>,
    cache_snapshots: BTreeMap<PathBuf, FileSnapshot>,
    active: bool,
}

impl GodotInstallTransaction {
    pub(super) fn begin(
        target: &Path,
        backup: &Path,
        project: &Path,
        catalog: Option<&Path>,
    ) -> io::Result<Self> {
        let mut registries = vec![FileSnapshot::read(
            project.join(crate::project::PROJECT_MANIFEST_RELATIVE),
        )?];
        if let Some(catalog) = catalog {
            registries.push(FileSnapshot::read(
                catalog.join(crate::catalog::PROJECT_CATALOG_RELATIVE),
            )?);
        }
        let had_target = target.exists();
        if had_target {
            // The directory hash rejects symlinks; backups must never silently omit content.
            let expected = hash_directory(target)?;
            if backup.exists() {
                fs::remove_dir_all(backup)?;
            }
            copy_directory(target, backup)?;
            if hash_directory(backup)? != expected {
                return Err(io::Error::other(
                    "Godot target backup did not preserve all files",
                ));
            }
        }
        let cache_prefixes = crate::godot_cache::cache_prefixes_for_target(project, target)?;
        let mut cache_snapshots = BTreeMap::new();
        for relative in crate::godot_cache::cache_files_for_prefixes(project, &cache_prefixes)? {
            cache_snapshots.insert(
                relative.clone(),
                FileSnapshot::read(project.join(relative))?,
            );
        }
        Ok(Self {
            target: target.into(),
            backup: backup.into(),
            had_target,
            registries,
            project: project.into(),
            cache_prefixes,
            cache_snapshots,
            active: true,
        })
    }

    /// Called after copying incoming textures, before Godot can replace any cache outputs.
    pub(super) fn capture_incoming_cache(&mut self) -> io::Result<()> {
        self.cache_prefixes
            .extend(crate::godot_cache::cache_prefixes_for_target(
                &self.project,
                &self.target,
            )?);
        for relative in
            crate::godot_cache::cache_files_for_prefixes(&self.project, &self.cache_prefixes)?
        {
            if !self.cache_snapshots.contains_key(&relative) {
                let snapshot = FileSnapshot::read(self.project.join(&relative))?;
                self.cache_snapshots.insert(relative, snapshot);
            }
        }
        Ok(())
    }

    fn restore_texture_cache(&self) -> io::Result<()> {
        // Validate .godot/imported again before deleting or restoring anything through it.
        let current =
            crate::godot_cache::cache_files_for_prefixes(&self.project, &self.cache_prefixes)?;
        for relative in current {
            if !self.cache_snapshots.contains_key(&relative) {
                fs::remove_file(self.project.join(relative))?;
            }
        }
        if !self.cache_snapshots.is_empty() {
            fs::create_dir_all(crate::godot_cache::cache_directory(&self.project)?)?;
            for snapshot in self.cache_snapshots.values() {
                snapshot.restore()?;
            }
        }
        Ok(())
    }

    fn rollback(&mut self) -> io::Result<()> {
        let mut errors = Vec::new();
        let restore_target = || -> io::Result<()> {
            if self.target.exists() {
                fs::remove_dir_all(&self.target)?;
            }
            if self.had_target {
                copy_directory(&self.backup, &self.target)?;
            }
            Ok(())
        };
        if let Err(error) = restore_target() {
            errors.push(error.to_string());
        }
        if let Err(error) = self.restore_texture_cache() {
            errors.push(format!("texture cache restoration failed: {error}"));
        }
        for snapshot in &self.registries {
            if let Err(error) = snapshot.restore() {
                errors.push(error.to_string());
            }
        }
        if errors.is_empty() {
            self.active = false;
            Ok(())
        } else {
            Err(io::Error::other(errors.join("; ")))
        }
    }

    pub(super) fn finish<T>(
        mut self,
        result: Result<T, AutomationRunError>,
    ) -> Result<T, AutomationRunError> {
        match result {
            Ok(value) => {
                self.active = false;
                Ok(value)
            }
            Err(error) => {
                if let Err(rollback) = self.rollback() {
                    return Err(AutomationRunError::Processing(format!(
                        "{error}; automatic rollback failed: {rollback}; original target backup: {}", self.backup.display()
                    )));
                }
                Err(error)
            }
        }
    }
}

impl Drop for GodotInstallTransaction {
    fn drop(&mut self) {
        if self.active {
            let _ = self.rollback();
        }
    }
}

/// Write directly to logs to avoid pipe deadlock, and cancel/reap before restoring files.
pub(super) fn run_godot_process(
    command: &mut Command,
    job_dir: &Path,
    log_name: &str,
    store: &JobStore,
    job_id: &str,
) -> Result<Output, AutomationRunError> {
    check_cancelled(store, job_id)?;
    let stdout_path = job_dir.join(format!("logs/{log_name}.stdout.log"));
    let stderr_path = job_dir.join(format!("logs/{log_name}.stderr.log"));
    command.stdout(Stdio::from(File::create(&stdout_path)?));
    command.stderr(Stdio::from(File::create(&stderr_path)?));
    let mut child = command.spawn()?;
    let status = loop {
        let result =
            check_cancelled(store, job_id).and_then(|()| child.try_wait().map_err(Into::into));
        match result {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(error);
            }
        }
    };
    Ok(Output {
        status,
        stdout: fs::read(stdout_path)?,
        stderr: fs::read(stderr_path)?,
    })
}

pub(super) fn validate_godot_output(
    output: &Output,
    phase: Option<&str>,
    target: &Path,
    asset_type: &str,
) -> Result<(), AutomationRunError> {
    if !output.status.success() {
        return Err(AutomationRunError::Processing(format!(
            "Godot {} failed with status {}",
            phase.unwrap_or("asset import"),
            output.status
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if let Some(error) = stdout.lines().chain(stderr.lines()).find(|line| {
        let line = line.trim_start();
        line.starts_with("ERROR:")
            || line.starts_with("SCRIPT ERROR:")
            || line.starts_with("Parse Error:")
            || line.starts_with("FAIL Forge")
    }) {
        return Err(AutomationRunError::Processing(format!(
            "Godot reported an error despite successful exit: {error}"
        )));
    }
    if let Some(phase) = phase {
        let results = stdout
            .lines()
            .filter_map(|line| line.strip_prefix("FORGE_INSTALL_RESULT "))
            .collect::<Vec<_>>();
        if results.len() != 1 {
            return Err(AutomationRunError::Processing(
                "Godot did not emit exactly one structured installation result".into(),
            ));
        }
        let result: serde_json::Value = serde_json::from_str(results[0])?;
        let expected_target = format!("res://{}", target.to_string_lossy().replace('\\', "/"));
        if result["schemaVersion"] != "1"
            || result["status"] != "succeeded"
            || result["phase"] != phase
            || result["target"] != expected_target
            || result["assetType"] != asset_type
        {
            return Err(AutomationRunError::Processing(
                "Godot installation result does not match the requested delivery".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_restores_target_and_both_registries_on_error_and_drop() {
        for dropped in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let project = root.path().join("game");
            let target = project.join("addons/forge_assets/hero");
            let backup = root.path().join("backup");
            let catalog = root.path().join("catalog");
            fs::create_dir_all(&target).unwrap();
            fs::create_dir_all(project.join(".forge")).unwrap();
            fs::create_dir_all(catalog.join(".forge")).unwrap();
            fs::write(target.join(".forge-owned.json"), "original ownership").unwrap();
            fs::write(target.join("sprite.png"), "old pixels").unwrap();
            fs::write(project.join(".forge/assets.json"), "old manifest").unwrap();
            fs::write(catalog.join(".forge/catalog.json"), "old catalog").unwrap();
            let before = hash_directory(&target).unwrap();
            let transaction =
                GodotInstallTransaction::begin(&target, &backup, &project, Some(&catalog)).unwrap();
            fs::remove_dir_all(&target).unwrap();
            fs::create_dir_all(&target).unwrap();
            fs::write(target.join("partial.tscn"), "new broken scene").unwrap();
            fs::write(project.join(".forge/assets.json"), "new manifest").unwrap();
            fs::write(catalog.join(".forge/catalog.json"), "new catalog").unwrap();
            if dropped {
                drop(transaction);
            } else {
                let error = transaction
                    .finish::<()>(Err(AutomationRunError::Cancelled))
                    .unwrap_err();
                assert!(matches!(error, AutomationRunError::Cancelled));
            }
            assert_eq!(hash_directory(&target).unwrap(), before);
            assert_eq!(
                fs::read(project.join(".forge/assets.json")).unwrap(),
                b"old manifest"
            );
            assert_eq!(
                fs::read(catalog.join(".forge/catalog.json")).unwrap(),
                b"old catalog"
            );
        }
    }

    #[test]
    fn failed_first_install_removes_target_and_new_manifest() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".forge")).unwrap();
        let target = root.path().join("target");
        let transaction =
            GodotInstallTransaction::begin(&target, &root.path().join("backup"), root.path(), None)
                .unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(root.path().join(".forge/assets.json"), "new manifest").unwrap();
        transaction
            .finish::<()>(Err(io::Error::other("disk error").into()))
            .unwrap_err();
        assert!(!target.exists());
        assert!(!root.path().join(".forge/assets.json").exists());
    }

    #[cfg(unix)]
    #[test]
    fn rollback_restores_owned_cache_bytes_permissions_and_removes_only_new_variants() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let project = root.path();
        let target = project.join("addons/forge_assets/hero");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("sprite.png"), "old red source").unwrap();
        let prefix = crate::godot_cache::cache_prefixes_for_target(project, &target)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let cache = project.join(".godot/imported");
        fs::create_dir_all(&cache).unwrap();
        let texture = cache.join(format!("{prefix}.ctex"));
        let checksum = cache.join(format!("{prefix}.md5"));
        let unrelated = cache.join("sprite.png-unrelatedasset.ctex");
        fs::write(&texture, "old red cache").unwrap();
        fs::write(&checksum, "old checksum").unwrap();
        fs::write(&unrelated, "unrelated cache").unwrap();
        fs::set_permissions(&texture, Permissions::from_mode(0o440)).unwrap();
        let mut transaction =
            GodotInstallTransaction::begin(&target, &project.join("backup"), project, None)
                .unwrap();
        fs::write(target.join("sprite.png"), "new blue source").unwrap();
        transaction.capture_incoming_cache().unwrap();
        fs::set_permissions(&texture, Permissions::from_mode(0o600)).unwrap();
        fs::write(&texture, "new blue cache").unwrap();
        fs::write(&checksum, "new checksum").unwrap();
        let new_variant = cache.join(format!("{prefix}.s3tc.ctex"));
        fs::write(&new_variant, "new variant").unwrap();
        fs::write(&unrelated, "unrelated update must survive").unwrap();
        transaction
            .finish::<()>(Err(AutomationRunError::Cancelled))
            .unwrap_err();
        assert_eq!(fs::read_to_string(&texture).unwrap(), "old red cache");
        assert_eq!(fs::read_to_string(&checksum).unwrap(), "old checksum");
        assert_eq!(
            fs::metadata(&texture).unwrap().permissions().mode() & 0o777,
            0o440
        );
        assert!(!new_variant.exists());
        assert_eq!(
            fs::read_to_string(&unrelated).unwrap(),
            "unrelated update must survive"
        );
    }

    #[test]
    fn first_install_failure_removes_new_target_caches_before_any_sidecar_exists() {
        let root = tempfile::tempdir().unwrap();
        let project = root.path();
        let target = project.join("addons/forge_assets/hero");
        let mut transaction =
            GodotInstallTransaction::begin(&target, &project.join("backup"), project, None)
                .unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("new.png"), "new source").unwrap();
        transaction.capture_incoming_cache().unwrap();
        let prefix = crate::godot_cache::cache_prefixes_for_target(project, &target)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let cache = project.join(".godot/imported");
        fs::create_dir_all(&cache).unwrap();
        for extension in ["ctex", "md5"] {
            fs::write(cache.join(format!("{prefix}.{extension}")), "new bytes").unwrap();
        }
        transaction
            .finish::<()>(Err(AutomationRunError::Cancelled))
            .unwrap_err();
        assert!(!target.exists());
        assert_eq!(fs::read_dir(cache).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn output_protocol_rejects_false_success_but_accepts_warnings() {
        use std::os::unix::process::ExitStatusExt;
        let mut output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: Vec::new(),
            stderr: Vec::new(),
        };
        let target = Path::new("addons/forge_assets/hero");
        assert!(validate_godot_output(&output, Some("install"), target, "animation").is_err());
        output.stdout = b"FORGE_INSTALL_RESULT {\"schemaVersion\":\"1\",\"status\":\"succeeded\",\"phase\":\"install\",\"target\":\"res://addons/forge_assets/hero\",\"assetType\":\"animation\"}\n".to_vec();
        output.stderr = b"WARNING: ordinary compatibility warning\n".to_vec();
        validate_godot_output(&output, Some("install"), target, "animation").unwrap();
        for text in [
            "ERROR: parse failed",
            "SCRIPT ERROR: invalid call",
            "FAIL Forge Godot install: texture missing",
        ] {
            output.stderr = text.as_bytes().into();
            assert!(validate_godot_output(&output, Some("install"), target, "animation").is_err());
        }
    }
}
