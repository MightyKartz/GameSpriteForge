use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use image::{imageops::FilterType, ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::asset_project::{
    hash_file, image_signature, matte_static_image, normalize_static_image,
    occupancy_profile_similarity, read_project, read_style_lock, resolve_relative, ImageSignature,
    PaletteColor, StaticAssetItemSpecV1, StaticAssetKind, StaticAssetSetSpecV1, StyleLockV1,
    SubjectRevisionRefV1, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
use crate::geometry::{
    assess_portrait_geometry, AssetGeometryReportV1, AssetGeometryVerdictV1,
    PortraitFramingProfileV1,
};
use crate::provider::{
    EditImageRequest, MediaGenerationProvider, ProviderImageReference, ReferenceRole,
};

pub const COLLECTION_LOCK_FILE: &str = "collection-lock.json";
pub const COLLECTION_CONSISTENCY_PROFILE: &str = "collection-consistency@1.1.0";
const LEGACY_COLLECTION_CONSISTENCY_PROFILE: &str = "collection-consistency@1.0.0";
pub const COLLECTION_ANCHOR_PROFILE: &str = "collection-anchor@1.1.0";
pub const PORTRAIT_REFRAME_PROFILE: &str = "portrait-reframe@1.0.0";

#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
    #[error("invalid collection: {0}")]
    Invalid(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("image error: {0}")]
    Image(#[from] image::ImageError),
    #[error("asset project error: {0}")]
    Asset(#[from] crate::asset_project::AssetProjectError),
    #[error("provider error: {0}")]
    Provider(#[from] crate::provider::ProviderError),
}

impl CollectionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Invalid(_) => "invalid_collection",
            Self::Io(_) => "collection_io_error",
            Self::Json(_) => "collection_invalid_json",
            Self::Image(_) => "collection_invalid_image",
            Self::Asset(_) => "collection_asset_error",
            Self::Provider(_) => "provider_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionGrounding {
    Center,
    Feet,
    Surface,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionSpecV1 {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub asset_kind: StaticAssetKind,
    pub prompt: String,
    #[serde(default)]
    pub materials: Vec<String>,
    #[serde(default = "default_scale")]
    pub scale: String,
    #[serde(default = "default_perspective")]
    pub perspective: String,
    #[serde(default)]
    pub grounding: Option<CollectionGrounding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framing_profile: Option<PortraitFramingProfileV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas_size: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor_image: Option<PathBuf>,
    #[serde(default = "default_license")]
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
}

impl CollectionSpecV1 {
    pub fn effective_portrait_framing_profile(&self) -> Option<PortraitFramingProfileV1> {
        (self.asset_kind == StaticAssetKind::PortraitSet)
            .then_some(self.framing_profile.unwrap_or_default())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionBaselineV1 {
    pub palette: Vec<PaletteColor>,
    pub edge_density: f32,
    pub foreground_scale: f32,
    pub perceptual_hash: String,
    pub anchor_x: f32,
    pub anchor_y: f32,
    pub canvas_size: u32,
}

impl CollectionBaselineV1 {
    pub fn signature(&self) -> Result<ImageSignature, CollectionError> {
        let perceptual_hash = u64::from_str_radix(&self.perceptual_hash, 16).map_err(|_| {
            CollectionError::Invalid("collection perceptualHash is not hexadecimal".into())
        })?;
        Ok(ImageSignature {
            palette: self.palette.clone(),
            edge_density: self.edge_density,
            foreground_scale: self.foreground_scale,
            perceptual_hash,
            anchor_x: self.anchor_x,
            anchor_y: self.anchor_y,
            width: self.canvas_size,
            height: self.canvas_size,
            alpha_present: true,
            cell_boundary_safe: true,
            subject_count: 1,
            foreground_width_ratio: self.foreground_scale,
            foreground_height_ratio: self.foreground_scale,
            foreground_area_ratio: 0.25,
            centroid_y_ratio: 0.5,
            detached_component_ratio: 0.0,
            bottom_band_span_ratio: 0.0,
            bottom_band_area_share: 0.0,
            platform_overhang_ratio: 0.0,
            row_occupancy_profile: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionLockV1 {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub revision: String,
    pub asset_kind: StaticAssetKind,
    pub provider_id: String,
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    pub style_revision: String,
    pub style_board_sha256: String,
    pub prompt: String,
    pub materials: Vec<String>,
    pub scale: String,
    pub perspective: String,
    pub grounding: CollectionGrounding,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framing_profile: Option<PortraitFramingProfileV1>,
    pub canvas_size: u32,
    pub anchor_path: PathBuf,
    pub anchor_sha256: String,
    pub medoid_path: PathBuf,
    pub medoid_sha256: String,
    pub baseline: CollectionBaselineV1,
    pub outlier_profile: String,
    pub license: String,
    pub created_at: DateTime<Utc>,
}

impl CollectionLockV1 {
    /// Old Portrait CollectionLocks predate explicit framing. Their established
    /// behavior is dialogue-bust framing; other collection kinds have no
    /// portrait framing semantics.
    pub fn effective_portrait_framing_profile(&self) -> Option<PortraitFramingProfileV1> {
        (self.asset_kind == StaticAssetKind::PortraitSet)
            .then_some(self.framing_profile.unwrap_or_default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectionBuildOutput {
    pub collection_lock_path: PathBuf,
    pub anchor_path: PathBuf,
    pub id: String,
    pub revision: String,
    pub anchor_sha256: String,
    pub anchor_report_path: Option<PathBuf>,
    pub provider_requests: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionAnchorQualityReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub asset_kind: StaticAssetKind,
    pub verdict: CollectionItemVerdict,
    pub subject_count: u32,
    pub detached_component_ratio: f32,
    pub bottom_band_span_ratio: f32,
    pub bottom_band_area_share: f32,
    pub platform_overhang_ratio: f32,
    pub foreground_area_ratio: f32,
    pub foreground_width_ratio: f32,
    pub foreground_height_ratio: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<AssetGeometryReportV1>,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionRevisionRefV1 {
    pub id: String,
    pub revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticItemRoleV1 {
    Icon,
    WorldProp,
    EquippedPreview,
    PortraitExpression,
    Decal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FootprintV1 {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecalBlendV1 {
    Mix,
    Multiply,
    Add,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StaticItemMetadataV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<StaticItemRoleV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expression: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footprint: Option<FootprintV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blend: Option<DecalBlendV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StaticCollectionSetSpecV2 {
    pub schema_version: String,
    pub kind: StaticAssetKind,
    pub id: String,
    pub name: String,
    pub collection: CollectionRevisionRefV1,
    pub items: Vec<StaticAssetItemSpecV1>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitExpressionSpecV1 {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitSetSpecV1 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub collection: CollectionRevisionRefV1,
    pub subject: SubjectRevisionRefV1,
    pub expressions: Vec<PortraitExpressionSpecV1>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PortraitSetSpecV2 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub collection: CollectionRevisionRefV1,
    pub subject: SubjectRevisionRefV1,
    pub framing_profile: PortraitFramingProfileV1,
    pub expressions: Vec<PortraitExpressionSpecV1>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquipmentItemSpecV1 {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<PathBuf>,
    #[serde(default)]
    pub equipped_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EquipmentSetSpecV1 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub collection: CollectionRevisionRefV1,
    pub items: Vec<EquipmentItemSpecV1>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecalItemSpecV1 {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<PathBuf>,
    pub footprint: FootprintV1,
    #[serde(default = "default_decal_blend")]
    pub blend: DecalBlendV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DecalSetSpecV1 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    pub collection: CollectionRevisionRefV1,
    pub items: Vec<DecalItemSpecV1>,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedStaticCollectionSpecV1 {
    pub asset: StaticAssetSetSpecV1,
    pub collection: CollectionRevisionRefV1,
    pub subject: Option<SubjectRevisionRefV1>,
    pub framing_profile: Option<PortraitFramingProfileV1>,
    pub item_metadata: BTreeMap<String, StaticItemMetadataV1>,
}

pub fn read_static_collection_spec(
    path: &Path,
    expected_kind: StaticAssetKind,
) -> Result<ResolvedStaticCollectionSpecV1, CollectionError> {
    let bytes = fs::read(path)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    let schema = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    let mut resolved = match expected_kind {
        StaticAssetKind::IconSet | StaticAssetKind::PropSet => {
            if schema != "2" {
                return Err(CollectionError::Invalid(format!(
                    "{} with CollectionLock requires schemaVersion 2",
                    expected_kind.as_str()
                )));
            }
            let spec: StaticCollectionSetSpecV2 = serde_json::from_value(value)?;
            if spec.kind != expected_kind {
                return Err(CollectionError::Invalid(
                    "static collection kind mismatch".into(),
                ));
            }
            ResolvedStaticCollectionSpecV1 {
                asset: StaticAssetSetSpecV1 {
                    schema_version: "2".into(),
                    kind: spec.kind,
                    id: spec.id,
                    name: spec.name,
                    items: spec.items,
                    license: spec.license,
                },
                collection: spec.collection,
                subject: None,
                framing_profile: None,
                item_metadata: BTreeMap::new(),
            }
        }
        StaticAssetKind::PortraitSet => {
            let (
                schema_version,
                kind,
                id,
                name,
                collection,
                subject,
                framing_profile,
                expressions,
                license,
            ) = match schema {
                "1" => {
                    let spec: PortraitSetSpecV1 = serde_json::from_value(value)?;
                    (
                        spec.schema_version,
                        spec.kind,
                        spec.id,
                        spec.name,
                        spec.collection,
                        spec.subject,
                        PortraitFramingProfileV1::DialogueBust,
                        spec.expressions,
                        spec.license,
                    )
                }
                "2" => {
                    let spec: PortraitSetSpecV2 = serde_json::from_value(value)?;
                    (
                        spec.schema_version,
                        spec.kind,
                        spec.id,
                        spec.name,
                        spec.collection,
                        spec.subject,
                        spec.framing_profile,
                        spec.expressions,
                        spec.license,
                    )
                }
                _ => {
                    return Err(CollectionError::Invalid(
                        "PortraitSet requires schemaVersion 1 or 2".into(),
                    ))
                }
            };
            if !matches!(schema_version.as_str(), "1" | "2")
                || kind != "portrait_set"
                || !valid_id(&id)
                || name.trim().is_empty()
                || expressions.is_empty()
            {
                return Err(CollectionError::Invalid(format!(
                    "invalid portrait_set V{schema_version} spec"
                )));
            }
            let required = ["neutral", "happy", "angry", "hurt", "surprised"];
            if expressions.len() != required.len()
                || required
                    .iter()
                    .any(|required| !expressions.iter().any(|item| item.id == *required))
            {
                return Err(CollectionError::Invalid(
                    "PortraitSet requires exactly neutral, happy, angry, hurt, and surprised"
                        .into(),
                ));
            }
            if schema_version == "2"
                && expressions.first().map(|item| item.id.as_str()) != Some("neutral")
            {
                return Err(CollectionError::Invalid(
                    "PortraitSet V2 requires neutral as the first expression so it can be locked as the immutable PortraitBase".into(),
                ));
            }
            let mut metadata = BTreeMap::new();
            let items = expressions
                .into_iter()
                .map(|item| {
                    metadata.insert(
                        item.id.clone(),
                        StaticItemMetadataV1 {
                            role: Some(StaticItemRoleV1::PortraitExpression),
                            expression: Some(item.id.clone()),
                            ..Default::default()
                        },
                    );
                    StaticAssetItemSpecV1 {
                        id: item.id,
                        name: item.name,
                        prompt: item.prompt,
                        reference_image: item.reference_image,
                    }
                })
                .collect();
            ResolvedStaticCollectionSpecV1 {
                asset: StaticAssetSetSpecV1 {
                    schema_version,
                    kind: expected_kind,
                    id,
                    name,
                    items,
                    license,
                },
                collection,
                subject: Some(subject),
                framing_profile: Some(framing_profile),
                item_metadata: metadata,
            }
        }
        StaticAssetKind::EquipmentSet => {
            let spec: EquipmentSetSpecV1 = serde_json::from_value(value)?;
            validate_named_set(
                &spec.schema_version,
                &spec.kind,
                "equipment_set",
                &spec.id,
                &spec.name,
                spec.items.len(),
            )?;
            let mut items = Vec::new();
            let mut metadata = BTreeMap::new();
            for item in spec.items {
                for (suffix, role, direction) in [
                    ("icon", StaticItemRoleV1::Icon, "inventory icon"),
                    ("world", StaticItemRoleV1::WorldProp, "world pickup prop"),
                ] {
                    let id = format!("{}__{suffix}", item.id);
                    items.push(StaticAssetItemSpecV1 {
                        id: id.clone(),
                        name: format!("{} {suffix}", item.name),
                        prompt: format!("{}; render as {direction}", item.prompt),
                        reference_image: item.reference_image.clone(),
                    });
                    metadata.insert(
                        id,
                        StaticItemMetadataV1 {
                            role: Some(role),
                            group_id: Some(item.id.clone()),
                            ..Default::default()
                        },
                    );
                }
                if item.equipped_preview {
                    let id = format!("{}__equipped", item.id);
                    items.push(StaticAssetItemSpecV1 {
                        id: id.clone(),
                        name: format!("{} equipped preview", item.name),
                        prompt: format!(
                            "{}; render as a static equipped preview without a character",
                            item.prompt
                        ),
                        reference_image: item.reference_image,
                    });
                    metadata.insert(
                        id,
                        StaticItemMetadataV1 {
                            role: Some(StaticItemRoleV1::EquippedPreview),
                            group_id: Some(item.id),
                            ..Default::default()
                        },
                    );
                }
            }
            ResolvedStaticCollectionSpecV1 {
                asset: StaticAssetSetSpecV1 {
                    schema_version: "1".into(),
                    kind: expected_kind,
                    id: spec.id,
                    name: spec.name,
                    items,
                    license: spec.license,
                },
                collection: spec.collection,
                subject: None,
                framing_profile: None,
                item_metadata: metadata,
            }
        }
        StaticAssetKind::DecalSet => {
            let spec: DecalSetSpecV1 = serde_json::from_value(value)?;
            validate_named_set(
                &spec.schema_version,
                &spec.kind,
                "decal_set",
                &spec.id,
                &spec.name,
                spec.items.len(),
            )?;
            let mut metadata = BTreeMap::new();
            let items = spec
                .items
                .into_iter()
                .map(|item| {
                    if item.footprint.width == 0
                        || item.footprint.height == 0
                        || item.footprint.width > 32
                        || item.footprint.height > 32
                    {
                        return Err(CollectionError::Invalid(
                            "decal footprint must be 1..=32 cells per axis".into(),
                        ));
                    }
                    metadata.insert(
                        item.id.clone(),
                        StaticItemMetadataV1 {
                            role: Some(StaticItemRoleV1::Decal),
                            footprint: Some(item.footprint),
                            blend: Some(item.blend),
                            ..Default::default()
                        },
                    );
                    Ok(StaticAssetItemSpecV1 {
                        id: item.id,
                        name: item.name,
                        prompt: item.prompt,
                        reference_image: item.reference_image,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            ResolvedStaticCollectionSpecV1 {
                asset: StaticAssetSetSpecV1 {
                    schema_version: "1".into(),
                    kind: expected_kind,
                    id: spec.id,
                    name: spec.name,
                    items,
                    license: spec.license,
                },
                collection: spec.collection,
                subject: None,
                framing_profile: None,
                item_metadata: metadata,
            }
        }
    };
    if resolved.asset.items.is_empty() || resolved.asset.items.len() > 64 {
        return Err(CollectionError::Invalid(
            "static set requires 1 through 64 materialized items".into(),
        ));
    }
    let mut ids = std::collections::BTreeSet::new();
    for item in &mut resolved.asset.items {
        if !valid_id(&item.id)
            || item.name.trim().is_empty()
            || item.prompt.trim().is_empty()
            || !ids.insert(item.id.clone())
        {
            return Err(CollectionError::Invalid(
                "static item ids must be unique and engine-safe, with non-empty name and prompt"
                    .into(),
            ));
        }
        if let Some(reference) = &item.reference_image {
            item.reference_image = Some(resolve_relative(root, reference));
        }
    }
    Ok(resolved)
}

fn validate_named_set(
    schema: &str,
    kind: &str,
    expected_kind: &str,
    id: &str,
    name: &str,
    count: usize,
) -> Result<(), CollectionError> {
    if schema != "1"
        || kind != expected_kind
        || !valid_id(id)
        || name.trim().is_empty()
        || count == 0
    {
        return Err(CollectionError::Invalid(format!(
            "invalid {expected_kind} V1 spec"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionItemVerdict {
    GameReady,
    AwaitingReview,
    Regenerate,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionItemConsistencyV1 {
    pub id: String,
    pub attempt: u8,
    #[serde(default = "default_true")]
    pub comparable: bool,
    pub anchor_similarity: f32,
    pub medoid_similarity: f32,
    pub collection_score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape_similarity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framing_similarity: Option<f32>,
    pub verdict: CollectionItemVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CollectionConsistencyReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub collection_id: String,
    pub collection_revision: String,
    pub medoid_item_id: String,
    pub medoid_sha256: String,
    pub pairwise_mean_similarity: f32,
    pub verdict: CollectionItemVerdict,
    pub items: Vec<CollectionItemConsistencyV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionAssessmentItemV1 {
    pub id: String,
    pub attempt: u8,
    pub path: Option<PathBuf>,
    pub base_verdict: CollectionItemVerdict,
    pub base_reasons: Vec<String>,
}

pub fn read_collection_spec(path: &Path) -> Result<CollectionSpecV1, CollectionError> {
    let mut spec: CollectionSpecV1 = serde_json::from_slice(&fs::read(path)?)?;
    validate_collection_spec(&spec)?;
    if let Some(anchor) = &spec.anchor_image {
        spec.anchor_image = Some(resolve_relative(
            path.parent().unwrap_or_else(|| Path::new(".")),
            anchor,
        ));
    }
    Ok(spec)
}

pub fn validate_collection_spec(spec: &CollectionSpecV1) -> Result<(), CollectionError> {
    if spec.schema_version != "1" {
        return Err(CollectionError::Invalid(
            "CollectionSpec requires schemaVersion 1".into(),
        ));
    }
    if !valid_id(&spec.id) || spec.name.trim().is_empty() || spec.prompt.trim().is_empty() {
        return Err(CollectionError::Invalid(
            "collection id, name, and prompt are required".into(),
        ));
    }
    if spec.materials.len() > 16 || spec.materials.iter().any(|value| value.trim().is_empty()) {
        return Err(CollectionError::Invalid(
            "materials accepts at most 16 non-empty values".into(),
        ));
    }
    if spec.asset_kind != StaticAssetKind::PortraitSet && spec.framing_profile.is_some() {
        return Err(CollectionError::Invalid(
            "framingProfile is reserved for portrait_set collections".into(),
        ));
    }
    if spec.effective_portrait_framing_profile() == Some(PortraitFramingProfileV1::FullBody)
        && spec
            .grounding
            .as_ref()
            .is_some_and(|grounding| *grounding != CollectionGrounding::Feet)
    {
        return Err(CollectionError::Invalid(
            "full_body portrait collections require feet grounding".into(),
        ));
    }
    if let Some(canvas) = spec.canvas_size {
        if !(64..=512).contains(&canvas) || !canvas.is_power_of_two() {
            return Err(CollectionError::Invalid(
                "canvasSize must be a power of two from 64 through 512".into(),
            ));
        }
    }
    if spec.license.trim().is_empty() {
        return Err(CollectionError::Invalid("license is required".into()));
    }
    if let Some(anchor) = &spec.anchor_image {
        let metadata = fs::symlink_metadata(anchor).map_err(|_| {
            CollectionError::Invalid(format!("anchor image does not exist: {}", anchor.display()))
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(CollectionError::Invalid(
                "anchor image must be a regular non-symlink file".into(),
            ));
        }
        if anchor.extension().and_then(|value| value.to_str()) != Some("png") {
            return Err(CollectionError::Invalid(
                "anchor image must be a PNG".into(),
            ));
        }
        image::open(anchor)?;
    }
    Ok(())
}

pub fn collection_requires_provider(spec_path: &Path) -> Result<bool, CollectionError> {
    Ok(read_collection_spec(spec_path)?.anchor_image.is_none())
}

pub fn build_collection_lock(
    project_root: &Path,
    spec_path: &Path,
    provider_id: &str,
    profile_id: &str,
    provider: Option<&dyn MediaGenerationProvider>,
    work_dir: &Path,
) -> Result<CollectionBuildOutput, CollectionError> {
    let mut project = read_project(project_root)?;
    if project.provider.id != provider_id || project.provider.profile_id != profile_id {
        return Err(CollectionError::Invalid(
            "collection generation must use the project Provider profile".into(),
        ));
    }
    let style_revision = project.current_style_revision.clone().ok_or_else(|| {
        CollectionError::Invalid("run `forge style create` before collection create".into())
    })?;
    let style_path = project_root
        .join(".forge/styles")
        .join(&style_revision)
        .join(STYLE_LOCK_FILE);
    let style = read_style_lock(&style_path)?;
    let mut spec = read_collection_spec(spec_path)?;
    let framing_profile = spec.effective_portrait_framing_profile();
    let image_model = provider
        .and_then(|provider| provider.resolved_image_model(spec.image_model.as_deref()))
        .or_else(|| spec.image_model.clone())
        .or_else(|| style.image_model.clone());
    spec.image_model = image_model.clone();
    let canvas = spec
        .canvas_size
        .unwrap_or_else(|| canvas_for_kind(spec.asset_kind, &style));
    let anchor_input_sha = spec
        .anchor_image
        .as_deref()
        .map(hash_file)
        .transpose()?
        .unwrap_or_else(|| "provider-generated".into());
    let revision = collection_revision(&spec, &style, provider_id, profile_id, &anchor_input_sha)?;
    let collection_dir = project_root
        .join(".forge/collections")
        .join(&spec.id)
        .join(&revision);
    let lock_path = collection_dir.join(COLLECTION_LOCK_FILE);
    if lock_path.is_file() {
        let lock = read_collection_lock(&lock_path)?;
        project
            .current_collection_revisions
            .insert(lock.id.clone(), lock.revision.clone());
        write_json_atomic(&project_root.join(FORGE_PROJECT_FILE), &project)?;
        return Ok(CollectionBuildOutput {
            collection_lock_path: lock_path,
            anchor_path: lock.anchor_path,
            id: lock.id,
            revision: lock.revision,
            anchor_sha256: lock.anchor_sha256,
            anchor_report_path: None,
            provider_requests: 0,
        });
    }
    fs::create_dir_all(work_dir)?;
    let raw_anchor = work_dir.join("collection-anchor-source.png");
    let provider_requests = if let Some(source) = &spec.anchor_image {
        fs::copy(source, &raw_anchor)?;
        0
    } else {
        let provider = provider.ok_or_else(|| {
            CollectionError::Invalid("collection anchor generation requires a Provider".into())
        })?;
        let style_reference =
            ProviderImageReference::from_path(ReferenceRole::Style, style.board_path.clone())?;
        provider.edit_image(
            &EditImageRequest {
                authorization_target: Some(format!("collection:{}", spec.id)),
                prompt: collection_anchor_prompt(&spec, &style),
                model: image_model.clone(),
                references: vec![style_reference],
                aspect_ratio: "1:1".into(),
                resolution: "1k".into(),
            },
            &raw_anchor,
        )?;
        1
    };
    let normalized = work_dir.join("collection-anchor.png");
    let grounding = grounding_for(&spec);
    let image = normalize_static_image(
        &raw_anchor,
        &normalized,
        canvas,
        grounding != CollectionGrounding::Center,
    )?;
    let mut anchor_report = assess_collection_anchor(spec.asset_kind, &image_signature(&image));
    if let Some(profile) = framing_profile {
        let matted_source = matte_static_image(&raw_anchor)?;
        let geometry = assess_portrait_geometry(&matted_source, profile);
        for reason in &geometry.reasons {
            if !anchor_report.reasons.contains(reason) {
                anchor_report.reasons.push(reason.clone());
            }
        }
        match geometry.verdict {
            AssetGeometryVerdictV1::Blocked => {
                anchor_report.verdict = CollectionItemVerdict::Blocked
            }
            AssetGeometryVerdictV1::AwaitingReview
                if anchor_report.verdict == CollectionItemVerdict::GameReady =>
            {
                anchor_report.verdict = CollectionItemVerdict::AwaitingReview
            }
            AssetGeometryVerdictV1::GameReady | AssetGeometryVerdictV1::AwaitingReview => {}
        }
        anchor_report.geometry = Some(geometry);
    }
    let anchor_report_path = work_dir.join("collection-anchor-quality-report.json");
    write_json_atomic(&anchor_report_path, &anchor_report)?;
    if anchor_report.verdict != CollectionItemVerdict::GameReady {
        return Err(CollectionError::Invalid(format!(
            "collection anchor failed {}: {}; report: {}",
            COLLECTION_ANCHOR_PROFILE,
            anchor_report.reasons.join(", "),
            anchor_report_path.display()
        )));
    }
    fs::create_dir_all(&collection_dir)?;
    let anchor_path = collection_dir.join("anchor.png");
    fs::copy(&normalized, &anchor_path)?;
    let medoid_path = collection_dir.join("medoid.png");
    fs::copy(&normalized, &medoid_path)?;
    let anchor_sha256 = hash_file(&anchor_path)?;
    let medoid_sha256 = hash_file(&medoid_path)?;
    let signature = image_signature(&image);
    let lock = CollectionLockV1 {
        schema_version: "1".into(),
        id: spec.id.clone(),
        name: spec.name,
        revision: revision.clone(),
        asset_kind: spec.asset_kind,
        provider_id: provider_id.into(),
        profile_id: profile_id.into(),
        image_model,
        style_revision: style.revision,
        style_board_sha256: style.board_sha256,
        prompt: spec.prompt,
        materials: spec.materials,
        scale: spec.scale,
        perspective: spec.perspective,
        grounding,
        framing_profile,
        canvas_size: canvas,
        anchor_path: anchor_path.clone(),
        anchor_sha256: anchor_sha256.clone(),
        medoid_path: medoid_path.clone(),
        medoid_sha256,
        baseline: baseline_from_signature(&signature, canvas),
        outlier_profile: COLLECTION_CONSISTENCY_PROFILE.into(),
        license: spec.license,
        created_at: Utc::now(),
    };
    write_json_atomic(&lock_path, &lock)?;
    project
        .current_collection_revisions
        .insert(spec.id.clone(), revision.clone());
    write_json_atomic(&project_root.join(FORGE_PROJECT_FILE), &project)?;
    Ok(CollectionBuildOutput {
        collection_lock_path: lock_path,
        anchor_path,
        id: spec.id,
        revision,
        anchor_sha256,
        anchor_report_path: Some(anchor_report_path),
        provider_requests,
    })
}

pub fn assess_collection_anchor(
    asset_kind: StaticAssetKind,
    signature: &ImageSignature,
) -> CollectionAnchorQualityReportV1 {
    let mut reasons = Vec::new();
    if !signature.alpha_present {
        reasons.push("anchor_alpha_missing".into());
    }
    if !signature.cell_boundary_safe {
        reasons.push("anchor_foreground_clipped".into());
    }
    if signature.subject_count == 0 {
        reasons.push("anchor_subject_missing".into());
    } else if signature.subject_count > 1 {
        reasons.push("anchor_multiple_subjects".into());
    }
    if signature.detached_component_ratio > 0.08 {
        reasons.push("anchor_detached_objects".into());
    }
    let platform_suspected = matches!(asset_kind, StaticAssetKind::IconSet)
        && signature.bottom_band_span_ratio >= 0.88
        && signature.bottom_band_area_share >= 0.24
        && signature.foreground_area_ratio >= 0.38
        && signature.platform_overhang_ratio >= 1.45;
    if platform_suspected {
        reasons.push("anchor_decorative_base".into());
    }
    if asset_kind == StaticAssetKind::PortraitSet
        && (signature.foreground_width_ratio < 0.35
            || signature.foreground_height_ratio < 0.55
            || signature.foreground_area_ratio < 0.22)
    {
        reasons.push("anchor_portrait_framing_invalid".into());
    }
    CollectionAnchorQualityReportV1 {
        schema_version: "1".into(),
        profile: COLLECTION_ANCHOR_PROFILE.into(),
        asset_kind,
        verdict: if reasons.is_empty() {
            CollectionItemVerdict::GameReady
        } else {
            CollectionItemVerdict::Blocked
        },
        subject_count: signature.subject_count,
        detached_component_ratio: signature.detached_component_ratio,
        bottom_band_span_ratio: signature.bottom_band_span_ratio,
        bottom_band_area_share: signature.bottom_band_area_share,
        platform_overhang_ratio: signature.platform_overhang_ratio,
        foreground_area_ratio: signature.foreground_area_ratio,
        foreground_width_ratio: signature.foreground_width_ratio,
        foreground_height_ratio: signature.foreground_height_ratio,
        geometry: None,
        reasons,
    }
}

pub fn collection_lock_path(project_root: &Path, id: &str, revision: &str) -> PathBuf {
    project_root
        .join(".forge/collections")
        .join(id)
        .join(revision)
        .join(COLLECTION_LOCK_FILE)
}

pub fn list_collection_locks(
    project_root: &Path,
    id: Option<&str>,
) -> Result<Vec<CollectionLockV1>, CollectionError> {
    let root = project_root.join(".forge/collections");
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut locks = Vec::new();
    for collection in fs::read_dir(root)? {
        let collection = collection?;
        if !collection.file_type()?.is_dir() {
            continue;
        }
        let collection_id = collection.file_name().to_string_lossy().to_string();
        if id.is_some_and(|expected| expected != collection_id) {
            continue;
        }
        for revision in fs::read_dir(collection.path())? {
            let revision = revision?;
            let path = revision.path().join(COLLECTION_LOCK_FILE);
            if path.is_file() {
                locks.push(read_collection_lock(&path)?);
            }
        }
    }
    locks.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then(left.created_at.cmp(&right.created_at))
    });
    Ok(locks)
}

pub fn read_collection_lock(path: &Path) -> Result<CollectionLockV1, CollectionError> {
    let lock: CollectionLockV1 = serde_json::from_slice(&fs::read(path)?)?;
    if lock.schema_version != "1"
        || !matches!(
            lock.outlier_profile.as_str(),
            COLLECTION_CONSISTENCY_PROFILE | LEGACY_COLLECTION_CONSISTENCY_PROFILE
        )
    {
        return Err(CollectionError::Invalid(
            "unsupported CollectionLock version or profile".into(),
        ));
    }
    for (label, path, expected) in [
        ("anchor", &lock.anchor_path, &lock.anchor_sha256),
        ("medoid", &lock.medoid_path, &lock.medoid_sha256),
    ] {
        if !path.is_file() || hash_file(path)? != *expected {
            return Err(CollectionError::Invalid(format!(
                "collection {label} changed after the revision was locked"
            )));
        }
    }
    Ok(lock)
}

pub fn assess_collection(
    lock: &CollectionLockV1,
    items: &[(String, u8, PathBuf)],
) -> Result<CollectionConsistencyReportV1, CollectionError> {
    assess_collection_complete(
        lock,
        &items
            .iter()
            .map(|(id, attempt, path)| CollectionAssessmentItemV1 {
                id: id.clone(),
                attempt: *attempt,
                path: Some(path.clone()),
                base_verdict: CollectionItemVerdict::GameReady,
                base_reasons: Vec::new(),
            })
            .collect::<Vec<_>>(),
    )
}

pub fn assess_collection_complete(
    lock: &CollectionLockV1,
    items: &[CollectionAssessmentItemV1],
) -> Result<CollectionConsistencyReportV1, CollectionError> {
    if items.is_empty() {
        return Err(CollectionError::Invalid(
            "collection consistency requires at least one item".into(),
        ));
    }
    let anchor = image_signature(&image::open(&lock.anchor_path)?.to_rgba8());
    let mut comparable = Vec::<(usize, ImageSignature, String)>::new();
    for (index, item) in items.iter().enumerate() {
        if let Some(path) = &item.path {
            let image = image::open(path)?.to_rgba8();
            comparable.push((index, image_signature(&image), hash_file(path)?));
        }
    }
    let signatures = comparable
        .iter()
        .map(|(_, signature, _)| signature.clone())
        .collect::<Vec<_>>();
    let matrix = pairwise_matrix(&signatures);
    let medoid_comparable_index = (0..comparable.len())
        .min_by(|left, right| {
            mean_without_diagonal(&matrix[*left], *left)
                .total_cmp(&mean_without_diagonal(&matrix[*right], *right))
                .reverse()
        })
        .unwrap_or(0);
    let medoid = comparable
        .get(medoid_comparable_index)
        .map(|(_, signature, _)| signature)
        .unwrap_or(&anchor);
    let mut reports = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let Some((_, signature, _)) = comparable
            .iter()
            .find(|(declared_index, _, _)| *declared_index == index)
        else {
            let mut reasons = item.base_reasons.clone();
            if !reasons
                .iter()
                .any(|reason| reason == "collection_item_unavailable")
            {
                reasons.push("collection_item_unavailable".into());
            }
            reports.push(CollectionItemConsistencyV1 {
                id: item.id.clone(),
                attempt: item.attempt.max(1),
                comparable: false,
                anchor_similarity: 0.0,
                medoid_similarity: 0.0,
                collection_score: 0.0,
                shape_similarity: None,
                framing_similarity: None,
                verdict: CollectionItemVerdict::Blocked,
                reasons,
            });
            continue;
        };
        let anchor_similarity = signature_similarity(signature, &anchor, lock.canvas_size);
        let medoid_similarity = signature_similarity(signature, medoid, lock.canvas_size);
        let score = 0.45 * anchor_similarity + 0.55 * medoid_similarity;
        let shape_similarity = geometry_similarity(signature, medoid);
        let framing_similarity = (lock.asset_kind == StaticAssetKind::PortraitSet)
            .then(|| portrait_framing_similarity(signature, &anchor));
        let hard = !signature.alpha_present
            || !signature.cell_boundary_safe
            || signature.subject_count != 1
            || signature.width != lock.canvas_size
            || signature.height != lock.canvas_size
            || item.base_verdict == CollectionItemVerdict::Blocked;
        let mut verdict = if hard {
            CollectionItemVerdict::Blocked
        } else if score < 0.55 {
            CollectionItemVerdict::Regenerate
        } else if score < 0.70 {
            CollectionItemVerdict::AwaitingReview
        } else {
            CollectionItemVerdict::GameReady
        };
        verdict = escalate_collection_verdict(verdict, item.base_verdict);
        let mut reasons = item.base_reasons.clone();
        if hard {
            reasons.push("collection_hard_gate".into());
        }
        if score < 0.55 {
            reasons.push("collection_outlier".into());
        } else if score < 0.70 {
            reasons.push("collection_similarity_gray".into());
        }
        if let Some(framing) = framing_similarity {
            let framing_verdict = portrait_framing_verdict(signature, &anchor, framing);
            verdict = escalate_collection_verdict(verdict, framing_verdict);
            if framing_verdict != CollectionItemVerdict::GameReady {
                reasons.push("portrait_framing_drift".into());
            }
        }
        reasons.sort();
        reasons.dedup();
        reports.push(CollectionItemConsistencyV1 {
            id: item.id.clone(),
            attempt: item.attempt.max(1),
            comparable: true,
            anchor_similarity,
            medoid_similarity,
            collection_score: score,
            shape_similarity: Some(shape_similarity),
            framing_similarity,
            verdict,
            reasons,
        });
    }
    let verdict = aggregate_verdict(&reports);
    let pairwise_mean_similarity = if comparable.len() < 2 {
        1.0
    } else {
        let mut total = 0.0;
        let mut count = 0usize;
        for (left, row) in matrix.iter().enumerate().take(comparable.len()) {
            for value in row.iter().take(comparable.len()).skip(left + 1) {
                total += value;
                count += 1;
            }
        }
        total / count as f32
    };
    Ok(CollectionConsistencyReportV1 {
        schema_version: "1".into(),
        profile: COLLECTION_CONSISTENCY_PROFILE.into(),
        collection_id: lock.id.clone(),
        collection_revision: lock.revision.clone(),
        medoid_item_id: comparable
            .get(medoid_comparable_index)
            .map(|(index, _, _)| items[*index].id.clone())
            .unwrap_or_else(|| "__anchor__".into()),
        medoid_sha256: comparable
            .get(medoid_comparable_index)
            .map(|(_, _, hash)| hash.clone())
            .unwrap_or_else(|| lock.anchor_sha256.clone()),
        pairwise_mean_similarity,
        verdict,
        items: reports,
    })
}

fn escalate_collection_verdict(
    left: CollectionItemVerdict,
    right: CollectionItemVerdict,
) -> CollectionItemVerdict {
    use CollectionItemVerdict::{AwaitingReview, Blocked, GameReady, Regenerate};
    match (left, right) {
        (Blocked, _) | (_, Blocked) => Blocked,
        (Regenerate, _) | (_, Regenerate) => Regenerate,
        (AwaitingReview, _) | (_, AwaitingReview) => AwaitingReview,
        _ => GameReady,
    }
}

fn geometry_similarity(left: &ImageSignature, right: &ImageSignature) -> f32 {
    let width = ratio_similarity(left.foreground_width_ratio, right.foreground_width_ratio);
    let height = ratio_similarity(left.foreground_height_ratio, right.foreground_height_ratio);
    let area = ratio_similarity(left.foreground_area_ratio, right.foreground_area_ratio);
    let occupancy =
        occupancy_profile_similarity(&left.row_occupancy_profile, &right.row_occupancy_profile);
    (0.25 * width + 0.20 * height + 0.20 * area + 0.35 * occupancy).clamp(0.0, 1.0)
}

fn portrait_framing_similarity(candidate: &ImageSignature, anchor: &ImageSignature) -> f32 {
    let centroid =
        (1.0 - (candidate.centroid_y_ratio - anchor.centroid_y_ratio).abs() / 0.25).clamp(0.0, 1.0);
    (0.30
        * ratio_similarity(
            candidate.foreground_width_ratio,
            anchor.foreground_width_ratio,
        )
        + 0.15
            * ratio_similarity(
                candidate.foreground_height_ratio,
                anchor.foreground_height_ratio,
            )
        + 0.20
            * ratio_similarity(
                candidate.foreground_area_ratio,
                anchor.foreground_area_ratio,
            )
        + 0.25
            * occupancy_profile_similarity(
                &candidate.row_occupancy_profile,
                &anchor.row_occupancy_profile,
            )
        + 0.10 * centroid)
        .clamp(0.0, 1.0)
}

fn portrait_framing_verdict(
    candidate: &ImageSignature,
    anchor: &ImageSignature,
    score: f32,
) -> CollectionItemVerdict {
    let width = ratio(
        candidate.foreground_width_ratio,
        anchor.foreground_width_ratio,
    );
    let height = ratio(
        candidate.foreground_height_ratio,
        anchor.foreground_height_ratio,
    );
    let area = ratio(
        candidate.foreground_area_ratio,
        anchor.foreground_area_ratio,
    );
    let centroid = (candidate.centroid_y_ratio - anchor.centroid_y_ratio).abs();
    if !(0.70..=1.30).contains(&width)
        || !(0.75..=1.25).contains(&height)
        || !(0.55..=1.45).contains(&area)
        || centroid > 0.12
        || score < 0.55
    {
        CollectionItemVerdict::Blocked
    } else if !(0.80..=1.20).contains(&width)
        || !(0.82..=1.18).contains(&height)
        || !(0.68..=1.32).contains(&area)
        || centroid > 0.08
        || score < 0.68
    {
        CollectionItemVerdict::Regenerate
    } else if !(0.88..=1.12).contains(&width)
        || !(0.90..=1.10).contains(&height)
        || !(0.80..=1.20).contains(&area)
        || centroid > 0.05
        || score < 0.78
    {
        CollectionItemVerdict::AwaitingReview
    } else {
        CollectionItemVerdict::GameReady
    }
}

/// Deterministically crops a clearly full-body/narrow Portrait toward the
/// Collection anchor's bust aspect before the ordinary consistency gates run.
/// Matching candidates are returned unchanged via `None`.
pub fn reframe_portrait_to_anchor(candidate: &RgbaImage, anchor: &RgbaImage) -> Option<RgbaImage> {
    let candidate_bounds = portrait_alpha_bounds(candidate)?;
    let anchor_bounds = portrait_alpha_bounds(anchor)?;
    let candidate_width = candidate_bounds.2 - candidate_bounds.0 + 1;
    let candidate_height = candidate_bounds.3 - candidate_bounds.1 + 1;
    let anchor_width = anchor_bounds.2 - anchor_bounds.0 + 1;
    let anchor_height = anchor_bounds.3 - anchor_bounds.1 + 1;
    let target_aspect = anchor_width as f32 / anchor_height.max(1) as f32;
    let candidate_aspect = candidate_width as f32 / candidate_height.max(1) as f32;
    if target_aspect <= f32::EPSILON || candidate_aspect >= target_aspect * 0.95 {
        return None;
    }
    let crop_height = (candidate_width as f32 / target_aspect)
        .round()
        .max(candidate_height as f32 * 0.35)
        .min(candidate_height as f32 * 0.82) as u32;
    if crop_height >= candidate_height {
        return None;
    }
    let cropped = image::imageops::crop_imm(
        candidate,
        candidate_bounds.0,
        candidate_bounds.1,
        candidate_width,
        crop_height,
    )
    .to_image();
    let canvas_size = candidate.width().min(candidate.height());
    let usable = (canvas_size as f32 * 0.82).round().max(1.0) as u32;
    let scale = (usable as f32 / cropped.width().max(1) as f32)
        .min(usable as f32 / cropped.height().max(1) as f32);
    let width = (cropped.width() as f32 * scale).round().max(1.0) as u32;
    let height = (cropped.height() as f32 * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(&cropped, width, height, FilterType::Lanczos3);
    let mut canvas =
        ImageBuffer::from_pixel(candidate.width(), candidate.height(), Rgba([0, 0, 0, 0]));
    let x = candidate.width().saturating_sub(width) / 2;
    let y = candidate.height().saturating_sub(height) / 2;
    image::imageops::overlay(&mut canvas, &resized, x.into(), y.into());
    Some(canvas)
}

fn portrait_alpha_bounds(image: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let mut left = image.width();
    let mut top = image.height();
    let mut right = 0;
    let mut bottom = 0;
    let mut found = false;
    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] <= 16 {
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

fn collection_revision(
    spec: &CollectionSpecV1,
    style: &StyleLockV1,
    provider_id: &str,
    profile_id: &str,
    anchor_input_sha: &str,
) -> Result<String, CollectionError> {
    let mut normalized = serde_json::to_value(spec)?;
    if let Some(object) = normalized.as_object_mut() {
        object.insert("anchorImage".into(), anchor_input_sha.into());
    }
    let bytes = serde_json::to_vec(&serde_json::json!({
        "spec": normalized,
        "styleRevision": style.revision,
        "styleBoardSha256": style.board_sha256,
        "providerId": provider_id,
        "profileId": profile_id,
        "profile": COLLECTION_CONSISTENCY_PROFILE,
    }))?;
    Ok(format!("r-{:x}", Sha256::digest(bytes))[..18].to_string())
}

fn baseline_from_signature(signature: &ImageSignature, canvas: u32) -> CollectionBaselineV1 {
    CollectionBaselineV1 {
        palette: signature.palette.clone(),
        edge_density: signature.edge_density,
        foreground_scale: signature.foreground_scale,
        perceptual_hash: format!("{:016x}", signature.perceptual_hash),
        anchor_x: signature.anchor_x,
        anchor_y: signature.anchor_y,
        canvas_size: canvas,
    }
}

fn collection_anchor_prompt(spec: &CollectionSpecV1, style: &StyleLockV1) -> String {
    let framing = match spec.effective_portrait_framing_profile() {
        Some(PortraitFramingProfileV1::DialogueBust) => format!(
            " Portrait framing contract: {}. This is a dialogue bust; legs are intentionally outside the asset contract.",
            PortraitFramingProfileV1::DialogueBust.as_str()
        ),
        Some(PortraitFramingProfileV1::FullBody) => format!(
            " Portrait framing contract: {}. Show the complete character from the top of the head through both feet, every limb visible and wholly inside the canvas.",
            PortraitFramingProfileV1::FullBody.as_str()
        ),
        None => String::new(),
    };
    format!(
        "Create exactly one canonical anchor for a cohesive {} collection: {}. Collection direction: {}. Materials: {}. Scale: {}. Preserve the style reference palette, line weight, lighting, and camera. Perspective: {}.{} Centered, full object, solid chroma green background, no text, border, UI, scene, or extra object.",
        spec.asset_kind.as_str(),
        spec.name,
        spec.prompt,
        if spec.materials.is_empty() { "unspecified".into() } else { spec.materials.join(", ") },
        spec.scale,
        if spec.perspective.trim().is_empty() { style.perspective.clone() } else { spec.perspective.clone() },
        framing,
    )
}

fn grounding_for(spec: &CollectionSpecV1) -> CollectionGrounding {
    spec.grounding.clone().unwrap_or(match spec.asset_kind {
        StaticAssetKind::PortraitSet
            if spec.effective_portrait_framing_profile()
                == Some(PortraitFramingProfileV1::FullBody) =>
        {
            CollectionGrounding::Feet
        }
        StaticAssetKind::IconSet | StaticAssetKind::PortraitSet => CollectionGrounding::Center,
        StaticAssetKind::PropSet | StaticAssetKind::EquipmentSet | StaticAssetKind::DecalSet => {
            CollectionGrounding::Surface
        }
    })
}

fn canvas_for_kind(kind: StaticAssetKind, style: &StyleLockV1) -> u32 {
    match kind {
        StaticAssetKind::IconSet | StaticAssetKind::PortraitSet => style.icon_canvas_size,
        StaticAssetKind::PropSet | StaticAssetKind::EquipmentSet | StaticAssetKind::DecalSet => {
            style.prop_canvas_size
        }
    }
}

fn pairwise_matrix(signatures: &[ImageSignature]) -> Vec<Vec<f32>> {
    signatures
        .iter()
        .map(|left| {
            signatures
                .iter()
                .map(|right| signature_similarity(left, right, left.width.max(right.width)))
                .collect()
        })
        .collect()
}

fn mean_without_diagonal(values: &[f32], index: usize) -> f32 {
    if values.len() <= 1 {
        return 1.0;
    }
    values
        .iter()
        .enumerate()
        .filter(|(candidate, _)| *candidate != index)
        .map(|(_, value)| value)
        .sum::<f32>()
        / (values.len() - 1) as f32
}

fn signature_similarity(left: &ImageSignature, right: &ImageSignature, canvas: u32) -> f32 {
    let palette = palette_similarity(&left.palette, &right.palette);
    let edge = ratio_similarity(left.edge_density, right.edge_density);
    let scale = ratio_similarity(left.foreground_scale, right.foreground_scale);
    let geometry = geometry_similarity(left, right);
    let perceptual =
        1.0 - (left.perceptual_hash ^ right.perceptual_hash).count_ones() as f32 / 64.0;
    let drift = ((left.anchor_x - right.anchor_x).powi(2)
        + (left.anchor_y - right.anchor_y).powi(2))
    .sqrt();
    let anchor = (1.0 - drift / (canvas.max(1) as f32 * 0.20)).clamp(0.0, 1.0);
    (0.35 * palette
        + 0.05 * edge
        + 0.15 * scale
        + 0.15 * perceptual
        + 0.10 * anchor
        + 0.20 * geometry)
        .clamp(0.0, 1.0)
}

fn palette_similarity(left: &[PaletteColor], right: &[PaletteColor]) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let weighted = left
        .iter()
        .map(|entry| {
            let color = parse_color(&entry.color);
            let closest = right
                .iter()
                .map(|candidate| color_distance(color, parse_color(&candidate.color)))
                .fold(f32::INFINITY, f32::min);
            entry.weight * (1.0 - closest / 441.7).clamp(0.0, 1.0)
        })
        .sum::<f32>();
    let total = left.iter().map(|entry| entry.weight).sum::<f32>();
    if total <= f32::EPSILON {
        0.0
    } else {
        (weighted / total).clamp(0.0, 1.0)
    }
}

fn parse_color(value: &str) -> [f32; 3] {
    let value = value.trim_start_matches('#');
    if value.len() != 6 {
        return [0.0; 3];
    }
    [0, 2, 4]
        .map(|offset| u8::from_str_radix(&value[offset..offset + 2], 16).unwrap_or_default() as f32)
}

fn color_distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    ((left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2))
        .sqrt()
}

fn ratio_similarity(left: f32, right: f32) -> f32 {
    if left <= f32::EPSILON || right <= f32::EPSILON {
        return f32::from(left <= f32::EPSILON && right <= f32::EPSILON);
    }
    (left.min(right) / left.max(right)).clamp(0.0, 1.0)
}

fn ratio(left: f32, right: f32) -> f32 {
    if right <= f32::EPSILON {
        return if left <= f32::EPSILON {
            1.0
        } else {
            f32::INFINITY
        };
    }
    left / right
}

fn default_true() -> bool {
    true
}

fn aggregate_verdict(items: &[CollectionItemConsistencyV1]) -> CollectionItemVerdict {
    if items
        .iter()
        .any(|item| item.verdict == CollectionItemVerdict::Blocked)
    {
        CollectionItemVerdict::Blocked
    } else if items
        .iter()
        .any(|item| item.verdict == CollectionItemVerdict::Regenerate)
    {
        CollectionItemVerdict::Regenerate
    } else if items
        .iter()
        .any(|item| item.verdict == CollectionItemVerdict::AwaitingReview)
    {
        CollectionItemVerdict::AwaitingReview
    } else {
        CollectionItemVerdict::GameReady
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), CollectionError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("json.{}.tmp", Uuid::new_v4()));
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn default_scale() -> String {
    "consistent".into()
}

fn default_perspective() -> String {
    "inherit_style".into()
}

fn default_license() -> String {
    "MIT".into()
}

fn default_decal_blend() -> DecalBlendV1 {
    DecalBlendV1::Mix
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba, RgbaImage};

    fn square(path: &Path, color: [u8; 4], offset: u32) {
        let mut image = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 24..104 {
            for x in (24 + offset)..(104 + offset).min(127) {
                image.put_pixel(x, y, Rgba(color));
            }
        }
        image.save(path).unwrap();
    }

    #[test]
    fn medoid_and_outlier_are_deterministic() {
        let temp = tempfile::tempdir().unwrap();
        let anchor = temp.path().join("anchor.png");
        let near = temp.path().join("near.png");
        let far = temp.path().join("far.png");
        square(&anchor, [180, 60, 40, 255], 0);
        square(&near, [184, 64, 44, 255], 1);
        square(&far, [20, 220, 220, 255], 20);
        let anchor_image: RgbaImage = image::open(&anchor).unwrap().to_rgba8();
        let signature = image_signature(&anchor_image);
        let lock = CollectionLockV1 {
            schema_version: "1".into(),
            id: "weapons".into(),
            name: "Weapons".into(),
            revision: "r-test".into(),
            asset_kind: StaticAssetKind::IconSet,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            image_model: None,
            style_revision: "style-test".into(),
            style_board_sha256: "a".repeat(64),
            prompt: "weapons".into(),
            materials: vec!["iron".into()],
            scale: "consistent".into(),
            perspective: "front".into(),
            grounding: CollectionGrounding::Center,
            framing_profile: None,
            canvas_size: 128,
            anchor_path: anchor.clone(),
            anchor_sha256: hash_file(&anchor).unwrap(),
            medoid_path: anchor.clone(),
            medoid_sha256: hash_file(&anchor).unwrap(),
            baseline: baseline_from_signature(&signature, 128),
            outlier_profile: COLLECTION_CONSISTENCY_PROFILE.into(),
            license: "MIT".into(),
            created_at: Utc::now(),
        };
        let report = assess_collection(
            &lock,
            &[
                ("anchor".into(), 1, anchor),
                ("near".into(), 1, near),
                ("far".into(), 1, far),
            ],
        )
        .unwrap();
        assert_eq!(report.medoid_item_id, "near");
        assert!(report.items[2].collection_score < report.items[1].collection_score);
    }

    fn lock_for(kind: StaticAssetKind, anchor: &Path) -> CollectionLockV1 {
        let signature = image_signature(&image::open(anchor).unwrap().to_rgba8());
        CollectionLockV1 {
            schema_version: "1".into(),
            id: "test-collection".into(),
            name: "Test Collection".into(),
            revision: "r-test".into(),
            asset_kind: kind,
            provider_id: "fixture".into(),
            profile_id: "default".into(),
            image_model: None,
            style_revision: "style-test".into(),
            style_board_sha256: "a".repeat(64),
            prompt: "test".into(),
            materials: vec![],
            scale: "consistent".into(),
            perspective: "front".into(),
            grounding: CollectionGrounding::Center,
            framing_profile: None,
            canvas_size: 128,
            anchor_path: anchor.to_path_buf(),
            anchor_sha256: hash_file(anchor).unwrap(),
            medoid_path: anchor.to_path_buf(),
            medoid_sha256: hash_file(anchor).unwrap(),
            baseline: baseline_from_signature(&signature, 128),
            outlier_profile: COLLECTION_CONSISTENCY_PROFILE.into(),
            license: "MIT".into(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn anchor_gate_blocks_multiple_subjects_and_decorative_platforms() {
        let mut valid = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 28..100 {
            for x in 42..86 {
                valid.put_pixel(x, y, Rgba([120, 80, 40, 255]));
            }
        }
        assert_eq!(
            assess_collection_anchor(StaticAssetKind::IconSet, &image_signature(&valid)).verdict,
            CollectionItemVerdict::GameReady
        );

        let mut multiple = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 35..90 {
            for x in 18..48 {
                multiple.put_pixel(x, y, Rgba([180, 70, 40, 255]));
            }
            for x in 80..110 {
                multiple.put_pixel(x, y, Rgba([40, 110, 180, 255]));
            }
        }
        let report =
            assess_collection_anchor(StaticAssetKind::IconSet, &image_signature(&multiple));
        assert_eq!(report.verdict, CollectionItemVerdict::Blocked);
        assert!(report.reasons.contains(&"anchor_multiple_subjects".into()));

        let mut platform = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 18..78 {
            for x in 46..82 {
                platform.put_pixel(x, y, Rgba([120, 80, 40, 255]));
            }
        }
        for y in 72..88 {
            for x in 61..67 {
                platform.put_pixel(x, y, Rgba([120, 80, 40, 255]));
            }
        }
        for y in 86..108 {
            for x in 14..114 {
                platform.put_pixel(x, y, Rgba([80, 120, 50, 255]));
            }
        }
        let report =
            assess_collection_anchor(StaticAssetKind::IconSet, &image_signature(&platform));
        assert_eq!(report.verdict, CollectionItemVerdict::Blocked);
        assert!(report.reasons.contains(&"anchor_decorative_base".into()));
    }

    #[test]
    fn complete_report_keeps_unavailable_items_and_blocks_the_collection() {
        let temp = tempfile::tempdir().unwrap();
        let anchor = temp.path().join("anchor.png");
        square(&anchor, [180, 60, 40, 255], 0);
        let lock = lock_for(StaticAssetKind::IconSet, &anchor);
        let report = assess_collection_complete(
            &lock,
            &[
                CollectionAssessmentItemV1 {
                    id: "ready".into(),
                    attempt: 1,
                    path: Some(anchor),
                    base_verdict: CollectionItemVerdict::GameReady,
                    base_reasons: vec![],
                },
                CollectionAssessmentItemV1 {
                    id: "failed".into(),
                    attempt: 2,
                    path: None,
                    base_verdict: CollectionItemVerdict::Regenerate,
                    base_reasons: vec!["edge_density_advisory".into()],
                },
            ],
        )
        .unwrap();
        assert_eq!(report.items.len(), 2);
        assert_eq!(report.verdict, CollectionItemVerdict::Blocked);
        assert!(!report.items[1].comparable);
    }

    #[test]
    fn portrait_framing_blocks_a_full_body_shape_mixed_with_a_bust_anchor() {
        let temp = tempfile::tempdir().unwrap();
        let anchor = temp.path().join("portrait-anchor.png");
        let candidate = temp.path().join("full-body.png");
        let mut bust = ImageBuffer::from_pixel(128, 128, Rgba::<u8>([0, 0, 0, 0]));
        for y in 20..112 {
            for x in 20..108 {
                bust.put_pixel(x, y, Rgba([120, 80, 40, 255]));
            }
        }
        bust.save(&anchor).unwrap();
        let mut full_body = ImageBuffer::from_pixel(128, 128, Rgba::<u8>([0, 0, 0, 0]));
        for y in 10..118 {
            for x in 48..80 {
                full_body.put_pixel(x, y, Rgba([120, 80, 40, 255]));
            }
        }
        full_body.save(&candidate).unwrap();
        let lock = lock_for(StaticAssetKind::PortraitSet, &anchor);
        let report = assess_collection_complete(
            &lock,
            &[CollectionAssessmentItemV1 {
                id: "happy".into(),
                attempt: 1,
                path: Some(candidate),
                base_verdict: CollectionItemVerdict::GameReady,
                base_reasons: vec![],
            }],
        )
        .unwrap();
        assert_eq!(report.verdict, CollectionItemVerdict::Blocked);
        assert!(report.items[0]
            .reasons
            .contains(&"portrait_framing_drift".into()));
    }

    #[test]
    fn deterministic_portrait_reframe_crops_only_a_tall_full_body_candidate() {
        let mut anchor = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 24..104 {
            for x in 24..104 {
                anchor.put_pixel(x, y, Rgba([120, 80, 40, 255]));
            }
        }
        let mut full_body = ImageBuffer::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
        for y in 10..118 {
            for x in 44..84 {
                full_body.put_pixel(x, y, Rgba([120, 80, 40, 255]));
            }
        }
        let reframed = reframe_portrait_to_anchor(&full_body, &anchor).unwrap();
        let signature = image_signature(&reframed);
        assert!(signature.foreground_width_ratio >= 0.75);
        assert!(signature.foreground_height_ratio <= 0.84);
        assert!(reframe_portrait_to_anchor(&anchor, &anchor).is_none());
    }

    fn portrait_spec_value(schema_version: &str) -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": schema_version,
            "kind": "portrait_set",
            "id": "hero-portraits",
            "name": "Hero Portraits",
            "collection": { "id": "portraits", "revision": "r-collection" },
            "subject": { "id": "hero", "revision": "r-subject" },
            "expressions": [
                { "id": "neutral", "name": "Neutral", "prompt": "neutral expression" },
                { "id": "happy", "name": "Happy", "prompt": "happy expression" },
                { "id": "angry", "name": "Angry", "prompt": "angry expression" },
                { "id": "hurt", "name": "Hurt", "prompt": "hurt expression" },
                { "id": "surprised", "name": "Surprised", "prompt": "surprised expression" }
            ],
            "license": "MIT"
        })
    }

    #[test]
    fn portrait_v1_resolves_to_legacy_dialogue_bust_profile() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("portrait-v1.json");
        fs::write(
            &path,
            serde_json::to_vec_pretty(&portrait_spec_value("1")).unwrap(),
        )
        .unwrap();
        let resolved = read_static_collection_spec(&path, StaticAssetKind::PortraitSet).unwrap();
        assert_eq!(resolved.asset.schema_version, "1");
        assert_eq!(
            resolved.framing_profile,
            Some(PortraitFramingProfileV1::DialogueBust)
        );
    }

    #[test]
    fn portrait_v2_requires_and_preserves_explicit_full_body_profile() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("portrait-v2.json");
        let mut value = portrait_spec_value("2");
        value["framingProfile"] = serde_json::json!("full_body@1.0.0");
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let resolved = read_static_collection_spec(&path, StaticAssetKind::PortraitSet).unwrap();
        assert_eq!(resolved.asset.schema_version, "2");
        assert_eq!(
            resolved.framing_profile,
            Some(PortraitFramingProfileV1::FullBody)
        );

        value.as_object_mut().unwrap().remove("framingProfile");
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let error = read_static_collection_spec(&path, StaticAssetKind::PortraitSet).unwrap_err();
        assert!(error.to_string().contains("framingProfile"));
    }

    #[test]
    fn portrait_v2_requires_neutral_first_for_the_portrait_base() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("portrait-v2-order.json");
        let mut value = portrait_spec_value("2");
        value["framingProfile"] = serde_json::json!("full_body@1.0.0");
        value["expressions"].as_array_mut().unwrap().swap(0, 1);
        fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
        let error = read_static_collection_spec(&path, StaticAssetKind::PortraitSet).unwrap_err();
        assert!(error
            .to_string()
            .contains("neutral as the first expression"));
    }

    #[test]
    fn legacy_portrait_lock_defaults_to_dialogue_bust_only_for_portraits() {
        let temp = tempfile::tempdir().unwrap();
        let anchor = temp.path().join("anchor.png");
        square(&anchor, [180, 60, 40, 255], 0);
        let portrait = lock_for(StaticAssetKind::PortraitSet, &anchor);
        let legacy_value = serde_json::to_value(&portrait).unwrap();
        assert!(legacy_value.get("framingProfile").is_none());
        let portrait: CollectionLockV1 = serde_json::from_value(legacy_value).unwrap();
        assert_eq!(
            portrait.effective_portrait_framing_profile(),
            Some(PortraitFramingProfileV1::DialogueBust)
        );
        let icon = lock_for(StaticAssetKind::IconSet, &anchor);
        assert_eq!(icon.effective_portrait_framing_profile(), None);
    }

    #[test]
    fn portrait_collection_profile_defaults_and_full_body_grounding_are_versioned() {
        let legacy: CollectionSpecV1 = serde_json::from_value(serde_json::json!({
            "schemaVersion": "1",
            "id": "portraits",
            "name": "Portraits",
            "assetKind": "portrait_set",
            "prompt": "cohesive portraits",
            "license": "MIT"
        }))
        .unwrap();
        assert_eq!(
            legacy.effective_portrait_framing_profile(),
            Some(PortraitFramingProfileV1::DialogueBust)
        );
        validate_collection_spec(&legacy).unwrap();

        let mut full_body = legacy.clone();
        full_body.framing_profile = Some(PortraitFramingProfileV1::FullBody);
        assert_eq!(grounding_for(&full_body), CollectionGrounding::Feet);
        validate_collection_spec(&full_body).unwrap();
        full_body.grounding = Some(CollectionGrounding::Center);
        assert!(validate_collection_spec(&full_body)
            .unwrap_err()
            .to_string()
            .contains("feet grounding"));
    }
}
