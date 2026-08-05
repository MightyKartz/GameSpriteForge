use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const PROJECT_CATALOG_RELATIVE: &str = ".forge/catalog.json";
pub const PROJECT_CATALOG_LOCK_RELATIVE: &str = ".forge/catalog.lock";

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("invalid project catalog: {0}")]
    Invalid(String),
    #[error("invalid project catalog: project catalog may not be a symbolic link")]
    Symlink,
    #[error("invalid project catalog: unsupported catalog schemaVersion {0}")]
    UnsupportedVersion(String),
    #[error("failed to lock project catalog: {0}")]
    Lock(std::io::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl CatalogError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "invalid_catalog",
            Self::Symlink => "catalog_symlink",
            Self::UnsupportedVersion(_) => "unsupported_catalog_version",
            Self::Lock(_) => "catalog_lock_failed",
            Self::Io(_) => "catalog_io_error",
            Self::Json(_) => "catalog_invalid_json",
        }
    }
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

/// Dependency pointer for a catalog asset: another asset/spec id plus,
/// optionally, the exact revision and content hash it was built against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogDependencyRefV1 {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

/// Style Lock / Subject Lock revisions the asset was generated against.
/// environment/collection/ui/effect are reserved for later lock kinds and
/// stay optional so V2 files remain lean.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogLockRevisionsV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
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

/// Stage 2 per-asset record: every V1 field plus the game-art manifest
/// provenance. All additions are optional so V2 files stay lean and older
/// readers (no deny_unknown_fields) can still parse them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCatalogEntryV2 {
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<CatalogDependencyRefV1>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locks: Option<CatalogLockRevisionsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_verdict: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quality_profile: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub game_ready: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reviewed_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance_summary: Option<String>,
}

