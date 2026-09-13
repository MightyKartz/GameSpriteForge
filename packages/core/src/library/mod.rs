//! Project-scoped, immutable resource records behind a single atomic catalog head.
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::catalog::{
    CatalogError, CatalogInstallRefV1, ProjectCatalogEntryV2, ProjectCatalogV1, ProjectCatalogV2,
    PROJECT_CATALOG_RELATIVE,
};
use crate::content_digest::{directory_inventory, is_link, validate_relative};

const OBJECTS: &str = ".forge/library/objects";
const IDENTITY: &str = ".forge/library/identity.json";
const LOCAL: &str = ".forge/library/local.json";
const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;

pub mod delivery;
pub mod finalize;
pub mod intake;
mod types;
pub use types::{
    AssetRecord, AssetRevision, InstallReference, LibraryCatalog, Location, MigrationAsset,
    MigrationPreview,
};
use types::{Identity, LocalConfig};

fn invalid(message: impl Into<String>) -> CatalogError {
    CatalogError::Invalid(message.into())
}

fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// Check every storage component, including Windows junctions. Read-only calls
/// do not create absent directories or silently follow links.
pub(crate) fn storage_path(root: &Path, relative: &str) -> Result<PathBuf, CatalogError> {
    validate_relative(relative)?;
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) if is_link(&metadata) => return Err(CatalogError::Symlink),
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(path)
}

fn read_bytes(root: &Path, relative: &str) -> Result<Vec<u8>, CatalogError> {
    let path = storage_path(root, relative)?;
    if fs::metadata(&path)?.len() > MAX_METADATA_BYTES {
        return Err(invalid("library metadata exceeds 16 MiB"));
    }
    Ok(fs::read(path)?)
}

fn read_json<T: DeserializeOwned>(root: &Path, relative: &str) -> Result<T, CatalogError> {
    Ok(serde_json::from_slice(&read_bytes(root, relative)?)?)
}

/// Validate the marker as well as the head, so a legacy writer cannot silently
/// turn a migrated library back into a writable V2 catalog.
pub fn is_library(root: &Path) -> Result<bool, CatalogError> {
    let path = storage_path(root, PROJECT_CATALOG_RELATIVE)?;
    if !path.exists() {
        if storage_path(root, IDENTITY)?.exists() {
            return Err(invalid(
                "library initialization was interrupted; repeat initialization or migration",
            ));
        }
        return Ok(false);
    }
    let value: serde_json::Value =
        serde_json::from_slice(&read_bytes(root, PROJECT_CATALOG_RELATIVE)?)?;
    let version = value["schemaVersion"]
        .as_str()
        .ok_or_else(|| invalid("schemaVersion is required"))?;
    if version != "3" && storage_path(root, IDENTITY)?.exists() {
        return Err(invalid("library head was overwritten or migration interrupted; restore the head or repeat the original migration"));
    }
    Ok(version == "3")
}

pub fn read_catalog(root: &Path) -> Result<LibraryCatalog, CatalogError> {
    let catalog: LibraryCatalog = read_json(root, PROJECT_CATALOG_RELATIVE)?;
    if catalog.schema_version != "3" {
        return Err(CatalogError::UnsupportedVersion(catalog.schema_version));
    }
    if catalog.project_id.trim().is_empty()
        || catalog.name.trim().is_empty()
        || catalog
            .migrated_from_sha256
            .as_ref()
            .is_some_and(|v| !valid_digest(v))
    {
        return Err(invalid("invalid library identity fields"));
    }
    let identity: Identity = read_json(root, IDENTITY)?;
    if catalog.project_id != identity.project_id
        || catalog.migrated_from_sha256 != identity.migrated_from_sha256
    {
        return Err(invalid("library identity does not match catalog head"));
    }
    Ok(catalog)
}

