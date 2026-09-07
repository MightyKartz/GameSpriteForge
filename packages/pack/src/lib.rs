use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

const GSFPACK_SCHEMA: &str = include_str!("../../../schemas/gsfpack.schema.json");
const MANIFEST_SCHEMA: &str = include_str!("../../../schemas/manifest.schema.json");
const ATLAS_SCHEMA: &str = include_str!("../../../schemas/atlas.schema.json");
const QUALITY_REPORT_SCHEMA: &str = include_str!("../../../schemas/quality-report.schema.json");
const PORTRAIT_BASE_LOCK_SCHEMA: &str =
    include_str!("../../../schemas/portrait-base-lock.schema.json");
const PORTRAIT_CONSISTENCY_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/portrait-consistency-report.schema.json");
const PORTRAIT_BASE_APPROVAL_SCHEMA: &str =
    include_str!("../../../schemas/portrait-base-approval.schema.json");
const CHARACTER_SEMANTIC_QUALITY_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/character-semantic-quality-report.schema.json");
const CHARACTER_SILHOUETTE_TEMPORAL_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/character-silhouette-temporal-report.schema.json");
const CHARACTER_SILHOUETTE_TEMPORAL_REPORT_V1_SCHEMA: &str =
    include_str!("../../../schemas/character-silhouette-temporal-report-v1.schema.json");
const CHARACTER_ALPHA_REPAIR_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/character-alpha-repair-report.schema.json");
const CHARACTER_SCALE_LOCK_SCHEMA: &str =
    include_str!("../../../schemas/character-scale-lock.schema.json");
const HAND_EQUIPMENT_CONTACT_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/hand-equipment-contact-report.schema.json");
const CHARACTER_DIRECTION_LOCK_SCHEMA: &str =
    include_str!("../../../schemas/character-direction-lock.schema.json");
const CHARACTER_DIRECTION_POSE_LOCK_SCHEMA: &str =
    include_str!("../../../schemas/character-direction-pose-lock.schema.json");
const CHARACTER_DIRECTION_MOTION_LOCK_SCHEMA: &str =
    include_str!("../../../schemas/character-direction-motion-lock.schema.json");
const CHARACTER_DIRECTION_MOTION_APPROVAL_SCHEMA: &str =
    include_str!("../../../schemas/character-direction-motion-approval.schema.json");
const CHARACTER_DIRECTION_GRID_LOCK_SCHEMA: &str =
    include_str!("../../../schemas/character-direction-grid-lock.schema.json");
const CHARACTER_DIRECTION_GRID_APPROVAL_SCHEMA: &str =
    include_str!("../../../schemas/character-direction-grid-approval.schema.json");
const CHARACTER_DIRECTION_GRID_APPEARANCE_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/character-direction-grid-appearance-report.schema.json");
const GRID_KEYFRAME_ACTION_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/grid-keyframe-action-report.schema.json");
const CHARACTER_MOTION_SEMANTICS_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/character-motion-semantics-report.schema.json");
const CHARACTER_GAIT_CYCLE_REPORT_AGGREGATE_SCHEMA: &str =
    include_str!("../../../schemas/character-gait-cycle-report-aggregate.schema.json");
const SOURCE_CYCLE_SAMPLING_REPORT_SCHEMA: &str =
    include_str!("../../../schemas/source-cycle-sampling-report.schema.json");
const ANIMATION_HUMAN_REVIEW_SCHEMA: &str =
    include_str!("../../../schemas/animation-human-review.schema.json");

const REQUIRED_FILES: &[&str] = &[
    "forgepack.json",
    "previews/preview.gif",
    "assets/frames",
    "assets/sprite_sheet.png",
    "assets/atlas.json",
    "assets/manifest.json",
    "quality-report.json",
];

const LEGACY_SPRITE_SHEET_IMAGE: &str = "sprite_sheet.png";

