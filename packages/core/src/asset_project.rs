use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::export::{
    build_preview_gif, build_sprite_sheet, GifBackground, PreviewGifParameters,
    SpriteSheetParameters,
};
use crate::matting::{apply_chroma_key, ChromaParameters};
use crate::provider::{GenerateImageRequest, MediaGenerationProvider, ProviderError};
use crate::quality::{QualityMetrics, QualityReport, QualityVerdict};

pub const FORGE_PROJECT_FILE: &str = "forge-project.json";
pub const STYLE_LOCK_FILE: &str = "style-lock.json";
pub const CONSISTENCY_PROFILE: &str = "consistency@1.3.0";
pub const STYLE_BASELINE_PROFILE: &str = "style-baseline@2.3.0";
pub const KEYFRAME_HARD_GATE_PROFILE: &str = "keyframe-hard-defects@1.0.0";
const NORMALIZED_FOREGROUND_EXTENT: f32 = 0.82;
const PALETTE_SIMILARITY_SIGMA: f32 = 112.0;
const PALETTE_COLOR_LIMIT: usize = 24;

#[derive(Debug, Error)]
pub enum AssetProjectError {
    #[error("invalid project or asset spec: {0}")]
    Invalid(String),
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("export error: {0}")]
    Export(#[from] crate::export::ExportError),
    #[error("pack error: {0}")]
    Pack(#[from] forge_pack::PackError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderSelection {
    pub id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForgeProjectV1 {
    pub schema_version: String,
    pub project_id: String,
    pub name: String,
    pub provider: ProviderSelection,
    pub output_dir: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_style_revision: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_environment_revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SamplingMode {
    Nearest,
    Linear,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleSpecV1 {
    pub schema_version: String,
    pub prompt: String,
    #[serde(default)]
    pub reference_images: Vec<PathBuf>,
    #[serde(default = "default_perspective")]
    pub perspective: String,
    #[serde(default = "default_lighting")]
    pub lighting: String,
    #[serde(default = "default_outline")]
    pub outline: String,
    #[serde(default = "default_background")]
    pub background: String,
    #[serde(default = "default_sampling")]
    pub sampling: SamplingMode,
    #[serde(default = "default_character_canvas")]
    pub character_canvas_size: u32,
    #[serde(default = "default_icon_canvas")]
    pub icon_canvas_size: u32,
    #[serde(default = "default_prop_canvas")]
    pub prop_canvas_size: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleBaseline {
    pub palette: Vec<PaletteColor>,
    pub edge_density: f32,
    pub foreground_scale: f32,
    pub perceptual_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PaletteColor {
    pub color: String,
    pub weight: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StyleLockV1 {
    pub schema_version: String,
    pub revision: String,
    pub provider_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    pub prompt: String,
    pub perspective: String,
    pub lighting: String,
    pub outline: String,
    pub background: String,
    pub sampling: SamplingMode,
    pub character_canvas_size: u32,
    pub icon_canvas_size: u32,
    pub prop_canvas_size: u32,
    pub board_path: PathBuf,
    pub board_sha256: String,
    pub reference_sha256: Vec<String>,
    #[serde(default = "legacy_style_baseline_profile")]
    pub baseline_profile: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub migrated_from_revision: Option<String>,
    pub baseline: StyleBaseline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterAssetSpecV1 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<PathBuf>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubjectRevisionRefV1 {
    pub id: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetWorkflowRefV1 {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterAssetSpecV2 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub subject: SubjectRevisionRefV1,
    pub workflow: AssetWorkflowRefV1,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticAssetKind {
    IconSet,
    PropSet,
}

impl StaticAssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IconSet => "icon_set",
            Self::PropSet => "prop_set",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StaticAssetItemSpecV1 {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StaticAssetSetSpecV1 {
    pub schema_version: String,
    pub kind: StaticAssetKind,
    pub id: String,
    pub name: String,
    pub items: Vec<StaticAssetItemSpecV1>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsistencyVerdict {
    GameReady,
    AwaitingReview,
    Regenerate,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsistencyMetrics {
    pub palette_overlap: f32,
    pub foreground_scale_ratio: f32,
    pub edge_density_ratio: f32,
    pub anchor_drift_px: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub perceptual_similarity: Option<f32>,
    pub canvas_matches: bool,
    pub alpha_present: bool,
    pub cell_boundary_safe: bool,
    #[serde(default = "default_subject_count")]
    pub subject_count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsistencyItemReport {
    pub id: String,
    pub attempt: u8,
    pub metrics: ConsistencyMetrics,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConsistencyReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub asset_type: String,
    pub style_revision: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_baseline_profile: Option<String>,
    pub verdict: ConsistencyVerdict,
    pub items: Vec<ConsistencyItemReport>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageSignature {
    pub palette: Vec<PaletteColor>,
    pub edge_density: f32,
    pub foreground_scale: f32,
    pub perceptual_hash: u64,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub width: u32,
    pub height: u32,
    pub alpha_present: bool,
    pub cell_boundary_safe: bool,
    pub subject_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleBuildOutput {
    pub style_lock_path: PathBuf,
    pub board_path: PathBuf,
    pub revision: String,
    pub board_sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaticPackItem {
    pub id: String,
    pub name: String,
    pub image_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticPackOutput {
    pub pack_dir: PathBuf,
    pub contact_sheet_path: PathBuf,
    pub consistency_report_path: PathBuf,
}

pub fn init_project(path: &Path, name: &str) -> Result<ForgeProjectV1, AssetProjectError> {
    if name.trim().is_empty() {
        return Err(AssetProjectError::Invalid(
            "project name is required".into(),
        ));
    }
    fs::create_dir_all(path)?;
    let project_path = path.join(FORGE_PROJECT_FILE);
    if project_path.exists() {
        return Err(AssetProjectError::Invalid(format!(
            "project already exists: {}",
            project_path.display()
        )));
    }
    let project_id = safe_id(name);
    if project_id.is_empty() {
        return Err(AssetProjectError::Invalid(
            "project name must contain letters or numbers".into(),
        ));
    }
    fs::create_dir_all(path.join(".forge/styles"))?;
    fs::create_dir_all(path.join("specs"))?;
    fs::create_dir_all(path.join("build"))?;
    let project = ForgeProjectV1 {
        schema_version: "1".into(),
        project_id,
        name: name.trim().into(),
        provider: ProviderSelection {
            id: "xai".into(),
            profile_id: "default".into(),
        },
        output_dir: PathBuf::from("build"),
        current_style_revision: None,
        current_environment_revision: None,
    };
    write_json_atomic(&project_path, &project)?;
    Ok(project)
}

pub fn read_project(path: &Path) -> Result<ForgeProjectV1, AssetProjectError> {
    let project_path = if path.is_dir() {
        path.join(FORGE_PROJECT_FILE)
    } else {
        path.to_path_buf()
    };
    let project: ForgeProjectV1 = serde_json::from_slice(&fs::read(&project_path)?)?;
    if project.schema_version != "1" {
        return Err(AssetProjectError::Invalid(format!(
            "unsupported project schema: {}",
            project.schema_version
        )));
    }
    Ok(project)
}

pub fn build_style_lock(
    project_root: &Path,
    spec_path: &Path,
    provider_id: &str,
    profile_id: &str,
    provider: &dyn MediaGenerationProvider,
    work_dir: &Path,
) -> Result<StyleBuildOutput, AssetProjectError> {
    let mut project = read_project(project_root)?;
    let mut spec: StyleSpecV1 = serde_json::from_slice(&fs::read(spec_path)?)?;
    spec.image_model = provider.resolved_image_model(spec.image_model.as_deref());
    validate_style_spec(&spec)?;
    let spec_root = spec_path.parent().unwrap_or_else(|| Path::new("."));
    let references = spec
        .reference_images
        .iter()
        .map(|path| resolve_relative(spec_root, path))
        .collect::<Vec<_>>();
    for path in &references {
        if !path.is_file() {
            return Err(AssetProjectError::Invalid(format!(
                "style reference does not exist: {}",
                path.display()
            )));
        }
    }
    let reference_sha256 = references
        .iter()
        .map(|path| hash_file(path))
        .collect::<Result<Vec<_>, _>>()?;
    let revision = style_revision(&spec, provider_id, profile_id, &reference_sha256)?;
    let style_dir = project_root.join(".forge/styles").join(&revision);
    if style_dir.join(STYLE_LOCK_FILE).is_file() {
        let lock = read_style_lock(&style_dir.join(STYLE_LOCK_FILE))?;
        project.current_style_revision = Some(lock.revision.clone());
        write_json_atomic(&project_root.join(FORGE_PROJECT_FILE), &project)?;
        return Ok(StyleBuildOutput {
            style_lock_path: style_dir.join(STYLE_LOCK_FILE),
            board_path: lock.board_path,
            revision: lock.revision,
            board_sha256: lock.board_sha256,
        });
    }
    let legacy_revision = legacy_style_revision(&spec, provider_id, profile_id, &reference_sha256)?;
    let legacy_lock = project_root
        .join(".forge/styles")
        .join(&legacy_revision)
        .join(STYLE_LOCK_FILE);
    let migrated_lock = legacy_lock
        .is_file()
        .then(|| read_style_lock(&legacy_lock))
        .transpose()?;
    fs::create_dir_all(work_dir)?;
    let work_board = work_dir.join("style-board.png");
    if let Some(lock) = &migrated_lock {
        fs::copy(&lock.board_path, &work_board)?;
    } else if references.is_empty() {
        provider.generate_image(
            &GenerateImageRequest {
                prompt: style_board_prompt(&spec),
                model: spec.image_model.clone(),
                aspect_ratio: "1:1".into(),
                resolution: "1k".into(),
            },
            &work_board,
        )?;
    } else {
        write_contact_sheet(&references, &work_board, 768)?;
    }
    let _ = image::open(&work_board)?;
    fs::create_dir_all(&style_dir)?;
    let board_path = style_dir.join("style-board.png");
    fs::copy(&work_board, &board_path)?;
    let board_sha256 = hash_file(&board_path)?;
    let image = image::open(&board_path)?.to_rgba8();
    let baseline = style_baseline(&image)?;
    let lock = StyleLockV1 {
        schema_version: "1".into(),
        revision: revision.clone(),
        provider_id: provider_id.into(),
        profile_id: profile_id.into(),
        image_model: spec.image_model.clone(),
        prompt: spec.prompt,
        perspective: spec.perspective,
        lighting: spec.lighting,
        outline: spec.outline,
        background: spec.background,
        sampling: spec.sampling,
        character_canvas_size: spec.character_canvas_size,
        icon_canvas_size: spec.icon_canvas_size,
        prop_canvas_size: spec.prop_canvas_size,
        board_path: board_path.clone(),
        board_sha256: board_sha256.clone(),
        reference_sha256,
        baseline_profile: STYLE_BASELINE_PROFILE.into(),
        migrated_from_revision: migrated_lock.map(|lock| lock.revision),
        baseline,
    };
    let style_lock_path = style_dir.join(STYLE_LOCK_FILE);
    write_json_atomic(&style_lock_path, &lock)?;
    project.current_style_revision = Some(revision.clone());
    write_json_atomic(&project_root.join(FORGE_PROJECT_FILE), &project)?;
    Ok(StyleBuildOutput {
        style_lock_path,
        board_path,
        revision,
        board_sha256,
    })
}

pub fn read_style_lock(path: &Path) -> Result<StyleLockV1, AssetProjectError> {
    let lock: StyleLockV1 = serde_json::from_slice(&fs::read(path)?)?;
    if lock.schema_version != "1" || !lock.board_path.is_file() {
        return Err(AssetProjectError::Invalid(
            "style lock is invalid or its board is missing".into(),
        ));
    }
    if hash_file(&lock.board_path)? != lock.board_sha256 {
        return Err(AssetProjectError::Invalid(
            "style board changed after the style was locked".into(),
        ));
    }
    Ok(lock)
}

pub fn normalize_static_image(
    input: &Path,
    output: &Path,
    canvas_size: u32,
    bottom_anchor: bool,
) -> Result<RgbaImage, AssetProjectError> {
    validate_canvas_size(canvas_size)?;
    let source = image::open(input)?.to_rgba8();
    let keyed = apply_chroma_key(&source, &ChromaParameters::default())
        .map_err(|error| AssetProjectError::Invalid(error.to_string()))?;
    let bbox = alpha_bounds(&keyed).ok_or_else(|| {
        AssetProjectError::Invalid("generated image has no foreground after matting".into())
    })?;
    let cropped = image::imageops::crop_imm(
        &keyed,
        bbox.0,
        bbox.1,
        bbox.2 - bbox.0 + 1,
        bbox.3 - bbox.1 + 1,
    )
    .to_image();
    let usable = (canvas_size as f32 * 0.82).round() as u32;
    let ratio =
        (usable as f32 / cropped.width() as f32).min(usable as f32 / cropped.height() as f32);
    let width = (cropped.width() as f32 * ratio).round().max(1.0) as u32;
    let height = (cropped.height() as f32 * ratio).round().max(1.0) as u32;
    let resized = image::imageops::resize(&cropped, width, height, FilterType::Lanczos3);
    let mut canvas = ImageBuffer::from_pixel(canvas_size, canvas_size, Rgba([0, 0, 0, 0]));
    let x = (canvas_size - width) / 2;
    let y = if bottom_anchor {
        canvas_size
            .saturating_sub(height)
            .saturating_sub(canvas_size / 16)
    } else {
        (canvas_size - height) / 2
    };
    image::imageops::overlay(&mut canvas, &resized, x.into(), y.into());
    for pixel in canvas.pixels_mut() {
        if pixel[3] < 32 {
            *pixel = Rgba([0, 0, 0, 0]);
        }
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    canvas.save(output)?;
    Ok(canvas)
}

pub fn image_signature(image: &RgbaImage) -> ImageSignature {
    let bounds = alpha_bounds(image);
    let alpha_present =
        image.pixels().any(|pixel| pixel[3] < 250) && image.pixels().any(|pixel| pixel[3] > 16);
    let foreground_scale = bounds
        .map(|(left, top, right, bottom)| {
            let foreground_extent = (right - left + 1).max(bottom - top + 1) as f32;
            foreground_extent / image.width().max(image.height()).max(1) as f32
        })
        .unwrap_or(0.0);
    let (anchor_x, anchor_y, cell_boundary_safe) = bounds
        .map(|(left, top, right, bottom)| {
            (
                (left + right) as f32 / 2.0,
                bottom as f32,
                left > 0 && top > 0 && right + 1 < image.width() && bottom + 1 < image.height(),
            )
        })
        .unwrap_or((0.0, 0.0, false));
    ImageSignature {
        palette: palette(image),
        edge_density: edge_density(image),
        foreground_scale,
        perceptual_hash: perceptual_hash(image),
        anchor_x,
        anchor_y,
        width: image.width(),
        height: image.height(),
        alpha_present,
        cell_boundary_safe,
        subject_count: major_subject_count(image),
    }
}

pub fn assess_consistency(
    id: &str,
    attempt: u8,
    candidate: &ImageSignature,
    style: &StyleLockV1,
    identity: Option<&ImageSignature>,
    edge_reference: Option<&ImageSignature>,
    expected_canvas: u32,
) -> ConsistencyItemReport {
    let palette_overlap = palette_overlap(&candidate.palette, &style.baseline.palette);
    let foreground_scale_ratio = ratio(candidate.foreground_scale, NORMALIZED_FOREGROUND_EXTENT);
    let edge_density_ratio = ratio(
        candidate.edge_density,
        edge_reference
            .map(|reference| reference.edge_density)
            .unwrap_or(style.baseline.edge_density),
    );
    let identity = identity.map(|reference| {
        1.0 - (candidate.perceptual_hash ^ reference.perceptual_hash).count_ones() as f32 / 64.0
    });
    let metrics = ConsistencyMetrics {
        palette_overlap,
        foreground_scale_ratio,
        edge_density_ratio,
        anchor_drift_px: (candidate.anchor_x - expected_canvas as f32 / 2.0).abs(),
        perceptual_similarity: identity,
        canvas_matches: candidate.width == expected_canvas && candidate.height == expected_canvas,
        alpha_present: candidate.alpha_present,
        cell_boundary_safe: candidate.cell_boundary_safe,
        subject_count: candidate.subject_count,
    };
    let mut reasons = Vec::new();
    let hard_blocked = !metrics.canvas_matches
        || !metrics.alpha_present
        || !metrics.cell_boundary_safe
        || metrics.subject_count != 1;
    if !metrics.canvas_matches {
        reasons.push("canvas_mismatch".into());
    }
    if !metrics.alpha_present {
        reasons.push("alpha_missing".into());
    }
    if !metrics.cell_boundary_safe {
        reasons.push("foreground_clipped".into());
    }
    if metrics.subject_count == 0 {
        reasons.push("subject_missing".into());
    } else if metrics.subject_count > 1 {
        reasons.push("multiple_subjects".into());
    }
    let regenerate = palette_overlap < 0.55
        || !(0.70..=1.30).contains(&foreground_scale_ratio)
        || !(0.60..=1.40).contains(&edge_density_ratio)
        || metrics.anchor_drift_px > 6.0
        || identity.is_some_and(|score| score < 0.55);
    let review = palette_overlap < 0.70
        || !(0.80..=1.20).contains(&foreground_scale_ratio)
        || !(0.75..=1.25).contains(&edge_density_ratio)
        || metrics.anchor_drift_px > 2.0
        || identity.is_some_and(|score| score < 0.70);
    if palette_overlap < 0.70 {
        reasons.push("palette_drift".into());
    }
    if !(0.80..=1.20).contains(&foreground_scale_ratio) {
        reasons.push("foreground_scale_drift".into());
    }
    if !(0.75..=1.25).contains(&edge_density_ratio) {
        reasons.push("edge_density_drift".into());
    }
    if metrics.anchor_drift_px > 2.0 {
        reasons.push("anchor_drift".into());
    }
    if identity.is_some_and(|score| score < 0.70) {
        reasons.push("identity_similarity_low".into());
    }
    let verdict = if hard_blocked {
        ConsistencyVerdict::Blocked
    } else if regenerate {
        ConsistencyVerdict::Regenerate
    } else if review {
        ConsistencyVerdict::AwaitingReview
    } else {
        ConsistencyVerdict::GameReady
    };
    ConsistencyItemReport {
        id: id.into(),
        attempt,
        metrics,
        verdict,
        reasons,
    }
}

pub fn apply_keyframe_hard_defects(
    candidate: &RgbaImage,
    identity: &RgbaImage,
    pose_guide: &RgbaImage,
    report: &mut ConsistencyItemReport,
) {
    let mut hard_reasons = Vec::new();
    if has_opaque_background_residual(candidate) {
        hard_reasons.push("opaque_background_residual");
    }
    if disconnected_low_alpha_noise_ratio(candidate) > 0.02 {
        hard_reasons.push("low_alpha_noise");
    }

    let aspect_ratio = foreground_aspect_ratio(candidate);
    let identity_aspect_ratio = foreground_aspect_ratio(identity);
    let aspect_ratio_drift = match (aspect_ratio, identity_aspect_ratio) {
        (Some(candidate), Some(identity)) if identity > f32::EPSILON => candidate / identity,
        _ => 1.0,
    };
    if !(0.60..=1.60).contains(&aspect_ratio_drift) {
        hard_reasons.push("silhouette_aspect_drift");
    }
    let candidate_pose_overlap = pose_guide_color_overlap(candidate, pose_guide);
    let identity_pose_overlap = pose_guide_color_overlap(identity, pose_guide);
    if candidate_pose_overlap > 0.04 && candidate_pose_overlap > identity_pose_overlap + 0.025 {
        hard_reasons.push("pose_structure_leak");
    }

    if !hard_reasons.is_empty() {
        report.verdict = ConsistencyVerdict::Blocked;
        for reason in hard_reasons {
            if !report.reasons.iter().any(|existing| existing == reason) {
                report.reasons.push(reason.into());
            }
        }
    }
}

fn has_opaque_background_residual(image: &RgbaImage) -> bool {
    let Some((left, top, right, bottom)) = alpha_bounds(image) else {
        return false;
    };
    let bbox_area = u64::from(right - left + 1) * u64::from(bottom - top + 1);
    let canvas_area = u64::from(image.width()) * u64::from(image.height());
    if bbox_area == 0 || canvas_area == 0 || bbox_area as f32 / (canvas_area as f32) < 0.20 {
        return false;
    }
    let foreground = image.pixels().filter(|pixel| pixel[3] > 16).count() as u64;
    foreground as f32 / bbox_area as f32 >= 0.90
}

fn foreground_aspect_ratio(image: &RgbaImage) -> Option<f32> {
    alpha_bounds(image).map(|(left, top, right, bottom)| {
        (right - left + 1) as f32 / (bottom - top + 1).max(1) as f32
    })
}

fn disconnected_low_alpha_noise_ratio(image: &RgbaImage) -> f32 {
    let components = alpha_component_areas(image, 0);
    let Some(largest) = components.iter().max() else {
        return 0.0;
    };
    let total = components.iter().sum::<usize>();
    let disconnected = total.saturating_sub(*largest);
    if disconnected < 64 || total == 0 {
        0.0
    } else {
        disconnected as f32 / total as f32
    }
}

fn alpha_component_areas(image: &RgbaImage, threshold: u8) -> Vec<usize> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    if width == 0 || height == 0 {
        return Vec::new();
    }
    let foreground = image
        .pixels()
        .map(|pixel| pixel[3] > threshold)
        .collect::<Vec<_>>();
    let mut visited = vec![false; foreground.len()];
    let mut areas = Vec::new();
    for start in 0..foreground.len() {
        if !foreground[start] || visited[start] {
            continue;
        }
        visited[start] = true;
        let mut queue = VecDeque::from([start]);
        let mut area = 0usize;
        while let Some(index) = queue.pop_front() {
            area += 1;
            let x = index % width;
            let y = index / width;
            for neighbor_y in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for neighbor_x in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let neighbor = neighbor_y * width + neighbor_x;
                    if foreground[neighbor] && !visited[neighbor] {
                        visited[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        areas.push(area);
    }
    areas
}

fn pose_guide_color_overlap(candidate: &RgbaImage, pose_guide: &RgbaImage) -> f32 {
    let mut colors = HashMap::<[u8; 3], usize>::new();
    for pixel in pose_guide.pixels() {
        let rgb = [pixel[0], pixel[1], pixel[2]];
        let range = *rgb.iter().max().unwrap_or(&0) - *rgb.iter().min().unwrap_or(&0);
        if pixel[3] > 16 && range > 32 && rgb.iter().any(|channel| *channel < 240) {
            *colors.entry(rgb).or_default() += 1;
        }
    }
    let Some((guide_color, _)) = colors.into_iter().max_by_key(|(_, count)| *count) else {
        return 0.0;
    };
    let mut foreground = 0usize;
    let mut matching = 0usize;
    for pixel in candidate.pixels() {
        if pixel[3] <= 16 {
            continue;
        }
        foreground += 1;
        let distance_squared = [pixel[0], pixel[1], pixel[2]]
            .iter()
            .zip(guide_color)
            .map(|(candidate, guide)| {
                let delta = i32::from(*candidate) - i32::from(guide);
                delta * delta
            })
            .sum::<i32>();
        matching += usize::from(distance_squared <= 40_i32.pow(2));
    }
    if foreground == 0 {
        0.0
    } else {
        matching as f32 / foreground as f32
    }
}

pub fn write_contact_sheet(
    sources: &[PathBuf],
    output: &Path,
    cell_size: u32,
) -> Result<(), AssetProjectError> {
    if sources.is_empty() {
        return Err(AssetProjectError::Invalid(
            "contact sheet requires at least one image".into(),
        ));
    }
    let columns = (sources.len() as f32).sqrt().ceil().max(1.0) as u32;
    let rows = (sources.len() as u32).div_ceil(columns);
    let mut sheet = ImageBuffer::from_pixel(
        columns * cell_size,
        rows * cell_size,
        Rgba([32, 34, 40, 255]),
    );
    for (index, source) in sources.iter().enumerate() {
        let image = image::open(source)?.to_rgba8();
        let thumbnail = DynamicImage::ImageRgba8(image).thumbnail(cell_size, cell_size);
        let x = index as u32 % columns * cell_size + (cell_size - thumbnail.width()) / 2;
        let y = index as u32 / columns * cell_size + (cell_size - thumbnail.height()) / 2;
        image::imageops::overlay(&mut sheet, &thumbnail.to_rgba8(), x.into(), y.into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    sheet.save(output)?;
    Ok(())
}

pub fn export_static_pack(
    exports_root: &Path,
    asset: &StaticAssetSetSpecV1,
    style: &StyleLockV1,
    provider_id: &str,
    items: &[StaticPackItem],
    report: &ConsistencyReportV1,
) -> Result<StaticPackOutput, AssetProjectError> {
    if items.is_empty() || items.len() != asset.items.len() {
        return Err(AssetProjectError::Invalid(
            "static pack requires one generated image per declared item".into(),
        ));
    }
    let export_dir = exports_root.join(&asset.id);
    let frames_dir = export_dir.join("frames");
    fs::create_dir_all(&frames_dir)?;
    let mut frames = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let target = frames_dir.join(format!("frame_{:03}.png", index + 1));
        fs::copy(&item.image_path, &target)?;
        frames.push(target);
    }
    let sheet = build_sprite_sheet(
        &frames,
        &export_dir,
        SpriteSheetParameters {
            columns: (items.len() as f32).sqrt().ceil().max(1.0) as u32,
            padding_px: 2,
            margin_px: 2,
            max_texture_size: 4096,
            allow_multi_sheet: true,
        },
    )?;
    let preview_path = export_dir.join("preview.gif");
    build_preview_gif(
        &frames,
        &preview_path,
        PreviewGifParameters {
            fps: 2.0,
            loop_animation: true,
            background: GifBackground::Transparent,
        },
    )?;
    let contact_sheet_path = export_dir.join("contact-sheet.png");
    write_contact_sheet(
        &frames,
        &contact_sheet_path,
        image::open(&frames[0])
            .map(|image| image.width())
            .unwrap_or(256),
    )?;

    let manifest_items = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let attempt = report
                .items
                .iter()
                .find(|entry| entry.id == item.id)
                .map(|entry| entry.attempt)
                .unwrap_or_default();
            serde_json::json!({
                "id": item.id,
                "name": item.name,
                "frame": index,
                "texture": format!("assets/items/{}.png", item.id),
                "provenance": {
                    "providerId": provider_id,
                    "styleRevision": style.revision,
                    "attempt": attempt,
                    "sha256": hash_file(&item.image_path).ok(),
                },
            })
        })
        .collect::<Vec<_>>();
    let animations = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            serde_json::json!({
                "name": item.id,
                "frames": [index],
                "fps": 1.0,
                "loop": false,
            })
        })
        .collect::<Vec<_>>();
    let sheet_images = sheet
        .sprite_sheet_paths
        .iter()
        .map(|path| {
            format!(
                "assets/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            )
        })
        .collect::<Vec<_>>();
    let canvas = image::open(&frames[0])?.width();
    let mut manifest = serde_json::json!({
        "assetType": asset.kind.as_str(),
        "name": asset.name,
        "sheet": {
            "image": "assets/sprite_sheet.png",
            "frameWidth": canvas,
            "frameHeight": canvas,
            "columns": sheet.atlas.columns,
            "rows": sheet.atlas.rows,
        },
        "animations": animations,
        "anchor": {
            "type": if asset.kind == StaticAssetKind::IconSet { "center" } else { "feet" },
            "x": canvas as f32 / 2.0,
            "y": if asset.kind == StaticAssetKind::IconSet { canvas as f32 / 2.0 } else { canvas as f32 },
        },
        "items": manifest_items,
    });
    if sheet_images.len() > 1 {
        manifest["sheet"]["images"] = serde_json::json!(sheet_images);
    }
    let quality = QualityReport {
        verdict: QualityVerdict::GameReady,
        metrics: QualityMetrics {
            bbox_bottom_drift_px: 0.0,
            bbox_center_x_drift_px: 0.0,
            bbox_center_y_drift_px: 0.0,
            bbox_width_variation_px: 0.0,
            alpha_coverage_avg: 0.25,
            loop_match_score: 1.0,
            frame_count: items.len(),
            frame_size_consistent: true,
            cell_boundary_safe: true,
        },
        recommendations: vec![],
        notes: vec!["static_asset_set".into()],
    };
    let forgepack_items = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let attempt = report
                .items
                .iter()
                .find(|entry| entry.id == item.id)
                .map(|entry| entry.attempt)
                .unwrap_or_default();
            serde_json::json!({
                "id": item.id,
                "name": item.name,
                "frame": index,
                "texture": format!("assets/items/{}.png", item.id),
                "provenance": {
                    "providerId": provider_id,
                    "styleRevision": style.revision,
                    "attempt": attempt,
                    "sha256": hash_file(&item.image_path).ok(),
                },
            })
        })
        .collect::<Vec<_>>();
    let forgepack = serde_json::json!({
        "schemaVersion": "2.0.0",
        "assetType": asset.kind.as_str(),
        "id": asset.id,
        "name": asset.name,
        "version": "0.1.0",
        "createdAt": chrono::Utc::now(),
        "creator": { "name": "Game Sprite Forge" },
        "license": { "type": asset.license },
        "source": {
            "kind": "provider_generation",
            "name": provider_id,
            "metadata": {
                "provider": provider_id,
                "styleRevision": style.revision,
                "styleBaselineProfile": style.baseline_profile,
                "consistencyProfile": CONSISTENCY_PROFILE,
            }
        },
        "animations": animations,
        "items": forgepack_items,
        "assets": {
            "frames": "assets/frames",
            "spriteSheet": "assets/sprite_sheet.png",
            "atlas": "assets/atlas.json",
            "manifest": "assets/manifest.json",
            "godotHelper": "assets/godot_import.json",
            "qualityReport": "quality-report.json",
            "consistencyReport": "consistency-report.json",
        },
        "previews": { "gif": "previews/preview.gif" },
    });

    let pack_dir = export_dir.join(format!("{}.gsfpack", asset.id));
    let pack_assets = pack_dir.join("assets");
    fs::create_dir_all(pack_assets.join("frames"))?;
    fs::create_dir_all(pack_assets.join("items"))?;
    fs::create_dir_all(pack_dir.join("previews"))?;
    for (index, item) in items.iter().enumerate() {
        fs::copy(
            &frames[index],
            pack_assets
                .join("frames")
                .join(format!("frame_{:03}.png", index + 1)),
        )?;
        fs::copy(
            &item.image_path,
            pack_assets.join("items").join(format!("{}.png", item.id)),
        )?;
    }
    for sheet_path in &sheet.sprite_sheet_paths {
        fs::copy(
            sheet_path,
            pack_assets.join(sheet_path.file_name().unwrap_or_default()),
        )?;
    }
    fs::copy(&sheet.atlas_path, pack_assets.join("atlas.json"))?;
    fs::write(
        pack_assets.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::write(
        pack_assets.join("godot_import.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "assetType": asset.kind.as_str(),
            "items": manifest["items"],
            "textures": sheet_images,
        }))?,
    )?;
    fs::write(
        pack_dir.join("forgepack.json"),
        serde_json::to_vec_pretty(&forgepack)?,
    )?;
    fs::write(
        pack_dir.join("quality-report.json"),
        serde_json::to_vec_pretty(&quality)?,
    )?;
    let consistency_report_path = pack_dir.join("consistency-report.json");
    fs::write(&consistency_report_path, serde_json::to_vec_pretty(report)?)?;
    fs::copy(&preview_path, pack_dir.join("previews/preview.gif"))?;
    fs::copy(
        &contact_sheet_path,
        pack_dir.join("previews/contact-sheet.png"),
    )?;
    forge_pack::validate_pack_layout(&pack_dir)?;
    Ok(StaticPackOutput {
        pack_dir,
        contact_sheet_path,
        consistency_report_path,
    })
}

pub fn resolve_relative(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

pub fn hash_file(path: &Path) -> Result<String, AssetProjectError> {
    Ok(format!("{:x}", Sha256::digest(fs::read(path)?)))
}

pub fn safe_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn validate_style_spec(spec: &StyleSpecV1) -> Result<(), AssetProjectError> {
    if spec.schema_version != "1" {
        return Err(AssetProjectError::Invalid(
            "style spec requires schemaVersion \"1\"".into(),
        ));
    }
    if spec.prompt.trim().is_empty() || spec.prompt.len() > 4_000 {
        return Err(AssetProjectError::Invalid(
            "style prompt must contain 1..=4000 characters".into(),
        ));
    }
    if spec.reference_images.len() > 3 {
        return Err(AssetProjectError::Invalid(
            "style supports at most three reference images".into(),
        ));
    }
    for size in [
        spec.character_canvas_size,
        spec.icon_canvas_size,
        spec.prop_canvas_size,
    ] {
        validate_canvas_size(size)?;
    }
    Ok(())
}

fn validate_canvas_size(size: u32) -> Result<(), AssetProjectError> {
    if !(64..=512).contains(&size) || !size.is_power_of_two() {
        return Err(AssetProjectError::Invalid(
            "canvas size must be a power of two from 64 through 512".into(),
        ));
    }
    Ok(())
}

fn style_revision(
    spec: &StyleSpecV1,
    provider_id: &str,
    profile_id: &str,
    references: &[String],
) -> Result<String, AssetProjectError> {
    let bytes = serde_json::to_vec(&(
        STYLE_BASELINE_PROFILE,
        spec,
        provider_id,
        profile_id,
        references,
    ))?;
    Ok(format!("{:x}", Sha256::digest(bytes))[..16].to_string())
}

fn legacy_style_revision(
    spec: &StyleSpecV1,
    provider_id: &str,
    profile_id: &str,
    references: &[String],
) -> Result<String, AssetProjectError> {
    let bytes = serde_json::to_vec(&(spec, provider_id, profile_id, references))?;
    Ok(format!("{:x}", Sha256::digest(bytes))[..16].to_string())
}

fn style_board_prompt(spec: &StyleSpecV1) -> String {
    format!(
        "{}. Create one clean 2D game art style board showing a character, an inventory icon, and a prop on one flat solid neutral background. Perspective: {}. Lighting: {}. Outline: {}. Keep palette, rendering, line weight, material language, and camera consistent. No border ornaments, text, logos, UI chrome, or photorealistic scene.",
        spec.prompt, spec.perspective, spec.lighting, spec.outline
    )
}

fn style_signature_image(image: &RgbaImage) -> Result<RgbaImage, AssetProjectError> {
    let original_coverage = alpha_coverage(image);
    if original_coverage > 0.02 && original_coverage < 0.90 {
        return Ok(image.clone());
    }

    let candidate = remove_dominant_border_background(image).ok_or_else(|| {
        AssetProjectError::Invalid(
            "style board has no separable, border-connected background for foreground metrics"
                .into(),
        )
    })?;
    let coverage = alpha_coverage(&candidate);
    if !(0.02..=0.80).contains(&coverage) {
        return Err(AssetProjectError::Invalid(format!(
            "style board foreground coverage {coverage:.3} is outside 0.02..=0.80"
        )));
    }
    Ok(candidate)
}

fn style_baseline(image: &RgbaImage) -> Result<StyleBaseline, AssetProjectError> {
    let foreground = style_signature_image(image)?;
    let foreground_signature = image_signature(&foreground);
    let edge_image = apply_chroma_key(image, &ChromaParameters::default())
        .ok()
        .filter(|candidate| alpha_bounds(candidate).is_some())
        .unwrap_or_else(|| image.clone());
    let edge_signature = image_signature(&edge_image);
    Ok(StyleBaseline {
        palette: foreground_signature.palette,
        edge_density: edge_signature.edge_density,
        foreground_scale: NORMALIZED_FOREGROUND_EXTENT,
        perceptual_hash: format!("{:016x}", foreground_signature.perceptual_hash),
    })
}

fn remove_dominant_border_background(image: &RgbaImage) -> Option<RgbaImage> {
    if image.width() == 0 || image.height() == 0 {
        return None;
    }
    let band = (image.width().min(image.height()) / 16).clamp(1, 64);
    let mut bins = HashMap::<[u8; 3], (u64, [u64; 3])>::new();
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] <= 16
            || (x >= band && y >= band && x + band < image.width() && y + band < image.height())
        {
            continue;
        }
        let bin = [pixel[0] & 0xE0, pixel[1] & 0xE0, pixel[2] & 0xE0];
        let entry = bins.entry(bin).or_insert((0, [0; 3]));
        entry.0 += 1;
        entry.1[0] += pixel[0] as u64;
        entry.1[1] += pixel[1] as u64;
        entry.1[2] += pixel[2] as u64;
    }
    let (_, (count, sums)) = bins.into_iter().max_by_key(|(_, value)| value.0)?;
    if count == 0 {
        return None;
    }
    let key = [
        (sums[0] / count) as u8,
        (sums[1] / count) as u8,
        (sums[2] / count) as u8,
    ];
    let width = image.width() as usize;
    let height = image.height() as usize;
    let mut output = image.clone();
    let mut visited = vec![false; width * height];
    let mut queue = VecDeque::new();
    let is_background = |pixel: &Rgba<u8>| {
        if pixel[3] <= 16 {
            return true;
        }
        let red = pixel[0] as i32 - key[0] as i32;
        let green = pixel[1] as i32 - key[1] as i32;
        let blue = pixel[2] as i32 - key[2] as i32;
        red * red + green * green + blue * blue <= 64 * 64
    };
    let enqueue = |x: usize, y: usize, visited: &mut [bool], queue: &mut VecDeque<usize>| {
        let index = y * width + x;
        if !visited[index] && is_background(image.get_pixel(x as u32, y as u32)) {
            visited[index] = true;
            queue.push_back(index);
        }
    };
    for x in 0..width {
        enqueue(x, 0, &mut visited, &mut queue);
        if height > 1 {
            enqueue(x, height - 1, &mut visited, &mut queue);
        }
    }
    for y in 0..height {
        enqueue(0, y, &mut visited, &mut queue);
        if width > 1 {
            enqueue(width - 1, y, &mut visited, &mut queue);
        }
    }
    while let Some(index) = queue.pop_front() {
        let x = index % width;
        let y = index / width;
        output.get_pixel_mut(x as u32, y as u32)[3] = 0;
        for (neighbor_x, neighbor_y) in [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ] {
            if neighbor_x < width && neighbor_y < height {
                enqueue(neighbor_x, neighbor_y, &mut visited, &mut queue);
            }
        }
    }
    Some(output)
}

fn alpha_coverage(image: &RgbaImage) -> f32 {
    let total = image.width() as u64 * image.height() as u64;
    if total == 0 {
        return 0.0;
    }
    image.pixels().filter(|pixel| pixel[3] > 16).count() as f32 / total as f32
}

fn palette(image: &RgbaImage) -> Vec<PaletteColor> {
    let mut counts = HashMap::<[u8; 3], u64>::new();
    let mut total = 0u64;
    for pixel in image.pixels() {
        if pixel[3] <= 16 {
            continue;
        }
        let key = [pixel[0] & 0xE0, pixel[1] & 0xE0, pixel[2] & 0xE0];
        *counts.entry(key).or_default() += 1;
        total += 1;
    }
    let mut values = counts.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    values
        .into_iter()
        .take(PALETTE_COLOR_LIMIT)
        .map(|(color, count)| PaletteColor {
            color: format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]),
            weight: if total == 0 {
                0.0
            } else {
                count as f32 / total as f32
            },
        })
        .collect()
}

fn major_subject_count(image: &RgbaImage) -> u32 {
    let width = image.width() as usize;
    let height = image.height() as usize;
    if width == 0 || height == 0 {
        return 0;
    }
    let foreground = image
        .pixels()
        .map(|pixel| pixel[3] > 16)
        .collect::<Vec<_>>();
    let foreground_pixels = foreground.iter().filter(|pixel| **pixel).count();
    if foreground_pixels == 0 {
        return 0;
    }
    let minimum_major_area = (foreground_pixels / 10).max(16);
    let mut visited = vec![false; foreground.len()];
    let mut major_components = 0u32;
    for start in 0..foreground.len() {
        if !foreground[start] || visited[start] {
            continue;
        }
        visited[start] = true;
        let mut queue = VecDeque::from([start]);
        let mut area = 0usize;
        while let Some(index) = queue.pop_front() {
            area += 1;
            let x = index % width;
            let y = index / width;
            let min_x = x.saturating_sub(1);
            let max_x = (x + 1).min(width - 1);
            let min_y = y.saturating_sub(1);
            let max_y = (y + 1).min(height - 1);
            for neighbor_y in min_y..=max_y {
                for neighbor_x in min_x..=max_x {
                    let neighbor = neighbor_y * width + neighbor_x;
                    if foreground[neighbor] && !visited[neighbor] {
                        visited[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
        }
        if area >= minimum_major_area {
            major_components += 1;
        }
    }
    major_components
}

pub(crate) fn palette_overlap(left: &[PaletteColor], right: &[PaletteColor]) -> f32 {
    let left = normalized_palette(left);
    let right = normalized_palette(right);
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    left.iter()
        .map(|(left_color, left_weight)| {
            let support = right
                .iter()
                .map(|(right_color, _)| {
                    let distance_squared = left_color
                        .iter()
                        .zip(right_color)
                        .map(|(left, right)| {
                            let delta = *left as f32 - *right as f32;
                            delta * delta
                        })
                        .sum::<f32>();
                    (-distance_squared / (2.0 * PALETTE_SIMILARITY_SIGMA.powi(2))).exp()
                })
                .fold(0.0, f32::max);
            left_weight * support
        })
        .sum::<f32>()
        .clamp(0.0, 1.0)
}

fn normalized_palette(palette: &[PaletteColor]) -> Vec<([u8; 3], f32)> {
    let parsed = palette
        .iter()
        .filter(|color| color.weight.is_finite() && color.weight > 0.0)
        .filter_map(|color| parse_palette_color(&color.color).map(|rgb| (rgb, color.weight)))
        .collect::<Vec<_>>();
    let total = parsed.iter().map(|(_, weight)| *weight).sum::<f32>();
    if total <= f32::EPSILON {
        return Vec::new();
    }
    parsed
        .into_iter()
        .map(|(color, weight)| (color, weight / total))
        .collect()
}

fn parse_palette_color(color: &str) -> Option<[u8; 3]> {
    let value = color.strip_prefix('#').unwrap_or(color);
    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ])
}

fn edge_density(image: &RgbaImage) -> f32 {
    if image.width() < 2 || image.height() < 2 {
        return 0.0;
    }
    let mut edges = 0u64;
    let mut samples = 0u64;
    for y in 0..image.height() - 1 {
        for x in 0..image.width() - 1 {
            let pixel = image.get_pixel(x, y);
            if pixel[3] <= 16 {
                continue;
            }
            let right = image.get_pixel(x + 1, y);
            let below = image.get_pixel(x, y + 1);
            let delta = color_delta(pixel, right).max(color_delta(pixel, below));
            edges += u64::from(delta > 72);
            samples += 1;
        }
    }
    if samples == 0 {
        0.0
    } else {
        edges as f32 / (samples as f32).sqrt()
    }
}

fn color_delta(left: &Rgba<u8>, right: &Rgba<u8>) -> u16 {
    left[0].abs_diff(right[0]) as u16
        + left[1].abs_diff(right[1]) as u16
        + left[2].abs_diff(right[2]) as u16
}

fn perceptual_hash(image: &RgbaImage) -> u64 {
    let gray = DynamicImage::ImageRgba8(image.clone())
        .resize_exact(9, 8, FilterType::Triangle)
        .to_luma8();
    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            if gray.get_pixel(x, y)[0] > gray.get_pixel(x + 1, y)[0] {
                hash |= 1 << (y * 8 + x);
            }
        }
    }
    hash
}

fn alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > 16 {
            found = true;
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    found.then_some((left, top, right, bottom))
}

fn ratio(value: f32, baseline: f32) -> f32 {
    if baseline <= f32::EPSILON {
        if value <= f32::EPSILON {
            1.0
        } else {
            2.0
        }
    } else {
        value / baseline
    }
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), AssetProjectError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn default_profile_id() -> String {
    "default".into()
}
fn default_perspective() -> String {
    "topdown".into()
}
fn default_lighting() -> String {
    "upper_left".into()
}
fn default_outline() -> String {
    "dark".into()
}
fn default_background() -> String {
    "transparent".into()
}
fn default_sampling() -> SamplingMode {
    SamplingMode::Nearest
}
fn default_character_canvas() -> u32 {
    256
}
fn default_icon_canvas() -> u32 {
    128
}
fn default_prop_canvas() -> u32 {
    256
}
fn default_license() -> String {
    "private".into()
}
fn default_subject_count() -> u32 {
    1
}
fn legacy_style_baseline_profile() -> String {
    "style-baseline@1.0.0".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consistency_thresholds_distinguish_pass_review_and_regenerate() {
        let style = StyleLockV1 {
            schema_version: "1".into(),
            revision: "style-1".into(),
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            image_model: None,
            prompt: "pixel art".into(),
            perspective: "topdown".into(),
            lighting: "upper_left".into(),
            outline: "dark".into(),
            background: "transparent".into(),
            sampling: SamplingMode::Nearest,
            character_canvas_size: 256,
            icon_canvas_size: 128,
            prop_canvas_size: 256,
            board_path: PathBuf::from("board.png"),
            board_sha256: "hash".into(),
            reference_sha256: vec![],
            baseline_profile: STYLE_BASELINE_PROFILE.into(),
            migrated_from_revision: None,
            baseline: StyleBaseline {
                palette: vec![PaletteColor {
                    color: "#202020".into(),
                    weight: 0.8,
                }],
                edge_density: 0.2,
                foreground_scale: 0.5,
                perceptual_hash: "0".into(),
            },
        };
        let signature = ImageSignature {
            palette: vec![PaletteColor {
                color: "#202020".into(),
                weight: 0.8,
            }],
            edge_density: 0.2,
            foreground_scale: NORMALIZED_FOREGROUND_EXTENT,
            perceptual_hash: 0,
            anchor_x: 64.0,
            anchor_y: 110.0,
            width: 128,
            height: 128,
            alpha_present: true,
            cell_boundary_safe: true,
            subject_count: 1,
        };
        let report = assess_consistency("icon", 1, &signature, &style, None, None, 128);
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
    }

    #[test]
    fn foreground_scale_uses_longest_extent_instead_of_shape_area() {
        let mut image = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 12..117 {
            for x in 54..74 {
                image.put_pixel(x, y, Rgba([40, 120, 80, 255]));
            }
        }
        let signature = image_signature(&image);
        assert!((signature.foreground_scale - 105.0 / 128.0).abs() < 0.001);
    }

    #[test]
    fn palette_keeps_legitimate_green_foreground() {
        let image = RgbaImage::from_pixel(16, 16, Rgba([0, 192, 64, 255]));
        let palette = palette(&image);
        assert_eq!(palette[0].color, "#00C040");
        assert_eq!(palette[0].weight, 1.0);
    }

    #[test]
    fn style_signature_excludes_dominant_opaque_background() {
        let mut image = RgbaImage::from_pixel(256, 256, Rgba([104, 132, 108, 255]));
        for y in 72..184 {
            for x in 40..96 {
                image.put_pixel(x, y, Rgba([16, 32, 56, 255]));
            }
            for x in 108..164 {
                image.put_pixel(x, y, Rgba([32, 144, 160, 255]));
            }
            for x in 176..232 {
                image.put_pixel(x, y, Rgba([176, 96, 48, 255]));
            }
        }
        for x in (12..244).step_by(40) {
            image.put_pixel(x, 4, Rgba([24, 48, 72, 255]));
        }

        let foreground = style_signature_image(&image).unwrap();
        let signature = image_signature(&foreground);

        assert!(alpha_coverage(&foreground) < 0.40);
        assert!(signature
            .palette
            .iter()
            .all(|color| color.color != "#608060"));
        assert!(signature
            .palette
            .iter()
            .any(|color| color.color == "#2080A0"));
    }

    #[test]
    fn legacy_style_lock_defaults_to_legacy_baseline_profile() {
        let lock: StyleLockV1 = serde_json::from_value(serde_json::json!({
            "schemaVersion": "1",
            "revision": "legacy",
            "providerId": "fixture",
            "profileId": "default",
            "prompt": "pixel art",
            "perspective": "topdown",
            "lighting": "upper_left",
            "outline": "dark",
            "background": "transparent",
            "sampling": "nearest",
            "characterCanvasSize": 256,
            "iconCanvasSize": 128,
            "propCanvasSize": 256,
            "boardPath": "board.png",
            "boardSha256": "hash",
            "referenceSha256": [],
            "baseline": {
                "palette": [{ "color": "#202020", "weight": 1.0 }],
                "edgeDensity": 1.0,
                "foregroundScale": 1.0,
                "perceptualHash": "0"
            }
        }))
        .unwrap();

        assert_eq!(lock.baseline_profile, "style-baseline@1.0.0");
        assert_eq!(lock.migrated_from_revision, None);
    }

    #[test]
    fn palette_overlap_accepts_near_colors_and_rejects_distant_colors() {
        let style = vec![
            PaletteColor {
                color: "#406040".into(),
                weight: 0.7,
            },
            PaletteColor {
                color: "#604020".into(),
                weight: 0.3,
            },
        ];
        let near = vec![
            PaletteColor {
                color: "#408060".into(),
                weight: 0.7,
            },
            PaletteColor {
                color: "#806040".into(),
                weight: 0.3,
            },
        ];
        let distant = vec![PaletteColor {
            color: "#E000E0".into(),
            weight: 1.0,
        }];
        assert!(palette_overlap(&near, &style) >= 0.70);
        assert!(palette_overlap(&distant, &style) < 0.55);
    }

    #[test]
    fn consistency_blocks_multiple_major_subjects_but_ignores_tiny_specks() {
        let mut image = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for &(left, top) in &[(12, 12), (76, 12), (12, 76), (76, 76)] {
            for y in top..top + 28 {
                for x in left..left + 28 {
                    image.put_pixel(x, y, Rgba([40, 120, 80, 255]));
                }
            }
        }
        image.put_pixel(63, 63, Rgba([255, 255, 255, 255]));
        let signature = image_signature(&image);
        assert_eq!(signature.subject_count, 4);
        let style = StyleLockV1 {
            schema_version: "1".into(),
            revision: "style-1".into(),
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            image_model: None,
            prompt: "pixel art".into(),
            perspective: "topdown".into(),
            lighting: "upper_left".into(),
            outline: "dark".into(),
            background: "transparent".into(),
            sampling: SamplingMode::Nearest,
            character_canvas_size: 128,
            icon_canvas_size: 128,
            prop_canvas_size: 128,
            board_path: PathBuf::from("board.png"),
            board_sha256: "hash".into(),
            reference_sha256: vec![],
            baseline_profile: STYLE_BASELINE_PROFILE.into(),
            migrated_from_revision: None,
            baseline: StyleBaseline {
                palette: signature.palette.clone(),
                edge_density: signature.edge_density,
                foreground_scale: NORMALIZED_FOREGROUND_EXTENT,
                perceptual_hash: "0".into(),
            },
        };
        let report = assess_consistency("walk_right", 1, &signature, &style, None, None, 128);
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.reasons.contains(&"multiple_subjects".into()));
    }

    #[test]
    fn keyframe_hard_gates_block_background_pose_leak_noise_and_extreme_silhouette() {
        let mut identity = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 24..40 {
            for x in 52..76 {
                identity.put_pixel(x, y, Rgba([40, 90, 150, 255]));
            }
        }
        for y in 40..88 {
            for x in 48..80 {
                identity.put_pixel(x, y, Rgba([40, 90, 150, 255]));
            }
        }
        for y in 50..70 {
            for x in 40..88 {
                identity.put_pixel(x, y, Rgba([40, 90, 150, 255]));
            }
        }
        for y in 88..112 {
            for x in 48..60 {
                identity.put_pixel(x, y, Rgba([40, 90, 150, 255]));
            }
            for x in 68..80 {
                identity.put_pixel(x, y, Rgba([40, 90, 150, 255]));
            }
        }
        let mut pose = RgbaImage::from_pixel(128, 128, Rgba([255, 255, 255, 255]));
        for y in 24..110 {
            for x in 61..67 {
                pose.put_pixel(x, y, Rgba([145, 65, 200, 255]));
            }
        }
        for y in 48..55 {
            for x in 24..104 {
                pose.put_pixel(x, y, Rgba([145, 65, 200, 255]));
            }
        }
        let style = StyleLockV1 {
            schema_version: "1".into(),
            revision: "hard-gate-style".into(),
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            image_model: Some("fixture-image".into()),
            prompt: "test".into(),
            perspective: "topdown".into(),
            lighting: "soft".into(),
            outline: "clean".into(),
            background: "transparent".into(),
            sampling: SamplingMode::Nearest,
            character_canvas_size: 128,
            icon_canvas_size: 128,
            prop_canvas_size: 128,
            board_path: PathBuf::from("board.png"),
            board_sha256: "hash".into(),
            reference_sha256: vec![],
            baseline_profile: STYLE_BASELINE_PROFILE.into(),
            migrated_from_revision: None,
            baseline: StyleBaseline {
                palette: image_signature(&identity).palette,
                edge_density: image_signature(&identity).edge_density,
                foreground_scale: NORMALIZED_FOREGROUND_EXTENT,
                perceptual_hash: "0".into(),
            },
        };
        let identity_signature = image_signature(&identity);
        let base_report = |candidate: &RgbaImage| {
            assess_consistency(
                "idle/frame-00",
                1,
                &image_signature(candidate),
                &style,
                Some(&identity_signature),
                Some(&identity_signature),
                128,
            )
        };

        let mut clean_report = base_report(&identity);
        apply_keyframe_hard_defects(&identity, &identity, &pose, &mut clean_report);
        assert!(!clean_report
            .reasons
            .iter()
            .any(|reason| reason.contains("background") || reason.contains("leak")));

        let mut background = identity.clone();
        for y in 12..116 {
            for x in 16..112 {
                background.put_pixel(x, y, Rgba([112, 112, 112, 255]));
            }
        }
        let mut background_report = base_report(&background);
        apply_keyframe_hard_defects(&background, &identity, &pose, &mut background_report);
        assert_eq!(background_report.verdict, ConsistencyVerdict::Blocked);
        assert!(background_report
            .reasons
            .contains(&"opaque_background_residual".into()));

        let mut noisy = identity.clone();
        for index in 0..96_u32 {
            let x = 2 + (index * 17) % 124;
            let y = 2 + (index * 29) % 124;
            if noisy.get_pixel(x, y)[3] == 0 {
                noisy.put_pixel(x, y, Rgba([255, 255, 255, 8]));
            }
        }
        let mut noisy_report = base_report(&noisy);
        apply_keyframe_hard_defects(&noisy, &identity, &pose, &mut noisy_report);
        assert_eq!(noisy_report.verdict, ConsistencyVerdict::Blocked);
        assert!(noisy_report.reasons.contains(&"low_alpha_noise".into()));

        let mut leaked_pose = identity.clone();
        for y in 48..56 {
            for x in 4..124 {
                leaked_pose.put_pixel(x, y, Rgba([145, 65, 200, 255]));
            }
        }
        let mut pose_report = base_report(&leaked_pose);
        apply_keyframe_hard_defects(&leaked_pose, &identity, &pose, &mut pose_report);
        assert_eq!(pose_report.verdict, ConsistencyVerdict::Blocked);
        assert!(pose_report.reasons.contains(&"pose_structure_leak".into()));

        let mut extreme = identity.clone();
        for y in 70..78 {
            for x in 1..127 {
                extreme.put_pixel(x, y, Rgba([20, 180, 80, 255]));
            }
        }
        let mut extreme_report = base_report(&extreme);
        apply_keyframe_hard_defects(&extreme, &identity, &pose, &mut extreme_report);
        assert_eq!(extreme_report.verdict, ConsistencyVerdict::Blocked);
        assert!(extreme_report
            .reasons
            .contains(&"silhouette_aspect_drift".into()));
    }

    #[test]
    fn project_init_is_non_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let first = init_project(temp.path(), "Forest Game").unwrap();
        assert_eq!(first.project_id, "forest-game");
        assert!(init_project(temp.path(), "Other").is_err());
    }
}
