//! Portable directory bundles. The manifest binds every payload file; a caller
//! supplies its SHA-256 separately when verifying a trusted transfer baseline.
use super::*;
use crate::content_digest::ContentInventory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferManifest {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub exported_at: chrono::DateTime<Utc>,
    pub exporter: serde_json::Value,
    pub source_catalog_sha256: String,
    pub payload: ContentInventory,
    pub selections: Vec<TransferredRevision>,
    pub consumer_lock_path: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferredRevision {
    pub asset_id: String,
    pub revision: String,
    pub content_sha256: String,
    pub media_path: String,
    pub legacy_pack_sha256: Option<String>,
    pub legacy_hash_style: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferReport {
    pub path: PathBuf,
    pub manifest_sha256: String,
    pub trusted_baseline_verified: bool,
    pub revisions: usize,
    pub payload_files: usize,
}

fn copy_file(source: &Path, target: &Path) -> Result<(), CatalogError> {
    fs::create_dir_all(target.parent().unwrap())?;
    fs::copy(source, target)?;
    fs::OpenOptions::new()
        .write(true)
        .open(target)?
        .sync_all()?;
    Ok(())
}
fn copy_content(
    source: &Path,
    target: &Path,
    content: &ContentInventory,
) -> Result<(), CatalogError> {
    if source.is_dir() {
        fs::create_dir_all(target)?;
        for file in &content.files {
            copy_file(&source.join(&file.path), &target.join(&file.path))?;
        }
    } else {
        copy_file(source, target)?;
    }
    if intake::content_at(target)? != *content {
        return Err(invalid("transferred media does not match revision"));
    }
    Ok(())
}
pub(super) fn legacy_style(path: &Path, recorded: &str) -> Result<String, CatalogError> {
    use crate::content_digest::{legacy_directory_digest, LegacyPathStyle};
    let posix = legacy_directory_digest(path, LegacyPathStyle::Posix)?;
    let windows = legacy_directory_digest(path, LegacyPathStyle::Windows)?;
    match (recorded == posix, recorded == windows) {
        (true, true) => Ok("posix_and_windows".into()),
        (true, false) => Ok("posix".into()),
        (false, true) => Ok("windows".into()),
        _ => Err(invalid(
            "historical Pack hash does not match either preserved v2 path convention",
        )),
    }
}

pub fn export(
    root: &Path,
    references: &[delivery::VersionRef],
    output: &Path,
    mut exporter: serde_json::Value,
    consumer_lock: Option<&Path>,
) -> Result<TransferReport, CatalogError> {
    if references.is_empty() {
        return Err(invalid("export requires at least one exact revision"));
    }
    if fs::symlink_metadata(output).is_ok() {
        return Err(invalid("bundle output already exists"));
    }
    let _lock = crate::catalog::lock_catalog(root)?;
    let head = read_bytes(root, PROJECT_CATALOG_RELATIVE)?;
    let catalog = read_catalog(root)?;
    let mut selected = BTreeMap::<String, BTreeSet<String>>::new();
    for reference in references {
        if !selected
            .entry(reference.asset_id.clone())
            .or_default()
            .insert(reference.revision.clone())
        {
            return Err(invalid("duplicate transfer selection"));
        }
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".forge-transfer-")
        .tempdir_in(parent)?;
    let payload = staging.path().join("payload");
    fs::create_dir(&payload)?;
    let mut exported_catalog = catalog.clone();
    exported_catalog
        .assets
        .retain(|id, _| selected.contains_key(id));
    let mut selections = vec![];
    for (id, revisions) in selected {
        let mut asset = read_asset(root, &catalog, &id)?;
        // Only the explicitly selected revision metadata, evidence and media
        // cross the boundary; unrelated history is not exported incidentally.
        asset
            .revisions
            .retain(|revision| revisions.contains(revision));
        asset
            .dispositions
            .retain(|revision, _| revisions.contains(revision));
        if asset
            .selected_revision
            .as_ref()
            .is_some_and(|r| !revisions.contains(r))
        {
            asset.selected_revision = None;
        }
        if asset
            .build_revision
            .as_ref()
            .is_some_and(|r| !revisions.contains(r))
        {
            asset.build_revision = None;
        }
        asset
            .installations
            .retain(|i| revisions.contains(&i.revision));
        asset.locations.clear();
        asset.additional_locations.clear();
        asset.spec_locations.clear();
        let mut review_objects = vec![];
        for digest in &asset.reviews {
            let review: review::ReviewRecord = read_object(root, digest)?;
            if revisions.contains(&review.revision) {
                copy_file(
                    &storage_path(root, &format!("{OBJECTS}/{digest}.json"))?,
                    &payload.join(format!("{OBJECTS}/{digest}.json")),
                )?;
                let evidence = storage_path(root, &review.evidence_path)?;
                let actual = intake::content_at(&evidence)?;
                if actual.files[0].sha256 != review.evidence_sha256
                    || actual.files[0].bytes != review.evidence_bytes
                {
                    return Err(invalid("review evidence is missing or changed"));
                }
                copy_file(&evidence, &payload.join(&review.evidence_path))?;
                review_objects.push(digest.clone());
            }
        }
        asset.reviews = review_objects;
        for revision_id in revisions {
            let reference = delivery::VersionRef {
                asset_id: id.clone(),
                revision: revision_id.clone(),
            };
            let resolved = delivery::resolve(root, &reference)?;
            let revision = read_revision(root, &asset, &revision_id)?;
            let content = revision
                .content
                .as_ref()
                .ok_or_else(|| invalid("selected revision has no verified media"))?;
            let filename = if resolved.path.is_dir() {
                "payload.gsfpack".into()
            } else {
                let extension = resolved
                    .path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("bin");
                let name = format!("source.{extension}");
                validate_relative(&name)?;
                name
            };
            let media_path = format!(".forge/library/media/{revision_id}/{filename}");
            copy_content(&resolved.path, &payload.join(&media_path), content)?;
            if intake::content_at(&resolved.path)? != *content {
                return Err(invalid("source changed while exporting"));
            }
            copy_file(
                &storage_path(root, &format!("{OBJECTS}/{revision_id}.json"))?,
                &payload.join(format!("{OBJECTS}/{revision_id}.json")),
            )?;
            let legacy_pack_sha256 = revision
                .legacy
                .as_ref()
                .map(|entry| entry.pack_sha256.clone());
            let legacy_hash_style = legacy_pack_sha256
                .as_ref()
                .map(|hash| legacy_style(&resolved.path, hash))
                .transpose()?;
            selections.push(TransferredRevision {
                asset_id: id.clone(),
                revision: revision_id.clone(),
                content_sha256: content.sha256.clone(),
                media_path: media_path.clone(),
                legacy_pack_sha256,
                legacy_hash_style,
            });
            asset.locations.insert(
                revision_id,
                Location {
                    root_id: "project".into(),
                    path: media_path,
                },
            );
        }
        let bytes = serde_json::to_vec(&asset)?;
        let digest = sha(&bytes);
        let object = payload.join(format!("{OBJECTS}/{digest}.json"));
        fs::create_dir_all(object.parent().unwrap())?;
        fs::write(object, bytes)?;
        exported_catalog.assets.insert(id, digest);
    }
    fs::write(
        payload.join(PROJECT_CATALOG_RELATIVE),
        serde_json::to_vec(&exported_catalog)?,
    )?;
    fs::write(
        payload.join(IDENTITY),
        serde_json::to_vec(&Identity {
            project_id: catalog.project_id.clone(),
            name: catalog.name.clone(),
            migrated_from_sha256: catalog.migrated_from_sha256.clone(),
        })?,
    )?;
    // Machine bindings and caches are never transferred. Project-relative
    // payload locations are sufficient to resolve every selected revision.
    if let Some(object) = exporter.as_object_mut() {
        object.remove("executable");
    }
    let consumer_lock_path = if let Some(lock) = consumer_lock {
        let path = ".forge/library/consumer-resources.lock.json";
        copy_file(lock, &payload.join(path))?;
        Some(path.into())
    } else {
        None
    };
    let manifest = TransferManifest {
        schema_version: "1".into(),
        project_id: catalog.project_id,
        name: catalog.name,
        exported_at: Utc::now(),
        exporter,
        source_catalog_sha256: sha(&head),
        payload: crate::content_digest::directory_inventory(&payload)?,
        selections,
        consumer_lock_path,
    };
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    fs::write(staging.path().join("bundle.json"), &bytes)?;
    if read_bytes(root, PROJECT_CATALOG_RELATIVE)? != head {
        return Err(invalid("catalog changed during export"));
    }
    verify(staging.path(), Some(&sha(&bytes)))?;
    fs::create_dir(output)?;
    for entry in fs::read_dir(staging.path())? {
        let entry = entry?;
        fs::rename(entry.path(), output.join(entry.file_name()))?;
    }
    Ok(TransferReport {
        path: output.into(),
        manifest_sha256: sha(&bytes),
        trusted_baseline_verified: false,
        revisions: manifest.selections.len(),
        payload_files: manifest.payload.files.len(),
    })
}

pub fn verify(
    bundle: &Path,
    expected_sha256: Option<&str>,
) -> Result<TransferReport, CatalogError> {
    let path = storage_path(bundle, "bundle.json")?;
    if fs::metadata(&path)?.len() > MAX_METADATA_BYTES {
        return Err(invalid("bundle manifest exceeds 16 MiB"));
    }
    let bytes = fs::read(path)?;
    let digest = sha(&bytes);
    if expected_sha256.is_some_and(|expected| expected != digest) {
        return Err(invalid(
            "bundle does not match the supplied trusted manifest digest",
        ));
    }
    let manifest: TransferManifest = serde_json::from_slice(&bytes)?;
    if manifest.schema_version != "1"
        || manifest.selections.is_empty()
        || !valid_digest(&manifest.source_catalog_sha256)
    {
        return Err(invalid("invalid transfer manifest"));
    }
    let payload = storage_path(bundle, "payload")?;
    if crate::content_digest::directory_inventory(&payload)? != manifest.payload {
        return Err(invalid(
            "bundle payload inventory differs: missing, changed, extra or duplicate files",
        ));
    }
    let catalog = read_catalog(&payload)?;
    if catalog.project_id != manifest.project_id {
        return Err(invalid("bundle library identity mismatch"));
    }
    let mut seen = BTreeSet::new();
    let mut expected_objects = BTreeSet::new();
    let mut allowed_files =
        BTreeSet::from([PROJECT_CATALOG_RELATIVE.to_string(), IDENTITY.to_string()]);
    for selected in &manifest.selections {
        if !seen.insert((selected.asset_id.clone(), selected.revision.clone())) {
            return Err(invalid("duplicate transfer selection"));
        }
        validate_relative(&selected.media_path)?;
        let prefix = format!(".forge/library/media/{}/", selected.revision);
        if !selected.media_path.starts_with(&prefix) {
            return Err(invalid("invalid retained transfer location"));
        }
        let asset = read_asset(&payload, &catalog, &selected.asset_id)?;
        expected_objects.insert(catalog.assets[&selected.asset_id].clone());
        allowed_files.insert(format!(
            "{OBJECTS}/{}.json",
            catalog.assets[&selected.asset_id]
        ));
        let revision = read_revision(&payload, &asset, &selected.revision)?;
        let resolved = delivery::resolve(
            &payload,
            &delivery::VersionRef {
                asset_id: selected.asset_id.clone(),
                revision: selected.revision.clone(),
            },
        )?;
        if resolved.content_sha256 != selected.content_sha256
            || resolved.path != fs::canonicalize(&payload)?.join(&selected.media_path)
        {
            return Err(invalid("bundle selection content/location mismatch"));
        }
        if revision.legacy.as_ref().map(|entry| &entry.pack_sha256)
            != selected.legacy_pack_sha256.as_ref()
        {
            return Err(invalid(
                "historical Pack hash was changed in transfer metadata",
            ));
        }
        if let Some(hash) = &selected.legacy_pack_sha256 {
            if Some(legacy_style(&resolved.path, hash)?) != selected.legacy_hash_style {
                return Err(invalid("historical Pack hash mapping mismatch"));
            }
        } else if selected.legacy_hash_style.is_some() {
            return Err(invalid("hash mapping without historical hash"));
        }
        expected_objects.insert(selected.revision.clone());
        allowed_files.insert(format!("{OBJECTS}/{}.json", selected.revision));
        if resolved.path.is_dir() {
            for file in &revision.content.as_ref().unwrap().files {
                allowed_files.insert(format!("{}/{}", selected.media_path, file.path));
            }
        } else {
            allowed_files.insert(selected.media_path.clone());
        }
        for object in &asset.reviews {
            let review: review::ReviewRecord = read_object(&payload, object)?;
            let evidence = intake::content_at(&storage_path(&payload, &review.evidence_path)?)?;
            if evidence.files[0].sha256 != review.evidence_sha256
                || evidence.files[0].bytes != review.evidence_bytes
            {
                return Err(invalid("transferred review evidence is corrupt"));
            }
            expected_objects.insert(object.clone());
            allowed_files.insert(format!("{OBJECTS}/{object}.json"));
            allowed_files.insert(review.evidence_path);
        }
    }
    if let Some(path) = &manifest.consumer_lock_path {
        if path != ".forge/library/consumer-resources.lock.json" {
            return Err(invalid("unexpected consumer lock path"));
        }
        let lock: delivery::ResourceLock =
            serde_json::from_slice(&fs::read(storage_path(&payload, path)?)?)?;
        if lock.schema_version != "1"
            || lock.project_id != manifest.project_id
            || lock.assets.len() != seen.len()
        {
            return Err(invalid("consumer lock does not match transfer selection"));
        }
        for (id, revision) in &seen {
            let selected = delivery::locked_reference(&payload, id, &payload.join(path))?;
            if &selected.revision != revision {
                return Err(invalid(
                    "consumer lock revision differs from transferred selection",
                ));
            }
        }
        allowed_files.insert(path.clone());
    }
    // Catalog metadata cannot quietly refer to revisions outside the selection.
    for id in catalog.assets.keys() {
        let asset = read_asset(&payload, &catalog, id)?;
        if asset
            .revisions
            .iter()
            .any(|revision| !seen.contains(&(id.clone(), revision.clone())))
        {
            return Err(invalid("bundle contains unselected revision metadata"));
        }
    }
    for file in &manifest.payload.files {
        if !allowed_files.contains(&file.path) {
            return Err(invalid("bundle contains an unrelated payload file"));
        }
        if file.path.starts_with(&format!("{OBJECTS}/")) {
            let object = file
                .path
                .trim_start_matches(&format!("{OBJECTS}/"))
                .trim_end_matches(".json");
            if !expected_objects.contains(object) {
                return Err(invalid("bundle contains unrelated immutable objects"));
            }
        }
        if file.path == LOCAL || file.path.contains("/cache/") || file.path.contains("/backups/") {
            return Err(invalid(
                "bundle contains machine-local configuration or cache",
            ));
        }
    }
    Ok(TransferReport {
        path: bundle.into(),
        manifest_sha256: digest,
        trusted_baseline_verified: expected_sha256.is_some(),
        revisions: seen.len(),
        payload_files: manifest.payload.files.len(),
    })
}

pub fn import(
    bundle: &Path,
    destination: &Path,
    expected_sha256: &str,
) -> Result<TransferReport, CatalogError> {
    let report = verify(bundle, Some(expected_sha256))?;
    if fs::symlink_metadata(destination).is_ok() {
        return Err(invalid("import destination must be a new directory; existing libraries require explicit metadata merge"));
    }
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".forge-import-")
        .tempdir_in(parent)?;
    let payload = bundle.join("payload");
    let inventory = crate::content_digest::directory_inventory(&payload)?;
    for file in &inventory.files {
        copy_file(&payload.join(&file.path), &staging.path().join(&file.path))?;
    }
    if crate::content_digest::directory_inventory(staging.path())? != inventory {
        return Err(invalid("bundle changed during import"));
    }
    verify(bundle, Some(expected_sha256))?;
    snapshot_sha256(staging.path())?;
    write_ignore(staging.path())?;
    fs::create_dir(destination)?;
    fs::rename(staging.path().join(".forge"), destination.join(".forge"))?;
    Ok(TransferReport {
        path: destination.into(),
        ..report
    })
}
