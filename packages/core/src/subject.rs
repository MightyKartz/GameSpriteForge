use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use image::{ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::asset_project::{
    hash_file, image_signature, normalize_static_image, read_project, read_style_lock,
    resolve_relative, STYLE_LOCK_FILE,
};
use crate::provider::{
    EditImageRequest, MediaGenerationProvider, ProviderError, ProviderImageReference, ReferenceRole,
};

pub const SUBJECT_LOCK_FILE: &str = "subject-lock.json";
pub const SUBJECT_PROFILE: &str = "subject-lock@1.0.0";

#[derive(Debug, Error)]
pub enum SubjectError {
    #[error("invalid subject: {0}")]
    Invalid(String),
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubjectSpecV1 {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub reference_images: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubjectIdentityBaselineV1 {
    pub perceptual_hash: String,
    pub foreground_scale: f32,
    pub edge_density: f32,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub subject_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubjectLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub id: String,
    pub name: String,
    pub revision: String,
    pub created_at: DateTime<Utc>,
    pub prompt: String,
    pub license: String,
    pub style_revision: String,
    pub style_sha256: String,
    pub provider_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    pub canonical_path: PathBuf,
    pub canonical_sha256: String,
    pub mask_path: PathBuf,
    pub mask_sha256: String,
    pub reference_sha256: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_asset_id: Option<String>,
    pub baseline: SubjectIdentityBaselineV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectBuildOutput {
    pub id: String,
    pub revision: String,
    pub subject_lock_path: PathBuf,
    pub canonical_path: PathBuf,
    pub canonical_sha256: String,
    pub mask_path: PathBuf,
    pub mask_sha256: String,
}

pub fn read_subject_spec(path: &Path) -> Result<SubjectSpecV1, SubjectError> {
    let mut spec: SubjectSpecV1 = serde_json::from_slice(&fs::read(path)?)?;
    validate_subject_spec(&spec)?;
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    spec.reference_images = spec
        .reference_images
        .iter()
        .map(|reference| resolve_relative(root, reference))
        .collect();
    for reference in &spec.reference_images {
        if !reference.is_file() {
            return Err(SubjectError::Invalid(format!(
                "subject reference does not exist: {}",
                reference.display()
            )));
        }
        image::open(reference)?;
    }
    Ok(spec)
}

pub fn validate_subject_spec(spec: &SubjectSpecV1) -> Result<(), SubjectError> {
    if spec.schema_version != "1" {
        return Err(SubjectError::Invalid(
            "subject spec requires schemaVersion 1".into(),
        ));
    }
    if !engine_safe(&spec.id) {
        return Err(SubjectError::Invalid(
            "subject id must contain only letters, numbers, '-' or '_'".into(),
        ));
    }
    if spec.name.trim().is_empty() || spec.prompt.trim().is_empty() {
        return Err(SubjectError::Invalid(
            "subject name and prompt are required".into(),
        ));
    }
    if spec.reference_images.len() > 2 {
        return Err(SubjectError::Invalid(
            "subject spec accepts at most two identity reference images".into(),
        ));
    }
    Ok(())
}

pub fn build_subject_lock(
    project_root: &Path,
    spec_path: &Path,
    provider_id: &str,
    profile_id: &str,
    provider: &dyn MediaGenerationProvider,
    work_dir: &Path,
) -> Result<SubjectBuildOutput, SubjectError> {
    let project =
        read_project(project_root).map_err(|error| SubjectError::Invalid(error.to_string()))?;
    if project.provider.id != provider_id || project.provider.profile_id != profile_id {
        return Err(SubjectError::Invalid(
            "subject generation must use the project Provider profile".into(),
        ));
    }
    let style_revision = project
        .current_style_revision
        .ok_or_else(|| SubjectError::Invalid("project has no current Style revision".into()))?;
    let style_path = project_root
        .join(".forge/styles")
        .join(&style_revision)
        .join(STYLE_LOCK_FILE);
    let style =
        read_style_lock(&style_path).map_err(|error| SubjectError::Invalid(error.to_string()))?;
    let mut spec = read_subject_spec(spec_path)?;
    spec.image_model =
        provider.resolved_image_model(spec.image_model.as_deref().or(style.image_model.as_deref()));
    let reference_sha256 = spec
        .reference_images
        .iter()
        .map(|path| hash_file(path).map_err(|error| SubjectError::Invalid(error.to_string())))
        .collect::<Result<Vec<_>, _>>()?;
    let revision = subject_revision(
        &spec,
        provider_id,
        profile_id,
        &style.revision,
        &style.board_sha256,
        &reference_sha256,
    )?;
    let subject_dir = project_root
        .join(".forge/subjects")
        .join(&spec.id)
        .join(&revision);
    let lock_path = subject_dir.join(SUBJECT_LOCK_FILE);
    if lock_path.is_file() {
        let lock = read_subject_lock(&lock_path)?;
        return Ok(output_from_lock(lock_path, &lock));
    }

    fs::create_dir_all(work_dir)?;
    let raw_path = work_dir.join("canonical-raw.png");
    let mut references = vec![ProviderImageReference::from_path(
        ReferenceRole::Style,
        style.board_path.clone(),
    )?];
    for reference in &spec.reference_images {
        references.push(ProviderImageReference::from_path(
            ReferenceRole::SubjectIdentity,
            reference.clone(),
        )?);
    }
    let media = provider.edit_image(
        &EditImageRequest {
            prompt: canonical_prompt(&spec.prompt),
            model: spec.image_model.clone().or(style.image_model.clone()),
            references,
            aspect_ratio: "1:1".into(),
            resolution: "1k".into(),
        },
        &raw_path,
    )?;
    if !media.path.starts_with(work_dir) {
        return Err(SubjectError::Invalid(
            "Provider subject output escaped the Job workspace".into(),
        ));
    }

    fs::create_dir_all(&subject_dir)?;
    let canonical_path = subject_dir.join("canonical.png");
    let normalized = normalize_static_image(
        &media.path,
        &canonical_path,
        style.character_canvas_size,
        true,
    )
    .map_err(|error| SubjectError::Invalid(error.to_string()))?;
    let signature = image_signature(&normalized);
    if !signature.alpha_present || !signature.cell_boundary_safe || signature.subject_count != 1 {
        return Err(SubjectError::Invalid(
            "canonical subject failed alpha, clipping, or single-subject gates".into(),
        ));
    }
    let mask_path = subject_dir.join("mask.png");
    let mask = ImageBuffer::from_fn(normalized.width(), normalized.height(), |x, y| {
        let alpha = normalized.get_pixel(x, y)[3];
        Rgba([255, 255, 255, alpha])
    });
    mask.save(&mask_path)?;
    let canonical_sha256 =
        hash_file(&canonical_path).map_err(|error| SubjectError::Invalid(error.to_string()))?;
    let mask_sha256 =
        hash_file(&mask_path).map_err(|error| SubjectError::Invalid(error.to_string()))?;
    let lock = SubjectLockV1 {
        schema_version: "1".into(),
        profile: SUBJECT_PROFILE.into(),
        id: spec.id.clone(),
        name: spec.name,
        revision: revision.clone(),
        created_at: Utc::now(),
        prompt: spec.prompt,
        license: spec.license,
        style_revision: style.revision,
        style_sha256: style.board_sha256,
        provider_id: provider_id.into(),
        profile_id: profile_id.into(),
        image_model: spec.image_model.or(style.image_model),
        canonical_path: canonical_path.clone(),
        canonical_sha256: canonical_sha256.clone(),
        mask_path: mask_path.clone(),
        mask_sha256: mask_sha256.clone(),
        reference_sha256,
        provider_asset_id: media.provider_asset_id,
        baseline: SubjectIdentityBaselineV1 {
            perceptual_hash: format!("{:016x}", signature.perceptual_hash),
            foreground_scale: signature.foreground_scale,
            edge_density: signature.edge_density,
            anchor_x: signature.anchor_x,
            anchor_y: signature.anchor_y,
            subject_count: signature.subject_count,
        },
    };
    write_json_atomic(&lock_path, &lock)?;
    Ok(output_from_lock(lock_path, &lock))
}

pub fn read_subject_lock(path: &Path) -> Result<SubjectLockV1, SubjectError> {
    let lock: SubjectLockV1 = serde_json::from_slice(&fs::read(path)?)?;
    if lock.schema_version != "1" || lock.profile != SUBJECT_PROFILE {
        return Err(SubjectError::Invalid("unsupported Subject Lock".into()));
    }
    for (path, expected, label) in [
        (&lock.canonical_path, &lock.canonical_sha256, "canonical"),
        (&lock.mask_path, &lock.mask_sha256, "mask"),
    ] {
        if !path.is_file() {
            return Err(SubjectError::Invalid(format!(
                "Subject Lock {label} is missing"
            )));
        }
        let actual = hash_file(path).map_err(|error| SubjectError::Invalid(error.to_string()))?;
        if &actual != expected {
            return Err(SubjectError::Invalid(format!(
                "Subject Lock {label} changed after locking"
            )));
        }
    }
    Ok(lock)
}

pub fn list_subject_locks(project_root: &Path) -> Result<Vec<SubjectLockV1>, SubjectError> {
    let root = project_root.join(".forge/subjects");
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut locks = Vec::new();
    for subject in fs::read_dir(root)? {
        let subject = subject?;
        if !subject.file_type()?.is_dir() {
            continue;
        }
        for revision in fs::read_dir(subject.path())? {
            let revision = revision?;
            let path = revision.path().join(SUBJECT_LOCK_FILE);
            if path.is_file() {
                locks.push(read_subject_lock(&path)?);
            }
        }
    }
    locks.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then(left.revision.cmp(&right.revision))
    });
    Ok(locks)
}

pub fn subject_lock_path(project_root: &Path, id: &str, revision: &str) -> PathBuf {
    project_root
        .join(".forge/subjects")
        .join(id)
        .join(revision)
        .join(SUBJECT_LOCK_FILE)
}

fn subject_revision(
    spec: &SubjectSpecV1,
    provider_id: &str,
    profile_id: &str,
    style_revision: &str,
    style_sha256: &str,
    reference_sha256: &[String],
) -> Result<String, SubjectError> {
    let input = serde_json::json!({
        "profile": SUBJECT_PROFILE,
        "spec": spec,
        "providerId": provider_id,
        "profileId": profile_id,
        "styleRevision": style_revision,
        "styleSha256": style_sha256,
        "referenceSha256": reference_sha256,
    });
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&input)?))[..16].to_string())
}

fn output_from_lock(path: PathBuf, lock: &SubjectLockV1) -> SubjectBuildOutput {
    SubjectBuildOutput {
        id: lock.id.clone(),
        revision: lock.revision.clone(),
        subject_lock_path: path,
        canonical_path: lock.canonical_path.clone(),
        canonical_sha256: lock.canonical_sha256.clone(),
        mask_path: lock.mask_path.clone(),
        mask_sha256: lock.mask_sha256.clone(),
    }
}

fn canonical_prompt(prompt: &str) -> String {
    format!(
        "Create the canonical identity image for this top-down 2D game character: {prompt}. The first reference defines visual style; identity references, when present, define the same character and must not be replaced. One complete subject only, neutral standing pose facing down, centered, fixed orthographic camera, transparent or flat removable background, no text, no UI, no props, no scene. Preserve face, hair, body proportions, clothing, colors, and equipment for every later animation frame."
    )
}

fn engine_safe(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn default_license() -> String {
    "MIT".into()
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), SubjectError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temp, path)?;
    Ok(())
}
