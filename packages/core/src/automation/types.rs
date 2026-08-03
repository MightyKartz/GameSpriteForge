use std::collections::BTreeMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::asset_project::StaticAssetSetSpecV1;
use crate::export::{PreviewGifParameters, SpriteSheetParameters};
use crate::frames::NormalizeOptions;
use crate::job::RepairContext;
use crate::matting::ChromaParameters;
use crate::project::ProviderAssetRef;
use crate::world::{BuildingKitSpecV1, TerrainSetSpecV1};

pub const AUTOMATION_SCHEMA_VERSION: &str = "1";

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateStaticAssetSetRequest {
    #[serde(default = "static_generation_schema_version")]
    pub schema_version: String,
    pub project_path: PathBuf,
    pub style_lock_path: PathBuf,
    pub provider_id: String,
    #[serde(default = "default_profile_id")]
    pub profile_id: String,
    pub asset: StaticAssetSetSpecV1,
    #[serde(default = "default_generation_attempts")]
    pub max_attempts_per_item: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reuse_from_job_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub retry_item_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub consistency_recheck_only: bool,
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
}

impl Default for GenerationPolicy {
    fn default() -> Self {
        Self {
            max_attempts_per_animation: default_generation_attempts(),
            target_frame_count: default_target_frame_count(),
            video_duration_seconds: default_video_duration(),
            image_model: None,
            video_model: None,
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
    pub required_animations: Vec<CharacterWorkflowAnimation>,
    pub optional_animations: Vec<CharacterWorkflowAnimation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterWorkflowAnimation {
    pub name: String,
    pub fps: f32,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum AutomationOperation {
    PrepareAsset(PrepareAssetRequest),
    PrepareCharacterPack(PrepareCharacterPackRequest),
    GenerateCharacterPack(GenerateCharacterPackRequest),
    CreateStyleLock(CreateStyleLockRequest),
    CreateSubjectLock(CreateSubjectLockRequest),
    GenerateStaticAssetSet(GenerateStaticAssetSetRequest),
    CreateEnvironmentLock(CreateEnvironmentLockRequest),
    GenerateTerrainSet(GenerateTerrainSetRequest),
    GenerateBuildingKit(GenerateBuildingKitRequest),
    CompileMap(CompileMapRequest),
    InstallGodot(GodotInstallRequest),
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