#[derive(Debug, thiserror::Error)]
pub enum PackError {
    #[error("pack path does not exist: {0}")]
    MissingPack(PathBuf),
    #[error("pack path is not a directory: {0}")]
    NotDirectory(PathBuf),
    #[error("required pack file is missing: {0}")]
    MissingFile(String),
    #[error("invalid pack asset path: {0}")]
    InvalidAssetPath(String),
    #[error("pack contains no frame pngs")]
    NoFrames,
    #[error("forgepack asset path mismatch for {field}: expected {expected}, got {actual}")]
    AssetPathMismatch {
        field: String,
        expected: String,
        actual: String,
    },
    #[error("schema compile failed for {schema}: {message}")]
    SchemaCompile { schema: String, message: String },
    #[error("schema validation failed for {document}: {message}")]
    SchemaValidation { document: String, message: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub frame_count: usize,
    pub preview_gif: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackInspectSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub frame_count: usize,
    pub preview_gif: String,
    pub root: PathBuf,
    pub manifest_path: PathBuf,
    pub atlas_path: PathBuf,
    pub quality_report_path: PathBuf,
    pub default_animation: String,
    pub animations: Vec<PackAnimationSummary>,
    pub asset_type: String,
    pub items: Vec<PackItemSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackItemSummary {
    pub id: String,
    pub name: String,
    pub frame: usize,
    pub texture: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackAnimationSummary {
    pub name: String,
    pub frame_count: usize,
    pub fps: f32,
    #[serde(rename = "loop")]
    pub loop_animation: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForgePackJson {
    schema_version: String,
    #[serde(default)]
    asset_type: Option<String>,
    id: String,
    name: String,
    version: String,
    previews: PackPreviews,
    assets: PackAssets,
    #[serde(default)]
    items: Vec<PackItem>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackItem {
    id: String,
    texture: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackPreviews {
    #[serde(default)]
    gif: Option<String>,
    #[serde(default)]
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackAssets {
    frames: Option<String>,
    sprite_sheet: Option<String>,
    atlas: Option<String>,
    manifest: String,
    godot_helper: Option<String>,
    quality_report: String,
    animation_human_review: Option<String>,
    consistency_report: Option<String>,
    character_semantic_quality_report: Option<String>,
    hand_equipment_contact_report: Option<String>,
    character_silhouette_temporal_report: Option<String>,
    character_silhouette_temporal_source_report: Option<String>,
    character_motion_semantics_report: Option<String>,
    character_gait_cycle_report: Option<String>,
    animation_sheet_report: Option<String>,
    character_alpha_repair_report: Option<String>,
    character_scale_lock: Option<String>,
    character_direction_lock: Option<String>,
    character_direction_pose_lock: Option<String>,
    character_direction_motion_lock: Option<String>,
    character_direction_motion_approval: Option<String>,
    direction_motion_provider_manifest: Option<String>,
    character_direction_grid_lock: Option<String>,
    character_direction_grid_approval: Option<String>,
    direction_grid_appearance_report: Option<String>,
    grid_keyframe_provider_manifest: Option<String>,
    #[serde(default)]
    grid_keyframe_action_reports: Vec<String>,
    keyframe_loop_trim_report: Option<String>,
    portrait_base_lock: Option<String>,
    portrait_base_approval: Option<String>,
    portrait_consistency_report: Option<String>,
    terrain_manifest: Option<String>,
    building_manifest: Option<String>,
    map_manifest: Option<String>,
    map_layout: Option<String>,
    validation_report: Option<String>,
    atlas_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImportedPack {
    pub summary: PackSummary,
    pub root: PathBuf,
    pub frame_paths: Vec<PathBuf>,
    pub forgepack: serde_json::Value,
    pub manifest: serde_json::Value,
    pub atlas: serde_json::Value,
    pub quality_report: serde_json::Value,
}

pub fn validate_pack_layout(pack_path: &Path) -> Result<(), PackError> {
    require_pack_root_directory(pack_path)?;

    require_regular_pack_file(pack_path, "forgepack.json")?;
    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    if metadata.schema_version == "3.0.0" {
        return validate_world_pack_layout(pack_path, &metadata);
    }

    for relative in REQUIRED_FILES {
        if *relative == "assets/frames" {
            require_regular_pack_directory(pack_path, relative)?;
        } else {
            require_regular_pack_file(pack_path, relative)?;
        }
    }

    expect_asset_path(
        "previews.gif",
        "previews/preview.gif",
        metadata.previews.gif.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.frames",
        "assets/frames",
        metadata.assets.frames.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.spriteSheet",
        "assets/sprite_sheet.png",
        metadata.assets.sprite_sheet.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.atlas",
        "assets/atlas.json",
        metadata.assets.atlas.as_deref().unwrap_or_default(),
    )?;
    expect_asset_path(
        "assets.manifest",
        "assets/manifest.json",
        &metadata.assets.manifest,
    )?;
    if let Some(godot_helper) = metadata.assets.godot_helper.as_deref() {
        expect_asset_path(
            "assets.godotHelper",
            "assets/godot_import.json",
            godot_helper,
        )?;
        require_regular_pack_file(pack_path, godot_helper)?;
    }
    expect_asset_path(
        "assets.qualityReport",
        "quality-report.json",
        &metadata.assets.quality_report,
    )?;
    if metadata.schema_version == "2.0.0" {
        let consistency = metadata
            .assets
            .consistency_report
            .as_deref()
            .ok_or_else(|| PackError::MissingFile("assets.consistencyReport".into()))?;
        expect_asset_path(
            "assets.consistencyReport",
            "consistency-report.json",
            consistency,
        )?;
        require_regular_pack_file(pack_path, consistency)?;
        if let Some(portrait_base_lock) = metadata.assets.portrait_base_lock.as_deref() {
            expect_asset_path(
                "assets.portraitBaseLock",
                "portrait-base-lock.json",
                portrait_base_lock,
            )?;
            require_regular_pack_file(pack_path, portrait_base_lock)?;
            validate_json_file(
                pack_path.join(portrait_base_lock),
                "portrait-base-lock.schema.json",
                PORTRAIT_BASE_LOCK_SCHEMA,
            )?;
        }
        if let Some(portrait_report) = metadata.assets.portrait_consistency_report.as_deref() {
            expect_asset_path(
                "assets.portraitConsistencyReport",
                "portrait-consistency-report.json",
                portrait_report,
            )?;
            require_regular_pack_file(pack_path, portrait_report)?;
            validate_json_file(
                pack_path.join(portrait_report),
                "portrait-consistency-report.schema.json",
                PORTRAIT_CONSISTENCY_REPORT_SCHEMA,
            )?;
        }
        if let Some(portrait_approval) = metadata.assets.portrait_base_approval.as_deref() {
            expect_asset_path(
                "assets.portraitBaseApproval",
                "portrait-base-approval.json",
                portrait_approval,
            )?;
            require_regular_pack_file(pack_path, portrait_approval)?;
            validate_json_file(
                pack_path.join(portrait_approval),
                "portrait-base-approval.schema.json",
                PORTRAIT_BASE_APPROVAL_SCHEMA,
            )?;
        }
        if matches!(
            metadata.asset_type.as_deref(),
            Some("icon_set" | "prop_set" | "portrait_set" | "equipment_set" | "decal_set")
        ) {
            if metadata.items.is_empty() {
                return Err(PackError::MissingFile("items".into()));
            }
            for item in &metadata.items {
                let expected = format!("assets/items/{}.png", item.id);
                expect_asset_path("items.texture", &expected, &item.texture)?;
                require_regular_pack_file(pack_path, &item.texture)?;
            }
        }
    }

    if frame_pngs(pack_path)?.is_empty() {
        return Err(PackError::NoFrames);
    }

    let forgepack_path = pack_path.join("forgepack.json");
    validate_json_file(
        forgepack_path.clone(),
        "gsfpack.schema.json",
        GSFPACK_SCHEMA,
    )?;
    let forgepack_document = read_json(forgepack_path)?;
    let manifest_path = pack_path.join("assets/manifest.json");
    validate_json_file(
        manifest_path.clone(),
        "manifest.schema.json",
        MANIFEST_SCHEMA,
    )?;
    let manifest_document = read_json(manifest_path)?;
    let godot_helper_document = metadata
        .assets
        .godot_helper
        .as_deref()
        .map(|helper| read_json(pack_path.join(helper)))
        .transpose()?;
    validate_animation_timing_contract(
        &forgepack_document,
        &manifest_document,
        godot_helper_document.as_ref(),
    )?;
    validate_godot_rendering_contract(&manifest_document, godot_helper_document.as_ref())?;
    validate_source_cycle_sampling_provenance(pack_path, &forgepack_document, &manifest_document)?;
    validate_json_file(
        pack_path.join("assets/atlas.json"),
        "atlas.schema.json",
        ATLAS_SCHEMA,
    )?;
    validate_atlas_images(pack_path)?;
    validate_json_file(
        pack_path.join("quality-report.json"),
        "quality-report.schema.json",
        QUALITY_REPORT_SCHEMA,
    )?;
    if let Some(review) = metadata.assets.animation_human_review.as_deref() {
        expect_asset_path(
            "assets.animationHumanReview",
            "quality/animation-human-review.json",
            review,
        )?;
        require_regular_pack_file(pack_path, review)?;
        validate_json_file(
            pack_path.join(review),
            "animation-human-review.schema.json",
            ANIMATION_HUMAN_REVIEW_SCHEMA,
        )?;
    }
    if let Some(report) = metadata.assets.character_semantic_quality_report.as_deref() {
        expect_asset_path(
            "assets.characterSemanticQualityReport",
            "character-semantic-quality-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        validate_json_file(
            pack_path.join(report),
            "character-semantic-quality-report.schema.json",
            CHARACTER_SEMANTIC_QUALITY_REPORT_SCHEMA,
        )?;
    }
    if let Some(lock) = metadata.assets.character_direction_lock.as_deref() {
        expect_asset_path(
            "assets.characterDirectionLock",
            "character-direction-lock.json",
            lock,
        )?;
        require_regular_pack_file(pack_path, lock)?;
        validate_json_file(
            pack_path.join(lock),
            "character-direction-lock.schema.json",
            CHARACTER_DIRECTION_LOCK_SCHEMA,
        )?;
    }
    if let Some(lock) = metadata.assets.character_direction_pose_lock.as_deref() {
        expect_asset_path(
            "assets.characterDirectionPoseLock",
            "character-direction-pose-lock.json",
            lock,
        )?;
        require_regular_pack_file(pack_path, lock)?;
        validate_json_file(
            pack_path.join(lock),
            "character-direction-pose-lock.schema.json",
            CHARACTER_DIRECTION_POSE_LOCK_SCHEMA,
        )?;
    }
    if let Some(lock) = metadata.assets.character_direction_motion_lock.as_deref() {
        expect_asset_path(
            "assets.characterDirectionMotionLock",
            "character-direction-motion-lock.json",
            lock,
        )?;
        require_regular_pack_file(pack_path, lock)?;
        validate_json_file(
            pack_path.join(lock),
            "character-direction-motion-lock.schema.json",
            CHARACTER_DIRECTION_MOTION_LOCK_SCHEMA,
        )?;
    }
    if let Some(approval) = metadata
        .assets
        .character_direction_motion_approval
        .as_deref()
    {
        expect_asset_path(
            "assets.characterDirectionMotionApproval",
            "character-direction-motion-approval.json",
            approval,
        )?;
        require_regular_pack_file(pack_path, approval)?;
        validate_json_file(
            pack_path.join(approval),
            "character-direction-motion-approval.schema.json",
            CHARACTER_DIRECTION_MOTION_APPROVAL_SCHEMA,
        )?;
    }
    if let Some(manifest) = metadata
        .assets
        .direction_motion_provider_manifest
        .as_deref()
    {
        expect_asset_path(
            "assets.directionMotionProviderManifest",
            "provenance/direction-motion-provider-manifest.json",
            manifest,
        )?;
        require_regular_pack_file(pack_path, manifest)?;
        read_json(pack_path.join(manifest))?;
    }
    if let Some(lock) = metadata.assets.character_direction_grid_lock.as_deref() {
        expect_asset_path(
            "assets.characterDirectionGridLock",
            "provenance/grid-keyframes/direction-grid-lock.json",
            lock,
        )?;
        require_regular_pack_file(pack_path, lock)?;
        validate_json_file(
            pack_path.join(lock),
            "character-direction-grid-lock.schema.json",
            CHARACTER_DIRECTION_GRID_LOCK_SCHEMA,
        )?;
        validate_direction_grid_portable_nodes(pack_path, lock)?;
    }
    if let Some(approval) = metadata.assets.character_direction_grid_approval.as_deref() {
        expect_asset_path(
            "assets.characterDirectionGridApproval",
            "provenance/grid-keyframes/direction-grid-approval.json",
            approval,
        )?;
        require_regular_pack_file(pack_path, approval)?;
        validate_json_file(
            pack_path.join(approval),
            "character-direction-grid-approval.schema.json",
            CHARACTER_DIRECTION_GRID_APPROVAL_SCHEMA,
        )?;
    }
    if let Some(report) = metadata.assets.direction_grid_appearance_report.as_deref() {
        expect_asset_path(
            "assets.directionGridAppearanceReport",
            "provenance/grid-keyframes/direction-grid-appearance-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        validate_direction_grid_appearance_report(pack_path, report)?;
    }
    if let Some(manifest) = metadata.assets.grid_keyframe_provider_manifest.as_deref() {
        expect_asset_path(
            "assets.gridKeyframeProviderManifest",
            "provenance/grid-keyframes/provider-manifest.json",
            manifest,
        )?;
        require_regular_pack_file(pack_path, manifest)?;
        read_json(pack_path.join(manifest))?;
    }
    for report in &metadata.assets.grid_keyframe_action_reports {
        require_regular_pack_file(pack_path, report)?;
        validate_json_file(
            pack_path.join(report),
            "grid-keyframe-action-report.schema.json",
            GRID_KEYFRAME_ACTION_REPORT_SCHEMA,
        )?;
        validate_grid_keyframe_portable_report(pack_path, report)?;
    }
    if let Some(report) = metadata.assets.hand_equipment_contact_report.as_deref() {
        expect_asset_path(
            "assets.handEquipmentContactReport",
            "hand-equipment-contact-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        validate_json_file(
            pack_path.join(report),
            "hand-equipment-contact-report.schema.json",
            HAND_EQUIPMENT_CONTACT_REPORT_SCHEMA,
        )?;
    }
    if let Some(report) = metadata
        .assets
        .character_silhouette_temporal_report
        .as_deref()
    {
        expect_asset_path(
            "assets.characterSilhouetteTemporalReport",
            "character-silhouette-temporal-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        let report_path = pack_path.join(report);
        let document = read_json(report_path.clone())?;
        let (schema_name, schema) = if document.get("profile").and_then(serde_json::Value::as_str)
            == Some("silhouette-temporal@1.0.0")
        {
            (
                "character-silhouette-temporal-report-v1.schema.json",
                CHARACTER_SILHOUETTE_TEMPORAL_REPORT_V1_SCHEMA,
            )
        } else {
            (
                "character-silhouette-temporal-report.schema.json",
                CHARACTER_SILHOUETTE_TEMPORAL_REPORT_SCHEMA,
            )
        };
        validate_json_file(report_path, schema_name, schema)?;
    }
    if let Some(report) = metadata
        .assets
        .character_silhouette_temporal_source_report
        .as_deref()
    {
        expect_asset_path(
            "assets.characterSilhouetteTemporalSourceReport",
            "character-silhouette-temporal-source-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        validate_json_file(
            pack_path.join(report),
            "character-silhouette-temporal-report.schema.json",
            CHARACTER_SILHOUETTE_TEMPORAL_REPORT_SCHEMA,
        )?;
    }
    if let Some(report) = metadata.assets.character_motion_semantics_report.as_deref() {
        expect_asset_path(
            "assets.characterMotionSemanticsReport",
            "character-motion-semantics-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        validate_json_file(
            pack_path.join(report),
            "character-motion-semantics-report.schema.json",
            CHARACTER_MOTION_SEMANTICS_REPORT_SCHEMA,
        )?;
    }
    if let Some(report) = metadata.assets.character_gait_cycle_report.as_deref() {
        expect_asset_path(
            "assets.characterGaitCycleReport",
            "character-gait-cycle-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        validate_json_file(
            pack_path.join(report),
            "character-gait-cycle-report-aggregate.schema.json",
            CHARACTER_GAIT_CYCLE_REPORT_AGGREGATE_SCHEMA,
        )?;
        let document = read_json(pack_path.join(report))?;
        validate_character_gait_cycle_contract(&document)?;
    }
    if let Some(report) = metadata.assets.animation_sheet_report.as_deref() {
        expect_asset_path(
            "assets.animationSheetReport",
            "animation-sheet-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        read_json(pack_path.join(report))?;
    }
    if let Some(report) = metadata.assets.character_alpha_repair_report.as_deref() {
        expect_asset_path(
            "assets.characterAlphaRepairReport",
            "character-alpha-repair-report.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        validate_json_file(
            pack_path.join(report),
            "character-alpha-repair-report.schema.json",
            CHARACTER_ALPHA_REPAIR_REPORT_SCHEMA,
        )?;
    }
    if let Some(lock) = metadata.assets.character_scale_lock.as_deref() {
        expect_asset_path(
            "assets.characterScaleLock",
            "character-scale-lock.json",
            lock,
        )?;
        require_regular_pack_file(pack_path, lock)?;
        validate_json_file(
            pack_path.join(lock),
            "character-scale-lock.schema.json",
            CHARACTER_SCALE_LOCK_SCHEMA,
        )?;
    }
    if let Some(report) = metadata.assets.keyframe_loop_trim_report.as_deref() {
        expect_asset_path(
            "assets.keyframeLoopTrimReport",
            "quality/keyframe-loop-trim.json",
            report,
        )?;
        require_regular_pack_file(pack_path, report)?;
        read_json(pack_path.join(report))?;
    }

    Ok(())
}

pub fn import_pack(pack_path: &Path) -> Result<ImportedPack, PackError> {
    validate_pack_layout(pack_path)?;
    let summary = read_pack_summary(pack_path)?;
    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;

    Ok(ImportedPack {
        summary,
        root: pack_path.to_path_buf(),
        frame_paths: if metadata.schema_version == "3.0.0" {
            vec![]
        } else {
            frame_pngs(pack_path)?
        },
        forgepack: read_json(pack_path.join("forgepack.json"))?,
        manifest: read_json(pack_path.join("assets/manifest.json"))?,
        atlas: if pack_path.join("assets/atlas.json").is_file() {
            read_json(pack_path.join("assets/atlas.json"))?
        } else {
            serde_json::Value::Null
        },
        quality_report: read_json(pack_path.join("quality-report.json"))?,
    })
}

pub fn read_pack_summary(pack_path: &Path) -> Result<PackSummary, PackError> {
    validate_pack_layout(pack_path)?;

    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    let frame_count = if metadata.schema_version == "3.0.0" {
        0
    } else {
        frame_pngs(pack_path)?.len()
    };
    let preview = metadata
        .previews
        .gif
        .or(metadata.previews.image)
        .unwrap_or_default();

    Ok(PackSummary {
        id: metadata.id,
        name: metadata.name,
        version: metadata.version,
        frame_count,
        preview_gif: preview,
    })
}

pub fn inspect_pack(pack_path: &Path) -> Result<PackInspectSummary, PackError> {
    let summary = read_pack_summary(pack_path)?;
    let metadata: ForgePackJson =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json"))?)?;
    let manifest = read_json(pack_path.join("assets/manifest.json"))?;
    let animations = manifest
        .get("animations")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|animation| {
            Some(PackAnimationSummary {
                name: animation.get("name")?.as_str()?.to_string(),
                frame_count: animation.get("frames")?.as_array()?.len(),
                fps: animation
                    .get("fps")
                    .and_then(|value| value.as_f64())
                    .unwrap_or(12.0) as f32,
                loop_animation: animation
                    .get("loop")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(true),
            })
        })
        .collect::<Vec<_>>();
    let asset_type = manifest
        .get("assetType")
        .and_then(|value| value.as_str())
        .or(metadata.asset_type.as_deref())
        .or_else(|| summary.name.is_empty().then_some("animation"))
        .unwrap_or(if animations.len() > 1 {
            "character"
        } else {
            "animation"
        })
        .to_string();
    let items = manifest
        .get("items")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|item| {
            Some(PackItemSummary {
                id: item.get("id")?.as_str()?.to_string(),
                name: item.get("name")?.as_str()?.to_string(),
                frame: item.get("frame")?.as_u64()? as usize,
                texture: item.get("texture")?.as_str()?.to_string(),
            })
        })
        .collect::<Vec<_>>();
    let default_animation = animations
        .first()
        .map(|animation| animation.name.clone())
        .unwrap_or_else(|| "idle".to_string());

    Ok(PackInspectSummary {
        id: summary.id,
        name: summary.name,
        version: summary.version,
        frame_count: summary.frame_count,
        preview_gif: summary.preview_gif,
        root: pack_path.to_path_buf(),
        manifest_path: pack_path.join("assets/manifest.json"),
        atlas_path: metadata
            .assets
            .atlas
            .or(metadata.assets.atlas_image)
            .map(|path| pack_path.join(path))
            .unwrap_or_else(|| pack_path.join("assets/manifest.json")),
        quality_report_path: pack_path.join("quality-report.json"),
        default_animation,
        animations,
        asset_type,
        items,
    })
}

fn validate_world_pack_layout(pack_path: &Path, metadata: &ForgePackJson) -> Result<(), PackError> {
    let asset_type = metadata
        .asset_type
        .as_deref()
        .ok_or_else(|| PackError::MissingFile("assetType".into()))?;
    if !matches!(asset_type, "terrain_set" | "building_kit" | "map") {
        return Err(PackError::SchemaValidation {
            document: "forgepack.json".into(),
            message: format!("unsupported V3 assetType {asset_type}"),
        });
    }
    expect_asset_path(
        "assets.manifest",
        "assets/manifest.json",
        &metadata.assets.manifest,
    )?;
    require_regular_pack_file(pack_path, &metadata.assets.manifest)?;
    expect_asset_path(
        "assets.qualityReport",
        "quality-report.json",
        &metadata.assets.quality_report,
    )?;
    require_regular_pack_file(pack_path, &metadata.assets.quality_report)?;
    let helper = metadata
        .assets
        .godot_helper
        .as_deref()
        .ok_or_else(|| PackError::MissingFile("assets.godotHelper".into()))?;
    expect_asset_path("assets.godotHelper", "assets/godot_import.json", helper)?;
    require_regular_pack_file(pack_path, helper)?;
    let preview = metadata
        .previews
        .image
        .as_deref()
        .ok_or_else(|| PackError::MissingFile("previews.image".into()))?;
    expect_asset_path("previews.image", "preview.png", preview)?;
    require_regular_pack_file(pack_path, preview)?;

    match asset_type {
        "terrain_set" => {
            let manifest = metadata
                .assets
                .terrain_manifest
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.terrainManifest".into()))?;
            expect_asset_path(
                "assets.terrainManifest",
                "assets/terrain-manifest.json",
                manifest,
            )?;
            require_regular_pack_file(pack_path, manifest)?;
            let atlas = metadata
                .assets
                .atlas_image
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.atlasImage".into()))?;
            expect_asset_path("assets.atlasImage", "assets/terrain-atlas.png", atlas)?;
            require_regular_pack_file(pack_path, atlas)?;
        }
        "building_kit" => {
            let manifest = metadata
                .assets
                .building_manifest
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.buildingManifest".into()))?;
            expect_asset_path(
                "assets.buildingManifest",
                "assets/building-manifest.json",
                manifest,
            )?;
            require_regular_pack_file(pack_path, manifest)?;
            let atlas = metadata
                .assets
                .atlas_image
                .as_deref()
                .ok_or_else(|| PackError::MissingFile("assets.atlasImage".into()))?;
            expect_asset_path("assets.atlasImage", "assets/building-atlas.png", atlas)?;
            require_regular_pack_file(pack_path, atlas)?;
        }
        "map" => {
            for (field, expected, actual) in [
                (
                    "assets.mapManifest",
                    "assets/map-manifest.json",
                    metadata.assets.map_manifest.as_deref(),
                ),
                (
                    "assets.mapLayout",
                    "assets/map-layout.json",
                    metadata.assets.map_layout.as_deref(),
                ),
                (
                    "assets.validationReport",
                    "validation-report.json",
                    metadata.assets.validation_report.as_deref(),
                ),
            ] {
                let actual = actual.ok_or_else(|| PackError::MissingFile(field.into()))?;
                expect_asset_path(field, expected, actual)?;
                require_regular_pack_file(pack_path, actual)?;
            }
            require_regular_pack_directory(pack_path, "assets/dependencies")?;
            require_regular_pack_directory(pack_path, "assets/runtime")?;
        }
        _ => unreachable!(),
    }

    validate_json_file(
        pack_path.join("forgepack.json"),
        "gsfpack.schema.json",
        GSFPACK_SCHEMA,
    )?;
    Ok(())
}

fn expect_asset_path(field: &str, expected: &str, actual: &str) -> Result<(), PackError> {
    if actual == expected {
        Ok(())
    } else {
        Err(PackError::AssetPathMismatch {
            field: field.to_string(),
            expected: expected.to_string(),
            actual: actual.to_string(),
        })
    }
}

fn validate_direction_grid_portable_nodes(
    pack_path: &Path,
    lock_relative: &str,
) -> Result<(), PackError> {
    let lock = read_json(pack_path.join(lock_relative))?;
    let nodes = lock
        .get("nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| PackError::SchemaValidation {
            document: lock_relative.into(),
            message: "direction grid lock has no nodes".into(),
        })?;
    for node in nodes {
        let path = node
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| PackError::SchemaValidation {
                document: lock_relative.into(),
                message: "direction grid node has no path".into(),
            })?;
        let generation_master_path = node
            .get("generationMasterPath")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if path != generation_master_path
            || !path.starts_with("provenance/grid-keyframes/direction-grid/nodes/")
        {
            return Err(PackError::InvalidAssetPath(path.into()));
        }
        require_regular_pack_file(pack_path, path)?;
        let actual = sha256_file(&pack_path.join(path))?;
        for field in ["sha256", "generationMasterSha256"] {
            if node.get(field).and_then(serde_json::Value::as_str) != Some(actual.as_str()) {
                return Err(PackError::SchemaValidation {
                    document: lock_relative.into(),
                    message: format!("direction grid node {field} does not match bytes"),
                });
            }
        }
    }
    Ok(())
}

fn validate_direction_grid_appearance_report(
    pack_path: &Path,
    report_relative: &str,
) -> Result<(), PackError> {
    let document_path = pack_path.join(report_relative);
    let document = read_json(document_path.clone())?;
    let mut schema = serde_json::from_str::<serde_json::Value>(
        CHARACTER_DIRECTION_GRID_APPEARANCE_REPORT_SCHEMA,
    )?;
    schema["properties"]["equipment"] =
        serde_json::from_str::<serde_json::Value>(HAND_EQUIPMENT_CONTACT_REPORT_SCHEMA)?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|error| PackError::SchemaCompile {
            schema: "character-direction-grid-appearance-report.schema.json".into(),
            message: error.to_string(),
        })?;
    if let Some(error) = validator.iter_errors(&document).next() {
        return Err(PackError::SchemaValidation {
            document: document_path.display().to_string(),
            message: error.to_string(),
        });
    }
    Ok(())
}

fn validate_grid_keyframe_portable_report(
    pack_path: &Path,
    report_relative: &str,
) -> Result<(), PackError> {
    if !report_relative.starts_with("provenance/grid-keyframes/actions/") {
        return Err(PackError::InvalidAssetPath(report_relative.into()));
    }
    let report = read_json(pack_path.join(report_relative))?;
    let frames = report
        .get("frames")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| PackError::SchemaValidation {
            document: report_relative.into(),
            message: "grid keyframe report has no frames".into(),
        })?;
    for frame in frames {
        let path = frame
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| PackError::SchemaValidation {
                document: report_relative.into(),
                message: "grid keyframe has no path".into(),
            })?;
        if !path.starts_with("provenance/grid-keyframes/actions/") {
            return Err(PackError::InvalidAssetPath(path.into()));
        }
        require_regular_pack_file(pack_path, path)?;
        if frame.get("sha256").and_then(serde_json::Value::as_str)
            != Some(sha256_file(&pack_path.join(path))?.as_str())
        {
            return Err(PackError::SchemaValidation {
                document: report_relative.into(),
                message: "grid keyframe SHA-256 does not match bytes".into(),
            });
        }
    }
    for field in ["motionReportPath", "equipmentReportPath"] {
        let path = report
            .get(field)
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| PackError::SchemaValidation {
                document: report_relative.into(),
                message: format!("grid keyframe report has no {field}"),
            })?;
        require_regular_pack_file(pack_path, path)?;
    }
    Ok(())
}

fn require_pack_root_directory(pack_path: &Path) -> Result<(), PackError> {
    let metadata = fs::symlink_metadata(pack_path).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            PackError::MissingPack(pack_path.to_path_buf())
        } else {
            PackError::Io(error)
        }
    })?;

    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(PackError::NotDirectory(pack_path.to_path_buf()));
    }

    Ok(())
}