pub fn read_object<T: DeserializeOwned>(root: &Path, digest: &str) -> Result<T, CatalogError> {
    if !valid_digest(digest) {
        return Err(invalid("invalid library object digest"));
    }
    let bytes = read_bytes(root, &format!("{OBJECTS}/{digest}.json"))?;
    if sha(&bytes) != digest {
        return Err(invalid(format!(
            "library object integrity mismatch: {digest}"
        )));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn write_object<T: Serialize>(root: &Path, value: &T) -> Result<String, CatalogError> {
    let bytes = serde_json::to_vec(value)?;
    check_metadata_size(&bytes)?;
    let digest = sha(&bytes);
    let path = storage_path(root, &format!("{OBJECTS}/{digest}.json"))?;
    fs::create_dir_all(path.parent().unwrap())?;
    if path.exists() {
        if fs::read(&path)? != bytes {
            return Err(invalid("existing library object is corrupt"));
        }
        return Ok(digest);
    }
    let mut staging = tempfile::NamedTempFile::new_in(path.parent().unwrap())?;
    staging.write_all(&bytes)?;
    staging.as_file().sync_all()?;
    staging
        .persist_noclobber(path)
        .map_err(|error| CatalogError::Io(error.error))?;
    Ok(digest)
}

fn write_json<T: Serialize>(root: &Path, relative: &str, value: &T) -> Result<(), CatalogError> {
    let bytes = serde_json::to_vec_pretty(value)?;
    check_metadata_size(&bytes)?;
    let path = storage_path(root, relative)?;
    crate::catalog::write_bytes_atomic(&path, &bytes)
}

fn check_metadata_size(bytes: &[u8]) -> Result<(), CatalogError> {
    if bytes.len() as u64 > MAX_METADATA_BYTES {
        return Err(invalid(
            "library metadata exceeds 16 MiB; no metadata was committed",
        ));
    }
    Ok(())
}

fn commit_head(root: &Path, expected: &[u8], catalog: &LibraryCatalog) -> Result<(), CatalogError> {
    if read_bytes(root, PROJECT_CATALOG_RELATIVE)? != expected {
        return Err(invalid(
            "catalog changed during transaction; no head update was committed",
        ));
    }
    write_json(root, PROJECT_CATALOG_RELATIVE, catalog)
}

/// Validate all referenced metadata before fingerprinting the head. The head
/// hashes immutable records transitively; local root bindings also affect reads.
pub fn snapshot_sha256(root: &Path) -> Result<String, CatalogError> {
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let catalog = read_catalog(root)?;
    for id in catalog.assets.keys() {
        let asset = read_asset(root, &catalog, id)?;
        for digest in &asset.revisions {
            read_revision(root, &asset, digest)?;
        }
    }
    if read_bytes(root, PROJECT_CATALOG_RELATIVE)? != head {
        return Err(invalid("catalog changed while reading its snapshot"));
    }
    let mut bytes = head;
    if storage_path(root, LOCAL)?.exists() {
        bytes.extend_from_slice(&read_bytes(root, LOCAL)?);
    }
    Ok(sha(&bytes))
}

pub fn read_asset(
    root: &Path,
    catalog: &LibraryCatalog,
    id: &str,
) -> Result<AssetRecord, CatalogError> {
    let digest = catalog
        .assets
        .get(id)
        .ok_or_else(|| invalid(format!("asset not found: {id}")))?;
    let asset: AssetRecord = read_object(root, digest)?;
    if asset.asset_id != id
        || id.trim().is_empty()
        || asset.revisions.is_empty()
        || asset.revisions.iter().any(|v| !valid_digest(v))
        || asset
            .revisions
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != asset.revisions.len()
    {
        return Err(invalid("invalid asset record identity or empty history"));
    }
    for selected in [&asset.selected_revision, &asset.build_revision]
        .into_iter()
        .flatten()
    {
        if !asset.revisions.contains(selected) {
            return Err(invalid("asset reference is absent from history"));
        }
    }
    for installed in &asset.installations {
        if !asset.revisions.contains(&installed.revision) {
            return Err(invalid("installation refers to an unknown revision"));
        }
        match (&installed.snapshot_text, &installed.snapshot_sha256) {
            (Some(text), Some(digest)) => {
                if sha(text.as_bytes()) != *digest
                    || installed
                        .install_job_id
                        .as_ref()
                        .is_none_or(|id| id.is_empty())
                {
                    return Err(invalid("installation snapshot identity is inconsistent"));
                }
                let _: crate::delivery::InstallSnapshot = serde_json::from_str(text)?;
            }
            (None, None) => (),
            _ => {
                return Err(invalid(
                    "installation snapshot text and digest must be recorded together",
                ))
            }
        }
    }
    Ok(asset)
}

pub fn read_revision(
    root: &Path,
    asset: &AssetRecord,
    digest: &str,
) -> Result<AssetRevision, CatalogError> {
    if !asset.revisions.iter().any(|item| item == digest) {
        return Err(invalid("revision does not belong to the asset"));
    }
    let revision: AssetRevision = read_object(root, digest)?;
    if revision.schema_version != "1"
        || revision.asset_id != asset.asset_id
        || revision
            .legacy
            .as_ref()
            .is_some_and(|entry| entry.asset_id != asset.asset_id)
    {
        return Err(invalid("invalid revision schema or asset identity"));
    }
    if let Some(content) = &revision.content {
        if content.algorithm != crate::content_digest::CONTENT_ALGORITHM
            || crate::content_digest::digest_inventory(&content.files)? != content.sha256
        {
            return Err(invalid("revision content inventory is inconsistent"));
        }
    }
    Ok(revision)
}

fn local_config(root: &Path) -> Result<LocalConfig, CatalogError> {
    if storage_path(root, LOCAL)?.exists() {
        read_json(root, LOCAL)
    } else {
        Ok(LocalConfig::default())
    }
}

fn locate(root: &Path, path: &Path, config: &mut LocalConfig) -> Result<Location, CatalogError> {
    let absolute_root = fs::canonicalize(root)?;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        absolute_root.join(path)
    };
    if let Ok(relative) = absolute
        .strip_prefix(&absolute_root)
        .or_else(|_| absolute.strip_prefix(root))
    {
        if relative.as_os_str().is_empty() {
            return Ok(Location {
                root_id: "project".into(),
                path: String::new(),
            });
        }
        if let Ok(path) = crate::content_digest::relative_path(relative) {
            return Ok(Location {
                root_id: "project".into(),
                path,
            });
        }
    }
    // The binding is machine-local. The root ID remains stable when users bind
    // this same portable record to another directory on another machine.
    let root_id = format!("external-{}", sha(absolute.to_string_lossy().as_bytes()));
    // A Windows absolute path read on POSIX is not relative to this project.
    // Keep it unresolved; the original spelling remains in the migration backup.
    let spelling = path.to_string_lossy();
    if !(cfg!(not(windows))
        && (spelling.as_bytes().get(1) == Some(&b':') || spelling.starts_with("\\\\")))
    {
        config.roots.insert(root_id.clone(), absolute);
    }
    Ok(Location {
        root_id,
        path: String::new(),
    })
}

pub fn resolve_location(root: &Path, location: &Location) -> Result<PathBuf, CatalogError> {
    let base = if location.root_id == "project" {
        fs::canonicalize(root)?
    } else {
        local_config(root)?
            .roots
            .get(&location.root_id)
            .cloned()
            .ok_or_else(|| {
                invalid(format!(
                    "resource root is not configured: {}",
                    location.root_id
                ))
            })?
    };
    if location.path.is_empty() {
        return Ok(base);
    }
    validate_relative(&location.path)?;
    storage_path(&base, &location.path)
}

fn identity_for(
    root: &Path,
    name: &str,
    original: Option<String>,
) -> Result<Identity, CatalogError> {
    if name.trim().is_empty() {
        return Err(invalid("library name is required"));
    }
    if storage_path(root, IDENTITY)?.exists() {
        let identity: Identity = read_json(root, IDENTITY)?;
        if identity.migrated_from_sha256 != original || identity.name != name {
            return Err(invalid(
                "existing library identity conflicts with initialization",
            ));
        }
        return Ok(identity);
    }
    let identity = Identity {
        project_id: if root.join(crate::asset_project::FORGE_PROJECT_FILE).exists() {
            crate::asset_project::read_project(root)
                .map_err(|e| invalid(e.to_string()))?
                .project_id
        } else {
            Uuid::new_v4().to_string()
        },
        name: name.into(),
        migrated_from_sha256: original,
    };
    write_json(root, IDENTITY, &identity)?;
    Ok(identity)
}

fn empty_catalog(identity: &Identity) -> LibraryCatalog {
    LibraryCatalog {
        schema_version: "3".into(),
        project_id: identity.project_id.clone(),
        name: identity.name.clone(),
        updated_at: Utc::now(),
        assets: BTreeMap::new(),
        migrated_from_sha256: identity.migrated_from_sha256.clone(),
    }
}

fn write_ignore(root: &Path) -> Result<(), CatalogError> {
    // The file lives inside library storage and is owned by this subsystem.
    let path = storage_path(root, ".forge/library/.gitignore")?;
    if !path.exists() {
        fs::write(path, "local.json\nbackups/\ncache/\nmedia/\n")?;
    }
    Ok(())
}

pub fn initialize(root: &Path, name: &str) -> Result<LibraryCatalog, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let path = storage_path(root, PROJECT_CATALOG_RELATIVE)?;
    if path.exists() {
        return Err(invalid(
            "catalog already exists; use explicit migration for V1/V2",
        ));
    }
    let identity = identity_for(root, name, None)?;
    let catalog = empty_catalog(&identity);
    write_ignore(root)?;
    write_json(root, PROJECT_CATALOG_RELATIVE, &catalog)?;
    Ok(catalog)
}

