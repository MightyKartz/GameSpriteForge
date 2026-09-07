use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use crate::asset_project::{hash_file, ConsistencyVerdict};
use crate::geometry::PortraitFramingProfileV1;
use crate::provider::ReferenceRole;

pub const PORTRAIT_BASE_LOCK_FILE: &str = "portrait-base-lock.json";
pub const PORTRAIT_BASE_LOCK_PROFILE: &str = "portrait-base-lock@1.0.0";
pub const PORTRAIT_CONSISTENCY_REPORT_FILE: &str = "portrait-consistency-report.json";
pub const PORTRAIT_LOCAL_PROFILE: &str = "portrait-local@1.1.0";
pub const PORTRAIT_BASE_APPROVAL_FILE: &str = "portrait-base-approval.json";
pub const PORTRAIT_BASE_APPROVAL_PROFILE: &str = "portrait-base-approval@1.0.0";

const ALPHA_THRESHOLD: u8 = 32;
const PIXEL_CHANGE_THRESHOLD: u8 = 12;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortraitGenerationPhaseV1 {
    #[default]
    LegacyAll,
    BaseOnly,
    Expressions,
}

impl PortraitGenerationPhaseV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LegacyAll => "legacy_all",
            Self::BaseOnly => "base_only",
            Self::Expressions => "expressions",
        }
    }
}

pub fn is_legacy_portrait_phase(value: &PortraitGenerationPhaseV1) -> bool {
    *value == PortraitGenerationPhaseV1::LegacyAll
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortraitNeutralReferencePolicyV1 {
    #[default]
    #[serde(rename = "legacy-three-reference@1.0.0")]
    LegacyThreeReference,
    #[serde(rename = "subject-style@1.0.0")]
    SubjectStyle,
    #[serde(rename = "subject-edit@1.0.0")]
    SubjectEdit,
}

impl PortraitNeutralReferencePolicyV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LegacyThreeReference => "legacy-three-reference@1.0.0",
            Self::SubjectStyle => "subject-style@1.0.0",
            Self::SubjectEdit => "subject-edit@1.0.0",
        }
    }
}

pub fn is_legacy_portrait_reference_policy(value: &PortraitNeutralReferencePolicyV1) -> bool {
    *value == PortraitNeutralReferencePolicyV1::LegacyThreeReference
}

