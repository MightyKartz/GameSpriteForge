//! Godot 4.6 texture cache ownership, shared by transactions and read-only audits.
//!
//! MD5 is only Godot's resource-path filename convention, never an integrity hash.
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) fn cache_files_for_target(project: &Path, target: &Path) -> io::Result<Vec<PathBuf>> {
    cache_files_for_prefixes(project, &cache_prefixes_for_target(project, target)?)
}

pub(crate) fn cache_directory(project: &Path) -> io::Result<PathBuf> {
    let mut cursor = fs::canonicalize(project)?;
    for part in [".godot", "imported"] {
        cursor.push(part);
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                return Err(invalid(
                    "Godot imported cache must use regular directories without symbolic links",
                ));
            }
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    Ok(cursor)
}

pub(crate) fn cache_prefixes_for_target(
    project: &Path,
    target: &Path,
) -> io::Result<BTreeSet<String>> {
    let canonical_project = fs::canonicalize(project)?;
    let relative = target
        .strip_prefix(project)
        .or_else(|_| target.strip_prefix(&canonical_project))
        .map_err(|_| invalid("cache target must be inside the project"))?;
    if !crate::delivery::safe_relative(relative) {
        return Err(invalid("unsafe target path for Godot cache"));
    }
    let project = canonical_project;
    let mut target = project.clone();
    for part in relative.components() {
        target.push(part);
        match fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(invalid("cache target may not traverse symbolic links"))
            }
            Ok(_) => (),
            Err(error) if error.kind() == io::ErrorKind::NotFound => (),
            Err(error) => return Err(error),
        }
    }
    let mut prefixes = BTreeSet::new();
    if !target.exists() {
        return Ok(prefixes);
    }
    for entry in crate::delivery::inventory(&target)? {
        let path = target.join(&entry.path);
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if extension.eq_ignore_ascii_case("png") {
            prefixes.insert(texture_prefix(&project, &path)?);
        } else if extension == "import" {
            let prefix = texture_prefix(&project, &path.with_extension(""))?;
            for value in quoted_strings(&fs::read_to_string(&path)?)? {
                if let Some(cache) = value.strip_prefix("res://.godot/") {
                    let relative_cache = Path::new(cache);
                    if !crate::delivery::safe_relative(relative_cache)
                        || relative_cache.components().count() != 2
                        || relative_cache.parent() != Some(Path::new("imported"))
                        || !relative_cache
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| is_texture_cache_name(name, &prefix))
                    {
                        return Err(invalid("texture .import references a cache outside its own resource-path prefix"));
                    }
                }
            }
            prefixes.insert(prefix);
        }
    }
    Ok(prefixes)
}

pub(crate) fn cache_files_for_prefixes(
    project: &Path,
    prefixes: &BTreeSet<String>,
) -> io::Result<Vec<PathBuf>> {
    let cache = cache_directory(project)?;
    let entries = match fs::read_dir(&cache) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !prefixes
            .iter()
            .any(|prefix| is_texture_cache_name(name, prefix))
        {
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(invalid(
                "owned Godot cache entry is not a regular file or is a symbolic link",
            ));
        }
        files.push(Path::new(".godot/imported").join(name));
    }
    files.sort();
    Ok(files)
}

fn texture_prefix(project: &Path, source: &Path) -> io::Result<String> {
    let relative = source
        .strip_prefix(project)
        .map_err(|_| invalid("texture is outside the project"))?;
    if !crate::delivery::safe_relative(relative) {
        return Err(invalid("unsafe texture resource path"));
    }
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid("texture filename must be UTF-8"))?;
    let resource_path = format!(
        "res://{}",
        relative
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/")
    );
    Ok(format!(
        "{name}-{:x}",
        md5::compute(resource_path.as_bytes())
    ))
}

fn is_texture_cache_name(name: &str, prefix: &str) -> bool {
    let Some(suffix) = name.strip_prefix(prefix) else {
        return false;
    };
    if matches!(suffix, ".ctex" | ".md5") {
        return true;
    }
    // Godot can produce multiple compression variants, e.g. .s3tc.ctex / .etc2.ctex.
    suffix
        .strip_prefix('.')
        .and_then(|value| value.strip_suffix(".ctex"))
        .is_some_and(|variant| {
            !variant.is_empty()
                && variant
                    .bytes()
                    .all(|value| value.is_ascii_alphanumeric() || value == b'_')
        })
}

fn quoted_strings(text: &str) -> io::Result<Vec<String>> {
    let bytes = text.as_bytes();
    let mut strings = Vec::new();
    let mut position = 0;
    while position < bytes.len() {
        if bytes[position] != b'"' {
            position += 1;
            continue;
        }
        let start = position;
        position += 1;
        while position < bytes.len() {
            match bytes[position] {
                b'\\' => position += 2,
                b'"' => {
                    position += 1;
                    break;
                }
                _ => position += 1,
            }
        }
        if position > bytes.len() {
            return Err(invalid("invalid quoted string in texture .import"));
        }
        strings.push(serde_json::from_str(&text[start..position]).map_err(invalid)?);
    }
    Ok(strings)
}

fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_godot_path_prefix_isolates_same_named_textures_and_formats() {
        let root = tempfile::tempdir().unwrap();
        let project = fs::canonicalize(root.path()).unwrap();
        let target = project.join("addons/forge_assets/cache-fixture/items");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("stone.png"), "source bytes").unwrap();
        let prefix = texture_prefix(&project, &target.join("stone.png")).unwrap();
        assert_eq!(prefix, "stone.png-6677c6701bfa908aeb0e0aaf62840226");
        let cache = project.join(".godot/imported");
        fs::create_dir_all(&cache).unwrap();
        for name in [
            format!("{prefix}.ctex"),
            format!("{prefix}.md5"),
            format!("{prefix}.s3tc.ctex"),
            format!("{prefix}.txt"),
            "stone.png-otherasset.ctex".into(),
        ] {
            fs::write(cache.join(name), "cache bytes").unwrap();
        }
        let files = cache_files_for_target(&project, &target).unwrap();
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|path| path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with(&prefix)));
    }

    #[test]
    fn imported_sidecar_cannot_claim_another_assets_cache_or_escape() {
        let root = tempfile::tempdir().unwrap();
        let project = fs::canonicalize(root.path()).unwrap();
        let target = project.join("addons/forge_assets/hero");
        fs::create_dir_all(&target).unwrap();
        for value in [
            "res://.godot/imported/other.png-01234567890123456789012345678901.ctex",
            "res://.godot/imported/../../outside.ctex",
        ] {
            fs::write(
                target.join("hero.png.import"),
                format!("[remap]\npath={value:?}\n"),
            )
            .unwrap();
            assert!(cache_files_for_target(&project, &target).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn cache_paths_may_not_follow_symbolic_links() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let project = fs::canonicalize(root.path()).unwrap();
        let target = project.join("addons/forge_assets/hero");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("hero.png"), "image").unwrap();
        let outside = project.join("outside");
        fs::create_dir(&outside).unwrap();
        symlink(&outside, project.join(".godot")).unwrap();
        assert!(cache_files_for_target(&project, &target).is_err());
        fs::remove_file(project.join(".godot")).unwrap();
        let cache = project.join(".godot/imported");
        fs::create_dir_all(&cache).unwrap();
        let prefix = texture_prefix(&project, &target.join("hero.png")).unwrap();
        symlink(&outside, cache.join(format!("{prefix}.ctex"))).unwrap();
        assert!(cache_files_for_target(&project, &target).is_err());
    }
}
