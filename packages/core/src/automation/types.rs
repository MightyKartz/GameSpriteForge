use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::asset_project::{CharacterEquipmentSpecV1, StaticAssetSetSpecV1};
use crate::character_camera::CharacterCameraProfileV1;
use crate::character_direction_motion::{
    CharacterMotionProfileV1, DirectionMotionGenerationStageV1,
};
use crate::character_grid::{CapeHemContractV1, GridPoseGuidanceV1};
use crate::collection::StaticItemMetadataV1;
use crate::export::{GodotRenderingContractV1, PreviewGifParameters, SpriteSheetParameters};
use crate::frames::NormalizeOptions;
use crate::geometry::PortraitFramingProfileV1;
use crate::job::RepairContext;
use crate::matting::ChromaParameters;
use crate::portrait::{
    is_legacy_portrait_phase, is_legacy_portrait_reference_policy, PortraitGenerationPhaseV1,
    PortraitNeutralReferencePolicyV1,
};
use crate::project::ProviderAssetRef;
use crate::world::{BuildingKitSpecV1, TerrainSetSpecV1};

pub const AUTOMATION_SCHEMA_VERSION: &str = "1";
pub const EXTERNAL_KEYFRAME_WORKFLOW_ID: &str = "topdown-external-keyframes";
pub const EXTERNAL_KEYFRAME_WORKFLOW_VERSION: &str = "11.0.0";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationProfile {
    pub schema_version: String,
    pub id: String,
    pub version: String,
    pub matting: ProfileMatting,
    pub normalize: NormalizeOptions,
    pub sheet: SpriteSheetParameters,
    pub preview: PreviewGifParameters,
    pub quality: QualityPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileMatting {
    pub mode: String,
    #[serde(flatten)]
    pub chroma: ChromaParameters,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct QualityPolicy {
    #[serde(default = "default_true")]
    pub require_game_ready: bool,
}

impl Default for QualityPolicy {
    fn default() -> Self {
        Self {
            require_game_ready: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct PrepareAssetRequest {
    #[serde(default = "schema_version")]
    pub schema_version: String,
    pub input: AssetInput,
    pub metadata: AssetMetadata,
    #[serde(default)]
    pub matting: MattingRecipe,
    #[serde(default = "profile_normalize")]
    pub normalize: NormalizeOptions,
    #[serde(default = "profile_sheet")]
    pub sheet: SpriteSheetParameters,
    #[serde(default)]
    pub quality: QualityPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct PrepareCharacterPackRequest {
    #[serde(default = "character_schema_version")]
    pub schema_version: String,
    pub metadata: CharacterPackMetadata,
    #[serde(default)]
    pub workflow: CharacterWorkflowSelection,
    pub animations: Vec<CharacterAnimationRecipe>,
    /// Optional semantic context for local frame ingestion. The external
    /// keyframe workflow requires this but never sends it to a Provider.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_profile: Option<CharacterCameraProfileV1>,
    #[serde(default)]
    pub equipment: CharacterEquipmentSpecV1,
    #[serde(default = "profile_normalize")]
    pub normalize: NormalizeOptions,
    #[serde(default = "profile_sheet")]
    pub sheet: SpriteSheetParameters,
    #[serde(default)]
    pub quality: QualityPolicy,
    /// Optional, explicitly requested local-only processing upgrade for source
    /// video clips. This is deliberately independent from the media-generation
    /// workflow so retained V6/V7 videos can be reprocessed without pretending
    /// to be a completed V8 generation Job.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_cycle_sampling_profile: Option<String>,
    /// Produce complete diagnostic frames/previews when the strict sampling
    /// budget fails. Preview-only Jobs never export a Pack and therefore
    /// cannot be mistaken for game-ready delivery.
    #[serde(default)]
    pub source_cycle_sampling_preview: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct GenerateCharacterPackRequest {
    #[serde(default = "generation_schema_version")]
    pub schema_version: String,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    pub character: GeneratedCharacterSpec,
    /// Explicit for `topdown-video@2.0.0`; defaults only to keep historical
    /// serialized Jobs readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera_profile: Option<CharacterCameraProfileV1>,
    #[serde(default)]
    pub equipment: CharacterEquipmentSpecV1,
    /// Records that the caller explicitly declared the equipment state rather
    /// than receiving the compatibility default while reading an older Job.
    #[serde(default)]
    pub equipment_explicit: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_lock_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_lock_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_from_job_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retry_animations: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub retry_stages: BTreeMap<String, CharacterRetryStage>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub retry_frames: BTreeMap<String, Vec<u8>>,
    /// Optional V9 DirectionGrid garment-topology contract. The
    /// `front_authoritative_no_skirt_hem` route is a one-request ImageLocks
    /// child whose approved front_idle is the sole visual authority.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction_grid_cape_contract: Option<CapeHemContractV1>,
    /// Generate and quality-check only the named directions without exporting a
    /// partial Character Pack. This is intended for bounded real-provider
    /// acceptance before committing to a complete 32-frame generation.
    #[serde(default)]
    pub validation_only: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_animations: Vec<String>,
    /// V8-only prompt/diagnostic guidance. The default deliberately makes no
    /// assumptions about anatomy or locomotion.
    #[serde(default)]
    pub motion_profile: CharacterMotionProfileV1,
    /// V8 can stop after the immutable eight-image lock so a user or agent can
    /// review it before authorizing any video requests.
    #[serde(default)]
    pub direction_motion_stage: DirectionMotionGenerationStageV1,
    pub metadata: CharacterPackMetadata,
    #[serde(default = "default_generated_workflow")]
    pub workflow: CharacterWorkflowSelection,
    #[serde(default)]
    pub generation: GenerationPolicy,
    #[serde(default = "profile_normalize")]
    pub normalize: NormalizeOptions,
    #[serde(default = "profile_sheet")]
    pub sheet: SpriteSheetParameters,
    #[serde(default)]
    pub quality: QualityPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalDirectionGridFilesV1 {
    pub front: PathBuf,
    pub rear: PathBuf,
    pub right: PathBuf,
    pub left: PathBuf,
}

impl ExternalDirectionGridFilesV1 {
    pub fn ordered(&self) -> [(&'static str, &PathBuf); 4] {
        [
            ("front", &self.front),
            ("rear", &self.rear),
            ("right", &self.right),
            ("left", &self.left),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportDirectionGridRequestV1 {
    #[serde(default = "schema_version")]
    pub schema_version: String,
    /// Immutable approved V9 DirectionGrid Job used only for identity, style,
    /// camera, equipment and lineage authority. The source is never mutated.
    pub source_job_dir: PathBuf,
    /// External square 2x2 sheet ordered front, rear, right, left. Exactly one
    /// of `sheetPath` and `directionPaths` must be present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sheet_path: Option<PathBuf>,
    /// Four independently produced, equally-sized square direction files.
    /// Their field names, rather than filesystem iteration, define order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction_paths: Option<ExternalDirectionGridFilesV1>,
    /// Optional character-specific garment topology. It is accepted only for
    /// the named four-file route so every direction remains explicitly bound.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cape_hem_contract: Option<CapeHemContractV1>,
    /// Stable provenance label such as `codex_builtin_image_gen`.
    pub generator: String,
    /// Human audit context; this is evidence, not an approval decision.
    pub note: String,
}

impl ImportDirectionGridRequestV1 {
    pub fn ordered_input_paths(&self) -> Vec<(&'static str, &PathBuf)> {
        if let Some(sheet) = &self.sheet_path {
            vec![("sheet", sheet)]
        } else {
            self.direction_paths
                .as_ref()
                .map(|paths| paths.ordered().into_iter().collect())
                .unwrap_or_default()
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterRetryStage {
    #[default]
    Auto,
    Still,
    Video,
    Loop,
    Matting,
    Frame,
    Consistency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateStyleLockRequest {
    #[serde(default = "style_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub spec_path: PathBuf,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateSubjectLockRequest {
    #[serde(default = "subject_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub spec_path: PathBuf,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_import_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub import_approval_note: Option<String>,
}

impl CreateSubjectLockRequest {
    pub fn is_local_import(&self) -> bool {
        self.canonical_import_path.is_some()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateCollectionLockRequest {
    #[serde(default = "collection_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub spec_path: PathBuf,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateStaticAssetSetRequest {
    #[serde(default = "static_generation_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub style_lock_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collection_lock_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_lock_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_spec_path: Option<PathBuf>,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    pub asset: StaticAssetSetSpecV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framing_profile: Option<PortraitFramingProfileV1>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub item_metadata: BTreeMap<String, StaticItemMetadataV1>,
    #[serde(default = "default_generation_attempts")]
    pub max_attempts_per_item: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_from_job_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retry_item_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub replacement_item_paths: BTreeMap<String, PathBuf>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub consistency_recheck_only: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub resume_incomplete_static: bool,
    #[serde(default, skip_serializing_if = "is_legacy_portrait_phase")]
    pub portrait_phase: PortraitGenerationPhaseV1,
    #[serde(default, skip_serializing_if = "is_legacy_portrait_reference_policy")]
    pub neutral_reference_policy: PortraitNeutralReferencePolicyV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portrait_base_parent_job_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateEnvironmentLockRequest {
    #[serde(default = "world_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub spec_path: PathBuf,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateTerrainSetRequest {
    #[serde(default = "world_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub environment_lock_path: PathBuf,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    pub asset: TerrainSetSpecV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateBuildingKitRequest {
    #[serde(default = "world_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub environment_lock_path: PathBuf,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    pub asset: BuildingKitSpecV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompileMapRequest {
    #[serde(default = "world_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub spec_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct GeneratedCharacterSpec {
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_image_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct GenerationPolicy {
    #[serde(default = "default_generation_attempts")]
    pub max_attempts_per_animation: u8,
    #[serde(default = "default_target_frame_count")]
    pub target_frame_count: u32,
    #[serde(default = "default_video_duration")]
    pub video_duration_seconds: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_model: Option<String>,
    #[serde(default)]
    pub pose_guidance: GridPoseGuidanceV1,
}

impl Default for GenerationPolicy {
    fn default() -> Self {
        Self {
            max_attempts_per_animation: default_generation_attempts(),
            target_frame_count: default_target_frame_count(),
            video_duration_seconds: default_video_duration(),
            image_model: None,
            video_model: None,
            pose_guidance: GridPoseGuidanceV1::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWorkflowCatalog {
    pub schema_version: String,
    pub workflows: Vec<CharacterWorkflowPreset>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWorkflowPreset {
    pub id: String,
    pub version: String,
    pub label: String,
    pub description: String,
    #[serde(default = "default_stable_status")]
    pub status: String,
    #[serde(default)]
    pub target_frame_count: u32,
    #[serde(default)]
    pub estimated_provider_requests: u32,
    #[serde(default)]
    pub maximum_provider_requests: u32,
    pub default_animation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback_profile: Option<String>,
    pub required_animations: Vec<CharacterWorkflowAnimation>,
    pub optional_animations: Vec<CharacterWorkflowAnimation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWorkflowAnimation {
    pub name: String,
    pub fps: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_duration_ms: Option<u64>,
    #[serde(rename = "loop")]
    pub loop_animation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CharacterWorkflowSelection {
    pub id: String,
    pub version: String,
}

impl Default for CharacterWorkflowSelection {
    fn default() -> Self {
        Self {
            id: "custom".into(),
            version: "1.0.0".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CharacterPackMetadata {
    pub name: String,
    pub default_animation: String,
    #[serde(default = "default_creator")]
    pub creator: String,
    #[serde(default = "default_license")]
    pub license: String,
    #[serde(default)]
    pub rendering: GodotRenderingContractV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CharacterAnimationRecipe {
    pub name: String,
    pub input: AssetInput,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default = "default_true", rename = "loop")]
    pub loop_animation: bool,
    #[serde(default)]
    pub matting: MattingRecipe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum AssetInput {
    PngSequence {
        paths: Vec<PathBuf>,
    },
    SpriteSheet {
        path: PathBuf,
        split: SpriteSheetSplit,
    },
    Gsfpack {
        path: PathBuf,
    },
    VideoClip {
        path: PathBuf,
        #[serde(default, rename = "startTimeMs")]
        start_time_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none", rename = "endTimeMs")]
        end_time_ms: Option<u64>,
        #[serde(default = "default_target_frame_count", rename = "targetFrameCount")]
        target_frame_count: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum SpriteSheetSplit {
    FixedGrid(FixedGridSplit),
    TransparentGutters {
        #[serde(default)]
        alpha_threshold: u8,
        #[serde(default = "default_gap")]
        min_gap_px: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct FixedGridSplit {
    pub frame_width: u32,
    pub frame_height: u32,
    pub columns: u32,
    pub rows: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum MattingRecipe {
    #[default]
    PreserveAlpha,
    AutoCorners {
        #[serde(flatten)]
        parameters: ChromaParameters,
    },
    ManualColor {
        color: String,
        #[serde(flatten)]
        parameters: ChromaParameters,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct AssetMetadata {
    pub name: String,
    #[serde(default = "default_animation")]
    pub animation: String,
    #[serde(default = "default_fps")]
    pub fps: f32,
    #[serde(default = "default_true", rename = "loop")]
    pub loop_animation: bool,
    #[serde(default = "default_creator")]
    pub creator: String,
    #[serde(default = "default_license")]
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct GodotInstallRequest {
    #[serde(default = "schema_version")]
    pub schema_version: String,
    pub pack_path: PathBuf,
    pub project_path: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_project_path: Option<PathBuf>,
    #[serde(default = "default_godot_target")]
    pub target: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_key: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_refs: Vec<ProviderAssetRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct BuildProjectRequestV1 {
    #[serde(default = "schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub manifest_path: PathBuf,
    /// Exact pure build plan reviewed by the caller. Execution recomputes the
    /// plan and refuses to run when this digest no longer matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_plan_sha256: Option<String>,
    /// Static provider descriptor captured without resolving credentials.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_model: Option<String>,
    /// Immutable source closure (project, manifest, specs/references and
    /// Style/Subject Locks) revalidated before every provider child.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_source_sha256: Option<String>,
    /// Explicit source parent for crash recovery. A new job may reuse only a
    /// succeeded child whose complete plan digest is identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_from_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_state_path: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume_state_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum AutomationOperation {
    PrepareAsset(PrepareAssetRequest),
    PrepareCharacterPack(PrepareCharacterPackRequest),
    GenerateCharacterPack(GenerateCharacterPackRequest),
    ImportDirectionGrid(ImportDirectionGridRequestV1),
    CreateStyleLock(CreateStyleLockRequest),
    CreateSubjectLock(CreateSubjectLockRequest),
    CreateCollectionLock(CreateCollectionLockRequest),
    GenerateStaticAssetSet(GenerateStaticAssetSetRequest),
    CreateEnvironmentLock(CreateEnvironmentLockRequest),
    GenerateTerrainSet(GenerateTerrainSetRequest),
    GenerateBuildingKit(GenerateBuildingKitRequest),
    CompileMap(CompileMapRequest),
    InstallGodot(GodotInstallRequest),
    BuildProject(BuildProjectRequestV1),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanState {
    Pending,
    Claimed,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationPlan {
    pub schema_version: String,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub state: PlanState,
    pub input_fingerprint: String,
    pub recipe_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<RepairContext>,
    /// The exact estimate shown when this single-use plan was prepared.
    /// Persisting it prevents execution from recomputing a different budget
    /// after the user has reviewed the plan. Older pending plans deserialize
    /// with an empty estimate and are handled conservatively by the CLI.
    #[serde(default)]
    pub estimate: PlanEstimateV1,
    pub operation: AutomationOperation,
    pub effects: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparedPlan {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub input_fingerprint: String,
    pub recipe_hash: String,
    pub effects: Vec<String>,
    pub estimate: PlanEstimateV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<RepairContext>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEstimateV1 {
    pub provider_request_estimate: u32,
    pub maximum_provider_requests: u32,
    #[serde(default)]
    pub cache_hit_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

pub fn automation_profile() -> AutomationProfile {
    serde_json::from_str(include_str!("../../../../profiles/godot-pixel-art.v1.json"))
        .expect("bundled automation profile must be valid")
}

pub fn character_workflow_catalog() -> CharacterWorkflowCatalog {
    serde_json::from_str(include_str!(
        "../../../../profiles/character-workflows.v1.json"
    ))
    .expect("bundled character workflow catalog must be valid")
}

fn schema_version() -> String {
    AUTOMATION_SCHEMA_VERSION.to_string()
}

fn character_schema_version() -> String {
    "2".to_string()
}

fn generation_schema_version() -> String {
    "3".to_string()
}

fn style_schema_version() -> String {
    "1".to_string()
}

fn subject_schema_version() -> String {
    "1".to_string()
}

fn collection_schema_version() -> String {
    "1".into()
}

fn static_generation_schema_version() -> String {
    "4".to_string()
}

fn world_schema_version() -> String {
    "1".to_string()
}

fn default_profile_id() -> String {
    "default".to_string()
}

fn default_generated_workflow() -> CharacterWorkflowSelection {
    CharacterWorkflowSelection {
        id: "topdown".into(),
        version: "1.0.0".into(),
    }
}

fn default_animation() -> String {
    "idle".to_string()
}

fn default_fps() -> f32 {
    12.0
}

fn default_true() -> bool {
    true
}

fn default_stable_status() -> String {
    "stable".into()
}

fn default_gap() -> u32 {
    1
}

fn default_target_frame_count() -> u32 {
    8
}

fn default_generation_attempts() -> u8 {
    2
}

fn default_video_duration() -> u32 {
    4
}

fn default_creator() -> String {
    "Game Sprite Forge".to_string()
}

fn default_license() -> String {
    "private".to_string()
}

fn default_godot_target() -> PathBuf {
    PathBuf::from("addons/forge_assets")
}

fn profile_normalize() -> NormalizeOptions {
    automation_profile().normalize
}

fn profile_sheet() -> SpriteSheetParameters {
    automation_profile().sheet
}

#[cfg(test)]
mod tests {
    use super::character_workflow_catalog;

    #[test]
    fn video_cycle_profile_preserves_public_playback_contract() {
        let catalog = character_workflow_catalog();
        let workflow = catalog
            .workflows
            .iter()
            .find(|workflow| workflow.id == "topdown-video-cycle" && workflow.version == "6.0.0")
            .expect("bundled V6 workflow");

        assert_eq!(
            workflow.playback_profile.as_deref(),
            Some("playback-cadence@1.0.0")
        );
        let durations = workflow
            .required_animations
            .iter()
            .map(|animation| (animation.name.as_str(), animation.cycle_duration_ms))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(durations.get("idle"), Some(&Some(1_600)));
        assert_eq!(durations.get("walk_up"), Some(&Some(800)));
        assert_eq!(durations.get("walk_right"), Some(&Some(800)));
        assert_eq!(durations.get("walk_down"), Some(&Some(800)));
    }
}