#[derive(Debug, thiserror::Error)]
pub enum PortraitError {
    #[error("invalid portrait: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("asset error: {0}")]
    Asset(#[from] crate::asset_project::AssetProjectError),
}

impl PortraitError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "invalid_portrait",
            Self::Io(_) => "portrait_io_error",
            Self::Json(_) => "portrait_invalid_json",
            Self::Image(_) => "portrait_invalid_image",
            Self::Asset(_) => "portrait_asset_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitFaceScopeV1 {
    pub canvas_width: u32,
    pub canvas_height: u32,
    pub center_x: u32,
    pub center_y: u32,
    pub radius_x: u32,
    pub radius_y: u32,
    pub feather_px: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortraitBaseProvenanceV1 {
    pub asset_id: String,
    pub provider_id: String,
    pub profile_id: String,
    pub model: Option<String>,
    pub style_revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub collection_id: String,
    pub collection_revision: String,
    pub framing_profile: PortraitFramingProfileV1,
    pub neutral_reference_policy: PortraitNeutralReferencePolicyV1,
    pub neutral_reference_roles: Vec<ReferenceRole>,
    pub neutral_reference_sha256: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitBaseLockV1 {
    pub schema_version: String,
    pub profile: String,
    pub asset_id: String,
    pub item_id: String,
    pub image_path: PathBuf,
    pub image_sha256: String,
    pub provider_edit_source_path: PathBuf,
    pub provider_edit_source_sha256: String,
    pub provider_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub style_revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub collection_id: String,
    pub collection_revision: String,
    pub framing_profile: PortraitFramingProfileV1,
    #[serde(default)]
    pub neutral_reference_policy: PortraitNeutralReferencePolicyV1,
    #[serde(default)]
    pub neutral_reference_roles: Vec<ReferenceRole>,
    #[serde(default)]
    pub neutral_reference_sha256: Vec<String>,
    pub face_scope: PortraitFaceScopeV1,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitLocalMetricsV1 {
    pub raw_outside_face_changed_ratio: f32,
    pub output_outside_face_changed_ratio: f32,
    pub face_changed_ratio: f32,
    pub skin_tone_delta_e: f32,
    pub skin_sample_count: u32,
    #[serde(default)]
    pub skin_correspondence_ratio: f32,
    pub protected_skin_artifact_ratio: f32,
    pub protected_skin_severe_artifact_ratio: f32,
    pub protected_skin_sample_count: u32,
    pub face_edge_density_ratio: f32,
    pub face_perceptual_similarity: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitLocalItemReportV1 {
    pub id: String,
    pub verdict: ConsistencyVerdict,
    pub metrics: PortraitLocalMetricsV1,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitConsistencyReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub base_item_id: String,
    pub base_sha256: String,
    pub face_scope: PortraitFaceScopeV1,
    pub verdict: ConsistencyVerdict,
    pub items: Vec<PortraitLocalItemReportV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitBaseApprovalV1 {
    pub schema_version: String,
    pub profile: String,
    pub source_job_id: String,
    pub asset_id: String,
    pub base_lock_sha256: String,
    pub neutral_sha256: String,
    pub neutral_reference_policy: PortraitNeutralReferencePolicyV1,
    pub accepted: bool,
    pub reason: String,
    pub reviewed_at: DateTime<Utc>,
}

pub fn materialize_portrait_base_lock(
    job_dir: &Path,
    normalized_base: &Path,
    provider_edit_source: Option<&Path>,
    provenance: PortraitBaseProvenanceV1,
) -> Result<PortraitBaseLockV1, PortraitError> {
    let base = image::open(normalized_base)?.to_rgba8();
    if base.width() == 0 || base.height() == 0 {
        return Err(PortraitError::Invalid(
            "neutral portrait base has an empty canvas".into(),
        ));
    }
    let face_scope = derive_face_scope(&base, provenance.framing_profile)?;
    let root = job_dir.join("portrait-base");
    fs::create_dir_all(&root)?;
    let image_path = root.join("neutral.png");
    fs::copy(normalized_base, &image_path)?;
    let provider_edit_source = provider_edit_source.unwrap_or(normalized_base);
    let provider_edit_source_path = root.join("provider-edit-source.png");
    fs::copy(provider_edit_source, &provider_edit_source_path)?;
    let lock = PortraitBaseLockV1 {
        schema_version: "1".into(),
        profile: PORTRAIT_BASE_LOCK_PROFILE.into(),
        asset_id: provenance.asset_id,
        item_id: "neutral".into(),
        image_sha256: hash_file(&image_path)?,
        image_path,
        provider_edit_source_sha256: hash_file(&provider_edit_source_path)?,
        provider_edit_source_path,
        provider_id: provenance.provider_id,
        profile_id: provenance.profile_id,
        model: provenance.model,
        style_revision: provenance.style_revision,
        subject_id: provenance.subject_id,
        subject_revision: provenance.subject_revision,
        collection_id: provenance.collection_id,
        collection_revision: provenance.collection_revision,
        framing_profile: provenance.framing_profile,
        neutral_reference_policy: provenance.neutral_reference_policy,
        neutral_reference_roles: provenance.neutral_reference_roles,
        neutral_reference_sha256: provenance.neutral_reference_sha256,
        face_scope,
        created_at: Utc::now(),
    };
    write_json_atomic(&root.join(PORTRAIT_BASE_LOCK_FILE), &lock)?;
    Ok(lock)
}

pub fn read_portrait_base_lock(path: &Path) -> Result<PortraitBaseLockV1, PortraitError> {
    let lock: PortraitBaseLockV1 = serde_json::from_slice(&fs::read(path)?)?;
    if lock.schema_version != "1" || lock.profile != PORTRAIT_BASE_LOCK_PROFILE {
        return Err(PortraitError::Invalid(
            "unsupported PortraitBaseLock profile".into(),
        ));
    }
    if !lock.image_path.is_file()
        || hash_file(&lock.image_path)? != lock.image_sha256
        || !lock.provider_edit_source_path.is_file()
        || hash_file(&lock.provider_edit_source_path)? != lock.provider_edit_source_sha256
    {
        return Err(PortraitError::Invalid(
            "PortraitBaseLock image closure changed after locking".into(),
        ));
    }
    Ok(lock)
}

pub fn approve_portrait_base(
    source_job_id: &str,
    job_dir: &Path,
    reason: &str,
    reviewed_at: DateTime<Utc>,
) -> Result<(PathBuf, PortraitBaseApprovalV1), PortraitError> {
    if source_job_id.trim().is_empty() || reason.trim().is_empty() {
        return Err(PortraitError::Invalid(
            "PortraitBase approval requires a Job ID and non-empty reason".into(),
        ));
    }
    let base_root = job_dir.join("portrait-base");
    let lock_path = base_root.join(PORTRAIT_BASE_LOCK_FILE);
    let lock = read_portrait_base_lock(&lock_path)?;
    let approval = PortraitBaseApprovalV1 {
        schema_version: "1".into(),
        profile: PORTRAIT_BASE_APPROVAL_PROFILE.into(),
        source_job_id: source_job_id.into(),
        asset_id: lock.asset_id.clone(),
        base_lock_sha256: hash_file(&lock_path)?,
        neutral_sha256: lock.image_sha256.clone(),
        neutral_reference_policy: lock.neutral_reference_policy,
        accepted: true,
        reason: reason.trim().into(),
        reviewed_at,
    };
    let path = base_root.join(PORTRAIT_BASE_APPROVAL_FILE);
    write_json_atomic(&path, &approval)?;
    Ok((path, approval))
}

pub fn read_portrait_base_approval(path: &Path) -> Result<PortraitBaseApprovalV1, PortraitError> {
    let approval: PortraitBaseApprovalV1 = serde_json::from_slice(&fs::read(path)?)?;
    if approval.schema_version != "1"
        || approval.profile != PORTRAIT_BASE_APPROVAL_PROFILE
        || !approval.accepted
        || approval.source_job_id.trim().is_empty()
        || approval.asset_id.trim().is_empty()
        || approval.reason.trim().is_empty()
    {
        return Err(PortraitError::Invalid(
            "PortraitBase approval is unsupported, incomplete, or rejected".into(),
        ));
    }
    Ok(approval)
}

pub fn validate_portrait_base_approval(
    job_dir: &Path,
    expected_job_id: &str,
) -> Result<(PortraitBaseLockV1, PortraitBaseApprovalV1), PortraitError> {
    let base_root = job_dir.join("portrait-base");
    let lock_path = base_root.join(PORTRAIT_BASE_LOCK_FILE);
    let approval_path = base_root.join(PORTRAIT_BASE_APPROVAL_FILE);
    let lock = read_portrait_base_lock(&lock_path)?;
    let approval = read_portrait_base_approval(&approval_path)?;
    if approval.source_job_id != expected_job_id
        || approval.asset_id != lock.asset_id
        || approval.base_lock_sha256 != hash_file(&lock_path)?
        || approval.neutral_sha256 != lock.image_sha256
        || approval.neutral_reference_policy != lock.neutral_reference_policy
    {
        return Err(PortraitError::Invalid(
            "PortraitBase approval no longer matches the immutable Lock closure".into(),
        ));
    }
    Ok((lock, approval))
}

pub fn base_item_report(
    verdict: ConsistencyVerdict,
    mut reasons: Vec<String>,
) -> PortraitLocalItemReportV1 {
    if !reasons.iter().any(|reason| reason == "portrait_base") {
        reasons.push("portrait_base".into());
    }
    PortraitLocalItemReportV1 {
        id: "neutral".into(),
        verdict,
        metrics: PortraitLocalMetricsV1 {
            raw_outside_face_changed_ratio: 0.0,
            output_outside_face_changed_ratio: 0.0,
            face_changed_ratio: 0.0,
            skin_tone_delta_e: 0.0,
            skin_sample_count: 0,
            skin_correspondence_ratio: 1.0,
            protected_skin_artifact_ratio: 0.0,
            protected_skin_severe_artifact_ratio: 0.0,
            protected_skin_sample_count: 0,
            face_edge_density_ratio: 1.0,
            face_perceptual_similarity: 1.0,
        },
        reasons,
    }
}

pub fn composite_and_assess_expression(
    id: &str,
    base: &RgbaImage,
    candidate: &RgbaImage,
    scope: &PortraitFaceScopeV1,
) -> Result<(RgbaImage, PortraitLocalItemReportV1), PortraitError> {
    validate_scope(base, candidate, scope)?;
    let raw_outside_face_changed_ratio =
        changed_ratio(base, candidate, scope, PixelRegion::OutsideFace, false);
    let face_changed_ratio = changed_ratio(base, candidate, scope, PixelRegion::Face, false);
    let output = composite_face(base, candidate, scope);
    let output_outside_face_changed_ratio =
        changed_ratio(base, &output, scope, PixelRegion::OutsideFace, true);
    let (skin_tone_delta_e, skin_sample_count, skin_correspondence_ratio) =
        skin_tone_delta_e(id, base, candidate, scope);
    let (
        protected_skin_artifact_ratio,
        protected_skin_severe_artifact_ratio,
        protected_skin_sample_count,
    ) = protected_skin_artifacts(id, base, candidate, scope);
    let base_edges = face_edge_density(base, scope);
    let candidate_edges = face_edge_density(candidate, scope);
    let face_edge_density_ratio = if base_edges <= f32::EPSILON {
        if candidate_edges <= f32::EPSILON {
            1.0
        } else {
            2.0
        }
    } else {
        candidate_edges / base_edges
    };
    let face_perceptual_similarity = face_similarity(base, candidate, scope);
    let metrics = PortraitLocalMetricsV1 {
        raw_outside_face_changed_ratio,
        output_outside_face_changed_ratio,
        face_changed_ratio,
        skin_tone_delta_e,
        skin_sample_count,
        skin_correspondence_ratio,
        protected_skin_artifact_ratio,
        protected_skin_severe_artifact_ratio,
        protected_skin_sample_count,
        face_edge_density_ratio,
        face_perceptual_similarity,
    };
    let (verdict, reasons) = assess_local_metrics(&metrics);
    Ok((
        output,
        PortraitLocalItemReportV1 {
            id: id.into(),
            verdict,
            metrics,
            reasons,
        },
    ))
}

pub fn aggregate_portrait_report(
    lock: &PortraitBaseLockV1,
    mut items: Vec<PortraitLocalItemReportV1>,
) -> PortraitConsistencyReportV1 {
    items.sort_by_key(|item| expression_order(&item.id));
    let verdict = items
        .iter()
        .fold(ConsistencyVerdict::GameReady, |current, item| {
            worse_verdict(current, item.verdict)
        });
    PortraitConsistencyReportV1 {
        schema_version: "1".into(),
        profile: PORTRAIT_LOCAL_PROFILE.into(),
        base_item_id: lock.item_id.clone(),
        base_sha256: lock.image_sha256.clone(),
        face_scope: lock.face_scope.clone(),
        verdict,
        items,
    }
}

fn derive_face_scope(
    image: &RgbaImage,
    framing: PortraitFramingProfileV1,
) -> Result<PortraitFaceScopeV1, PortraitError> {
    let (left, top, right, bottom) = alpha_bounds(image)
        .ok_or_else(|| PortraitError::Invalid("neutral portrait has no foreground".into()))?;
    let width = right - left + 1;
    let height = bottom - top + 1;
    let (center_y_percent, radius_x_percent, radius_y_percent) = match framing {
        PortraitFramingProfileV1::FullBody => (27, 19, 15),
        PortraitFramingProfileV1::DialogueBust => (33, 23, 22),
    };
    let center_x = left + width / 2;
    let center_y = top + height * center_y_percent / 100;
    let radius_x = (width * radius_x_percent / 100).max(4);
    let radius_y = (height * radius_y_percent / 100).max(4);
    let feather_px = (radius_x.min(radius_y) / 5).max(2);
    Ok(PortraitFaceScopeV1 {
        canvas_width: image.width(),
        canvas_height: image.height(),
        center_x,
        center_y,
        radius_x,
        radius_y,
        feather_px,
    })
}

fn validate_scope(
    base: &RgbaImage,
    candidate: &RgbaImage,
    scope: &PortraitFaceScopeV1,
) -> Result<(), PortraitError> {
    if base.dimensions() != candidate.dimensions()
        || base.width() != scope.canvas_width
        || base.height() != scope.canvas_height
        || scope.radius_x == 0
        || scope.radius_y == 0
    {
        return Err(PortraitError::Invalid(
            "portrait base, candidate, and face scope canvas mismatch".into(),
        ));
    }
    Ok(())
}

fn composite_face(
    base: &RgbaImage,
    candidate: &RgbaImage,
    scope: &PortraitFaceScopeV1,
) -> RgbaImage {
    let mut output = base.clone();
    for y in 0..base.height() {
        for x in 0..base.width() {
            let weight = face_weight(scope, x, y);
            if weight <= 0.0 {
                continue;
            }
            let source = candidate.get_pixel(x, y);
            let target = base.get_pixel(x, y);
            let mut blended = [0u8; 4];
            for channel in 0..4 {
                blended[channel] = ((source[channel] as f32 * weight)
                    + (target[channel] as f32 * (1.0 - weight)))
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
            output.put_pixel(x, y, Rgba(blended));
        }
    }
    output
}

#[derive(Clone, Copy)]
enum PixelRegion {
    Face,
    OutsideFace,
}

fn changed_ratio(
    base: &RgbaImage,
    candidate: &RgbaImage,
    scope: &PortraitFaceScopeV1,
    region: PixelRegion,
    exact: bool,
) -> f32 {
    let mut compared = 0usize;
    let mut changed = 0usize;
    for y in 0..base.height() {
        for x in 0..base.width() {
            let inside = face_weight(scope, x, y) > 0.0;
            if matches!(region, PixelRegion::Face) != inside {
                continue;
            }
            let left = base.get_pixel(x, y);
            let right = candidate.get_pixel(x, y);
            if left[3] <= ALPHA_THRESHOLD && right[3] <= ALPHA_THRESHOLD {
                continue;
            }
            compared += 1;
            let pixel_changed = if exact {
                left != right
            } else {
                (0..4)
                    .any(|channel| left[channel].abs_diff(right[channel]) > PIXEL_CHANGE_THRESHOLD)
            };
            changed += usize::from(pixel_changed);
        }
    }
    changed as f32 / compared.max(1) as f32
}

fn skin_tone_delta_e(
    expression_id: &str,
    base: &RgbaImage,
    candidate: &RgbaImage,
    scope: &PortraitFaceScopeV1,
) -> (f32, u32, f32) {
    let mut deltas = Vec::new();
    let mut eligible_source_skin = 0u32;
    for y in 0..base.height() {
        for x in 0..base.width() {
            if face_weight(scope, x, y) < 0.55
                || is_expression_feature_zone(expression_id, scope, x, y)
            {
                continue;
            }
            let source = base.get_pixel(x, y);
            let target = candidate.get_pixel(x, y);
            if !is_probable_skin(source) {
                continue;
            }
            eligible_source_skin += 1;
            if !is_probable_skin(target) {
                continue;
            }
            deltas.push(cie76(rgb_to_lab(source), rgb_to_lab(target)));
        }
    }
    deltas.sort_by(|left, right| left.total_cmp(right));
    let trim = if deltas.len() >= 20 {
        deltas.len() / 10
    } else {
        0
    };
    let retained = &deltas[trim..deltas.len().saturating_sub(trim)];
    let robust_delta = retained.iter().sum::<f32>() / retained.len().max(1) as f32;
    let matched = deltas.len() as u32;
    (
        robust_delta,
        matched,
        matched as f32 / eligible_source_skin.max(1) as f32,
    )
}

fn is_probable_skin(pixel: &Rgba<u8>) -> bool {
    if pixel[3] <= ALPHA_THRESHOLD {
        return false;
    }
    let [r, g, b, _] = pixel.0;
    let maximum = r.max(g).max(b);
    let minimum = r.min(g).min(b);
    r.saturating_add(8) >= g
        && g.saturating_add(12) >= b
        && r > b.saturating_add(10)
        && maximum.saturating_sub(minimum) >= 12
        && maximum >= 55
        && minimum <= 235
}

fn protected_skin_artifacts(
    expression_id: &str,
    base: &RgbaImage,
    candidate: &RgbaImage,
    scope: &PortraitFaceScopeV1,
) -> (f32, f32, u32) {
    let mut samples = 0u32;
    let mut artifacts = 0u32;
    let mut severe = 0u32;
    for y in 0..base.height() {
        for x in 0..base.width() {
            if face_weight(scope, x, y) <= 0.0
                || is_expression_feature_zone(expression_id, scope, x, y)
            {
                continue;
            }
            let source = base.get_pixel(x, y);
            let target = candidate.get_pixel(x, y);
            if !is_probable_skin(source) || target[3] <= ALPHA_THRESHOLD {
                continue;
            }
            samples += 1;
            let darkening = luminance(source) - luminance(target);
            artifacts += u32::from(darkening > 25.0);
            severe += u32::from(darkening > 45.0);
        }
    }
    (
        artifacts as f32 / samples.max(1) as f32,
        severe as f32 / samples.max(1) as f32,
        samples,
    )
}

fn is_expression_feature_zone(
    expression_id: &str,
    scope: &PortraitFaceScopeV1,
    x: u32,
    y: u32,
) -> bool {
    let dx = (x as f32 - scope.center_x as f32) / scope.radius_x.max(1) as f32;
    let dy = (y as f32 - scope.center_y as f32) / scope.radius_y.max(1) as f32;
    let (eye_top, eye_bottom, eye_width, mouth_top, mouth_bottom, mouth_width) = match expression_id
    {
        "happy" => (-0.66, 0.04, 0.86, 0.04, 0.68, 0.76),
        "angry" => (-0.70, 0.08, 0.90, 0.08, 0.64, 0.68),
        "hurt" => (-0.72, 0.18, 0.92, 0.08, 0.66, 0.68),
        "surprised" => (-0.74, 0.20, 0.94, 0.00, 0.76, 0.76),
        _ => (-0.62, -0.03, 0.82, 0.12, 0.60, 0.62),
    };
    let eyes_and_brows = (eye_top..=eye_bottom).contains(&dy) && dx.abs() <= eye_width;
    let mouth = (mouth_top..=mouth_bottom).contains(&dy) && dx.abs() <= mouth_width;
    eyes_and_brows || mouth
}

fn face_edge_density(image: &RgbaImage, scope: &PortraitFaceScopeV1) -> f32 {
    let mut edges = 0usize;
    let mut comparisons = 0usize;
    for y in 0..image.height().saturating_sub(1) {
        for x in 0..image.width().saturating_sub(1) {
            if face_weight(scope, x, y) <= 0.0 {
                continue;
            }
            let pixel = image.get_pixel(x, y);
            if pixel[3] <= ALPHA_THRESHOLD {
                continue;
            }
            let current = luminance(pixel);
            for (next_x, next_y) in [(x + 1, y), (x, y + 1)] {
                if face_weight(scope, next_x, next_y) <= 0.0 {
                    continue;
                }
                let next = image.get_pixel(next_x, next_y);
                if next[3] <= ALPHA_THRESHOLD {
                    continue;
                }
                comparisons += 1;
                edges += usize::from((current - luminance(next)).abs() >= 24.0);
            }
        }
    }
    edges as f32 / comparisons.max(1) as f32
}

fn face_similarity(base: &RgbaImage, candidate: &RgbaImage, scope: &PortraitFaceScopeV1) -> f32 {
    let mut difference = 0.0f64;
    let mut compared = 0usize;
    for y in 0..base.height() {
        for x in 0..base.width() {
            if face_weight(scope, x, y) <= 0.0 {
                continue;
            }
            let left = base.get_pixel(x, y);
            let right = candidate.get_pixel(x, y);
            if left[3] <= ALPHA_THRESHOLD && right[3] <= ALPHA_THRESHOLD {
                continue;
            }
            difference += (0..3)
                .map(|channel| f64::from(left[channel].abs_diff(right[channel])))
                .sum::<f64>()
                / (255.0 * 3.0);
            compared += 1;
        }
    }
    (1.0 - (difference / compared.max(1) as f64) as f32).clamp(0.0, 1.0)
}

fn assess_local_metrics(metrics: &PortraitLocalMetricsV1) -> (ConsistencyVerdict, Vec<String>) {
    let mut verdict = ConsistencyVerdict::GameReady;
    let mut reasons = Vec::new();
    if metrics.output_outside_face_changed_ratio > 0.0 {
        verdict = ConsistencyVerdict::Blocked;
        reasons.push("portrait_protected_pixels_changed".into());
    }
    if metrics.skin_sample_count < 24 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::AwaitingReview);
        reasons.push("portrait_skin_samples_insufficient".into());
    } else if metrics.skin_tone_delta_e > 18.0 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::Regenerate);
        reasons.push("portrait_skin_tone_drift".into());
    } else if metrics.skin_tone_delta_e > 10.0 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::AwaitingReview);
        reasons.push("portrait_skin_tone_review".into());
    }
    if metrics.skin_correspondence_ratio < 0.65 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::Regenerate);
        reasons.push("portrait_skin_correspondence_lost".into());
    } else if metrics.skin_correspondence_ratio < 0.80 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::AwaitingReview);
        reasons.push("portrait_skin_correspondence_review".into());
    }
    let protected_artifact_count =
        metrics.protected_skin_artifact_ratio * metrics.protected_skin_sample_count as f32;
    let protected_severe_count =
        metrics.protected_skin_severe_artifact_ratio * metrics.protected_skin_sample_count as f32;
    if metrics.protected_skin_sample_count < 24 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::AwaitingReview);
        reasons.push("portrait_protected_skin_samples_insufficient".into());
    } else if (metrics.protected_skin_artifact_ratio > 0.25 && protected_artifact_count >= 32.0)
        || (metrics.protected_skin_severe_artifact_ratio > 0.04 && protected_severe_count >= 20.0)
    {
        verdict = worse_verdict(verdict, ConsistencyVerdict::Regenerate);
        reasons.push("portrait_protected_skin_artifact".into());
    } else if (metrics.protected_skin_artifact_ratio > 0.05 && protected_artifact_count >= 24.0)
        || (metrics.protected_skin_severe_artifact_ratio > 0.025 && protected_severe_count >= 12.0)
    {
        verdict = worse_verdict(verdict, ConsistencyVerdict::AwaitingReview);
        reasons.push("portrait_protected_skin_artifact_review".into());
    }
    if !(0.70..=1.35).contains(&metrics.face_edge_density_ratio) {
        verdict = worse_verdict(verdict, ConsistencyVerdict::Regenerate);
        reasons.push("portrait_face_artifact".into());
    } else if !(0.80..=1.22).contains(&metrics.face_edge_density_ratio) {
        verdict = worse_verdict(verdict, ConsistencyVerdict::AwaitingReview);
        reasons.push("portrait_face_detail_review".into());
    }
    if metrics.face_perceptual_similarity < 0.78 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::Regenerate);
        reasons.push("portrait_face_identity_drift".into());
    } else if metrics.face_perceptual_similarity < 0.88 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::AwaitingReview);
        reasons.push("portrait_face_identity_review".into());
    }
    if metrics.face_changed_ratio < 0.002 {
        verdict = worse_verdict(verdict, ConsistencyVerdict::Regenerate);
        reasons.push("portrait_expression_unchanged".into());
    }
    (verdict, reasons)
}