fn legacy_catalog(bytes: &[u8]) -> Result<ProjectCatalogV2, CatalogError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    match value["schemaVersion"].as_str() {
        Some("1") => Ok(ProjectCatalogV2::from(serde_json::from_slice::<
            ProjectCatalogV1,
        >(bytes)?)),
        Some("2") => Ok(serde_json::from_slice(bytes)?),
        _ => Err(invalid("migration requires an existing V1 or V2 catalog")),
    }
}

pub fn migration_preview(root: &Path) -> Result<MigrationPreview, CatalogError> {
    let bytes = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let old = legacy_catalog(&bytes)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let mut assets = Vec::new();
    for (id, entry) in old.assets {
        if id != entry.asset_id {
            return Err(invalid("legacy catalog key differs from assetId"));
        }
        let path = if entry.pack_path.is_absolute() {
            entry.pack_path.clone()
        } else {
            root.join(&entry.pack_path)
        };
        let mut issues = Vec::new();
        let content = match directory_inventory(&path) {
            Ok(content) => {
                if crate::delivery::directory_sha256(&path)? != entry.pack_sha256 {
                    issues.push("recorded Pack digest does not match native bytes; legacy path style may differ".into());
                }
                Some(content.sha256)
            }
            Err(error) => {
                issues.push(format!("Pack unavailable or not portable: {error}"));
                None
            }
        };
        assets.push(MigrationAsset {
            asset_id: id,
            recorded_pack_sha256: entry.pack_sha256,
            content_sha256: content,
            issues,
        });
    }
    let mut preview = MigrationPreview {
        source_version: value["schemaVersion"].as_str().unwrap().into(),
        catalog_sha256: sha(&bytes),
        expected_sha256: String::new(),
        assets,
    };
    preview.expected_sha256 = sha(&serde_json::to_vec(&preview)?);
    Ok(preview)
}

