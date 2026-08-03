use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const PROJECT_CATALOG_RELATIVE: &str = ".forge/catalog.json";

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("invalid project catalog: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogSubjectRefV1 {
    pub id: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogStyleRefV1 {
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogProviderRefV1 {
    pub provider_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogInstallRefV1 {
    pub godot_project: PathBuf,
    pub target: PathBuf,
    pub installed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCatalogEntryV1 {
    pub asset_id: String,
    pub name: String,
    pub kind: String,
    pub pack_path: PathBuf,
    pub pack_sha256: String,
    pub source_job_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<CatalogStyleRefV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<CatalogSubjectRefV1>,
    pub workflow: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<CatalogProviderRefV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed: Option<CatalogInstallRefV1>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCatalogV1 {
    pub schema_version: String,
    pub updated_at: DateTime<Utc>,
    pub assets: BTreeMap<String, ProjectCatalogEntryV1>,
}

impl Default for ProjectCatalogV1 {
    fn default() -> Self {
        Self {
            schema_version: "1".into(),
            updated_at: Utc::now(),
            assets: BTreeMap::new(),
        }
    }
}

pub fn read_project_catalog(project_root: &Path) -> Result<ProjectCatalogV1, CatalogError> {
    let path = project_root.join(PROJECT_CATALOG_RELATIVE);
    if !path.is_file() {
        return Ok(ProjectCatalogV1::default());
    }
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(CatalogError::Invalid(
            "project catalog may not be a symbolic link".into(),
        ));
    }
    let catalog: ProjectCatalogV1 = serde_json::from_slice(&fs::read(path)?)?;
    if catalog.schema_version != "1" {
        return Err(CatalogError::Invalid(format!(
            "unsupported catalog schemaVersion {}",
            catalog.schema_version
        )));
    }
    Ok(catalog)
}

pub fn register_catalog_asset(
    project_root: &Path,
    entry: ProjectCatalogEntryV1,
) -> Result<PathBuf, CatalogError> {
    validate_entry(&entry)?;
    let mut catalog = read_project_catalog(project_root)?;
    catalog.assets.insert(entry.asset_id.clone(), entry);
    catalog.updated_at = Utc::now();
    let path = project_root.join(PROJECT_CATALOG_RELATIVE);
    write_json_atomic(&path, &catalog)?;
    Ok(path)
}

pub fn link_catalog_install(
    project_root: &Path,
    asset_id: &str,
    godot_project: PathBuf,
    target: PathBuf,
) -> Result<PathBuf, CatalogError> {
    let mut catalog = read_project_catalog(project_root)?;
    let entry = catalog
        .assets
        .get_mut(asset_id)
        .ok_or_else(|| CatalogError::Invalid(format!("catalog asset not found: {asset_id}")))?;
    entry.installed = Some(CatalogInstallRefV1 {
        godot_project,
        target,
        installed_at: Utc::now(),
    });
    catalog.updated_at = Utc::now();
    let path = project_root.join(PROJECT_CATALOG_RELATIVE);
    write_json_atomic(&path, &catalog)?;
    Ok(path)
}

fn validate_entry(entry: &ProjectCatalogEntryV1) -> Result<(), CatalogError> {
    if entry.asset_id.trim().is_empty()
        || entry.name.trim().is_empty()
        || entry.kind.trim().is_empty()
        || entry.workflow.trim().is_empty()
    {
        return Err(CatalogError::Invalid(
            "assetId, name, kind, and workflow are required".into(),
        ));
    }
    if entry.pack_sha256.len() != 64
        || !entry
            .pack_sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(CatalogError::Invalid(
            "packSha256 must be a 64-character SHA-256".into(),
        ));
    }
    Ok(())
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), CatalogError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
