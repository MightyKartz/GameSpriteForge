use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::asset_project::{read_project, FORGE_PROJECT_FILE};
use crate::catalog::{
    read_project_catalog, CatalogError, CatalogReviewStatusV1, ProjectCatalogEntryV2,
};
use crate::project::read_project_manifest;

pub const PROJECT_AUDIT_PROFILE: &str = "project-audit@1.1.0";

#[derive(Debug, thiserror::Error)]
pub enum ProjectAuditError {
    #[error("invalid project audit: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("catalog error: {0}")]
    Catalog(#[from] CatalogError),
}

impl ProjectAuditError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "invalid_project_audit",
            Self::Io(_) => "project_audit_io_error",
            Self::Json(_) => "project_audit_invalid_json",
            Self::Catalog(error) => error.code(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAuditScopeV1 {
    Completeness,
    Consistency,
    Quality,
    Provenance,
    Godot,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSeverityV1 {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectAuditFindingV1 {
    pub scope: ProjectAuditScopeV1,
    pub severity: AuditSeverityV1,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectAuditSummaryV1 {
    pub asset_count: usize,
    pub audited_pack_count: usize,
    pub installed_asset_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub clean: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectAuditReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub project_id: String,
    pub project_path: PathBuf,
    pub scope: ProjectAuditScopeV1,
    pub generated_at: DateTime<Utc>,
    pub summary: ProjectAuditSummaryV1,
    pub findings: Vec<ProjectAuditFindingV1>,
}

pub fn audit_project(
    project_root: &Path,
    scope: ProjectAuditScopeV1,
) -> Result<ProjectAuditReportV1, ProjectAuditError> {
    let project_root = project_root.canonicalize()?;
    let project = read_project(&project_root)
        .map_err(|error| ProjectAuditError::Invalid(error.to_string()))?;
    let catalog = read_project_catalog(&project_root)?;
    let mut findings = Vec::new();
    let mut audited_pack_count = 0usize;
    let mut installed_asset_count = 0usize;
    let mut catalog_pack_paths = BTreeSet::new();
    let mut godot_projects = BTreeMap::<PathBuf, BTreeSet<String>>::new();

    if includes(scope, ProjectAuditScopeV1::Completeness)
        && !project_root.join(FORGE_PROJECT_FILE).is_file()
    {
        finding(
            &mut findings,
            ProjectAuditScopeV1::Completeness,
            AuditSeverityV1::Error,
            "project_file_missing",
            "forge-project.json is missing",
            None,
            Some(project_root.join(FORGE_PROJECT_FILE)),
        );
    }

    for entry in catalog.assets.values() {
        let pack_path = resolve_catalog_path(&project_root, &entry.pack_path);
        catalog_pack_paths.insert(pack_path.clone());
        if !pack_path.is_dir() {
            if includes(scope, ProjectAuditScopeV1::Completeness) {
                finding(
                    &mut findings,
                    ProjectAuditScopeV1::Completeness,
                    AuditSeverityV1::Error,
                    "pack_missing",
                    "catalog Pack directory is missing",
                    Some(entry.asset_id.clone()),
                    Some(pack_path),
                );
            }
            continue;
        }
        audited_pack_count += 1;
        audit_pack(entry, &pack_path, scope, &mut findings)?;
        if let Some(install) = &entry.installed {
            installed_asset_count += 1;
            godot_projects
                .entry(install.godot_project.clone())
                .or_default()
                .insert(entry.asset_id.clone());
            if includes(scope, ProjectAuditScopeV1::Godot) {
                audit_godot_install(entry, install, &mut findings)?;
            }
        }
    }

    if includes(scope, ProjectAuditScopeV1::Godot) {
        for (godot_project, expected_assets) in godot_projects {
            audit_godot_closure(
                &catalog.assets,
                &godot_project,
                &expected_assets,
                &mut findings,
            )?;
        }
    }

    if includes(scope, ProjectAuditScopeV1::Completeness) {
        for pack in discover_pack_directories(&project_root.join("build"))? {
            if !catalog_pack_paths.contains(&pack) {
                finding(
                    &mut findings,
                    ProjectAuditScopeV1::Completeness,
                    AuditSeverityV1::Warning,
                    "orphan_pack",
                    "Pack exists under build but is not registered in the project catalog",
                    None,
                    Some(pack),
                );
            }
        }
    }

    findings.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .reverse()
            .then(left.scope_name().cmp(right.scope_name()))
            .then(left.code.cmp(&right.code))
            .then(left.asset_id.cmp(&right.asset_id))
            .then(left.path.cmp(&right.path))
    });
    let error_count = findings
        .iter()
        .filter(|finding| finding.severity == AuditSeverityV1::Error)
        .count();
    let warning_count = findings
        .iter()
        .filter(|finding| finding.severity == AuditSeverityV1::Warning)
        .count();
    Ok(ProjectAuditReportV1 {
        schema_version: "1".into(),
        profile: PROJECT_AUDIT_PROFILE.into(),
        project_id: project.project_id,
        project_path: project_root,
        scope,
        generated_at: Utc::now(),
        summary: ProjectAuditSummaryV1 {
            asset_count: catalog.assets.len(),
            audited_pack_count,
            installed_asset_count,
            error_count,
            warning_count,
            clean: error_count == 0 && warning_count == 0,
        },
        findings,
    })
}

fn audit_pack(
    entry: &ProjectCatalogEntryV2,
    pack_path: &Path,
    scope: ProjectAuditScopeV1,
    findings: &mut Vec<ProjectAuditFindingV1>,
) -> Result<(), ProjectAuditError> {
    if includes(scope, ProjectAuditScopeV1::Quality)
        && (entry.game_ready == Some(false)
            || entry
                .review
                .as_ref()
                .is_some_and(|review| review.status == CatalogReviewStatusV1::Quarantined))
    {
        finding(
            findings,
            ProjectAuditScopeV1::Quality,
            AuditSeverityV1::Error,
            "catalog_asset_quarantined",
            "catalog asset is quarantined or explicitly not game-ready",
            Some(entry.asset_id.clone()),
            Some(pack_path.to_path_buf()),
        );
    }
    if includes(scope, ProjectAuditScopeV1::Completeness) {
        if let Err(error) = forge_pack::validate_pack_layout(pack_path) {
            finding(
                findings,
                ProjectAuditScopeV1::Completeness,
                AuditSeverityV1::Error,
                "pack_invalid",
                &error.to_string(),
                Some(entry.asset_id.clone()),
                Some(pack_path.to_path_buf()),
            );
        }
        match hash_directory(pack_path) {
            Ok(actual) if actual != entry.pack_sha256 => finding(
                findings,
                ProjectAuditScopeV1::Completeness,
                AuditSeverityV1::Error,
                "pack_sha_mismatch",
                "catalog packSha256 does not match Pack contents",
                Some(entry.asset_id.clone()),
                Some(pack_path.to_path_buf()),
            ),
            Err(error) => finding(
                findings,
                ProjectAuditScopeV1::Completeness,
                AuditSeverityV1::Error,
                "pack_hash_failed",
                &error.to_string(),
                Some(entry.asset_id.clone()),
                Some(pack_path.to_path_buf()),
            ),
            _ => {}
        }
    }

    if includes(scope, ProjectAuditScopeV1::Quality) {
        audit_verdict_file(
            entry,
            &pack_path.join("quality-report.json"),
            "quality_verdict_not_game_ready",
            findings,
        )?;
        let consistency = pack_path.join("consistency-report.json");
        if consistency.is_file() {
            audit_verdict_file(
                entry,
                &consistency,
                "consistency_verdict_not_game_ready",
                findings,
            )?;
        }
    }

    if includes(scope, ProjectAuditScopeV1::Consistency) {
        let collection = pack_path.join("collection-consistency-report.json");
        let portrait = pack_path.join("portrait-consistency-report.json");
        if entry.kind == "portrait_set" && portrait.is_file() {
            audit_verdict_file(entry, &portrait, "portrait_local_outlier_present", findings)?;
        } else if matches!(
            entry.kind.as_str(),
            "portrait_set" | "equipment_set" | "decal_set"
        ) && !collection.is_file()
        {
            finding(
                findings,
                ProjectAuditScopeV1::Consistency,
                AuditSeverityV1::Error,
                "collection_report_missing",
                "collection-backed Pack has no collection consistency report",
                Some(entry.asset_id.clone()),
                Some(collection),
            );
        } else if collection.is_file() {
            audit_verdict_file(entry, &collection, "collection_outlier_present", findings)?;
        }
        if let (Some(style), Some(locks)) = (&entry.style, &entry.locks) {
            if locks
                .style
                .as_deref()
                .is_some_and(|revision| revision != style.revision)
            {
                finding(
                    findings,
                    ProjectAuditScopeV1::Consistency,
                    AuditSeverityV1::Error,
                    "style_lock_mismatch",
                    "catalog style revision and locks.style differ",
                    Some(entry.asset_id.clone()),
                    Some(pack_path.to_path_buf()),
                );
            }
        }
    }

    if includes(scope, ProjectAuditScopeV1::Provenance) {
        let forgepack_path = pack_path.join("forgepack.json");
        let forgepack: serde_json::Value = serde_json::from_slice(&fs::read(&forgepack_path)?)?;
        let license = forgepack
            .pointer("/license/type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if license.trim().is_empty() {
            finding(
                findings,
                ProjectAuditScopeV1::Provenance,
                AuditSeverityV1::Error,
                "license_missing",
                "Pack license.type is missing",
                Some(entry.asset_id.clone()),
                Some(forgepack_path),
            );
        }
        if forgepack.get("source").is_none() {
            finding(
                findings,
                ProjectAuditScopeV1::Provenance,
                AuditSeverityV1::Error,
                "provenance_missing",
                "Pack source provenance is missing",
                Some(entry.asset_id.clone()),
                Some(pack_path.to_path_buf()),
            );
        }
        scan_text_tree(entry, pack_path, ProjectAuditScopeV1::Provenance, findings)?;
    }
    Ok(())
}

fn audit_verdict_file(
    entry: &ProjectCatalogEntryV2,
    path: &Path,
    code: &str,
    findings: &mut Vec<ProjectAuditFindingV1>,
) -> Result<(), ProjectAuditError> {
    if !path.is_file() {
        finding(
            findings,
            ProjectAuditScopeV1::Quality,
            AuditSeverityV1::Error,
            "quality_report_missing",
            "required quality report is missing",
            Some(entry.asset_id.clone()),
            Some(path.to_path_buf()),
        );
        return Ok(());
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    let verdict = value
        .get("verdict")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if !matches!(verdict, "game_ready" | "passed") {
        finding(
            findings,
            if code.starts_with("collection") {
                ProjectAuditScopeV1::Consistency
            } else {
                ProjectAuditScopeV1::Quality
            },
            AuditSeverityV1::Error,
            code,
            &format!("report verdict is {verdict:?}, not game_ready"),
            Some(entry.asset_id.clone()),
            Some(path.to_path_buf()),
        );
    }
    Ok(())
}

fn audit_godot_install(
    entry: &ProjectCatalogEntryV2,
    install: &crate::catalog::CatalogInstallRefV1,
    findings: &mut Vec<ProjectAuditFindingV1>,
) -> Result<(), ProjectAuditError> {
    let target = if install.target.is_absolute() {
        install.target.clone()
    } else {
        install.godot_project.join(&install.target)
    };
    if !target.is_dir() {
        finding(
            findings,
            ProjectAuditScopeV1::Godot,
            AuditSeverityV1::Error,
            "godot_install_missing",
            "catalog records an install but its target directory is missing",
            Some(entry.asset_id.clone()),
            Some(target),
        );
        return Ok(());
    }
    scan_text_tree(entry, &target, ProjectAuditScopeV1::Godot, findings)
}

fn audit_godot_closure(
    catalog_assets: &BTreeMap<String, ProjectCatalogEntryV2>,
    godot_project: &Path,
    expected_assets: &BTreeSet<String>,
    findings: &mut Vec<ProjectAuditFindingV1>,
) -> Result<(), ProjectAuditError> {
    let manifest = read_project_manifest(godot_project)
        .map_err(|error| ProjectAuditError::Invalid(error.to_string()))?;
    for asset_id in expected_assets {
        let Some(catalog) = catalog_assets.get(asset_id) else {
            continue;
        };
        let Some(installed) = manifest.assets.get(asset_id) else {
            finding(
                findings,
                ProjectAuditScopeV1::Godot,
                AuditSeverityV1::Error,
                "godot_manifest_asset_missing",
                "catalog install is absent from Godot .forge/assets.json",
                Some(asset_id.clone()),
                Some(godot_project.join(".forge/assets.json")),
            );
            continue;
        };
        let installed_kind = serde_json::to_value(installed.kind)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        if installed_kind != catalog.kind {
            finding(
                findings,
                ProjectAuditScopeV1::Godot,
                AuditSeverityV1::Error,
                "godot_asset_kind_mismatch",
                &format!(
                    "Godot kind {installed_kind:?} does not match catalog kind {:?}",
                    catalog.kind
                ),
                Some(asset_id.clone()),
                Some(godot_project.join(".forge/assets.json")),
            );
        }
        if installed.pack_sha256 != catalog.pack_sha256 {
            finding(
                findings,
                ProjectAuditScopeV1::Godot,
                AuditSeverityV1::Error,
                "godot_pack_sha_mismatch",
                "Godot manifest Pack SHA does not match catalog",
                Some(asset_id.clone()),
                Some(godot_project.join(".forge/assets.json")),
            );
        }
        if let Some(install) = &catalog.installed {
            if installed.godot_target != install.target {
                finding(
                    findings,
                    ProjectAuditScopeV1::Godot,
                    AuditSeverityV1::Error,
                    "godot_target_mismatch",
                    "Godot manifest target does not match catalog install target",
                    Some(asset_id.clone()),
                    Some(godot_project.join(".forge/assets.json")),
                );
            }
        }
        if catalog.game_ready == Some(false)
            || catalog
                .review
                .as_ref()
                .is_some_and(|review| review.status == CatalogReviewStatusV1::Quarantined)
        {
            finding(
                findings,
                ProjectAuditScopeV1::Godot,
                AuditSeverityV1::Error,
                "quarantined_asset_installed",
                "a quarantined or non-game-ready asset remains installed in Godot",
                Some(asset_id.clone()),
                Some(godot_project.join(&installed.godot_target)),
            );
        }
    }
    for (asset_id, installed) in &manifest.assets {
        let linked = catalog_assets.get(asset_id).is_some_and(|entry| {
            entry.installed.as_ref().is_some_and(|install| {
                install.godot_project == godot_project
                    && install.target == installed.godot_target
                    && entry.pack_sha256 == installed.pack_sha256
            })
        });
        if !linked {
            finding(
                findings,
                ProjectAuditScopeV1::Godot,
                AuditSeverityV1::Error,
                "orphan_godot_asset",
                "Godot .forge/assets.json entry has no matching installed catalog asset",
                Some(asset_id.clone()),
                Some(godot_project.join(&installed.godot_target)),
            );
        }
    }
    Ok(())
}

fn scan_text_tree(
    entry: &ProjectCatalogEntryV2,
    root: &Path,
    scope: ProjectAuditScopeV1,
    findings: &mut Vec<ProjectAuditFindingV1>,
) -> Result<(), ProjectAuditError> {
    for path in collect_files(root)? {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !matches!(extension, "json" | "log" | "tres" | "tscn" | "gd" | "txt") {
            continue;
        }
        let metadata = fs::metadata(&path)?;
        if matches!(extension, "tres" | "tscn") && metadata.len() >= 1024 * 1024 {
            finding(
                findings,
                ProjectAuditScopeV1::Godot,
                AuditSeverityV1::Error,
                "godot_text_resource_too_large",
                "Godot text resource is at least 1 MiB",
                Some(entry.asset_id.clone()),
                Some(path.clone()),
            );
        }
        if metadata.len() > 4 * 1024 * 1024 {
            continue;
        }
        let text = String::from_utf8_lossy(&fs::read(&path)?).to_string();
        let lowercase = text.to_ascii_lowercase();
        if lowercase.contains("packedbytearray")
            || lowercase.contains("imagetexture.create_from_image")
        {
            finding(
                findings,
                ProjectAuditScopeV1::Godot,
                AuditSeverityV1::Error,
                "embedded_image_resource",
                "Godot resource contains embedded image data",
                Some(entry.asset_id.clone()),
                Some(path.clone()),
            );
        }
        if lowercase.contains("authorization: bearer")
            || lowercase.contains("\"access_token\"")
            || lowercase.contains("\"refresh_token\"")
            || lowercase.contains("\"device_code\"")
            || lowercase.contains("x-api-key")
        {
            finding(
                findings,
                scope,
                AuditSeverityV1::Error,
                "credential_material_detected",
                "text artifact contains a forbidden credential marker",
                Some(entry.asset_id.clone()),
                Some(path.clone()),
            );
        }
        if lowercase.contains("https://")
            && (lowercase.contains("temporary")
                || lowercase.contains("signed_url")
                || lowercase.contains("expires="))
        {
            finding(
                findings,
                scope,
                AuditSeverityV1::Error,
                "temporary_media_url_detected",
                "text artifact contains a temporary or signed media URL",
                Some(entry.asset_id.clone()),
                Some(path),
            );
        }
    }
    Ok(())
}

fn discover_pack_directories(root: &Path) -> Result<Vec<PathBuf>, ProjectAuditError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    Ok(collect_directories(root)?
        .into_iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("gsfpack"))
        .collect())
}

fn collect_directories(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut directories = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                directories.push(entry.path());
                pending.push(entry.path());
            }
        }
    }
    directories.sort();
    Ok(directories)
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn hash_directory(root: &Path) -> Result<String, std::io::Error> {
    if fs::symlink_metadata(root)?.file_type().is_symlink() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "directory root may not be a symbolic link",
        ));
    }
    let mut paths = collect_files(root)?
        .into_iter()
        .map(|path| path.strip_prefix(root).unwrap_or(&path).to_path_buf())
        .collect::<Vec<_>>();
    paths.sort();
    let mut hasher = Sha256::new();
    hasher.update(b"forge-directory-hash-v2\0");
    for relative in paths {
        let relative_text = relative.to_string_lossy();
        let contents = fs::read(root.join(&relative))?;
        hasher.update(b"file\0");
        hasher.update((relative_text.len() as u64).to_le_bytes());
        hasher.update(relative_text.as_bytes());
        hasher.update((contents.len() as u64).to_le_bytes());
        hasher.update(contents);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn includes(selected: ProjectAuditScopeV1, requested: ProjectAuditScopeV1) -> bool {
    selected == ProjectAuditScopeV1::All || selected == requested
}

fn resolve_catalog_path(project_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

fn finding(
    findings: &mut Vec<ProjectAuditFindingV1>,
    scope: ProjectAuditScopeV1,
    severity: AuditSeverityV1,
    code: &str,
    message: &str,
    asset_id: Option<String>,
    path: Option<PathBuf>,
) {
    findings.push(ProjectAuditFindingV1 {
        scope,
        severity,
        code: code.into(),
        message: message.into(),
        asset_id,
        path,
    });
}

impl ProjectAuditFindingV1 {
    fn scope_name(&self) -> &'static str {
        match self.scope {
            ProjectAuditScopeV1::Completeness => "completeness",
            ProjectAuditScopeV1::Consistency => "consistency",
            ProjectAuditScopeV1::Quality => "quality",
            ProjectAuditScopeV1::Provenance => "provenance",
            ProjectAuditScopeV1::Godot => "godot",
            ProjectAuditScopeV1::All => "all",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_hash_is_order_independent_and_detects_changes() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("b.txt"), b"b").unwrap();
        fs::write(temp.path().join("a.txt"), b"a").unwrap();
        let first = hash_directory(temp.path()).unwrap();
        fs::write(temp.path().join("a.txt"), b"changed").unwrap();
        let second = hash_directory(temp.path()).unwrap();
        assert_ne!(first, second);
    }
}
