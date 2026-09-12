//! Retained media, explicit version selection and deterministic delivery bindings.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VersionRef {
    pub asset_id: String,
    pub revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LockedRevision {
    pub revision: String,
    pub content_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceLock {
    pub schema_version: String,
    pub project_id: String,
    pub assets: BTreeMap<String, LockedRevision>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedResource {
    pub asset_id: String,
    pub revision: String,
    pub content_sha256: String,
    pub path: PathBuf,
    pub retained: bool,
}

pub fn resolve(root: &Path, reference: &VersionRef) -> Result<ResolvedResource, CatalogError> {
    let catalog = read_catalog(root)?;
    let asset = read_asset(root, &catalog, &reference.asset_id)?;
    let revision = read_revision(root, &asset, &reference.revision)?;
    let expected = revision
        .content
        .ok_or_else(|| invalid("revision has no verified content inventory"))?;
    let mut issues = vec![];
    for location in asset.locations.get(&reference.revision).into_iter().chain(
        asset
            .additional_locations
            .get(&reference.revision)
            .into_iter()
            .flatten(),
    ) {
        let result = resolve_location(root, location).and_then(|path| {
            if intake::content_at(&path)? != expected {
                return Err(invalid("revision bytes have changed"));
            }
            Ok(path)
        });
        match result {
            Ok(path) => {
                return Ok(ResolvedResource {
                    asset_id: reference.asset_id.clone(),
                    revision: reference.revision.clone(),
                    content_sha256: expected.sha256,
                    path,
                    retained: location.root_id == "project"
                        && location.path.starts_with(".forge/library/media/"),
                })
            }
            Err(error) => issues.push(error.to_string()),
        }
    }
    Err(invalid(format!(
        "revision unavailable; restore retained media or rebind its source root: {}",
        issues.join("; ")
    )))
}

pub fn retain(root: &Path, reference: &VersionRef) -> Result<ResolvedResource, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut asset = read_asset(root, &catalog, &reference.asset_id)?;
    let revision = read_revision(root, &asset, &reference.revision)?;
    let expected = revision
        .content
        .ok_or_else(|| invalid("cannot retain unverified historical content"))?;
    let source = resolve(root, reference)?;
    if source.retained {
        return Ok(source);
    }
    let media = storage_path(root, ".forge/library/media")?;
    fs::create_dir_all(&media)?;
    let destination = storage_path(
        root,
        &format!(".forge/library/media/{}", reference.revision),
    )?;
    let filename = if source.path.is_dir() {
        "payload.gsfpack".into()
    } else {
        let extension = source
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("bin");
        let name = format!("source.{extension}");
        validate_relative(&name)?;
        name
    };
    let final_path = destination.join(&filename);
    if !destination.exists() {
        let staging = tempfile::Builder::new()
            .prefix(".retain-")
            .tempdir_in(&media)?;
        let payload = staging.path().join(&filename);
        if source.path.is_dir() {
            fs::create_dir(&payload)?;
            for file in &expected.files {
                let target = payload.join(&file.path);
                fs::create_dir_all(target.parent().unwrap())?;
                fs::copy(source.path.join(&file.path), &target)?;
                fs::OpenOptions::new()
                    .write(true)
                    .open(target)?
                    .sync_all()?;
            }
        } else {
            fs::copy(&source.path, &payload)?;
            fs::OpenOptions::new()
                .write(true)
                .open(&payload)?
                .sync_all()?;
        }
        if intake::content_at(&payload)? != expected
            || intake::content_at(&source.path)? != expected
        {
            return Err(invalid("source changed during retention"));
        }
        fs::rename(staging.path(), &destination)?;
        #[cfg(unix)]
        fs::File::open(&media)?.sync_all()?;
    }
    if intake::content_at(&final_path)? != expected {
        return Err(invalid("retained destination is corrupt"));
    }
    let location = Location {
        root_id: "project".into(),
        path: format!(".forge/library/media/{}/{filename}", reference.revision),
    };
    if let Some(old) = asset.locations.insert(reference.revision.clone(), location) {
        let alternatives = asset
            .additional_locations
            .entry(reference.revision.clone())
            .or_default();
        if !alternatives.contains(&old) {
            alternatives.push(old);
        }
    }
    catalog
        .assets
        .insert(reference.asset_id.clone(), write_object(root, &asset)?);
    catalog.updated_at = Utc::now();
    commit_head(root, &head, &catalog)?;
    resolve(root, reference)
}

pub fn select(root: &Path, reference: &VersionRef) -> Result<AssetRecord, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut asset = read_asset(root, &catalog, &reference.asset_id)?;
    read_revision(root, &asset, &reference.revision)?;
    if asset.selected_revision.as_ref() != Some(&reference.revision) {
        asset.selected_revision = Some(reference.revision.clone());
        catalog
            .assets
            .insert(reference.asset_id.clone(), write_object(root, &asset)?);
        catalog.updated_at = Utc::now();
        commit_head(root, &head, &catalog)?;
    }
    Ok(asset)
}

