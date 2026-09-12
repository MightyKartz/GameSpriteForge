//! Durable publication requests live beside outputs, never inside Pack bytes.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectBinding {
    pub project_path: PathBuf,
    pub asset_id: String,
}
impl ProjectBinding {
    pub fn validate(&self) -> Result<(), CatalogError> {
        if self.asset_id.trim().is_empty() || !self.project_path.is_absolute() {
            return Err(invalid(
                "assetProject requires an absolute projectPath and nonempty assetId",
            ));
        }
        read_catalog(&self.project_path)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum PendingPublication {
    Local {
        binding: ProjectBinding,
        item: Box<intake::IntakeItem>,
        source: BTreeMap<String, serde_json::Value>,
    },
    Catalog {
        project: PathBuf,
        entry: Box<ProjectCatalogEntryV2>,
    },
}

fn write_pending(pack: &Path, pending: &PendingPublication) -> Result<PathBuf, CatalogError> {
    let bytes = serde_json::to_vec(pending)?;
    let parent = pack.parent().ok_or_else(|| invalid("Pack has no parent"))?;
    let path = parent.join(format!("pack.publication-{}.json", sha(&bytes)));
    if path.exists() {
        if is_link(&fs::symlink_metadata(&path)?) || fs::read(&path)? != bytes {
            return Err(invalid("existing publication request is corrupt"));
        }
        return Ok(path);
    }
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(&path) {
        Ok(_) => (),
        Err(error) => {
            // Another identical publisher may have won. Never replace a live
            // reader's file, which also avoids Windows sharing violations.
            if !path.is_file()
                || is_link(&fs::symlink_metadata(&path)?)
                || fs::read(&path)? != bytes
            {
                return Err(CatalogError::Io(error.error));
            }
        }
    }
    Ok(path)
}

pub fn local_output(
    binding: &ProjectBinding,
    pack: &Path,
    source: BTreeMap<String, serde_json::Value>,
) -> Result<PathBuf, CatalogError> {
    let path = stage_local(binding, pack, source)?;
    complete_local(&path)?;
    Ok(path)
}

fn complete_local(path: &Path) -> Result<(), CatalogError> {
    recover(path).map_err(|e| invalid(format!("output is complete; publication pending at {}: {e}; run forge asset recover --input <this-path>", path.display())))?;
    Ok(())
}

fn stage_local(
    binding: &ProjectBinding,
    pack: &Path,
    mut source: BTreeMap<String, serde_json::Value>,
) -> Result<PathBuf, CatalogError> {
    if let Some(producer) = source
        .get_mut("producerBuild")
        .and_then(|v| v.as_object_mut())
    {
        producer.remove("executable");
    }
    forge_pack::validate_pack_layout(pack).map_err(|e| invalid(e.to_string()))?;
    let summary = forge_pack::inspect_pack(pack).map_err(|e| invalid(e.to_string()))?;
    let pack = fs::canonicalize(pack)?;
    let item = intake::IntakeItem {
        asset_id: binding.asset_id.clone(),
        name: summary.name,
        path: pack.clone(),
        kind: "pack".into(),
        tags: vec![],
        expected_content: intake::content_at(&pack)?,
        new_revision: true,
    };
    let pending = PendingPublication::Local {
        binding: binding.clone(),
        item: Box::new(item),
        source,
    };
    write_pending(&pack, &pending)
}

/// Called by existing generation publications. Retains the exact V2 provenance
/// required by the immutable adapter and by interrupted parent build recovery.
pub(crate) fn stage_catalog(
    project: &Path,
    entry: &ProjectCatalogEntryV2,
) -> Result<Option<PathBuf>, CatalogError> {
    if !entry.pack_path.is_dir() {
        return Ok(None);
    }
    let path = write_pending(
        &entry.pack_path,
        &PendingPublication::Catalog {
            project: project.into(),
            entry: Box::new(entry.clone()),
        },
    )?;
    Ok(Some(path))
}

pub fn recover(path: &Path) -> Result<PathBuf, CatalogError> {
    if is_link(&fs::symlink_metadata(path)?) {
        return Err(CatalogError::Symlink);
    }
    if fs::metadata(path)?.len() > MAX_METADATA_BYTES {
        return Err(invalid("publication metadata exceeds 16 MiB"));
    }
    let pending: PendingPublication = serde_json::from_slice(&fs::read(path)?)?;
    match pending {
        PendingPublication::Local {
            binding,
            item,
            source,
        } => {
            binding.validate()?;
            if binding.asset_id != item.asset_id {
                return Err(invalid("publication asset ID mismatch"));
            }
            intake::register_production(
                &binding.project_path,
                &intake::IntakeBatch {
                    schema_version: "1".into(),
                    items: vec![*item],
                },
                &source,
            )?;
            Ok(binding.project_path.join(PROJECT_CATALOG_RELATIVE))
        }
        PendingPublication::Catalog { project, entry } => {
            if crate::delivery::directory_sha256(&entry.pack_path)? != entry.pack_sha256 {
                return Err(invalid("pending Pack has changed"));
            }
            forge_pack::validate_pack_layout(&entry.pack_path)
                .map_err(|e| invalid(e.to_string()))?;
            crate::catalog::publish_catalog_asset(&project, *entry)
        }
    }
}

pub fn binding(operation: &crate::automation::AutomationOperation) -> Option<&ProjectBinding> {
    use crate::automation::AutomationOperation::*;
    match operation {
        PrepareStatic(r) => r.asset_project.as_ref(),
        PrepareAudio(r) => r.asset_project.as_ref(),
        PrepareAsset(r) => r.asset_project.as_ref(),
        PrepareCharacterPack(r) => r.asset_project.as_ref(),
        _ => None,
    }
}

pub(crate) fn finalize_job(
    store: &crate::job::JobStore,
    job: &crate::job::JobRecord,
    operation: &crate::automation::AutomationOperation,
) -> Result<(), CatalogError> {
    let Some(binding) = binding(operation) else {
        return Ok(());
    };
    if job.lifecycle_state != crate::job::JobLifecycleState::Succeeded {
        return Ok(());
    }
    let Some(pack) = job.artifacts.iter().find(|a| a.kind == "gsfpack") else {
        return Ok(());
    };
    let mut source = BTreeMap::new();
    source.insert("method".into(), serde_json::json!("local_job"));
    source.insert("sourceJobId".into(), serde_json::json!(job.job_id));
    source.insert("executionId".into(), serde_json::json!(job.job_id));
    source.insert(
        "operationSha256".into(),
        serde_json::json!(sha(&serde_json::to_vec(operation)?)),
    );
    let provenance = job.job_dir.join("execution-provenance.json");
    if provenance.is_file() {
        let bytes = fs::read(provenance)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        source.insert(
            "executionProvenanceSha256".into(),
            serde_json::json!(sha(&bytes)),
        );
        if let Some(producer) = value.get("producer") {
            source.insert("producerBuild".into(), producer.clone());
        }
    }
    let pending = stage_local(binding, &pack.path, source)?;
    let result = complete_local(&pending);
    let pending_hash = fs::read(&pending).ok().map(|bytes| sha(&bytes));
    store
        .update_record(&job.job_id, |record| {
            record
                .artifacts
                .retain(|a| a.kind != "asset_publication_request");
            if let Some(hash) = &pending_hash {
                record.artifacts.push(crate::job::JobArtifactRecord {
                    kind: "asset_publication_request".into(),
                    path: pending.clone(),
                    sha256: Some(hash.clone()),
                });
            }
            if let Err(error) = &result {
                record.error_code = Some("asset_registration_pending".into());
                record.error_summary = Some(error.to_string());
                record.recoverable = true;
                record.next_actions.push("recover_asset_publication".into());
            }
        })
        .map_err(|e| invalid(e.to_string()))?;
    result.map(|_| ())
}