fn require_regular_pack_file(pack_path: &Path, relative: &str) -> Result<(), PackError> {
    require_regular_pack_entry(pack_path, relative, PackEntryKind::File)
}

fn require_regular_pack_directory(pack_path: &Path, relative: &str) -> Result<(), PackError> {
    require_regular_pack_entry(pack_path, relative, PackEntryKind::Directory)
}

fn require_regular_pack_entry(
    pack_path: &Path,
    relative: &str,
    kind: PackEntryKind,
) -> Result<(), PackError> {
    let root = fs::canonicalize(pack_path)?;
    let path = pack_path.join(relative);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(PackError::MissingFile(relative.to_string()));
        }
        Err(error) => return Err(PackError::Io(error)),
    };

    if metadata.file_type().is_symlink() {
        return Err(PackError::InvalidAssetPath(relative.to_string()));
    }

    if !kind.matches(&metadata) {
        return Err(PackError::MissingFile(relative.to_string()));
    }

    let canonical = fs::canonicalize(&path)?;
    if !canonical.starts_with(&root) {
        return Err(PackError::InvalidAssetPath(relative.to_string()));
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum PackEntryKind {
    File,
    Directory,
}

impl PackEntryKind {
    fn matches(self, metadata: &fs::Metadata) -> bool {
        match self {
            PackEntryKind::File => metadata.is_file(),
            PackEntryKind::Directory => metadata.is_dir(),
        }
    }
}

fn frame_pngs(pack_path: &Path) -> Result<Vec<PathBuf>, PackError> {
    require_regular_pack_directory(pack_path, "assets/frames")?;
    let mut paths = fs::read_dir(pack_path.join("assets/frames"))?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            if !entry.file_type().ok()?.is_file() {
                return None;
            }
            let path = entry.path();
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
                .then_some(path)
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

fn read_json(path: PathBuf) -> Result<serde_json::Value, PackError> {
    Ok(serde_json::from_slice(&fs::read(path)?)?)
}

#[derive(Debug)]
struct AnimationTiming {
    name: String,
    frame_count: usize,
    // Runtime and manifest types expose FPS as f32. Parsing all three JSON
    // documents back to the same precision avoids treating serde's short f32
    // spelling (`6.855184`) and json!()'s exact f64 spelling
    // (`6.855184078216553`) as different cadence values.
    fps: Option<f32>,
    loop_animation: Option<bool>,
    frame_durations_ms: Option<Vec<u64>>,
}

fn validate_animation_timing_contract(
    forgepack: &serde_json::Value,
    manifest: &serde_json::Value,
    godot_helper: Option<&serde_json::Value>,
) -> Result<(), PackError> {
    let forgepack_timings = animation_timings(forgepack.get("animations"), "forgepack.json")?;
    let manifest_timings = animation_timings(manifest.get("animations"), "assets/manifest.json")?;
    validate_animation_timing_cardinality("forgepack.json", &forgepack_timings)?;
    validate_animation_timing_cardinality("assets/manifest.json", &manifest_timings)?;
    // Static V2 packs use one-frame animation metadata for atlas compatibility,
    // but their Godot helper is item/texture based rather than SpriteFrames.
    // Keep the two canonical metadata documents synchronized and leave the
    // type-specific helper to the static-pack validator below.
    if forgepack
        .get("assetType")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|asset_type| {
            matches!(
                asset_type,
                "icon_set" | "prop_set" | "portrait_set" | "equipment_set" | "decal_set"
            )
        })
    {
        validate_animation_metadata_pair(&forgepack_timings, &manifest_timings)?;
        return Ok(());
    }
    let helper_timings = godot_helper
        .map(|helper| {
            let sprite_frames = helper
                .get("spriteFrames")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| {
                    timing_error("assets/godot_import.json", "spriteFrames is missing")
                })?;
            animation_timings(sprite_frames.get("animations"), "assets/godot_import.json")
        })
        .transpose()?;
    if let Some(helper_timings) = helper_timings.as_ref() {
        validate_animation_timing_cardinality("assets/godot_import.json", helper_timings)?;
    }

    // A helper declaration identifies the modern, three-document contract. Packs
    // without one retain V1/V2 compatibility after their local timing validation.
    let Some(helper_timings) = helper_timings.as_ref() else {
        return Ok(());
    };
    validate_modern_animation_timing_contract(
        &forgepack_timings,
        &manifest_timings,
        helper_timings,
    )?;

    Ok(())
}

