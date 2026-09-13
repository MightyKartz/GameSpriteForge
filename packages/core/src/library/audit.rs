//! Read-only integrity and availability checks. Local bindings never rewrite
//! shared revision identity or turn missing media into verified content.
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetAudit {
    pub project_id: String,
    pub checked_at: chrono::DateTime<Utc>,
    pub metadata_sha256: String,
    pub revisions: Vec<intake::SearchHit>,
    pub evidence_issues: Vec<String>,
    pub installations: Vec<InstallationAudit>,
    pub external_lineage: Vec<String>,
    pub complete_media: bool,
    pub scope: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallationAudit {
    pub asset_id: String,
    pub revision: String,
    pub root_id: String,
    pub project: Option<PathBuf>,
    pub target: PathBuf,
    pub status: String,
    pub detail: String,
}

fn installation(root: &Path, asset: &AssetRecord, known: &InstallReference) -> InstallationAudit {
    let mut report = InstallationAudit {
        asset_id: asset.asset_id.clone(),
        revision: known.revision.clone(),
        root_id: known.project.root_id.clone(),
        project: None,
        target: known.target.clone(),
        status: "unknown".into(),
        detail: String::new(),
    };
    let check = || -> Result<(String, String, PathBuf), CatalogError> {
        let project = resolve_location(root, &known.project)?;
        storage_path(&project, crate::project::PROJECT_MANIFEST_RELATIVE)?;
        let manifest =
            crate::project::read_project_manifest(&project).map_err(|e| invalid(e.to_string()))?;
        let logical_target = known.target.to_string_lossy().replace('\\', "/");
        let current = manifest.assets.iter().find(|(_, entry)| {
            entry.godot_target.to_string_lossy().replace('\\', "/") == logical_target
        });
        let Some((key, entry)) = current else {
            return Ok(("not_in_connected_registry".into(), "No current registry entry for this historical target; runtime references are not scanned.".into(), project));
        };
        let Some(snapshot) = &known.snapshot_sha256 else {
            return Ok((
                "legacy_evidence_insufficient".into(),
                "Historical assertion has no independently retained installation snapshot.".into(),
                project,
            ));
        };
        if entry.install_snapshot_sha256.as_ref() != Some(snapshot)
            || known.install_job_id.as_deref() != Some(entry.last_job_id.as_str())
        {
            return Ok((
                "superseded".into(),
                "Connected project registry records a different installation at this target."
                    .into(),
                project,
            ));
        }
        let pack = delivery::resolve(
            root,
            &delivery::VersionRef {
                asset_id: asset.asset_id.clone(),
                revision: known.revision.clone(),
            },
        )?;
        match crate::delivery::verify_install(&project, key, Some(&pack.path)) {
            Ok(_) => Ok(("verified_current_installation".into(), "Registry, retained snapshot, installed files, Pack and applicable caches match; native load and runtime usage were not assessed.".into(), project)),
            Err(error) => Ok(("verification_failed".into(), error.to_string(), project)),
        }
    };
    match check() {
        Ok((status, detail, project)) => {
            report.status = status;
            report.detail = detail;
            report.project = Some(project);
        }
        Err(error) => report.detail = error.to_string(),
    }
    report
}

pub fn verify(root: &Path) -> Result<AssetAudit, CatalogError> {
    let snapshot = snapshot_sha256(root)?;
    let catalog = read_catalog(root)?;
    let mut report = AssetAudit {
        project_id: catalog.project_id.clone(),
        checked_at: Utc::now(),
        metadata_sha256: snapshot.clone(),
        revisions: vec![],
        evidence_issues: vec![],
        installations: vec![],
        external_lineage: vec![],
        complete_media: true,
        scope: "Registered revisions and retained evidence only; installation history does not establish current runtime usage or global absence of references.",
    };
    let known: std::collections::BTreeSet<String> = catalog
        .assets
        .keys()
        .map(|id| read_asset(root, &catalog, id))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flat_map(|a| a.revisions)
        .collect();
    for id in catalog.assets.keys() {
        let asset = read_asset(root, &catalog, id)?;
        report.installations.extend(
            asset
                .installations
                .iter()
                .map(|known| installation(root, &asset, known)),
        );
        for revision in &asset.revisions {
            let record = read_revision(root, &asset, revision)?;
            for parent in record.parent_revisions {
                if !known.contains(&parent) {
                    report.external_lineage.push(parent);
                }
            }
        }
        for digest in &asset.reviews {
            let review: review::ReviewRecord = read_object(root, digest)?;
            let valid = review::verify_evidence(root, &review).is_ok();
            if !valid {
                report
                    .evidence_issues
                    .push(format!("{id}/{digest}: review evidence missing or changed"));
            }
        }
        report.revisions.extend(intake::history(root, id)?);
    }
    report.external_lineage.sort();
    report.external_lineage.dedup();
    report.complete_media = report.revisions.iter().all(|r| r.status == "available")
        && report.evidence_issues.is_empty();
    if snapshot_sha256(root)? != snapshot {
        return Err(invalid("library changed during integrity audit"));
    }
    Ok(report)
}

pub fn bind_root(root: &Path, root_id: &str, path: &Path) -> Result<AssetAudit, CatalogError> {
    if root_id == "project" {
        return Err(invalid(
            "the project root is resolved from the library directory and cannot be rebound",
        ));
    }
    let _lock = crate::catalog::lock_catalog(root)?;
    let catalog = read_catalog(root)?;
    let mut found = false;
    for id in catalog.assets.keys() {
        let asset = read_asset(root, &catalog, id)?;
        found |= asset
            .locations
            .values()
            .chain(asset.additional_locations.values().flatten())
            .chain(asset.spec_locations.values())
            .chain(asset.installations.iter().map(|i| &i.project))
            .any(|location| location.root_id == root_id);
    }
    if !found {
        return Err(invalid("root ID is not referenced by this library"));
    }
    if is_link(&fs::symlink_metadata(path)?) {
        return Err(CatalogError::Symlink);
    }
    let path = fs::canonicalize(path)?;
    let mut local = local_config(root)?;
    local.roots.insert(root_id.into(), path);
    write_json(root, LOCAL, &local)?;
    verify(root)
}