fn face_weight(scope: &PortraitFaceScopeV1, x: u32, y: u32) -> f32 {
    let dx = (x as f32 - scope.center_x as f32) / scope.radius_x.max(1) as f32;
    let dy = (y as f32 - scope.center_y as f32) / scope.radius_y.max(1) as f32;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance >= 1.0 {
        return 0.0;
    }
    let feather = scope.feather_px as f32 / scope.radius_x.min(scope.radius_y).max(1) as f32;
    let inner = (1.0 - feather).clamp(0.0, 0.95);
    if distance <= inner {
        1.0
    } else {
        ((1.0 - distance) / (1.0 - inner)).clamp(0.0, 1.0)
    }
}

fn alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] <= ALPHA_THRESHOLD {
            continue;
        }
        found = true;
        left = left.min(x);
        top = top.min(y);
        right = right.max(x);
        bottom = bottom.max(y);
    }
    found.then_some((left, top, right, bottom))
}

fn luminance(pixel: &Rgba<u8>) -> f32 {
    0.2126 * pixel[0] as f32 + 0.7152 * pixel[1] as f32 + 0.0722 * pixel[2] as f32
}

fn rgb_to_lab(pixel: &Rgba<u8>) -> [f32; 3] {
    let linear = |value: u8| {
        let value = value as f32 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let r = linear(pixel[0]);
    let g = linear(pixel[1]);
    let b = linear(pixel[2]);
    let x = (0.4124564 * r + 0.3575761 * g + 0.1804375 * b) / 0.95047;
    let y = 0.2126729 * r + 0.7151522 * g + 0.072175 * b;
    let z = (0.0193339 * r + 0.119192 * g + 0.9503041 * b) / 1.08883;
    let pivot = |value: f32| {
        if value > 0.008856 {
            value.cbrt()
        } else {
            7.787 * value + 16.0 / 116.0
        }
    };
    let fx = pivot(x);
    let fy = pivot(y);
    let fz = pivot(z);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn cie76(left: [f32; 3], right: [f32; 3]) -> f32 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

fn worse_verdict(left: ConsistencyVerdict, right: ConsistencyVerdict) -> ConsistencyVerdict {
    if verdict_rank(right) > verdict_rank(left) {
        right
    } else {
        left
    }
}

fn verdict_rank(verdict: ConsistencyVerdict) -> u8 {
    match verdict {
        ConsistencyVerdict::GameReady => 0,
        ConsistencyVerdict::AwaitingReview => 1,
        ConsistencyVerdict::Regenerate => 2,
        ConsistencyVerdict::Blocked => 3,
    }
}

fn expression_order(id: &str) -> usize {
    match id {
        "neutral" => 0,
        "happy" => 1,
        "angry" => 2,
        "hurt" => 3,
        "surprised" => 4,
        _ => usize::MAX,
    }
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), PortraitError> {
    let parent = path
        .parent()
        .ok_or_else(|| PortraitError::Invalid("output path has no parent".into()))?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("portrait"),
        uuid::Uuid::new_v4()
    ));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use image::ImageBuffer;

    use super::*;

    fn base_image() -> RgbaImage {
        let mut image = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 8..116 {
            for x in 38..90 {
                image.put_pixel(x, y, Rgba([70, 90, 45, 255]));
            }
        }
        for y in 30..62 {
            for x in 49..79 {
                image.put_pixel(x, y, Rgba([174, 111, 76, 255]));
            }
        }
        for y in 44..48 {
            for x in [55, 72] {
                image.put_pixel(x, y, Rgba([30, 20, 15, 255]));
            }
        }
        image
    }

    #[test]
    fn face_composite_keeps_every_protected_pixel_exact() {
        let base = base_image();
        let scope = derive_face_scope(&base, PortraitFramingProfileV1::FullBody).unwrap();
        let mut candidate = base.clone();
        for pixel in candidate.pixels_mut() {
            if pixel[3] > ALPHA_THRESHOLD {
                *pixel = Rgba([220, 70, 20, 255]);
            }
        }
        let (output, report) =
            composite_and_assess_expression("happy", &base, &candidate, &scope).unwrap();
        assert!(report.metrics.raw_outside_face_changed_ratio > 0.5);
        assert_eq!(report.metrics.output_outside_face_changed_ratio, 0.0);
        for y in 0..base.height() {
            for x in 0..base.width() {
                if face_weight(&scope, x, y) == 0.0 {
                    assert_eq!(base.get_pixel(x, y), output.get_pixel(x, y));
                }
            }
        }
    }

    #[test]
    fn skin_and_face_artifact_drift_are_not_silently_game_ready() {
        let base = base_image();
        let scope = derive_face_scope(&base, PortraitFramingProfileV1::FullBody).unwrap();
        let mut candidate = base.clone();
        for y in 36..58 {
            for x in 52..76 {
                candidate.put_pixel(x, y, Rgba([245, 180, 130, 255]));
            }
        }
        for offset in 0..12 {
            candidate.put_pixel(54 + offset, 40 + offset / 2, Rgba([20, 10, 10, 255]));
            candidate.put_pixel(70 - offset, 47 + offset / 3, Rgba([20, 10, 10, 255]));
        }
        let (_, report) =
            composite_and_assess_expression("hurt", &base, &candidate, &scope).unwrap();
        assert_ne!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(report.reasons.iter().any(|reason| {
            matches!(
                reason.as_str(),
                "portrait_skin_tone_drift"
                    | "portrait_skin_tone_review"
                    | "portrait_face_artifact"
                    | "portrait_face_detail_review"
            )
        }));
    }

    #[test]
    fn surprised_eye_and_mouth_expansion_do_not_count_as_skin_tone_drift() {
        let base = base_image();
        let scope = derive_face_scope(&base, PortraitFramingProfileV1::FullBody).unwrap();
        let mut candidate = base.clone();
        for y in 0..base.height() {
            for x in 0..base.width() {
                if is_expression_feature_zone("surprised", &scope, x, y)
                    && is_probable_skin(base.get_pixel(x, y))
                {
                    let replacement = if (x + y) % 2 == 0 {
                        Rgba([245, 245, 240, 255])
                    } else {
                        Rgba([35, 20, 18, 255])
                    };
                    candidate.put_pixel(x, y, replacement);
                }
            }
        }
        let (_, report) =
            composite_and_assess_expression("surprised", &base, &candidate, &scope).unwrap();
        assert_eq!(report.metrics.skin_tone_delta_e, 0.0);
        assert_eq!(report.metrics.skin_correspondence_ratio, 1.0);
        assert_eq!(report.metrics.protected_skin_artifact_ratio, 0.0);
        assert!(!report.reasons.iter().any(|reason| {
            matches!(
                reason.as_str(),
                "portrait_skin_tone_drift"
                    | "portrait_skin_tone_review"
                    | "portrait_skin_correspondence_lost"
                    | "portrait_skin_correspondence_review"
                    | "portrait_protected_skin_artifact"
                    | "portrait_protected_skin_artifact_review"
            )
        }));
    }

    #[test]
    fn non_feature_skin_replacement_loses_correspondence_and_cannot_pass() {
        let base = base_image();
        let scope = derive_face_scope(&base, PortraitFramingProfileV1::FullBody).unwrap();
        let mut candidate = base.clone();
        for y in 0..base.height() {
            for x in 0..base.width() {
                if face_weight(&scope, x, y) >= 0.55
                    && !is_expression_feature_zone("happy", &scope, x, y)
                    && is_probable_skin(base.get_pixel(x, y))
                {
                    candidate.put_pixel(x, y, Rgba([30, 90, 220, 255]));
                }
            }
        }
        let (_, report) =
            composite_and_assess_expression("happy", &base, &candidate, &scope).unwrap();
        assert_eq!(
            report.verdict,
            ConsistencyVerdict::Regenerate,
            "{report:#?}"
        );
        assert!(report.metrics.skin_correspondence_ratio < 0.65);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason == "portrait_skin_correspondence_lost"));
    }

    #[test]
    fn cheek_marks_outside_expression_features_require_regeneration() {
        let base = base_image();
        let scope = derive_face_scope(&base, PortraitFramingProfileV1::FullBody).unwrap();
        let mut candidate = base.clone();
        for y in (scope.center_y - 2)..=(scope.center_y + 4) {
            for x in
                (scope.center_x - scope.radius_x + 1)..=(scope.center_x - scope.radius_x / 2 - 1)
            {
                candidate.put_pixel(x, y, Rgba([20, 10, 10, 255]));
            }
        }
        let (_, report) =
            composite_and_assess_expression("hurt", &base, &candidate, &scope).unwrap();
        assert_eq!(report.verdict, ConsistencyVerdict::Regenerate);
        assert!(
            report.reasons.iter().any(|reason| matches!(
                reason.as_str(),
                "portrait_protected_skin_artifact" | "portrait_face_artifact"
            )),
            "{report:#?}"
        );
    }

    #[test]
    fn approval_is_bound_to_job_lock_image_and_reference_policy() {
        let temp = tempfile::tempdir().unwrap();
        let normalized = temp.path().join("normalized.png");
        let provider = temp.path().join("provider.png");
        base_image().save(&normalized).unwrap();
        base_image().save(&provider).unwrap();
        let lock = materialize_portrait_base_lock(
            temp.path(),
            &normalized,
            Some(&provider),
            PortraitBaseProvenanceV1 {
                asset_id: "hero-portraits".into(),
                provider_id: "fixture".into(),
                profile_id: "default".into(),
                model: Some("fixture-image".into()),
                style_revision: "style-r1".into(),
                subject_id: "hero".into(),
                subject_revision: "subject-r1".into(),
                collection_id: "portraits".into(),
                collection_revision: "collection-r1".into(),
                framing_profile: PortraitFramingProfileV1::FullBody,
                neutral_reference_policy: PortraitNeutralReferencePolicyV1::SubjectStyle,
                neutral_reference_roles: vec![ReferenceRole::SubjectIdentity, ReferenceRole::Style],
                neutral_reference_sha256: vec!["a".repeat(64), "b".repeat(64)],
            },
        )
        .unwrap();
        let (approval_path, approval) = approve_portrait_base(
            "base-job-1",
            temp.path(),
            "identity and equipment approved",
            Utc::now(),
        )
        .unwrap();
        assert_eq!(approval.neutral_sha256, lock.image_sha256);
        assert_eq!(
            approval.neutral_reference_policy,
            PortraitNeutralReferencePolicyV1::SubjectStyle
        );
        validate_portrait_base_approval(temp.path(), "base-job-1").unwrap();

        let mut value: serde_json::Value =
            serde_json::from_slice(&fs::read(&approval_path).unwrap()).unwrap();
        value["neutralSha256"] = serde_json::json!("0".repeat(64));
        fs::write(&approval_path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let error = validate_portrait_base_approval(temp.path(), "base-job-1").unwrap_err();
        assert!(error.to_string().contains("no longer matches"));
    }
}
