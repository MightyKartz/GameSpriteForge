use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::frames::FootAnchor;
use crate::quality::QualityReport;

use super::sheet::Atlas;

pub const SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnimationRendering {
    #[serde(default)]
    pub profile: AnimationRenderingProfile,
    pub texture_filter: crate::asset_project::SamplingMode,
    pub pixel_snap: bool,
    #[serde(default)]
    pub mirror_policy: AnimationMirrorPolicy,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnimationRenderingProfile {
    #[default]
    #[serde(rename = "godot-sprite-rendering@1.0.0")]
    V1,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnimationMirrorPolicy {
    #[default]
    Auto,
}

impl Default for AnimationRendering {
    fn default() -> Self {
        Self {
            profile: AnimationRenderingProfile::V1,
            texture_filter: crate::asset_project::SamplingMode::Nearest,
            pixel_snap: true,
            mirror_policy: AnimationMirrorPolicy::Auto,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EngineManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering: Option<AnimationRendering>,
    pub name: String,
    pub sheet: ManifestSheet,
    pub animations: Vec<ManifestAnimation>,
    pub anchor: ManifestAnchor,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_durations_ms: Option<Vec<u32>>,
    pub name: String,
    pub frames: Vec<usize>,
    pub fps: f32,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering: Option<AnimationRendering>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_durations_ms: Option<Vec<u32>>,
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
    pub loop_animation: bool,
    pub anchor: FootAnchor,
    pub quality_report: QualityReport,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterPackMetadataParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rendering: Option<AnimationRendering>,
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
    pub quality_report: QualityReport,
}

pub fn engine_manifest(
    name: String,
    animation_name: String,
    fps: f32,
    loop_animation: bool,
    animation_frames: Option<Vec<usize>>,
    anchor: FootAnchor,
    atlas: &Atlas,
) -> EngineManifest {
    let frames = animation_frames
        .filter(|frames| !frames.is_empty())
        .unwrap_or_else(|| (0..atlas.frames.len()).collect());

    EngineManifest {
        rendering: None,
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
        animations: vec![ManifestAnimation {
            frame_durations_ms: None,
            name: animation_name,
            frames,
            fps,
            loop_animation,
        }],
        anchor: ManifestAnchor {
            anchor_type: if anchor.locked_by_user {
                "custom".to_string()
            } else {
                "feet".to_string()
            },
            x: anchor.x,
            y: anchor.y,
        },
    }
}

pub fn engine_manifest_for_animations(
    name: String,
    animations: Vec<ManifestAnimation>,
    anchor: FootAnchor,
    atlas: &Atlas,
) -> EngineManifest {
    EngineManifest {
        rendering: None,
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
    }
}

pub fn export_metadata(params: PackMetadataParams, atlas: &Atlas) -> ExportMetadata {
    let mut manifest = engine_manifest(
        params.name.clone(),
        params.animation_name.clone(),
        params.fps,
        params.loop_animation,
        params.animation_frames,
        params.anchor,
        atlas,
    );
    manifest.rendering = params.rendering;
    manifest.animations[0].frame_durations_ms = params.frame_durations_ms;
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
    let mut manifest =
        engine_manifest_for_animations(params.name.clone(), animations, params.anchor, atlas);
    manifest.rendering = params.rendering;
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
