mod plan;
mod repair;
mod runner;
mod types;

pub use plan::{PlanStore, PlanStoreError, PLAN_TTL_MINUTES};
pub use repair::{
    analyze_repair, character_quality_snapshot, prepare_repair_plan, single_quality_snapshot,
    write_repair_comparison, RepairAnalysis, RepairComparison, RepairError, MAX_REPAIR_ATTEMPTS,
};
pub use runner::{run_operation, run_operation_with_provider, stage_plan_job, AutomationRunError};
pub use types::{
    automation_profile, character_workflow_catalog, AssetInput, AssetMetadata, AutomationOperation,
    AutomationPlan, AutomationProfile, CharacterAnimationRecipe, CharacterPackMetadata,
    CharacterRetryStage, CharacterWorkflowAnimation, CharacterWorkflowCatalog,
    CharacterWorkflowPreset, CharacterWorkflowSelection, CompileMapRequest,
    CreateEnvironmentLockRequest, CreateStyleLockRequest, CreateSubjectLockRequest, FixedGridSplit,
    GenerateBuildingKitRequest, GenerateCharacterPackRequest, GenerateStaticAssetSetRequest,
    GenerateTerrainSetRequest, GeneratedCharacterSpec, GenerationPolicy, GodotInstallRequest,
    MattingRecipe, PlanEstimateV1, PrepareAssetRequest, PrepareCharacterPackRequest, PreparedPlan,
    QualityPolicy, SpriteSheetSplit,
};
