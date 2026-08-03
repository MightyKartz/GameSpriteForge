use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const PROJECT_MANIFEST_RELATIVE: &str = ".forge/assets.json";

#[derive(Debug, Error)]
pub enum ProjectManifestError {
    #[error("projectPath must contain project.godot: {0}")]
    InvalidProject(PathBuf),
    #[error("Forge project metadata may not traverse a symbolic link: {0}")]
    SymbolicLink(PathBuf),
    #[error("invalid project asset manifest schemaVersion: {0}")]
    InvalidSchema(String),
    #[error("io error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAssetRef {
    pub provider: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAssetKind {
    Animation,
    Character,
    IconSet,
    PropSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectPathKind {
    ProjectRelative,
    ExternalAbsolute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPathRef {
    pub kind: ProjectPathKind,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAssetAnimation {
    pub name: String,
    pub frame_count: usize,
    pub fps: f32,
    #[serde(rename = "loop")]
    pub loop_animation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAssetEntry {
    pub asset_id: String,
    pub name: String,
    pub kind: ProjectAssetKind,
    pub revision: u32,
    #[serde(default)]
    pub pack_sha256: String,
    pub pack: ProjectPathRef,
    pub godot_target: PathBuf,
    pub scene_path: PathBuf,
    pub sprite_frames_path: PathBuf,
    pub usage_path: PathBuf,
    pub default_animation: String,
    pub animations: Vec<ProjectAssetAnimation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_refs: Vec<ProviderAssetRef>,
    pub last_job_id: String,
    pub installed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAssetManifest {
    pub schema_version: String,
    pub updated_at: DateTime<Utc>,
    pub assets: BTreeMap<String, ProjectAssetEntry>,
}

impl Default for ProjectAssetManifest {
    fn default() -> Self {
        Self {
            schema_version: "1".into(),
            updated_at: Utc::now(),
            assets: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspection {
    pub project_path: PathBuf,
    pub manifest_path: PathBuf,
    pub manifest: ProjectAssetManifest,
}

pub struct RegisterProjectAsset<'a> {
    pub project_path: &'a Path,
    pub asset_key: &'a str,
    pub pack_path: &'a Path,
    pub pack_sha256: &'a str,
    pub godot_target: &'a Path,
    pub scene_path: &'a Path,
    pub sprite_frames_path: &'a Path,
    pub usage_path: &'a Path,
    pub pack: &'a forge_pack::PackInspectSummary,
    pub provider_refs: &'a [ProviderAssetRef],
    pub job_id: &'a str,
}

pub fn inspect_project(project_path: &Path) -> Result<ProjectInspection, ProjectManifestError> {
    validate_project(project_path)?;
    Ok(ProjectInspection {
        project_path: project_path.to_path_buf(),
        manifest_path: project_path.join(PROJECT_MANIFEST_RELATIVE),
        manifest: read_project_manifest(project_path)?,
    })
}

pub fn read_project_manifest(
    project_path: &Path,
) -> Result<ProjectAssetManifest, ProjectManifestError> {
    validate_project(project_path)?;
    let manifest_path = project_path.join(PROJECT_MANIFEST_RELATIVE);
    ensure_metadata_path_safe(project_path)?;
    if !manifest_path.exists() {
        return Ok(ProjectAssetManifest::default());
    }
    if fs::symlink_metadata(&manifest_path)
        .map_err(|source| io_error(&manifest_path, source))?
        .file_type()
        .is_symlink()
    {
        return Err(ProjectManifestError::SymbolicLink(manifest_path));
    }
    let manifest: ProjectAssetManifest = serde_json::from_slice(
        &fs::read(&manifest_path).map_err(|source| io_error(&manifest_path, source))?,
    )?;
    if manifest.schema_version != "1" {
        return Err(ProjectManifestError::InvalidSchema(manifest.schema_version));
    }
    Ok(manifest)
}

pub fn register_project_asset(
    params: RegisterProjectAsset<'_>,
) -> Result<PathBuf, ProjectManifestError> {
    let mut manifest = read_project_manifest(params.project_path)?;
    let project_root = fs::canonicalize(params.project_path)
        .map_err(|source| io_error(params.project_path, source))?;
    let canonical_pack =
        fs::canonicalize(params.pack_path).map_err(|source| io_error(params.pack_path, source))?;
    let pack_ref = if let Ok(relative) = canonical_pack.strip_prefix(&project_root) {
        ProjectPathRef {
            kind: ProjectPathKind::ProjectRelative,
            path: relative.to_path_buf(),
        }
    } else {
        ProjectPathRef {
            kind: ProjectPathKind::ExternalAbsolute,
            path: canonical_pack,
        }
    };
    let kind = match params.pack.asset_type.as_str() {
        "character" => ProjectAssetKind::Character,
        "icon_set" => ProjectAssetKind::IconSet,
        "prop_set" => ProjectAssetKind::PropSet,
        _ if params.pack.animations.len() > 1 => ProjectAssetKind::Character,
        _ => ProjectAssetKind::Animation,
    };
    let animations = params
        .pack
        .animations
        .iter()
        .map(|animation| ProjectAssetAnimation {
            name: animation.name.clone(),
            frame_count: animation.frame_count,
            fps: animation.fps,
            loop_animation: animation.loop_animation,
        })
        .collect::<Vec<_>>();
    let previous = manifest.assets.get(params.asset_key);
    let unchanged = previous.is_some_and(|entry| {
        entry.asset_id == params.pack.id
            && entry.pack_sha256 == params.pack_sha256
            && entry.pack == pack_ref
            && entry.godot_target == params.godot_target
            && entry.provider_refs == params.provider_refs
    });
    let revision = previous
        .map(|entry| {
            if unchanged {
                entry.revision
            } else {
                entry.revision + 1
            }
        })
        .unwrap_or(1);
    let installed_at = if unchanged {
        previous
            .map(|entry| entry.installed_at)
            .unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };
    manifest.assets.insert(
        params.asset_key.to_string(),
        ProjectAssetEntry {
            asset_id: params.pack.id.clone(),
            name: params.pack.name.clone(),
            kind,
            revision,
            pack_sha256: params.pack_sha256.to_string(),
            pack: pack_ref,
            godot_target: params.godot_target.to_path_buf(),
            scene_path: params.scene_path.to_path_buf(),
            sprite_frames_path: params.sprite_frames_path.to_path_buf(),
            usage_path: params.usage_path.to_path_buf(),
            default_animation: params.pack.default_animation.clone(),
            animations,
            provider_refs: params.provider_refs.to_vec(),
            last_job_id: params.job_id.to_string(),
            installed_at,
        },
    );
    manifest.updated_at = Utc::now();
    let manifest_path = params.project_path.join(PROJECT_MANIFEST_RELATIVE);
    let parent = manifest_path
        .parent()
        .expect("project manifest always has a parent");
    fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    write_json_atomic(&manifest_path, &manifest)?;
    Ok(manifest_path)
}

fn validate_project(project_path: &Path) -> Result<(), ProjectManifestError> {
    if !project_path.join("project.godot").is_file() {
        return Err(ProjectManifestError::InvalidProject(
            project_path.to_path_buf(),
        ));
    }
    Ok(())
}

fn ensure_metadata_path_safe(project_path: &Path) -> Result<(), ProjectManifestError> {
    let forge_dir = project_path.join(".forge");
    if forge_dir.exists()
        && fs::symlink_metadata(&forge_dir)
            .map_err(|source| io_error(&forge_dir, source))?
            .file_type()
            .is_symlink()
    {
        return Err(ProjectManifestError::SymbolicLink(forge_dir));
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), ProjectManifestError> {
    let temporary = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)
        .map_err(|source| io_error(&temporary, source))?;
    fs::rename(&temporary, path).map_err(|source| io_error(path, source))
}

fn io_error(path: &Path, source: std::io::Error) -> ProjectManifestError {
    ProjectManifestError::Io {
        path: path.to_path_buf(),
        source,
    }
}