/// Caller holds the catalog lock. New objects are written before replacing the
/// head; a crash leaves either the old head or a complete new head. Orphaned
/// immutable objects are harmless, and installation rollback restores only head.
pub(crate) fn publish_unlocked(
    root: &Path,
    mut entry: ProjectCatalogEntryV2,
) -> Result<PathBuf, CatalogError> {
    let expected_head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut config = local_config(root)?;
    let mut asset = match catalog.assets.get(&entry.asset_id) {
        Some(_) => read_asset(root, &catalog, &entry.asset_id)?,
        None => AssetRecord {
            asset_id: entry.asset_id.clone(),
            name: entry.name.clone(),
            kind: entry.kind.clone(),
            tags: Vec::new(),
            selected_revision: None,
            build_revision: None,
            revisions: Vec::new(),
            locations: BTreeMap::new(),
            spec_locations: BTreeMap::new(),
            additional_locations: BTreeMap::new(),
            installations: Vec::new(),
        },
    };
    for digest in &asset.revisions {
        let revision = read_revision(root, &asset, digest)?;
        if revision.legacy.as_ref().is_some_and(|old| {
            !entry.source_job_id.is_empty()
                && old.source_job_id == entry.source_job_id
                && old.pack_sha256 == entry.pack_sha256
        }) {
            return Ok(root.join(PROJECT_CATALOG_RELATIVE));
        }
    }
    asset.name = entry.name.clone();
    asset.kind = entry.kind.clone();
    let location = locate(root, &entry.pack_path, &mut config)?;
    let absolute_pack = if entry.pack_path.is_absolute() {
        entry.pack_path.clone()
    } else {
        root.join(&entry.pack_path)
    };
    let content = directory_inventory(&absolute_pack)?;
    if crate::delivery::directory_sha256(&absolute_pack)? != entry.pack_sha256 {
        return Err(invalid("published Pack changed before catalog commit"));
    }
    forge_pack::validate_pack_layout(&absolute_pack).map_err(|e| invalid(e.to_string()))?;
    let spec_location = entry
        .spec_path
        .as_ref()
        .map(|p| locate(root, p, &mut config))
        .transpose()?;
    entry.pack_path = PathBuf::new();
    entry.spec_path = None;
    entry.installed = None;
    let revision = AssetRevision {
        schema_version: "1".into(),
        asset_id: asset.asset_id.clone(),
        content: Some(content),
        legacy: Some(entry),
        parent_revisions: Vec::new(),
        source: BTreeMap::new(),
    };
    let digest = write_object(root, &revision)?;
    asset.locations.insert(digest.clone(), location);
    if let Some(location) = spec_location {
        asset.spec_locations.insert(digest.clone(), location);
    }
    asset.revisions.push(digest.clone());
    asset.build_revision = Some(digest);
    catalog
        .assets
        .insert(asset.asset_id.clone(), write_object(root, &asset)?);
    catalog.updated_at = Utc::now();
    write_json(root, LOCAL, &config)?;
    commit_head(root, &expected_head, &catalog)?;
    Ok(root.join(PROJECT_CATALOG_RELATIVE))
}