fn validate_animation_timing_cardinality(
    source: &str,
    timings: &[AnimationTiming],
) -> Result<(), PackError> {
    for timing in timings {
        if let Some(durations) = &timing.frame_durations_ms {
            if !durations.is_empty() && durations.len() != timing.frame_count {
                return Err(timing_error(
                    source,
                    format!(
                        "animation {} has {} frames but {} frameDurationsMs values",
                        timing.name,
                        timing.frame_count,
                        durations.len()
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_animation_metadata_pair(
    forgepack: &[AnimationTiming],
    manifest: &[AnimationTiming],
) -> Result<(), PackError> {
    if animation_name_set(forgepack, "forgepack.json")?
        != animation_name_set(manifest, "assets/manifest.json")?
    {
        return Err(timing_error(
            "assets/manifest.json",
            "animation names differ from forgepack.json",
        ));
    }
    for forgepack_animation in forgepack {
        let manifest_animation = animation_by_name(manifest, &forgepack_animation.name)
            .expect("matching animation names were validated");
        if forgepack_animation.frame_count != manifest_animation.frame_count {
            return Err(timing_error(
                "assets/manifest.json",
                format!(
                    "animation {} frame count differs from forgepack.json",
                    forgepack_animation.name
                ),
            ));
        }
        validate_optional_timing_field(
            &forgepack_animation.name,
            "fps",
            vec![
                ("forgepack.json", forgepack_animation.fps),
                ("assets/manifest.json", manifest_animation.fps),
            ],
        )?;
        validate_optional_timing_field(
            &forgepack_animation.name,
            "loop",
            vec![
                ("forgepack.json", forgepack_animation.loop_animation),
                ("assets/manifest.json", manifest_animation.loop_animation),
            ],
        )?;
        validate_optional_timing_field(
            &forgepack_animation.name,
            "frameDurationsMs",
            vec![
                (
                    "forgepack.json",
                    forgepack_animation.frame_durations_ms.clone(),
                ),
                (
                    "assets/manifest.json",
                    manifest_animation.frame_durations_ms.clone(),
                ),
            ],
        )?;
    }
    Ok(())
}

fn validate_modern_animation_timing_contract(
    forgepack: &[AnimationTiming],
    manifest: &[AnimationTiming],
    godot_helper: &[AnimationTiming],
) -> Result<(), PackError> {
    let forgepack_names = animation_name_set(forgepack, "forgepack.json")?;
    let manifest_names = animation_name_set(manifest, "assets/manifest.json")?;
    let helper_names = animation_name_set(godot_helper, "assets/godot_import.json")?;
    if forgepack_names != manifest_names {
        return Err(timing_error(
            "assets/manifest.json",
            "animation names differ from forgepack.json",
        ));
    }
    if forgepack_names != helper_names {
        return Err(timing_error(
            "assets/godot_import.json",
            "animation names differ from forgepack.json",
        ));
    }

    for forgepack_animation in forgepack {
        let manifest_animation = animation_by_name(manifest, &forgepack_animation.name)
            .expect("matching animation names were validated");
        let helper_animation = animation_by_name(godot_helper, &forgepack_animation.name)
            .expect("matching animation names were validated");
        if forgepack_animation.frame_count != manifest_animation.frame_count {
            return Err(timing_error(
                "assets/manifest.json",
                format!(
                    "animation {} frame count differs from forgepack.json",
                    forgepack_animation.name
                ),
            ));
        }
        if forgepack_animation.frame_count != helper_animation.frame_count {
            return Err(timing_error(
                "assets/godot_import.json",
                format!(
                    "animation {} frame count differs from forgepack.json",
                    forgepack_animation.name
                ),
            ));
        }
        validate_optional_timing_field(
            &forgepack_animation.name,
            "fps",
            vec![
                ("forgepack.json", forgepack_animation.fps),
                ("assets/manifest.json", manifest_animation.fps),
                ("assets/godot_import.json", helper_animation.fps),
            ],
        )?;
        validate_optional_timing_field(
            &forgepack_animation.name,
            "loop",
            vec![
                ("forgepack.json", forgepack_animation.loop_animation),
                ("assets/manifest.json", manifest_animation.loop_animation),
                ("assets/godot_import.json", helper_animation.loop_animation),
            ],
        )?;
        validate_optional_timing_field(
            &forgepack_animation.name,
            "frameDurationsMs",
            vec![
                (
                    "forgepack.json",
                    forgepack_animation.frame_durations_ms.clone(),
                ),
                (
                    "assets/manifest.json",
                    manifest_animation.frame_durations_ms.clone(),
                ),
                (
                    "assets/godot_import.json",
                    helper_animation.frame_durations_ms.clone(),
                ),
            ],
        )?;
    }

    Ok(())
}

fn animation_name_set(
    animations: &[AnimationTiming],
    source: &str,
) -> Result<HashSet<String>, PackError> {
    let names = animations
        .iter()
        .map(|animation| animation.name.clone())
        .collect::<HashSet<_>>();
    if names.len() != animations.len() {
        return Err(timing_error(source, "animation names must be unique"));
    }
    Ok(names)
}

fn animation_by_name<'a>(
    animations: &'a [AnimationTiming],
    name: &str,
) -> Option<&'a AnimationTiming> {
    animations.iter().find(|animation| animation.name == name)
}

fn validate_optional_timing_field<T: PartialEq>(
    animation_name: &str,
    field: &str,
    values: Vec<(&str, Option<T>)>,
) -> Result<(), PackError> {
    if values.iter().all(|(_, value)| value.is_none()) {
        // Explicit legacy branch: a timing field omitted everywhere remains valid.
        return Ok(());
    }
    let mut values = values.into_iter();
    let (_, expected) = values
        .next()
        .expect("modern timing contract always has three documents");
    let expected = expected.ok_or_else(|| {
        timing_error(
            "forgepack.json",
            format!("animation {animation_name} {field} is missing"),
        )
    })?;
    for (source, value) in values {
        let value = value.ok_or_else(|| {
            timing_error(
                source,
                format!("animation {animation_name} {field} is missing"),
            )
        })?;
        if value != expected {
            return Err(timing_error(
                source,
                format!("animation {animation_name} {field} differs from forgepack.json"),
            ));
        }
    }
    Ok(())
}

fn animation_timings(
    animations: Option<&serde_json::Value>,
    source: &str,
) -> Result<Vec<AnimationTiming>, PackError> {
    let animations = animations
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| timing_error(source, "animations must be an array"))?;
    animations
        .iter()
        .map(|animation| {
            let animation = animation
                .as_object()
                .ok_or_else(|| timing_error(source, "animation must be an object"))?;
            let name = animation
                .get("name")
                .and_then(serde_json::Value::as_str)
                .filter(|name| !name.is_empty())
                .ok_or_else(|| timing_error(source, "animation name is missing"))?
                .to_string();
            let frames = animation
                .get("frames")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| timing_error(source, format!("animation {name} frames are missing")))?;
            let fps = animation
                .get("fps")
                .map(|value| {
                    value
                        .as_f64()
                        .map(|fps| fps as f32)
                        .filter(|fps| fps.is_finite() && *fps > 0.0)
                        .ok_or_else(|| {
                            timing_error(source, format!("animation {name} fps must be positive"))
                        })
                })
                .transpose()?;
            let loop_animation = animation
                .get("loop")
                .map(|value| {
                    value.as_bool().ok_or_else(|| {
                        timing_error(source, format!("animation {name} loop must be a boolean"))
                    })
                })
                .transpose()?;
            let frame_durations_ms = animation
                .get("frameDurationsMs")
                .map(|value| {
                    value
                        .as_array()
                        .ok_or_else(|| {
                            timing_error(
                                source,
                                format!("animation {name} frameDurationsMs must be an array"),
                            )
                        })?
                        .iter()
                        .map(|duration| {
                            duration.as_u64().filter(|duration| *duration > 0).ok_or_else(|| {
                                timing_error(
                                    source,
                                    format!(
                                        "animation {name} frameDurationsMs must contain positive integers"
                                    ),
                                )
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .transpose()?;
            Ok(AnimationTiming {
                name,
                frame_count: frames.len(),
                fps,
                loop_animation,
                frame_durations_ms,
            })
        })
        .collect()
}

fn timing_error(document: &str, message: impl Into<String>) -> PackError {
    PackError::SchemaValidation {
        document: document.to_string(),
        message: message.into(),
    }
}

fn validate_godot_rendering_contract(
    manifest: &serde_json::Value,
    godot_helper: Option<&serde_json::Value>,
) -> Result<(), PackError> {
    let manifest_rendering = manifest.get("rendering");
    let helper_rendering =
        godot_helper.and_then(|helper| helper.pointer("/spriteFrames/rendering"));
    match (manifest_rendering, helper_rendering) {
        (None, None) => return Ok(()),
        (Some(_), None) => {
            return Err(timing_error(
                "assets/godot_import.json",
                "spriteFrames.rendering is missing",
            ));
        }
        (None, Some(_)) => {
            return Err(timing_error("assets/manifest.json", "rendering is missing"));
        }
        (Some(manifest_rendering), Some(helper_rendering))
            if manifest_rendering != helper_rendering =>
        {
            return Err(timing_error(
                "assets/godot_import.json",
                "spriteFrames.rendering differs from assets/manifest.json",
            ));
        }
        (Some(_), Some(_)) => {}
    }

    let rendering = manifest_rendering
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| timing_error("assets/manifest.json", "rendering must be an object"))?;
    if rendering.get("profile").and_then(serde_json::Value::as_str)
        != Some("godot-sprite-rendering@1.0.0")
    {
        return Err(timing_error(
            "assets/manifest.json",
            "rendering.profile must be godot-sprite-rendering@1.0.0",
        ));
    }
    if !matches!(
        rendering
            .get("textureFilter")
            .and_then(serde_json::Value::as_str),
        Some("nearest" | "linear")
    ) {
        return Err(timing_error(
            "assets/manifest.json",
            "rendering.textureFilter must be nearest or linear",
        ));
    }
    let pixel_snap = rendering
        .get("pixelSnap")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            timing_error(
                "assets/manifest.json",
                "rendering.pixelSnap must be a boolean",
            )
        })?;
    let mirror_policy = rendering
        .get("mirrorPolicy")
        .and_then(serde_json::Value::as_str)
        .filter(|policy| {
            matches!(
                *policy,
                "auto" | "right_only" | "explicit_left" | "mirror_right_to_left"
            )
        })
        .ok_or_else(|| {
            timing_error(
                "assets/manifest.json",
                "rendering.mirrorPolicy is unsupported",
            )
        })?;

    if pixel_snap {
        for coordinate in ["x", "y"] {
            let value = manifest
                .pointer(&format!("/anchor/{coordinate}"))
                .and_then(serde_json::Value::as_f64)
                .ok_or_else(|| {
                    timing_error(
                        "assets/manifest.json",
                        format!("anchor.{coordinate} must be numeric"),
                    )
                })?;
            if value.fract().abs() > f64::EPSILON {
                return Err(timing_error(
                    "assets/manifest.json",
                    format!("pixel-snapped anchor.{coordinate} must be an integer"),
                ));
            }
        }
    }

    let animation_names = manifest
        .get("animations")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|animation| animation.get("name").and_then(serde_json::Value::as_str))
        .collect::<HashSet<_>>();
    let required = match mirror_policy {
        "right_only" => &["idle_right", "walk_right"][..],
        "explicit_left" => &["idle_left", "walk_left"][..],
        "mirror_right_to_left" => &["idle_right", "walk_right"][..],
        _ => &[][..],
    };
    if !required.is_empty() && required.iter().any(|name| !animation_names.contains(name)) {
        return Err(timing_error(
            "assets/manifest.json",
            format!(
                "rendering.mirrorPolicy {mirror_policy} requires animations {}",
                required.join(", ")
            ),
        ));
    }
    if mirror_policy == "right_only"
        && ["idle_left", "walk_left"]
            .iter()
            .any(|name| animation_names.contains(name))
    {
        return Err(timing_error(
            "assets/manifest.json",
            "rendering.mirrorPolicy right_only forbids idle_left and walk_left",
        ));
    }
    Ok(())
}

fn validate_character_gait_cycle_contract(document: &serde_json::Value) -> Result<(), PackError> {
    let fail = |message: String| PackError::SchemaValidation {
        document: "character-gait-cycle-report.json".into(),
        message,
    };
    let animations = document
        .get("animations")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| fail("animations must be an array".into()))?;
    let mut seen = HashSet::new();
    for animation in animations {
        let name = animation
            .get("animation")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| fail("animation name is missing".into()))?;
        if !seen.insert(name.to_string()) {
            return Err(fail(format!("duplicate gait animation: {name}")));
        }
        let direction = animation
            .get("direction")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| fail(format!("{name} direction is missing")))?;
        let expected_direction = match name {
            "walk_up" => "rear",
            "walk_right" => "right",
            "walk_down" => "front",
            "walk_left" => "left",
            _ => return Err(fail(format!("unexpected gait animation: {name}"))),
        };
        if direction != expected_direction {
            return Err(fail(format!(
                "{name} must use direction {expected_direction}, got {direction}"
            )));
        }
        if animation.get("verdict").and_then(serde_json::Value::as_str) != Some("game_ready") {
            return Err(fail(format!("{name} gait evidence is not game_ready")));
        }
        let candidate_count = animation
            .get("candidateFrameCount")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| fail(format!("{name} candidateFrameCount is missing")))?
            as usize;
        let start = animation
            .get("selectedStartFrame")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| fail(format!("{name} selectedStartFrame is missing")))?
            as usize;
        let end = animation
            .get("selectedEndBoundaryFrame")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| fail(format!("{name} selectedEndBoundaryFrame is missing")))?
            as usize;
        if start >= end || end >= candidate_count {
            return Err(fail(format!(
                "{name} selected interval is outside candidate frames"
            )));
        }
        let duration = animation
            .get("sourceDurationMs")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| fail(format!("{name} sourceDurationMs is missing")))?;
        if !(700..=1_200).contains(&duration) {
            return Err(fail(format!(
                "{name} source duration {duration}ms is outside 700–1200ms"
            )));
        }
        let indices = |field: &str| -> Result<Vec<usize>, PackError> {
            animation
                .get(field)
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| fail(format!("{name} {field} is missing")))?
                .iter()
                .map(|value| {
                    value
                        .as_u64()
                        .map(|value| value as usize)
                        .ok_or_else(|| fail(format!("{name} {field} contains a non-index")))
                })
                .collect()
        };
        let output = indices("outputFrameIndices")?;
        let source = indices("sourceFrameIndices")?;
        let contacts = indices("contactFrames")?;
        let passings = indices("passingFrames")?;
        if output.len() != 8
            || output.first().copied() != Some(start)
            || output.iter().any(|index| *index >= end)
            || output.windows(2).any(|pair| pair[0] >= pair[1])
            || source != output
            || contacts != [output[0], output[4]]
            || passings != [output[2], output[6]]
        {
            return Err(fail(format!(
                "{name} gait phase indices are inconsistent or non-monotonic"
            )));
        }
    }
    let required = HashSet::from([
        "walk_up".to_string(),
        "walk_right".to_string(),
        "walk_down".to_string(),
    ]);
    let complete = HashSet::from([
        "walk_up".to_string(),
        "walk_right".to_string(),
        "walk_down".to_string(),
        "walk_left".to_string(),
    ]);
    if seen != required && seen != complete {
        return Err(fail(
            "gait report must contain walk_up, walk_right, and walk_down, with optional explicit walk_left"
                .into(),
        ));
    }
    Ok(())
}

