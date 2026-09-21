//! Explicit, temporary filesystem probes. Never weakens publication semantics.
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Check {
    supported: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Check {
    fn run(operation: impl FnOnce() -> io::Result<()>) -> Self {
        match operation() {
            Ok(()) => Self {
                supported: true,
                error: None,
            },
            Err(error) => Self {
                supported: false,
                error: Some(error.to_string()),
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageReport {
    schema_version: &'static str,
    path: PathBuf,
    pub supported: bool,
    exclusive_publication: Check,
    existing_file_protection: Check,
    replacement: Check,
    hard_links: Check,
    file_locking: Check,
    temporary_files_removed: bool,
    notes: Vec<&'static str>,
}

fn staged(root: &Path, bytes: &[u8]) -> io::Result<tempfile::NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new_in(root)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    Ok(file)
}

/// The caller explicitly selects an existing directory for a bounded write probe.
/// Every test uses only a newly-created private subdirectory, removed before return.
pub fn check(path: &Path) -> io::Result<StorageReport> {
    let path = path.canonicalize()?;
    if !path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "storage check requires an existing directory",
        ));
    }
    let scratch = tempfile::Builder::new()
        .prefix(".forge-storage-check-")
        .tempdir_in(&path)?;
    let root = scratch.path();
    let exclusive_publication = Check::run(|| {
        let target = root.join("published");
        staged(root, b"complete output")?
            .persist_noclobber(&target)
            .map_err(|e| e.error)?;
        if fs::read(target)? != b"complete output" {
            return Err(io::Error::other("published content mismatch"));
        }
        Ok(())
    });
    let existing_file_protection = Check::run(|| {
        let target = root.join("protected");
        fs::write(&target, b"original")?;
        let denied = staged(root, b"replacement")?
            .persist_noclobber(&target)
            .is_err();
        if !denied || fs::read(target)? != b"original" {
            return Err(io::Error::other(
                "exclusive publication replaced an existing file",
            ));
        }
        Ok(())
    });
    let replacement = Check::run(|| {
        let target = root.join("replace");
        fs::write(&target, b"old")?;
        staged(root, b"new")?
            .persist(&target)
            .map_err(|e| e.error)?;
        if fs::read(target)? != b"new" {
            return Err(io::Error::other("replacement content mismatch"));
        }
        Ok(())
    });
    // Standard receipt publication currently uses hard links, independently of
    // tempfile's platform-specific exclusive-publication implementation.
    let hard_links = Check::run(|| {
        let source = root.join("link-source");
        let target = root.join("link-target");
        fs::write(&source, b"receipt")?;
        fs::hard_link(&source, &target)?;
        fs::remove_file(source)?;
        if fs::read(target)? != b"receipt" {
            return Err(io::Error::other("hard-link content mismatch"));
        }
        Ok(())
    });
    let file_locking = Check::run(|| {
        let target = root.join("lock");
        let first = File::create(&target)?;
        let second = fs::OpenOptions::new().read(true).write(true).open(target)?;
        first
            .try_lock()
            .map_err(|e| io::Error::other(e.to_string()))?;
        match second.try_lock() {
            Err(fs::TryLockError::WouldBlock) => (),
            _ => {
                return Err(io::Error::other(
                    "a second file handle did not observe exclusive lock contention",
                ))
            }
        }
        drop(first);
        second
            .try_lock()
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(())
    });
    let supported = [
        &exclusive_publication,
        &existing_file_protection,
        &replacement,
        &hard_links,
        &file_locking,
    ]
    .iter()
    .all(|check| check.supported);
    // Report cleanup failure instead of claiming the write probe left no files.
    scratch.close()?;
    Ok(StorageReport {
        schema_version: "1", path, supported, exclusive_publication,
        existing_file_protection, replacement, hard_links, file_locking,
        temporary_files_removed: true,
        notes: vec![
            "This explicit check created and removed only its own temporary subdirectory.",
            "supported covers the tested operations at this path now, not power-loss durability or a complete Godot installation.",
            "If unsupported, select a supported working/output directory; do not bypass ownership, locks or final-project verification with recursive copies.",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_leaves_caller_files_and_directory_inventory_unchanged() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("keep"), b"consumer bytes").unwrap();
        let report = check(root.path()).unwrap();
        assert!(report.supported, "{report:?}");
        assert!(report.temporary_files_removed);
        assert_eq!(
            fs::read(root.path().join("keep")).unwrap(),
            b"consumer bytes"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn missing_paths_and_files_are_not_created_or_replaced() {
        let root = tempfile::tempdir().unwrap();
        assert!(check(&root.path().join("absent")).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
        let file = root.path().join("file");
        fs::write(&file, b"original").unwrap();
        assert!(check(&file).is_err());
        assert_eq!(fs::read(file).unwrap(), b"original");
    }
}
