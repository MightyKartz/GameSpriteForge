use crate::{catalog::ProjectCatalogEntryV2, content_digest::ContentInventory};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryCatalog {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    /// Asset ID -> immutable AssetRecord object SHA-256.
    pub assets: BTreeMap<String, String>,
    pub migrated_from_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Location {
    pub root_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallReference {
    pub revision: String,
    pub project: Location,
    pub target: PathBuf,
    pub installed_at: DateTime<Utc>,
    pub evidence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetRecord {
    pub asset_id: String,
    pub name: String,
    pub kind: String,
    pub tags: Vec<String>,
    pub selected_revision: Option<String>,
    /// Latest successful build result for legacy build/diff adapters, independent of selection.
    pub build_revision: Option<String>,
    pub revisions: Vec<String>,
    pub locations: BTreeMap<String, Location>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub additional_locations: BTreeMap<String, Vec<Location>>,
    pub spec_locations: BTreeMap<String, Location>,
    pub installations: Vec<InstallReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reviews: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetRevision {
    pub schema_version: String,
    pub asset_id: String,
    pub content: Option<ContentInventory>,
    /// Legacy execution metadata with machine paths removed. Original migration
    /// bytes are retained separately, and are never silently rewritten.
    pub legacy: Option<ProjectCatalogEntryV2>,
    pub parent_revisions: Vec<String>,
    pub source: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Identity {
    pub(super) project_id: String,
    pub(super) name: String,
    pub(super) migrated_from_sha256: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct LocalConfig {
    pub(super) roots: BTreeMap<String, PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigrationAsset {
    pub asset_id: String,
    pub recorded_pack_sha256: String,
    pub content_sha256: Option<String>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigrationPreview {
    pub source_version: String,
    pub catalog_sha256: String,
    pub expected_sha256: String,
    pub assets: Vec<MigrationAsset>,
}