impl From<ProjectCatalogEntryV1> for ProjectCatalogEntryV2 {
    fn from(entry: ProjectCatalogEntryV1) -> Self {
        Self {
            asset_id: entry.asset_id,
            name: entry.name,
            kind: entry.kind,
            pack_path: entry.pack_path,
            pack_sha256: entry.pack_sha256,
            source_job_id: entry.source_job_id,
            parent_job_id: entry.parent_job_id,
            style: entry.style,
            subject: entry.subject,
            workflow: entry.workflow,
            provider: entry.provider,
            installed: entry.installed,
            created_at: entry.created_at,
            spec_path: None,
            spec_sha256: None,
            dependencies: None,
            locks: None,
            workflow_profile: None,
            workflow_version: None,
            pack_version: None,
            quality_verdict: None,
            quality_profile: None,
            game_ready: None,
            generated_at: None,
            reviewed_at: None,
            license: None,
            provenance_summary: None,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCatalogV2 {
    pub schema_version: String,
    pub updated_at: DateTime<Utc>,
    pub assets: BTreeMap<String, ProjectCatalogEntryV2>,
}

impl Default for ProjectCatalogV2 {
    fn default() -> Self {
        Self {
            schema_version: "2".into(),
            updated_at: Utc::now(),
            assets: BTreeMap::new(),
        }
    }
}

impl From<ProjectCatalogV1> for ProjectCatalogV2 {
    fn from(catalog: ProjectCatalogV1) -> Self {
        Self {
            schema_version: "2".into(),
            updated_at: catalog.updated_at,
            assets: catalog
                .assets
                .into_iter()
                .map(|(key, entry)| (key, entry.into()))
                .collect(),
        }
    }
}

/// Reads the project catalog, upgrading V1 files to V2 in memory. A missing
/// catalog yields an empty V2 default; schemaVersion "3" or newer is rejected.
pub fn read_project_catalog(project_root: &Path) -> Result<ProjectCatalogV2, CatalogError> {
    let path = project_root.join(PROJECT_CATALOG_RELATIVE);
    if !path.is_file() {
        return Ok(ProjectCatalogV2::default());
    }
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(CatalogError::Symlink);
    }
    let bytes = fs::read(path)?;
    match catalog_schema_version(&bytes)?.as_str() {
        "1" => Ok(ProjectCatalogV2::from(serde_json::from_slice::<
            ProjectCatalogV1,
        >(&bytes)?)),
        "2" => Ok(serde_json::from_slice(&bytes)?),
        other => Err(CatalogError::UnsupportedVersion(other.to_owned())),
    }
}

/// Writes the catalog under an exclusive `.forge/catalog.lock` lock so
/// concurrent builds cannot lose updates or observe torn JSON.
pub fn write_project_catalog(
    project_root: &Path,
    catalog: &ProjectCatalogV2,
) -> Result<PathBuf, CatalogError> {
    let _lock = lock_catalog(project_root)?;
    write_project_catalog_unlocked(project_root, catalog)
}

pub fn register_catalog_asset(
    project_root: &Path,
    entry: ProjectCatalogEntryV1,
) -> Result<PathBuf, CatalogError> {
    validate_required_fields(
        &entry.asset_id,
        &entry.name,
        &entry.kind,
        &entry.workflow,
        &entry.pack_sha256,
    )?;
    let _lock = lock_catalog(project_root)?;
    let mut catalog = read_project_catalog(project_root)?;
    catalog.assets.insert(entry.asset_id.clone(), entry.into());
    write_project_catalog_unlocked(project_root, &catalog)
}

pub fn register_catalog_asset_v2(
    project_root: &Path,
    entry: ProjectCatalogEntryV2,
) -> Result<PathBuf, CatalogError> {
    validate_required_fields(
        &entry.asset_id,
        &entry.name,
        &entry.kind,
        &entry.workflow,
        &entry.pack_sha256,
    )?;
    let _lock = lock_catalog(project_root)?;
    let mut catalog = read_project_catalog(project_root)?;
    catalog.assets.insert(entry.asset_id.clone(), entry);
    write_project_catalog_unlocked(project_root, &catalog)
}

pub fn link_catalog_install(
    project_root: &Path,
    asset_id: &str,
    godot_project: PathBuf,
    target: PathBuf,
) -> Result<PathBuf, CatalogError> {
    let _lock = lock_catalog(project_root)?;
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
    write_project_catalog_unlocked(project_root, &catalog)
}

fn catalog_schema_version(bytes: &[u8]) -> Result<String, CatalogError> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    value
        .get("schemaVersion")
        .and_then(|version| version.as_str())
        .map(str::to_owned)
        .ok_or_else(|| CatalogError::Invalid("schemaVersion is required".into()))
}

fn lock_catalog(project_root: &Path) -> Result<File, CatalogError> {
    let path = project_root.join(PROJECT_CATALOG_LOCK_RELATIVE);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .map_err(CatalogError::Lock)?;
    file.lock().map_err(CatalogError::Lock)?;
    Ok(file)
}

fn write_project_catalog_unlocked(
    project_root: &Path,
    catalog: &ProjectCatalogV2,
) -> Result<PathBuf, CatalogError> {
    let mut catalog = catalog.clone();
    catalog.updated_at = Utc::now();
    let path = project_root.join(PROJECT_CATALOG_RELATIVE);
    write_json_atomic(&path, &catalog)?;
    Ok(path)
}

fn validate_required_fields(
    asset_id: &str,
    name: &str,
    kind: &str,
    workflow: &str,
    pack_sha256: &str,
) -> Result<(), CatalogError> {
    if asset_id.trim().is_empty()
        || name.trim().is_empty()
        || kind.trim().is_empty()
        || workflow.trim().is_empty()
    {
        return Err(CatalogError::Invalid(
            "assetId, name, kind, and workflow are required".into(),
        ));
    }
    if pack_sha256.len() != 64
        || !pack_sha256
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
    let temp = path.with_extension(format!("json.{}.tmp", Uuid::new_v4()));
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn fixed_time(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn sample_v1_entry(asset_id: &str, pack_sha256: &str) -> ProjectCatalogEntryV1 {
        ProjectCatalogEntryV1 {
            asset_id: asset_id.into(),
            name: format!("Asset {asset_id}"),
            kind: "character".into(),
            pack_path: PathBuf::from(format!("packs/{asset_id}.gsfpack")),
            pack_sha256: pack_sha256.into(),
            source_job_id: "job-1".into(),
            parent_job_id: Some("job-0".into()),
            style: Some(CatalogStyleRefV1 {
                revision: "7".into(),
            }),
            subject: Some(CatalogSubjectRefV1 {
                id: "hero-subject".into(),
                revision: "3".into(),
            }),
            workflow: "topdown-keyframes@2.0.0".into(),
            provider: Some(CatalogProviderRefV1 {
                provider_id: "xai".into(),
                profile_id: "default".into(),
                model: Some("grok-image".into()),
            }),
            installed: None,
            created_at: fixed_time("2026-01-01T00:00:00Z"),
        }
    }

    fn sample_v2_entry(asset_id: &str, pack_sha256: &str) -> ProjectCatalogEntryV2 {
        let mut entry = ProjectCatalogEntryV2::from(sample_v1_entry(asset_id, pack_sha256));
        entry.spec_path = Some(PathBuf::from(format!("specs/{asset_id}.json")));
        entry.spec_sha256 = Some("b".repeat(64));
        entry.dependencies = Some(vec![CatalogDependencyRefV1 {
            id: "shared-style".into(),
            revision: Some("7".into()),
            hash: Some("c".repeat(64)),
        }]);
        entry.locks = Some(CatalogLockRevisionsV1 {
            style: Some("7".into()),
            subject: Some("3".into()),
            ..CatalogLockRevisionsV1::default()
        });
        entry.workflow_profile = Some("topdown-keyframes".into());
        entry.workflow_version = Some("2.0.0".into());
        entry.pack_version = Some("1.0.0".into());
        entry.quality_verdict = Some("passed".into());
        entry.quality_profile = Some("character-hard-gates@1.0.0".into());
        entry.game_ready = Some(true);
        entry.generated_at = Some(fixed_time("2026-01-02T00:00:00Z"));
        entry.reviewed_at = Some(fixed_time("2026-01-03T00:00:00Z"));
        entry.license = Some("CC0-1.0".into());
        entry.provenance_summary = Some("xai grok-image via topdown-keyframes@2.0.0".into());
        entry
    }

    #[test]
    fn missing_catalog_returns_empty_v2_default() {
        let temp = tempfile::tempdir().unwrap();
        let catalog = read_project_catalog(temp.path()).unwrap();
        assert_eq!(catalog.schema_version, "2");
        assert!(catalog.assets.is_empty());
    }

    #[test]
    fn v1_catalog_upgrades_and_roundtrips_all_v1_fields() {
        let temp = tempfile::tempdir().unwrap();
        let entry = sample_v1_entry("hero", &"1".repeat(64));
        let v1 = ProjectCatalogV1 {
            schema_version: "1".into(),
            updated_at: fixed_time("2026-01-01T00:00:00Z"),
            assets: BTreeMap::from([("hero".to_owned(), entry.clone())]),
        };
        let path = temp.path().join(PROJECT_CATALOG_RELATIVE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, serde_json::to_vec_pretty(&v1).unwrap()).unwrap();

        let upgraded = read_project_catalog(temp.path()).unwrap();
        assert_eq!(upgraded.schema_version, "2");
        assert_eq!(upgraded.updated_at, v1.updated_at);
        let upgraded_entry = upgraded.assets.get("hero").unwrap();
        assert_eq!(
            upgraded_entry,
            &ProjectCatalogEntryV2::from(entry.clone()),
            "V1 upgrade must map every V1 field and default the rest"
        );

        write_project_catalog(temp.path(), &upgraded).unwrap();
        let reread = read_project_catalog(temp.path()).unwrap();
        assert_eq!(reread.assets, upgraded.assets);
        assert_eq!(
            ProjectCatalogEntryV2::from(entry),
            *reread.assets.get("hero").unwrap(),
            "write/re-read must preserve all V1 fields"
        );

        let on_disk: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(on_disk["schemaVersion"], "2");
        // Forward-readable: the V2 file still deserializes as a V1 catalog.
        let as_v1: ProjectCatalogV1 = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(as_v1.assets.len(), 1);
        assert_eq!(as_v1.assets["hero"].asset_id, "hero");
    }

    #[test]
    fn register_catalog_asset_v2_persists_richer_fields() {
        let temp = tempfile::tempdir().unwrap();
        let entry = sample_v2_entry("hero", &"2".repeat(64));
        register_catalog_asset_v2(temp.path(), entry.clone()).unwrap();

        let catalog = read_project_catalog(temp.path()).unwrap();
        assert_eq!(catalog.schema_version, "2");
        assert_eq!(catalog.assets.get("hero"), Some(&entry));

        let on_disk: serde_json::Value =
            serde_json::from_slice(&fs::read(temp.path().join(PROJECT_CATALOG_RELATIVE)).unwrap())
                .unwrap();
        let hero = &on_disk["assets"]["hero"];
        assert_eq!(hero["specSha256"], "b".repeat(64));
        assert_eq!(hero["gameReady"], true);
        assert_eq!(hero["license"], "CC0-1.0");
        assert_eq!(hero["locks"]["style"], "7");
        assert_eq!(hero["dependencies"][0]["id"], "shared-style");
    }

    #[test]
    fn v2_files_stay_lean_when_optional_fields_are_absent() {
        let temp = tempfile::tempdir().unwrap();
        register_catalog_asset(temp.path(), sample_v1_entry("hero", &"3".repeat(64))).unwrap();

        let on_disk: serde_json::Value =
            serde_json::from_slice(&fs::read(temp.path().join(PROJECT_CATALOG_RELATIVE)).unwrap())
                .unwrap();
        assert_eq!(on_disk["schemaVersion"], "2");
        let hero = &on_disk["assets"]["hero"];
        for absent in [
            "specPath",
            "specSha256",
            "dependencies",
            "locks",
            "workflowProfile",
            "workflowVersion",
            "packVersion",
            "qualityVerdict",
            "qualityProfile",
            "gameReady",
            "generatedAt",
            "reviewedAt",
            "license",
            "provenanceSummary",
        ] {
            assert!(
                hero.get(absent).is_none(),
                "absent optional field {absent} must not be serialized"
            );
        }
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(PROJECT_CATALOG_RELATIVE);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": "3",
                "updatedAt": "2026-01-01T00:00:00Z",
                "assets": {}
            }))
            .unwrap(),
        )
        .unwrap();

        let error = read_project_catalog(temp.path()).unwrap_err();
        assert_eq!(error.code(), "unsupported_catalog_version");
        assert!(error
            .to_string()
            .contains("unsupported catalog schemaVersion 3"));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_catalog_is_rejected() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real-catalog.json");
        fs::write(
            &real,
            br#"{"schemaVersion":"2","updatedAt":"2026-01-01T00:00:00Z","assets":{}}"#,
        )
        .unwrap();
        let link_dir = temp.path().join("project");
        fs::create_dir_all(link_dir.join(".forge")).unwrap();
        symlink(&real, link_dir.join(PROJECT_CATALOG_RELATIVE)).unwrap();

        let error = read_project_catalog(&link_dir).unwrap_err();
        assert_eq!(error.code(), "catalog_symlink");
        assert!(error.to_string().contains("symbolic link"));
    }

    #[test]
    fn concurrent_registers_do_not_lose_updates() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        const WRITERS: usize = 8;
        let barrier = Arc::new(Barrier::new(WRITERS));
        let mut handles = Vec::new();
        for index in 0..WRITERS {
            let barrier = barrier.clone();
            let root = root.clone();
            handles.push(thread::spawn(move || {
                let entry = sample_v1_entry(&format!("asset-{index}"), &format!("{index:064x}"));
                barrier.wait();
                register_catalog_asset(&root, entry).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }

        let catalog = read_project_catalog(&root).unwrap();
        assert_eq!(catalog.assets.len(), WRITERS, "no register may be lost");
        for index in 0..WRITERS {
            assert!(catalog.assets.contains_key(&format!("asset-{index}")));
        }
        // The file must be intact JSON (no torn writes) and V2 on disk.
        let on_disk: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(PROJECT_CATALOG_RELATIVE)).unwrap())
                .unwrap();
        assert_eq!(on_disk["schemaVersion"], "2");
        assert_eq!(on_disk["assets"].as_object().unwrap().len(), WRITERS);
    }

    #[test]
    fn link_catalog_install_preserves_v2_fields() {
        let temp = tempfile::tempdir().unwrap();
        let entry = sample_v2_entry("hero", &"4".repeat(64));
        register_catalog_asset_v2(temp.path(), entry).unwrap();

        link_catalog_install(
            temp.path(),
            "hero",
            PathBuf::from("/tmp/godot-project"),
            PathBuf::from("addons/forge_assets/hero"),
        )
        .unwrap();

        let catalog = read_project_catalog(temp.path()).unwrap();
        let linked = catalog.assets.get("hero").unwrap();
        let installed = linked.installed.as_ref().unwrap();
        assert_eq!(installed.godot_project, PathBuf::from("/tmp/godot-project"));
        assert_eq!(installed.target, PathBuf::from("addons/forge_assets/hero"));
        assert_eq!(linked.spec_sha256, Some("b".repeat(64)));
        assert_eq!(linked.game_ready, Some(true));
        assert_eq!(linked.license, Some("CC0-1.0".into()));
    }
}