pub fn migrate(
    root: &Path,
    name: &str,
    expected_sha256: &str,
) -> Result<LibraryCatalog, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let preview = migration_preview(root)?;
    if preview.expected_sha256 != expected_sha256 {
        return Err(invalid(
            "catalog or resource contents changed since migration preview",
        ));
    }
    let bytes = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    if sha(&bytes) != preview.catalog_sha256 {
        return Err(invalid("catalog changed since migration preview"));
    }
    let old = legacy_catalog(&bytes)?;
    let backup = storage_path(
        root,
        &format!(".forge/library/backups/{}.json", preview.catalog_sha256),
    )?;
    fs::create_dir_all(backup.parent().unwrap())?;
    write_ignore(root)?;
    if backup.exists() && fs::read(&backup)? != bytes {
        return Err(invalid("migration backup is corrupt"));
    }
    if !backup.exists() {
        fs::write(&backup, &bytes)?;
    }
    let identity = identity_for(root, name, Some(preview.catalog_sha256.clone()))?;
    let mut catalog = empty_catalog(&identity);
    let mut config = local_config(root)?;
    for (id, mut entry) in old.assets {
        if id != entry.asset_id {
            return Err(invalid("legacy catalog key differs from assetId"));
        }
        let location = locate(root, &entry.pack_path, &mut config)?;
        let absolute = if entry.pack_path.is_absolute() {
            entry.pack_path.clone()
        } else {
            root.join(&entry.pack_path)
        };
        // Unavailable/drifted history remains an assertion, never verified content.
        let content = directory_inventory(&absolute).ok().filter(|_| {
            crate::delivery::directory_sha256(&absolute).ok().as_deref()
                == Some(entry.pack_sha256.as_str())
        });
        let spec_location = entry
            .spec_path
            .as_ref()
            .map(|p| locate(root, p, &mut config))
            .transpose()?;
        let installed = entry.installed.take();
        entry.pack_path = PathBuf::new();
        entry.spec_path = None;
        let revision = AssetRevision {
            schema_version: "1".into(),
            asset_id: id.clone(),
            content,
            legacy: Some(entry.clone()),
            parent_revisions: Vec::new(),
            source: BTreeMap::from([(
                "migrationBackupSha256".into(),
                preview.catalog_sha256.clone().into(),
            )]),
        };
        let digest = write_object(root, &revision)?;
        let mut asset = AssetRecord {
            asset_id: id.clone(),
            name: entry.name,
            kind: entry.kind,
            tags: Vec::new(),
            selected_revision: None,
            build_revision: Some(digest.clone()),
            revisions: vec![digest.clone()],
            locations: BTreeMap::from([(digest.clone(), location)]),
            spec_locations: BTreeMap::new(),
            additional_locations: BTreeMap::new(),
            installations: Vec::new(),
        };
        if let Some(location) = spec_location {
            asset.spec_locations.insert(digest.clone(), location);
        }
        if let Some(installed) = installed {
            asset.installations.push(InstallReference {
                install_job_id: None,
                snapshot_text: None,
                snapshot_sha256: None,
                revision: digest,
                project: locate(root, &installed.godot_project, &mut config)?,
                target: installed.target,
                installed_at: installed.installed_at,
                evidence: "legacy_installation_assertion".into(),
            });
        }
        catalog.assets.insert(id, write_object(root, &asset)?);
    }
    write_ignore(root)?;
    write_json(root, LOCAL, &config)?;
    // Recheck the original head while holding the lock, before the only visible commit.
    if migration_preview(root)?.expected_sha256 != expected_sha256 {
        return Err(invalid(
            "catalog or resource contents changed during migration",
        ));
    }
    commit_head(root, &bytes, &catalog)?;
    Ok(catalog)
}

