//! GameArtManifest (stage 2): manifest types, full validation, path-safe
//! spec resolution, canonical normalization and hashing.

mod build;
mod diff;
mod manifest;
mod plan;
mod types;

pub use build::{
    reconcile_interrupted_builds, run_build_project, BuildAssetStatusV1, BuildResultStatusV1,
    BuildStateAssetV1, ProjectBuildAssetResultV1, ProjectBuildReportV1, ProjectBuildStateV1,
    ProjectBuildSummaryV1, BUILD_STATE_FILE, BUILD_STATE_SCHEMA_VERSION,
    PROJECT_BUILD_REPORT_ARTIFACT_KIND, PROJECT_BUILD_REPORT_FILE, PROJECT_BUILD_REPORT_KIND,
    PROJECT_BUILD_REPORT_SCHEMA_VERSION,
};

pub use diff::{
    compute_project_diff, project_source_sha256, reasons, topological_build_order,
    AssetDiffActionV1, DiffActionKindV1, ProjectDiffV1, ResolvedLockRefV1,
    PROJECT_DIFF_SCHEMA_VERSION,
};
pub use manifest::{ValidatedAssetSpec, ValidatedManifest};
pub use plan::{
    compute_build_plan, PlanActionKindV1, ProjectBuildPlanProviderV1, ProjectBuildPlanV1,
    ProjectPlanActionV1, ProviderCapabilityInput, CACHE_KIND_CATALOG_REUSE,
    CHARACTER_VIDEO_WORKFLOW, PROJECT_BUILD_PLAN_KIND, PROJECT_BUILD_PLAN_SCHEMA_VERSION,
    STATIC_SET_WORKFLOW,
};
pub use types::{
    is_valid_id, AssetKind, GameArtAssetV1, GameArtDefaultsV1, GameArtError, GameArtManifestV1,
    GameArtProviderV1, LockKind, LockRef, GAME_ART_MANIFEST_KIND, GAME_ART_MANIFEST_SCHEMA_VERSION,
};
