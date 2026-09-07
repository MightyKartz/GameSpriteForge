use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use image::imageops::FilterType;
use image::{DynamicImage, ImageBuffer, Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::character_camera::CharacterCameraProfileV1;
use crate::character_direction::{assess_direction_anchor, DirectionViewV1};
use crate::character_direction_motion::CharacterMotionProfileV1;
use crate::collection::{CollectionGrounding, CollectionLockV1, StaticItemMetadataV1};
use crate::export::{
    build_preview_gif, build_sprite_sheet, GifBackground, PreviewGifParameters,
    SpriteSheetParameters,
};
use crate::frames::FrameBbox;
use crate::geometry::AssetGeometryReportV1;
use crate::matting::{apply_chroma_key, ChromaParameters};
use crate::portrait::{
    PortraitBaseApprovalV1, PortraitBaseLockV1, PortraitConsistencyReportV1,
    PORTRAIT_BASE_APPROVAL_FILE, PORTRAIT_BASE_LOCK_FILE, PORTRAIT_CONSISTENCY_REPORT_FILE,
};
use crate::provider::{GenerateImageRequest, MediaGenerationProvider, ProviderError};
use crate::quality::{QualityMetrics, QualityReport, QualityVerdict};

pub const FORGE_PROJECT_FILE: &str = "forge-project.json";
pub const STYLE_LOCK_FILE: &str = "style-lock.json";
pub const CONSISTENCY_PROFILE: &str = "consistency@1.5.0";
pub const CHARACTER_CONSISTENCY_PROFILE: &str = "consistency@1.6.0";
pub const CHARACTER_ACTION_CONSISTENCY_PROFILE: &str = "consistency@1.7.0";
pub const STYLE_BASELINE_PROFILE: &str = "style-baseline@2.3.0";
pub const KEYFRAME_HARD_GATE_PROFILE: &str = "keyframe-hard-defects@1.0.0";
pub const CHARACTER_IDENTITY_PROFILE: &str = "character-identity@1.1.0";
pub const CHARACTER_CANONICAL_IDENTITY_PROFILE: &str = "character-canonical-identity@1.0.0";
pub const CHARACTER_DIRECTION_PROFILE: &str = "direction-quality@1.2.0";
/// Legacy report default retained when reading pre-camera-profile Jobs.
pub const CHARACTER_CAMERA_PROFILE: &str = "topdown-3q-orthographic@1.0.0";
pub const CHARACTER_FRAMING_PROFILE: &str = "body-framing@1.0.0";
pub const CHARACTER_EQUIPMENT_EFFECT_PROFILE: &str = "equipment-effect-consistency@1.1.0";
pub const CHARACTER_SILHOUETTE_TEMPORAL_PROFILE: &str = "silhouette-temporal@2.0.0";
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
    #[cfg(feature = "pixel-delivery-v2")]
    #[error("pixel delivery error: {0}")]
    PixelGrid(#[from] crate::pixel_grid::PixelGridError),
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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub current_collection_revisions: BTreeMap<String, String>,
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterEquipmentKindV1 {
    #[default]
    None,
    StaffLike,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterEquipmentSpecV1 {
    #[serde(default)]
    pub kind: CharacterEquipmentKindV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_image: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterAssetSpecV2 {
    pub schema_version: String,
    pub kind: String,
    pub id: String,
    pub name: String,
    /// Legacy workflows require a SubjectLock. V8 may instead establish its
    /// canonical visual truth by generating `front_idle` directly from prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<SubjectRevisionRefV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    pub workflow: AssetWorkflowRefV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_profile: Option<CharacterCameraProfileV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equipment: Option<CharacterEquipmentSpecV1>,
    /// Optional generation/diagnostic guidance. It never limits which subject
    /// classes may be generated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub motion_profile: Option<CharacterMotionProfileV1>,
    /// Grid-only switch for pose-guide reference images. It is ignored by all
    /// existing workflows and defaults to enabled for backward compatibility.
    #[serde(default)]
    pub pose_guidance: crate::character_grid::GridPoseGuidanceV1,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticAssetKind {
    IconSet,
    PropSet,
    PortraitSet,
    EquipmentSet,
    DecalSet,
}

impl StaticAssetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IconSet => "icon_set",
            Self::PropSet => "prop_set",
            Self::PortraitSet => "portrait_set",
            Self::EquipmentSet => "equipment_set",
            Self::DecalSet => "decal_set",
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_anchor_similarity: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_anchor_geometry_similarity: Option<f32>,
    pub verdict: ConsistencyVerdict,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<AssetGeometryReportV1>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterIdentityMetricsV1 {
    pub visible_face_required: bool,
    pub face_color_component_pixels: u32,
    pub enclosed_feature_count: u32,
    pub enclosed_feature_pixel_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterIdentityReportV1 {
    pub schema_version: String,
    pub profile: String,
    pub verdict: ConsistencyVerdict,
    pub metrics: CharacterIdentityMetricsV1,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterSemanticAnimationReportV1 {
    pub name: String,
    pub expected_direction: String,
    pub frame_count: usize,
    pub visible_face_frames: usize,
    pub visible_face_ratio: f32,
    #[serde(default)]
    pub direction_match_frames: usize,
    #[serde(default)]
    pub direction_match_ratio: f32,
    #[serde(default)]
    pub maximum_consecutive_direction_mismatch_frames: usize,
    pub detached_emissive_effect_frames: usize,
    pub maximum_detached_emissive_components: usize,
    #[serde(default)]
    pub attached_emissive_halo_frames: usize,
    #[serde(default)]
    pub maximum_attached_emissive_halo_pixels: usize,
    #[serde(default)]
    pub body_top_margin_ratio: f32,
    #[serde(default)]
    pub body_height_ratio: f32,
    #[serde(default)]
    pub body_center_x_px: f32,
    #[serde(default)]
    pub foot_anchor_y_px: f32,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterSemanticQualityReportV1 {
    pub schema_version: String,
    pub direction_profile: String,
    #[serde(default = "default_character_camera_profile")]
    pub camera_profile: String,
    #[serde(default = "default_character_framing_profile")]
    pub framing_profile: String,
    pub equipment_effect_profile: String,
    pub verdict: ConsistencyVerdict,
    pub animations: Vec<CharacterSemanticAnimationReportV1>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterSilhouetteTemporalAnimationReportV1 {
    pub name: String,
    pub frame_count: usize,
    pub adjacent_core_mask_iou_min: f32,
    pub adjacent_core_mask_iou_median: f32,
    pub adjacent_contour_distance_px_max: f32,
    #[serde(default)]
    pub persistent_core_uncertainty_ratio: f32,
    #[serde(default)]
    pub unsupported_core_edge_ratio_max: f32,
    #[serde(default)]
    pub unsupported_full_edge_ratio_max: f32,
    #[serde(default)]
    pub edge_color_flicker_ratio_max: f32,
    pub body_width_variation_ratio: f32,
    pub body_height_variation_ratio: f32,
    pub body_center_step_px_max: f32,
    pub foot_anchor_step_px_max: f32,
    pub transitions: Vec<CharacterSilhouetteTemporalTransitionReportV1>,
    pub verdict: ConsistencyVerdict,
    #[serde(default)]
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterSilhouetteTemporalTransitionReportV1 {
    pub from_frame: usize,
    pub to_frame: usize,
    pub core_mask_iou: f32,
    pub contour_distance_px: f32,
    #[serde(default)]
    pub unsupported_core_edge_ratio: f32,
    #[serde(default)]
    pub unsupported_full_edge_ratio: f32,
    #[serde(default)]
    pub edge_color_flicker_ratio: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CharacterSilhouetteTemporalReportV1 {
    pub schema_version: String,
    pub profile: String,
    #[serde(default = "default_silhouette_evaluation_stage")]
    pub evaluation_stage: String,
    pub verdict: ConsistencyVerdict,
    pub animations: Vec<CharacterSilhouetteTemporalAnimationReportV1>,
}

fn default_silhouette_evaluation_stage() -> String {
    "post_repair".into()
}

/// Evaluates the frames that will be exported. The source-stage variant below
/// must also pass; post-processing can therefore never certify a defect that it
/// introduced or hid itself.
pub fn assess_character_silhouette_temporal(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
) -> CharacterSilhouetteTemporalReportV1 {
    assess_character_silhouette_temporal_stage(animations, "post_repair")
}

pub fn assess_character_silhouette_temporal_source(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
) -> CharacterSilhouetteTemporalReportV1 {
    assess_character_silhouette_temporal_stage(animations, "source")
}

fn assess_character_silhouette_temporal_stage(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    evaluation_stage: &str,
) -> CharacterSilhouetteTemporalReportV1 {
    let mut reports = Vec::with_capacity(animations.len());
    for (name, frames) in animations {
        let bodies = frames
            .iter()
            .filter_map(character_body_bbox)
            .collect::<Vec<_>>();
        let mut reasons = Vec::new();
        if frames.len() < 2 || bodies.len() != frames.len() {
            reasons.push("silhouette_frames_missing".into());
        }
        let mut adjacent_iou = Vec::new();
        let mut adjacent_contour = Vec::new();
        let mut transitions = Vec::new();
        let mut center_steps = Vec::new();
        let mut foot_steps = Vec::new();
        let mut unsupported_core_edges = Vec::new();
        let mut unsupported_full_edges = Vec::new();
        let mut edge_color_flicker = Vec::new();
        if bodies.len() == frames.len() && frames.len() >= 2 {
            for index in 0..frames.len() {
                let next = (index + 1) % frames.len();
                let (iou, contour) = aligned_upper_body_temporal_metrics(
                    &frames[index],
                    bodies[index],
                    &frames[next],
                    bodies[next],
                );
                adjacent_iou.push(iou);
                adjacent_contour.push(contour);
                let (unsupported_core, unsupported_full, color_flicker) =
                    aligned_full_body_edge_metrics(
                        &frames[index],
                        bodies[index],
                        &frames[next],
                        bodies[next],
                    );
                unsupported_core_edges.push(unsupported_core);
                unsupported_full_edges.push(unsupported_full);
                edge_color_flicker.push(color_flicker);
                transitions.push(CharacterSilhouetteTemporalTransitionReportV1 {
                    from_frame: index,
                    to_frame: next,
                    core_mask_iou: iou,
                    contour_distance_px: contour,
                    unsupported_core_edge_ratio: unsupported_core,
                    unsupported_full_edge_ratio: unsupported_full,
                    edge_color_flicker_ratio: color_flicker,
                });
                center_steps.push((bodies[index].center_x - bodies[next].center_x).abs());
                foot_steps.push((bodies[index].bottom_y - bodies[next].bottom_y).abs());
            }
        }
        let adjacent_core_mask_iou_min = min_f32(&adjacent_iou);
        let adjacent_core_mask_iou_median = median_f32(adjacent_iou);
        let adjacent_contour_distance_px_max = max_f32(&adjacent_contour);
        let persistent_core_uncertainty_ratio = temporal_core_uncertainty_ratio(frames, &bodies);
        let unsupported_core_edge_ratio_max = max_f32(&unsupported_core_edges);
        let unsupported_full_edge_ratio_max = max_f32(&unsupported_full_edges);
        let edge_color_flicker_ratio_max = max_f32(&edge_color_flicker);
        let widths = bodies.iter().map(|body| body.width).collect::<Vec<_>>();
        let heights = bodies.iter().map(|body| body.height).collect::<Vec<_>>();
        let body_width_variation_ratio = variation_ratio(&widths);
        let body_height_variation_ratio = variation_ratio(&heights);
        let body_center_step_px_max = max_f32(&center_steps);
        let foot_anchor_step_px_max = max_f32(&foot_steps);

        if adjacent_core_mask_iou_min < 0.90 || adjacent_core_mask_iou_median < 0.93 {
            reasons.push("upper_body_mask_flicker".into());
        }
        if adjacent_contour_distance_px_max > 2.0 {
            reasons.push("upper_body_contour_drift".into());
        }
        if persistent_core_uncertainty_ratio > 0.08 {
            reasons.push("persistent_core_boundary_drift".into());
        }
        if unsupported_core_edge_ratio_max > 0.12 {
            reasons.push("unsupported_core_edge_drift".into());
        }
        // Whole-body edge change is a hard gate only for idle. Locomotion
        // intentionally moves legs, arms, capes, and equipment; its full-body
        // ratio remains diagnostic while the persistent head/torso core is the
        // structural gate.
        if name == "idle" && unsupported_full_edge_ratio_max > 0.18 {
            reasons.push("unsupported_full_body_edge_drift".into());
        }
        if edge_color_flicker_ratio_max > 0.10 {
            reasons.push("temporal_edge_color_flicker".into());
        }
        if body_center_step_px_max > 2.0 {
            reasons.push("temporal_body_center_step".into());
        }
        if foot_anchor_step_px_max > 2.0 {
            reasons.push("temporal_foot_anchor_step".into());
        }
        let verdict = if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        };
        reports.push(CharacterSilhouetteTemporalAnimationReportV1 {
            name: name.clone(),
            frame_count: frames.len(),
            adjacent_core_mask_iou_min,
            adjacent_core_mask_iou_median,
            adjacent_contour_distance_px_max,
            persistent_core_uncertainty_ratio,
            unsupported_core_edge_ratio_max,
            unsupported_full_edge_ratio_max,
            edge_color_flicker_ratio_max,
            body_width_variation_ratio,
            body_height_variation_ratio,
            body_center_step_px_max,
            foot_anchor_step_px_max,
            transitions,
            verdict,
            reasons,
        });
    }
    CharacterSilhouetteTemporalReportV1 {
        schema_version: "1".into(),
        profile: CHARACTER_SILHOUETTE_TEMPORAL_PROFILE.into(),
        evaluation_stage: evaluation_stage.into(),
        verdict: if reports
            .iter()
            .all(|report| report.verdict == ConsistencyVerdict::GameReady)
        {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        animations: reports,
    }
}

/// Deterministic semantic gate for the fixed top-down locomotion contract.
/// Face visibility is deliberately used only where it is a reliable signal:
/// front-facing idle/down must retain a readable face, while up-facing frames
/// must not show one. Side-facing direction remains prompt-locked until the
/// optional vision component can provide a calibrated pose classifier.
/// Detached bright foreground components are forbidden in base locomotion so
/// transient magic effects remain separate runtime assets.
pub fn assess_character_animation_semantics(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    character_prompt: &str,
) -> CharacterSemanticQualityReportV1 {
    assess_character_animation_semantics_for_camera(
        animations,
        character_prompt,
        CharacterCameraProfileV1::default(),
    )
}

pub fn assess_character_animation_semantics_for_camera(
    animations: &BTreeMap<String, Vec<RgbaImage>>,
    character_prompt: &str,
    camera_profile: CharacterCameraProfileV1,
) -> CharacterSemanticQualityReportV1 {
    let mut reports = Vec::with_capacity(animations.len());
    for (name, frames) in animations {
        let expected_view = DirectionViewV1::for_animation(name);
        let direction_matches = expected_view
            .map(|view| {
                frames
                    .iter()
                    .map(|frame| {
                        assess_direction_anchor(frame, character_prompt, view).verdict
                            == ConsistencyVerdict::GameReady
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec![true; frames.len()]);
        let direction_match_frames = direction_matches.iter().filter(|matches| **matches).count();
        let direction_match_ratio =
            direction_match_frames as f32 / direction_matches.len().max(1) as f32;
        let maximum_consecutive_direction_mismatch_frames =
            maximum_cyclic_false_run(&direction_matches);
        let visible_face_frames = frames
            .iter()
            .filter(|frame| {
                let report = if matches!(name.as_str(), "idle" | "walk_down") {
                    assess_character_canonical_identity_reference(frame, character_prompt)
                } else {
                    assess_character_identity_reference(frame, character_prompt)
                };
                report.metrics.visible_face_required
                    && report.verdict == ConsistencyVerdict::GameReady
                    && report.metrics.enclosed_feature_pixel_ratio >= 0.02
            })
            .count();
        let frame_count = frames.len();
        let visible_face_ratio = visible_face_frames as f32 / frame_count.max(1) as f32;
        let effect_components = frames
            .iter()
            .map(detached_emissive_component_count)
            .collect::<Vec<_>>();
        let detached_emissive_effect_frames =
            effect_components.iter().filter(|count| **count > 0).count();
        let maximum_detached_emissive_components =
            effect_components.into_iter().max().unwrap_or_default();
        let halo_pixels = frames
            .iter()
            .map(attached_emissive_halo_pixel_count)
            .collect::<Vec<_>>();
        let attached_emissive_halo_frames =
            halo_pixels.iter().filter(|pixels| **pixels > 0).count();
        let maximum_attached_emissive_halo_pixels =
            halo_pixels.into_iter().max().unwrap_or_default();
        let body_boxes = frames
            .iter()
            .filter_map(character_body_bbox)
            .collect::<Vec<_>>();
        let body_top_margin_ratio = median_f32(
            body_boxes
                .iter()
                .map(|bbox| bbox.top / frames.first().map(RgbaImage::height).unwrap_or(1) as f32)
                .collect(),
        );
        let body_height_ratio = median_f32(
            body_boxes
                .iter()
                .map(|bbox| bbox.height / frames.first().map(RgbaImage::height).unwrap_or(1) as f32)
                .collect(),
        );
        let body_center_x_px = median_f32(body_boxes.iter().map(|bbox| bbox.center_x).collect());
        let foot_anchor_y_px = median_f32(body_boxes.iter().map(|bbox| bbox.bottom_y).collect());
        let mut reasons = Vec::new();
        let expected_direction = match name.as_str() {
            "idle" | "walk_down" => "down",
            "walk_up" => "up",
            "walk_right" => "right",
            _ => "unspecified",
        }
        .to_string();
        if !prompt_allows_hidden_face(character_prompt) {
            match name.as_str() {
                "idle" | "walk_down" if visible_face_ratio < 0.625 => {
                    reasons.push("front_direction_face_missing".into());
                }
                "walk_up" if visible_face_ratio > 0.125 => {
                    reasons.push("walk_up_face_visible".into());
                }
                _ => {}
            }
        }
        if !matches!(camera_profile, CharacterCameraProfileV1::LegacyThreeQuarter)
            && expected_view.is_some()
            && (direction_match_ratio < 0.875 || maximum_consecutive_direction_mismatch_frames > 1)
        {
            reasons.push("selected_interval_direction_drift".into());
        }
        if detached_emissive_effect_frames > 0 {
            reasons.push("unexpected_detached_emissive_effect".into());
        }
        if !character_prompt_allows_baked_emissive_effect(character_prompt)
            && attached_emissive_halo_frames > 0
        {
            reasons.push("unexpected_attached_emissive_halo".into());
        }
        if body_boxes.len() != frame_count {
            reasons.push("body_framing_foreground_missing".into());
        }
        reports.push(CharacterSemanticAnimationReportV1 {
            name: name.clone(),
            expected_direction,
            frame_count,
            visible_face_frames,
            visible_face_ratio,
            direction_match_frames,
            direction_match_ratio,
            maximum_consecutive_direction_mismatch_frames,
            detached_emissive_effect_frames,
            maximum_detached_emissive_components,
            attached_emissive_halo_frames,
            maximum_attached_emissive_halo_pixels,
            body_top_margin_ratio,
            body_height_ratio,
            body_center_x_px,
            foot_anchor_y_px,
            verdict: if reasons.is_empty() {
                ConsistencyVerdict::GameReady
            } else {
                ConsistencyVerdict::Blocked
            },
            reasons,
        });
    }
    if reports.len() > 1 {
        if let Some(idle) = reports.iter().find(|report| report.name == "idle").cloned() {
            for report in &mut reports {
                let canvas_height = animations
                    .get(&report.name)
                    .and_then(|frames| frames.first())
                    .map(RgbaImage::height)
                    .unwrap_or(1) as f32;
                let top_drift_px = (report.body_top_margin_ratio - idle.body_top_margin_ratio)
                    .abs()
                    * canvas_height;
                let height_ratio =
                    report.body_height_ratio / idle.body_height_ratio.max(f32::EPSILON);
                if report.body_top_margin_ratio > 0.18 || top_drift_px > 8.0 {
                    report.reasons.push("body_top_alignment_drift".into());
                }
                if !(0.93..=1.07).contains(&height_ratio) {
                    report.reasons.push("body_scale_drift".into());
                }
                if (report.body_center_x_px - idle.body_center_x_px).abs() > 6.0 {
                    report.reasons.push("body_center_drift".into());
                }
                if (report.foot_anchor_y_px - idle.foot_anchor_y_px).abs() > 3.0 {
                    report.reasons.push("foot_anchor_drift".into());
                }
                report.reasons.sort();
                report.reasons.dedup();
                report.verdict = if report.reasons.is_empty() {
                    ConsistencyVerdict::GameReady
                } else {
                    ConsistencyVerdict::Blocked
                };
            }
        }
    }
    CharacterSemanticQualityReportV1 {
        schema_version: "1".into(),
        direction_profile: CHARACTER_DIRECTION_PROFILE.into(),
        camera_profile: camera_profile.as_str().into(),
        framing_profile: CHARACTER_FRAMING_PROFILE.into(),
        equipment_effect_profile: CHARACTER_EQUIPMENT_EFFECT_PROFILE.into(),
        verdict: if reports
            .iter()
            .all(|report| report.verdict == ConsistencyVerdict::GameReady)
        {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        animations: reports,
    }
}

fn maximum_cyclic_false_run(values: &[bool]) -> usize {
    if values.is_empty() || values.iter().all(|value| *value) {
        return 0;
    }
    if values.iter().all(|value| !*value) {
        return values.len();
    }
    let mut maximum = 0usize;
    let mut current = 0usize;
    for value in values.iter().cycle().take(values.len() * 2) {
        if *value {
            current = 0;
        } else {
            current += 1;
            maximum = maximum.max(current.min(values.len()));
        }
    }
    maximum.min(values.len())
}

fn default_character_camera_profile() -> String {
    CHARACTER_CAMERA_PROFILE.into()
}

fn default_character_framing_profile() -> String {
    CHARACTER_FRAMING_PROFILE.into()
}

/// Estimates the body footprint while deliberately excluding narrow held
/// equipment such as a staff or sword. Dense horizontal rows locate the
/// head/feet, then horizontal quantiles reject thin side appendages.
pub fn character_body_bbox(image: &RgbaImage) -> Option<FrameBbox> {
    if image.width() == 0 || image.height() == 0 {
        return None;
    }
    let alpha_threshold = 48;
    let mut row_counts = vec![0usize; image.height() as usize];
    for (_, y, pixel) in image.enumerate_pixels() {
        if pixel[3] > alpha_threshold {
            row_counts[y as usize] += 1;
        }
    }
    let dense_row = ((image.width() as f32 * 0.07).round() as usize).max(10);
    let top = row_counts.iter().position(|count| *count >= dense_row)? as u32;
    let bottom = row_counts.iter().rposition(|count| *count >= 6)? as u32 + 1;
    if bottom <= top {
        return None;
    }
    let mut xs = Vec::new();
    let mut foreground = 0usize;
    for y in top..bottom {
        for x in 0..image.width() {
            if image.get_pixel(x, y)[3] > alpha_threshold {
                xs.push(x);
                foreground += 1;
            }
        }
    }
    if xs.len() < 32 {
        return None;
    }
    xs.sort_unstable();
    let trim = (xs.len() as f32 * 0.08).floor() as usize;
    let left = xs[trim.min(xs.len() - 1)];
    let right = xs[xs.len().saturating_sub(trim + 1)].saturating_add(1);
    if right <= left {
        return None;
    }
    Some(FrameBbox::from_bounds(
        left as f32,
        top as f32,
        right as f32,
        bottom as f32,
        foreground as f32 / (image.width() * image.height()) as f32,
    ))
}

fn aligned_upper_body_temporal_metrics(
    first: &RgbaImage,
    first_body: FrameBbox,
    second: &RgbaImage,
    second_body: FrameBbox,
) -> (f32, f32) {
    if first.dimensions() != second.dimensions() || first.width() == 0 || first.height() == 0 {
        return (0.0, 8.0);
    }
    let dx = (first_body.center_x - second_body.center_x).round() as i32;
    // Align from the gameplay root (body center + foot baseline), never from
    // the head edge being evaluated. Aligning by head-top hid vertical outline
    // drift in the previous profile.
    let dy = (first_body.bottom_y - second_body.bottom_y).round() as i32;
    let left = (first_body.left + first_body.width * 0.05).floor().max(0.0) as u32;
    let right = (first_body.right - first_body.width * 0.05)
        .ceil()
        .min(first.width() as f32) as u32;
    let top = first_body.top.floor().max(0.0) as u32;
    let bottom = (first_body.top + first_body.height * 0.60)
        .ceil()
        .min(first.height() as f32) as u32;
    if left >= right || top >= bottom {
        return (0.0, 8.0);
    }

    let mut intersection = 0usize;
    let mut union = 0usize;
    let len = first.width() as usize * first.height() as usize;
    let mut first_edges = vec![false; len];
    let mut second_edges = vec![false; len];
    for y in top..bottom {
        for x in left..right {
            let first_foreground = alpha_foreground(first, x as i32, y as i32);
            let second_x = x as i32 - dx;
            let second_y = y as i32 - dy;
            let second_foreground = alpha_foreground(second, second_x, second_y);
            if first_foreground || second_foreground {
                union += 1;
            }
            if first_foreground && second_foreground {
                intersection += 1;
            }
            let index = y as usize * first.width() as usize + x as usize;
            first_edges[index] = alpha_edge(first, x as i32, y as i32);
            second_edges[index] = alpha_edge(second, second_x, second_y);
        }
    }
    let iou = if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    };
    let contour = contour_distance(
        &first_edges,
        &second_edges,
        first.width(),
        first.height(),
        left,
        top,
        right,
        bottom,
    )
    .max(contour_distance(
        &second_edges,
        &first_edges,
        first.width(),
        first.height(),
        left,
        top,
        right,
        bottom,
    ));
    (iou, contour)
}

fn aligned_full_body_edge_metrics(
    first: &RgbaImage,
    first_body: FrameBbox,
    second: &RgbaImage,
    second_body: FrameBbox,
) -> (f32, f32, f32) {
    if first.dimensions() != second.dimensions() || first.width() == 0 || first.height() == 0 {
        return (1.0, 1.0, 1.0);
    }
    let dx = (first_body.center_x - second_body.center_x).round() as i32;
    let dy = (first_body.bottom_y - second_body.bottom_y).round() as i32;
    let full = (0, 0, first.width(), first.height());
    let core = (
        (first_body.left + first_body.width * 0.05).floor().max(0.0) as u32,
        first_body.top.floor().max(0.0) as u32,
        (first_body.right - first_body.width * 0.05)
            .ceil()
            .min(first.width() as f32) as u32,
        (first_body.top + first_body.height * 0.60)
            .ceil()
            .min(first.height() as f32) as u32,
    );
    let reverse_core = (
        (second_body.left + second_body.width * 0.05)
            .floor()
            .max(0.0) as u32,
        second_body.top.floor().max(0.0) as u32,
        (second_body.right - second_body.width * 0.05)
            .ceil()
            .min(second.width() as f32) as u32,
        (second_body.top + second_body.height * 0.60)
            .ceil()
            .min(second.height() as f32) as u32,
    );
    let (core_forward, core_color_forward) = unsupported_edge_ratio(first, second, dx, dy, core, 2);
    let (core_reverse, core_color_reverse) =
        unsupported_edge_ratio(second, first, -dx, -dy, reverse_core, 2);
    let (full_forward, full_color_forward) = unsupported_edge_ratio(first, second, dx, dy, full, 2);
    let (full_reverse, full_color_reverse) =
        unsupported_edge_ratio(second, first, -dx, -dy, full, 2);
    (
        core_forward.max(core_reverse),
        full_forward.max(full_reverse),
        core_color_forward
            .max(core_color_reverse)
            .max(full_color_forward.min(full_color_reverse)),
    )
}

fn unsupported_edge_ratio(
    source: &RgbaImage,
    target: &RgbaImage,
    dx: i32,
    dy: i32,
    bounds: (u32, u32, u32, u32),
    radius: i32,
) -> (f32, f32) {
    let mut edge_count = 0usize;
    let mut unsupported = 0usize;
    let mut color_compared = 0usize;
    let mut color_flicker = 0usize;
    for y in bounds.1..bounds.3 {
        for x in bounds.0..bounds.2 {
            if !alpha_edge(source, x as i32, y as i32) {
                continue;
            }
            edge_count += 1;
            let target_x = x as i32 - dx;
            let target_y = y as i32 - dy;
            let mut nearest: Option<(i32, i32, i32)> = None;
            for offset_y in -radius..=radius {
                for offset_x in -radius..=radius {
                    let candidate_x = target_x + offset_x;
                    let candidate_y = target_y + offset_y;
                    if !alpha_edge(target, candidate_x, candidate_y) {
                        continue;
                    }
                    let distance_squared = offset_x * offset_x + offset_y * offset_y;
                    if nearest
                        .is_none_or(|(_, _, nearest_distance)| distance_squared < nearest_distance)
                    {
                        nearest = Some((candidate_x, candidate_y, distance_squared));
                    }
                }
            }
            let Some((matched_x, matched_y, _)) = nearest else {
                unsupported += 1;
                continue;
            };
            color_compared += 1;
            let first = source.get_pixel(x, y);
            let second = target.get_pixel(matched_x as u32, matched_y as u32);
            let distance = (0..3)
                .map(|channel| {
                    let delta = first[channel] as f32 - second[channel] as f32;
                    delta * delta
                })
                .sum::<f32>()
                .sqrt()
                / 441.67294;
            if distance > 0.18 {
                color_flicker += 1;
            }
        }
    }
    (
        unsupported as f32 / edge_count.max(1) as f32,
        color_flicker as f32 / color_compared.max(1) as f32,
    )
}

fn temporal_core_uncertainty_ratio(frames: &[RgbaImage], bodies: &[FrameBbox]) -> f32 {
    if frames.len() < 2 || bodies.len() != frames.len() || frames[0].width() == 0 {
        return 1.0;
    }
    let reference = bodies[0];
    let left = (reference.left + reference.width * 0.05).floor().max(0.0) as u32;
    let right = (reference.right - reference.width * 0.05)
        .ceil()
        .min(frames[0].width() as f32) as u32;
    let top = reference.top.floor().max(0.0) as u32;
    let bottom = (reference.top + reference.height * 0.60)
        .ceil()
        .min(frames[0].height() as f32) as u32;
    if left >= right || top >= bottom {
        return 1.0;
    }
    let lower = ((frames.len() as f32) * 0.25).ceil() as usize;
    let upper = ((frames.len() as f32) * 0.75).floor() as usize;
    let mut uncertain = 0usize;
    let mut median_foreground_area = Vec::with_capacity(frames.len());
    for (frame, body) in frames.iter().zip(bodies) {
        let dx = (reference.center_x - body.center_x).round() as i32;
        let dy = (reference.bottom_y - body.bottom_y).round() as i32;
        let mut area = 0usize;
        for y in top..bottom {
            for x in left..right {
                if alpha_foreground(frame, x as i32 - dx, y as i32 - dy) {
                    area += 1;
                }
            }
        }
        median_foreground_area.push(area as f32);
    }
    for y in top..bottom {
        for x in left..right {
            let occupied = frames
                .iter()
                .zip(bodies)
                .filter(|(frame, body)| {
                    let dx = (reference.center_x - body.center_x).round() as i32;
                    let dy = (reference.bottom_y - body.bottom_y).round() as i32;
                    alpha_foreground(frame, x as i32 - dx, y as i32 - dy)
                })
                .count();
            if occupied >= lower && occupied <= upper {
                uncertain += 1;
            }
        }
    }
    uncertain as f32 / median_f32(median_foreground_area).max(1.0)
}

fn alpha_foreground(image: &RgbaImage, x: i32, y: i32) -> bool {
    x >= 0
        && y >= 0
        && x < image.width() as i32
        && y < image.height() as i32
        && image.get_pixel(x as u32, y as u32)[3] > 48
}

fn alpha_edge(image: &RgbaImage, x: i32, y: i32) -> bool {
    alpha_foreground(image, x, y)
        && [(x - 1, y), (x + 1, y), (x, y - 1), (x, y + 1)]
            .iter()
            .any(|(neighbor_x, neighbor_y)| !alpha_foreground(image, *neighbor_x, *neighbor_y))
}

#[allow(clippy::too_many_arguments)]
fn contour_distance(
    source: &[bool],
    target: &[bool],
    width: u32,
    height: u32,
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
) -> f32 {
    let mut total = 0.0;
    let mut count = 0usize;
    let radius = 6i32;
    for y in top..bottom {
        for x in left..right {
            let index = y as usize * width as usize + x as usize;
            if !source[index] {
                continue;
            }
            count += 1;
            let mut nearest_squared = (radius + 1) * (radius + 1);
            for dy in -radius..=radius {
                for dx in -radius..=radius {
                    let candidate_x = x as i32 + dx;
                    let candidate_y = y as i32 + dy;
                    if candidate_x < 0
                        || candidate_y < 0
                        || candidate_x >= width as i32
                        || candidate_y >= height as i32
                    {
                        continue;
                    }
                    let candidate = candidate_y as usize * width as usize + candidate_x as usize;
                    if target[candidate] {
                        nearest_squared = nearest_squared.min(dx * dx + dy * dy);
                    }
                }
            }
            total += (nearest_squared as f32).sqrt();
        }
    }
    if count == 0 {
        radius as f32 + 1.0
    } else {
        total / count as f32
    }
}

fn attached_emissive_halo_pixel_count(image: &RgbaImage) -> usize {
    let count = attached_emissive_halo_pixels(image).len();
    let threshold = ((image.width() * image.height()) as f32 * 0.000_12).ceil() as usize;
    if count >= threshold.max(6) {
        count
    } else {
        0
    }
}

/// Removes only translucent pixels in the ring around a very bright attached
/// equipment core. Opaque crystal pixels and the two-pixel antialiasing edge
/// remain intact; this is intentionally narrower than generic glow removal.
pub fn strip_unexpected_attached_emissive_halo(image: &mut RgbaImage) -> usize {
    let pixels = attached_emissive_halo_pixels(image);
    for (x, y) in &pixels {
        image.put_pixel(*x, *y, Rgba([0, 0, 0, 0]));
    }
    pixels.len()
}

fn attached_emissive_halo_pixels(image: &RgbaImage) -> Vec<(u32, u32)> {
    let cores = image
        .enumerate_pixels()
        .filter(|(_, _, pixel)| {
            pixel[3] >= 224
                && pixel.0.iter().take(3).copied().max().unwrap_or_default() >= 220
                && pixel_luminance(pixel) >= 185.0
                && is_emissive_pixel(pixel)
        })
        .map(|(x, y, _)| (x, y))
        .collect::<Vec<_>>();
    if cores.is_empty() {
        return Vec::new();
    }
    let radius = ((image.width().max(image.height()) as f32 / 32.0).round() as i32).clamp(8, 24);
    let radius_squared = radius * radius;
    let width = image.width() as i32;
    let height = image.height() as i32;
    let mut near_core = vec![false; image.width() as usize * image.height() as usize];
    for (core_x, core_y) in cores {
        let core_x = core_x as i32;
        let core_y = core_y as i32;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let distance = dx * dx + dy * dy;
                if !(9..=radius_squared).contains(&distance) {
                    continue;
                }
                let x = core_x + dx;
                let y = core_y + dy;
                if x >= 0 && y >= 0 && x < width && y < height {
                    near_core[y as usize * image.width() as usize + x as usize] = true;
                }
            }
        }
    }
    image
        .enumerate_pixels()
        .filter_map(|(x, y, pixel)| {
            if !(12..=224).contains(&pixel[3]) {
                return None;
            }
            near_core[y as usize * image.width() as usize + x as usize].then_some((x, y))
        })
        .collect()
}

pub fn character_prompt_allows_baked_emissive_effect(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    [
        "baked glow",
        "glowing aura",
        "emitting light",
        "light spill",
        "magic halo",
        "permanent aura",
    ]
    .iter()
    .any(|term| prompt.contains(term))
}

fn median_f32(mut values: Vec<f32>) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f32::total_cmp);
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}

fn min_f32(values: &[f32]) -> f32 {
    values.iter().copied().reduce(f32::min).unwrap_or_default()
}

fn max_f32(values: &[f32]) -> f32 {
    values.iter().copied().reduce(f32::max).unwrap_or_default()
}

fn variation_ratio(values: &[f32]) -> f32 {
    let median = median_f32(values.to_vec());
    if median <= f32::EPSILON {
        return 0.0;
    }
    (max_f32(values) - min_f32(values)) / median
}

fn detached_emissive_component_count(image: &RgbaImage) -> usize {
    let mut foreground = BTreeSet::<(u32, u32)>::new();
    for y in 0..image.height() {
        for x in 0..image.width() {
            if image.get_pixel(x, y)[3] > 64 {
                foreground.insert((x, y));
            }
        }
    }
    let mut visited = BTreeSet::<(u32, u32)>::new();
    let mut components = Vec::<Vec<(u32, u32)>>::new();
    for start in foreground.iter().copied() {
        if !visited.insert(start) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        let mut component = Vec::new();
        while let Some((x, y)) = queue.pop_front() {
            component.push((x, y));
            for neighbor in orthogonal_neighbors(x, y, image.width(), image.height()) {
                if foreground.contains(&neighbor) && visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
        components.push(component);
    }
    components.sort_by_key(|component| std::cmp::Reverse(component.len()));
    let main_area = components.first().map(Vec::len).unwrap_or_default();
    components
        .iter()
        .skip(1)
        .filter(|component| {
            component.len() >= 5
                && component.len() <= (main_area / 20).max(5)
                && component
                    .iter()
                    .filter(|(x, y)| is_emissive_pixel(image.get_pixel(*x, *y)))
                    .count()
                    >= (component.len() / 2).max(3)
        })
        .count()
}

fn is_emissive_pixel(pixel: &Rgba<u8>) -> bool {
    let rgb = [pixel[0], pixel[1], pixel[2]];
    let spread = rgb.iter().max().unwrap_or(&0) - rgb.iter().min().unwrap_or(&0);
    pixel[3] > 64 && spread >= 40 && pixel_luminance(pixel) >= 120.0
}

/// Deterministic, deliberately conservative pre-video identity gate. It does
/// not claim general face recognition; it only verifies that an upper-center
/// warm face region contains at least two enclosed dark feature components
/// (normally eyes and/or mouth). Explicitly masked, helmeted, robotic, or
/// faceless character prompts opt out of this visible-face contract.
pub fn assess_character_identity_reference(
    image: &RgbaImage,
    character_prompt: &str,
) -> CharacterIdentityReportV1 {
    let visible_face_required = !prompt_allows_hidden_face(character_prompt);
    let mut reasons = Vec::new();
    let Some((left, top, right, bottom)) = alpha_bounds(image) else {
        return CharacterIdentityReportV1 {
            schema_version: "1".into(),
            profile: CHARACTER_IDENTITY_PROFILE.into(),
            verdict: ConsistencyVerdict::Blocked,
            metrics: CharacterIdentityMetricsV1 {
                visible_face_required,
                face_color_component_pixels: 0,
                enclosed_feature_count: 0,
                enclosed_feature_pixel_ratio: 0.0,
            },
            reasons: vec!["character_identity_foreground_missing".into()],
        };
    };
    if !visible_face_required {
        return CharacterIdentityReportV1 {
            schema_version: "1".into(),
            profile: CHARACTER_IDENTITY_PROFILE.into(),
            verdict: ConsistencyVerdict::GameReady,
            metrics: CharacterIdentityMetricsV1 {
                visible_face_required,
                face_color_component_pixels: 0,
                enclosed_feature_count: 0,
                enclosed_feature_pixel_ratio: 0.0,
            },
            reasons,
        };
    }

    let width = right - left + 1;
    let height = bottom - top + 1;
    let search_left = left + width * 20 / 100;
    let search_right = right.saturating_sub(width * 20 / 100);
    let search_top = top + height * 10 / 100;
    let search_bottom = (top + height * 48 / 100).min(bottom);
    let expected_x = left + width / 2;
    let expected_y = top + height * 29 / 100;
    let seed_radius_x = (width / 12).max(4);
    let seed_radius_y = (height / 12).max(4);
    let mut seed = None::<(u32, u32, Rgba<u8>, f32)>;
    for y in
        expected_y.saturating_sub(seed_radius_y)..=(expected_y + seed_radius_y).min(search_bottom)
    {
        for x in expected_x.saturating_sub(seed_radius_x)
            ..=(expected_x + seed_radius_x).min(search_right)
        {
            if x < search_left || y < search_top {
                continue;
            }
            let pixel = *image.get_pixel(x, y);
            if !probable_character_skin_seed(&pixel) {
                continue;
            }
            let luminance = pixel_luminance(&pixel);
            if seed
                .as_ref()
                .is_none_or(|(_, _, _, current)| luminance > *current)
            {
                seed = Some((x, y, pixel, luminance));
            }
        }
    }
    let Some((seed_x, seed_y, seed_color, seed_luminance)) = seed else {
        reasons.push("character_visible_face_region_missing".into());
        return CharacterIdentityReportV1 {
            schema_version: "1".into(),
            profile: CHARACTER_IDENTITY_PROFILE.into(),
            verdict: ConsistencyVerdict::Blocked,
            metrics: CharacterIdentityMetricsV1 {
                visible_face_required,
                face_color_component_pixels: 0,
                enclosed_feature_count: 0,
                enclosed_feature_pixel_ratio: 0.0,
            },
            reasons,
        };
    };

    let mut component = BTreeSet::<(u32, u32)>::new();
    let mut queue = VecDeque::from([(seed_x, seed_y)]);
    component.insert((seed_x, seed_y));
    while let Some((x, y)) = queue.pop_front() {
        for (next_x, next_y) in orthogonal_neighbors(x, y, image.width(), image.height()) {
            if next_x < search_left
                || next_x > search_right
                || next_y < search_top
                || next_y > search_bottom
                || component.contains(&(next_x, next_y))
            {
                continue;
            }
            let pixel = image.get_pixel(next_x, next_y);
            if probable_character_skin(pixel) && rgb_distance_squared(pixel, &seed_color) <= 75 * 75
            {
                component.insert((next_x, next_y));
                queue.push_back((next_x, next_y));
            }
        }
    }
    let minimum_face_pixels = ((width as usize * height as usize) / 2000).max(24);
    if component.len() < minimum_face_pixels {
        reasons.push("character_visible_face_region_missing".into());
        return CharacterIdentityReportV1 {
            schema_version: "1".into(),
            profile: CHARACTER_IDENTITY_PROFILE.into(),
            verdict: ConsistencyVerdict::Blocked,
            metrics: CharacterIdentityMetricsV1 {
                visible_face_required,
                face_color_component_pixels: component.len().min(u32::MAX as usize) as u32,
                enclosed_feature_count: 0,
                enclosed_feature_pixel_ratio: 0.0,
            },
            reasons,
        };
    }

    let component_left = component.iter().map(|(x, _)| *x).min().unwrap_or(seed_x);
    let component_right = component.iter().map(|(x, _)| *x).max().unwrap_or(seed_x);
    let component_top = component.iter().map(|(_, y)| *y).min().unwrap_or(seed_y);
    let component_bottom = component.iter().map(|(_, y)| *y).max().unwrap_or(seed_y);
    let expand_x = ((component_right - component_left + 1) / 6).max(3);
    let expand_y = ((component_bottom - component_top + 1) / 6).max(3);
    let feature_left = component_left.saturating_sub(expand_x).max(search_left);
    let feature_right = (component_right + expand_x).min(search_right);
    let feature_top = component_top.saturating_sub(expand_y).max(search_top);
    let feature_bottom = (component_bottom + expand_y).min(search_bottom);
    let mut dark_pixels = BTreeSet::<(u32, u32)>::new();
    for y in feature_top..=feature_bottom {
        for x in feature_left..=feature_right {
            let pixel = image.get_pixel(x, y);
            if pixel[3] > 16 && pixel_luminance(pixel) + 45.0 < seed_luminance {
                dark_pixels.insert((x, y));
            }
        }
    }
    let mut visited = BTreeSet::<(u32, u32)>::new();
    let minimum_feature_pixels = if image.width().max(image.height()) <= 128 {
        1
    } else {
        (component.len() / 1200).max(3)
    };
    let mut feature_count = 0u32;
    let mut feature_pixels = 0usize;
    for start in dark_pixels.iter().copied() {
        if visited.contains(&start) {
            continue;
        }
        let mut feature_queue = VecDeque::from([start]);
        visited.insert(start);
        let mut area = 0usize;
        let mut touches_boundary = false;
        while let Some((x, y)) = feature_queue.pop_front() {
            area += 1;
            touches_boundary |=
                x == feature_left || x == feature_right || y == feature_top || y == feature_bottom;
            for neighbor in orthogonal_neighbors(x, y, image.width(), image.height()) {
                if dark_pixels.contains(&neighbor) && visited.insert(neighbor) {
                    feature_queue.push_back(neighbor);
                }
            }
        }
        if !touches_boundary && area >= minimum_feature_pixels {
            feature_count = feature_count.saturating_add(1);
            feature_pixels = feature_pixels.saturating_add(area);
        }
    }
    let feature_ratio = feature_pixels as f32 / component.len().max(1) as f32;
    if feature_count < 2 || feature_ratio < 0.003 {
        reasons.push("character_face_detail_missing".into());
    }
    CharacterIdentityReportV1 {
        schema_version: "1".into(),
        profile: CHARACTER_IDENTITY_PROFILE.into(),
        verdict: if reasons.is_empty() {
            ConsistencyVerdict::GameReady
        } else {
            ConsistencyVerdict::Blocked
        },
        metrics: CharacterIdentityMetricsV1 {
            visible_face_required,
            face_color_component_pixels: component.len().min(u32::MAX as usize) as u32,
            enclosed_feature_count: feature_count,
            enclosed_feature_pixel_ratio: feature_ratio,
        },
        reasons,
    }
}

/// Canonical-reference-only fallback for painted sprites whose brightest warm
/// pixel is an isolated highlight. Animation direction semantics deliberately
/// continue to use [`assess_character_identity_reference`] so a rear-facing
/// hood or costume detail cannot be reclassified as a visible face.
pub fn assess_character_canonical_identity_reference(
    image: &RgbaImage,
    character_prompt: &str,
) -> CharacterIdentityReportV1 {
    let primary = assess_character_identity_reference(image, character_prompt);
    if primary.verdict == ConsistencyVerdict::GameReady || !primary.metrics.visible_face_required {
        return primary;
    }
    if primary.reasons.iter().any(|reason| {
        !matches!(
            reason.as_str(),
            "character_visible_face_region_missing" | "character_face_detail_missing"
        )
    }) {
        return primary;
    }
    let Some((left, top, right, bottom)) = alpha_bounds(image) else {
        return primary;
    };
    let width = right - left + 1;
    let height = bottom - top + 1;
    let expected_x = left + width / 2;
    let face_left = expected_x.saturating_sub(width * 18 / 100).max(left);
    let face_right = (expected_x + width * 18 / 100).min(right);
    let face_top = (top + height * 16 / 100).min(bottom);
    let face_bottom = (top + height * 42 / 100).min(bottom);
    if face_left >= face_right || face_top >= face_bottom {
        return primary;
    }

    let mut skin_luminances = Vec::new();
    for y in face_top..=face_bottom {
        for x in face_left..=face_right {
            let pixel = image.get_pixel(x, y);
            if probable_character_skin(pixel) {
                skin_luminances.push(pixel_luminance(pixel));
            }
        }
    }
    let minimum_face_pixels = ((width as usize * height as usize) / 2000).max(24);
    if skin_luminances.len() < minimum_face_pixels {
        return primary;
    }
    skin_luminances.sort_by(f32::total_cmp);
    let reference_luminance = skin_luminances[skin_luminances.len().saturating_mul(4) / 5];
    let mut dark_pixels = BTreeSet::<(u32, u32)>::new();
    for y in face_top..=face_bottom {
        for x in face_left..=face_right {
            let pixel = image.get_pixel(x, y);
            if pixel[3] > 16 && pixel_luminance(pixel) + 35.0 < reference_luminance {
                dark_pixels.insert((x, y));
            }
        }
    }
    let minimum_feature_pixels = if image.width().max(image.height()) <= 128 {
        1
    } else {
        3
    };
    let mut visited = BTreeSet::<(u32, u32)>::new();
    let mut feature_count = 0u32;
    let mut feature_pixels = 0usize;
    for start in dark_pixels.iter().copied() {
        if visited.contains(&start) {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        visited.insert(start);
        let mut area = 0usize;
        let mut touches_boundary = false;
        while let Some((x, y)) = queue.pop_front() {
            area += 1;
            touches_boundary |=
                x == face_left || x == face_right || y == face_top || y == face_bottom;
            for neighbor in orthogonal_neighbors(x, y, image.width(), image.height()) {
                if dark_pixels.contains(&neighbor) && visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
        if !touches_boundary && area >= minimum_feature_pixels {
            feature_count = feature_count.saturating_add(1);
            feature_pixels = feature_pixels.saturating_add(area);
        }
    }
    let feature_ratio = feature_pixels as f32 / skin_luminances.len().max(1) as f32;
    if feature_count < 2 || feature_ratio < 0.003 {
        return primary;
    }
    CharacterIdentityReportV1 {
        schema_version: "1".into(),
        profile: CHARACTER_CANONICAL_IDENTITY_PROFILE.into(),
        verdict: ConsistencyVerdict::GameReady,
        metrics: CharacterIdentityMetricsV1 {
            visible_face_required: true,
            face_color_component_pixels: skin_luminances.len().min(u32::MAX as usize) as u32,
            enclosed_feature_count: feature_count,
            enclosed_feature_pixel_ratio: feature_ratio,
        },
        reasons: Vec::new(),
    }
}

fn prompt_allows_hidden_face(prompt: &str) -> bool {
    let prompt = prompt.to_ascii_lowercase();
    [
        "faceless",
        "featureless face",
        "face hidden",
        "hidden face",
        "masked",
        "full mask",
        "closed helmet",
        "robot",
        "no visible face",
    ]
    .iter()
    .any(|term| prompt.contains(term))
}

fn probable_character_skin(pixel: &Rgba<u8>) -> bool {
    if pixel[3] <= 16 {
        return false;
    }
    let [red, green, blue, _] = pixel.0;
    let maximum = red.max(green).max(blue);
    let minimum = red.min(green).min(blue);
    red.saturating_add(8) >= green
        && green.saturating_add(12) >= blue
        && red > blue.saturating_add(10)
        && maximum.saturating_sub(minimum) >= 12
        && maximum >= 55
        && minimum <= 235
}

fn probable_character_skin_seed(pixel: &Rgba<u8>) -> bool {
    // The broader skin predicate is appropriate for flood filling shaded
    // faces, but seed selection must exclude near-neutral eye whites and
    // highlights. Otherwise the brightest plausible pixel can be a sclera
    // and the selected component shrinks to a few dozen pixels.
    probable_character_skin(pixel) && pixel[0].saturating_sub(pixel[1]) >= 24
}

fn pixel_luminance(pixel: &Rgba<u8>) -> f32 {
    0.2126 * f32::from(pixel[0]) + 0.7152 * f32::from(pixel[1]) + 0.0722 * f32::from(pixel[2])
}

fn rgb_distance_squared(left: &Rgba<u8>, right: &Rgba<u8>) -> u32 {
    (0..3)
        .map(|channel| {
            let delta = i32::from(left[channel]) - i32::from(right[channel]);
            (delta * delta) as u32
        })
        .sum()
}

fn orthogonal_neighbors(x: u32, y: u32, width: u32, height: u32) -> Vec<(u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors
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
    pub foreground_width_ratio: f32,
    pub foreground_height_ratio: f32,
    pub foreground_area_ratio: f32,
    pub centroid_y_ratio: f32,
    pub detached_component_ratio: f32,
    pub bottom_band_span_ratio: f32,
    pub bottom_band_area_share: f32,
    pub platform_overhang_ratio: f32,
    pub row_occupancy_profile: Vec<f32>,
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

pub struct StaticPackContext<'a> {
    pub provider_id: &'a str,
    pub item_metadata: &'a BTreeMap<String, StaticItemMetadataV1>,
    pub collection: Option<&'a CollectionLockV1>,
    pub manual_item_ids: &'a BTreeSet<String>,
    pub portrait_base: Option<&'a PortraitBaseLockV1>,
    pub portrait_approval: Option<&'a PortraitBaseApprovalV1>,
    pub portrait_report: Option<&'a PortraitConsistencyReportV1>,
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
        current_collection_revisions: BTreeMap::new(),
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
                authorization_target: Some("style_board".into()),
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
    #[cfg(feature = "pixel-delivery-v2")]
    crate::pixel_grid::write_style_palette_lock(project_root, &lock, 32)?;
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
    let keyed = matte_static_image(input)?;
    normalize_matted_static_image(&keyed, output, canvas_size, bottom_anchor)
}

/// Crops and scales an image that has already crossed a deterministic
/// background-cleanup boundary. This deliberately performs no second matting
/// pass, so its input hash is the exact image propagated to downstream nodes.
pub fn normalize_matted_static_image(
    keyed: &RgbaImage,
    output: &Path,
    canvas_size: u32,
    bottom_anchor: bool,
) -> Result<RgbaImage, AssetProjectError> {
    validate_canvas_size(canvas_size)?;
    let bbox = alpha_bounds(keyed).ok_or_else(|| {
        AssetProjectError::Invalid("generated image has no foreground after matting".into())
    })?;
    let cropped = image::imageops::crop_imm(
        keyed,
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

/// Character-delivery-only pixel-grid normalization. Existing callers keep
/// `normalize_matted_static_image`; only character workflows opt into this
/// path when `pixel-delivery-v2` is explicitly compiled in.
#[cfg(feature = "pixel-delivery-v2")]
pub fn normalize_matted_static_image_character_delivery(
    keyed: &RgbaImage,
    output: &Path,
    canvas_size: u32,
    bottom_anchor: bool,
) -> Result<RgbaImage, AssetProjectError> {
    validate_canvas_size(canvas_size)?;
    let bbox = alpha_bounds(keyed).ok_or_else(|| {
        AssetProjectError::Invalid("generated image has no foreground after matting".into())
    })?;
    let cropped = image::imageops::crop_imm(
        keyed,
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
    let palette = crate::pixel_grid::PaletteLockV1::derive(
        &cropped,
        crate::pixel_grid::PaletteSourceV1 {
            kind: "canonical_idle".into(),
            revision: None,
            path: output.to_path_buf(),
            sha256: crate::pixel_grid::rgba_sha256(&cropped),
        },
        24,
    )?;
    let (resized, report) =
        crate::pixel_grid::resize_for_delivery(&cropped, width, height, &palette)?;
    crate::pixel_grid::write_json_atomic(&output.with_extension("pixel-delivery.json"), &report)?;
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
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    canvas.save(output)?;
    Ok(canvas)
}

/// Decode and deterministically matte a provider image without cropping or
/// rescaling it. Geometry gates use this pre-normalization image so a bust
/// cannot be made to look full-body merely by filling the output canvas.
pub fn matte_static_image(input: &Path) -> Result<RgbaImage, AssetProjectError> {
    let source = image::open(input)?.to_rgba8();
    apply_chroma_key(&source, &ChromaParameters::default())
        .map_err(|error| AssetProjectError::Invalid(error.to_string()))
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
    let geometry = foreground_geometry(image, bounds);
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
        foreground_width_ratio: geometry.width_ratio,
        foreground_height_ratio: geometry.height_ratio,
        foreground_area_ratio: geometry.area_ratio,
        centroid_y_ratio: geometry.centroid_y_ratio,
        detached_component_ratio: geometry.detached_component_ratio,
        bottom_band_span_ratio: geometry.bottom_band_span_ratio,
        bottom_band_area_share: geometry.bottom_band_area_share,
        platform_overhang_ratio: geometry.platform_overhang_ratio,
        row_occupancy_profile: geometry.row_occupancy_profile,
    }
}

#[derive(Default)]
struct ForegroundGeometry {
    width_ratio: f32,
    height_ratio: f32,
    area_ratio: f32,
    centroid_y_ratio: f32,
    detached_component_ratio: f32,
    bottom_band_span_ratio: f32,
    bottom_band_area_share: f32,
    platform_overhang_ratio: f32,
    row_occupancy_profile: Vec<f32>,
}

fn foreground_geometry(
    image: &RgbaImage,
    bounds: Option<(u32, u32, u32, u32)>,
) -> ForegroundGeometry {
    const PROFILE_BINS: usize = 16;
    let Some((left, top, right, bottom)) = bounds else {
        return ForegroundGeometry {
            row_occupancy_profile: vec![0.0; PROFILE_BINS],
            ..Default::default()
        };
    };
    let bbox_width = right - left + 1;
    let bbox_height = bottom - top + 1;
    let mut foreground = 0usize;
    let mut centroid_y = 0f64;
    let mut row_counts = vec![0usize; bbox_height as usize];
    let mut longest_runs = vec![0usize; bbox_height as usize];
    for y in top..=bottom {
        let mut current_run = 0usize;
        let mut longest_run = 0usize;
        for x in left..=right {
            if image.get_pixel(x, y)[3] > 16 {
                foreground += 1;
                centroid_y += f64::from(y);
                row_counts[(y - top) as usize] += 1;
                current_run += 1;
                longest_run = longest_run.max(current_run);
            } else {
                current_run = 0;
            }
        }
        longest_runs[(y - top) as usize] = longest_run;
    }
    let bbox_area = (bbox_width as usize).saturating_mul(bbox_height as usize);
    let bottom_band_start = bbox_height.saturating_mul(3) / 4;
    let bottom_area = row_counts
        .iter()
        .skip(bottom_band_start as usize)
        .sum::<usize>();
    let bottom_band_span_ratio = longest_runs
        .iter()
        .skip(bottom_band_start as usize)
        .copied()
        .max()
        .unwrap_or_default() as f32
        / bbox_width.max(1) as f32;
    let middle_start = bbox_height / 4;
    let middle_end = bbox_height.saturating_mul(2) / 3;
    let middle_span = longest_runs
        .iter()
        .skip(middle_start as usize)
        .take(middle_end.saturating_sub(middle_start).max(1) as usize)
        .copied()
        .max()
        .unwrap_or_default() as f32;
    let bottom_span = bottom_band_span_ratio * bbox_width.max(1) as f32;
    let components = alpha_component_areas(image, 16);
    let component_total = components.iter().sum::<usize>();
    let detached_component_ratio = components
        .iter()
        .max()
        .map(|largest| {
            component_total.saturating_sub(*largest) as f32 / component_total.max(1) as f32
        })
        .unwrap_or_default();
    let row_occupancy_profile = (0..PROFILE_BINS)
        .map(|bin| {
            let start = bin * row_counts.len() / PROFILE_BINS;
            let end = ((bin + 1) * row_counts.len() / PROFILE_BINS).max(start + 1);
            let occupied = row_counts[start..end.min(row_counts.len())]
                .iter()
                .sum::<usize>();
            occupied as f32
                / ((end.min(row_counts.len()) - start).max(1) * bbox_width as usize) as f32
        })
        .collect();
    ForegroundGeometry {
        width_ratio: bbox_width as f32 / image.width().max(1) as f32,
        height_ratio: bbox_height as f32 / image.height().max(1) as f32,
        area_ratio: foreground as f32 / bbox_area.max(1) as f32,
        centroid_y_ratio: if foreground == 0 {
            0.0
        } else {
            (centroid_y / foreground as f64) as f32 / image.height().max(1) as f32
        },
        detached_component_ratio,
        bottom_band_span_ratio,
        bottom_band_area_share: bottom_area as f32 / foreground.max(1) as f32,
        platform_overhang_ratio: if middle_span <= f32::EPSILON {
            0.0
        } else {
            bottom_span / middle_span
        },
        row_occupancy_profile,
    }
}

pub fn occupancy_profile_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let mean_delta = left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right).abs())
        .sum::<f32>()
        / left.len() as f32;
    (1.0 - mean_delta).clamp(0.0, 1.0)
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
        semantic_anchor_similarity: None,
        semantic_anchor_geometry_similarity: None,
        verdict,
        geometry: None,
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
    items: &[StaticPackItem],
    report: &ConsistencyReportV1,
    context: StaticPackContext<'_>,
) -> Result<StaticPackOutput, AssetProjectError> {
    let StaticPackContext {
        provider_id,
        item_metadata,
        collection,
        manual_item_ids,
        portrait_base,
        portrait_approval,
        portrait_report,
    } = context;
    let consistency_report_bytes = serde_json::to_vec_pretty(report)?;
    let consistency_report_sha256 = format!("{:x}", Sha256::digest(&consistency_report_bytes));
    let portrait_report_bytes = portrait_report.map(serde_json::to_vec_pretty).transpose()?;
    let portrait_report_sha256 = portrait_report_bytes
        .as_ref()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    let portrait_approval_bytes = portrait_approval
        .map(serde_json::to_vec_pretty)
        .transpose()?;
    let portrait_approval_sha256 = portrait_approval_bytes
        .as_ref()
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)));
    let geometry_profile = report
        .items
        .iter()
        .find_map(|item| item.geometry.as_ref())
        .map(|geometry| geometry.geometry_profile.as_str());
    let geometry_report_profile = report
        .items
        .iter()
        .find_map(|item| item.geometry.as_ref())
        .map(|geometry| geometry.profile.as_str());
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
            scale: 1,
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
            let mut entry = serde_json::json!({
                "id": item.id,
                "name": item.name,
                "frame": index,
                "texture": format!("assets/items/{}.png", item.id),
                "provenance": {
                    "providerId": if manual_item_ids.contains(&item.id) { "manual_replacement" } else { provider_id },
                    "styleRevision": style.revision,
                    "attempt": attempt,
                    "sha256": hash_file(&item.image_path).ok(),
                },
            });
            if let Some(metadata) = item_metadata.get(&item.id) {
                entry["metadata"] = serde_json::to_value(metadata).unwrap_or_default();
            }
            entry
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
    let center_anchor = collection
        .is_some_and(|lock| lock.grounding == CollectionGrounding::Center)
        || matches!(
            asset.kind,
            StaticAssetKind::IconSet | StaticAssetKind::PortraitSet
        );
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
            "type": if center_anchor { "center" } else { "feet" },
            "x": canvas as f32 / 2.0,
            "y": if center_anchor { canvas as f32 / 2.0 } else { canvas as f32 },
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
            let mut entry = serde_json::json!({
                "id": item.id,
                "name": item.name,
                "frame": index,
                "texture": format!("assets/items/{}.png", item.id),
                "provenance": {
                    "providerId": if manual_item_ids.contains(&item.id) { "manual_replacement" } else { provider_id },
                    "styleRevision": style.revision,
                    "attempt": attempt,
                    "sha256": hash_file(&item.image_path).ok(),
                },
            });
            if let Some(metadata) = item_metadata.get(&item.id) {
                entry["metadata"] = serde_json::to_value(metadata).unwrap_or_default();
            }
            entry
        })
        .collect::<Vec<_>>();
    let mut forgepack = serde_json::json!({
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
                "assetGeometryProfile": geometry_profile,
                "assetGeometryReportProfile": geometry_report_profile,
                "consistencyReportSha256": consistency_report_sha256,
                "collectionId": collection.map(|lock| lock.id.as_str()),
                "collectionRevision": collection.map(|lock| lock.revision.as_str()),
                "portraitBaseProfile": portrait_base.map(|lock| lock.profile.as_str()),
                "portraitBaseItemId": portrait_base.map(|lock| lock.item_id.as_str()),
                "portraitBaseSha256": portrait_base.map(|lock| lock.image_sha256.as_str()),
                "portraitNeutralReferencePolicy": portrait_base.map(|lock| lock.neutral_reference_policy.as_str()),
                "portraitNeutralReferenceRoles": portrait_base.map(|lock| &lock.neutral_reference_roles),
                "portraitNeutralReferenceSha256": portrait_base.map(|lock| &lock.neutral_reference_sha256),
                "portraitBaseApprovalProfile": portrait_approval.map(|approval| approval.profile.as_str()),
                "portraitBaseApprovalSourceJobId": portrait_approval.map(|approval| approval.source_job_id.as_str()),
                "portraitBaseApprovalSha256": portrait_approval_sha256,
                "portraitLocalProfile": portrait_report.map(|report| report.profile.as_str()),
                "portraitConsistencyReportSha256": portrait_report_sha256,
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
    if portrait_base.is_some() {
        forgepack["assets"]["portraitBaseLock"] = serde_json::json!(PORTRAIT_BASE_LOCK_FILE);
    }
    if portrait_report.is_some() {
        forgepack["assets"]["portraitConsistencyReport"] =
            serde_json::json!(PORTRAIT_CONSISTENCY_REPORT_FILE);
    }
    if portrait_approval.is_some() {
        forgepack["assets"]["portraitBaseApproval"] =
            serde_json::json!(PORTRAIT_BASE_APPROVAL_FILE);
    }

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
    fs::write(&consistency_report_path, consistency_report_bytes)?;
    if let Some(lock) = portrait_base {
        fs::write(
            pack_dir.join(PORTRAIT_BASE_LOCK_FILE),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schemaVersion": lock.schema_version,
                "profile": lock.profile,
                "assetId": lock.asset_id,
                "itemId": lock.item_id,
                "imageSha256": lock.image_sha256,
                "providerEditSourceSha256": lock.provider_edit_source_sha256,
                "providerId": lock.provider_id,
                "profileId": lock.profile_id,
                "model": lock.model,
                "styleRevision": lock.style_revision,
                "subjectId": lock.subject_id,
                "subjectRevision": lock.subject_revision,
                "collectionId": lock.collection_id,
                "collectionRevision": lock.collection_revision,
                "framingProfile": lock.framing_profile,
                "neutralReferencePolicy": lock.neutral_reference_policy,
                "neutralReferenceRoles": lock.neutral_reference_roles,
                "neutralReferenceSha256": lock.neutral_reference_sha256,
                "faceScope": lock.face_scope,
                "createdAt": lock.created_at,
            }))?,
        )?;
    }
    if let Some(bytes) = portrait_approval_bytes {
        fs::write(pack_dir.join(PORTRAIT_BASE_APPROVAL_FILE), bytes)?;
    }
    if let Some(bytes) = portrait_report_bytes {
        fs::write(pack_dir.join(PORTRAIT_CONSISTENCY_REPORT_FILE), bytes)?;
    }
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
    // HashMap iteration uses a per-process seed. Counts alone therefore make
    // equal-weight colors swap order across identical replays, which changes
    // the truncated palette and downstream report hashes. Break ties by RGB.
    values.sort_by(|(left_color, left_count), (right_color, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_color.cmp(right_color))
    });
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

    fn character_identity_fixture(with_features: bool) -> RgbaImage {
        let mut image = ImageBuffer::from_pixel(256, 256, Rgba([0, 0, 0, 0]));
        for y in 20..236 {
            for x in 64..192 {
                image.put_pixel(x, y, Rgba([70, 90, 45, 255]));
            }
        }
        for y in 52..116 {
            for x in 92..164 {
                image.put_pixel(x, y, Rgba([220, 165, 120, 255]));
            }
        }
        if with_features {
            for y in 76..83 {
                for x in 106..114 {
                    image.put_pixel(x, y, Rgba([35, 22, 18, 255]));
                }
                for x in 142..150 {
                    image.put_pixel(x, y, Rgba([35, 22, 18, 255]));
                }
            }
            for y in 99..104 {
                for x in 119..138 {
                    image.put_pixel(x, y, Rgba([45, 25, 20, 255]));
                }
            }
        }
        image
    }

    fn character_identity_fixture_with_bright_sclera() -> RgbaImage {
        let mut image = character_identity_fixture(false);
        for (left, right) in [(104, 120), (140, 156)] {
            for y in 73..87 {
                for x in left..right {
                    image.put_pixel(x, y, Rgba([252, 248, 235, 255]));
                }
            }
        }
        for y in 77..84 {
            for x in 110..116 {
                image.put_pixel(x, y, Rgba([35, 22, 18, 255]));
            }
            for x in 146..152 {
                image.put_pixel(x, y, Rgba([35, 22, 18, 255]));
            }
        }
        for y in 99..104 {
            for x in 119..138 {
                image.put_pixel(x, y, Rgba([45, 25, 20, 255]));
            }
        }
        image
    }

    fn character_identity_fixture_with_isolated_skin_highlight() -> RgbaImage {
        let mut image = character_identity_fixture(true);
        for y in 71..79 {
            for x in 123..131 {
                image.put_pixel(x, y, Rgba([70, 90, 45, 255]));
            }
        }
        for (x, y) in [(126, 74), (127, 74), (126, 75)] {
            image.put_pixel(x, y, Rgba([250, 185, 125, 255]));
        }
        image
    }

    #[test]
    fn character_identity_gate_blocks_blank_face_before_video_work() {
        let report = assess_character_identity_reference(
            &character_identity_fixture(false),
            "a hooded human ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report
            .reasons
            .iter()
            .any(|reason| reason == "character_face_detail_missing"));
        assert_eq!(report.metrics.enclosed_feature_count, 0);
    }

    #[test]
    fn character_identity_gate_accepts_visible_eyes_and_mouth() {
        let report = assess_character_identity_reference(
            &character_identity_fixture(true),
            "a hooded human ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:#?}");
        assert!(report.metrics.enclosed_feature_count >= 2);
    }

    #[test]
    fn character_identity_gate_does_not_select_bright_eye_white_as_face_skin() {
        let report = assess_character_identity_reference(
            &character_identity_fixture_with_bright_sclera(),
            "a hooded human ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:#?}");
        assert!(report.metrics.face_color_component_pixels > 1_000);
        assert!(report.metrics.enclosed_feature_count >= 2);
    }

    #[test]
    fn canonical_identity_fallback_ignores_isolated_skin_colored_highlight() {
        let image = character_identity_fixture_with_isolated_skin_highlight();
        let primary = assess_character_identity_reference(&image, "a hooded human ranger");
        assert_eq!(primary.verdict, ConsistencyVerdict::Blocked);
        let canonical =
            assess_character_canonical_identity_reference(&image, "a hooded human ranger");
        assert_eq!(canonical.verdict, ConsistencyVerdict::GameReady);
        assert_eq!(canonical.profile, CHARACTER_CANONICAL_IDENTITY_PROFILE);
        assert!(canonical.metrics.enclosed_feature_count >= 2);
    }

    #[test]
    fn canonical_identity_fallback_does_not_accept_a_blank_face() {
        let canonical = assess_character_canonical_identity_reference(
            &character_identity_fixture(false),
            "a hooded human ranger",
        );
        assert_eq!(canonical.verdict, ConsistencyVerdict::Blocked);
        assert!(canonical
            .reasons
            .contains(&"character_face_detail_missing".into()));
    }

    #[test]
    fn explicit_faceless_character_opts_out_of_visible_face_gate() {
        let report = assess_character_identity_reference(
            &character_identity_fixture(false),
            "a deliberately faceless masked forest spirit",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady);
        assert!(!report.metrics.visible_face_required);
    }

    fn semantic_animation_fixture(
        up_has_face: bool,
        detached_effect: bool,
    ) -> BTreeMap<String, Vec<RgbaImage>> {
        let mut animations = BTreeMap::new();
        animations.insert("idle".into(), vec![character_identity_fixture(true); 8]);
        animations.insert(
            "walk_down".into(),
            vec![character_identity_fixture(true); 8],
        );
        animations.insert(
            "walk_up".into(),
            vec![character_identity_fixture(up_has_face); 8],
        );
        let mut right = vec![character_identity_fixture(true); 8];
        if detached_effect {
            for y in 30..33 {
                for x in 210..213 {
                    right[3].put_pixel(x, y, Rgba([255, 210, 45, 255]));
                }
            }
        }
        animations.insert("walk_right".into(), right);
        animations
    }

    #[test]
    fn character_semantic_gate_requires_back_facing_walk_up() {
        let report = assess_character_animation_semantics(
            &semantic_animation_fixture(true, false),
            "a hooded human ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        let up = report
            .animations
            .iter()
            .find(|animation| animation.name == "walk_up")
            .unwrap();
        assert_eq!(up.visible_face_frames, 8);
        assert!(up
            .reasons
            .iter()
            .any(|reason| reason == "walk_up_face_visible"));
    }

    #[test]
    fn character_semantic_gate_accepts_front_and_back_face_contract() {
        let report = assess_character_animation_semantics(
            &semantic_animation_fixture(false, false),
            "a hooded human ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:#?}");
    }

    #[test]
    fn modern_camera_blocks_direction_drift_inside_selected_interval() {
        let mut animations = semantic_animation_fixture(false, false);
        let walk_up = animations.get_mut("walk_up").unwrap();
        walk_up[2] = character_identity_fixture(true);
        walk_up[3] = character_identity_fixture(true);

        let report = assess_character_animation_semantics_for_camera(
            &animations,
            "a hooded human ranger",
            CharacterCameraProfileV1::TopdownThreeQuarter,
        );
        let up = report
            .animations
            .iter()
            .find(|animation| animation.name == "walk_up")
            .unwrap();

        assert_eq!(up.verdict, ConsistencyVerdict::Blocked, "{up:#?}");
        assert!(up.direction_match_ratio < 0.875);
        assert!(up.maximum_consecutive_direction_mismatch_frames >= 2);
        assert!(up
            .reasons
            .contains(&"selected_interval_direction_drift".into()));
    }

    #[test]
    fn character_semantic_gate_blocks_detached_emissive_effects() {
        let report = assess_character_animation_semantics(
            &semantic_animation_fixture(false, true),
            "a hooded human ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        let right = report
            .animations
            .iter()
            .find(|animation| animation.name == "walk_right")
            .unwrap();
        assert_eq!(right.detached_emissive_effect_frames, 1);
        assert!(right
            .reasons
            .iter()
            .any(|reason| reason == "unexpected_detached_emissive_effect"));
    }

    #[test]
    fn character_semantic_gate_strips_attached_translucent_halo() {
        let mut animations = semantic_animation_fixture(false, false);
        {
            let frame = &mut animations.get_mut("walk_right").unwrap()[3];
            for x in 202..219 {
                for y in 52..69 {
                    frame.put_pixel(x, y, Rgba([180, 125, 35, 128]));
                }
            }
            for x in 192..211 {
                for y in 60..64 {
                    frame.put_pixel(x, y, Rgba([70, 45, 25, 255]));
                }
            }
            for x in 208..214 {
                for y in 58..64 {
                    frame.put_pixel(x, y, Rgba([255, 225, 110, 255]));
                }
            }
        }
        let blocked = assess_character_animation_semantics(&animations, "a hooded human ranger");
        assert_eq!(blocked.verdict, ConsistencyVerdict::Blocked);
        assert!(blocked
            .animations
            .iter()
            .find(|report| report.name == "walk_right")
            .unwrap()
            .reasons
            .iter()
            .any(|reason| reason == "unexpected_attached_emissive_halo"));

        let removed = strip_unexpected_attached_emissive_halo(
            &mut animations.get_mut("walk_right").unwrap()[3],
        );
        assert!(removed > 0);
        let repaired = assess_character_animation_semantics(&animations, "a hooded human ranger");
        assert_eq!(
            repaired.verdict,
            ConsistencyVerdict::GameReady,
            "{repaired:#?}"
        );
    }

    fn back_facing_scarf_fixture() -> RgbaImage {
        let mut image = character_identity_fixture(false);
        for y in 52..116 {
            for x in 92..164 {
                image.put_pixel(x, y, Rgba([70, 90, 45, 255]));
            }
        }
        for y in 92..112 {
            for x in 80..176 {
                image.put_pixel(x, y, Rgba([220, 145, 70, 255]));
            }
        }
        for (x, y) in [(96, 99), (116, 103), (140, 98), (158, 104)] {
            for offset_y in 0..2 {
                for offset_x in 0..2 {
                    image.put_pixel(x + offset_x, y + offset_y, Rgba([60, 35, 20, 255]));
                }
            }
        }
        image
    }

    #[test]
    fn character_semantic_gate_does_not_treat_a_rear_scarf_as_a_face() {
        let image = back_facing_scarf_fixture();
        let identity = assess_character_identity_reference(&image, "a hooded human ranger");
        assert_eq!(identity.verdict, ConsistencyVerdict::GameReady);
        assert!(identity.metrics.enclosed_feature_pixel_ratio < 0.02);
        let report = assess_character_animation_semantics(
            &BTreeMap::from([("walk_up".into(), vec![image])]),
            "a hooded human ranger",
        );
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:#?}");
        assert_eq!(report.animations[0].visible_face_frames, 0);
    }

    #[test]
    fn silhouette_temporal_gate_accepts_a_stable_loop_with_bounded_translation() {
        let mut frames = Vec::new();
        for offset in [0i32, 2, 2, 0, -2, -2, 0, 0] {
            let mut frame = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
            for y in 24..112 {
                for x in 44..84 {
                    frame.put_pixel((x + offset) as u32, y, Rgba([60, 90, 45, 255]));
                }
            }
            frames.push(frame);
        }
        let report =
            assess_character_silhouette_temporal(&BTreeMap::from([("idle".into(), frames)]));
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:#?}");
    }

    #[test]
    fn silhouette_temporal_gate_blocks_body_center_steps_above_two_pixels() {
        let mut frames = Vec::new();
        for offset in [0i32, 3, 0, 0, 0, 0, 0, 0] {
            let mut frame = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
            for y in 24..112 {
                for x in 44..84 {
                    frame.put_pixel((x + offset) as u32, y, Rgba([60, 90, 45, 255]));
                }
            }
            frames.push(frame);
        }
        let report =
            assess_character_silhouette_temporal(&BTreeMap::from([("idle".into(), frames)]));
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.animations[0]
            .reasons
            .contains(&"temporal_body_center_step".into()));
    }

    #[test]
    fn silhouette_temporal_gate_blocks_a_single_frame_upper_body_notch() {
        let mut frames = vec![RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0])); 8];
        for frame in &mut frames {
            for y in 24..112 {
                for x in 44..84 {
                    frame.put_pixel(x, y, Rgba([60, 90, 45, 255]));
                }
            }
        }
        for y in 36..72 {
            for x in 72..84 {
                frames[3].put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        let report =
            assess_character_silhouette_temporal(&BTreeMap::from([("walk_right".into(), frames)]));
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.animations[0]
            .reasons
            .iter()
            .any(|reason| reason == "upper_body_mask_flicker"
                || reason == "upper_body_contour_drift"));
    }

    #[test]
    fn silhouette_temporal_gate_allows_motion_outside_a_stable_rigid_core() {
        let mut frames = Vec::new();
        for arm_offset in [0i32, 4, 8, 4, 0, -4, -8, -4] {
            let mut frame = RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0]));
            for y in 20..112 {
                for x in 44..84 {
                    frame.put_pixel(x, y, Rgba([60, 90, 45, 255]));
                }
            }
            for y in 76..92 {
                for x in 0..8 {
                    let left = (36 + arm_offset + x) as u32;
                    let right = (84 - arm_offset + x) as u32;
                    frame.put_pixel(left, y, Rgba([70, 80, 50, 255]));
                    frame.put_pixel(right, y, Rgba([70, 80, 50, 255]));
                }
            }
            frames.push(frame);
        }
        let report = assess_character_silhouette_temporal_source(&BTreeMap::from([(
            "walk_down".into(),
            frames,
        )]));
        assert_eq!(report.verdict, ConsistencyVerdict::GameReady, "{report:#?}");
        assert_eq!(report.evaluation_stage, "source");
    }

    #[test]
    fn silhouette_temporal_gate_blocks_colored_edge_flicker() {
        let mut frames = vec![RgbaImage::from_pixel(128, 128, Rgba([0, 0, 0, 0])); 8];
        for (index, frame) in frames.iter_mut().enumerate() {
            for y in 20..112 {
                for x in 44..84 {
                    let edge = x == 44 || x == 83 || y == 20 || y == 111;
                    let color = if edge && index % 2 == 1 {
                        [230, 30, 210, 255]
                    } else {
                        [60, 90, 45, 255]
                    };
                    frame.put_pixel(x, y, Rgba(color));
                }
            }
        }
        let report =
            assess_character_silhouette_temporal_source(&BTreeMap::from([("idle".into(), frames)]));
        assert_eq!(report.verdict, ConsistencyVerdict::Blocked);
        assert!(report.animations[0]
            .reasons
            .contains(&"temporal_edge_color_flicker".into()));
    }

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
            foreground_width_ratio: NORMALIZED_FOREGROUND_EXTENT,
            foreground_height_ratio: NORMALIZED_FOREGROUND_EXTENT,
            foreground_area_ratio: 0.5,
            centroid_y_ratio: 0.5,
            detached_component_ratio: 0.0,
            bottom_band_span_ratio: 0.5,
            bottom_band_area_share: 0.2,
            platform_overhang_ratio: 1.0,
            row_occupancy_profile: vec![0.5; 16],
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
    fn palette_breaks_equal_weight_ties_by_rgb_for_replay_determinism() {
        let mut image = RgbaImage::new(4, 1);
        image.put_pixel(0, 0, Rgba([224, 0, 0, 255]));
        image.put_pixel(1, 0, Rgba([0, 224, 0, 255]));
        image.put_pixel(2, 0, Rgba([0, 0, 224, 255]));
        image.put_pixel(3, 0, Rgba([224, 224, 0, 255]));

        let palette = palette(&image);
        assert_eq!(
            palette
                .iter()
                .map(|color| color.color.as_str())
                .collect::<Vec<_>>(),
            vec!["#0000E0", "#00E000", "#E00000", "#E0E000"]
        );
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

    #[cfg(feature = "pixel-delivery-v2")]
    #[test]
    fn pixel_delivery_normalization_writes_binary_palette_report() {
        let temp = tempfile::tempdir().unwrap();
        let mut source = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for y in 16..48 {
            for x in 16..48 {
                let color = if x < 32 { [9, 12, 20] } else { [230, 220, 210] };
                source.put_pixel(x, y, Rgba([color[0], color[1], color[2], 255]));
            }
        }
        let output_path = temp.path().join("delivery.png");
        let output =
            normalize_matted_static_image_character_delivery(&source, &output_path, 64, true)
                .unwrap();
        assert!(output
            .pixels()
            .all(|pixel| pixel[3] == 0 || pixel[3] == 255));
        let report_path = output_path.with_extension("pixel-delivery.json");
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(&report_path).unwrap()).unwrap();
        assert_eq!(report["profile"], "pixel-delivery@2.0.0");
        assert_eq!(report["alphaBinary"], true);
        assert_eq!(report["paletteLocked"], true);
        assert!(report["colorCount"].as_u64().unwrap() <= 24);
    }
}