fn validate_atlas_images(pack_path: &Path) -> Result<(), PackError> {
    let atlas = read_json(pack_path.join("assets/atlas.json"))?;
    let mut page_images = None;

    if let Some(image) = atlas.get("image").and_then(|value| value.as_str()) {
        validate_page_image_name(image)?;
        if image != LEGACY_SPRITE_SHEET_IMAGE {
            return Err(PackError::InvalidAssetPath(image.to_string()));
        }
        require_page_image_file(pack_path, image)?;
    }

    if let Some(images) = atlas.get("images").and_then(|value| value.as_array()) {
        let mut validated_images = HashSet::new();
        for image in images {
            let Some(image) = image.as_str() else {
                continue;
            };
            validate_page_image_name(image)?;
            require_page_image_file(pack_path, image)?;
            validated_images.insert(image.to_string());
        }
        page_images = Some(validated_images);
    }

    if let Some(frames) = atlas.get("frames").and_then(|value| value.as_array()) {
        for frame in frames {
            let Some(image) = frame.get("image").and_then(|value| value.as_str()) else {
                continue;
            };
            validate_page_image_name(image)?;
            if let Some(images) = page_images.as_ref() {
                if !images.contains(image) {
                    return Err(PackError::InvalidAssetPath(image.to_string()));
                }
            } else {
                require_page_image_file(pack_path, image)?;
            }
        }
    }

    Ok(())
}