pub fn catalog_view(root: &Path) -> Result<ProjectCatalogV2, CatalogError> {
    let catalog = read_catalog(root)?;
    let mut assets = BTreeMap::new();
    for id in catalog.assets.keys() {
        let asset = read_asset(root, &catalog, id)?;
        let Some(digest) = &asset.build_revision else {
            continue;
        };
        let revision = read_revision(root, &asset, digest)?;
        let Some(mut entry) = revision.legacy else {
            continue;
        };
        let location = asset
            .locations
            .get(digest)
            .ok_or_else(|| invalid("revision location is missing"))?;
        entry.pack_path = resolve_location(root, location)?;
        entry.spec_path = asset
            .spec_locations
            .get(digest)
            .map(|p| resolve_location(root, p))
            .transpose()?;
        if let Some(installed) = asset
            .installations
            .iter()
            .rev()
            .find(|i| &i.revision == digest)
        {
            entry.installed = Some(CatalogInstallRefV1 {
                godot_project: resolve_location(root, &installed.project)?,
                target: installed.target.clone(),
                installed_at: installed.installed_at,
            });
        }
        assets.insert(id.clone(), entry);
    }
    Ok(ProjectCatalogV2 {
        schema_version: "2".into(),
        updated_at: catalog.updated_at,
        assets,
    })
}

pub fn revision_for_publication(
    root: &Path,
    id: &str,
    job: &str,
    pack_sha256: &str,
) -> Result<String, CatalogError> {
    let catalog = read_catalog(root)?;
    let asset = read_asset(root, &catalog, id)?;
    for digest in asset.revisions.iter().rev() {
        let revision = read_revision(root, &asset, digest)?;
        if revision
            .legacy
            .as_ref()
            .is_some_and(|entry| entry.source_job_id == job && entry.pack_sha256 == pack_sha256)
        {
            return Ok(digest.clone());
        }
    }
    Err(invalid(
        "the requested execution/Pack publication is not in library history",
    ))
}

fn verify_publication(
    root: &Path,
    asset: &AssetRecord,
    digest: &str,
) -> Result<PathBuf, CatalogError> {
    let revision = read_revision(root, asset, digest)?;
    let content = revision.content.ok_or_else(|| invalid("this historical revision has no verified content; register a verified Pack before installation"))?;
    let legacy = revision
        .legacy
        .ok_or_else(|| invalid("this revision has no Pack publication"))?;
    let location = asset
        .locations
        .get(digest)
        .ok_or_else(|| invalid("revision has no location"))?;
    let pack = resolve_location(root, location)?;
    if directory_inventory(&pack)?.sha256 != content.sha256
        || crate::delivery::directory_sha256(&pack)? != legacy.pack_sha256
    {
        return Err(invalid(
            "library Pack contents differ from the recorded revision",
        ));
    }
    forge_pack::validate_pack_layout(&pack).map_err(|e| invalid(e.to_string()))?;
    Ok(pack)
}

pub fn validate_current_pack(root: &Path, pack: &Path) -> Result<(), CatalogError> {
    let catalog = read_catalog(root)?;
    let expected = fs::canonicalize(pack)?;
    for id in catalog.assets.keys() {
        let asset = read_asset(root, &catalog, id)?;
        let Some(digest) = &asset.build_revision else {
            continue;
        };
        let Some(location) = asset.locations.get(digest) else {
            continue;
        };
        if resolve_location(root, location)
            .ok()
            .and_then(|p| fs::canonicalize(p).ok())
            .as_ref()
            == Some(&expected)
        {
            verify_publication(root, &asset, digest)?;
            return Ok(());
        }
    }
    Err(invalid("Pack does not match a current library publication"))
}

