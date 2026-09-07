use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::frames::FootAnchor;
use crate::quality::QualityReport;

use super::sheet::Atlas;

pub const SCHEMA_VERSION: &str = "1.0.0";
pub const GODOT_RENDERING_PROFILE_V1: &str = "godot-sprite-rendering@1.0.0";

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GodotTextureFilterV1 {
    #[default]
    Nearest,
    Linear,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CharacterMirrorPolicyV1 {
    #[default]
    Auto,
    RightOnly,
    ExplicitLeft,
    MirrorRightToLeft,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GodotRenderingContractV1 {
    #[serde(default = "godot_rendering_profile")]
    pub profile: String,
    #[serde(default)]
    pub texture_filter: GodotTextureFilterV1,
    #[serde(default = "default_pixel_snap")]
    pub pixel_snap: bool,
    #[serde(default)]
    pub mirror_policy: CharacterMirrorPolicyV1,
}

impl Default for GodotRenderingContractV1 {
    fn default() -> Self {
        Self {
            profile: GODOT_RENDERING_PROFILE_V1.into(),
            texture_filter: GodotTextureFilterV1::Nearest,
            pixel_snap: true,
            mirror_policy: CharacterMirrorPolicyV1::Auto,
        }
    }
}

fn godot_rendering_profile() -> String {
    GODOT_RENDERING_PROFILE_V1.into()
}

fn default_pixel_snap() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EngineManifest {
    pub name: String,
    pub sheet: ManifestSheet,
    pub animations: Vec<ManifestAnimation>,
    pub anchor: ManifestAnchor,
    #[serde(default)]
    pub rendering: GodotRenderingContractV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestSheet {
    pub image: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestAnimation {
    pub name: String,
    pub frames: Vec<usize>,
    pub fps: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frame_durations_ms: Vec<u64>,
    #[serde(rename = "loop")]
    pub loop_animation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManifestAnchor {
    #[serde(rename = "type")]
    pub anchor_type: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForgePackMetadata {
    pub schema_version: String,
    pub id: String,
    pub name: String,
    pub version: String,
    pub created_at: DateTime<Utc>,
    pub creator: PackCreator,
    pub license: PackLicense,
    pub source: PackSource,
    pub animations: Vec<ManifestAnimation>,
    pub assets: PackAssets,
    pub previews: PackPreviews,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackCreator {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackLicense {
    #[serde(rename = "type")]
    pub license_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PackSource {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackAssets {
    pub frames: String,
    pub sprite_sheet: String,
    pub atlas: String,
    pub manifest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub godot_helper: Option<String>,
    pub quality_report: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation_human_review: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PackPreviews {
    pub gif: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExportMetadata {
    pub manifest: EngineManifest,
    pub forgepack: ForgePackMetadata,
    pub quality_report: QualityReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackMetadataParams {
    pub id: String,
    pub name: String,
    pub version: String,
    pub creator_name: String,
    pub license_type: String,
    pub source_kind: String,
    pub source_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_metadata: Option<serde_json::Value>,
    pub animation_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation_frames: Option<Vec<usize>>,
    pub fps: f32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frame_durations_ms: Vec<u64>,
    pub loop_animation: bool,
    pub anchor: FootAnchor,
    #[serde(default)]
    pub rendering: GodotRenderingContractV1,
    pub quality_report: QualityReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterPackMetadataParams {
    pub id: String,
    pub name: String,
    pub version: String,
    pub creator_name: String,
    pub license_type: String,
    pub source_kind: String,
    pub source_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_metadata: Option<serde_json::Value>,
    pub default_animation: String,
    pub anchor: FootAnchor,
    #[serde(default)]
    pub rendering: GodotRenderingContractV1,
    pub quality_report: QualityReport,
}

pub fn engine_manifest(params: &PackMetadataParams, atlas: &Atlas) -> EngineManifest {
    let frames = params
        .animation_frames
        .clone()
        .filter(|frames| !frames.is_empty())
        .unwrap_or_else(|| (0..atlas.frames.len()).collect());

    EngineManifest {
        name: params.name.clone(),
        sheet: ManifestSheet {
            image: "assets/sprite_sheet.png".to_string(),
            images: atlas
                .images
                .iter()
                .map(|image| format!("assets/{image}"))
                .collect(),
            frame_width: atlas.frame_width,
            frame_height: atlas.frame_height,
            columns: atlas.columns,
            rows: atlas.rows,
        },
        animations: vec![ManifestAnimation {
            name: params.animation_name.clone(),
            frames,
            fps: params.fps,
            frame_durations_ms: params.frame_durations_ms.clone(),
            loop_animation: params.loop_animation,
        }],
        anchor: ManifestAnchor {
            anchor_type: if params.anchor.locked_by_user {
                "custom".to_string()
            } else {
                "feet".to_string()
            },
            x: params.anchor.x,
            y: params.anchor.y,
        },
        rendering: params.rendering.clone(),
    }
}

pub fn engine_manifest_for_animations(
    name: String,
    animations: Vec<ManifestAnimation>,
    anchor: FootAnchor,
    rendering: GodotRenderingContractV1,
    atlas: &Atlas,
) -> EngineManifest {
    EngineManifest {
        name,
        sheet: ManifestSheet {
            image: "assets/sprite_sheet.png".to_string(),
            images: atlas
                .images
                .iter()
                .map(|image| format!("assets/{image}"))
                .collect(),
            frame_width: atlas.frame_width,
            frame_height: atlas.frame_height,
            columns: atlas.columns,
            rows: atlas.rows,
        },
        animations,
        anchor: ManifestAnchor {
            anchor_type: if anchor.locked_by_user {
                "custom".to_string()
            } else {
                "feet".to_string()
            },
            x: anchor.x,
            y: anchor.y,
        },
        rendering,
    }
}

pub fn export_metadata(params: PackMetadataParams, atlas: &Atlas) -> ExportMetadata {
    let manifest = engine_manifest(&params, atlas);
    let forgepack = ForgePackMetadata {
        schema_version: SCHEMA_VERSION.to_string(),
        id: params.id,
        name: params.name,
        version: params.version,
        created_at: Utc::now(),
        creator: PackCreator {
            name: params.creator_name,
        },
        license: PackLicense {
            license_type: params.license_type,
            text: None,
            url: None,
        },
        source: PackSource {
            kind: params.source_kind,
            name: params.source_name,
            metadata: params
                .source_metadata
                .unwrap_or_else(|| serde_json::json!({})),
        },
        animations: manifest.animations.clone(),
        assets: PackAssets {
            frames: "assets/frames".to_string(),
            sprite_sheet: "assets/sprite_sheet.png".to_string(),
            atlas: "assets/atlas.json".to_string(),
            manifest: "assets/manifest.json".to_string(),
            godot_helper: Some("assets/godot_import.json".to_string()),
            quality_report: "quality-report.json".to_string(),
            animation_human_review: None,
        },
        previews: PackPreviews {
            gif: "previews/preview.gif".to_string(),
        },
    };

    ExportMetadata {
        manifest,
        forgepack,
        quality_report: params.quality_report,
    }
}

pub fn export_character_metadata(
    params: CharacterPackMetadataParams,
    animations: Vec<ManifestAnimation>,
    atlas: &Atlas,
) -> ExportMetadata {
    let manifest = engine_manifest_for_animations(
        params.name.clone(),
        animations,
        params.anchor,
        params.rendering,
        atlas,
    );
    let forgepack = ForgePackMetadata {
        schema_version: SCHEMA_VERSION.to_string(),
        id: params.id,
        name: params.name,
        version: params.version,
        created_at: Utc::now(),
        creator: PackCreator {
            name: params.creator_name,
        },
        license: PackLicense {
            license_type: params.license_type,
            text: None,
            url: None,
        },
        source: PackSource {
            kind: params.source_kind,
            name: params.source_name,
            metadata: params.source_metadata.unwrap_or_else(
                || serde_json::json!({ "defaultAnimation": params.default_animation }),
            ),
        },
        animations: manifest.animations.clone(),
        assets: PackAssets {
            frames: "assets/frames".to_string(),
            sprite_sheet: "assets/sprite_sheet.png".to_string(),
            atlas: "assets/atlas.json".to_string(),
            manifest: "assets/manifest.json".to_string(),
            godot_helper: Some("assets/godot_import.json".to_string()),
            quality_report: "quality-report.json".to_string(),
            animation_human_review: None,
        },
        previews: PackPreviews {
            gif: "previews/preview.gif".to_string(),
        },
    };

    ExportMetadata {
        manifest,
        forgepack,
        quality_report: params.quality_report,
    }
}
