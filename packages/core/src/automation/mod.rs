mod plan;
mod repair;
mod runner;
mod types;

/// Exact xAI image candidate admitted only by the bounded DirectionGrid
/// comparison route. It is deliberately not the default image model.
pub const XAI_IMAGE_2_CANDIDATE_MODEL: &str = "grok-imagine-image-2.0";
/// Conservative single-edit authorization ceiling for the Image 2.0
/// DirectionGrid comparison. The live catalog price may be lower, but real
/// execution must never depend on a mutable price observation.
pub const XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS: u64 = 1_400_000_000;
pub const FRONT_AUTHORITATIVE_DIRECTION_GRID_MAX_COST_TICKS: u64 = 1_400_000_000;

pub use plan::{fingerprint_operation_inputs, PlanStore, PlanStoreError, PLAN_TTL_MINUTES};
pub use repair::{
    analyze_repair, character_quality_snapshot, prepare_repair_plan, single_quality_snapshot,
    write_repair_comparison, RepairAnalysis, RepairComparison, RepairError, MAX_REPAIR_ATTEMPTS,
};
pub use runner::{
    hash_directory, register_static_catalog_output, run_operation, run_operation_with_provider,
    stage_plan_job, AutomationRunError,
};
pub use types::{
    automation_profile, character_workflow_catalog, AssetInput, AssetMetadata, AutomationOperation,
    AutomationPlan, AutomationProfile, BuildProjectRequestV1, CharacterAnimationRecipe,
    CharacterPackMetadata, CharacterRetryStage, CharacterWorkflowAnimation,
    CharacterWorkflowCatalog, CharacterWorkflowPreset, CharacterWorkflowSelection,
    CompileMapRequest, CreateCollectionLockRequest, CreateEnvironmentLockRequest,
    CreateStyleLockRequest, CreateSubjectLockRequest, ExternalDirectionGridFilesV1, FixedGridSplit,
    GenerateBuildingKitRequest, GenerateCharacterPackRequest, GenerateStaticAssetSetRequest,
    GenerateTerrainSetRequest, GeneratedCharacterSpec, GenerationPolicy, GodotInstallRequest,
    ImportDirectionGridRequestV1, MattingRecipe, PlanEstimateV1, PrepareAssetRequest,
    PrepareCharacterPackRequest, PreparedPlan, QualityPolicy, SpriteSheetSplit,
};