pub(crate) fn link_install_unlocked(
    root: &Path,
    id: &str,
    game: PathBuf,
    target: PathBuf,
) -> Result<PathBuf, CatalogError> {
    let expected_head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut asset = read_asset(root, &catalog, id)?;
    let revision = asset
        .build_revision
        .clone()
        .ok_or_else(|| invalid("asset has no build revision"))?;
    verify_publication(root, &asset, &revision)?;
    let mut config = local_config(root)?;
    asset.installations.push(InstallReference {
        install_job_id: None,
        snapshot_text: None,
        snapshot_sha256: None,
        revision,
        project: locate(root, &game, &mut config)?,
        target,
        installed_at: Utc::now(),
        evidence: "legacy_installation_assertion".into(),
    });
    catalog
        .assets
        .insert(id.into(), write_object(root, &asset)?);
    catalog.updated_at = Utc::now();
    write_json(root, LOCAL, &config)?;
    commit_head(root, &expected_head, &catalog)?;
    Ok(root.join(PROJECT_CATALOG_RELATIVE))
}

/// Available legacy execution projections across history, newest first. Missing
/// local root bindings do not destroy metadata; they simply cannot be reused.
pub fn publication_history(
    root: &Path,
    id: &str,
) -> Result<Vec<ProjectCatalogEntryV2>, CatalogError> {
    let catalog = read_catalog(root)?;
    if !catalog.assets.contains_key(id) {
        return Ok(vec![]);
    }
    let asset = read_asset(root, &catalog, id)?;
    let mut entries = vec![];
    for digest in asset.revisions.iter().rev() {
        let revision = read_revision(root, &asset, digest)?;
        let Some(mut entry) = revision.legacy else {
            continue;
        };
        let locations = asset
            .locations
            .get(digest)
            .into_iter()
            .chain(asset.additional_locations.get(digest).into_iter().flatten());
        for location in locations {
            let Ok(path) = resolve_location(root, location) else {
                continue;
            };
            if intake::content_at(&path).ok().as_ref() == revision.content.as_ref()
                && revision.content.is_some()
            {
                entry.pack_path = path;
                entry.spec_path = asset
                    .spec_locations
                    .get(digest)
                    .and_then(|location| resolve_location(root, location).ok());
                entries.push(entry);
                break;
            }
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod metadata_write_tests {
    use super::*;

    #[test]
    fn object_size_boundary_remains_readable_and_oversize_is_not_published() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        initialize(root, "Size boundary").unwrap();
        let before = read_bytes(root, PROJECT_CATALOG_RELATIVE).unwrap();
        for length in [
            MAX_METADATA_BYTES as usize - 3,
            MAX_METADATA_BYTES as usize - 2,
        ] {
            let value = "x".repeat(length);
            let digest = write_object(root, &value).unwrap();
            assert_eq!(read_object::<String>(root, &digest).unwrap(), value);
        }
        let too_large = "x".repeat(MAX_METADATA_BYTES as usize - 1);
        let digest = sha(&serde_json::to_vec(&too_large).unwrap());
        assert!(write_object(root, &too_large).is_err());
        assert!(!root.join(format!("{OBJECTS}/{digest}.json")).exists());
        assert_eq!(read_bytes(root, PROJECT_CATALOG_RELATIVE).unwrap(), before);
        assert!(read_catalog(root).unwrap().assets.is_empty());
    }

    #[test]
    fn oversized_head_and_local_configuration_preserve_readable_previous_state() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let mut catalog = initialize(root, "Original").unwrap();
        let head = read_bytes(root, PROJECT_CATALOG_RELATIVE).unwrap();
        catalog.name = "x".repeat(MAX_METADATA_BYTES as usize);
        assert!(commit_head(root, &head, &catalog).is_err());
        assert_eq!(read_bytes(root, PROJECT_CATALOG_RELATIVE).unwrap(), head);
        assert_eq!(read_catalog(root).unwrap().name, "Original");
        write_json(root, LOCAL, &LocalConfig::default()).unwrap();
        let local = read_bytes(root, LOCAL).unwrap();
        let oversized = LocalConfig {
            roots: BTreeMap::from([(
                "external".into(),
                PathBuf::from("x".repeat(MAX_METADATA_BYTES as usize)),
            )]),
        };
        assert!(write_json(root, LOCAL, &oversized).is_err());
        assert_eq!(read_bytes(root, LOCAL).unwrap(), local);
        assert!(local_config(root).unwrap().roots.is_empty());
    }
}