pub fn write_lock(
    root: &Path,
    reference: &VersionRef,
    output: &Path,
) -> Result<ResourceLock, CatalogError> {
    let _lock = crate::catalog::lock_catalog(root)?;
    let catalog = read_catalog(root)?;
    let resolved = resolve(root, reference)?;
    let mut lock = if output.exists() {
        if is_link(&fs::symlink_metadata(output)?) {
            return Err(CatalogError::Symlink);
        }
        let lock: ResourceLock = serde_json::from_slice(&fs::read(output)?)?;
        if lock.schema_version != "1" || lock.project_id != catalog.project_id {
            return Err(invalid(
                "resource lock belongs to a different library or schema",
            ));
        }
        lock
    } else {
        ResourceLock {
            schema_version: "1".into(),
            project_id: catalog.project_id,
            assets: BTreeMap::new(),
        }
    };
    lock.assets.insert(
        reference.asset_id.clone(),
        LockedRevision {
            revision: reference.revision.clone(),
            content_sha256: resolved.content_sha256,
        },
    );
    crate::catalog::write_json_atomic(output, &lock)?;
    Ok(lock)
}

pub fn locked_reference(root: &Path, id: &str, path: &Path) -> Result<VersionRef, CatalogError> {
    let lock: ResourceLock = serde_json::from_slice(&fs::read(path)?)?;
    let catalog = read_catalog(root)?;
    if lock.schema_version != "1" || lock.project_id != catalog.project_id {
        return Err(invalid("resource lock library identity mismatch"));
    }
    let locked = lock
        .assets
        .get(id)
        .ok_or_else(|| invalid("asset is absent from resource lock"))?;
    let reference = VersionRef {
        asset_id: id.into(),
        revision: locked.revision.clone(),
    };
    let resolved = resolve(root, &reference)?;
    if resolved.content_sha256 != locked.content_sha256 {
        return Err(invalid("resource lock digest mismatch"));
    }
    Ok(reference)
}

pub fn install_impact(
    root: &Path,
    reference: &VersionRef,
) -> Result<serde_json::Value, CatalogError> {
    let resolved = resolve(root, reference)?;
    forge_pack::validate_pack_layout(&resolved.path).map_err(|e| invalid(e.to_string()))?;
    let pack = forge_pack::inspect_pack(&resolved.path).map_err(|e| invalid(e.to_string()))?;
    let catalog = read_catalog(root)?;
    let mut related = vec![];
    for id in catalog.assets.keys() {
        let asset = read_asset(root, &catalog, id)?;
        for digest in &asset.revisions {
            if read_revision(root, &asset, digest)?
                .content
                .as_ref()
                .is_some_and(|c| c.sha256 == resolved.content_sha256)
            {
                related.push(VersionRef {
                    asset_id: id.clone(),
                    revision: digest.clone(),
                });
            }
        }
    }
    Ok(
        serde_json::json!({"scope":"whole_pack", "resource":resolved, "pack":pack,
        "relatedLibraryRevisions":related, "humanApproval":"not_inferred", "memberOnlyInstall":false}),
    )
}

pub(crate) fn link_revision_unlocked(
    root: &Path,
    reference: &VersionRef,
    pack: &Path,
    game: &Path,
    target: &Path,
    job_id: &str,
) -> Result<PathBuf, CatalogError> {
    let resolved = resolve(root, reference)?;
    if fs::canonicalize(pack)? != fs::canonicalize(&resolved.path)? {
        return Err(invalid(
            "installed Pack does not resolve to the specified revision",
        ));
    }
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let mut catalog = read_catalog(root)?;
    let mut config = local_config(root)?;
    let mut asset = read_asset(root, &catalog, &reference.asset_id)?;
    let snapshot_text =
        fs::read_to_string(game.join(target).join(crate::delivery::INSTALL_SNAPSHOT))?;
    let _: crate::delivery::InstallSnapshot = serde_json::from_str(&snapshot_text)?;
    let snapshot_sha256 = sha(snapshot_text.as_bytes());
    asset.installations.push(InstallReference {
        install_job_id: Some(job_id.into()),
        snapshot_text: Some(snapshot_text),
        snapshot_sha256: Some(snapshot_sha256),
        revision: reference.revision.clone(),
        project: locate(root, game, &mut config)?,
        target: target.into(),
        installed_at: Utc::now(),
        evidence: "installation_transaction".into(),
    });
    catalog
        .assets
        .insert(reference.asset_id.clone(), write_object(root, &asset)?);
    catalog.updated_at = Utc::now();
    write_json(root, LOCAL, &config)?;
    commit_head(root, &head, &catalog)?;
    Ok(root.join(PROJECT_CATALOG_RELATIVE))
}
