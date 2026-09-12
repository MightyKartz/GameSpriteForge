use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::{AssetInput, AutomationOperation};

/// Complete local input closure. Directory Packs include every file and reject symlinks.
pub fn local_source_files(operation: &AutomationOperation) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    match operation {
        AutomationOperation::PrepareAudio(request) => {
            paths.extend(request.items.iter().map(|item| item.path.clone()));
        }
        AutomationOperation::PrepareStatic(request) => {
            paths.extend(request.items.iter().map(|item| item.path.clone()));
        }
        AutomationOperation::PrepareAsset(request) => input_files(&request.input, &mut paths)?,
        AutomationOperation::PrepareCharacterPack(request) => {
            for animation in &request.animations {
                input_files(&animation.input, &mut paths)?;
            }
        }
        _ => return Ok(paths),
    }
    let mut canonical = BTreeSet::new();
    for path in paths {
        if !path.is_file() {
            return Err(format!("source is not a regular file: {}", path.display()));
        }
        canonical.insert(fs::canonicalize(&path).map_err(|e| format!("{}: {e}", path.display()))?);
    }
    Ok(canonical.into_iter().collect())
}

fn input_files(input: &AssetInput, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    match input {
        AssetInput::PngSequence { paths: inputs } => paths.extend(inputs.iter().cloned()),
        AssetInput::SpriteSheet { path, .. } | AssetInput::VideoClip { path, .. } => {
            paths.push(path.clone())
        }
        AssetInput::Gsfpack { path } => directory_files(path, paths)?,
    }
    Ok(())
}

fn directory_files(path: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!(
            "source Pack contains symbolic link: {}",
            path.display()
        ));
    }
    if metadata.is_file() {
        paths.push(path.to_path_buf());
    } else if metadata.is_dir() {
        for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
            directory_files(&entry.map_err(|e| e.to_string())?.path(), paths)?;
        }
    } else {
        return Err(format!("unsupported source file: {}", path.display()));
    }
    Ok(())
}

pub fn validate_source_locks(operation: &AutomationOperation) -> Result<(), String> {
    let locks = match operation {
        AutomationOperation::PrepareAudio(request) => &request.source_locks,
        AutomationOperation::PrepareStatic(request) => &request.source_locks,
        AutomationOperation::PrepareAsset(request) => &request.source_locks,
        AutomationOperation::PrepareCharacterPack(request) => &request.source_locks,
        _ => return Ok(()),
    };
    if locks.is_empty() {
        return Ok(());
    }
    let mut expected = BTreeMap::new();
    for lock in locks {
        if lock.sha256.len() != 64 || !lock.sha256.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(format!(
                "sourceLocks SHA-256 must contain 64 hex digits: {}",
                lock.path.display()
            ));
        }
        let path = fs::canonicalize(&lock.path)
            .map_err(|e| format!("sourceLocks {}: {e}", lock.path.display()))?;
        if expected
            .insert(path, lock.sha256.to_ascii_lowercase())
            .is_some()
        {
            return Err("sourceLocks contains duplicate paths".into());
        }
    }
    let sources = local_source_files(operation)?;
    if sources.iter().collect::<BTreeSet<_>>() != expected.keys().collect::<BTreeSet<_>>() {
        return Err(
            "sourceLocks must cover every local source file exactly once, without unrelated paths"
                .into(),
        );
    }
    for path in sources {
        let actual = crate::delivery::hash_file(&path).map_err(|e| e.to_string())?;
        if expected[&path] != actual {
            return Err(format!(
                "reviewed source SHA-256 mismatch: {} (expected {}, actual {actual})",
                path.display(),
                expected[&path]
            ));
        }
    }
    Ok(())
}