fn validate_page_image_name(image: &str) -> Result<(), PackError> {
    let path = Path::new(image);
    let has_png_extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("png"))
        .unwrap_or(false);

    if image.is_empty()
        || image.contains('/')
        || image.contains('\\')
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        || path.file_name().and_then(|value| value.to_str()) != Some(image)
        || !has_png_extension
    {
        return Err(PackError::InvalidAssetPath(image.to_string()));
    }

    Ok(())
}

fn require_page_image_file(pack_path: &Path, image: &str) -> Result<(), PackError> {
    let relative = format!("assets/{image}");
    require_regular_pack_file(pack_path, &relative)
}

fn validate_json_file(
    document_path: PathBuf,
    schema_name: &str,
    schema_source: &str,
) -> Result<(), PackError> {
    let schema = serde_json::from_str::<serde_json::Value>(schema_source)?;
    let document = read_json(document_path.clone())?;
    let validator =
        jsonschema::validator_for(&schema).map_err(|error| PackError::SchemaCompile {
            schema: schema_name.to_string(),
            message: error.to_string(),
        })?;

    if let Some(error) = validator.iter_errors(&document).next() {
        return Err(PackError::SchemaValidation {
            document: document_path.display().to_string(),
            message: error.to_string(),
        });
    }

    Ok(())
}

