//! Platform-independent identities for retained resource bytes.
//!
//! This is a new, explicitly named algorithm. Plans, Packs and old receipts
//! keep their existing directory-hash-v2 identity; never reinterpret that hash.
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONTENT_ALGORITHM: &str = "forge-content-inventory-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentFile {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContentInventory {
    pub algorithm: String,
    pub sha256: String,
    pub files: Vec<ContentFile>,
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

/// Validate shared paths without silently changing the spelling of a file.
pub fn validate_relative(path: &str) -> io::Result<()> {
    for part in path.split('/') {
        let base = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
            || matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && matches!(base.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(invalid(format!("non-portable resource path: {path:?}")));
        }
    }
    Ok(())
}

pub fn relative_path(path: &Path) -> io::Result<String> {
    let parts = path
        .components()
        .map(|component| match component {
            Component::Normal(part) => part
                .to_str()
                .ok_or_else(|| invalid("resource paths must be UTF-8")),
            _ => Err(invalid("resource paths must be relative normal components")),
        })
        .collect::<io::Result<Vec<_>>>()?;
    let result = parts.join("/");
    validate_relative(&result)?;
    Ok(result)
}

pub fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

/// Hash a complete file inventory sorted by exact UTF-8 portable path bytes.
/// Empty directories and machine locations are not part of content identity.
pub fn digest_inventory(files: &[ContentFile]) -> io::Result<String> {
    let mut sorted: Vec<_> = files.iter().collect();
    sorted.sort_by(|a, b| a.path.cmp(&b.path));
    let mut spellings = BTreeMap::new();
    let mut file_paths = std::collections::BTreeSet::new();
    let mut digest = Sha256::new();
    digest.update(CONTENT_ALGORITHM.as_bytes());
    digest.update([0]);
    for file in sorted {
        validate_relative(&file.path)?;
        if file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("expected a lowercase SHA-256 file digest"));
        }
        if !file_paths.insert(file.path.clone()) {
            return Err(invalid(format!("duplicate file: {}", file.path)));
        }
        let mut prefix = String::new();
        for part in file.path.split('/') {
            if !prefix.is_empty() {
                if file_paths.contains(&prefix) {
                    return Err(invalid(format!("file is also a directory: {prefix}")));
                }
                prefix.push('/');
            }
            prefix.push_str(part);
            if let Some(old) = spellings.insert(prefix.to_lowercase(), prefix.clone()) {
                if old != prefix {
                    return Err(invalid(format!("case-colliding paths: {old}, {prefix}")));
                }
            }
        }
        digest.update(b"file\0");
        digest.update((file.path.len() as u64).to_le_bytes());
        digest.update(file.path.as_bytes());
        digest.update(file.bytes.to_le_bytes());
        digest.update(file.sha256.as_bytes());
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub fn directory_inventory(root: &Path) -> io::Result<ContentInventory> {
    fn visit(root: &Path, path: &Path, files: &mut Vec<ContentFile>) -> io::Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if is_link(&metadata) {
            return Err(invalid(format!(
                "resource contains a link: {}",
                path.display()
            )));
        }
        if metadata.is_dir() {
            for entry in fs::read_dir(path)? {
                let child = entry?.path();
                relative_path(
                    child
                        .strip_prefix(root)
                        .map_err(|e| invalid(e.to_string()))?,
                )?;
                visit(root, &child, files)?;
            }
        } else if metadata.is_file() {
            // Use the same opened file for both byte count and streaming hash.
            let mut reader = fs::File::open(path)?;
            let mut hasher = Sha256::new();
            let bytes = io::copy(&mut reader, &mut hasher)?;
            files.push(ContentFile {
                path: relative_path(
                    path.strip_prefix(root)
                        .map_err(|e| invalid(e.to_string()))?,
                )?,
                bytes,
                sha256: format!("{:x}", hasher.finalize()),
            });
        } else {
            return Err(invalid(
                "resource must contain only regular files and directories",
            ));
        }
        Ok(())
    }
    if !root.is_dir() {
        return Err(invalid("resource root must be a directory"));
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ContentInventory {
        algorithm: CONTENT_ALGORITHM.into(),
        sha256: digest_inventory(&files)?,
        files,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyPathStyle {
    Posix,
    Windows,
}

/// Recompute the historical v2 hash with the ORIGINAL producer's path style.
/// Callers must retain the original hash/style evidence; a matching recomputed
/// value alone is not proof of origin. No receipt or Pack bytes are rewritten.
pub fn legacy_directory_digest(root: &Path, style: LegacyPathStyle) -> io::Result<String> {
    // Validate portable names, collisions and links before reading legacy bytes.
    directory_inventory(root)?;
    let mut digest = Sha256::new();
    digest.update(b"forge-directory-hash-v2\0");
    // v2 uses Path ordering (components), NOT the v1 content inventory ordering.
    for file in crate::delivery::inventory(root)? {
        let portable = relative_path(&file.path)?;
        let name = match style {
            LegacyPathStyle::Posix => portable,
            LegacyPathStyle::Windows => portable.replace('/', "\\"),
        };
        let bytes = fs::read(root.join(file.path))?;
        digest.update(b"file\0");
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(&bytes);
    }
    Ok(format!("{:x}", digest.finalize()))
}