fn validate_source_cycle_sampling_provenance(
    pack_path: &Path,
    forgepack: &serde_json::Value,
    manifest: &serde_json::Value,
) -> Result<(), PackError> {
    let fail = |message: &str| PackError::SchemaValidation {
        document: pack_path.join("forgepack.json").display().to_string(),
        message: format!("source cycle sampling provenance: {message}"),
    };
    let evidence = match forgepack.pointer("/source/metadata/sourceCycleSampling") {
        Some(evidence) => evidence,
        None if forgepack
            .pointer("/assets/characterDirectionMotionLock")
            .is_some() =>
        {
            return Err(fail(
                "direction-motion packs require source cycle sampling provenance",
            ));
        }
        None => return Ok(()),
    };
    if evidence.get("profile").and_then(serde_json::Value::as_str)
        != Some("source-cycle-sampling@1.0.0")
        || evidence
            .get("nativeFpsCeiling")
            .and_then(serde_json::Value::as_u64)
            != Some(24)
        || evidence
            .get("maximumCandidateFrames")
            .and_then(serde_json::Value::as_u64)
            != Some(120)
    {
        return Err(fail("profile or extraction bounds do not match V8.1"));
    }
    let reports = evidence
        .get("reports")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| fail("reports object is missing"))?;
    let direction_motion_pack = forgepack
        .pointer("/assets/characterDirectionMotionLock")
        .is_some();
    let expected = if direction_motion_pack {
        let names = ["walk_down", "walk_up", "walk_right", "walk_left"]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if reports.len() != names.len() || names.iter().any(|name| !reports.contains_key(name)) {
            return Err(fail("exactly four directional reports are required"));
        }
        names
    } else {
        if evidence
            .get("sourceWorkflow")
            .and_then(serde_json::Value::as_str)
            .is_none_or(str::is_empty)
            || evidence
                .get("processingUpgrade")
                .and_then(serde_json::Value::as_str)
                != Some("source-cycle-sampling@1.0.0")
            || evidence
                .get("providerRequests")
                .and_then(serde_json::Value::as_u64)
                != Some(0)
        {
            return Err(fail(
                "local reprocessing provenance is missing its source workflow or zero-request declaration",
            ));
        }
        let names = reports.keys().cloned().collect::<Vec<_>>();
        if names.is_empty()
            || names.iter().any(|name| {
                name.is_empty()
                    || !name.chars().all(|character| {
                        character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                    })
            })
        {
            return Err(fail("local reprocessing report names are invalid"));
        }
        names
    };
    let manifest_animations = manifest
        .get("animations")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| fail("assets manifest animations are missing"))?;

    for animation in expected {
        let summary = reports
            .get(&animation)
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| fail("a directional report summary is missing"))?;
        let expected_relative = format!("provenance/source-cycle-sampling/{animation}.json");
        if summary.get("path").and_then(serde_json::Value::as_str)
            != Some(expected_relative.as_str())
        {
            return Err(fail("report path is not canonical"));
        }
        require_regular_pack_file(pack_path, &expected_relative)?;
        let report_path = pack_path.join(&expected_relative);
        validate_json_file(
            report_path.clone(),
            "source-cycle-sampling-report.schema.json",
            SOURCE_CYCLE_SAMPLING_REPORT_SCHEMA,
        )?;
        let report = read_json(report_path.clone())?;
        let recorded_hash = summary
            .get("sha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| fail("report sha256 is missing"))?;
        if recorded_hash != sha256_file(&report_path)? {
            return Err(fail("report sha256 does not match its bytes"));
        }

        let start =
            json_usize(&report, "selectedStartFrame").ok_or_else(|| fail("start missing"))?;
        let end = json_usize(&report, "selectedEndBoundaryFrame")
            .ok_or_else(|| fail("end boundary missing"))?;
        let source_count = json_usize(&report, "sourceCycleFrameCount")
            .ok_or_else(|| fail("source frame count missing"))?;
        let output_count = json_usize(&report, "outputFrameCount")
            .ok_or_else(|| fail("output frame count missing"))?;
        let source_timestamps = json_u64_array(&report, "sourceTimestampsMs")
            .ok_or_else(|| fail("source timestamps are invalid"))?;
        let output_indices = json_usize_array(&report, "outputFrameIndices")
            .ok_or_else(|| fail("output indices are invalid"))?;
        let output_timestamps = json_u64_array(&report, "outputTimestampsMs")
            .ok_or_else(|| fail("output timestamps are invalid"))?;
        let mandatory = json_usize_array(&report, "mandatoryFrameIndices")
            .ok_or_else(|| fail("mandatory indices are invalid"))?;
        let threshold = report
            .get("reconstructionErrorThreshold")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| fail("reconstruction threshold is missing"))?;
        let error = report
            .get("normalizedReconstructionError")
            .and_then(serde_json::Value::as_f64)
            .ok_or_else(|| fail("reconstruction error is missing"))?;
        let decision = report
            .get("decision")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| fail("sampling decision is missing"))?;

        let output_valid = matches!(output_count, 8 | 10 | 12)
            && output_indices.len() == output_count
            && output_timestamps.len() == output_count
            && output_indices.first().copied() == Some(start)
            && output_indices.windows(2).all(|pair| pair[0] < pair[1])
            && output_indices
                .iter()
                .all(|index| *index >= start && *index < end)
            && output_indices
                .iter()
                .zip(&output_timestamps)
                .all(|(index, timestamp)| source_timestamps.get(index - start) == Some(timestamp));
        let source_valid = start < end
            && source_count == end - start
            && source_timestamps.len() == source_count + 1
            && source_timestamps.windows(2).all(|pair| pair[0] < pair[1]);
        let mandatory_valid = mandatory.windows(2).all(|pair| pair[0] < pair[1])
            && mandatory.iter().all(|index| output_indices.contains(index));
        let decision_valid = matches!(
            (output_count, decision),
            (8, "minimum_count_sufficient")
                | (10, "promoted_for_motion_complexity")
                | (12, "maximum_count_required")
        );
        if !source_valid
            || !output_valid
            || !mandatory_valid
            || !decision_valid
            || !threshold.is_finite()
            || threshold < 0.0
            || !error.is_finite()
            || error > threshold
        {
            return Err(fail("report field relationships are invalid"));
        }
        for (summary_field, report_field) in [
            ("selectedStartFrame", "selectedStartFrame"),
            ("selectedEndBoundaryFrame", "selectedEndBoundaryFrame"),
            ("sourceCycleFrameCount", "sourceCycleFrameCount"),
            ("outputFrameCount", "outputFrameCount"),
            ("decision", "decision"),
        ] {
            if summary.get(summary_field) != report.get(report_field) {
                return Err(fail("forgepack summary does not match the report"));
            }
        }
        let animation_manifest = manifest_animations
            .iter()
            .find(|entry| {
                entry.get("name").and_then(serde_json::Value::as_str) == Some(animation.as_str())
            })
            .ok_or_else(|| fail("animation is missing from assets manifest"))?;
        let durations = json_u64_array(animation_manifest, "frameDurationsMs")
            .ok_or_else(|| fail("animation frame durations are invalid"))?;
        let source_duration = source_timestamps
            .last()
            .zip(source_timestamps.first())
            .map(|(last, first)| last.saturating_sub(*first))
            .unwrap_or_default();
        let mut expected_durations = output_timestamps
            .windows(2)
            .map(|pair| pair[1].saturating_sub(pair[0]))
            .collect::<Vec<_>>();
        expected_durations.push(
            source_timestamps
                .last()
                .copied()
                .unwrap_or_default()
                .saturating_sub(output_timestamps.last().copied().unwrap_or_default()),
        );
        if durations != expected_durations || durations.iter().sum::<u64>() != source_duration {
            return Err(fail(
                "Godot/manifest per-frame timing does not equal the selected PTS cycle",
            ));
        }
    }
    Ok(())
}

fn json_usize(document: &serde_json::Value, field: &str) -> Option<usize> {
    document.get(field)?.as_u64()?.try_into().ok()
}

fn json_usize_array(document: &serde_json::Value, field: &str) -> Option<Vec<usize>> {
    document
        .get(field)?
        .as_array()?
        .iter()
        .map(|value| value.as_u64()?.try_into().ok())
        .collect()
}

fn json_u64_array(document: &serde_json::Value, field: &str) -> Option<Vec<u64>> {
    document
        .get(field)?
        .as_array()?
        .iter()
        .map(serde_json::Value::as_u64)
        .collect()
}

fn sha256_file(path: &Path) -> Result<String, PackError> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod timing_contract_tests {
    use super::*;

    fn source_sampling_fixture() -> (tempfile::TempDir, serde_json::Value, serde_json::Value) {
        let temp = tempfile::tempdir().unwrap();
        let sampling_root = temp.path().join("provenance/source-cycle-sampling");
        fs::create_dir_all(&sampling_root).unwrap();
        let source_timestamps = (0..=24).map(|index| index * 42).collect::<Vec<_>>();
        let output_indices = vec![0, 3, 6, 9, 12, 15, 18, 21];
        let output_timestamps = output_indices
            .iter()
            .map(|index| source_timestamps[*index])
            .collect::<Vec<_>>();
        let mut summaries = serde_json::Map::new();
        let mut manifest_animations = Vec::new();
        for animation in ["walk_down", "walk_up", "walk_right", "walk_left"] {
            let report = serde_json::json!({
                "schemaVersion": "1",
                "profile": "source-cycle-sampling@1.0.0",
                "selectedStartFrame": 0,
                "selectedEndBoundaryFrame": 24,
                "sourceCycleFrameCount": 24,
                "sourceTimestampsMs": source_timestamps,
                "mandatoryFrameIndices": [],
                "outputFrameIndices": output_indices,
                "outputTimestampsMs": output_timestamps,
                "outputFrameCount": 8,
                "reconstructionErrorThreshold": 0.12,
                "normalizedReconstructionError": 0.05,
                "decision": "minimum_count_sufficient"
            });
            let path = sampling_root.join(format!("{animation}.json"));
            fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
            summaries.insert(
                animation.into(),
                serde_json::json!({
                    "path": format!("provenance/source-cycle-sampling/{animation}.json"),
                    "sha256": sha256_file(&path).unwrap(),
                    "selectedStartFrame": 0,
                    "selectedEndBoundaryFrame": 24,
                    "sourceCycleFrameCount": 24,
                    "outputFrameCount": 8,
                    "decision": "minimum_count_sufficient"
                }),
            );
            manifest_animations.push(serde_json::json!({
                "name": animation,
                "frameDurationsMs": vec![126; 8]
            }));
        }
        let forgepack = serde_json::json!({
            "assets": { "characterDirectionMotionLock": "character-direction-motion-lock.json" },
            "source": { "metadata": { "sourceCycleSampling": {
                "profile": "source-cycle-sampling@1.0.0",
                "nativeFpsCeiling": 24,
                "maximumCandidateFrames": 120,
                "reports": summaries
            }}}
        });
        let manifest = serde_json::json!({ "animations": manifest_animations });
        (temp, forgepack, manifest)
    }

    #[test]
    fn static_metadata_rejects_matching_but_wrong_duration_cardinality() {
        let animation = serde_json::json!({
            "name": "potion",
            "frames": [0],
            "fps": 1.0,
            "loop": false,
            "frameDurationsMs": [100, 100]
        });
        let forgepack = serde_json::json!({
            "assetType": "icon_set",
            "animations": [animation.clone()]
        });
        let manifest = serde_json::json!({ "animations": [animation] });
        let helper = serde_json::json!({ "assetType": "icon_set", "items": [] });

        assert!(matches!(
            validate_animation_timing_contract(&forgepack, &manifest, Some(&helper)).unwrap_err(),
            PackError::SchemaValidation { document, message }
                if document == "forgepack.json"
                    && message.contains("1 frames but 2 frameDurationsMs values")
        ));
    }

    #[test]
    fn direction_motion_pack_requires_source_sampling_provenance() {
        let (temp, mut forgepack, manifest) = source_sampling_fixture();
        validate_source_cycle_sampling_provenance(temp.path(), &forgepack, &manifest).unwrap();
        forgepack["source"]["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("sourceCycleSampling");
        assert!(
            validate_source_cycle_sampling_provenance(temp.path(), &forgepack, &manifest)
                .unwrap_err()
                .to_string()
                .contains("require source cycle sampling")
        );
    }

    #[test]
    fn source_sampling_rejects_same_total_with_wrong_per_frame_durations() {
        let (temp, forgepack, mut manifest) = source_sampling_fixture();
        let durations = manifest["animations"][0]["frameDurationsMs"]
            .as_array_mut()
            .unwrap();
        durations[0] = serde_json::json!(125);
        durations[1] = serde_json::json!(127);
        assert!(
            validate_source_cycle_sampling_provenance(temp.path(), &forgepack, &manifest)
                .unwrap_err()
                .to_string()
                .contains("per-frame timing")
        );
    }

    #[test]
    fn source_sampling_pack_rejects_an_output_that_skips_cycle_start() {
        let (temp, mut forgepack, manifest) = source_sampling_fixture();
        let path = temp
            .path()
            .join("provenance/source-cycle-sampling/walk_down.json");
        let mut report = read_json(path.clone()).unwrap();
        report["outputFrameIndices"][0] = serde_json::json!(1);
        report["outputTimestampsMs"][0] = serde_json::json!(42);
        fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
        forgepack["source"]["metadata"]["sourceCycleSampling"]["reports"]["walk_down"]["sha256"] =
            serde_json::json!(sha256_file(&path).unwrap());
        assert!(
            validate_source_cycle_sampling_provenance(temp.path(), &forgepack, &manifest)
                .unwrap_err()
                .to_string()
                .contains("field relationships")
        );
    }

    #[test]
    fn v93_grid_keyframe_schema_requires_the_exact_asymmetric_frame_layout() {
        let hash = "0".repeat(64);
        let frame = |index: u8,
                     phase: &str,
                     generation_method: &str,
                     pose_profile: &str,
                     reused: bool,
                     provider_request_occurred: bool| {
            serde_json::json!({
                "frameIndex": index,
                "phase": phase,
                "attempt": if reused { 1 } else { 2 },
                "generationMethod": generation_method,
                "path": format!("frame-{index}.png"),
                "sha256": hash,
                "reused": reused,
                "providerRequestOccurred": provider_request_occurred,
                "poseStructurePath": format!("pose-{index}.png"),
                "poseStructureSha256": hash,
                "poseStructureProfile": pose_profile
            })
        };
        let report = serde_json::json!({
            "schemaVersion": "1",
            "profile": "grid-keyframe-action-report@1.3.0",
            "animation": "walk_down",
            "directionNodeId": "front_idle",
            "directionAnchorSha256": hash,
            "phaseProfile": "grid-action-phases@1.0.0",
            "frames": [
                frame(0, "left_contact", "byte_reuse", "grid-pose-structure@1.0.0", true, false),
                frame(1, "left_passing", "byte_reuse", "grid-pose-structure@1.0.0", true, false),
                frame(2, "right_contact", "asymmetric_guide_fresh_retry", "grid-pose-structure@1.1.0", false, true),
                frame(3, "right_passing", "byte_reuse", "grid-pose-structure@1.0.0", true, false)
            ],
            "motionReportPath": "motion.json",
            "motionReportSha256": hash,
            "motionVerdict": "game_ready",
            "equipmentReportPath": "equipment.json",
            "equipmentReportSha256": hash,
            "equipmentVerdict": "game_ready",
            "lateralityReportPath": "laterality.json",
            "lateralityReportSha256": hash,
            "lateralityVerdict": "game_ready",
            "retriedFrames": [2]
        });
        let schema: serde_json::Value =
            serde_json::from_str(GRID_KEYFRAME_ACTION_REPORT_SCHEMA).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&report));

        let mut wrong_position = report.clone();
        wrong_position["frames"].as_array_mut().unwrap().swap(0, 2);
        assert!(!validator.is_valid(&wrong_position));

        let mut wrong_method = report;
        wrong_method["frames"][2]["generationMethod"] = serde_json::json!("byte_reuse");
        wrong_method["frames"][2]["poseStructureProfile"] =
            serde_json::json!("grid-pose-structure@1.0.0");
        wrong_method["frames"][2]["providerRequestOccurred"] = serde_json::json!(false);
        wrong_method["frames"][2]["reused"] = serde_json::json!(true);
        assert!(!validator.is_valid(&wrong_method));
    }

    #[test]
    fn v94_grid_keyframe_schema_requires_platform_safe_frame_and_report() {
        let hash = "0".repeat(64);
        let frame = |index: u8,
                     phase: &str,
                     generation_method: &str,
                     pose_profile: &str,
                     reused: bool,
                     provider_request_occurred: bool| {
            serde_json::json!({
                "frameIndex": index,
                "phase": phase,
                "attempt": if reused { 1 } else { 4 },
                "generationMethod": generation_method,
                "path": format!("frame-{index}.png"),
                "sha256": hash,
                "reused": reused,
                "providerRequestOccurred": provider_request_occurred,
                "poseStructurePath": format!("pose-{index}.png"),
                "poseStructureSha256": hash,
                "poseStructureProfile": pose_profile
            })
        };
        let report = serde_json::json!({
            "schemaVersion": "1",
            "profile": "grid-keyframe-action-report@1.4.0",
            "animation": "walk_down",
            "directionNodeId": "front_idle",
            "directionAnchorSha256": hash,
            "phaseProfile": "grid-action-phases@1.0.0",
            "frames": [
                frame(0, "left_contact", "byte_reuse", "grid-pose-structure@1.0.0", true, false),
                frame(1, "left_passing", "byte_reuse", "grid-pose-structure@1.0.0", true, false),
                frame(2, "right_contact", "platform_safe_guide_fresh_retry", "grid-pose-structure@1.2.0", false, true),
                frame(3, "right_passing", "byte_reuse", "grid-pose-structure@1.0.0", true, false)
            ],
            "motionReportPath": "motion.json",
            "motionReportSha256": hash,
            "motionVerdict": "game_ready",
            "equipmentReportPath": "equipment.json",
            "equipmentReportSha256": hash,
            "equipmentVerdict": "game_ready",
            "footwearReportPath": "footwear.json",
            "footwearReportSha256": hash,
            "footwearVerdict": "game_ready",
            "lateralityReportPath": "laterality.json",
            "lateralityReportSha256": hash,
            "lateralityVerdict": "game_ready",
            "retriedFrames": [2]
        });
        let schema: serde_json::Value =
            serde_json::from_str(GRID_KEYFRAME_ACTION_REPORT_SCHEMA).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&report));

        let mut wrong_guide = report.clone();
        wrong_guide["frames"][2]["poseStructureProfile"] =
            serde_json::json!("grid-pose-structure@1.1.0");
        assert!(!validator.is_valid(&wrong_guide));

        let mut missing_gate = report;
        missing_gate
            .as_object_mut()
            .unwrap()
            .remove("footwearVerdict");
        assert!(!validator.is_valid(&missing_gate));
    }
}
