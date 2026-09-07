use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

use clap::{Args, Parser, Subcommand, ValueEnum};
use forge_core::asset_project::{
    assess_character_identity_reference, export_static_pack, hash_file as hash_asset_file,
    init_project, matte_static_image, read_project, read_style_lock, resolve_relative,
    write_contact_sheet, CharacterAssetSpecV1, CharacterAssetSpecV2, CharacterEquipmentKindV1,
    ConsistencyReportV1, ConsistencyVerdict, StaticAssetKind, StaticAssetSetSpecV1,
    StaticPackContext, StaticPackItem, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
#[cfg(feature = "map-compiler")]
use forge_core::automation::CompileMapRequest;
#[cfg(feature = "collection-assets")]
use forge_core::automation::CreateCollectionLockRequest;
#[cfg(feature = "consistency-v2")]
use forge_core::automation::CreateSubjectLockRequest;
#[cfg(feature = "building-assets")]
use forge_core::automation::GenerateBuildingKitRequest;
use forge_core::automation::{
    analyze_repair, automation_profile, character_workflow_catalog, fingerprint_operation_inputs,
    hash_directory, prepare_repair_plan, register_static_catalog_output,
    run_operation_with_provider, stage_plan_job, AutomationOperation, AutomationPlan,
    CharacterRetryStage, CreateStyleLockRequest, ExternalDirectionGridFilesV1,
    GenerateCharacterPackRequest, GenerateStaticAssetSetRequest, GodotInstallRequest,
    ImportDirectionGridRequestV1, PlanStore, PrepareAssetRequest, PrepareCharacterPackRequest,
    FRONT_AUTHORITATIVE_DIRECTION_GRID_MAX_COST_TICKS, XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS,
    XAI_IMAGE_2_CANDIDATE_MODEL,
};
#[cfg(feature = "terrain-assets")]
use forge_core::automation::{CreateEnvironmentLockRequest, GenerateTerrainSetRequest};
#[cfg(feature = "consistency-v2")]
use forge_core::benchmark::{
    plan_character_benchmark, run_character_benchmark, summarize_character_benchmark,
    BenchmarkWorkflow, CharacterBenchmarkExecutionOptions, CharacterBenchmarkManifestV1,
    CharacterBenchmarkRunV1,
};
use forge_core::catalog::{
    read_project_catalog, set_catalog_review, CatalogReviewRefV1, CatalogReviewStatusV1,
};
use forge_core::character_direction_motion::{
    CharacterMotionProfileV1, DirectionMotionApprovalV1, DirectionMotionGenerationStageV1,
    DirectionMotionLockV1, DIRECTION_MOTION_APPROVAL_FILE, DIRECTION_MOTION_APPROVAL_PROFILE,
};
use forge_core::character_grid::{
    CapeHemConsistencyReportV1, CapeHemContractV1, DirectionGridAppearanceReportV1,
    DirectionGridApprovalV1, DirectionGridImportConsistencyReportV1, DirectionGridLockV1,
    DirectionGridProducerKindV1, GridPoseGuidanceV1, CAPE_HEM_CONSISTENCY_PROFILE,
    DIRECTION_GRID_APPEARANCE_FILE, DIRECTION_GRID_APPEARANCE_PROFILE,
    DIRECTION_GRID_IMPORT_CONSISTENCY_PROFILE, DIRECTION_GRID_IMPORT_LOCK_PROFILE,
    GRID_APPROVAL_FILE, GRID_APPROVAL_PROFILE,
};
#[cfg(feature = "collection-assets")]
use forge_core::collection::{
    collection_lock_path, list_collection_locks, read_static_collection_spec,
};
use forge_core::collection::{
    collection_requires_provider, read_collection_lock, CollectionConsistencyReportV1,
    CollectionItemVerdict,
};
#[cfg(feature = "consistency-v2")]
use forge_core::component::{
    inspect_component, install_component, list_components, FixtureVisionComponent, VisionComponent,
    VisionComponentRequestV1, VisionOperation,
};
use forge_core::equipment_contact::assess_hand_equipment_contact_with_kind;
use forge_core::footwear_platform::{
    assess_footwear_platform_action, repair_neutral_gray_footwear_leak,
};
use forge_core::gait_laterality::assess_front_gait_laterality;
#[cfg(feature = "game-art-manifest")]
use forge_core::game_art::{
    compute_build_plan, compute_project_diff, project_source_sha256, GameArtError,
    GameArtManifestV1, ProviderCapabilityInput, BUILD_STATE_FILE,
};
use forge_core::job::{
    JobArtifactRecord, JobLifecycleState, JobOperationKind, JobRecord, JobState, JobStore,
    SourceKind, JOB_WORKSPACE_JSON,
};
use forge_core::keyframe_cleanup::{
    AlphaEdgeHaloReportV1, ALPHA_EDGE_HALO_PROFILE, CHECKERBOARD_SHEET_MATTING_PROFILE,
    CHECKERBOARD_SHEET_MATTING_PROFILE_LEGACY,
};
use forge_core::motion_semantics::{
    assess_character_motion_semantics, audit_character_pack_motion,
};
use forge_core::portrait::{
    approve_portrait_base, read_portrait_base_lock, validate_portrait_base_approval,
    PortraitConsistencyReportV1, PortraitGenerationPhaseV1, PortraitNeutralReferencePolicyV1,
    PORTRAIT_BASE_LOCK_FILE, PORTRAIT_CONSISTENCY_REPORT_FILE,
};
#[cfg(feature = "collection-assets")]
use forge_core::project_audit::{audit_project, ProjectAuditScopeV1};
use forge_core::provider::{CredentialKind, MediaGenerationProvider, ProviderHealth};
#[cfg(feature = "consistency-v2")]
use forge_core::subject::list_subject_locks;
use forge_core::subject::{read_subject_lock, subject_lock_path};
use forge_core::workflow_graph::{read_workflow_graph, WORKFLOW_GRAPH_FILE};
#[cfg(feature = "map-compiler")]
use forge_core::world::validate_map_pack;
#[cfg(feature = "building-assets")]
use forge_core::world::BuildingKitSpecV1;
#[cfg(feature = "terrain-assets")]
use forge_core::world::{
    read_environment_lock, test_terrain_pack, TerrainSetSpecV1, ENVIRONMENT_LOCK_FILE,
};
use forge_providers::auth::{
    login_xai_device_code, logout_xai_profile, save_xai_auth_preference, CredentialStorageKind,
    CredentialStore, XaiAuthMethod,
};
use forge_providers::authorization::{
    AuthorizationManifestV1, AuthorizationStore, AuthorizedTargetV1, ProviderAuthorizationConfig,
    AUTHORIZATION_MANIFEST_SCHEMA_VERSION,
};
use forge_providers::{
    list_provider_health_noninteractive, list_provider_model_routes,
    resolve_image_model as resolve_provider_image_model, resolve_provider,
    resolve_provider_with_authorization, resolve_video_model as resolve_provider_video_model,
    XAI_PROVIDER_ID,
};
use serde::Serialize;

const JSON_SCHEMA_VERSION: &str = "1";
const V93_ASYMMETRIC_MAX_COST_TICKS: u64 = 1_400_000_000;
const V10_WALK_DOWN_MAX_COST_TICKS: u64 = 3_300_000_000;

#[derive(Parser)]
#[command(
    name = "forge",
    version,
    about = "Agent-first game asset generation and Godot delivery"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Doctor(JsonFlag),
    Asset {
        #[command(subcommand)]
        command: AssetCommand,
    },
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
    Job {
        #[command(subcommand)]
        command: JobCommand,
    },
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Style {
        #[command(subcommand)]
        command: StyleCommand,
    },
    #[cfg(feature = "collection-assets")]
    Collection {
        #[command(subcommand)]
        command: CollectionCommand,
    },
    #[cfg(feature = "consistency-v2")]
    Subject {
        #[command(subcommand)]
        command: SubjectCommand,
    },
    #[cfg(any(feature = "consistency-v2", feature = "game-art-manifest"))]
    Schema {
        #[command(subcommand)]
        command: SchemaCommand,
    },
    #[cfg(feature = "consistency-v2")]
    Component {
        #[command(subcommand)]
        command: ComponentCommand,
    },
    #[cfg(feature = "consistency-v2")]
    Benchmark {
        #[command(subcommand)]
        command: BenchmarkCommand,
    },
    #[cfg(feature = "terrain-assets")]
    Environment {
        #[command(subcommand)]
        command: EnvironmentCommand,
    },
    Generate {
        #[command(subcommand)]
        command: GenerateCommand,
    },
    #[cfg(feature = "terrain-assets")]
    Terrain {
        #[command(subcommand)]
        command: TerrainCommand,
    },
    #[cfg(feature = "building-assets")]
    Building {
        #[command(subcommand)]
        command: BuildingCommand,
    },
    #[cfg(feature = "map-compiler")]
    Map {
        #[command(subcommand)]
        command: MapCommand,
    },
    Godot {
        #[command(subcommand)]
        command: GodotCommand,
    },
    Profile {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },
    Repair {
        #[command(subcommand)]
        command: RepairCommand,
    },
    Plan {
        #[command(subcommand)]
        command: PlanCommand,
    },
    #[command(name = "__worker", hide = true)]
    Worker {
        #[arg(long)]
        job_id: String,
    },
}

#[derive(Args, Default)]
struct JsonFlag {
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum AssetCommand {
    List {
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long, default_value_t = 20)]
        recent: usize,
        #[command(flatten)]
        json: JsonFlag,
    },
    Inspect {
        #[arg(long)]
        pack: Option<PathBuf>,
        #[arg(long)]
        project: Option<PathBuf>,
        #[arg(long)]
        id: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
    #[cfg(feature = "collection-assets")]
    ExportEditable {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        output: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
    #[cfg(feature = "collection-assets")]
    ReplaceItem {
        #[arg(long)]
        id: String,
        #[arg(long)]
        item: String,
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        wait: bool,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Subcommand)]
enum PackCommand {
    Validate {
        #[arg(long)]
        path: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
    /// Run the zero-cost character motion semantics audit on an existing pack.
    AuditMotion {
        #[arg(long)]
        path: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Subcommand)]
enum JobCommand {
    List {
        #[arg(long, default_value_t = 20)]
        recent: usize,
        #[command(flatten)]
        json: JsonFlag,
    },
    Get {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Cancel {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Report {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Graph {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Replay {
        #[arg(long)]
        id: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        wait: bool,
        #[command(flatten)]
        json: JsonFlag,
    },
    Reveal {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Retry {
        #[arg(long)]
        id: String,
        #[arg(long)]
        item: Option<String>,
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=7))]
        frame: Option<u8>,
        #[arg(long, value_enum, default_value_t = CharacterRetryStageArg::Auto)]
        stage: CharacterRetryStageArg,
        /// Explicitly upgrade a stable topdown-grid@9.2.0 laterality failure
        /// to the topdown-grid@9.3.0 asymmetric-guide remediation contract.
        #[arg(long)]
        asymmetric_gait: bool,
        /// Prepare a one-request V9 DirectionGrid child whose approved
        /// front_idle is the sole identity and garment-topology authority.
        #[arg(long)]
        front_authoritative_grid: bool,
        /// Prepare the exact Image 2.0 DirectionGrid comparison child. Model
        /// overrides are accepted only for the bounded candidate route.
        #[arg(long)]
        image_model: Option<String>,
        #[arg(long)]
        wait: bool,
        #[arg(long, conflicts_with = "wait")]
        plan_only: bool,
        /// Replace an inherited real-provider authorization with a new,
        /// explicitly reviewed durable authorization.
        #[arg(long)]
        authorization: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
    /// Import an external 2x2 front/rear/right/left sheet as a local-only V9
    /// DirectionGrid review child. This never calls a media Provider.
    ImportDirectionGrid {
        #[arg(long)]
        id: String,
        #[arg(
            long,
            required_unless_present_all = ["front", "rear", "right", "left"],
            conflicts_with_all = ["front", "rear", "right", "left"]
        )]
        sheet: Option<PathBuf>,
        #[arg(long, requires_all = ["rear", "right", "left"], conflicts_with = "sheet")]
        front: Option<PathBuf>,
        #[arg(long, requires_all = ["front", "right", "left"], conflicts_with = "sheet")]
        rear: Option<PathBuf>,
        #[arg(long, requires_all = ["front", "rear", "left"], conflicts_with = "sheet")]
        right: Option<PathBuf>,
        #[arg(long, requires_all = ["front", "rear", "right"], conflicts_with = "sheet")]
        left: Option<PathBuf>,
        #[arg(long, default_value = "codex_builtin_image_gen")]
        generator: String,
        #[arg(long, value_enum)]
        cape_hem: Option<CapeHemContractArg>,
        #[arg(long)]
        note: String,
        #[arg(long)]
        wait: bool,
        #[arg(long, conflicts_with = "wait")]
        plan_only: bool,
        #[command(flatten)]
        json: JsonFlag,
    },
    /// Deterministically remove the uniquely assessed neutral-gray footwear
    /// contamination from a manually rejected V9.4 walk_down frame 2.
    CleanupFootwear {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    AssembleCharacter {
        #[arg(long)]
        base: String,
        #[arg(long = "source", required = true)]
        sources: Vec<String>,
        #[arg(long)]
        wait: bool,
        #[command(flatten)]
        json: JsonFlag,
    },
    Review {
        #[arg(long)]
        id: String,
        #[arg(long)]
        accept: bool,
        #[arg(long)]
        reason: String,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum CharacterRetryStageArg {
    #[default]
    Auto,
    Still,
    Video,
    Loop,
    Matting,
    Frame,
    Consistency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum CapeHemContractArg {
    ContinuousGoldLowerHem,
    FrontAuthoritativeNoSkirtHem,
}

impl From<CapeHemContractArg> for CapeHemContractV1 {
    fn from(value: CapeHemContractArg) -> Self {
        match value {
            CapeHemContractArg::ContinuousGoldLowerHem => Self::ContinuousGoldLowerHem,
            CapeHemContractArg::FrontAuthoritativeNoSkirtHem => Self::FrontAuthoritativeNoSkirtHem,
        }
    }
}

impl From<CharacterRetryStageArg> for CharacterRetryStage {
    fn from(value: CharacterRetryStageArg) -> Self {
        match value {
            CharacterRetryStageArg::Auto => Self::Auto,
            CharacterRetryStageArg::Still => Self::Still,
            CharacterRetryStageArg::Video => Self::Video,
            CharacterRetryStageArg::Loop => Self::Loop,
            CharacterRetryStageArg::Matting => Self::Matting,
            CharacterRetryStageArg::Frame => Self::Frame,
            CharacterRetryStageArg::Consistency => Self::Consistency,
        }
    }
}

#[derive(Subcommand)]
enum ProjectCommand {
    Init {
        #[arg(long)]
        path: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "xai")]
        provider: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Inspect {
        #[arg(long)]
        project: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
    #[cfg(feature = "game-art-manifest")]
    Diff {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
    #[cfg(feature = "game-art-manifest")]
    PlanBuild {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        resume_from_job: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
    #[cfg(feature = "collection-assets")]
    Audit {
        #[arg(long)]
        project: PathBuf,
        #[arg(long, value_enum, default_value_t = ProjectAuditScopeArg::All)]
        scope: ProjectAuditScopeArg,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "collection-assets")]
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum ProjectAuditScopeArg {
    Completeness,
    Consistency,
    Quality,
    Provenance,
    Godot,
    #[default]
    All,
}

#[cfg(feature = "collection-assets")]
impl From<ProjectAuditScopeArg> for ProjectAuditScopeV1 {
    fn from(value: ProjectAuditScopeArg) -> Self {
        match value {
            ProjectAuditScopeArg::Completeness => Self::Completeness,
            ProjectAuditScopeArg::Consistency => Self::Consistency,
            ProjectAuditScopeArg::Quality => Self::Quality,
            ProjectAuditScopeArg::Provenance => Self::Provenance,
            ProjectAuditScopeArg::Godot => Self::Godot,
            ProjectAuditScopeArg::All => Self::All,
        }
    }
}

#[derive(Subcommand)]
enum StyleCommand {
    Create(ProjectSpecInput),
    Inspect {
        #[arg(long)]
        project: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "collection-assets")]
#[derive(Subcommand)]
enum CollectionCommand {
    Create(ProjectSpecInput),
    Inspect {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        revision: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "consistency-v2")]
#[derive(Subcommand)]
enum SubjectCommand {
    Create(ProjectSpecInput),
    #[cfg(feature = "subject-import")]
    Import(SubjectImportInput),
    List {
        #[arg(long)]
        project: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
    Inspect {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        id: String,
        #[arg(long)]
        revision: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "subject-import")]
#[derive(Args)]
struct SubjectImportInput {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    spec: PathBuf,
    #[arg(long)]
    canonical: PathBuf,
    #[arg(long)]
    approval_note: String,
    #[arg(long)]
    wait: bool,
    #[arg(long, conflicts_with = "wait")]
    plan_only: bool,
    #[command(flatten)]
    json: JsonFlag,
}

#[cfg(any(feature = "consistency-v2", feature = "game-art-manifest"))]
#[derive(Subcommand)]
enum SchemaCommand {
    List(JsonFlag),
    Show {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "consistency-v2")]
#[derive(Subcommand)]
enum ComponentCommand {
    List(JsonFlag),
    Doctor {
        component: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Install {
        component: String,
        #[arg(long)]
        accept_licenses: bool,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "consistency-v2")]
#[derive(Subcommand)]
enum BenchmarkCommand {
    Validate {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long, default_value = "fixture")]
        provider: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Plan {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long, default_value = "xai")]
        provider: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Summarize {
        #[arg(long)]
        input: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
    RunCharacter {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, default_value = "fixture")]
        provider: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[arg(long, value_enum, default_value = "both")]
        workflow: BenchmarkWorkflowArg,
        #[arg(long)]
        limit: Option<usize>,
        #[arg(long)]
        skip_godot: bool,
        #[arg(long)]
        accept_provider_cost: bool,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "consistency-v2")]
#[derive(Clone, Copy, ValueEnum)]
enum BenchmarkWorkflowArg {
    Video,
    Keyframes,
    Both,
}

#[cfg(feature = "terrain-assets")]
#[derive(Subcommand)]
enum EnvironmentCommand {
    Create(ProjectSpecInput),
    Inspect {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        revision: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Subcommand)]
enum GenerateCommand {
    Character(CharacterGenerateInput),
    IconSet(ProjectSpecInput),
    PropSet(ProjectSpecInput),
    #[cfg(feature = "collection-assets")]
    PortraitSet(PortraitSetInput),
    #[cfg(feature = "collection-assets")]
    EquipmentSet(ProjectSpecInput),
    #[cfg(feature = "collection-assets")]
    DecalSet(ProjectSpecInput),
    #[cfg(feature = "terrain-assets")]
    TerrainSet(ProjectSpecInput),
    #[cfg(feature = "building-assets")]
    BuildingKit(ProjectSpecInput),
}

#[cfg(feature = "terrain-assets")]
#[derive(Subcommand)]
enum TerrainCommand {
    Test {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[arg(long, default_value_t = 32)]
        samples: u32,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "building-assets")]
#[derive(Subcommand)]
enum BuildingCommand {
    Test {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long, default_value_t = 42)]
        seed: u64,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[cfg(feature = "map-compiler")]
#[derive(Subcommand)]
enum MapCommand {
    Schema(JsonFlag),
    Compile(ProjectSpecInput),
    Validate {
        #[arg(long)]
        pack: PathBuf,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Subcommand)]
enum GodotCommand {
    PlanInstall {
        #[arg(long)]
        pack: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        catalog_project: Option<PathBuf>,
        #[arg(long)]
        target: Option<PathBuf>,
        #[arg(long)]
        asset_key: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Args)]
struct ProjectSpecInput {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    spec: PathBuf,
    #[arg(long)]
    wait: bool,
    #[arg(long, conflicts_with = "wait")]
    plan_only: bool,
    /// Durable real-provider authorization created with `forge provider authorize`.
    #[arg(long)]
    authorization: Option<String>,
    #[command(flatten)]
    json: JsonFlag,
}

#[derive(Args)]
struct CharacterGenerateInput {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    spec: PathBuf,
    /// Generate and quality-check one Character direction without exporting a
    /// partial Pack. Supported by topdown-keyframes@2.2.0 and @2.3.0.
    #[arg(long, value_parser = ["idle", "idle_down", "idle_up", "idle_right", "idle_left", "walk_up", "walk_right", "walk_left", "walk_down"])]
    validation_animation: Option<String>,
    /// V8/V9: stop after the immutable image lock so it can be reviewed before
    /// motion-video or action-grid authorization is consumed.
    #[arg(long, value_enum, default_value_t = DirectionMotionStageArg::ImageLocks)]
    direction_motion_stage: DirectionMotionStageArg,
    /// V8/V9 complete stage: reuse an explicitly approved immutable image-lock Job.
    #[arg(long)]
    image_lock_job: Option<String>,
    #[arg(long)]
    wait: bool,
    #[arg(long, conflicts_with = "wait")]
    plan_only: bool,
    /// Durable real-provider authorization created with `forge provider authorize`.
    #[arg(long)]
    authorization: Option<String>,
    #[command(flatten)]
    json: JsonFlag,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum DirectionMotionStageArg {
    #[default]
    ImageLocks,
    Complete,
}

impl From<DirectionMotionStageArg> for DirectionMotionGenerationStageV1 {
    fn from(value: DirectionMotionStageArg) -> Self {
        match value {
            DirectionMotionStageArg::ImageLocks => Self::ImageLocks,
            DirectionMotionStageArg::Complete => Self::Complete,
        }
    }
}

#[cfg(feature = "collection-assets")]
#[derive(Args)]
struct PortraitSetInput {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    spec: PathBuf,
    #[arg(long, value_enum, default_value_t = PortraitPhaseArg::Base)]
    phase: PortraitPhaseArg,
    #[arg(long, requires = "phase")]
    base_job: Option<String>,
    #[arg(
        long,
        value_enum,
        default_value_t = PortraitNeutralReferencePolicyArg::SubjectStyle
    )]
    neutral_reference_policy: PortraitNeutralReferencePolicyArg,
    #[arg(long)]
    wait: bool,
    #[arg(long, conflicts_with = "wait")]
    plan_only: bool,
    #[arg(long)]
    authorization: Option<String>,
    #[command(flatten)]
    json: JsonFlag,
}

#[cfg(feature = "collection-assets")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum PortraitPhaseArg {
    #[default]
    Base,
    Expressions,
}

#[cfg(feature = "collection-assets")]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
enum PortraitNeutralReferencePolicyArg {
    #[default]
    SubjectStyle,
    SubjectEdit,
    LegacyThreeReference,
}

#[cfg(feature = "collection-assets")]
impl From<PortraitNeutralReferencePolicyArg> for PortraitNeutralReferencePolicyV1 {
    fn from(value: PortraitNeutralReferencePolicyArg) -> Self {
        match value {
            PortraitNeutralReferencePolicyArg::SubjectStyle => Self::SubjectStyle,
            PortraitNeutralReferencePolicyArg::SubjectEdit => Self::SubjectEdit,
            PortraitNeutralReferencePolicyArg::LegacyThreeReference => Self::LegacyThreeReference,
        }
    }
}

#[derive(Subcommand)]
enum ProfileCommand {
    CharacterWorkflows(JsonFlag),
}

#[derive(Subcommand)]
enum ProviderCommand {
    List(JsonFlag),
    /// List Forge's static model routing. This is not a live entitlement or
    /// paid inference request.
    Models {
        #[arg(long)]
        provider: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Login {
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "oauth")]
        method: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[arg(long, value_enum, default_value_t = CredentialStoreArg::Keychain)]
        credential_store: CredentialStoreArg,
        #[arg(long)]
        allow_file_token_storage: bool,
        #[command(flatten)]
        json: JsonFlag,
    },
    Logout {
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Doctor {
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    /// Create a non-secret, immutable allowance whose request and cost caps
    /// are enforced across retries, replay Jobs, and independent processes.
    Authorize {
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "default")]
        profile: String,
        #[arg(long)]
        id: String,
        #[arg(long = "target", required = true)]
        targets: Vec<String>,
        #[arg(long, default_value_t = 2)]
        max_requests_per_target: u32,
        #[arg(long)]
        max_requests: Option<u32>,
        /// Exact cap on all Provider operations, including transport/poll.
        /// V9.3 asymmetric gait requires this to be explicitly set to 1.
        #[arg(long)]
        max_provider_operations: Option<u32>,
        #[arg(long)]
        max_cost_ticks: u64,
        #[arg(long)]
        cost_reservation_ticks_per_request: u64,
        #[arg(long = "model", required = true)]
        models: Vec<String>,
        #[arg(long)]
        source_job: Option<String>,
        /// Bind this immutable grant to one prepared Plan recipe hash.
        #[arg(long, requires = "input_fingerprint")]
        recipe_hash: Option<String>,
        /// Bind this immutable grant to one prepared Plan input fingerprint.
        #[arg(long, requires = "recipe_hash")]
        input_fingerprint: Option<String>,
        #[arg(long, default_value_t = 60)]
        expires_minutes: i64,
        #[command(flatten)]
        json: JsonFlag,
    },
    Authorization {
        #[arg(long)]
        id: String,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum CredentialStoreArg {
    Keychain,
    File,
}

#[derive(Subcommand)]
enum RepairCommand {
    Analyze {
        #[arg(long)]
        job: String,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Subcommand)]
enum PlanCommand {
    PrepareAsset(RequestInput),
    PrepareCharacter(RequestInput),
    GenerateCharacter(RequestInput),
    InstallGodot(RequestInput),
    RepairJob {
        #[arg(long)]
        job: String,
        #[command(flatten)]
        json: JsonFlag,
    },
    Execute {
        #[arg(long)]
        token: String,
        #[arg(long)]
        wait: bool,
        #[arg(long)]
        authorization: Option<String>,
        #[command(flatten)]
        json: JsonFlag,
    },
}

#[derive(Args)]
struct RequestInput {
    #[arg(long)]
    request: Option<PathBuf>,
    #[arg(long)]
    stdin: bool,
    #[command(flatten)]
    json: JsonFlag,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Envelope<T: Serialize> {
    schema_version: &'static str,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ErrorBody>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorBody {
    code: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DoctorOutput {
    cli_version: &'static str,
    cli_path: PathBuf,
    profile_id: String,
    profile_version: String,
    job_store: PathBuf,
    plan_store: PathBuf,
    godot_path: Option<PathBuf>,
    godot_version: Option<String>,
    godot_supported: bool,
    ffmpeg_path: Option<PathBuf>,
    ffprobe_path: Option<PathBuf>,
    platform_supported: bool,
    providers: Vec<ProviderHealth>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AssetRecord {
    job_id: String,
    asset_id: Option<String>,
    name: Option<String>,
    artifact: JobArtifactRecord,
}

fn main() {
    if let Err((code, message)) = run() {
        let envelope: Envelope<serde_json::Value> = Envelope {
            schema_version: JSON_SCHEMA_VERSION,
            ok: false,
            data: None,
            error: Some(ErrorBody { code, message }),
        };
        println!(
            "{}",
            serde_json::to_string(&envelope).expect("error envelope serializes")
        );
        std::process::exit(1);
    }
}

fn run() -> Result<(), (String, String)> {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor(_) => {
            let profile = automation_profile();
            let job_store = job_store()?;
            let plan_store = plan_store()?;
            let godot_path = locate_godot();
            let godot_version = godot_path.as_deref().and_then(godot_version);
            let ffmpeg = forge_core::video::resolve_ffmpeg_paths(
                &forge_core::video::FfmpegSearch::default(),
            )
            .ok();
            success(&DoctorOutput {
                cli_version: env!("CARGO_PKG_VERSION"),
                cli_path: env::current_exe().map_err(io_error)?,
                profile_id: profile.id,
                profile_version: profile.version,
                job_store: job_store.root().to_path_buf(),
                plan_store: plan_store.root().to_path_buf(),
                godot_path,
                godot_supported: godot_version
                    .as_deref()
                    .is_some_and(|version| version.starts_with("4.6.")),
                godot_version,
                ffmpeg_path: ffmpeg.as_ref().map(|paths| paths.ffmpeg_path.clone()),
                ffprobe_path: ffmpeg.as_ref().map(|paths| paths.ffprobe_path.clone()),
                platform_supported: cfg!(all(target_os = "macos", target_arch = "aarch64")),
                providers: list_provider_health_noninteractive(),
            })
        }
        Command::Asset { command } => match command {
            AssetCommand::List {
                project,
                kind,
                recent,
                ..
            } => {
                if let Some(project) = project {
                    let catalog = read_project_catalog(&project).map_err(display_error)?;
                    let assets = catalog
                        .assets
                        .into_values()
                        .filter(|entry| kind.as_ref().is_none_or(|kind| &entry.kind == kind))
                        .take(recent)
                        .collect::<Vec<_>>();
                    success(&assets)
                } else {
                    let assets = job_store()?
                        .list_records()
                        .map_err(display_error)?
                        .into_iter()
                        .flat_map(asset_records)
                        .take(recent)
                        .collect::<Vec<_>>();
                    success(&assets)
                }
            }
            AssetCommand::Inspect {
                pack, project, id, ..
            } => {
                if let Some(pack) = pack {
                    let summary = forge_pack::inspect_pack(&pack).map_err(display_error)?;
                    success(&summary)
                } else if let (Some(project), Some(id)) = (project, id) {
                    let catalog = read_project_catalog(&project).map_err(display_error)?;
                    let entry = catalog.assets.get(&id).ok_or_else(|| {
                        (
                            "asset_not_found".into(),
                            format!("catalog asset not found: {id}"),
                        )
                    })?;
                    success(entry)
                } else {
                    Err((
                        "invalid_arguments".into(),
                        "use --pack, or use --project together with --id".into(),
                    ))
                }
            }
            #[cfg(feature = "collection-assets")]
            AssetCommand::ExportEditable {
                project,
                id,
                output,
                ..
            } => export_editable_asset(&project, &id, &output),
            #[cfg(feature = "collection-assets")]
            AssetCommand::ReplaceItem {
                id,
                item,
                path,
                wait,
                ..
            } => replace_static_item(&id, &item, &path, wait),
        },
        Command::Pack { command } => match command {
            PackCommand::Validate { path, .. } => {
                forge_pack::validate_pack_layout(&path).map_err(display_error)?;
                success(&serde_json::json!({ "path": path, "valid": true }))
            }
            PackCommand::AuditMotion { path, .. } => {
                forge_pack::validate_pack_layout(&path).map_err(display_error)?;
                let report = audit_character_pack_motion(&path).map_err(display_error)?;
                success(&serde_json::json!({ "path": path, "report": report }))
            }
        },
        Command::Job { command } => match command {
            JobCommand::List { recent, .. } => {
                let mut records = job_store()?.list_records().map_err(display_error)?;
                records.truncate(recent);
                success(&records)
            }
            JobCommand::Get { id, .. } => {
                let record = job_store()?.read_record(&id).map_err(display_error)?;
                success(&record)
            }
            JobCommand::Cancel { id, .. } => {
                let store = job_store()?;
                store
                    .request_cancellation_cascade(&id)
                    .map_err(display_error)?;
                let record = store.read_record(&id).map_err(display_error)?;
                success(&record)
            }
            JobCommand::Report { id, .. } => {
                let record = job_store()?.read_record(&id).map_err(display_error)?;
                let report_artifacts = record
                    .artifacts
                    .iter()
                    .filter(|artifact| {
                        matches!(
                            artifact.kind.as_str(),
                            "quality_report"
                                | "animation_quality_report"
                                | "loop_selection_report"
                                | "consistency_report"
                                | "character_identity_report"
                                | "character_semantic_quality_report"
                                | "character_silhouette_temporal_report"
                                | "character_silhouette_temporal_source_report"
                                | "character_motion_semantics_report"
                                | "character_alpha_repair_report"
                                | "character_framing_report"
                                | "character_assembly_manifest"
                                | "portrait_base_lock"
                                | "portrait_base_approval"
                                | "portrait_consistency_report"
                                | "direction_lock"
                                | "contact_sheet"
                                | "provider_manifest"
                                | "provider_usage"
                                | "provider_video_ticket"
                                | "workflow_graph"
                                | "terrain_quality_report"
                                | "building_quality_report"
                                | "map_validation_report"
                        )
                    })
                    .collect::<Vec<_>>();
                let mut reports = serde_json::Map::new();
                for artifact in &report_artifacts {
                    if artifact.path.extension().and_then(|value| value.to_str()) == Some("json") {
                        if let Ok(bytes) = fs::read(&artifact.path) {
                            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                                reports.insert(artifact.kind.clone(), value);
                            }
                        }
                    }
                }
                // Keep the persisted verdict immutable, but also expose a
                // read-only diagnostic using the current local implementation.
                // This lets repaired quality profiles re-evaluate already paid
                // pixels without resolving credentials or issuing a request.
                if let Some(AutomationOperation::GenerateCharacterPack(request)) = record
                    .recipe
                    .clone()
                    .and_then(|value| serde_json::from_value(value).ok())
                {
                    let reference_path = record
                        .artifacts
                        .iter()
                        .find(|artifact| artifact.kind == "provider_reference")
                        .map(|artifact| artifact.path.clone())
                        .unwrap_or_else(|| {
                            record
                                .job_dir
                                .join("source/provider/reference/reference.png")
                        });
                    if reference_path.is_file() {
                        if let Ok(image) = matte_static_image(&reference_path) {
                            let diagnostic = assess_character_identity_reference(
                                &image,
                                &request.character.prompt,
                            );
                            reports.insert(
                                "character_identity_diagnostic".into(),
                                serde_json::json!({
                                    "source": "current_local_implementation",
                                    "referencePath": reference_path,
                                    "providerRequestOccurred": false,
                                    "report": diagnostic,
                                }),
                            );
                        }
                    }
                }
                let provider_requests = reports
                    .get("provider_manifest")
                    .and_then(|value| value.pointer("/usage/requests"))
                    .or_else(|| {
                        reports
                            .get("provider_usage")
                            .and_then(|value| value.pointer("/usage/requests"))
                    })
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(0);
                let provider_cost_ticks = reports
                    .get("provider_manifest")
                    .and_then(|value| value.pointer("/usage/costInUsdTicks"))
                    .or_else(|| {
                        reports
                            .get("provider_usage")
                            .and_then(|value| value.pointer("/usage/costInUsdTicks"))
                    })
                    .and_then(serde_json::Value::as_u64);
                let authorization_accounting = record
                    .authorization_id
                    .as_deref()
                    .map(|authorization_id| {
                        let store =
                            AuthorizationStore::open(authorization_store_root()?, authorization_id)
                                .map_err(display_error)?;
                        Ok::<_, (String, String)>(serde_json::json!({
                            "manifest": store.manifest().map_err(display_error)?,
                            "ledger": store.ledger().map_err(display_error)?,
                        }))
                    })
                    .transpose()?;
                success(&serde_json::json!({
                    "job": record,
                    "reportArtifacts": report_artifacts,
                    "reports": reports,
                    "providerRequestOccurred": provider_requests > 0,
                    "providerRequestCount": provider_requests,
                    "providerCostInUsdTicks": provider_cost_ticks,
                    "authorizationAccounting": authorization_accounting,
                }))
            }
            JobCommand::Graph { id, .. } => {
                let record = job_store()?.read_record(&id).map_err(display_error)?;
                let path = record.job_dir.join(WORKFLOW_GRAPH_FILE);
                if !path.is_file() {
                    return Err((
                        "workflow_graph_missing".into(),
                        format!("job {id} does not contain {WORKFLOW_GRAPH_FILE}"),
                    ));
                }
                success(&read_workflow_graph(&path).map_err(display_error)?)
            }
            JobCommand::Replay { id, from, wait, .. } => replay_job(&id, &from, wait),
            JobCommand::Reveal { id, .. } => {
                open_forge_job(&id)?;
                let record = job_store()?.read_record(&id).map_err(display_error)?;
                success(
                    &serde_json::json!({ "jobId": id, "jobDir": record.job_dir, "revealed": true }),
                )
            }
            JobCommand::Retry {
                id,
                item,
                frame,
                stage,
                asymmetric_gait,
                front_authoritative_grid,
                image_model,
                wait,
                plan_only,
                authorization,
                ..
            } => retry_job(
                &id,
                item.as_deref(),
                frame,
                RetryJobOptions {
                    stage,
                    asymmetric_gait,
                    front_authoritative_grid,
                    image_model: image_model.as_deref(),
                    wait,
                    plan_only,
                    authorization: authorization.as_deref(),
                },
            ),
            JobCommand::ImportDirectionGrid {
                id,
                sheet,
                front,
                rear,
                right,
                left,
                generator,
                cape_hem,
                note,
                wait,
                plan_only,
                ..
            } => import_direction_grid_job(
                &id,
                sheet.as_deref(),
                [
                    front.as_deref(),
                    rear.as_deref(),
                    right.as_deref(),
                    left.as_deref(),
                ],
                &generator,
                cape_hem.map(Into::into),
                &note,
                wait,
                plan_only,
            ),
            JobCommand::CleanupFootwear { id, .. } => cleanup_footwear_job(&id),
            JobCommand::AssembleCharacter {
                base,
                sources,
                wait,
                ..
            } => assemble_character_jobs(&base, &sources, wait),
            JobCommand::Review {
                id, accept, reason, ..
            } => review_job(&id, accept, &reason),
        },
        Command::Project { command } => match command {
            ProjectCommand::Init {
                path,
                name,
                provider,
                profile,
                ..
            } => {
                let mut project = init_project(&path, &name).map_err(display_error)?;
                if provider != "xai" || profile != "default" {
                    project.provider.id = provider;
                    project.provider.profile_id = profile;
                    fs::write(
                        path.join(FORGE_PROJECT_FILE),
                        serde_json::to_vec_pretty(&project).map_err(json_error)?,
                    )
                    .map_err(io_error)?;
                }
                success(&serde_json::json!({
                    "project": project,
                    "projectFile": path.join(FORGE_PROJECT_FILE)
                }))
            }
            ProjectCommand::Inspect { project, .. } => {
                if project.join(FORGE_PROJECT_FILE).is_file() {
                    success(&read_project(&project).map_err(display_error)?)
                } else {
                    let inspection =
                        forge_core::project::inspect_project(&project).map_err(display_error)?;
                    success(&inspection)
                }
            }
            #[cfg(feature = "game-art-manifest")]
            ProjectCommand::Diff {
                project, manifest, ..
            } => project_diff(&project, &manifest),
            #[cfg(feature = "game-art-manifest")]
            ProjectCommand::PlanBuild {
                project,
                manifest,
                resume_from_job,
                ..
            } => project_plan_build(&project, &manifest, resume_from_job.as_deref()),
            #[cfg(feature = "collection-assets")]
            ProjectCommand::Audit { project, scope, .. } => {
                success(&audit_project(&project, scope.into()).map_err(display_error)?)
            }
        },
        Command::Style { command } => match command {
            StyleCommand::Create(input) => {
                let project = read_project(&input.project).map_err(display_error)?;
                let request = CreateStyleLockRequest {
                    schema_version: "1".into(),
                    project_path: input.project,
                    spec_path: input.spec,
                    provider_id: project.provider.id,
                    profile_id: project.provider.profile_id,
                };
                prepare_or_plan(
                    AutomationOperation::CreateStyleLock(request),
                    input.wait,
                    input.plan_only,
                    input.authorization.as_deref(),
                )
            }
            StyleCommand::Inspect { project, .. } => {
                let definition = read_project(&project).map_err(display_error)?;
                let revision = definition.current_style_revision.ok_or_else(|| {
                    (
                        "style_missing".into(),
                        "project has no locked style revision".into(),
                    )
                })?;
                let path = project
                    .join(".forge/styles")
                    .join(revision)
                    .join(STYLE_LOCK_FILE);
                success(&read_style_lock(&path).map_err(display_error)?)
            }
        },
        #[cfg(feature = "collection-assets")]
        Command::Collection { command } => match command {
            CollectionCommand::Create(input) => {
                let project = read_project(&input.project).map_err(display_error)?;
                let request = CreateCollectionLockRequest {
                    schema_version: "1".into(),
                    project_path: input.project,
                    spec_path: input.spec,
                    provider_id: project.provider.id,
                    profile_id: project.provider.profile_id,
                };
                prepare_or_plan(
                    AutomationOperation::CreateCollectionLock(request),
                    input.wait,
                    input.plan_only,
                    input.authorization.as_deref(),
                )
            }
            CollectionCommand::Inspect {
                project,
                id,
                revision,
                ..
            } => {
                let path = if let Some(revision) = revision {
                    collection_lock_path(&project, &id, &revision)
                } else {
                    list_collection_locks(&project, Some(&id))
                        .map_err(display_error)?
                        .into_iter()
                        .max_by_key(|lock| lock.created_at)
                        .map(|lock| collection_lock_path(&project, &lock.id, &lock.revision))
                        .ok_or_else(|| {
                            (
                                "collection_not_found".into(),
                                format!("collection not found: {id}"),
                            )
                        })?
                };
                success(&read_collection_lock(&path).map_err(display_error)?)
            }
        },
        #[cfg(feature = "consistency-v2")]
        Command::Subject { command } => match command {
            SubjectCommand::Create(input) => {
                let project = read_project(&input.project).map_err(display_error)?;
                let request = CreateSubjectLockRequest {
                    schema_version: "1".into(),
                    project_path: input.project,
                    spec_path: input.spec,
                    provider_id: project.provider.id,
                    profile_id: project.provider.profile_id,
                    canonical_import_path: None,
                    import_approval_note: None,
                };
                prepare_or_plan(
                    AutomationOperation::CreateSubjectLock(request),
                    input.wait,
                    input.plan_only,
                    input.authorization.as_deref(),
                )
            }
            #[cfg(feature = "subject-import")]
            SubjectCommand::Import(input) => {
                let project = read_project(&input.project).map_err(display_error)?;
                let canonical = input.canonical.canonicalize().map_err(io_error)?;
                let request = CreateSubjectLockRequest {
                    schema_version: "1".into(),
                    project_path: input.project,
                    spec_path: input.spec,
                    provider_id: project.provider.id,
                    profile_id: project.provider.profile_id,
                    canonical_import_path: Some(canonical),
                    import_approval_note: Some(input.approval_note),
                };
                prepare_or_plan(
                    AutomationOperation::CreateSubjectLock(request),
                    input.wait,
                    input.plan_only,
                    None,
                )
            }
            SubjectCommand::List { project, .. } => {
                success(&list_subject_locks(&project).map_err(display_error)?)
            }
            SubjectCommand::Inspect {
                project,
                id,
                revision,
                ..
            } => {
                let path = if let Some(revision) = revision {
                    subject_lock_path(&project, &id, &revision)
                } else {
                    list_subject_locks(&project)
                        .map_err(display_error)?
                        .into_iter()
                        .filter(|lock| lock.id == id)
                        .max_by_key(|lock| lock.created_at)
                        .map(|lock| subject_lock_path(&project, &lock.id, &lock.revision))
                        .ok_or_else(|| {
                            (
                                "subject_not_found".into(),
                                format!("subject not found: {id}"),
                            )
                        })?
                };
                success(&read_subject_lock(&path).map_err(display_error)?)
            }
        },
        #[cfg(any(feature = "consistency-v2", feature = "game-art-manifest"))]
        Command::Schema { command } => match command {
            SchemaCommand::List(_) => {
                let mut schemas = vec![
                    "asset@1.0.0",
                    "style@1.0.0",
                    "character-semantic-quality-report@1.0.0",
                    "character-direction-still-preflight@1.0.0",
                    "character-silhouette-temporal-report@1.0.0",
                    "character-silhouette-temporal-report@2.0.0",
                    "character-alpha-repair-report@1.0.0",
                    "character-motion-semantics-report@1.0.0",
                    "character-gait-cycle-report@1.0.0",
                ];
                #[cfg(feature = "consistency-v2")]
                schemas.extend([
                    "character@2.0.0",
                    "character-benchmark@1.0.0",
                    "character-direction-lock@1.0.0",
                    "character-direction-lock@1.1.0",
                    "character-direction-pose-lock@1.0.0",
                    "character-direction-motion-lock@1.0.0",
                    "keyframe-background-cleanup-report@1.0.0",
                    "subject@1.0.0",
                ]);
                #[cfg(feature = "subject-import")]
                schemas.push("subject-import-report@1.0.0");
                #[cfg(feature = "grid-generation")]
                schemas.extend([
                    "character-direction-grid-lock@1.0.0",
                    "character-direction-grid-lock@1.1.0",
                    "character-direction-grid-lock@1.2.0",
                    "character-direction-grid-appearance-report@1.0.0",
                    "character-direction-grid-approval@1.0.0",
                    "grid-action-report@1.0.0",
                    "grid-action-report@1.1.0",
                    "grid-keyframe-action-report@1.0.0",
                    "grid-keyframe-action-report@1.1.0",
                    "grid-keyframe-action-report@1.2.0",
                    "grid-keyframe-action-report@1.3.0",
                    "grid-keyframe-action-report@1.4.0",
                    "grid-keyframe-action-report@1.5.0",
                    "gait-laterality@1.0.0",
                    "checkerboard-sheet-matting-report@1.0.0",
                    "checkerboard-sheet-matting-report@1.1.0",
                    "direction-grid-import-evidence@1.0.0",
                    "direction-grid-import-evidence@1.1.0",
                    "direction-grid-import-consistency@1.0.0",
                    "direction-grid-authority-alignment@1.0.0",
                    "alpha-edge-halo@1.0.0",
                    "cape-hem-consistency@1.0.0",
                    "character-anchor-stabilization@1.0.0",
                ]);
                #[cfg(feature = "map-compiler")]
                schemas.push("map@1.0.0");
                #[cfg(feature = "game-art-manifest")]
                schemas.push("game-art-manifest@1.0.0");
                #[cfg(feature = "collection-assets")]
                schemas.extend([
                    "collection@1.0.0",
                    "collection-static@2.0.0",
                    "portrait-set@1.0.0",
                    "portrait-set@2.0.0",
                    "equipment-set@1.0.0",
                    "decal-set@1.0.0",
                    "collection-anchor-quality-report@1.1.0",
                    "collection-consistency-report@1.1.0",
                    "portrait-base-lock@1.0.0",
                    "portrait-base-approval@1.0.0",
                    "portrait-local@1.0.0",
                    "portrait-local@1.1.0",
                    "project-audit-report@1.1.0",
                ]);
                schemas.extend([
                    "provider-authorization@1.0.0",
                    "provider-request-ledger@1.0.0",
                ]);
                success(&serde_json::json!({ "schemas": schemas }))
            }
            SchemaCommand::Show { id, .. } => {
                let source = match id.as_str() {
                    "asset@1.0.0" => include_str!("../../../schemas/asset-spec.schema.json"),
                    #[cfg(feature = "consistency-v2")]
                    "character@2.0.0" => {
                        include_str!("../../../schemas/character-v2.schema.json")
                    }
                    #[cfg(feature = "consistency-v2")]
                    "character-benchmark@1.0.0" => {
                        include_str!("../../../schemas/character-benchmark.schema.json")
                    }
                    #[cfg(feature = "consistency-v2")]
                    "character-direction-lock@1.0.0" => {
                        include_str!("../../../schemas/character-direction-lock.schema.json")
                    }
                    #[cfg(feature = "consistency-v2")]
                    "character-direction-lock@1.1.0" => {
                        include_str!("../../../schemas/character-direction-lock.schema.json")
                    }
                    #[cfg(feature = "consistency-v2")]
                    "character-direction-pose-lock@1.0.0" => {
                        include_str!("../../../schemas/character-direction-pose-lock.schema.json")
                    }
                    #[cfg(feature = "consistency-v2")]
                    "character-direction-motion-lock@1.0.0" => {
                        include_str!("../../../schemas/character-direction-motion-lock.schema.json")
                    }
                    #[cfg(feature = "consistency-v2")]
                    "keyframe-background-cleanup-report@1.0.0" => {
                        include_str!(
                            "../../../schemas/keyframe-background-cleanup-report.schema.json"
                        )
                    }
                    #[cfg(feature = "grid-generation")]
                    "character-direction-grid-lock@1.0.0" => {
                        include_str!("../../../schemas/character-direction-grid-lock.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "character-direction-grid-lock@1.1.0" => {
                        include_str!("../../../schemas/character-direction-grid-lock.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "character-direction-grid-lock@1.2.0" => {
                        include_str!("../../../schemas/character-direction-grid-lock.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "character-direction-grid-appearance-report@1.0.0" => {
                        include_str!(
                            "../../../schemas/character-direction-grid-appearance-report.schema.json"
                        )
                    }
                    #[cfg(feature = "grid-generation")]
                    "character-direction-grid-approval@1.0.0" => {
                        include_str!(
                            "../../../schemas/character-direction-grid-approval.schema.json"
                        )
                    }
                    #[cfg(feature = "grid-generation")]
                    "grid-action-report@1.0.0" | "grid-action-report@1.1.0" => {
                        include_str!("../../../schemas/grid-action-report.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "grid-keyframe-action-report@1.0.0"
                    | "grid-keyframe-action-report@1.1.0"
                    | "grid-keyframe-action-report@1.2.0"
                    | "grid-keyframe-action-report@1.3.0"
                    | "grid-keyframe-action-report@1.4.0"
                    | "grid-keyframe-action-report@1.5.0" => {
                        include_str!("../../../schemas/grid-keyframe-action-report.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "gait-laterality@1.0.0" => {
                        include_str!("../../../schemas/gait-laterality-report.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "checkerboard-sheet-matting-report@1.0.0"
                    | "checkerboard-sheet-matting-report@1.1.0" => {
                        include_str!(
                            "../../../schemas/checkerboard-sheet-matting-report.schema.json"
                        )
                    }
                    #[cfg(feature = "grid-generation")]
                    "direction-grid-import-evidence@1.0.0"
                    | "direction-grid-import-evidence@1.1.0" => {
                        include_str!("../../../schemas/direction-grid-import-evidence.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "direction-grid-import-consistency@1.0.0" => {
                        include_str!(
                            "../../../schemas/direction-grid-import-consistency-report.schema.json"
                        )
                    }
                    #[cfg(feature = "grid-generation")]
                    "direction-grid-authority-alignment@1.0.0" => {
                        include_str!(
                            "../../../schemas/direction-grid-authority-alignment-report.schema.json"
                        )
                    }
                    #[cfg(feature = "grid-generation")]
                    "alpha-edge-halo@1.0.0" => {
                        include_str!("../../../schemas/alpha-edge-halo-report.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "cape-hem-consistency@1.0.0" => {
                        include_str!("../../../schemas/cape-hem-consistency-report.schema.json")
                    }
                    #[cfg(feature = "grid-generation")]
                    "character-anchor-stabilization@1.0.0" => {
                        include_str!(
                            "../../../schemas/character-anchor-stabilization-report.schema.json"
                        )
                    }
                    "style@1.0.0" => include_str!("../../../schemas/style-spec.schema.json"),
                    "character-semantic-quality-report@1.0.0" => {
                        include_str!(
                            "../../../schemas/character-semantic-quality-report.schema.json"
                        )
                    }
                    "character-direction-still-preflight@1.0.0" => {
                        include_str!(
                            "../../../schemas/character-direction-still-preflight.schema.json"
                        )
                    }
                    "character-silhouette-temporal-report@1.0.0" => {
                        include_str!(
                            "../../../schemas/character-silhouette-temporal-report-v1.schema.json"
                        )
                    }
                    "character-silhouette-temporal-report@2.0.0" => {
                        include_str!(
                            "../../../schemas/character-silhouette-temporal-report.schema.json"
                        )
                    }
                    "character-alpha-repair-report@1.0.0" => {
                        include_str!("../../../schemas/character-alpha-repair-report.schema.json")
                    }
                    "character-motion-semantics-report@1.0.0" => {
                        include_str!(
                            "../../../schemas/character-motion-semantics-report.schema.json"
                        )
                    }
                    "character-gait-cycle-report@1.0.0" => {
                        include_str!("../../../schemas/character-gait-cycle-report.schema.json")
                    }
                    #[cfg(feature = "consistency-v2")]
                    "subject@1.0.0" => include_str!("../../../schemas/subject-spec.schema.json"),
                    #[cfg(feature = "subject-import")]
                    "subject-import-report@1.0.0" => {
                        include_str!("../../../schemas/subject-import-report.schema.json")
                    }
                    #[cfg(feature = "map-compiler")]
                    "map@1.0.0" => include_str!("../../../schemas/map-spec.schema.json"),
                    #[cfg(feature = "game-art-manifest")]
                    "game-art-manifest@1.0.0" => {
                        include_str!("../../../schemas/game-art-manifest.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "collection@1.0.0" => {
                        include_str!("../../../schemas/collection-spec.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "collection-static@2.0.0" => {
                        include_str!("../../../schemas/collection-static-spec.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "portrait-set@1.0.0" => {
                        include_str!("../../../schemas/portrait-set-spec.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "portrait-set@2.0.0" => {
                        include_str!("../../../schemas/portrait-set-v2.schema.json")
                    }
                    "provider-authorization@1.0.0" => {
                        include_str!("../../../schemas/provider-authorization.schema.json")
                    }
                    "provider-request-ledger@1.0.0" => {
                        include_str!("../../../schemas/provider-request-ledger.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "equipment-set@1.0.0" => {
                        include_str!("../../../schemas/equipment-set-spec.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "decal-set@1.0.0" => {
                        include_str!("../../../schemas/decal-set-spec.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "collection-anchor-quality-report@1.1.0" => {
                        include_str!(
                            "../../../schemas/collection-anchor-quality-report.schema.json"
                        )
                    }
                    #[cfg(feature = "collection-assets")]
                    "collection-consistency-report@1.1.0" => {
                        include_str!("../../../schemas/collection-consistency-report.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "portrait-base-lock@1.0.0" => {
                        include_str!("../../../schemas/portrait-base-lock.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "portrait-base-approval@1.0.0" => {
                        include_str!("../../../schemas/portrait-base-approval.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "portrait-local@1.0.0" | "portrait-local@1.1.0" => {
                        include_str!("../../../schemas/portrait-consistency-report.schema.json")
                    }
                    #[cfg(feature = "collection-assets")]
                    "project-audit-report@1.1.0" => {
                        include_str!("../../../schemas/project-audit-report.schema.json")
                    }
                    _ => return Err(("schema_not_found".into(), format!("unknown schema: {id}"))),
                };
                let schema: serde_json::Value = serde_json::from_str(source).map_err(json_error)?;
                success(&schema)
            }
        },
        #[cfg(feature = "consistency-v2")]
        Command::Component { command } => match command {
            ComponentCommand::List(_) => success(&list_components().map_err(display_error)?),
            ComponentCommand::Doctor { component, .. } => {
                if component == "fixture-vision" {
                    let response = FixtureVisionComponent
                        .invoke(&VisionComponentRequestV1 {
                            schema_version: "1".into(),
                            request_id: "forge-component-doctor".into(),
                            operation: VisionOperation::Health,
                            inputs: Vec::new(),
                            parameters: serde_json::Value::Null,
                        })
                        .map_err(display_error)?;
                    success(&response)
                } else {
                    success(&inspect_component(&component).map_err(display_error)?)
                }
            }
            ComponentCommand::Install {
                component,
                accept_licenses,
                ..
            } => success(&install_component(&component, accept_licenses).map_err(display_error)?),
        },
        #[cfg(feature = "consistency-v2")]
        Command::Benchmark { command } => match command {
            BenchmarkCommand::Validate {
                manifest,
                provider,
                profile,
                ..
            } => {
                let suite = read_benchmark_manifest(&manifest)?;
                validate_benchmark_references(&suite, &manifest)?;
                let plan =
                    plan_character_benchmark(&suite, &provider, &profile).map_err(display_error)?;
                success(&serde_json::json!({
                    "valid": true,
                    "manifest": manifest,
                    "manifestSha256": hash_asset_file(&manifest).map_err(display_error)?,
                    "plan": plan,
                }))
            }
            BenchmarkCommand::Plan {
                manifest,
                provider,
                profile,
                ..
            } => {
                let suite = read_benchmark_manifest(&manifest)?;
                validate_benchmark_references(&suite, &manifest)?;
                success(
                    &plan_character_benchmark(&suite, &provider, &profile)
                        .map_err(display_error)?,
                )
            }
            BenchmarkCommand::Summarize { input, .. } => {
                let run: CharacterBenchmarkRunV1 =
                    serde_json::from_slice(&fs::read(&input).map_err(io_error)?)
                        .map_err(json_error)?;
                success(&summarize_character_benchmark(&run).map_err(display_error)?)
            }
            BenchmarkCommand::RunCharacter {
                manifest,
                output,
                provider,
                profile,
                workflow,
                limit,
                skip_godot,
                accept_provider_cost,
                ..
            } => {
                if limit == Some(0) {
                    return Err((
                        "benchmark_limit_invalid".into(),
                        "--limit must be greater than zero".into(),
                    ));
                }
                if provider != "fixture" && !accept_provider_cost {
                    return Err((
                        "benchmark_provider_cost_not_accepted".into(),
                        "real Provider benchmarks require --accept-provider-cost after reviewing `forge benchmark plan`".into(),
                    ));
                }
                ensure_real_provider_execution(&provider)?;
                let suite = read_benchmark_manifest(&manifest)?;
                validate_benchmark_references(&suite, &manifest)?;
                if env::var_os("FORGE_CACHE_STORE").is_none() {
                    env::set_var("FORGE_CACHE_STORE", output.join("cache"));
                }
                let resolved = resolve_provider(&provider, &profile).map_err(display_error)?;
                let workflows = match workflow {
                    BenchmarkWorkflowArg::Video => vec![BenchmarkWorkflow::Video],
                    BenchmarkWorkflowArg::Keyframes => vec![BenchmarkWorkflow::Keyframes],
                    BenchmarkWorkflowArg::Both => {
                        vec![BenchmarkWorkflow::Video, BenchmarkWorkflow::Keyframes]
                    }
                };
                let godot_project = (!skip_godot).then(|| output.join("godot"));
                let (_, summary) = run_character_benchmark(
                    &suite,
                    &manifest,
                    &CharacterBenchmarkExecutionOptions {
                        output_root: output.clone(),
                        provider_id: provider,
                        profile_id: profile,
                        workflows,
                        limit,
                        godot_project,
                    },
                    resolved.as_ref(),
                )
                .map_err(display_error)?;
                success(&serde_json::json!({
                    "run": output.join("benchmark-run.json"),
                    "summary": output.join("benchmark-summary.json"),
                    "results": summary,
                }))
            }
        },
        #[cfg(feature = "terrain-assets")]
        Command::Environment { command } => match command {
            EnvironmentCommand::Create(input) => {
                let project = read_project(&input.project).map_err(display_error)?;
                let request = CreateEnvironmentLockRequest {
                    schema_version: "1".into(),
                    project_path: input.project,
                    spec_path: input.spec,
                    provider_id: project.provider.id,
                    profile_id: project.provider.profile_id,
                };
                prepare_and_execute(
                    AutomationOperation::CreateEnvironmentLock(request),
                    input.wait,
                    input.authorization.as_deref(),
                )
            }
            EnvironmentCommand::Inspect {
                project, revision, ..
            } => {
                let definition = read_project(&project).map_err(display_error)?;
                let revision = revision
                    .or(definition.current_environment_revision)
                    .ok_or_else(|| {
                        (
                            "environment_missing".into(),
                            "project has no locked environment revision".into(),
                        )
                    })?;
                let path = project
                    .join(".forge/environments")
                    .join(revision)
                    .join(ENVIRONMENT_LOCK_FILE);
                success(&read_environment_lock(&path).map_err(display_error)?)
            }
        },
        Command::Generate { command } => match command {
            GenerateCommand::Character(input) => generate_character(input),
            GenerateCommand::IconSet(input) => generate_static(input, "icon_set"),
            GenerateCommand::PropSet(input) => generate_static(input, "prop_set"),
            #[cfg(feature = "collection-assets")]
            GenerateCommand::PortraitSet(input) => generate_portrait(input),
            #[cfg(feature = "collection-assets")]
            GenerateCommand::EquipmentSet(input) => generate_static(input, "equipment_set"),
            #[cfg(feature = "collection-assets")]
            GenerateCommand::DecalSet(input) => generate_static(input, "decal_set"),
            #[cfg(feature = "terrain-assets")]
            GenerateCommand::TerrainSet(input) => generate_terrain(input),
            #[cfg(feature = "building-assets")]
            GenerateCommand::BuildingKit(input) => generate_building(input),
        },
        #[cfg(feature = "terrain-assets")]
        Command::Terrain { command } => match command {
            TerrainCommand::Test {
                pack,
                seed,
                samples,
                ..
            } => {
                let quality: serde_json::Value = serde_json::from_slice(
                    &fs::read(pack.join("quality-report.json")).map_err(io_error)?,
                )
                .map_err(json_error)?;
                let pressure = test_terrain_pack(&pack, seed, samples).map_err(display_error)?;
                let valid = quality.get("verdict").and_then(serde_json::Value::as_str)
                    == Some("game_ready")
                    && pressure.verdict == "game_ready";
                success(&serde_json::json!({
                    "pack": pack,
                    "seed": seed,
                    "samples": samples,
                    "quality": quality,
                    "pressureTest": pressure,
                    "valid": valid
                }))
            }
        },
        #[cfg(feature = "building-assets")]
        Command::Building { command } => match command {
            BuildingCommand::Test { pack, seed, .. } => {
                forge_pack::validate_pack_layout(&pack).map_err(display_error)?;
                let quality: serde_json::Value = serde_json::from_slice(
                    &fs::read(pack.join("quality-report.json")).map_err(io_error)?,
                )
                .map_err(json_error)?;
                success(&serde_json::json!({
                    "pack": pack,
                    "seed": seed,
                    "quality": quality,
                    "valid": quality.get("verdict").and_then(serde_json::Value::as_str) == Some("game_ready")
                }))
            }
        },
        #[cfg(feature = "map-compiler")]
        Command::Map { command } => match command {
            MapCommand::Schema(_) => {
                let schema: serde_json::Value =
                    serde_json::from_str(include_str!("../../../schemas/map-spec.schema.json"))
                        .map_err(json_error)?;
                success(&schema)
            }
            MapCommand::Compile(input) => {
                let request = CompileMapRequest {
                    schema_version: "1".into(),
                    project_path: input.project,
                    spec_path: input.spec,
                };
                prepare_and_execute(AutomationOperation::CompileMap(request), input.wait, None)
            }
            MapCommand::Validate { pack, .. } => {
                success(&validate_map_pack(&pack).map_err(display_error)?)
            }
        },
        Command::Godot { command } => match command {
            GodotCommand::PlanInstall {
                pack,
                project,
                catalog_project,
                target,
                asset_key,
                ..
            } => {
                let target = target.unwrap_or_else(|| {
                    let name = pack
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .unwrap_or("asset");
                    PathBuf::from("addons/forge_assets").join(sanitize_cli_id(name))
                });
                let request = GodotInstallRequest {
                    schema_version: "1".into(),
                    pack_path: pack,
                    project_path: project,
                    catalog_project_path: catalog_project,
                    target,
                    asset_key,
                    provider_refs: vec![],
                };
                let plan = plan_store()?
                    .prepare(AutomationOperation::InstallGodot(request))
                    .map_err(display_error)?;
                success(&plan)
            }
        },
        Command::Profile { command } => match command {
            ProfileCommand::CharacterWorkflows(_) => {
                let mut catalog = character_workflow_catalog();
                if !cfg!(feature = "consistency-v2") {
                    catalog.workflows.retain(|workflow| {
                        !matches!(
                            workflow.id.as_str(),
                            "topdown-video"
                                | "topdown-video-locked"
                                | "topdown-video-cycle"
                                | "topdown-keyframes"
                                | "topdown-keyposes"
                                | "topdown-spritesheet"
                                | "topdown-frames"
                        )
                    });
                }
                success(&catalog)
            }
        },
        Command::Provider { command } => match command {
            ProviderCommand::List(_) => success(&list_provider_health_noninteractive()),
            ProviderCommand::Models { provider, .. } => {
                success(&list_provider_model_routes(&provider).map_err(display_error)?)
            }
            ProviderCommand::Login {
                provider,
                method,
                profile,
                credential_store,
                allow_file_token_storage,
                ..
            } => {
                if provider != XAI_PROVIDER_ID || !matches!(method.as_str(), "oauth" | "api-key") {
                    return Err((
                        "unsupported_auth_method".into(),
                        "supported xAI methods are oauth and api-key".into(),
                    ));
                }
                if method == "api-key" && credential_store == CredentialStoreArg::File {
                    return Err((
                        "unsupported_credential_store".into(),
                        "API keys require Keychain storage; file storage is available only for Preview OAuth development"
                            .into(),
                    ));
                }
                let force_file_storage = credential_store == CredentialStoreArg::File;
                let store = CredentialStore::system(profile.clone())
                    .with_file_fallback(allow_file_token_storage)
                    .with_file_storage(force_file_storage);
                let (auth_kind, storage) = if method == "api-key" {
                    let key = rpassword::prompt_password("xAI API key: ").map_err(io_error)?;
                    forge_providers::auth::save_xai_api_key(&profile, &key)
                        .map_err(display_error)?;
                    save_xai_auth_preference(
                        &profile,
                        XaiAuthMethod::ApiKey,
                        CredentialStorageKind::Keychain,
                    )
                    .map_err(display_error)?;
                    ("api_key", CredentialStorageKind::Keychain)
                } else {
                    let storage = login_xai_device_code(
                        store,
                        |authorization| {
                            eprintln!(
                                "Open {} and enter code {}",
                                authorization
                                    .verification_uri_complete
                                    .as_deref()
                                    .unwrap_or(&authorization.verification_uri),
                                authorization.user_code
                            );
                        },
                        || false,
                    )
                    .map_err(display_error)?;
                    save_xai_auth_preference(&profile, XaiAuthMethod::OAuthDeviceCode, storage)
                        .map_err(display_error)?;
                    ("oauth_device_code", storage)
                };
                success(&serde_json::json!({
                    "providerId": provider,
                    "profileId": profile,
                    "authenticated": true,
                    "authKind": auth_kind,
                    "credentialStorage": storage,
                    "preview": method == "oauth"
                }))
            }
            ProviderCommand::Logout {
                provider, profile, ..
            } => {
                if provider != XAI_PROVIDER_ID {
                    return Err((
                        "unsupported_provider".into(),
                        format!("provider {provider} has no stored credentials"),
                    ));
                }
                logout_xai_profile(&profile).map_err(display_error)?;
                success(&serde_json::json!({
                    "providerId": provider,
                    "profileId": profile,
                    "authenticated": false
                }))
            }
            ProviderCommand::Doctor {
                provider, profile, ..
            } => success(&provider_health(&provider, &profile)),
            ProviderCommand::Authorize {
                provider,
                profile,
                id,
                targets,
                max_requests_per_target,
                max_requests,
                max_provider_operations,
                max_cost_ticks,
                cost_reservation_ticks_per_request,
                models,
                source_job,
                recipe_hash,
                input_fingerprint,
                expires_minutes,
                ..
            } => create_provider_authorization(
                &provider,
                &profile,
                &id,
                &targets,
                max_requests_per_target,
                max_requests,
                max_provider_operations,
                max_cost_ticks,
                cost_reservation_ticks_per_request,
                models,
                source_job.as_deref(),
                recipe_hash,
                input_fingerprint,
                expires_minutes,
            ),
            ProviderCommand::Authorization { id, .. } => {
                let store = AuthorizationStore::open(authorization_store_root()?, &id)
                    .map_err(display_error)?;
                success(&serde_json::json!({
                    "manifest": store.manifest().map_err(display_error)?,
                    "ledger": store.ledger().map_err(display_error)?,
                }))
            }
        },
        Command::Repair { command } => match command {
            RepairCommand::Analyze { job, .. } => {
                let analysis = analyze_repair(&job_store()?, &job).map_err(display_error)?;
                success(&analysis)
            }
        },
        Command::Plan { command } => match command {
            PlanCommand::PrepareAsset(input) => {
                let request: PrepareAssetRequest = read_request(&input)?;
                let plan = plan_store()?
                    .prepare(AutomationOperation::PrepareAsset(request))
                    .map_err(display_error)?;
                success(&plan)
            }
            PlanCommand::PrepareCharacter(input) => {
                let request: PrepareCharacterPackRequest = read_request(&input)?;
                let plan = plan_store()?
                    .prepare(AutomationOperation::PrepareCharacterPack(request))
                    .map_err(display_error)?;
                success(&plan)
            }
            PlanCommand::GenerateCharacter(input) => {
                let request: GenerateCharacterPackRequest = read_request(&input)?;
                ensure_character_workflow_enabled(&request)?;
                let mut operation = AutomationOperation::GenerateCharacterPack(request);
                resolve_operation_models(&mut operation)?;
                let plan = plan_store()?.prepare(operation).map_err(display_error)?;
                success(&plan)
            }
            PlanCommand::InstallGodot(input) => {
                let request: GodotInstallRequest = read_request(&input)?;
                let plan = plan_store()?
                    .prepare(AutomationOperation::InstallGodot(request))
                    .map_err(display_error)?;
                success(&plan)
            }
            PlanCommand::RepairJob { job, .. } => {
                let plan = prepare_repair_plan(&plan_store()?, &job_store()?, &job)
                    .map_err(display_error)?;
                success(&plan)
            }
            PlanCommand::Execute {
                token,
                wait,
                authorization,
                ..
            } => {
                let plans = plan_store()?;
                let pending = plans.inspect_pending(&token).map_err(display_error)?;
                verify_plan_inputs(&pending)?;
                preflight_plan_operation(&pending.operation)?;
                validate_scoped_character_execution_authorization(
                    &pending,
                    authorization.as_deref(),
                )?;
                let plan = plans.claim(&token).map_err(display_error)?;
                let store = job_store()?;
                let record =
                    stage_and_attach_authorization(&store, &plan, authorization.as_deref())?;
                if wait {
                    let result = run_claimed_plan(&store, &record.job_id, &plan)?;
                    success(&result)
                } else {
                    spawn_worker(&record)?;
                    success(&record)
                }
            }
        },
        Command::Worker { job_id } => {
            let store = job_store()?;
            let record = store.read_record(&job_id).map_err(display_error)?;
            let bytes = fs::read(record.job_dir.join(JOB_WORKSPACE_JSON)).map_err(io_error)?;
            let plan: AutomationPlan = serde_json::from_slice(&bytes).map_err(json_error)?;
            run_claimed_plan(&store, &job_id, &plan)?;
            Ok(())
        }
    }
}

fn success<T: Serialize>(data: &T) -> Result<(), (String, String)> {
    let envelope = Envelope {
        schema_version: JSON_SCHEMA_VERSION,
        ok: true,
        data: Some(data),
        error: None,
    };
    println!("{}", serde_json::to_string(&envelope).map_err(json_error)?);
    Ok(())
}

#[cfg(feature = "consistency-v2")]
fn read_benchmark_manifest(path: &Path) -> Result<CharacterBenchmarkManifestV1, (String, String)> {
    let manifest: CharacterBenchmarkManifestV1 =
        serde_json::from_slice(&fs::read(path).map_err(io_error)?).map_err(json_error)?;
    manifest.validate().map_err(display_error)?;
    Ok(manifest)
}

#[cfg(feature = "consistency-v2")]
fn validate_benchmark_references(
    manifest: &CharacterBenchmarkManifestV1,
    manifest_path: &Path,
) -> Result<(), (String, String)> {
    let root = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    for (owner, reference) in manifest
        .styles
        .iter()
        .flat_map(|style| {
            style
                .spec
                .reference_images
                .iter()
                .map(move |path| (format!("style {}", style.id), path))
        })
        .chain(manifest.cases.iter().flat_map(|case| {
            case.subject
                .reference_images
                .iter()
                .map(move |path| (format!("case {}", case.id), path))
        }))
    {
        if reference.is_absolute()
            || reference
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir))
        {
            return Err((
                "benchmark_reference_invalid".into(),
                format!(
                    "{owner} reference must stay relative to the benchmark manifest: {}",
                    reference.display()
                ),
            ));
        }
        let resolved = root.join(reference);
        if !resolved.is_file() {
            return Err((
                "benchmark_reference_missing".into(),
                format!("{owner} reference does not exist: {}", resolved.display()),
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "game-art-manifest")]
fn project_diff(project: &Path, manifest: &Path) -> Result<(), (String, String)> {
    let validated = GameArtManifestV1::load_validated(manifest).map_err(game_art_error)?;
    let diff = compute_project_diff(project, &validated).map_err(game_art_error)?;
    success(&diff)
}

/// Plan-only project build: computes the offline build plan and prepares a
/// single-use plan token; execution goes through `forge plan execute`. The
/// provider is only asked for its static capability descriptor — no provider
/// network I/O happens here.
#[cfg(feature = "game-art-manifest")]
fn project_plan_build(
    project: &Path,
    manifest: &Path,
    resume_from_job: Option<&str>,
) -> Result<(), (String, String)> {
    // Canonicalize before deriving any digest so symlink aliases (notably
    // macOS /tmp -> /private/tmp) cannot produce a token that invalidates
    // itself at execution.
    let canonical_project = project.canonicalize().map_err(io_error)?;
    let canonical_manifest = manifest.canonicalize().map_err(io_error)?;
    let validated =
        GameArtManifestV1::load_validated(&canonical_manifest).map_err(game_art_error)?;
    let diff = compute_project_diff(&canonical_project, &validated).map_err(game_art_error)?;
    let capabilities = provider_capability_input(
        &validated.manifest.provider.id,
        &validated.manifest.provider.profile_id,
    )?;
    let plan = compute_build_plan(&canonical_project, &validated, &diff, &capabilities)
        .map_err(game_art_error)?;
    let plan_sha256 = plan.plan_sha256();
    let source_sha256 =
        project_source_sha256(&canonical_project, &validated, &diff).map_err(game_art_error)?;
    let (resume_state_path, resume_state_sha256) = if let Some(source_job_id) = resume_from_job {
        let source = job_store()?
            .read_record(source_job_id)
            .map_err(display_error)?;
        if source.operation_kind != forge_core::job::JobOperationKind::BuildProject {
            return Err((
                "invalid_resume_source".into(),
                format!("job {source_job_id} is not a build_project job"),
            ));
        }
        if source.lifecycle_state != forge_core::job::JobLifecycleState::Failed
            || !source.recoverable
            || source.worker_pid.is_some()
        {
            return Err((
                "resume_source_active".into(),
                "resume source must be a terminal failed, recoverable build with no live worker"
                    .into(),
            ));
        }
        let path = source.job_dir.join(BUILD_STATE_FILE);
        let bytes = fs::read(&path).map_err(io_error)?;
        let state: forge_core::game_art::ProjectBuildStateV1 =
            serde_json::from_slice(&bytes).map_err(json_error)?;
        if state.plan_sha256 != plan_sha256 {
            return Err((
                "resume_plan_mismatch".into(),
                "resume source was created from a different complete build plan".into(),
            ));
        }
        (
            Some(path.canonicalize().map_err(io_error)?),
            Some(hash_asset_file(&path).map_err(display_error)?),
        )
    } else {
        (None, None)
    };
    // Build the operation through its tagged JSON form so the CLI does not
    // depend on the core facade re-exporting the request type.
    let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
        "kind": "build_project",
        "request": {
            "schemaVersion": "1",
            "projectPath": canonical_project,
            "manifestPath": canonical_manifest,
            "expectedPlanSha256": plan_sha256,
            "providerCapabilities": capabilities.capabilities,
            "imageModel": capabilities.image_model,
            "videoModel": capabilities.video_model,
            "expectedSourceSha256": source_sha256,
            "resumeFromJobId": resume_from_job,
            "resumeStatePath": resume_state_path,
            "resumeStateSha256": resume_state_sha256,
        }
    }))
    .map_err(json_error)?;
    let prepared = plan_store()?.prepare(operation).map_err(display_error)?;
    let mut plan_json = serde_json::to_value(&plan).map_err(json_error)?;
    plan_json
        .as_object_mut()
        .expect("ProjectBuildPlanV1 serializes to an object")
        .insert("planSha256".into(), plan_sha256.into());
    success(&serde_json::json!({
        "plan": plan_json,
        "token": prepared.token,
        "expiresAt": prepared.expires_at,
        "inputFingerprint": prepared.input_fingerprint,
        "effects": prepared.effects,
    }))
}

/// Fill the offline plan layer's provider input from the resolved provider's
/// static capability descriptor (snake_case names) and default model pins.
#[cfg(feature = "game-art-manifest")]
fn provider_capability_input(
    provider_id: &str,
    _profile_id: &str,
) -> Result<ProviderCapabilityInput, (String, String)> {
    let descriptor = list_provider_health_noninteractive()
        .into_iter()
        .find(|health| health.provider_id == provider_id)
        .ok_or_else(|| {
            (
                "unsupported_provider".into(),
                format!("provider {provider_id} has no static descriptor"),
            )
        })?;
    let capabilities = descriptor
        .capabilities
        .into_iter()
        .filter_map(|capability| {
            serde_json::to_value(capability)
                .ok()?
                .as_str()
                .map(str::to_owned)
        })
        .collect();
    Ok(ProviderCapabilityInput {
        capabilities,
        image_model: Some(resolve_provider_image_model(provider_id, None).map_err(display_error)?),
        video_model: Some(resolve_provider_video_model(provider_id, None).map_err(display_error)?),
    })
}

#[cfg(feature = "game-art-manifest")]
fn game_art_error(error: GameArtError) -> (String, String) {
    (error.code().into(), error.to_string())
}

fn provider_health(provider_id: &str, profile_id: &str) -> ProviderHealth {
    match resolve_provider(provider_id, profile_id) {
        Ok(provider) => provider.health_check(),
        Err(error) => list_provider_health_noninteractive()
            .into_iter()
            .find(|health| health.provider_id == provider_id)
            .map(|mut health| {
                health.authenticated = false;
                health.message = Some(error.to_string());
                health
            })
            .unwrap_or_else(|| ProviderHealth {
                provider_id: provider_id.into(),
                available: false,
                authenticated: false,
                auth_kind: CredentialKind::None,
                capabilities: Vec::new(),
                constraints: None,
                message: Some(error.to_string()),
            }),
    }
}

fn resolve_provider_for_job(
    store: &JobStore,
    job_id: &str,
    provider_id: &str,
    profile_id: &str,
) -> Result<std::sync::Arc<dyn MediaGenerationProvider>, forge_core::provider::ProviderError> {
    let record = store.read_record(job_id).map_err(|error| {
        forge_core::provider::ProviderError::Unavailable(format!(
            "cannot read Job authorization binding: {error}"
        ))
    })?;
    let Some(authorization_id) = record.authorization_id.clone() else {
        // Compatibility path for older automation. New real-provider examples
        // use a durable authorization; legacy environment guards remain
        // available until the next breaking CLI release.
        return resolve_provider(provider_id, profile_id);
    };
    let root = authorization_store_root()
        .map_err(|(_, message)| forge_core::provider::ProviderError::Unavailable(message))?;
    let mut authorization = ProviderAuthorizationConfig::new(root, authorization_id);
    authorization.expected_manifest_sha256 = record.authorization_manifest_sha256.clone();
    authorization.job_id = Some(record.job_id.clone());
    authorization.lineage_root_job_id = record.lineage_root_job_id.clone().or(Some(record.job_id));
    authorization.recipe_hash = record.recipe_hash.clone();
    authorization.input_fingerprint = record.input_hash.clone();
    resolve_provider_with_authorization(provider_id, profile_id, authorization)
}

fn run_plan_operation(
    store: &JobStore,
    job_id: &str,
    operation: &AutomationOperation,
) -> Result<JobRecord, (String, String)> {
    if let AutomationOperation::GenerateCharacterPack(request) = operation {
        ensure_character_workflow_enabled(request)?;
    }
    let durable_authorization = store
        .read_record(job_id)
        .map_err(display_error)?
        .authorization_id
        .is_some();
    if let Some(provider_id) = operation_provider_id(operation) {
        if !durable_authorization {
            ensure_real_provider_execution(provider_id)?;
        }
    }
    let provider = match operation {
        #[cfg(feature = "game-art-manifest")]
        AutomationOperation::BuildProject(request) => {
            let validated = GameArtManifestV1::load_validated(&request.manifest_path)
                .map_err(game_art_error)?;
            let diff =
                compute_project_diff(&request.project_path, &validated).map_err(game_art_error)?;
            let capabilities = ProviderCapabilityInput {
                capabilities: request.provider_capabilities.iter().cloned().collect(),
                image_model: request.image_model.clone(),
                video_model: request.video_model.clone(),
            };
            let plan = compute_build_plan(&request.project_path, &validated, &diff, &capabilities)
                .map_err(game_art_error)?;
            if plan.provider_request_estimate == 0 {
                None
            } else {
                let provider_id = validated.manifest.provider.id.as_str();
                if !durable_authorization {
                    ensure_real_provider_execution(provider_id)?;
                }
                ensure_project_plan_budget(&plan)?;
                Some(
                    resolve_provider_for_job(
                        store,
                        job_id,
                        provider_id,
                        &validated.manifest.provider.profile_id,
                    )
                    .map_err(display_error)?,
                )
            }
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            Some(if character_request_is_local_only(request) {
                forge_providers::resolve_local_replay_provider(&request.provider_id)
                    .map_err(display_error)?
            } else {
                resolve_provider_for_job(store, job_id, &request.provider_id, &request.profile_id)
                    .map_err(display_error)?
            })
        }
        AutomationOperation::CreateStyleLock(request) => Some(
            resolve_provider_for_job(store, job_id, &request.provider_id, &request.profile_id)
                .map_err(display_error)?,
        ),
        AutomationOperation::CreateSubjectLock(request) => {
            if request.is_local_import() {
                None
            } else {
                Some(
                    resolve_provider_for_job(
                        store,
                        job_id,
                        &request.provider_id,
                        &request.profile_id,
                    )
                    .map_err(display_error)?,
                )
            }
        }
        AutomationOperation::CreateCollectionLock(request) => {
            if collection_requires_provider(&request.spec_path).map_err(display_error)? {
                Some(
                    resolve_provider_for_job(
                        store,
                        job_id,
                        &request.provider_id,
                        &request.profile_id,
                    )
                    .map_err(display_error)?,
                )
            } else {
                None
            }
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            if request.consistency_recheck_only {
                None
            } else {
                Some(
                    resolve_provider_for_job(
                        store,
                        job_id,
                        &request.provider_id,
                        &request.profile_id,
                    )
                    .map_err(display_error)?,
                )
            }
        }
        AutomationOperation::CreateEnvironmentLock(request) => Some(
            resolve_provider_for_job(store, job_id, &request.provider_id, &request.profile_id)
                .map_err(display_error)?,
        ),
        AutomationOperation::GenerateTerrainSet(request) => Some(
            resolve_provider_for_job(store, job_id, &request.provider_id, &request.profile_id)
                .map_err(display_error)?,
        ),
        AutomationOperation::GenerateBuildingKit(request) => Some(
            resolve_provider_for_job(store, job_id, &request.provider_id, &request.profile_id)
                .map_err(display_error)?,
        ),
        _ => None,
    };
    run_operation_with_provider(store, job_id, operation, provider.as_deref())
        .map_err(|error| (error.code().into(), error.to_string()))
}

fn verify_plan_inputs(plan: &AutomationPlan) -> Result<(), (String, String)> {
    let current = fingerprint_operation_inputs(&plan.operation).map_err(display_error)?;
    if current != plan.input_fingerprint {
        return Err((
            "input_changed".into(),
            "operation inputs changed after plan creation".into(),
        ));
    }
    Ok(())
}

fn run_claimed_plan(
    store: &JobStore,
    job_id: &str,
    plan: &AutomationPlan,
) -> Result<JobRecord, (String, String)> {
    let record = store.read_record(job_id).map_err(display_error)?;
    if record.lifecycle_state != JobLifecycleState::Queued
        || record.recipe_hash.as_deref() != Some(plan.recipe_hash.as_str())
        || record.input_hash.as_deref() != Some(plan.input_fingerprint.as_str())
    {
        return Err((
            "job_execution_not_queued".into(),
            "the staged Job is not queued for this exact single-use plan".into(),
        ));
    }
    if let Err(error) = verify_plan_inputs(plan) {
        let _ = store.update_record(job_id, |record| {
            record.state = forge_core::job::JobState::Failed;
            record.lifecycle_state = forge_core::job::JobLifecycleState::Failed;
            record.worker_pid = None;
            record.error_code = Some(error.0.clone());
            record.error_summary = Some(error.1.clone());
            record.next_actions = vec!["prepare_new_plan".into(), "job_report".into()];
        });
        return Err(error);
    }
    run_plan_operation(store, job_id, &plan.operation)
}

fn preflight_plan_operation(operation: &AutomationOperation) -> Result<(), (String, String)> {
    if let AutomationOperation::GenerateCharacterPack(request) = operation {
        ensure_character_workflow_enabled(request)?;
    }
    if let AutomationOperation::BuildProject(request) = operation {
        #[cfg(not(feature = "game-art-manifest"))]
        {
            let _ = request;
            return Err((
                "feature_not_available".into(),
                "this forge binary was built without game-art-manifest support".into(),
            ));
        }
        #[cfg(feature = "game-art-manifest")]
        {
            let validated = GameArtManifestV1::load_validated(&request.manifest_path)
                .map_err(game_art_error)?;
            let diff =
                compute_project_diff(&request.project_path, &validated).map_err(game_art_error)?;
            let capabilities = ProviderCapabilityInput {
                capabilities: request.provider_capabilities.iter().cloned().collect(),
                image_model: request.image_model.clone(),
                video_model: request.video_model.clone(),
            };
            let plan = compute_build_plan(&request.project_path, &validated, &diff, &capabilities)
                .map_err(game_art_error)?;
            if request
                .expected_plan_sha256
                .as_deref()
                .is_some_and(|expected| expected != plan.plan_sha256())
            {
                return Err((
                    "input_changed".into(),
                    "project inputs changed after plan creation".into(),
                ));
            }
            if !plan.unmet_capabilities.is_empty() {
                return Err((
                    "provider_capability_missing".into(),
                    format!(
                        "provider is missing required capabilities: {}",
                        plan.unmet_capabilities.join(", ")
                    ),
                ));
            }
            let source_sha256 = project_source_sha256(&request.project_path, &validated, &diff)
                .map_err(game_art_error)?;
            if request
                .expected_source_sha256
                .as_deref()
                .is_some_and(|expected| expected != source_sha256)
            {
                return Err((
                    "input_changed".into(),
                    "project source closure changed after plan creation".into(),
                ));
            }
            validate_resume_preflight(request, &plan.plan_sha256())?;
            if plan.provider_request_estimate > 0 {
                ensure_real_provider_execution(&validated.manifest.provider.id)?;
                ensure_project_plan_budget(&plan)?;
                resolve_provider(
                    &validated.manifest.provider.id,
                    &validated.manifest.provider.profile_id,
                )
                .map_err(display_error)?;
            }
            return Ok(());
        }
    }
    if let Some(provider_id) = operation_provider_id(operation) {
        ensure_real_provider_execution(provider_id)?;
        let profile_id = match operation {
            AutomationOperation::GenerateCharacterPack(request) => &request.profile_id,
            AutomationOperation::CreateStyleLock(request) => &request.profile_id,
            AutomationOperation::CreateSubjectLock(request) => &request.profile_id,
            AutomationOperation::CreateCollectionLock(request) => &request.profile_id,
            AutomationOperation::GenerateStaticAssetSet(request) => &request.profile_id,
            AutomationOperation::CreateEnvironmentLock(request) => &request.profile_id,
            AutomationOperation::GenerateTerrainSet(request) => &request.profile_id,
            AutomationOperation::GenerateBuildingKit(request) => &request.profile_id,
            _ => unreachable!("provider operation has a profile"),
        };
        resolve_provider(provider_id, profile_id).map_err(display_error)?;
    }
    Ok(())
}

fn operation_provider_id(operation: &AutomationOperation) -> Option<&str> {
    match operation {
        AutomationOperation::GenerateCharacterPack(request) => {
            (!character_request_is_local_only(request)).then_some(request.provider_id.as_str())
        }
        AutomationOperation::CreateStyleLock(request) => Some(&request.provider_id),
        AutomationOperation::CreateSubjectLock(request) => {
            (!request.is_local_import()).then_some(request.provider_id.as_str())
        }
        AutomationOperation::CreateCollectionLock(request) => {
            collection_requires_provider(&request.spec_path)
                .unwrap_or(true)
                .then_some(request.provider_id.as_str())
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            (!request.consistency_recheck_only).then_some(request.provider_id.as_str())
        }
        AutomationOperation::CreateEnvironmentLock(request) => Some(&request.provider_id),
        AutomationOperation::GenerateTerrainSet(request) => Some(&request.provider_id),
        AutomationOperation::GenerateBuildingKit(request) => Some(&request.provider_id),
        _ => None,
    }
}

fn character_request_is_local_only(request: &GenerateCharacterPackRequest) -> bool {
    !request.retry_animations.is_empty()
        && request.retry_animations.iter().all(|animation| {
            request.retry_stages.get(animation).is_some_and(|stage| {
                matches!(
                    stage,
                    CharacterRetryStage::Loop
                        | CharacterRetryStage::Matting
                        | CharacterRetryStage::Consistency
                )
            })
        })
}

fn ensure_real_provider_execution(provider_id: &str) -> Result<(), (String, String)> {
    if provider_id == "fixture" {
        return Ok(());
    }
    if env::var("FORGE_REAL_PROVIDER_ACCEPT").as_deref() != Ok("1") {
        return Err((
            "real_provider_not_accepted".into(),
            "set FORGE_REAL_PROVIDER_ACCEPT=1 only after reviewing the plan".into(),
        ));
    }
    for name in [
        "FORGE_REAL_PROVIDER_MAX_REQUESTS",
        "FORGE_REAL_PROVIDER_MAX_COST_TICKS",
    ] {
        let valid = env::var(name)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .is_some_and(|value| value > 0);
        if !valid {
            return Err((
                "real_provider_not_accepted".into(),
                format!("{name} must be set to a positive integer"),
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "game-art-manifest")]
fn ensure_project_plan_budget(
    plan: &forge_core::game_art::ProjectBuildPlanV1,
) -> Result<(), (String, String)> {
    if plan.provider.id == "fixture" {
        return Ok(());
    }
    let max_requests = env::var("FORGE_REAL_PROVIDER_MAX_REQUESTS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    if max_requests < plan.maximum_provider_requests {
        return Err((
            "provider_budget_exceeded".into(),
            format!(
                "reviewed project plan may require {} requests, but FORGE_REAL_PROVIDER_MAX_REQUESTS={max_requests}",
                plan.maximum_provider_requests
            ),
        ));
    }
    if let Some(maximum_cost) = plan.maximum_cost_ticks {
        let accepted = env::var("FORGE_REAL_PROVIDER_MAX_COST_TICKS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        if accepted < maximum_cost {
            return Err((
                "provider_budget_exceeded".into(),
                format!(
                    "reviewed project plan may cost {maximum_cost} ticks, but FORGE_REAL_PROVIDER_MAX_COST_TICKS={accepted}"
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "game-art-manifest")]
fn validate_resume_preflight(
    request: &forge_core::automation::BuildProjectRequestV1,
    plan_sha256: &str,
) -> Result<(), (String, String)> {
    match (
        request.resume_from_job_id.as_deref(),
        request.resume_state_path.as_ref(),
        request.resume_state_sha256.as_deref(),
    ) {
        (None, None, None) => Ok(()),
        (Some(job_id), Some(path), Some(expected_sha256)) => {
            let source = job_store()?.read_record(job_id).map_err(display_error)?;
            if source.operation_kind != forge_core::job::JobOperationKind::BuildProject {
                return Err((
                    "invalid_resume_source".into(),
                    format!("job {job_id} is not a build_project job"),
                ));
            }
            if source.lifecycle_state != forge_core::job::JobLifecycleState::Failed
                || !source.recoverable
                || source.worker_pid.is_some()
            {
                return Err((
                    "resume_source_active".into(),
                    "resume source must be a terminal failed, recoverable build with no live worker"
                        .into(),
                ));
            }
            let expected_path = source.job_dir.join(BUILD_STATE_FILE);
            if path.canonicalize().map_err(io_error)?
                != expected_path.canonicalize().map_err(io_error)?
            {
                return Err((
                    "invalid_resume_source".into(),
                    "resume state path does not belong to the source job".into(),
                ));
            }
            let actual = hash_asset_file(path).map_err(display_error)?;
            if actual != expected_sha256 {
                return Err((
                    "input_changed".into(),
                    "resume state changed after plan creation".into(),
                ));
            }
            let state: forge_core::game_art::ProjectBuildStateV1 =
                serde_json::from_slice(&fs::read(path).map_err(io_error)?).map_err(json_error)?;
            if state.plan_sha256 != plan_sha256 {
                return Err((
                    "resume_plan_mismatch".into(),
                    "resume state belongs to a different complete plan".into(),
                ));
            }
            Ok(())
        }
        _ => Err((
            "invalid_resume_source".into(),
            "resumeFromJobId, resumeStatePath and resumeStateSha256 must be supplied together"
                .into(),
        )),
    }
}

fn ensure_character_workflow_enabled(
    request: &GenerateCharacterPackRequest,
) -> Result<(), (String, String)> {
    if matches!(
        request.workflow.id.as_str(),
        "topdown-video"
            | "topdown-video-locked"
            | "topdown-video-cycle"
            | "topdown-keyframes"
            | "topdown-keyposes"
            | "topdown-spritesheet"
            | "topdown-frames"
    ) && !cfg!(feature = "consistency-v2")
    {
        return Err((
            "feature_not_available".into(),
            "topdown video/locked-video/video-cycle/keyframe/keypose/spritesheet/frames workflows require a Forge consistency-v2 build"
                .into(),
        ));
    }
    if (request.workflow.id == "topdown-grid"
        && matches!(
            request.workflow.version.as_str(),
            "9.0.0" | "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
        )
        || (request.workflow.id == "topdown-cycle" && request.workflow.version == "10.0.0"))
        && !cfg!(feature = "grid-generation")
    {
        return Err((
            "feature_not_available".into(),
            "topdown-grid@9.x and topdown-cycle@10.0.0 require a Forge grid-generation build"
                .into(),
        ));
    }
    Ok(())
}

fn prepare_and_execute(
    mut operation: AutomationOperation,
    wait: bool,
    authorization: Option<&str>,
) -> Result<(), (String, String)> {
    resolve_operation_models(&mut operation)?;
    let plans = plan_store()?;
    let plan = plans.prepare(operation).map_err(display_error)?;
    let pending = plans.inspect_pending(&plan.token).map_err(display_error)?;
    verify_plan_inputs(&pending)?;
    preflight_plan_operation(&pending.operation)?;
    validate_scoped_character_execution_authorization(&pending, authorization)?;
    let claimed = plans.claim(&plan.token).map_err(display_error)?;
    let store = job_store()?;
    let record = stage_and_attach_authorization(&store, &claimed, authorization)?;
    if wait {
        let completed = run_plan_operation(&store, &record.job_id, &claimed.operation)?;
        success(&completed)
    } else {
        spawn_worker(&record)?;
        success(&record)
    }
}

fn validate_scoped_character_execution_authorization(
    plan: &AutomationPlan,
    authorization: Option<&str>,
) -> Result<(), (String, String)> {
    let AutomationOperation::GenerateCharacterPack(request) = &plan.operation else {
        return Ok(());
    };
    if request.direction_grid_cape_contract == Some(CapeHemContractV1::FrontAuthoritativeNoSkirtHem)
    {
        return validate_front_authoritative_grid_authorization(plan, request, authorization);
    }
    if request.generation.image_model.as_deref() == Some(XAI_IMAGE_2_CANDIDATE_MODEL) {
        return validate_image2_candidate_execution_authorization(plan, request, authorization);
    }
    if request.workflow.id == "topdown-cycle" && request.workflow.version == "10.0.0" {
        return validate_v10_execution_authorization(plan, request, authorization);
    }
    if request.workflow.id != "topdown-grid"
        || !matches!(
            request.workflow.version.as_str(),
            "9.3.0" | "9.4.0" | "9.5.0"
        )
    {
        return Ok(());
    }
    let workflow = format!("topdown-grid@{}", request.workflow.version);
    let authorization = authorization.ok_or_else(|| {
        (
            "asymmetric_gait_authorization_required".into(),
            format!("{workflow} requires a new explicit authorization"),
        )
    })?;
    let source_dir = request.reuse_from_job_dir.as_ref().ok_or_else(|| {
        (
            "invalid_asymmetric_gait_remediation".into(),
            format!("{workflow} requires its immutable predecessor source Job"),
        )
    })?;
    let source: JobRecord =
        serde_json::from_slice(&fs::read(source_dir.join("job.json")).map_err(io_error)?)
            .map_err(json_error)?;
    if source.authorization_id.as_deref() == Some(authorization) {
        return Err((
            "asymmetric_gait_authorization_not_independent".into(),
            format!("{workflow} may not reuse the source Job authorization"),
        ));
    }
    let authorization_store = AuthorizationStore::open(authorization_store_root()?, authorization)
        .map_err(display_error)?;
    let manifest = authorization_store.manifest().map_err(display_error)?;
    let model = request.generation.image_model.as_deref().ok_or_else(|| {
        (
            "asymmetric_gait_authorization_model_missing".into(),
            format!("{workflow} must resolve one image model before authorization"),
        )
    })?;
    let lineage_root = source
        .lineage_root_job_id
        .as_deref()
        .unwrap_or(&source.job_id);
    validate_v93_authorization_manifest(
        &manifest,
        request,
        model,
        lineage_root,
        &plan.recipe_hash,
        &plan.input_fingerprint,
    )?;
    if authorization_store.ensure_active_and_pristine().is_err() {
        return Err((
            "asymmetric_gait_authorization_not_fresh".into(),
            format!("{workflow} requires an unexpired authorization with a physically empty request ledger"),
        ));
    }
    Ok(())
}

fn validate_front_authoritative_grid_authorization(
    plan: &AutomationPlan,
    request: &GenerateCharacterPackRequest,
    authorization: Option<&str>,
) -> Result<(), (String, String)> {
    if request.provider_id == "fixture" {
        return Ok(());
    }
    let authorization = authorization.ok_or_else(|| {
        (
            "front_authoritative_direction_grid_authorization_required".into(),
            "front-authoritative DirectionGrid generation requires a new explicit authorization"
                .into(),
        )
    })?;
    let source_dir = request.reuse_from_job_dir.as_ref().ok_or_else(|| {
        (
            "front_authoritative_direction_grid_source_missing".into(),
            "front-authoritative DirectionGrid generation requires its immutable approved source"
                .into(),
        )
    })?;
    let source: JobRecord =
        serde_json::from_slice(&fs::read(source_dir.join("job.json")).map_err(io_error)?)
            .map_err(json_error)?;
    if source.authorization_id.as_deref() == Some(authorization) {
        return Err((
            "front_authoritative_direction_grid_authorization_not_independent".into(),
            "the new grid may not reuse its source Job authorization".into(),
        ));
    }
    let authorization_store = AuthorizationStore::open(authorization_store_root()?, authorization)
        .map_err(display_error)?;
    let manifest = authorization_store.manifest().map_err(display_error)?;
    let model = request.generation.image_model.as_deref().ok_or_else(|| {
        (
            "front_authoritative_direction_grid_model_missing".into(),
            "the reviewed Plan must bind one image model".into(),
        )
    })?;
    let lineage_root = source
        .lineage_root_job_id
        .as_deref()
        .unwrap_or(&source.job_id);
    validate_front_authoritative_grid_authorization_manifest(
        &manifest,
        request,
        model,
        lineage_root,
        &plan.recipe_hash,
        &plan.input_fingerprint,
    )?;
    authorization_store
        .ensure_active_and_pristine()
        .map_err(|_| {
            (
                "front_authoritative_direction_grid_authorization_not_fresh".into(),
                "requires an unexpired authorization with a physically empty request ledger".into(),
            )
        })?;
    Ok(())
}

fn validate_front_authoritative_grid_authorization_manifest(
    manifest: &AuthorizationManifestV1,
    request: &GenerateCharacterPackRequest,
    model: &str,
    lineage_root: &str,
    recipe_hash: &str,
    input_fingerprint: &str,
) -> Result<(), (String, String)> {
    let exact_target = manifest.allowed_targets.as_slice()
        == [AuthorizedTargetV1 {
            target_id: "direction_grid".into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: FRONT_AUTHORITATIVE_DIRECTION_GRID_MAX_COST_TICKS,
        }];
    if request.provider_id != "xai"
        || request.profile_id != "default"
        || manifest.provider_id != "xai"
        || manifest.profile_id != "default"
        || manifest.allowed_models.as_slice() != [model]
        || manifest.max_total_requests != 1
        || manifest.max_total_provider_operations != Some(1)
        || manifest.max_total_cost_ticks != FRONT_AUTHORITATIVE_DIRECTION_GRID_MAX_COST_TICKS
        || !exact_target
        || manifest.source_lineage_root_job_id.as_deref() != Some(lineage_root)
        || manifest.recipe_hash.as_deref() != Some(recipe_hash)
        || manifest.input_fingerprint.as_deref() != Some(input_fingerprint)
    {
        return Err((
            "front_authoritative_direction_grid_authorization_scope_mismatch".into(),
            format!(
                "requires xai/default, one model {model}, target direction_grid, one request, one provider operation, {FRONT_AUTHORITATIVE_DIRECTION_GRID_MAX_COST_TICKS} cost ticks, approved-source lineage, and the exact reviewed Plan hashes"
            ),
        ));
    }
    Ok(())
}

fn validate_image2_candidate_execution_authorization(
    plan: &AutomationPlan,
    request: &GenerateCharacterPackRequest,
    authorization: Option<&str>,
) -> Result<(), (String, String)> {
    let authorization = authorization.ok_or_else(|| {
        (
            "xai_image2_candidate_authorization_required".into(),
            "Image 2.0 DirectionGrid comparison requires a new explicit authorization".into(),
        )
    })?;
    let source_dir = request.reuse_from_job_dir.as_ref().ok_or_else(|| {
        (
            "xai_image2_candidate_source_missing".into(),
            "Image 2.0 comparison requires its immutable approved V9 source Job".into(),
        )
    })?;
    let source: JobRecord =
        serde_json::from_slice(&fs::read(source_dir.join("job.json")).map_err(io_error)?)
            .map_err(json_error)?;
    if source.authorization_id.as_deref() == Some(authorization) {
        return Err((
            "xai_image2_candidate_authorization_not_independent".into(),
            "Image 2.0 comparison may not reuse the approved V9 source authorization".into(),
        ));
    }
    let authorization_store = AuthorizationStore::open(authorization_store_root()?, authorization)
        .map_err(display_error)?;
    let manifest = authorization_store.manifest().map_err(display_error)?;
    let lineage_root = source
        .lineage_root_job_id
        .as_deref()
        .unwrap_or(&source.job_id);
    validate_image2_candidate_authorization_manifest(
        &manifest,
        request,
        lineage_root,
        &plan.recipe_hash,
        &plan.input_fingerprint,
    )?;
    authorization_store
        .ensure_active_and_pristine()
        .map_err(|_| {
            (
                "xai_image2_candidate_authorization_not_fresh".into(),
                "Image 2.0 comparison requires an unexpired authorization with a physically empty request ledger"
                    .into(),
            )
        })?;
    Ok(())
}

fn validate_image2_candidate_authorization_manifest(
    manifest: &AuthorizationManifestV1,
    request: &GenerateCharacterPackRequest,
    lineage_root: &str,
    recipe_hash: &str,
    input_fingerprint: &str,
) -> Result<(), (String, String)> {
    let exact_target = manifest.allowed_targets.as_slice()
        == [AuthorizedTargetV1 {
            target_id: "direction_grid".into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS,
        }];
    if request.provider_id != "xai"
        || request.profile_id != "default"
        || request.generation.image_model.as_deref() != Some(XAI_IMAGE_2_CANDIDATE_MODEL)
        || manifest.provider_id != "xai"
        || manifest.profile_id != "default"
        || manifest.allowed_models.as_slice() != [XAI_IMAGE_2_CANDIDATE_MODEL]
        || manifest.max_total_requests != 1
        || manifest.max_total_provider_operations != Some(1)
        || manifest.max_total_cost_ticks != XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS
        || !exact_target
        || manifest.source_lineage_root_job_id.as_deref() != Some(lineage_root)
        || manifest.recipe_hash.as_deref() != Some(recipe_hash)
        || manifest.input_fingerprint.as_deref() != Some(input_fingerprint)
    {
        return Err((
            "xai_image2_candidate_authorization_scope_mismatch".into(),
            format!(
                "Image 2.0 comparison requires xai/default, model {XAI_IMAGE_2_CANDIDATE_MODEL}, target direction_grid, one request, one model operation, {XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS} cost ticks, approved V9 lineage, and exact reviewed Plan hashes"
            ),
        ));
    }
    Ok(())
}

fn validate_v10_execution_authorization(
    plan: &AutomationPlan,
    request: &GenerateCharacterPackRequest,
    authorization: Option<&str>,
) -> Result<(), (String, String)> {
    if request.provider_id == "fixture" {
        return Ok(());
    }
    let authorization = authorization.ok_or_else(|| {
        (
            "topdown_cycle_v10_authorization_required".into(),
            "topdown-cycle@10.0.0 real walk_down validation requires a new explicit authorization"
                .into(),
        )
    })?;
    let source_dir = request.reuse_from_job_dir.as_ref().ok_or_else(|| {
        (
            "topdown_cycle_v10_source_missing".into(),
            "topdown-cycle@10.0.0 requires its immutable approved V9 source Job".into(),
        )
    })?;
    let source: JobRecord =
        serde_json::from_slice(&fs::read(source_dir.join("job.json")).map_err(io_error)?)
            .map_err(json_error)?;
    if source.authorization_id.as_deref() == Some(authorization) {
        return Err((
            "topdown_cycle_v10_authorization_not_independent".into(),
            "topdown-cycle@10.0.0 may not reuse the approved V9 source authorization".into(),
        ));
    }
    let authorization_store = AuthorizationStore::open(authorization_store_root()?, authorization)
        .map_err(display_error)?;
    let manifest = authorization_store.manifest().map_err(display_error)?;
    let model = request.generation.video_model.as_deref().ok_or_else(|| {
        (
            "topdown_cycle_v10_authorization_model_missing".into(),
            "topdown-cycle@10.0.0 must resolve one video model before authorization".into(),
        )
    })?;
    let lineage_root = source
        .lineage_root_job_id
        .as_deref()
        .unwrap_or(&source.job_id);
    validate_v10_authorization_manifest(
        &manifest,
        request,
        model,
        lineage_root,
        &plan.recipe_hash,
        &plan.input_fingerprint,
    )?;
    authorization_store
        .ensure_active_and_pristine()
        .map_err(|_| {
            (
                "topdown_cycle_v10_authorization_not_fresh".into(),
                "topdown-cycle@10.0.0 requires an unexpired authorization with a physically empty request ledger"
                    .into(),
            )
        })?;
    Ok(())
}

fn validate_v10_authorization_manifest(
    manifest: &AuthorizationManifestV1,
    request: &GenerateCharacterPackRequest,
    model: &str,
    lineage_root: &str,
    recipe_hash: &str,
    input_fingerprint: &str,
) -> Result<(), (String, String)> {
    let target = "walk_down:video";
    let exact_target = manifest.allowed_targets.as_slice()
        == [AuthorizedTargetV1 {
            target_id: target.into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: V10_WALK_DOWN_MAX_COST_TICKS,
        }];
    if request.provider_id != "xai"
        || request.profile_id != "default"
        || !request.validation_only
        || request.validation_animations != ["walk_down"]
        || request.metadata.default_animation != "walk_down"
        || request.generation.video_duration_seconds != 4
        || model != "grok-imagine-video-1.5"
        || manifest.provider_id != request.provider_id
        || manifest.profile_id != request.profile_id
        || manifest.max_total_requests != 1
        || manifest.max_total_provider_operations != Some(1)
        || manifest.max_total_cost_ticks != V10_WALK_DOWN_MAX_COST_TICKS
        || manifest.allowed_models.as_slice() != [model]
        || !exact_target
        || manifest.source_lineage_root_job_id.as_deref() != Some(lineage_root)
        || manifest.recipe_hash.as_deref() != Some(recipe_hash)
        || manifest.input_fingerprint.as_deref() != Some(input_fingerprint)
    {
        return Err((
            "topdown_cycle_v10_authorization_scope_mismatch".into(),
            format!(
                "topdown-cycle@10.0.0 requires xai/default, one model grok-imagine-video-1.5, one target {target}, one generation request, one model operation, {V10_WALK_DOWN_MAX_COST_TICKS} cost ticks, the approved V9 lineage, and exact reviewed Plan hashes"
            ),
        ));
    }
    Ok(())
}

fn validate_v93_authorization_manifest(
    manifest: &AuthorizationManifestV1,
    request: &GenerateCharacterPackRequest,
    model: &str,
    lineage_root: &str,
    recipe_hash: &str,
    input_fingerprint: &str,
) -> Result<(), (String, String)> {
    let target = "walk_down:frame:2";
    let exact_target = manifest.allowed_targets.as_slice()
        == [AuthorizedTargetV1 {
            target_id: target.into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: V93_ASYMMETRIC_MAX_COST_TICKS,
        }];
    if manifest.provider_id != request.provider_id
        || manifest.profile_id != request.profile_id
        || manifest.max_total_requests != 1
        || manifest.max_total_provider_operations != Some(1)
        || manifest.max_total_cost_ticks != V93_ASYMMETRIC_MAX_COST_TICKS
        || manifest.allowed_models.as_slice() != [model]
        || !exact_target
        || manifest.source_lineage_root_job_id.as_deref() != Some(lineage_root)
        || manifest.recipe_hash.as_deref() != Some(recipe_hash)
        || manifest.input_fingerprint.as_deref() != Some(input_fingerprint)
    {
        return Err((
            "asymmetric_gait_authorization_scope_mismatch".into(),
            format!(
                "topdown-grid@9.3.0 requires an exact independent grant: provider/profile {}, {}, one model {model}, one target {target}, one request, {V93_ASYMMETRIC_MAX_COST_TICKS} cost ticks, lineage {lineage_root}, and the reviewed Plan recipe/input hashes",
                request.provider_id, request.profile_id
            ),
        ));
    }
    Ok(())
}

fn stage_and_attach_authorization(
    store: &JobStore,
    plan: &AutomationPlan,
    authorization: Option<&str>,
) -> Result<JobRecord, (String, String)> {
    let record = stage_plan_job(store, plan).map_err(display_error)?;
    let Some(authorization) = authorization else {
        return Ok(record);
    };
    match attach_authorization(store, &record, plan, authorization) {
        Ok(attached) => Ok(attached),
        Err(error) => {
            let _ = store.update_record(&record.job_id, |record| {
                record.authorization_id = None;
                record.authorization_manifest_sha256 = None;
                record.state = forge_core::job::JobState::Failed;
                record.lifecycle_state = JobLifecycleState::Failed;
                record.error_code = Some(error.0.clone());
                record.error_summary = Some(error.1.clone());
                record.next_actions = vec!["prepare_new_plan".into()];
            });
            Err(error)
        }
    }
}

fn prepare_or_plan(
    mut operation: AutomationOperation,
    wait: bool,
    plan_only: bool,
    authorization: Option<&str>,
) -> Result<(), (String, String)> {
    resolve_operation_models(&mut operation)?;
    if plan_only {
        let plan = plan_store()?.prepare(operation).map_err(display_error)?;
        return success(&plan);
    }
    prepare_and_execute(operation, wait, authorization)
}

fn resolve_operation_models(operation: &mut AutomationOperation) -> Result<(), (String, String)> {
    match operation {
        AutomationOperation::GenerateCharacterPack(request) => {
            request.generation.image_model = Some(
                resolve_provider_image_model(
                    &request.provider_id,
                    request.generation.image_model.as_deref(),
                )
                .map_err(display_error)?,
            );
            if matches!(
                request.workflow.id.as_str(),
                "topdown-spritesheet" | "topdown-frames" | "topdown-grid"
            ) {
                request.generation.video_model = None;
            } else {
                request.generation.video_model = Some(
                    resolve_provider_video_model(
                        &request.provider_id,
                        request.generation.video_model.as_deref(),
                    )
                    .map_err(display_error)?,
                );
            }
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            request.image_model = Some(
                resolve_provider_image_model(&request.provider_id, request.image_model.as_deref())
                    .map_err(display_error)?,
            );
        }
        _ => {}
    }
    Ok(())
}

fn generate_character(input: CharacterGenerateInput) -> Result<(), (String, String)> {
    let project = read_project(&input.project).map_err(display_error)?;
    let revision = project.current_style_revision.ok_or_else(|| {
        (
            "style_missing".into(),
            "run `forge style create` before generation".into(),
        )
    })?;
    let style_lock_path = input
        .project
        .join(".forge/styles")
        .join(&revision)
        .join(STYLE_LOCK_FILE);
    let spec_bytes = fs::read(&input.spec).map_err(io_error)?;
    let value: serde_json::Value = serde_json::from_slice(&spec_bytes).map_err(json_error)?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if schema_version == "2" && !cfg!(feature = "consistency-v2") {
        return Err((
            "feature_not_available".into(),
            "Character V2 requires a Forge v0.3 build with consistency-v2 enabled".into(),
        ));
    }
    if input.validation_animation.is_some() && schema_version != "2" {
        return Err((
            "invalid_validation_scope".into(),
            "--validation-animation requires a Character V2 spec".into(),
        ));
    }
    let mut request: GenerateCharacterPackRequest = if schema_version == "1" {
        let mut spec: CharacterAssetSpecV1 = serde_json::from_value(value).map_err(json_error)?;
        if spec.kind != "character" {
            return Err((
                "invalid_asset_spec".into(),
                "character spec kind must be character".into(),
            ));
        }
        if let Some(reference) = &spec.reference_image {
            spec.reference_image = Some(resolve_relative(
                input.spec.parent().unwrap_or(Path::new(".")),
                reference,
            ));
        }
        serde_json::from_value(serde_json::json!({
            "schemaVersion": "3",
            "providerId": project.provider.id,
            "profileId": project.provider.profile_id,
            "projectPath": input.project,
            "assetId": spec.id,
            "styleLockPath": style_lock_path,
            "character": {
                "prompt": spec.prompt,
                "referenceImagePath": spec.reference_image,
            },
            "cameraProfile": "topdown-3q-orthographic@1.0.0",
            "metadata": {
                "name": spec.name,
                "defaultAnimation": "idle",
                "creator": "Game Sprite Forge",
                "license": spec.license,
            },
            "workflow": { "id": "topdown", "version": "1.0.0" },
            "generation": { "maxAttemptsPerAnimation": 2, "targetFrameCount": 8, "videoDurationSeconds": 4 },
            "quality": { "requireGameReady": true }
        }))
        .map_err(json_error)?
    } else if schema_version == "2" {
        let mut spec: CharacterAssetSpecV2 = serde_json::from_value(value).map_err(json_error)?;
        let valid_keyframe = spec.workflow.id == "topdown-keyframes"
            && matches!(
                spec.workflow.version.as_str(),
                "2.0.0" | "2.1.0" | "2.2.0" | "2.3.0"
            );
        let valid_keypose = spec.workflow.id == "topdown-keyposes"
            && matches!(spec.workflow.version.as_str(), "2.4.0" | "2.5.0");
        let valid_sprite_sheet =
            spec.workflow.id == "topdown-spritesheet" && spec.workflow.version == "3.0.0";
        let valid_locked_frames =
            spec.workflow.id == "topdown-frames" && spec.workflow.version == "4.0.0";
        let valid_video = spec.workflow.id == "topdown-video" && spec.workflow.version == "2.0.0";
        let valid_locked_video =
            spec.workflow.id == "topdown-video-locked" && spec.workflow.version == "5.0.0";
        let valid_video_cycle =
            spec.workflow.id == "topdown-video-cycle" && spec.workflow.version == "6.0.0";
        let valid_direction_poses =
            spec.workflow.id == "topdown-direction-poses" && spec.workflow.version == "7.0.0";
        let valid_direction_motion =
            spec.workflow.id == "topdown-direction-motion" && spec.workflow.version == "8.0.0";
        let cycle_requested =
            spec.workflow.id == "topdown-cycle" && spec.workflow.version == "10.0.0";
        let grid_requested = spec.workflow.id == "topdown-grid"
            && matches!(
                spec.workflow.version.as_str(),
                "9.0.0" | "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
            );
        let grid_keyframes = spec.workflow.id == "topdown-grid"
            && matches!(
                spec.workflow.version.as_str(),
                "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
            );
        let grid_structured_gait = spec.workflow.id == "topdown-grid"
            && matches!(
                spec.workflow.version.as_str(),
                "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
            );
        let valid_grid = grid_requested && cfg!(feature = "grid-generation");
        let valid_cycle = cycle_requested && cfg!(feature = "grid-generation");
        if (grid_requested || cycle_requested) && !cfg!(feature = "grid-generation") {
            return Err((
                "feature_not_available".into(),
                "topdown-grid@9.x and topdown-cycle@10.0.0 require a Forge build with grid-generation enabled"
                    .into(),
            ));
        }
        if spec.kind != "character"
            || (!valid_video
                && !valid_locked_video
                && !valid_video_cycle
                && !valid_direction_poses
                && !valid_direction_motion
                && !valid_grid
                && !valid_cycle
                && !valid_keyframe
                && !valid_keypose
                && !valid_sprite_sheet
                && !valid_locked_frames)
        {
            return Err((
                "invalid_asset_spec".into(),
                "Character V2 requires topdown-video@2.0.0, topdown-video-locked@5.0.0, topdown-video-cycle@6.0.0, topdown-direction-poses@7.0.0, topdown-direction-motion@8.0.0, topdown-grid@9.0.0 through @9.5.0, topdown-cycle@10.0.0, topdown-keyframes@2.0.0 through @2.3.0, topdown-keyposes@2.4.0 through @2.5.0, topdown-spritesheet@3.0.0, or topdown-frames@4.0.0".into(),
            ));
        }
        if (valid_video
            || valid_locked_video
            || valid_video_cycle
            || valid_direction_poses
            || valid_direction_motion
            || valid_grid
            || valid_cycle)
            && spec.camera_profile.is_none()
        {
            return Err((
                "character_camera_profile_required".into(),
                "topdown video, direction, and grid workflows require cameraProfile topdown-orthographic@2.0.0 or topdown-three-quarter@1.0.0".into(),
            ));
        }
        let camera_profile = spec.camera_profile.unwrap_or_default();
        let mut equipment = match spec.equipment.take() {
            Some(equipment) => equipment,
            None if valid_video
                || valid_locked_video
                || valid_video_cycle
                || valid_direction_poses
                || valid_cycle
                || matches!(
                    spec.workflow.version.as_str(),
                    "2.2.0"
                        | "2.3.0"
                        | "2.4.0"
                        | "2.5.0"
                        | "3.0.0"
                        | "4.0.0"
                        | "9.0.0"
                        | "9.1.0"
                        | "9.2.0"
                        | "9.3.0"
                        | "9.4.0"
                        | "9.5.0"
                ) =>
            {
                return Err((
                    "character_equipment_required".into(),
                    "topdown-video@2.0.0, topdown-video-locked@5.0.0, topdown-video-cycle@6.0.0, topdown-direction-poses@7.0.0, topdown-direction-motion@8.0.0, topdown-keyframes@2.2.0/@2.3.0, topdown-keyposes@2.4.0/@2.5.0, topdown-spritesheet@3.0.0, and topdown-frames@4.0.0 require an explicit equipment.kind of none or staff_like"
                        .into(),
                ));
            }
            None => Default::default(),
        };
        if let Some(reference) = &equipment.reference_image {
            equipment.reference_image = Some(resolve_relative(
                input.spec.parent().unwrap_or(Path::new(".")),
                reference,
            ));
        }
        let subject = spec
            .subject
            .as_ref()
            .map(|subject| {
                let path = subject_lock_path(&input.project, &subject.id, &subject.revision);
                let lock = read_subject_lock(&path).map_err(display_error)?;
                if lock.style_revision != revision {
                    return Err((
                        "subject_style_mismatch".into(),
                        format!(
                            "SubjectLock uses style revision {}, but the project locks {}",
                            lock.style_revision, revision
                        ),
                    ));
                }
                Ok((path, lock))
            })
            .transpose()?;
        if !valid_direction_motion && subject.is_none() {
            return Err((
                "subject_lock_required".into(),
                "this Character workflow requires subject.id and subject.revision".into(),
            ));
        }
        let character_prompt = spec
            .prompt
            .clone()
            .or_else(|| subject.as_ref().map(|(_, lock)| lock.prompt.clone()))
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| {
                (
                    "character_prompt_required".into(),
                    "V8 requires prompt when no SubjectLock is provided".into(),
                )
            })?;
        let subject_path = subject.as_ref().map(|(path, _)| path.clone());
        let subject_canonical = subject
            .as_ref()
            .map(|(_, lock)| lock.canonical_path.clone());
        serde_json::from_value(serde_json::json!({
            "schemaVersion": "3",
            "providerId": project.provider.id,
            "profileId": project.provider.profile_id,
            "projectPath": input.project,
            "assetId": spec.id,
            "styleLockPath": style_lock_path,
            "subjectLockPath": subject_path,
            "character": {
                "prompt": character_prompt,
                "referenceImagePath": if valid_direction_motion { None::<PathBuf> } else { subject_canonical },
            },
            "cameraProfile": camera_profile,
            "equipment": equipment,
            "equipmentExplicit": true,
            "metadata": {
                "name": spec.name,
                "defaultAnimation": if valid_direction_motion || valid_grid || valid_cycle { "idle_down" } else { "idle" },
                "creator": "Game Sprite Forge",
                "license": spec.license,
            },
            "workflow": { "id": spec.workflow.id, "version": spec.workflow.version },
            "motionProfile": if valid_cycle { CharacterMotionProfileV1::BipedWalk } else { spec.motion_profile.unwrap_or_default() },
            "directionMotionStage": DirectionMotionGenerationStageV1::from(input.direction_motion_stage),
            "generation": {
                "maxAttemptsPerAnimation": if valid_cycle { 1 } else { 2 },
                "targetFrameCount": if valid_direction_motion || valid_cycle { 12 } else if valid_grid || valid_keypose || valid_sprite_sheet || valid_locked_frames { 4 } else { 8 },
                "videoDurationSeconds": 4,
                "poseGuidance": if valid_cycle || grid_keyframes { GridPoseGuidanceV1::Disabled } else if grid_structured_gait { GridPoseGuidanceV1::Grayscale } else { spec.pose_guidance },
            },
            "quality": { "requireGameReady": true }
        }))
        .map_err(json_error)?
    } else {
        return Err((
            "invalid_asset_spec".into(),
            "character spec requires schemaVersion 1 or 2".into(),
        ));
    };
    if request.workflow.id == "topdown-direction-motion" && request.workflow.version == "8.0.0" {
        match request.direction_motion_stage {
            DirectionMotionGenerationStageV1::ImageLocks => {
                if input.image_lock_job.is_some() {
                    return Err((
                        "direction_motion_source_unexpected".into(),
                        "--image-lock-job is only valid with --direction-motion-stage complete"
                            .into(),
                    ));
                }
            }
            DirectionMotionGenerationStageV1::Complete => {
                let source_job_id = input.image_lock_job.as_deref().ok_or_else(|| {
                    (
                        "direction_motion_approval_required".into(),
                        "V8 complete requires --image-lock-job <approved-job-id>; it never regenerates the eight image locks"
                            .into(),
                    )
                })?;
                let source = job_store()?
                    .read_record(source_job_id)
                    .map_err(display_error)?;
                if source.lifecycle_state != JobLifecycleState::Succeeded
                    || !source
                        .job_dir
                        .join(DIRECTION_MOTION_APPROVAL_FILE)
                        .is_file()
                {
                    return Err((
                        "direction_motion_approval_required".into(),
                        "the source image-lock Job has not been explicitly approved".into(),
                    ));
                }
                let source_operation: AutomationOperation =
                    serde_json::from_value(source.recipe.clone().ok_or_else(|| {
                        (
                            "direction_motion_source_invalid".into(),
                            "the source image-lock Job has no immutable recipe".into(),
                        )
                    })?)
                    .map_err(json_error)?;
                let AutomationOperation::GenerateCharacterPack(source_request) = source_operation
                else {
                    return Err((
                        "direction_motion_source_invalid".into(),
                        "the source Job is not a Character generation Job".into(),
                    ));
                };
                if source_request.workflow != request.workflow
                    || source_request.direction_motion_stage
                        != DirectionMotionGenerationStageV1::ImageLocks
                    || source_request.asset_id != request.asset_id
                    || source_request.provider_id != request.provider_id
                    || source_request.profile_id != request.profile_id
                {
                    return Err((
                        "direction_motion_source_mismatch".into(),
                        "the approved image lock is bound to another workflow, asset, or Provider profile"
                            .into(),
                    ));
                }
                request.reuse_from_job_dir = Some(source.job_dir);
            }
        }
    } else if request.workflow.id == "topdown-grid" && request.workflow.version == "9.0.0" {
        match request.direction_motion_stage {
            DirectionMotionGenerationStageV1::ImageLocks => {
                if input.image_lock_job.is_some() {
                    return Err((
                        "direction_grid_source_unexpected".into(),
                        "--image-lock-job is only valid with --direction-motion-stage complete"
                            .into(),
                    ));
                }
            }
            DirectionMotionGenerationStageV1::Complete => {
                let source_job_id = input.image_lock_job.as_deref().ok_or_else(|| {
                    (
                        "direction_grid_approval_required".into(),
                        "topdown-grid action_grid requires --image-lock-job <approved-job-id>"
                            .into(),
                    )
                })?;
                let source = job_store()?
                    .read_record(source_job_id)
                    .map_err(display_error)?;
                if source.lifecycle_state != JobLifecycleState::Succeeded
                    || !source.job_dir.join(GRID_APPROVAL_FILE).is_file()
                {
                    return Err((
                        "direction_grid_approval_required".into(),
                        "the source direction grid Job has not been explicitly approved".into(),
                    ));
                }
                let source_operation: AutomationOperation =
                    serde_json::from_value(source.recipe.clone().ok_or_else(|| {
                        (
                            "direction_grid_source_invalid".into(),
                            "the source direction grid Job has no immutable recipe".into(),
                        )
                    })?)
                    .map_err(json_error)?;
                let AutomationOperation::GenerateCharacterPack(source_request) = source_operation
                else {
                    return Err((
                        "direction_grid_source_invalid".into(),
                        "the source Job is not a Character generation Job".into(),
                    ));
                };
                if source_request.workflow != request.workflow
                    || source_request.direction_motion_stage
                        != DirectionMotionGenerationStageV1::ImageLocks
                    || source_request.asset_id != request.asset_id
                    || source_request.provider_id != request.provider_id
                    || source_request.profile_id != request.profile_id
                {
                    return Err((
                        "direction_grid_source_mismatch".into(),
                        "the approved direction grid is bound to another workflow, asset, or Provider profile"
                            .into(),
                    ));
                }
                request.reuse_from_job_dir = Some(source.job_dir);
            }
        }
    } else if request.workflow.id == "topdown-grid"
        && matches!(
            request.workflow.version.as_str(),
            "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
        )
    {
        let version = request.workflow.version.clone();
        if request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete {
            return Err((
                "grid_keyframe_stage_invalid".into(),
                format!("topdown-grid@{version} is complete-only; use --direction-motion-stage complete"),
            ));
        }
        let source_job_id = input.image_lock_job.as_deref().ok_or_else(|| {
            (
                "direction_grid_approval_required".into(),
                format!("topdown-grid@{version} requires --image-lock-job <approved-v9.0-job-id>"),
            )
        })?;
        let source = job_store()?
            .read_record(source_job_id)
            .map_err(display_error)?;
        if source.lifecycle_state != JobLifecycleState::Succeeded
            || !source.job_dir.join(GRID_APPROVAL_FILE).is_file()
        {
            return Err((
                "direction_grid_approval_required".into(),
                "the V9.0 Direction Grid Job has not been explicitly approved".into(),
            ));
        }
        let source_operation: AutomationOperation =
            serde_json::from_value(source.recipe.clone().ok_or_else(|| {
                (
                    "direction_grid_source_invalid".into(),
                    "the source Direction Grid Job has no immutable recipe".into(),
                )
            })?)
            .map_err(json_error)?;
        let AutomationOperation::GenerateCharacterPack(source_request) = source_operation else {
            return Err((
                "direction_grid_source_invalid".into(),
                "the source Job is not Character generation".into(),
            ));
        };
        if source_request.workflow.id != "topdown-grid"
            || source_request.workflow.version != "9.0.0"
            || source_request.direction_motion_stage != DirectionMotionGenerationStageV1::ImageLocks
            || source_request.asset_id != request.asset_id
            || source_request.provider_id != request.provider_id
            || source_request.profile_id != request.profile_id
        {
            return Err((
                "direction_grid_source_mismatch".into(),
                "V9.1/V9.2/V9.3 requires an approved V9.0 Direction Grid for the same asset and Provider profile"
                    .into(),
            ));
        }
        request.reuse_from_job_dir = Some(source.job_dir);
    } else if request.workflow.id == "topdown-cycle" && request.workflow.version == "10.0.0" {
        if request.direction_motion_stage != DirectionMotionGenerationStageV1::Complete {
            return Err((
                "topdown_cycle_stage_invalid".into(),
                "topdown-cycle@10.0.0 is complete-only; use --direction-motion-stage complete"
                    .into(),
            ));
        }
        let source_job_id = input.image_lock_job.as_deref().ok_or_else(|| {
            (
                "direction_grid_approval_required".into(),
                "topdown-cycle@10.0.0 requires --image-lock-job <approved-v9.0-job-id>".into(),
            )
        })?;
        let source = job_store()?
            .read_record(source_job_id)
            .map_err(display_error)?;
        if source.lifecycle_state != JobLifecycleState::Succeeded
            || !source.job_dir.join(GRID_APPROVAL_FILE).is_file()
        {
            return Err((
                "direction_grid_approval_required".into(),
                "the V9.0 Direction Grid Job has not been explicitly approved".into(),
            ));
        }
        let source_operation: AutomationOperation =
            serde_json::from_value(source.recipe.clone().ok_or_else(|| {
                (
                    "direction_grid_source_invalid".into(),
                    "the source Direction Grid Job has no immutable recipe".into(),
                )
            })?)
            .map_err(json_error)?;
        let AutomationOperation::GenerateCharacterPack(source_request) = source_operation else {
            return Err((
                "direction_grid_source_invalid".into(),
                "the source Job is not Character generation".into(),
            ));
        };
        if source_request.workflow.id != "topdown-grid"
            || source_request.workflow.version != "9.0.0"
            || source_request.direction_motion_stage != DirectionMotionGenerationStageV1::ImageLocks
            || source_request.asset_id != request.asset_id
        {
            return Err((
                "direction_grid_source_mismatch".into(),
                "V10 requires an approved V9.0 Direction Grid for the same asset".into(),
            ));
        }
        request.reuse_from_job_dir = Some(source.job_dir);
    } else if input.image_lock_job.is_some() {
        return Err((
            "direction_motion_source_unexpected".into(),
            "--image-lock-job is only supported by topdown-direction-motion@8.0.0, topdown-grid@9.x, and topdown-cycle@10.0.0".into(),
        ));
    }
    if let Some(animation) = input.validation_animation {
        if request.workflow.id != "topdown-video"
            && request.workflow.id != "topdown-video-locked"
            && request.workflow.id != "topdown-video-cycle"
            && request.workflow.id != "topdown-direction-poses"
            && request.workflow.id != "topdown-direction-motion"
            && request.workflow.id != "topdown-grid"
            && request.workflow.id != "topdown-cycle"
            && !matches!(
                request.workflow.version.as_str(),
                "2.2.0" | "2.3.0" | "2.4.0" | "2.5.0" | "3.0.0" | "4.0.0"
            )
        {
            return Err((
                "invalid_validation_scope".into(),
                "--validation-animation requires topdown-video@2.0.0, topdown-video-locked@5.0.0, topdown-video-cycle@6.0.0, topdown-direction-poses@7.0.0, topdown-direction-motion@8.0.0, topdown-grid@9.0.0, topdown-keyframes@2.2.0/@2.3.0, topdown-keyposes@2.4.0/@2.5.0, topdown-spritesheet@3.0.0, or topdown-frames@4.0.0".into(),
            ));
        }
        request.validation_only = true;
        request.validation_animations = vec![animation.clone()];
        request.metadata.default_animation = animation;
    }
    prepare_or_plan(
        AutomationOperation::GenerateCharacterPack(request),
        input.wait,
        input.plan_only,
        input.authorization.as_deref(),
    )
}

fn generate_static(input: ProjectSpecInput, expected_kind: &str) -> Result<(), (String, String)> {
    generate_static_with_portrait_options(input, expected_kind, None)
}

#[cfg(feature = "collection-assets")]
fn generate_portrait(input: PortraitSetInput) -> Result<(), (String, String)> {
    let options = PortraitGenerationOptions {
        phase: match input.phase {
            PortraitPhaseArg::Base => PortraitGenerationPhaseV1::BaseOnly,
            PortraitPhaseArg::Expressions => PortraitGenerationPhaseV1::Expressions,
        },
        neutral_reference_policy: input.neutral_reference_policy.into(),
        base_job_id: input.base_job,
    };
    generate_static_with_portrait_options(
        ProjectSpecInput {
            project: input.project,
            spec: input.spec,
            wait: input.wait,
            plan_only: input.plan_only,
            authorization: input.authorization,
            json: input.json,
        },
        "portrait_set",
        Some(options),
    )
}

struct PortraitGenerationOptions {
    phase: PortraitGenerationPhaseV1,
    neutral_reference_policy: PortraitNeutralReferencePolicyV1,
    base_job_id: Option<String>,
}

fn generate_static_with_portrait_options(
    input: ProjectSpecInput,
    expected_kind: &str,
    portrait_options: Option<PortraitGenerationOptions>,
) -> Result<(), (String, String)> {
    let project = read_project(&input.project).map_err(display_error)?;
    let revision = project.current_style_revision.ok_or_else(|| {
        (
            "style_missing".into(),
            "run `forge style create` before generation".into(),
        )
    })?;
    let style_lock_path = input
        .project
        .join(".forge/styles")
        .join(&revision)
        .join(STYLE_LOCK_FILE);
    let bytes = fs::read(&input.spec).map_err(io_error)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(json_error)?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let expected = match expected_kind {
        "icon_set" => forge_core::asset_project::StaticAssetKind::IconSet,
        "prop_set" => forge_core::asset_project::StaticAssetKind::PropSet,
        "portrait_set" => forge_core::asset_project::StaticAssetKind::PortraitSet,
        "equipment_set" => forge_core::asset_project::StaticAssetKind::EquipmentSet,
        "decal_set" => forge_core::asset_project::StaticAssetKind::DecalSet,
        _ => {
            return Err((
                "invalid_asset_spec".into(),
                format!("unsupported static asset kind: {expected_kind}"),
            ))
        }
    };
    let source_spec_path = input.spec.clone();
    let (mut spec, collection_lock_path, subject_lock_path, framing_profile, item_metadata) =
        if schema_version == "1"
            && matches!(
                expected,
                forge_core::asset_project::StaticAssetKind::IconSet
                    | forge_core::asset_project::StaticAssetKind::PropSet
            )
        {
            let spec: StaticAssetSetSpecV1 = serde_json::from_value(value).map_err(json_error)?;
            (spec, None, None, None, Default::default())
        } else {
            #[cfg(feature = "collection-assets")]
            {
                let resolved =
                    read_static_collection_spec(&input.spec, expected).map_err(display_error)?;
                let lock_path = collection_lock_path(
                    &input.project,
                    &resolved.collection.id,
                    &resolved.collection.revision,
                );
                let lock = read_collection_lock(&lock_path).map_err(display_error)?;
                if lock.asset_kind != expected || lock.style_revision != revision {
                    return Err((
                        "collection_lock_mismatch".into(),
                        "CollectionLock kind and Style revision must match the asset spec".into(),
                    ));
                }
                let subject_path = resolved.subject.as_ref().map(|subject| {
                    subject_lock_path(&input.project, &subject.id, &subject.revision)
                });
                if let Some(path) = &subject_path {
                    let subject = read_subject_lock(path).map_err(display_error)?;
                    if subject.style_revision != revision {
                        return Err((
                            "subject_style_mismatch".into(),
                            "Portrait SubjectLock and project Style revision differ".into(),
                        ));
                    }
                }
                (
                    resolved.asset,
                    Some(lock_path),
                    subject_path,
                    resolved.framing_profile,
                    resolved.item_metadata,
                )
            }
            #[cfg(not(feature = "collection-assets"))]
            {
                return Err((
                    "feature_not_available".into(),
                    "this spec requires a Forge build with collection-assets enabled".into(),
                ));
            }
        };
    if spec.kind != expected {
        return Err((
            "invalid_asset_spec".into(),
            format!("asset spec kind must be {expected_kind}"),
        ));
    }
    let root = input.spec.parent().unwrap_or(Path::new("."));
    for item in &mut spec.items {
        if let Some(reference) = &item.reference_image {
            if !reference.is_absolute() {
                item.reference_image = Some(resolve_relative(root, reference));
            }
        }
    }
    let portrait_v2 = spec.kind == StaticAssetKind::PortraitSet && spec.schema_version == "2";
    let (
        portrait_phase,
        neutral_reference_policy,
        portrait_base_parent_job_id,
        reuse_from_job_dir,
        retry_item_ids,
    ) = if portrait_v2 {
        let options = portrait_options.ok_or_else(|| {
            (
                "portrait_phase_required".into(),
                "Portrait V2 must be generated through `forge generate portrait-set`".into(),
            )
        })?;
        match options.phase {
            PortraitGenerationPhaseV1::BaseOnly => {
                if options.base_job_id.is_some() {
                    return Err((
                        "portrait_base_job_unexpected".into(),
                        "--base-job is only valid with --phase expressions".into(),
                    ));
                }
                (
                    PortraitGenerationPhaseV1::BaseOnly,
                    options.neutral_reference_policy,
                    None,
                    None,
                    vec![],
                )
            }
            PortraitGenerationPhaseV1::Expressions => {
                let base_job_id = options.base_job_id.ok_or_else(|| {
                    (
                        "portrait_base_job_required".into(),
                        "--phase expressions requires --base-job <approved-job-id>".into(),
                    )
                })?;
                let source = job_store()?
                    .read_record(&base_job_id)
                    .map_err(display_error)?;
                let (approved_lock, _approval) =
                    validate_portrait_base_approval(&source.job_dir, &base_job_id)
                        .map_err(display_error)?;
                let retry_item_ids = spec
                    .items
                    .iter()
                    .filter(|item| item.id != "neutral")
                    .map(|item| item.id.clone())
                    .collect();
                (
                    PortraitGenerationPhaseV1::Expressions,
                    approved_lock.neutral_reference_policy,
                    Some(base_job_id),
                    Some(source.job_dir),
                    retry_item_ids,
                )
            }
            PortraitGenerationPhaseV1::LegacyAll => unreachable!(),
        }
    } else {
        if portrait_options
            .as_ref()
            .and_then(|options| options.base_job_id.as_ref())
            .is_some()
        {
            return Err((
                "portrait_v2_required".into(),
                "--base-job requires a Portrait V2 spec".into(),
            ));
        }
        (
            PortraitGenerationPhaseV1::LegacyAll,
            PortraitNeutralReferencePolicyV1::LegacyThreeReference,
            None,
            None,
            vec![],
        )
    };
    let request = GenerateStaticAssetSetRequest {
        schema_version: if portrait_v2 { "5" } else { "4" }.into(),
        project_path: input.project,
        style_lock_path,
        collection_lock_path,
        subject_lock_path,
        source_spec_path: Some(source_spec_path),
        provider_id: project.provider.id,
        profile_id: project.provider.profile_id,
        asset: spec,
        framing_profile,
        item_metadata,
        max_attempts_per_item: 2,
        image_model: None,
        reuse_from_job_dir,
        retry_item_ids,
        replacement_item_paths: Default::default(),
        consistency_recheck_only: false,
        resume_incomplete_static: false,
        portrait_phase,
        neutral_reference_policy,
        portrait_base_parent_job_id,
    };
    prepare_or_plan(
        AutomationOperation::GenerateStaticAssetSet(request),
        input.wait,
        input.plan_only,
        input.authorization.as_deref(),
    )
}

#[cfg(feature = "collection-assets")]
fn replace_static_item(
    source_job_id: &str,
    item_id: &str,
    replacement: &Path,
    wait: bool,
) -> Result<(), (String, String)> {
    let source = job_store()?
        .read_record(source_job_id)
        .map_err(display_error)?;
    if source.worker_pid.is_some()
        || matches!(
            source.lifecycle_state,
            forge_core::job::JobLifecycleState::Queued
                | forge_core::job::JobLifecycleState::Running
        )
    {
        return Err((
            "source_job_active".into(),
            "replace-item requires a terminal source Job".into(),
        ));
    }
    let recipe = source.recipe.clone().ok_or_else(|| {
        (
            "recipe_missing".into(),
            "source Job has no immutable recipe".into(),
        )
    })?;
    let mut operation: AutomationOperation = serde_json::from_value(recipe).map_err(json_error)?;
    let AutomationOperation::GenerateStaticAssetSet(request) = &mut operation else {
        return Err((
            "unsupported_replace".into(),
            "replace-item is available only for static set Jobs".into(),
        ));
    };
    if !request.asset.items.iter().any(|item| item.id == item_id) {
        return Err((
            "item_not_found".into(),
            format!("static item not found: {item_id}"),
        ));
    }
    let replacement = replacement.canonicalize().map_err(io_error)?;
    request.reuse_from_job_dir = Some(source.job_dir.clone());
    request.retry_item_ids = vec![item_id.into()];
    request.replacement_item_paths = [(item_id.into(), replacement)].into_iter().collect();
    request.consistency_recheck_only = true;
    prepare_and_execute(operation, wait, None)
}

#[cfg(feature = "collection-assets")]
fn export_editable_asset(
    project_root: &Path,
    asset_id: &str,
    output: &Path,
) -> Result<(), (String, String)> {
    if output.exists() {
        return Err((
            "output_exists".into(),
            format!(
                "editable export target already exists: {}",
                output.display()
            ),
        ));
    }
    let catalog = read_project_catalog(project_root).map_err(display_error)?;
    let entry = catalog.assets.get(asset_id).ok_or_else(|| {
        (
            "asset_not_found".into(),
            format!("catalog asset not found: {asset_id}"),
        )
    })?;
    let pack_path = if entry.pack_path.is_absolute() {
        entry.pack_path.clone()
    } else {
        project_root.join(&entry.pack_path)
    };
    forge_pack::validate_pack_layout(&pack_path).map_err(display_error)?;
    let inspected = forge_pack::inspect_pack(&pack_path).map_err(display_error)?;
    if inspected.items.is_empty() {
        return Err((
            "asset_not_editable".into(),
            "editable export currently supports static item Packs".into(),
        ));
    }
    let partial = output.with_extension(format!("forge-partial-{}", std::process::id()));
    if partial.exists() {
        return Err((
            "partial_output_exists".into(),
            format!("stale partial export exists: {}", partial.display()),
        ));
    }
    let items_root = partial.join("items");
    fs::create_dir_all(&items_root).map_err(io_error)?;
    let mut item_entries = Vec::with_capacity(inspected.items.len());
    for item in &inspected.items {
        let source = pack_path.join(&item.texture);
        let target = items_root.join(format!("{}.png", item.id));
        fs::copy(&source, &target).map_err(io_error)?;
        item_entries.push(serde_json::json!({
            "id": item.id,
            "name": item.name,
            "path": format!("items/{}.png", item.id),
            "sha256": hash_asset_file(&target).map_err(display_error)?,
        }));
    }
    for name in [
        "consistency-report.json",
        "collection-consistency-report.json",
        "collection-lock-ref.json",
    ] {
        let source = pack_path.join(name);
        if source.is_file() {
            fs::copy(source, partial.join(name)).map_err(io_error)?;
        }
    }
    let source = job_store()?
        .read_record(&entry.source_job_id)
        .map_err(display_error)?;
    let operation: AutomationOperation = source
        .recipe
        .clone()
        .map(serde_json::from_value)
        .transpose()
        .map_err(json_error)?
        .ok_or_else(|| ("recipe_missing".into(), "source Job has no recipe".into()))?;
    let source_spec = if let AutomationOperation::GenerateStaticAssetSet(request) = operation {
        if let Some(path) = request.source_spec_path.filter(|path| path.is_file()) {
            fs::copy(&path, partial.join("source-spec.json")).map_err(io_error)?;
            Some(serde_json::json!({
                "path": "source-spec.json",
                "sha256": hash_asset_file(&path).map_err(display_error)?,
            }))
        } else {
            None
        }
    } else {
        None
    };
    let forgepack: serde_json::Value =
        serde_json::from_slice(&fs::read(pack_path.join("forgepack.json")).map_err(io_error)?)
            .map_err(json_error)?;
    let editable_manifest = serde_json::json!({
        "schemaVersion": "1",
        "kind": "forge_editable_static_asset",
        "assetId": entry.asset_id,
        "assetType": inspected.asset_type,
        "sourceJobId": entry.source_job_id,
        "sourcePackSha256": entry.pack_sha256,
        "license": forgepack.get("license"),
        "styleRevision": entry.style.as_ref().map(|style| style.revision.as_str()),
        "locks": entry.locks,
        "sourceSpec": source_spec,
        "items": item_entries,
        "instructions": "Edit only items/*.png without changing canvas size or transparency; import with forge asset replace-item.",
    });
    fs::write(
        partial.join("editable-manifest.json"),
        serde_json::to_vec_pretty(&editable_manifest).map_err(json_error)?,
    )
    .map_err(io_error)?;
    fs::rename(&partial, output).map_err(io_error)?;
    success(&serde_json::json!({
        "assetId": asset_id,
        "output": output,
        "itemCount": inspected.items.len(),
        "sourcePackSha256": entry.pack_sha256,
    }))
}

#[cfg(feature = "terrain-assets")]
fn environment_lock_for_project(
    project_path: &Path,
) -> Result<(forge_core::asset_project::ForgeProjectV1, PathBuf), (String, String)> {
    let project = read_project(project_path).map_err(display_error)?;
    let revision = project
        .current_environment_revision
        .clone()
        .ok_or_else(|| {
            (
                "environment_missing".into(),
                "run `forge environment create` before world generation".into(),
            )
        })?;
    let lock_path = project_path
        .join(".forge/environments")
        .join(revision)
        .join(ENVIRONMENT_LOCK_FILE);
    Ok((project, lock_path))
}

#[cfg(feature = "terrain-assets")]
fn generate_terrain(input: ProjectSpecInput) -> Result<(), (String, String)> {
    let (project, environment_lock_path) = environment_lock_for_project(&input.project)?;
    let mut spec: TerrainSetSpecV1 =
        serde_json::from_slice(&fs::read(&input.spec).map_err(io_error)?).map_err(json_error)?;
    let environment = read_environment_lock(&environment_lock_path).map_err(display_error)?;
    if spec.environment_revision.is_none() {
        spec.environment_revision = Some(environment.revision);
    }
    let request = GenerateTerrainSetRequest {
        schema_version: "1".into(),
        project_path: input.project,
        environment_lock_path,
        provider_id: project.provider.id,
        profile_id: project.provider.profile_id,
        asset: spec,
    };
    prepare_and_execute(
        AutomationOperation::GenerateTerrainSet(request),
        input.wait,
        input.authorization.as_deref(),
    )
}

#[cfg(feature = "building-assets")]
fn generate_building(input: ProjectSpecInput) -> Result<(), (String, String)> {
    let (project, environment_lock_path) = environment_lock_for_project(&input.project)?;
    let mut spec: BuildingKitSpecV1 =
        serde_json::from_slice(&fs::read(&input.spec).map_err(io_error)?).map_err(json_error)?;
    let environment = read_environment_lock(&environment_lock_path).map_err(display_error)?;
    if spec.environment_revision.is_none() {
        spec.environment_revision = Some(environment.revision);
    }
    let request = GenerateBuildingKitRequest {
        schema_version: "1".into(),
        project_path: input.project,
        environment_lock_path,
        provider_id: project.provider.id,
        profile_id: project.provider.profile_id,
        asset: spec,
    };
    prepare_and_execute(
        AutomationOperation::GenerateBuildingKit(request),
        input.wait,
        input.authorization.as_deref(),
    )
}

struct RetryJobOptions<'a> {
    stage: CharacterRetryStageArg,
    asymmetric_gait: bool,
    front_authoritative_grid: bool,
    image_model: Option<&'a str>,
    wait: bool,
    plan_only: bool,
    authorization: Option<&'a str>,
}

#[allow(clippy::too_many_arguments)]
fn import_direction_grid_job(
    source_id: &str,
    sheet: Option<&Path>,
    directions: [Option<&Path>; 4],
    generator: &str,
    cape_hem_contract: Option<CapeHemContractV1>,
    note: &str,
    wait: bool,
    plan_only: bool,
) -> Result<(), (String, String)> {
    if generator.trim().is_empty() || note.trim().is_empty() {
        return Err((
            "invalid_direction_grid_import".into(),
            "--generator and --note must be non-empty".into(),
        ));
    }
    let source = job_store()?.read_record(source_id).map_err(display_error)?;
    if source.lifecycle_state != JobLifecycleState::Succeeded {
        return Err((
            "direction_grid_import_source_invalid".into(),
            "external DirectionGrid import requires a succeeded approved V9 source Job".into(),
        ));
    }
    let sheet_path = sheet
        .map(Path::canonicalize)
        .transpose()
        .map_err(io_error)?;
    let direction_paths = if directions.iter().all(Option::is_some) {
        Some(ExternalDirectionGridFilesV1 {
            front: directions[0]
                .expect("all direction paths checked")
                .canonicalize()
                .map_err(io_error)?,
            rear: directions[1]
                .expect("all direction paths checked")
                .canonicalize()
                .map_err(io_error)?,
            right: directions[2]
                .expect("all direction paths checked")
                .canonicalize()
                .map_err(io_error)?,
            left: directions[3]
                .expect("all direction paths checked")
                .canonicalize()
                .map_err(io_error)?,
        })
    } else {
        None
    };
    let request = ImportDirectionGridRequestV1 {
        schema_version: "1".into(),
        source_job_dir: source.job_dir,
        sheet_path,
        direction_paths,
        cape_hem_contract,
        generator: generator.into(),
        note: note.into(),
    };
    prepare_or_plan(
        AutomationOperation::ImportDirectionGrid(request),
        wait,
        plan_only,
        None,
    )
}

fn retry_job(
    id: &str,
    item: Option<&str>,
    frame: Option<u8>,
    options: RetryJobOptions<'_>,
) -> Result<(), (String, String)> {
    let RetryJobOptions {
        stage,
        asymmetric_gait,
        front_authoritative_grid,
        image_model,
        wait,
        plan_only,
        authorization,
    } = options;
    let source = job_store()?.read_record(id).map_err(display_error)?;
    if asymmetric_gait {
        let authorization = authorization.ok_or_else(|| {
            (
                "asymmetric_gait_authorization_required".into(),
                "--asymmetric-gait requires a new explicit --authorization".into(),
            )
        })?;
        if source.authorization_id.as_deref() == Some(authorization) {
            return Err((
                "asymmetric_gait_authorization_not_independent".into(),
                "--asymmetric-gait may not reuse the source Job authorization".into(),
            ));
        }
        if stage != CharacterRetryStageArg::Frame {
            return Err((
                "invalid_asymmetric_gait_remediation".into(),
                "--asymmetric-gait requires explicit --stage frame".into(),
            ));
        }
    }
    let recipe = source.recipe.clone().ok_or_else(|| {
        (
            "recipe_missing".into(),
            "job has no immutable recipe".into(),
        )
    })?;
    let mut operation: AutomationOperation = serde_json::from_value(recipe).map_err(json_error)?;
    if front_authoritative_grid {
        let base_request = match &operation {
            AutomationOperation::GenerateCharacterPack(request) => Some(request.clone()),
            AutomationOperation::ImportDirectionGrid(import) => {
                let authority: JobRecord = serde_json::from_slice(
                    &fs::read(import.source_job_dir.join("job.json")).map_err(io_error)?,
                )
                .map_err(json_error)?;
                authority
                    .recipe
                    .as_ref()
                    .and_then(|recipe| serde_json::from_value(recipe.clone()).ok())
                    .and_then(|operation| match operation {
                        AutomationOperation::GenerateCharacterPack(request) => Some(request),
                        _ => None,
                    })
            }
            _ => None,
        };
        let exact_source = base_request.as_ref().is_some_and(|request| {
            request.workflow.id == "topdown-grid"
                && request.workflow.version == "9.0.0"
                && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
                && source.lifecycle_state == JobLifecycleState::Succeeded
                && source.job_dir.join(GRID_APPROVAL_FILE).is_file()
                && item == Some("direction_grid")
                && frame.is_none()
                && stage == CharacterRetryStageArg::Still
        });
        if !exact_source
            || !plan_only
            || wait
            || authorization.is_some()
            || asymmetric_gait
            || image_model.is_some()
        {
            return Err((
                "front_authoritative_direction_grid_scope_invalid".into(),
                "--front-authoritative-grid requires a succeeded approved topdown-grid@9.0.0 ImageLocks source, --item direction_grid --stage still --plan-only, and no frame/wait/authorization/asymmetric/model override"
                    .into(),
            ));
        }
        operation = AutomationOperation::GenerateCharacterPack(
            base_request.expect("front-authoritative source shape checked"),
        );
        let AutomationOperation::GenerateCharacterPack(request) = &mut operation else {
            unreachable!()
        };
        request.reuse_from_job_dir = Some(source.job_dir.clone());
        request.retry_animations = vec!["direction_grid".into()];
        request.retry_stages = [("direction_grid".into(), CharacterRetryStage::Still)].into();
        request.retry_frames.clear();
        request.validation_only = false;
        request.validation_animations.clear();
        request.generation.max_attempts_per_animation = 1;
        request.direction_grid_cape_contract =
            Some(CapeHemContractV1::FrontAuthoritativeNoSkirtHem);
        request.metadata.name = format!(
            "{} Front-Authoritative DirectionGrid",
            request.metadata.name
        );
        return prepare_or_plan(operation, false, true, None);
    }
    let image2_candidate = image_model == Some(XAI_IMAGE_2_CANDIDATE_MODEL);
    if image_model.is_some() && !image2_candidate {
        return Err((
            "unsupported_retry_model_override".into(),
            format!("retry model overrides currently accept only {XAI_IMAGE_2_CANDIDATE_MODEL}"),
        ));
    }
    if image2_candidate {
        let exact_source = matches!(
            &operation,
            AutomationOperation::GenerateCharacterPack(request)
                if request.provider_id == "xai"
                    && request.profile_id == "default"
                    && request.workflow.id == "topdown-grid"
                    && request.workflow.version == "9.0.0"
                    && request.direction_motion_stage
                        == DirectionMotionGenerationStageV1::ImageLocks
                    && source.lifecycle_state == JobLifecycleState::Succeeded
                    && source.job_dir.join(GRID_APPROVAL_FILE).is_file()
                    && item == Some("direction_grid")
                    && frame.is_none()
                    && stage == CharacterRetryStageArg::Still
        );
        if !exact_source || !plan_only || authorization.is_some() || wait {
            return Err((
                "xai_image2_candidate_scope_forbidden".into(),
                format!(
                    "{XAI_IMAGE_2_CANDIDATE_MODEL} requires an approved xai/default topdown-grid@9.0.0 source, --item direction_grid --stage still --plan-only, no --wait, and no authorization until the reviewed Plan is executed"
                ),
            ));
        }
    }
    if matches!(
        &operation,
        AutomationOperation::GenerateCharacterPack(request)
            if request.workflow.id == "topdown-cycle"
                && request.workflow.version == "10.0.0"
    ) {
        if source.lifecycle_state != JobLifecycleState::Failed
            || source.error_code.as_deref() != Some("character_scale_lock_failed")
            || item != Some("walk_down")
            || frame.is_some()
            || stage != CharacterRetryStageArg::Loop
            || asymmetric_gait
            || image_model.is_some()
            || authorization.is_some()
        {
            return Err((
                "topdown_cycle_v101_scope_invalid".into(),
                "V10 anchor replay requires the failed scale-lock Job with --item walk_down --stage loop, no frame/model override/authorization"
                    .into(),
            ));
        }
        let AutomationOperation::GenerateCharacterPack(request) = &mut operation else {
            unreachable!()
        };
        request.workflow.version = "10.1.0".into();
        request.reuse_from_job_dir = Some(source.job_dir.clone());
        request.retry_animations = vec!["walk_down".into()];
        request.retry_frames.clear();
        request.retry_stages = [("walk_down".into(), CharacterRetryStage::Loop)].into();
        request.metadata.name = format!("{} V10.1 Anchor Replay", request.metadata.name);
        return prepare_or_plan(operation, wait, plan_only, None);
    }
    match &mut operation {
        AutomationOperation::GenerateStaticAssetSet(request) => {
            let portrait_v2 = request.asset.kind == StaticAssetKind::PortraitSet
                && request.asset.schema_version == "2";
            let all_static_item_ids = request
                .asset
                .items
                .iter()
                .map(|candidate| candidate.id.clone())
                .collect::<Vec<_>>();
            let expression_item_ids = all_static_item_ids
                .iter()
                .filter(|item| item.as_str() != "neutral")
                .cloned()
                .collect::<Vec<_>>();
            if frame.is_some() {
                return Err((
                    "unsupported_retry_frame".into(),
                    "--frame is only available for topdown-keyframes Character jobs".into(),
                ));
            }
            if !matches!(
                stage,
                CharacterRetryStageArg::Auto | CharacterRetryStageArg::Consistency
            ) {
                return Err((
                    "unsupported_retry_stage".into(),
                    "static assets support only --stage auto or --stage consistency".into(),
                ));
            }
            request.reuse_from_job_dir = None;
            request.retry_item_ids.clear();
            request.consistency_recheck_only = false;
            request.resume_incomplete_static = false;
            if stage == CharacterRetryStageArg::Consistency {
                let project = read_project(&request.project_path).map_err(display_error)?;
                let revision = project.current_style_revision.ok_or_else(|| {
                    (
                        "style_missing".into(),
                        "run `forge style create` before consistency recheck".into(),
                    )
                })?;
                request.style_lock_path = request
                    .project_path
                    .join(".forge/styles")
                    .join(revision)
                    .join(STYLE_LOCK_FILE);
                request.reuse_from_job_dir = Some(source.job_dir.clone());
                request.retry_item_ids =
                    if request.portrait_phase == PortraitGenerationPhaseV1::BaseOnly {
                        if item.is_some_and(|item| item != "neutral") {
                            return Err((
                                "portrait_base_retry_item_invalid".into(),
                                "base_only Portrait Job can retry only neutral".into(),
                            ));
                        }
                        vec!["neutral".into()]
                    } else if request.portrait_phase == PortraitGenerationPhaseV1::Expressions {
                        if item == Some("neutral") {
                            return Err((
                            "portrait_base_immutable".into(),
                            "an approved PortraitBase cannot be retried inside an expressions Job"
                                .into(),
                        ));
                        }
                        item.map(|item| vec![item.into()])
                            .unwrap_or_else(|| expression_item_ids.clone())
                    } else if let Some(item_id) = item {
                        if !request
                            .asset
                            .items
                            .iter()
                            .any(|candidate| candidate.id == item_id)
                        {
                            return Err((
                                "item_not_found".into(),
                                format!("job has no item {item_id}"),
                            ));
                        }
                        if portrait_v2 && item_id == "neutral" {
                            all_static_item_ids.clone()
                        } else {
                            vec![item_id.into()]
                        }
                    } else {
                        request
                            .asset
                            .items
                            .iter()
                            .map(|candidate| candidate.id.clone())
                            .collect()
                    };
                request.consistency_recheck_only = true;
            } else if item.is_none()
                && source.lifecycle_state == forge_core::job::JobLifecycleState::Failed
            {
                if !matches!(
                    source.error_code.as_deref(),
                    Some(
                        "provider_request_failed"
                            | "provider_unavailable"
                            | "provider_rate_limited"
                            | "provider_io_error"
                    )
                ) {
                    return Err((
                        "static_resume_not_recoverable".into(),
                        "failed static Job is not a recoverable Provider transport failure".into(),
                    ));
                }
                let mut retry_items = Vec::new();
                for asset_item in &request.asset.items {
                    let step = source
                        .steps
                        .iter()
                        .find(|step| step.name == format!("item:{}", asset_item.id));
                    let normalized = source
                        .job_dir
                        .join("normalized/static")
                        .join(format!("{}.png", asset_item.id));
                    if step.is_some_and(|step| step.state == "succeeded") {
                        if !normalized.is_file() {
                            return Err((
                                "legacy_artifact_missing".into(),
                                format!(
                                    "completed source item {} has no reusable normalized PNG",
                                    asset_item.id
                                ),
                            ));
                        }
                    } else {
                        retry_items.push(asset_item.id.clone());
                    }
                }
                if retry_items.is_empty() {
                    return Err((
                        "static_resume_not_required".into(),
                        "failed static Job has no incomplete item".into(),
                    ));
                }
                request.reuse_from_job_dir = Some(source.job_dir.clone());
                if request.portrait_phase == PortraitGenerationPhaseV1::BaseOnly {
                    retry_items.retain(|item| item == "neutral");
                } else if request.portrait_phase == PortraitGenerationPhaseV1::Expressions {
                    retry_items.retain(|item| item != "neutral");
                } else if portrait_v2 && retry_items.iter().any(|item| item == "neutral") {
                    retry_items = all_static_item_ids.clone();
                }
                request.retry_item_ids = retry_items;
                request.resume_incomplete_static = true;
            } else if let Some(item_id) = item {
                if !request
                    .asset
                    .items
                    .iter()
                    .any(|candidate| candidate.id == item_id)
                {
                    return Err((
                        "item_not_found".into(),
                        format!("job has no item {item_id}"),
                    ));
                }
                request.reuse_from_job_dir = Some(source.job_dir.clone());
                request.retry_item_ids =
                    if request.portrait_phase == PortraitGenerationPhaseV1::BaseOnly {
                        if item_id != "neutral" {
                            return Err((
                                "portrait_base_retry_item_invalid".into(),
                                "base_only Portrait Job can retry only neutral".into(),
                            ));
                        }
                        vec!["neutral".into()]
                    } else if request.portrait_phase == PortraitGenerationPhaseV1::Expressions
                        && item_id == "neutral"
                    {
                        return Err((
                            "portrait_base_immutable".into(),
                            "an approved PortraitBase cannot be retried inside an expressions Job"
                                .into(),
                        ));
                    } else if portrait_v2 && item_id == "neutral" {
                        all_static_item_ids
                    } else {
                        vec![item_id.into()]
                    };
            }
        }
        AutomationOperation::GenerateCharacterPack(request) if item.is_some() => {
            let is_keyframe = request.workflow.id == "topdown-keyframes"
                && matches!(
                    request.workflow.version.as_str(),
                    "2.0.0" | "2.1.0" | "2.2.0" | "2.3.0"
                )
                || (request.workflow.id == "topdown-keyposes"
                    && matches!(request.workflow.version.as_str(), "2.4.0" | "2.5.0"));
            let is_sprite_sheet =
                request.workflow.id == "topdown-spritesheet" && request.workflow.version == "3.0.0";
            let is_locked_frames =
                request.workflow.id == "topdown-frames" && request.workflow.version == "4.0.0";
            let is_locked_video = (request.workflow.id == "topdown-video-locked"
                && request.workflow.version == "5.0.0")
                || (request.workflow.id == "topdown-video-cycle"
                    && request.workflow.version == "6.0.0");
            let is_direction_motion = request.workflow.id == "topdown-direction-motion"
                && request.workflow.version == "8.0.0";
            let is_grid =
                request.workflow.id == "topdown-grid" && request.workflow.version == "9.0.0";
            let is_grid_keyframes = request.workflow.id == "topdown-grid"
                && matches!(
                    request.workflow.version.as_str(),
                    "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
                );
            if asymmetric_gait {
                if request.workflow.id != "topdown-grid"
                    || request.workflow.version != "9.2.0"
                    || source.lifecycle_state != JobLifecycleState::Failed
                    || source.error_code.as_deref()
                        != Some(forge_core::gait_laterality::WALK_LATERALITY_NOT_ALTERNATING)
                    || item != Some("walk_down")
                    || frame != Some(2)
                    || stage != CharacterRetryStageArg::Frame
                {
                    return Err((
                        "invalid_asymmetric_gait_remediation".into(),
                        "--asymmetric-gait requires a stable topdown-grid@9.2.0 laterality-failed Job with --item walk_down --frame 2 --stage frame"
                            .into(),
                    ));
                }
                request.workflow.version = "9.3.0".into();
            }
            let animation = item.unwrap();
            let grid_direction_retry = is_grid_direction_lock_retry(request, animation);
            let valid_animation = if grid_direction_retry {
                true
            } else if is_direction_motion {
                matches!(
                    animation,
                    "idle_down"
                        | "idle_up"
                        | "idle_right"
                        | "idle_left"
                        | "walk_down"
                        | "walk_up"
                        | "walk_right"
                        | "walk_left"
                )
            } else {
                matches!(animation, "idle" | "walk_up" | "walk_right" | "walk_down")
            };
            let valid_animation = if grid_direction_retry {
                true
            } else if is_grid {
                matches!(
                    animation,
                    "idle_down"
                        | "idle_up"
                        | "idle_right"
                        | "idle_left"
                        | "walk_down"
                        | "walk_up"
                        | "walk_right"
                        | "walk_left"
                )
            } else {
                valid_animation
            };
            if is_direction_motion && frame.is_some() {
                return Err((
                    "unsupported_retry_frame".into(),
                    "topdown-direction-motion@8.0.0 image locks retry by direction node, not frame"
                        .into(),
                ));
            }
            if is_direction_motion
                && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
                && !matches!(
                    stage,
                    CharacterRetryStageArg::Auto | CharacterRetryStageArg::Still
                )
            {
                return Err((
                    "unsupported_retry_stage".into(),
                    "topdown-direction-motion@8.0.0 image locks accept only --stage auto or --stage still"
                        .into(),
                ));
            }
            if is_grid
                && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
                && (!grid_direction_retry
                    || frame.is_some()
                    || !matches!(
                        stage,
                        CharacterRetryStageArg::Auto | CharacterRetryStageArg::Still
                    ))
            {
                return Err((
                    "unsupported_retry_stage".into(),
                    "topdown-grid@9.0.0 direction lock retry requires --item direction_grid and --stage auto or still"
                        .into(),
                ));
            }
            let valid_animation = if is_grid_keyframes {
                matches!(
                    animation,
                    "walk_down" | "walk_up" | "walk_right" | "walk_left"
                )
            } else {
                valid_animation
            };
            if !valid_animation {
                return Err((
                    "item_not_found".into(),
                    format!("job has no animation {animation}"),
                ));
            }
            if frame.is_some() && !(is_keyframe || is_locked_frames || is_grid_keyframes) {
                return Err((
                    "unsupported_retry_frame".into(),
                    "--frame is available for topdown keyframe/keypose, topdown-frames, and topdown-grid@9.1.0 through @9.5.0 workflows".into(),
                ));
            }
            if stage == CharacterRetryStageArg::Frame && frame.is_none() {
                return Err((
                    "retry_frame_required".into(),
                    "--stage frame requires --frame within the workflow frame range".into(),
                ));
            }
            if is_locked_video
                && matches!(
                    stage,
                    CharacterRetryStageArg::Still | CharacterRetryStageArg::Frame
                )
            {
                return Err((
                    "unsupported_retry_stage".into(),
                    "topdown-video-locked@5.0.0 and topdown-video-cycle@6.0.0 keep DirectionLock immutable; retry video, loop, matting, consistency, or auto"
                        .into(),
                ));
            }
            let frame_count = if request.workflow.id == "topdown-keyposes"
                || is_locked_frames
                || is_grid
                || is_grid_keyframes
            {
                4
            } else {
                8
            };
            if frame.is_some_and(|frame| frame >= frame_count) {
                return Err((
                    "retry_frame_out_of_range".into(),
                    format!(
                        "{}@{} accepts frames 0..{}",
                        request.workflow.id,
                        request.workflow.version,
                        frame_count - 1
                    ),
                ));
            }
            if frame.is_some()
                && !matches!(
                    stage,
                    CharacterRetryStageArg::Auto | CharacterRetryStageArg::Frame
                )
            {
                return Err((
                    "unsupported_retry_stage".into(),
                    "a frame retry accepts only --stage auto or --stage frame".into(),
                ));
            }
            if is_sprite_sheet
                && !matches!(
                    stage,
                    CharacterRetryStageArg::Auto
                        | CharacterRetryStageArg::Still
                        | CharacterRetryStageArg::Matting
                        | CharacterRetryStageArg::Loop
                        | CharacterRetryStageArg::Consistency
                )
            {
                return Err((
                    "unsupported_retry_stage".into(),
                    "topdown-spritesheet accepts --stage auto, still, matting, loop, or consistency"
                        .into(),
                ));
            }
            let selected_stage = if grid_direction_retry {
                CharacterRetryStage::Still
            } else if frame.is_some() || is_grid_keyframes {
                CharacterRetryStage::Frame
            } else if (is_sprite_sheet || is_locked_frames) && stage == CharacterRetryStageArg::Auto
            {
                CharacterRetryStage::Still
            } else {
                select_character_retry_stage(&source, animation, stage)
            };
            request.reuse_from_job_dir = Some(source.job_dir.clone());
            if image2_candidate {
                request.generation.image_model = Some(XAI_IMAGE_2_CANDIDATE_MODEL.into());
                request.generation.max_attempts_per_animation = 1;
            }
            request.retry_animations = vec![animation.into()];
            request.retry_frames.clear();
            request.retry_stages.clear();
            if let Some(frame) = frame {
                request.retry_frames.insert(animation.into(), vec![frame]);
            }
            request
                .retry_stages
                .insert(animation.into(), selected_stage);
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            if asymmetric_gait {
                return Err((
                    "invalid_asymmetric_gait_remediation".into(),
                    "--asymmetric-gait requires --item walk_down --frame 2".into(),
                ));
            }
            if stage == CharacterRetryStageArg::Consistency {
                request.reuse_from_job_dir = Some(source.job_dir.clone());
                request.retry_animations = vec![
                    "idle".into(),
                    "walk_up".into(),
                    "walk_right".into(),
                    "walk_down".into(),
                ];
                request.retry_frames.clear();
                request.retry_stages = request
                    .retry_animations
                    .iter()
                    .map(|animation| (animation.clone(), CharacterRetryStage::Consistency))
                    .collect();
            } else if stage == CharacterRetryStageArg::Auto
                && source.error_summary.as_deref().is_some_and(|summary| {
                    summary.contains("canonical character reference failed character-identity@")
                })
                && source
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.kind == "provider_reference")
            {
                // A repaired local identity profile may accept already-paid
                // canonical pixels. Resume all missing animation stages from
                // that immutable reference instead of spending the second
                // subject_reference allowance.
                request.reuse_from_job_dir = Some(source.job_dir.clone());
                request.retry_animations = vec![
                    "idle".into(),
                    "walk_up".into(),
                    "walk_right".into(),
                    "walk_down".into(),
                ];
                request.retry_frames.clear();
                request.retry_stages = request
                    .retry_animations
                    .iter()
                    .map(|animation| (animation.clone(), CharacterRetryStage::Auto))
                    .collect();
            } else if stage != CharacterRetryStageArg::Auto {
                return Err((
                    "retry_item_required".into(),
                    "Character retry stages require --item <animation>".into(),
                ));
            }
        }
        _ => {
            if asymmetric_gait {
                return Err((
                    "invalid_asymmetric_gait_remediation".into(),
                    "--asymmetric-gait is supported only for generated Character grid Jobs".into(),
                ));
            }
            return Err((
                "unsupported_retry".into(),
                "only generated character and static set jobs can be retried".into(),
            ));
        }
    }
    prepare_or_plan(operation, wait, plan_only, authorization)
}

fn is_grid_direction_lock_retry(request: &GenerateCharacterPackRequest, item: &str) -> bool {
    request.workflow.id == "topdown-grid"
        && request.workflow.version == "9.0.0"
        && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
        && item == "direction_grid"
}

fn assemble_character_jobs(
    base_id: &str,
    source_args: &[String],
    wait: bool,
) -> Result<(), (String, String)> {
    let required = ["idle", "walk_up", "walk_right", "walk_down"];
    let mut sources = std::collections::BTreeMap::<String, String>::new();
    for value in source_args {
        let (animation, job_id) = value.split_once('=').ok_or_else(|| {
            (
                "invalid_animation_source".into(),
                "--source must use animation=job-id".into(),
            )
        })?;
        if !required.contains(&animation) || job_id.is_empty() {
            return Err((
                "invalid_animation_source".into(),
                format!("unsupported Character source: {value}"),
            ));
        }
        if sources.insert(animation.into(), job_id.into()).is_some() {
            return Err((
                "duplicate_animation_source".into(),
                format!("duplicate source for {animation}"),
            ));
        }
    }
    if sources.len() != required.len() {
        return Err((
            "animation_source_missing".into(),
            "assemble-character requires idle, walk_up, walk_right, and walk_down".into(),
        ));
    }

    let store = job_store()?;
    let base = store.read_record(base_id).map_err(display_error)?;
    let base_recipe = base.recipe.clone().ok_or_else(|| {
        (
            "recipe_missing".into(),
            "base Character Job has no immutable recipe".into(),
        )
    })?;
    let base_operation: AutomationOperation =
        serde_json::from_value(base_recipe.clone()).map_err(json_error)?;
    let AutomationOperation::GenerateCharacterPack(base_request) = &base_operation else {
        return Err((
            "unsupported_assembly".into(),
            "base Job is not a generated Character".into(),
        ));
    };
    let lineage_root = base
        .lineage_root_job_id
        .clone()
        .unwrap_or_else(|| base.job_id.clone());
    let base_manifest_path = base.job_dir.join("source/provider-manifest.json");
    let base_manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(validate_job_file(&base, &base_manifest_path)?).map_err(io_error)?,
    )
    .map_err(json_error)?;
    let reference = base_manifest.get("reference").cloned().ok_or_else(|| {
        (
            "provider_manifest_invalid".into(),
            "base Provider manifest has no reference".into(),
        )
    })?;
    let reference_path = PathBuf::from(
        reference
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                (
                    "provider_manifest_invalid".into(),
                    "base reference has no path".into(),
                )
            })?,
    );
    let reference_sha = reference
        .get("sha256")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            (
                "provider_manifest_invalid".into(),
                "base reference has no SHA-256".into(),
            )
        })?;
    validate_job_file(&base, &reference_path)?;
    if hash_asset_file(&reference_path).map_err(display_error)? != reference_sha {
        return Err((
            "source_hash_mismatch".into(),
            "base Character reference SHA-256 changed".into(),
        ));
    }

    let staging = store
        .create_job(SourceKind::FromCode)
        .map_err(display_error)?;
    let provider_root = staging.job_dir.join("source/provider");
    let staged_reference = provider_root.join("reference/reference.png");
    fs::create_dir_all(staged_reference.parent().unwrap()).map_err(io_error)?;
    fs::copy(&reference_path, &staged_reference).map_err(io_error)?;
    let mut animations = serde_json::Map::new();
    let mut artifacts = vec![JobArtifactRecord {
        kind: "provider_reference".into(),
        path: staged_reference.clone(),
        sha256: Some(reference_sha.into()),
    }];
    let mut assembly_sources = serde_json::Map::new();

    for animation in required {
        let source_id = sources.get(animation).expect("all sources checked above");
        let source = store.read_record(source_id).map_err(display_error)?;
        let source_lineage = source
            .lineage_root_job_id
            .clone()
            .unwrap_or_else(|| source.job_id.clone());
        if source_lineage != lineage_root {
            return Err((
                "assembly_lineage_mismatch".into(),
                format!("{animation} source belongs to another Job lineage"),
            ));
        }
        let operation: AutomationOperation =
            serde_json::from_value(source.recipe.clone().ok_or_else(|| {
                (
                    "recipe_missing".into(),
                    format!("{animation} source has no recipe"),
                )
            })?)
            .map_err(json_error)?;
        let AutomationOperation::GenerateCharacterPack(source_request) = operation else {
            return Err((
                "unsupported_assembly".into(),
                format!("{animation} source is not a generated Character"),
            ));
        };
        if source_request.provider_id != base_request.provider_id
            || source_request.profile_id != base_request.profile_id
            || source_request.asset_id != base_request.asset_id
            || source_request.generation.image_model != base_request.generation.image_model
            || source_request.generation.video_model != base_request.generation.video_model
            || source_request.character.prompt != base_request.character.prompt
        {
            return Err((
                "assembly_identity_mismatch".into(),
                format!("{animation} source does not match the base Character lock"),
            ));
        }
        let manifest_path = source.job_dir.join("source/provider-manifest.json");
        let manifest: serde_json::Value = serde_json::from_slice(
            &fs::read(validate_job_file(&source, &manifest_path)?).map_err(io_error)?,
        )
        .map_err(json_error)?;
        let mut media = manifest
            .pointer(&format!("/animations/{animation}"))
            .cloned()
            .ok_or_else(|| {
                (
                    "animation_source_missing".into(),
                    format!("Job {source_id} has no Provider media for {animation}"),
                )
            })?;
        let still_path = PathBuf::from(
            media
                .get("stillPath")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    (
                        "provider_manifest_invalid".into(),
                        "stillPath missing".into(),
                    )
                })?,
        );
        let video_path = PathBuf::from(
            media
                .get("videoPath")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    (
                        "provider_manifest_invalid".into(),
                        "videoPath missing".into(),
                    )
                })?,
        );
        let still_sha = media
            .get("stillSha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                (
                    "provider_manifest_invalid".into(),
                    "stillSha256 missing".into(),
                )
            })?
            .to_owned();
        let video_sha = media
            .get("videoSha256")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                (
                    "provider_manifest_invalid".into(),
                    "videoSha256 missing".into(),
                )
            })?
            .to_owned();
        validate_job_file(&source, &still_path)?;
        validate_job_file(&source, &video_path)?;
        if hash_asset_file(&still_path).map_err(display_error)? != still_sha
            || hash_asset_file(&video_path).map_err(display_error)? != video_sha
        {
            return Err((
                "source_hash_mismatch".into(),
                format!("{animation} source media SHA-256 changed"),
            ));
        }
        let target = provider_root.join(animation).join("reused");
        fs::create_dir_all(&target).map_err(io_error)?;
        let staged_still = target.join("direction.png");
        let staged_video = target.join("animation.mp4");
        fs::copy(&still_path, &staged_still).map_err(io_error)?;
        fs::copy(&video_path, &staged_video).map_err(io_error)?;
        media["stillPath"] = serde_json::json!(staged_still);
        media["videoPath"] = serde_json::json!(staged_video);
        media["retryMethod"] = serde_json::json!(format!("multi_source_assembly:{source_id}"));
        animations.insert(animation.into(), media);
        artifacts.extend([
            JobArtifactRecord {
                kind: format!("reused_still_{animation}"),
                path: staged_still,
                sha256: Some(still_sha.clone()),
            },
            JobArtifactRecord {
                kind: format!("reused_video_{animation}"),
                path: staged_video,
                sha256: Some(video_sha.clone()),
            },
        ]);
        assembly_sources.insert(
            animation.into(),
            serde_json::json!({"jobId": source_id, "stillSha256": still_sha, "videoSha256": video_sha}),
        );
    }

    let mut staged_manifest = base_manifest;
    staged_manifest["reference"]["path"] = serde_json::json!(staged_reference);
    staged_manifest["animations"] = serde_json::Value::Object(animations);
    staged_manifest["retryAnimations"] = serde_json::json!(required);
    staged_manifest["retrySourceJob"] = serde_json::json!(base_id);
    staged_manifest["retryStages"] = serde_json::json!({
        "idle": "consistency",
        "walk_up": "consistency",
        "walk_right": "consistency",
        "walk_down": "consistency"
    });
    staged_manifest["usage"] = serde_json::json!({
        "requests": 0, "generatedImages": 0, "generatedVideos": 0,
        "editedVideos": 0, "privateFileUploads": 0
    });
    let provider_manifest_path = staging.job_dir.join("source/provider-manifest.json");
    fs::write(
        &provider_manifest_path,
        serde_json::to_vec_pretty(&staged_manifest).map_err(json_error)?,
    )
    .map_err(io_error)?;
    let assembly_manifest_path = staging.job_dir.join("character-assembly-manifest.json");
    fs::write(
        &assembly_manifest_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1",
            "profile": "character-multi-source-assembly@1.0.0",
            "baseJobId": base_id,
            "lineageRootJobId": lineage_root,
            "providerRequestOccurred": false,
            "sources": assembly_sources
        }))
        .map_err(json_error)?,
    )
    .map_err(io_error)?;
    artifacts.extend([
        JobArtifactRecord {
            kind: "provider_manifest".into(),
            path: provider_manifest_path.clone(),
            sha256: Some(hash_asset_file(&provider_manifest_path).map_err(display_error)?),
        },
        JobArtifactRecord {
            kind: "character_assembly_manifest".into(),
            path: assembly_manifest_path.clone(),
            sha256: Some(hash_asset_file(&assembly_manifest_path).map_err(display_error)?),
        },
    ]);
    store
        .update_record(&staging.job_id, |record| {
            record.state = JobState::QualityChecked;
            record.lifecycle_state = JobLifecycleState::AwaitingReview;
            record.operation_kind = JobOperationKind::GenerateCharacterPack;
            record.asset_id = base.asset_id.clone();
            record.parent_job_id = Some(base.job_id.clone());
            record.lineage_root_job_id = Some(lineage_root.clone());
            record.recipe = Some(base_recipe.clone());
            record.artifacts = artifacts.clone();
            record.progress = 1.0;
            record.next_actions = vec!["job_retry_consistency".into()];
        })
        .map_err(display_error)?;
    retry_job(
        &staging.job_id,
        None,
        None,
        RetryJobOptions {
            stage: CharacterRetryStageArg::Consistency,
            asymmetric_gait: false,
            front_authoritative_grid: false,
            image_model: None,
            wait,
            plan_only: false,
            authorization: None,
        },
    )
}

fn validate_job_file(record: &JobRecord, path: &Path) -> Result<PathBuf, (String, String)> {
    if !path.is_file() {
        return Err((
            "source_file_missing".into(),
            format!("source file is missing: {}", path.display()),
        ));
    }
    let root = fs::canonicalize(&record.job_dir).map_err(io_error)?;
    let canonical = fs::canonicalize(path).map_err(io_error)?;
    if !canonical.starts_with(&root) {
        return Err((
            "source_path_escape".into(),
            format!("source file escaped Job {}", record.job_id),
        ));
    }
    Ok(canonical)
}

fn cleanup_footwear_job(id: &str) -> Result<(), (String, String)> {
    const SCOPE: &str =
        "topdown-grid@9.4.0:walk_down:frame:2:deterministic-neutral-gray-cleanup@1.0.0";
    let store = job_store()?;
    let source = store.read_record(id).map_err(display_error)?;
    let validated = forge_core::grid_retry_source::validate_v94_footwear_cleanup_source_closure(
        &source.job_dir,
        &[2],
    )
    .map_err(display_error)?;
    let child = store
        .create_claimed_child_job(&source.job_dir, &source.job_id, SCOPE, SourceKind::FromCode)
        .map_err(display_error)?;
    let result = materialize_footwear_cleanup(&store, &child, &validated);
    match result {
        Ok(record) => success(&serde_json::json!({
            "job": record,
            "providerRequestOccurred": false,
            "providerRequestCount": 0,
            "providerCostInUsdTicks": 0
        })),
        Err(error) => {
            let _ = store.update_record(&child.job_id, |record| {
                record.state = JobState::Failed;
                record.lifecycle_state = JobLifecycleState::Failed;
                record.progress = 1.0;
                record.error_code = Some("footwear_deterministic_cleanup_failed".into());
                record.error_summary = Some(error.1.clone());
                record.recoverable = false;
                record.next_actions = vec!["job_report".into()];
            });
            Err(error)
        }
    }
}

fn materialize_footwear_cleanup(
    store: &JobStore,
    child: &JobRecord,
    source: &forge_core::grid_retry_source::ValidatedGridFootwearCleanupSourceV1,
) -> Result<JobRecord, (String, String)> {
    let frame_root = child.job_dir.join("grid-keyframes/walk_down");
    let report_root = child.job_dir.join("source/local-footwear-cleanup");
    let review_root = child.job_dir.join("grid-keyframe-validation");
    fs::create_dir_all(&frame_root).map_err(io_error)?;
    fs::create_dir_all(&report_root).map_err(io_error)?;
    fs::create_dir_all(&review_root).map_err(io_error)?;

    let mut frame_paths = Vec::with_capacity(4);
    let mut frame_sha256 = Vec::with_capacity(4);
    let mut source_sha256 = Vec::with_capacity(4);
    let mut repair_report = None;
    for frame in &source.report.frames {
        let output = frame_root.join(format!("frame-{:02}.png", frame.frame_index));
        source_sha256.push(frame.sha256.clone());
        if frame.frame_index == 2 {
            let image = image::open(&frame.path).map_err(display_error)?.to_rgba8();
            let (repaired, report) = repair_neutral_gray_footwear_leak(&image);
            if report.verdict != ConsistencyVerdict::GameReady
                || !report.non_mask_pixels_unchanged
                || report.modified_pixels == 0
            {
                return Err((
                    "footwear_deterministic_cleanup_blocked".into(),
                    "neutral-gray cleanup did not produce an auditable game-ready frame".into(),
                ));
            }
            repaired.save(&output).map_err(display_error)?;
            repair_report = Some(report);
        } else {
            fs::copy(&frame.path, &output).map_err(io_error)?;
        }
        frame_sha256.push(hash_asset_file(&output).map_err(display_error)?);
        frame_paths.push(output);
    }
    let repair_report = repair_report.ok_or_else(|| {
        (
            "footwear_deterministic_cleanup_blocked".into(),
            "source report has no frame 2".into(),
        )
    })?;

    let frames = frame_paths
        .iter()
        .map(|path| image::open(path).map(|image| image.to_rgba8()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(display_error)?;
    let animations = BTreeMap::from([("walk_down".into(), frames.clone())]);
    let motion = assess_character_motion_semantics(&animations);
    let laterality = assess_front_gait_laterality("walk_down", &frames);
    let equipment =
        assess_hand_equipment_contact_with_kind(&animations, CharacterEquipmentKindV1::None);
    let footwear = assess_footwear_platform_action("walk_down", &frames);
    let accepted = motion.verdict == ConsistencyVerdict::GameReady
        && laterality.verdict == ConsistencyVerdict::GameReady
        && equipment.verdict == ConsistencyVerdict::GameReady
        && footwear.verdict == ConsistencyVerdict::GameReady;

    let repair_path = report_root.join("footwear-neutral-gray-repair.json");
    let motion_path = report_root.join("walk_down-motion.json");
    let laterality_path = report_root.join("walk_down-laterality.json");
    let equipment_path = report_root.join("walk_down-equipment.json");
    let footwear_path = report_root.join("walk_down-footwear.json");
    for (path, value) in [
        (
            &repair_path,
            serde_json::to_value(&repair_report).map_err(json_error)?,
        ),
        (
            &motion_path,
            serde_json::to_value(&motion).map_err(json_error)?,
        ),
        (
            &laterality_path,
            serde_json::to_value(&laterality).map_err(json_error)?,
        ),
        (
            &equipment_path,
            serde_json::to_value(&equipment).map_err(json_error)?,
        ),
        (
            &footwear_path,
            serde_json::to_value(&footwear).map_err(json_error)?,
        ),
    ] {
        fs::write(path, serde_json::to_vec_pretty(&value).map_err(json_error)?)
            .map_err(io_error)?;
    }
    let contact_path = review_root.join("walk_down-contact-sheet.png");
    write_contact_sheet(&frame_paths, &contact_path, 256).map_err(display_error)?;
    let usage_path = child.job_dir.join("provider-usage.json");
    let usage = serde_json::json!({
        "providerId": "local",
        "schemaVersion": "1",
        "usage": {
            "costInUsdTicks": 0,
            "editedVideos": 0,
            "generatedImages": 0,
            "generatedVideos": 0,
            "privateFileUploads": 0,
            "requests": 0
        }
    });
    fs::write(
        &usage_path,
        serde_json::to_vec_pretty(&usage).map_err(json_error)?,
    )
    .map_err(io_error)?;
    let manifest_path = report_root.join("local-cleanup-manifest.json");
    let manifest = serde_json::json!({
        "schemaVersion": "1",
        "profile": "grid-local-footwear-cleanup@1.0.0",
        "sourceJobId": source.job.job_id,
        "sourceWorkflow": "topdown-grid@9.4.0",
        "animation": "walk_down",
        "frameIndex": 2,
        "sourceFrameSha256": source_sha256[2],
        "outputFrameSha256": frame_sha256[2],
        "sourceFrameSha256s": source_sha256,
        "outputFrameSha256s": frame_sha256,
        "providerRequestOccurred": false,
        "providerRequestCount": 0,
        "modifiedPixels": repair_report.modified_pixels,
        "repairBounds": repair_report.bounds,
        "nonMaskPixelsUnchanged": repair_report.non_mask_pixels_unchanged,
        "motionVerdict": motion.verdict,
        "lateralityVerdict": laterality.verdict,
        "equipmentVerdict": equipment.verdict,
        "footwearVerdict": footwear.verdict
    });
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(json_error)?,
    )
    .map_err(io_error)?;

    let artifact = |kind: &str, path: &Path| -> Result<JobArtifactRecord, (String, String)> {
        Ok(JobArtifactRecord {
            kind: kind.into(),
            path: path.to_path_buf(),
            sha256: Some(hash_asset_file(path).map_err(display_error)?),
        })
    };
    let mut artifacts = vec![
        artifact("footwear_cleanup_report", &repair_path)?,
        artifact("grid_keyframe_motion_walk_down", &motion_path)?,
        artifact("grid_keyframe_laterality_walk_down", &laterality_path)?,
        artifact("grid_keyframe_equipment_walk_down", &equipment_path)?,
        artifact("grid_keyframe_footwear_walk_down", &footwear_path)?,
        artifact(
            "grid_keyframe_validation_contact_sheet_walk_down",
            &contact_path,
        )?,
        artifact("local_cleanup_manifest", &manifest_path)?,
        artifact("provider_usage", &usage_path)?,
    ];
    for (index, path) in frame_paths.iter().enumerate() {
        artifacts.push(artifact(&format!("grid_keyframe_walk_down_{index}"), path)?);
    }
    let input_hash =
        hash_asset_file(&source.job.job_dir.join("job.json")).map_err(display_error)?;
    let record = store
        .update_record(&child.job_id, |record| {
            record.asset_id = source.job.asset_id.clone();
            record.parent_job_id = Some(source.job.job_id.clone());
            record.lineage_root_job_id = source
                .job
                .lineage_root_job_id
                .clone()
                .or_else(|| Some(source.job.job_id.clone()));
            record.operation_kind = JobOperationKind::GenerateCharacterPack;
            record.state = if accepted {
                JobState::QualityChecked
            } else {
                JobState::Failed
            };
            record.lifecycle_state = if accepted {
                JobLifecycleState::AwaitingReview
            } else {
                JobLifecycleState::Failed
            };
            record.progress = 1.0;
            record.input_hash = Some(input_hash.clone());
            record.artifacts = artifacts.clone();
            record.error_code = Some(if accepted {
                "footwear_cleanup_review_required".into()
            } else {
                "footwear_deterministic_cleanup_quality_failed".into()
            });
            record.error_summary = Some(if accepted {
                "walk_down frame 2 passed deterministic neutral-gray cleanup and awaits native review; no Provider request or Pack export occurred".into()
            } else {
                "deterministic footwear cleanup failed one or more walk_down quality gates".into()
            });
            record.recoverable = accepted;
            record.next_actions = vec!["job_report".into()];
        })
        .map_err(display_error)?;
    if !accepted {
        return Err((
            "footwear_deterministic_cleanup_quality_failed".into(),
            "deterministic footwear cleanup failed quality gates".into(),
        ));
    }
    Ok(record)
}

fn replay_job(id: &str, from: &str, wait: bool) -> Result<(), (String, String)> {
    let source = job_store()?.read_record(id).map_err(display_error)?;
    let graph_path = source.job_dir.join(WORKFLOW_GRAPH_FILE);
    let graph = read_workflow_graph(&graph_path).map_err(display_error)?;
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == from)
        .ok_or_else(|| {
            (
                "workflow_node_not_found".into(),
                format!("workflow node not found: {from}"),
            )
        })?;
    let item = node.item.as_deref();
    let stage = match node.stage.as_str() {
        "frame_image" => CharacterRetryStageArg::Frame,
        "animation_video" => CharacterRetryStageArg::Video,
        "direction_still" | "direction_still_preflight" => CharacterRetryStageArg::Still,
        "matting" => CharacterRetryStageArg::Matting,
        "loop_select" | "loop_quality" => CharacterRetryStageArg::Loop,
        "collection_consistency" | "quality" | "shared_normalize" | "pack" => {
            CharacterRetryStageArg::Consistency
        }
        "provisional_align" => CharacterRetryStageArg::Matting,
        other => {
            return Err((
                "workflow_node_not_replayable".into(),
                format!("workflow node stage is not replayable: {other}"),
            ))
        }
    };
    retry_job(
        id,
        item,
        node.frame,
        RetryJobOptions {
            stage,
            asymmetric_gait: false,
            front_authoritative_grid: false,
            image_model: None,
            wait,
            plan_only: false,
            authorization: None,
        },
    )
}

fn select_character_retry_stage(
    source: &JobRecord,
    animation: &str,
    requested: CharacterRetryStageArg,
) -> CharacterRetryStage {
    if requested != CharacterRetryStageArg::Auto {
        return requested.into();
    }
    let read_artifact = |kind: &str| {
        source
            .artifacts
            .iter()
            .rev()
            .find(|artifact| artifact.kind == kind)
            .and_then(|artifact| fs::read(&artifact.path).ok())
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    };
    if let Some(consistency) = read_artifact("consistency_report") {
        let direction_failed = consistency
            .get("items")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .any(|item| {
                item.get("id").and_then(serde_json::Value::as_str) == Some(animation)
                    && matches!(
                        item.get("verdict").and_then(serde_json::Value::as_str),
                        Some("regenerate" | "blocked")
                    )
            });
        if direction_failed {
            return CharacterRetryStage::Still;
        }
    }
    if let Some(semantic) = read_artifact("character_semantic_quality_report") {
        if let Some(report) = semantic
            .get("animations")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find(|entry| entry.get("name").and_then(serde_json::Value::as_str) == Some(animation))
        {
            let reasons = report
                .get("reasons")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>();
            if reasons.contains(&"unexpected_detached_emissive_effect") {
                return CharacterRetryStage::Video;
            }
            if reasons.contains(&"unexpected_attached_emissive_halo") {
                return CharacterRetryStage::Matting;
            }
            if reasons.iter().any(|reason| {
                matches!(
                    *reason,
                    "body_top_alignment_drift"
                        | "body_scale_drift"
                        | "body_center_drift"
                        | "foot_anchor_drift"
                )
            }) {
                return CharacterRetryStage::Consistency;
            }
            if reasons.iter().any(|reason| {
                matches!(
                    *reason,
                    "walk_up_face_visible" | "front_direction_face_missing"
                )
            }) {
                return CharacterRetryStage::Still;
            }
        }
    }
    if let Some(loop_report) = read_artifact("loop_selection_report") {
        let report = loop_report
            .get("animations")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find(|entry| entry.get("name").and_then(serde_json::Value::as_str) == Some(animation))
            .and_then(|entry| entry.get("report"));
        if let Some(report) = report {
            let foreground_missing = report
                .get("reasons")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .any(|reason| reason.as_str() == Some("foreground_missing"));
            return if foreground_missing {
                CharacterRetryStage::Matting
            } else if report.get("verdict").and_then(serde_json::Value::as_str)
                == Some("game_ready")
            {
                CharacterRetryStage::Loop
            } else {
                CharacterRetryStage::Video
            };
        }
    }
    let has_video = source.artifacts.iter().any(|artifact| {
        artifact
            .kind
            .starts_with(&format!("provider_video_{animation}"))
    });
    if has_video {
        CharacterRetryStage::Loop
    } else {
        CharacterRetryStage::Still
    }
}

fn review_job(id: &str, accept: bool, reason: &str) -> Result<(), (String, String)> {
    if reason.trim().is_empty() {
        return Err((
            "review_reason_required".into(),
            "review requires a non-empty reason".into(),
        ));
    }
    let store = job_store()?;
    let record = store.read_record(id).map_err(display_error)?;
    if record.error_code.as_deref() == Some("footwear_cleanup_review_required") {
        return review_local_footwear_cleanup(&store, &record, accept, reason);
    }
    let operation: AutomationOperation =
        serde_json::from_value(record.recipe.clone().ok_or_else(|| {
            (
                "recipe_missing".into(),
                "job has no immutable recipe".into(),
            )
        })?)
        .map_err(json_error)?;
    let is_static = matches!(operation, AutomationOperation::GenerateStaticAssetSet(_));
    let reviewable = record.lifecycle_state == forge_core::job::JobLifecycleState::AwaitingReview
        || (!accept
            && is_static
            && record.lifecycle_state == forge_core::job::JobLifecycleState::Succeeded);
    if !reviewable {
        return Err((
            "job_not_awaiting_review".into(),
            "job is not awaiting review; only a succeeded static Pack may be quarantined".into(),
        ));
    }
    if accept && matches!(operation, AutomationOperation::GenerateCharacterPack(_)) {
        for (report_name, failure_summary) in [
            (
                "character-semantic-quality-report.json",
                "direction or detached equipment-effect failures cannot be manually accepted",
            ),
            (
                "character-silhouette-temporal-report.json",
                "temporal silhouette or Alpha failures cannot be manually accepted",
            ),
            (
                "character-silhouette-temporal-source-report.json",
                "source temporal silhouette failures cannot be manually accepted",
            ),
            (
                "character-alpha-repair-report.json",
                "Alpha repair budget failures cannot be manually accepted",
            ),
            (
                DIRECTION_GRID_APPEARANCE_FILE,
                "undeclared equipment or direction-grid hand-state failures cannot be manually accepted",
            ),
        ] {
            let report_path = record.job_dir.join(report_name);
            if !report_path.is_file() {
                continue;
            }
            let report: serde_json::Value =
                serde_json::from_slice(&fs::read(&report_path).map_err(io_error)?)
                    .map_err(json_error)?;
            if matches!(
                report.get("verdict").and_then(serde_json::Value::as_str),
                Some("blocked" | "regenerate")
            ) {
                return Err(("hard_failure_not_reviewable".into(), failure_summary.into()));
            }
        }
    }
    let validated_direction_grid_lock = if accept {
        match &operation {
            AutomationOperation::GenerateCharacterPack(request)
                if request.workflow.id == "topdown-grid"
                    && request.workflow.version == "9.0.0"
                    && request.direction_motion_stage
                        == DirectionMotionGenerationStageV1::ImageLocks =>
            {
                Some(validate_direction_grid_review_candidate(&record)?)
            }
            AutomationOperation::ImportDirectionGrid(request) => Some(
                validate_imported_direction_grid_review_candidate(&record, request)?,
            ),
            _ => None,
        }
    } else {
        None
    };
    let path = record.job_dir.join("review-decision.json");
    let reviewed_at = chrono::Utc::now();
    fs::write(
        &path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1", "accepted": accept, "reason": reason, "reviewedAt": reviewed_at
        }))
        .map_err(json_error)?,
    )
    .map_err(io_error)?;
    let decision_sha256 = hash_asset_file(&path).map_err(display_error)?;
    if accept {
        if let AutomationOperation::GenerateCharacterPack(request) = &operation {
            if request.workflow.id == "topdown-direction-motion"
                && request.workflow.version == "8.0.0"
                && request.direction_motion_stage == DirectionMotionGenerationStageV1::ImageLocks
            {
                let lock_path = record.job_dir.join("source/direction-motion-lock.json");
                let lock: DirectionMotionLockV1 =
                    serde_json::from_slice(&fs::read(&lock_path).map_err(io_error)?)
                        .map_err(json_error)?;
                if lock.nodes.len() != 8
                    || lock.nodes.iter().any(|node| {
                        matches!(
                            node.feedback.verdict,
                            ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
                        )
                    })
                    || ["walk_down", "walk_up", "walk_right", "walk_left"]
                        .iter()
                        .any(|animation| lock.video_input(animation).is_err())
                {
                    return Err((
                        "hard_failure_not_reviewable".into(),
                        "the direction-motion lock is incomplete or contains a blocked/regenerate node"
                            .into(),
                    ));
                }
                let lock_sha256 = hash_asset_file(&lock_path).map_err(display_error)?;
                let approval = DirectionMotionApprovalV1 {
                    schema_version: "1".into(),
                    profile: DIRECTION_MOTION_APPROVAL_PROFILE.into(),
                    source_job_id: id.into(),
                    lock_sha256,
                    node_sha256: lock
                        .nodes
                        .iter()
                        .map(|node| (node.node_id.clone(), node.sha256.clone()))
                        .collect(),
                    accepted: true,
                    reason: reason.into(),
                    reviewed_at,
                };
                let approval_path = record.job_dir.join(DIRECTION_MOTION_APPROVAL_FILE);
                fs::write(
                    &approval_path,
                    serde_json::to_vec_pretty(&approval).map_err(json_error)?,
                )
                .map_err(io_error)?;
                let approval_sha256 = hash_asset_file(&approval_path).map_err(display_error)?;
                let updated = store
                    .update_record(id, |record| {
                        record.artifacts.extend([
                            JobArtifactRecord {
                                kind: "review_decision".into(),
                                path: path.clone(),
                                sha256: Some(decision_sha256.clone()),
                            },
                            JobArtifactRecord {
                                kind: "character_direction_motion_approval".into(),
                                path: approval_path.clone(),
                                sha256: Some(approval_sha256.clone()),
                            },
                        ]);
                        record.state = forge_core::job::JobState::QualityChecked;
                        record.lifecycle_state = forge_core::job::JobLifecycleState::Succeeded;
                        record.error_code = None;
                        record.error_summary = None;
                        record.recoverable = false;
                        record.next_actions = vec![
                            "generate_direction_motion_videos".into(),
                            "job_report".into(),
                        ];
                    })
                    .map_err(display_error)?;
                return success(&updated);
            }
        }
        if let Some(lock) = validated_direction_grid_lock.as_ref() {
            let lock_path = record.job_dir.join("source/direction-grid-lock.json");
            let lock_sha256 = hash_asset_file(&lock_path).map_err(display_error)?;
            let approval = DirectionGridApprovalV1 {
                schema_version: "1".into(),
                profile: GRID_APPROVAL_PROFILE.into(),
                source_job_id: id.into(),
                lock_sha256,
                node_sha256: lock.node_hashes(),
                accepted: true,
                reason: reason.into(),
                reviewed_at,
            };
            let approval_path = record.job_dir.join(GRID_APPROVAL_FILE);
            fs::write(
                &approval_path,
                serde_json::to_vec_pretty(&approval).map_err(json_error)?,
            )
            .map_err(io_error)?;
            let approval_sha256 = hash_asset_file(&approval_path).map_err(display_error)?;
            let updated = store
                .update_record(id, |record| {
                    record.artifacts.extend([
                        JobArtifactRecord {
                            kind: "review_decision".into(),
                            path: path.clone(),
                            sha256: Some(decision_sha256.clone()),
                        },
                        JobArtifactRecord {
                            kind: "direction_grid_approval".into(),
                            path: approval_path.clone(),
                            sha256: Some(approval_sha256.clone()),
                        },
                    ]);
                    record.state = forge_core::job::JobState::QualityChecked;
                    record.lifecycle_state = forge_core::job::JobLifecycleState::Succeeded;
                    record.error_code = None;
                    record.error_summary = None;
                    record.recoverable = false;
                    record.next_actions = vec!["generate_grid_actions".into(), "job_report".into()];
                })
                .map_err(display_error)?;
            return success(&updated);
        }
        if let AutomationOperation::GenerateStaticAssetSet(request) = &operation {
            if request.portrait_phase == PortraitGenerationPhaseV1::BaseOnly {
                let report: ConsistencyReportV1 = serde_json::from_slice(
                    &fs::read(record.job_dir.join("consistency-report.json")).map_err(io_error)?,
                )
                .map_err(json_error)?;
                let portrait_report: PortraitConsistencyReportV1 = serde_json::from_slice(
                    &fs::read(record.job_dir.join(PORTRAIT_CONSISTENCY_REPORT_FILE))
                        .map_err(io_error)?,
                )
                .map_err(json_error)?;
                let hard_base_failure = report.items.iter().any(|item| {
                    matches!(
                        item.verdict,
                        ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
                    )
                }) || portrait_report.items.iter().any(|item| {
                    matches!(
                        item.verdict,
                        ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
                    )
                });
                if hard_base_failure {
                    return Err((
                        "hard_failure_not_reviewable".into(),
                        "blocked or regenerate neutral PortraitBase cannot be approved".into(),
                    ));
                }
                let (approval_path, _approval) =
                    approve_portrait_base(id, &record.job_dir, reason, reviewed_at)
                        .map_err(display_error)?;
                let approval_sha256 = hash_asset_file(&approval_path).map_err(display_error)?;
                let updated = store
                    .update_record(id, |record| {
                        record.artifacts.push(JobArtifactRecord {
                            kind: "review_decision".into(),
                            path: path.clone(),
                            sha256: Some(decision_sha256.clone()),
                        });
                        record.artifacts.push(JobArtifactRecord {
                            kind: "portrait_base_approval".into(),
                            path: approval_path.clone(),
                            sha256: Some(approval_sha256.clone()),
                        });
                        record.state = forge_core::job::JobState::QualityChecked;
                        record.lifecycle_state = forge_core::job::JobLifecycleState::Succeeded;
                        record.error_code = None;
                        record.error_summary = None;
                        record.recoverable = false;
                        record.next_actions =
                            vec!["generate_portrait_expressions".into(), "job_report".into()];
                    })
                    .map_err(display_error)?;
                return success(&updated);
            }
        }
        if let Some(candidate) = record
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == "candidate_gsfpack")
            .map(|artifact| artifact.path.clone())
        {
            let updated = store
                .update_record(id, |record| {
                    if let Some(artifact) = record
                        .artifacts
                        .iter_mut()
                        .find(|artifact| artifact.kind == "candidate_gsfpack")
                    {
                        artifact.kind = "gsfpack".into();
                    }
                    record.artifacts.push(JobArtifactRecord {
                        kind: "review_decision".into(),
                        path: path.clone(),
                        sha256: Some(decision_sha256.clone()),
                    });
                    record.lifecycle_state = forge_core::job::JobLifecycleState::Succeeded;
                    record.state = forge_core::job::JobState::Exported;
                    record.error_code = None;
                    record.error_summary = None;
                    record.recoverable = false;
                    record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
                })
                .map_err(display_error)?;
            forge_pack::validate_pack_layout(&candidate).map_err(display_error)?;
            return success(&updated);
        }
        if let AutomationOperation::GenerateStaticAssetSet(request) = operation.clone() {
            let report_path = record.job_dir.join("consistency-report.json");
            let mut report: ConsistencyReportV1 =
                serde_json::from_slice(&fs::read(&report_path).map_err(io_error)?)
                    .map_err(json_error)?;
            if report.items.iter().any(|item| {
                matches!(
                    item.verdict,
                    ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
                )
            }) {
                return Err((
                    "hard_failure_not_reviewable".into(),
                    "blocked or regenerate consistency results cannot be manually accepted".into(),
                ));
            }
            report.verdict = ConsistencyVerdict::GameReady;
            for item in &mut report.items {
                if item.verdict == ConsistencyVerdict::AwaitingReview {
                    item.verdict = ConsistencyVerdict::GameReady;
                    item.reasons.push("accepted_by_review".into());
                }
            }
            let collection_report_path = record.job_dir.join("collection-consistency-report.json");
            let mut collection_report = if collection_report_path.is_file() {
                Some(
                    serde_json::from_slice::<CollectionConsistencyReportV1>(
                        &fs::read(&collection_report_path).map_err(io_error)?,
                    )
                    .map_err(json_error)?,
                )
            } else {
                None
            };
            if collection_report.as_ref().is_some_and(|report| {
                report.items.iter().any(|item| {
                    matches!(
                        item.verdict,
                        CollectionItemVerdict::Blocked | CollectionItemVerdict::Regenerate
                    )
                })
            }) {
                return Err((
                    "hard_failure_not_reviewable".into(),
                    "blocked or regenerate collection results cannot be manually accepted".into(),
                ));
            }
            if let Some(collection_report) = &mut collection_report {
                collection_report.verdict = CollectionItemVerdict::GameReady;
                for item in &mut collection_report.items {
                    if item.verdict == CollectionItemVerdict::AwaitingReview {
                        item.verdict = CollectionItemVerdict::GameReady;
                        item.reasons.push("accepted_by_review".into());
                    }
                }
            }
            let portrait_base_lock_path = record
                .job_dir
                .join("portrait-base")
                .join(PORTRAIT_BASE_LOCK_FILE);
            let portrait_base = portrait_base_lock_path
                .is_file()
                .then(|| read_portrait_base_lock(&portrait_base_lock_path))
                .transpose()
                .map_err(display_error)?;
            let portrait_approval =
                if request.portrait_phase == PortraitGenerationPhaseV1::Expressions {
                    let source = request.reuse_from_job_dir.as_deref().ok_or_else(|| {
                        (
                            "portrait_base_source_missing".into(),
                            "Portrait expression Job has no reusable source".into(),
                        )
                    })?;
                    let base_job_id =
                        request
                            .portrait_base_parent_job_id
                            .as_deref()
                            .ok_or_else(|| {
                                (
                                    "portrait_base_job_required".into(),
                                    "Portrait expression Job has no approved base parent".into(),
                                )
                            })?;
                    let jobs_root = source.parent().ok_or_else(|| {
                        (
                            "portrait_base_source_invalid".into(),
                            "Portrait source is not inside a JobStore".into(),
                        )
                    })?;
                    let (_lock, approval) =
                        validate_portrait_base_approval(&jobs_root.join(base_job_id), base_job_id)
                            .map_err(display_error)?;
                    Some(approval)
                } else {
                    None
                };
            let portrait_report_path = record.job_dir.join(PORTRAIT_CONSISTENCY_REPORT_FILE);
            let mut portrait_report = if portrait_report_path.is_file() {
                Some(
                    serde_json::from_slice::<PortraitConsistencyReportV1>(
                        &fs::read(&portrait_report_path).map_err(io_error)?,
                    )
                    .map_err(json_error)?,
                )
            } else {
                None
            };
            if portrait_report.as_ref().is_some_and(|report| {
                report.items.iter().any(|item| {
                    matches!(
                        item.verdict,
                        ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
                    )
                })
            }) {
                return Err((
                    "hard_failure_not_reviewable".into(),
                    "blocked or regenerate Portrait-local results cannot be manually accepted"
                        .into(),
                ));
            }
            if let Some(portrait_report) = &mut portrait_report {
                portrait_report.verdict = ConsistencyVerdict::GameReady;
                for item in &mut portrait_report.items {
                    if item.verdict == ConsistencyVerdict::AwaitingReview {
                        item.verdict = ConsistencyVerdict::GameReady;
                        item.reasons.push("accepted_by_review".into());
                    }
                }
            }
            let style = read_style_lock(&request.style_lock_path).map_err(display_error)?;
            let collection = request
                .collection_lock_path
                .as_deref()
                .map(read_collection_lock)
                .transpose()
                .map_err(display_error)?;
            let items = request
                .asset
                .items
                .iter()
                .map(|item| StaticPackItem {
                    id: item.id.clone(),
                    name: item.name.clone(),
                    image_path: record
                        .job_dir
                        .join("normalized/static")
                        .join(format!("{}.png", item.id)),
                })
                .collect::<Vec<_>>();
            if items.iter().any(|item| !item.image_path.is_file()) {
                return Err((
                    "review_material_missing".into(),
                    "reviewed item output is missing; run targeted retry".into(),
                ));
            }
            let manual_item_ids = request
                .replacement_item_paths
                .keys()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>();
            let output = export_static_pack(
                &record.job_dir.join("exports"),
                &request.asset,
                &style,
                &items,
                &report,
                StaticPackContext {
                    provider_id: &request.provider_id,
                    item_metadata: &request.item_metadata,
                    collection: collection.as_ref(),
                    manual_item_ids: &manual_item_ids,
                    portrait_base: portrait_base.as_ref(),
                    portrait_approval: portrait_approval.as_ref(),
                    portrait_report: portrait_report.as_ref(),
                },
            )
            .map_err(display_error)?;
            if let Some(collection_report) = &collection_report {
                fs::write(
                    output.pack_dir.join("collection-consistency-report.json"),
                    serde_json::to_vec_pretty(collection_report).map_err(json_error)?,
                )
                .map_err(io_error)?;
            }
            if let Some(lock) = &collection {
                fs::write(
                    output.pack_dir.join("collection-lock-ref.json"),
                    serde_json::to_vec_pretty(&serde_json::json!({
                        "schemaVersion": "1",
                        "id": lock.id,
                        "revision": lock.revision,
                        "assetKind": lock.asset_kind,
                        "styleRevision": lock.style_revision,
                        "styleBoardSha256": lock.style_board_sha256,
                        "anchorSha256": lock.anchor_sha256,
                        "medoidSha256": lock.medoid_sha256,
                        "outlierProfile": lock.outlier_profile,
                        "license": lock.license,
                    }))
                    .map_err(json_error)?,
                )
                .map_err(io_error)?;
            }
            let pack_sha256 = hash_directory(&output.pack_dir).map_err(io_error)?;
            forge_pack::validate_pack_layout(&output.pack_dir).map_err(display_error)?;
            let catalog_path = register_static_catalog_output(
                &request,
                id,
                record.parent_job_id.clone(),
                output.pack_dir.clone(),
                pack_sha256.clone(),
                Some(CatalogReviewRefV1 {
                    status: CatalogReviewStatusV1::Approved,
                    reason: reason.into(),
                    reviewed_at,
                }),
            )
            .map_err(display_error)?;
            fs::write(
                &report_path,
                serde_json::to_vec_pretty(&report).map_err(json_error)?,
            )
            .map_err(io_error)?;
            if let Some(collection_report) = &collection_report {
                fs::write(
                    &collection_report_path,
                    serde_json::to_vec_pretty(collection_report).map_err(json_error)?,
                )
                .map_err(io_error)?;
            }
            if let Some(portrait_report) = &portrait_report {
                fs::write(
                    &portrait_report_path,
                    serde_json::to_vec_pretty(portrait_report).map_err(json_error)?,
                )
                .map_err(io_error)?;
            }
            let report_sha256 = hash_asset_file(&report_path).map_err(display_error)?;
            let collection_report_sha256 = collection_report
                .as_ref()
                .map(|_| hash_asset_file(&collection_report_path).map_err(display_error))
                .transpose()?;
            let portrait_report_sha256 = portrait_report
                .as_ref()
                .map(|_| hash_asset_file(&portrait_report_path).map_err(display_error))
                .transpose()?;
            let catalog_sha256 = hash_asset_file(&catalog_path).map_err(display_error)?;
            let updated = store
                .update_record(id, |record| {
                    if let Some(artifact) = record
                        .artifacts
                        .iter_mut()
                        .find(|artifact| artifact.kind == "consistency_report")
                    {
                        artifact.sha256 = Some(report_sha256.clone());
                    }
                    if let Some(sha256) = &collection_report_sha256 {
                        if let Some(artifact) = record
                            .artifacts
                            .iter_mut()
                            .find(|artifact| artifact.kind == "collection_consistency_report")
                        {
                            artifact.sha256 = Some(sha256.clone());
                        }
                    }
                    if let Some(sha256) = &portrait_report_sha256 {
                        if let Some(artifact) = record
                            .artifacts
                            .iter_mut()
                            .find(|artifact| artifact.kind == "portrait_consistency_report")
                        {
                            artifact.sha256 = Some(sha256.clone());
                        }
                    }
                    record.artifacts.push(JobArtifactRecord {
                        kind: "review_decision".into(),
                        path: path.clone(),
                        sha256: Some(decision_sha256.clone()),
                    });
                    record.artifacts.push(JobArtifactRecord {
                        kind: "gsfpack".into(),
                        path: output.pack_dir.clone(),
                        sha256: Some(pack_sha256.clone()),
                    });
                    record.artifacts.push(JobArtifactRecord {
                        kind: "project_catalog".into(),
                        path: catalog_path.clone(),
                        sha256: Some(catalog_sha256.clone()),
                    });
                    record.lifecycle_state = forge_core::job::JobLifecycleState::Succeeded;
                    record.state = forge_core::job::JobState::Exported;
                    record.error_code = None;
                    record.error_summary = None;
                    record.recoverable = false;
                    record.next_actions = vec!["inspect_asset".into(), "plan_install_godot".into()];
                })
                .map_err(display_error)?;
            return success(&updated);
        }
    }
    let quarantine_catalog =
        if !accept && record.lifecycle_state == forge_core::job::JobLifecycleState::Succeeded {
            if let AutomationOperation::GenerateStaticAssetSet(request) = &operation {
                let catalog_path = set_catalog_review(
                    &request.project_path,
                    &request.asset.id,
                    CatalogReviewStatusV1::Quarantined,
                    reason,
                )
                .map_err(display_error)?;
                Some((
                    catalog_path.clone(),
                    hash_asset_file(&catalog_path).map_err(display_error)?,
                ))
            } else {
                None
            }
        } else {
            None
        };
    let updated = store
        .update_record(id, |record| {
            record.artifacts.push(JobArtifactRecord {
                kind: "review_decision".into(),
                path: path.clone(),
                sha256: Some(decision_sha256.clone()),
            });
            record.error_summary =
                Some("review rejected; asset is quarantined and regeneration is required".into());
            record.error_code = Some("manual_rejected".into());
            record.recoverable = true;
            if let Some((catalog_path, sha256)) = &quarantine_catalog {
                record.artifacts.push(JobArtifactRecord {
                    kind: "project_catalog".into(),
                    path: catalog_path.clone(),
                    sha256: Some(sha256.clone()),
                });
            }
            record.next_actions = vec![
                "retry_item".into(),
                "job_report".into(),
                "project_audit".into(),
            ];
        })
        .map_err(display_error)?;
    success(&updated)
}

fn review_local_footwear_cleanup(
    store: &JobStore,
    record: &JobRecord,
    accept: bool,
    reason: &str,
) -> Result<(), (String, String)> {
    if record.lifecycle_state != JobLifecycleState::AwaitingReview {
        return Err((
            "job_not_awaiting_review".into(),
            "local footwear cleanup Job is not awaiting review".into(),
        ));
    }
    let parent_id = record.parent_job_id.as_deref().ok_or_else(|| {
        (
            "footwear_cleanup_provenance_invalid".into(),
            "local footwear cleanup Job has no parent".into(),
        )
    })?;
    let parent = store.read_record(parent_id).map_err(display_error)?;
    let source = forge_core::grid_retry_source::validate_v94_footwear_cleanup_source_closure(
        &parent.job_dir,
        &[2],
    )
    .map_err(display_error)?;
    let artifact_path = |kind: &str| -> Result<PathBuf, (String, String)> {
        let matches = record
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == kind)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err((
                "footwear_cleanup_provenance_invalid".into(),
                format!("expected exactly one {kind} artifact"),
            ));
        }
        let artifact = matches[0];
        let path = validate_job_file(record, &artifact.path)?;
        let expected = artifact.sha256.as_deref().ok_or_else(|| {
            (
                "footwear_cleanup_provenance_invalid".into(),
                format!("{kind} artifact has no SHA-256"),
            )
        })?;
        if hash_asset_file(&path).map_err(display_error)? != expected {
            return Err((
                "footwear_cleanup_provenance_invalid".into(),
                format!("{kind} artifact changed after generation"),
            ));
        }
        Ok(path)
    };
    let manifest_path = artifact_path("local_cleanup_manifest")?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(io_error)?).map_err(json_error)?;
    let repair_path = artifact_path("footwear_cleanup_report")?;
    let repair: serde_json::Value =
        serde_json::from_slice(&fs::read(&repair_path).map_err(io_error)?).map_err(json_error)?;
    let usage_path = artifact_path("provider_usage")?;
    let usage: serde_json::Value =
        serde_json::from_slice(&fs::read(&usage_path).map_err(io_error)?).map_err(json_error)?;
    let modified_pixels = repair
        .get("modifiedPixels")
        .and_then(serde_json::Value::as_u64)
        .filter(|count| *count > 0)
        .ok_or_else(|| {
            (
                "footwear_cleanup_provenance_invalid".into(),
                "repair report has no positive modified-pixel count".into(),
            )
        })?;
    let repair_bounds = repair
        .get("bounds")
        .filter(|bounds| bounds.as_array().is_some_and(|values| values.len() == 4))
        .cloned()
        .ok_or_else(|| {
            (
                "footwear_cleanup_provenance_invalid".into(),
                "repair report has no four-value repair bounds".into(),
            )
        })?;
    if manifest.get("profile").and_then(serde_json::Value::as_str)
        != Some("grid-local-footwear-cleanup@1.0.0")
        || manifest
            .get("sourceJobId")
            .and_then(serde_json::Value::as_str)
            != Some(source.job.job_id.as_str())
        || manifest
            .get("providerRequestOccurred")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
        || manifest
            .get("providerRequestCount")
            .and_then(serde_json::Value::as_u64)
            != Some(0)
        || repair.get("verdict").and_then(serde_json::Value::as_str) != Some("game_ready")
        || repair
            .get("nonMaskPixelsUnchanged")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        || manifest
            .get("modifiedPixels")
            .and_then(serde_json::Value::as_u64)
            != Some(modified_pixels)
        || manifest.get("repairBounds") != Some(&repair_bounds)
        || usage
            .pointer("/usage/requests")
            .and_then(serde_json::Value::as_u64)
            != Some(0)
        || usage
            .pointer("/usage/generatedImages")
            .and_then(serde_json::Value::as_u64)
            != Some(0)
        || usage
            .pointer("/usage/generatedVideos")
            .and_then(serde_json::Value::as_u64)
            != Some(0)
        || usage
            .pointer("/usage/costInUsdTicks")
            .and_then(serde_json::Value::as_u64)
            != Some(0)
    {
        return Err((
            "footwear_cleanup_provenance_invalid".into(),
            "local cleanup manifest, repair mask, or zero-request usage contract is invalid".into(),
        ));
    }

    let mut frames = Vec::with_capacity(4);
    for index in 0..4_usize {
        let path = artifact_path(&format!("grid_keyframe_walk_down_{index}"))?;
        let sha256 = hash_asset_file(&path).map_err(display_error)?;
        if index == 2 {
            if sha256
                != manifest
                    .get("outputFrameSha256")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                || source.report.frames[index].sha256
                    != manifest
                        .get("sourceFrameSha256")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                || sha256 == source.report.frames[index].sha256
            {
                return Err((
                    "footwear_cleanup_provenance_invalid".into(),
                    "frame 2 before/after SHA-256 binding is invalid".into(),
                ));
            }
        } else if sha256 != source.report.frames[index].sha256 {
            return Err((
                "footwear_cleanup_provenance_invalid".into(),
                format!("byte-reused frame {index} changed"),
            ));
        }
        frames.push(image::open(path).map_err(display_error)?.to_rgba8());
    }
    let animations = BTreeMap::from([("walk_down".into(), frames.clone())]);
    let motion = assess_character_motion_semantics(&animations);
    let laterality = assess_front_gait_laterality("walk_down", &frames);
    let equipment =
        assess_hand_equipment_contact_with_kind(&animations, CharacterEquipmentKindV1::None);
    let footwear = assess_footwear_platform_action("walk_down", &frames);
    if [
        motion.verdict,
        laterality.verdict,
        equipment.verdict,
        footwear.verdict,
    ]
    .iter()
    .any(|verdict| *verdict != ConsistencyVerdict::GameReady)
    {
        return Err((
            "hard_failure_not_reviewable".into(),
            "current local footwear cleanup pixels fail a deterministic quality gate".into(),
        ));
    }

    let decision_path = record.job_dir.join("review-decision.json");
    fs::write(
        &decision_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schemaVersion": "1",
            "accepted": accept,
            "reason": reason,
            "reviewedAt": chrono::Utc::now(),
            "profile": "grid-local-footwear-cleanup-review@1.0.0",
            "providerRequestCount": 0,
            "modifiedPixels": modified_pixels,
            "repairBounds": repair_bounds
        }))
        .map_err(json_error)?,
    )
    .map_err(io_error)?;
    let decision_sha256 = hash_asset_file(&decision_path).map_err(display_error)?;
    let updated = store
        .update_record(&record.job_id, |record| {
            record.artifacts.push(JobArtifactRecord {
                kind: "review_decision".into(),
                path: decision_path.clone(),
                sha256: Some(decision_sha256.clone()),
            });
            if accept {
                record.state = JobState::QualityChecked;
                record.lifecycle_state = JobLifecycleState::Succeeded;
                record.error_code = None;
                record.error_summary = None;
                record.recoverable = false;
                record.next_actions = vec!["job_report".into()];
            } else {
                record.state = JobState::Failed;
                record.lifecycle_state = JobLifecycleState::AwaitingReview;
                record.error_code = Some("manual_rejected".into());
                record.error_summary =
                    Some("review rejected; deterministic cleanup is quarantined".into());
                record.recoverable = false;
                record.next_actions = vec!["job_report".into()];
            }
        })
        .map_err(display_error)?;
    success(&updated)
}

fn validate_direction_grid_review_candidate(
    record: &JobRecord,
) -> Result<DirectionGridLockV1, (String, String)> {
    let lock_path = record.job_dir.join("source/direction-grid-lock.json");
    validate_job_file(record, &lock_path)?;
    let lock: DirectionGridLockV1 =
        serde_json::from_slice(&fs::read(&lock_path).map_err(io_error)?).map_err(json_error)?;
    let appearance_path = lock.appearance_report_path.as_ref().ok_or_else(|| {
        (
            "hard_failure_not_reviewable".into(),
            "the direction grid has no bound semantic appearance report".into(),
        )
    })?;
    validate_job_file(record, appearance_path)?;
    let appearance_sha256 = hash_asset_file(appearance_path).map_err(display_error)?;
    if lock.appearance_report_sha256.as_deref() != Some(appearance_sha256.as_str()) {
        return Err((
            "hard_failure_not_reviewable".into(),
            "the direction-grid semantic report changed after generation".into(),
        ));
    }
    let appearance: DirectionGridAppearanceReportV1 =
        serde_json::from_slice(&fs::read(appearance_path).map_err(io_error)?)
            .map_err(json_error)?;
    if lock.nodes.len() != 4
        || ["front_idle", "back_idle", "right_idle", "left_idle"]
            .iter()
            .any(|node_id| lock.node(node_id).is_none())
        || appearance.profile != DIRECTION_GRID_APPEARANCE_PROFILE
        || matches!(
            appearance.verdict,
            ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
        )
    {
        return Err((
            "hard_failure_not_reviewable".into(),
            "the direction grid lock is incomplete or contains a semantic hard failure".into(),
        ));
    }
    match (
        lock.cape_hem_contract,
        lock.cape_hem_report_path.as_ref(),
        lock.cape_hem_report_sha256.as_deref(),
    ) {
        (None, None, None) => {}
        (Some(contract), Some(path), Some(expected_sha256)) => {
            validate_job_file(record, path)?;
            let actual_sha256 = hash_asset_file(path).map_err(display_error)?;
            let report: CapeHemConsistencyReportV1 =
                serde_json::from_slice(&fs::read(path).map_err(io_error)?).map_err(json_error)?;
            if actual_sha256 != expected_sha256
                || report.profile != CAPE_HEM_CONSISTENCY_PROFILE
                || report.contract != contract
                || report.nodes.len() != 4
                || report.verdict != ConsistencyVerdict::GameReady
            {
                return Err((
                    "hard_failure_not_reviewable".into(),
                    "the direction grid fails its bound cape topology contract".into(),
                ));
            }
            for evidence in &report.nodes {
                let node = lock.node(&evidence.node_id).ok_or_else(|| {
                    (
                        "hard_failure_not_reviewable".into(),
                        format!("cape topology node {} is missing", evidence.node_id),
                    )
                })?;
                if evidence.path != node.path
                    || evidence.sha256 != node.sha256
                    || evidence.verdict != ConsistencyVerdict::GameReady
                    || hash_asset_file(&node.path).map_err(display_error)? != node.sha256
                {
                    return Err((
                        "hard_failure_not_reviewable".into(),
                        format!("cape topology evidence changed for {}", node.node_id),
                    ));
                }
            }
            if contract == CapeHemContractV1::FrontAuthoritativeNoSkirtHem {
                let producer = lock.producer.as_ref().ok_or_else(|| {
                    (
                        "hard_failure_not_reviewable".into(),
                        "front-authoritative grid has no Provider producer evidence".into(),
                    )
                })?;
                let downstream = lock.downstream_binding.as_ref().ok_or_else(|| {
                    (
                        "hard_failure_not_reviewable".into(),
                        "front-authoritative grid has no downstream binding".into(),
                    )
                })?;
                let evidence_path = producer.evidence_path.as_ref().ok_or_else(|| {
                    (
                        "hard_failure_not_reviewable".into(),
                        "front-authoritative grid has no source evidence".into(),
                    )
                })?;
                validate_job_file(record, evidence_path)?;
                let evidence_sha256 = hash_asset_file(evidence_path).map_err(display_error)?;
                let evidence: serde_json::Value =
                    serde_json::from_slice(&fs::read(evidence_path).map_err(io_error)?)
                        .map_err(json_error)?;
                if producer.kind != DirectionGridProducerKindV1::ProviderGeneration
                    || !producer.provider_request_occurred
                    || producer.evidence_sha256.as_deref() != Some(evidence_sha256.as_str())
                    || downstream.provider_id != lock.provider_id
                    || downstream.profile_id != lock.profile_id
                    || downstream.image_model != lock.image_model
                    || evidence.get("profile").and_then(serde_json::Value::as_str)
                        != Some("front-authoritative-direction-grid@1.0.0")
                    || evidence
                        .get("authoritySha256")
                        .and_then(serde_json::Value::as_str)
                        != Some(producer.input_sha256.as_str())
                {
                    return Err((
                        "hard_failure_not_reviewable".into(),
                        "front-authoritative Provider/source evidence is incomplete or changed"
                            .into(),
                    ));
                }
            }
        }
        _ => {
            return Err((
                "hard_failure_not_reviewable".into(),
                "the direction grid has incomplete cape topology evidence".into(),
            ));
        }
    }
    Ok(lock)
}

fn validate_imported_direction_grid_review_candidate(
    record: &JobRecord,
    request: &ImportDirectionGridRequestV1,
) -> Result<DirectionGridLockV1, (String, String)> {
    if record.operation_kind != JobOperationKind::ImportDirectionGrid
        || record.authorization_id.is_some()
        || record.parent_job_id.is_none()
    {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "import review requires an authorization-free imported DirectionGrid child".into(),
        ));
    }
    let artifact_path = |kind: &str| -> Result<PathBuf, (String, String)> {
        let matches = record
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == kind)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err((
                "direction_grid_import_provenance_invalid".into(),
                format!("expected exactly one {kind} artifact"),
            ));
        }
        let artifact = matches[0];
        let path = validate_job_file(record, &artifact.path)?;
        let expected = artifact.sha256.as_deref().ok_or_else(|| {
            (
                "direction_grid_import_provenance_invalid".into(),
                format!("{kind} artifact has no SHA-256"),
            )
        })?;
        if hash_asset_file(&path).map_err(display_error)? != expected {
            return Err((
                "direction_grid_import_provenance_invalid".into(),
                format!("{kind} artifact changed after import"),
            ));
        }
        Ok(path)
    };

    let lock = validate_direction_grid_review_candidate(record)?;
    let Some(producer) = lock.producer.as_ref() else {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "imported DirectionGrid Lock has no typed producer provenance".into(),
        ));
    };
    let Some(downstream) = lock.downstream_binding.as_ref() else {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "imported DirectionGrid Lock has no separated downstream binding".into(),
        ));
    };
    if lock.profile != DIRECTION_GRID_IMPORT_LOCK_PROFILE
        || producer.kind != DirectionGridProducerKindV1::ExternalImport
        || producer.generator != request.generator
        || producer.provider_request_occurred
        || downstream.provider_id != lock.provider_id
        || downstream.profile_id != lock.profile_id
        || downstream.image_model != lock.image_model
    {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "producer provenance and downstream Provider binding are not separated correctly"
                .into(),
        ));
    }
    if lock.nodes.iter().any(|node| {
        node.provider_request_occurred
            || hash_asset_file(&node.path).ok().as_deref() != Some(node.sha256.as_str())
            || hash_asset_file(&node.generation_master_path)
                .ok()
                .as_deref()
                != Some(node.generation_master_sha256.as_str())
            || validate_job_file(record, &node.path).is_err()
            || validate_job_file(record, &node.generation_master_path).is_err()
    }) {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "imported node paths, hashes or zero-request flags are invalid".into(),
        ));
    }

    let evidence_path = artifact_path("direction_grid_import_evidence")?;
    let evidence: serde_json::Value =
        serde_json::from_slice(&fs::read(&evidence_path).map_err(io_error)?).map_err(json_error)?;
    let original_path = artifact_path("direction_grid_import_original")?;
    let matted_path = artifact_path("direction_grid_import_matted")?;
    let matting_path = artifact_path("checkerboard_matting_report")?;
    let matted_alpha_edge_path = artifact_path("alpha_edge_halo_matted_report")?;
    let alpha_edge_path = artifact_path("alpha_edge_halo_report")?;
    let consistency_path = artifact_path("direction_grid_import_consistency_report")?;
    let authority_alignment_path = artifact_path("direction_grid_authority_alignment_report")?;
    let _review_package_path = artifact_path("direction_grid_native_review_package")?;
    let alignment_path = artifact_path("direction_grid_alignment_report")?;
    let usage_path = artifact_path("provider_usage")?;
    let manifest_path = artifact_path("provider_manifest")?;
    let source_job: JobRecord = serde_json::from_slice(
        &fs::read(request.source_job_dir.join("job.json")).map_err(io_error)?,
    )
    .map_err(json_error)?;
    let source_lock_path = request
        .source_job_dir
        .join("source/direction-grid-lock.json");
    let source_lock_sha256 = hash_asset_file(&source_lock_path).map_err(display_error)?;
    let original_sha256 = hash_asset_file(&original_path).map_err(display_error)?;
    let matted_sha256 = hash_asset_file(&matted_path).map_err(display_error)?;
    let matting_sha256 = hash_asset_file(&matting_path).map_err(display_error)?;
    let matted_alpha_edge_sha256 =
        hash_asset_file(&matted_alpha_edge_path).map_err(display_error)?;
    let alpha_edge_sha256 = hash_asset_file(&alpha_edge_path).map_err(display_error)?;
    let consistency_sha256 = hash_asset_file(&consistency_path).map_err(display_error)?;
    let alignment_sha256 = hash_asset_file(&alignment_path).map_err(display_error)?;
    let authority_alignment_sha256 =
        hash_asset_file(&authority_alignment_path).map_err(display_error)?;
    if source_job.lifecycle_state != JobLifecycleState::Succeeded
        || request
            .source_job_dir
            .file_name()
            .and_then(|value| value.to_str())
            != Some(source_job.job_id.as_str())
        || evidence.get("profile").and_then(serde_json::Value::as_str)
            != Some("direction-grid-import@1.1.0")
        || evidence
            .get("generator")
            .and_then(serde_json::Value::as_str)
            != Some(request.generator.as_str())
        || evidence.get("note").and_then(serde_json::Value::as_str) != Some(request.note.as_str())
        || evidence
            .get("sourceJobId")
            .and_then(serde_json::Value::as_str)
            != Some(source_job.job_id.as_str())
        || record.parent_job_id.as_deref() != Some(source_job.job_id.as_str())
        || evidence
            .get("sourceLockSha256")
            .and_then(serde_json::Value::as_str)
            != Some(source_lock_sha256.as_str())
        || evidence
            .get("inputSheetSha256")
            .and_then(serde_json::Value::as_str)
            != Some(original_sha256.as_str())
        || evidence
            .get("materializedOriginalSha256")
            .and_then(serde_json::Value::as_str)
            != Some(original_sha256.as_str())
        || evidence
            .get("mattedSheetSha256")
            .and_then(serde_json::Value::as_str)
            != Some(matted_sha256.as_str())
        || evidence
            .get("mattingReportSha256")
            .and_then(serde_json::Value::as_str)
            != Some(matting_sha256.as_str())
        || evidence
            .get("mattedAlphaEdgeReportSha256")
            .and_then(serde_json::Value::as_str)
            != Some(matted_alpha_edge_sha256.as_str())
        || evidence
            .get("alphaEdgeReportSha256")
            .and_then(serde_json::Value::as_str)
            != Some(alpha_edge_sha256.as_str())
        || evidence
            .get("consistencyReportSha256")
            .and_then(serde_json::Value::as_str)
            != Some(consistency_sha256.as_str())
        || evidence
            .get("alignmentReportSha256")
            .and_then(serde_json::Value::as_str)
            != Some(alignment_sha256.as_str())
        || evidence
            .get("authorityAlignmentReportSha256")
            .and_then(serde_json::Value::as_str)
            != Some(authority_alignment_sha256.as_str())
        || evidence
            .get("providerRequestOccurred")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
        || lock.sheet_sha256 != original_sha256
        || producer.input_sha256 != original_sha256
        || producer.matted_sha256.as_deref() != Some(matted_sha256.as_str())
        || producer.evidence_path.as_deref() != Some(evidence_path.as_path())
        || producer.evidence_sha256.as_deref()
            != Some(
                hash_asset_file(&evidence_path)
                    .map_err(display_error)?
                    .as_str(),
            )
        || lock.import_consistency_report_path.as_deref() != Some(consistency_path.as_path())
        || lock.import_consistency_report_sha256.as_deref() != Some(consistency_sha256.as_str())
        || lock.alpha_edge_report_path.as_deref() != Some(alpha_edge_path.as_path())
        || lock.alpha_edge_report_sha256.as_deref() != Some(alpha_edge_sha256.as_str())
    {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "import evidence no longer binds the source, input, matting output or recipe".into(),
        ));
    }
    let expected_mode = if request.sheet_path.is_some() {
        "sheet"
    } else {
        "four_files"
    };
    let evidence_inputs = evidence
        .get("inputs")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            (
                "direction_grid_import_provenance_invalid".into(),
                "import evidence has no ordered input closure".into(),
            )
        })?;
    let request_inputs = request.ordered_input_paths();
    if evidence
        .get("inputMode")
        .and_then(serde_json::Value::as_str)
        != Some(expected_mode)
        || evidence_inputs.len() != request_inputs.len()
    {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "import input mode or ordered input count changed".into(),
        ));
    }
    for ((role, source_path), evidence_input) in request_inputs.iter().zip(evidence_inputs) {
        let canonical_source_path = source_path.canonicalize().map_err(io_error)?;
        let source_sha256 = hash_asset_file(&canonical_source_path).map_err(display_error)?;
        let materialized_path = artifact_path(&format!("direction_grid_import_input_{role}"))?;
        let materialized_sha256 = hash_asset_file(&materialized_path).map_err(display_error)?;
        if evidence_input
            .get("role")
            .and_then(serde_json::Value::as_str)
            != Some(*role)
            || evidence_input
                .get("sourcePath")
                .and_then(serde_json::Value::as_str)
                != canonical_source_path.to_str()
            || evidence_input
                .get("sourceSha256")
                .and_then(serde_json::Value::as_str)
                != Some(source_sha256.as_str())
            || evidence_input
                .get("materializedPath")
                .and_then(serde_json::Value::as_str)
                != materialized_path.to_str()
            || evidence_input
                .get("materializedSha256")
                .and_then(serde_json::Value::as_str)
                != Some(materialized_sha256.as_str())
            || source_sha256 != materialized_sha256
        {
            return Err((
                "direction_grid_import_provenance_invalid".into(),
                format!("ordered {role} input changed after import"),
            ));
        }
    }

    let matting: serde_json::Value =
        serde_json::from_slice(&fs::read(&matting_path).map_err(io_error)?).map_err(json_error)?;
    if !matches!(
        matting.get("profile").and_then(serde_json::Value::as_str),
        Some(CHECKERBOARD_SHEET_MATTING_PROFILE | CHECKERBOARD_SHEET_MATTING_PROFILE_LEGACY)
    ) || matting.get("verdict").and_then(serde_json::Value::as_str) != Some("game_ready")
        || matting
            .get("significantComponentCount")
            .and_then(serde_json::Value::as_u64)
            != Some(4)
        || matting
            .get("neutralEdgeResidualPixels")
            .and_then(serde_json::Value::as_u64)
            .is_none_or(|count| count > 16)
        || matting
            .get("cleanedBorderOpaqueRatio")
            .and_then(serde_json::Value::as_f64)
            .is_none_or(|ratio| ratio > 0.001)
    {
        return Err((
            "hard_failure_not_reviewable".into(),
            "current imported pixels fail the checkerboard matting contract".into(),
        ));
    }
    let alpha_edge: AlphaEdgeHaloReportV1 =
        serde_json::from_slice(&fs::read(&alpha_edge_path).map_err(io_error)?)
            .map_err(json_error)?;
    if alpha_edge.profile != ALPHA_EDGE_HALO_PROFILE
        || alpha_edge.verdict != ConsistencyVerdict::GameReady
        || alpha_edge.border_opaque_ratio > 0.001
        || alpha_edge.neutral_halo_pixel_count > 16
        || alpha_edge.dark_outline_discontinuity_budget != 32
        || alpha_edge.dark_outline_discontinuity_pixel_count
            > alpha_edge.dark_outline_discontinuity_budget
    {
        return Err((
            "hard_failure_not_reviewable".into(),
            "current imported pixels fail the Alpha edge/halo contract".into(),
        ));
    }
    let consistency: DirectionGridImportConsistencyReportV1 =
        serde_json::from_slice(&fs::read(&consistency_path).map_err(io_error)?)
            .map_err(json_error)?;
    if consistency.profile != DIRECTION_GRID_IMPORT_CONSISTENCY_PROFILE
        || consistency.absolute_appearance_sha256
            != lock.appearance_report_sha256.clone().unwrap_or_default()
        || consistency.nodes.len() != 4
        || matches!(
            consistency.verdict,
            ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
        )
    {
        return Err((
            "hard_failure_not_reviewable".into(),
            "current import fails same-direction source-relative identity consistency".into(),
        ));
    }
    match request.cape_hem_contract {
        Some(contract) => {
            let path = artifact_path("cape_hem_consistency_report")?;
            let sha256 = hash_asset_file(&path).map_err(display_error)?;
            let report: CapeHemConsistencyReportV1 =
                serde_json::from_slice(&fs::read(&path).map_err(io_error)?).map_err(json_error)?;
            if lock.cape_hem_contract != Some(contract)
                || lock.cape_hem_report_path.as_deref() != Some(path.as_path())
                || lock.cape_hem_report_sha256.as_deref() != Some(sha256.as_str())
                || report.profile != CAPE_HEM_CONSISTENCY_PROFILE
                || report.contract != contract
                || report.nodes.len() != 4
                || report.verdict != ConsistencyVerdict::GameReady
                || evidence
                    .get("capeHemContract")
                    .and_then(serde_json::Value::as_str)
                    != Some(contract.as_str())
                || evidence
                    .get("capeHemReportSha256")
                    .and_then(serde_json::Value::as_str)
                    != Some(sha256.as_str())
            {
                return Err((
                    "hard_failure_not_reviewable".into(),
                    "current import fails the explicit cape hem topology contract".into(),
                ));
            }
            for node in &report.nodes {
                let lock_node = lock.node(&node.node_id).ok_or_else(|| {
                    (
                        "direction_grid_import_provenance_invalid".into(),
                        format!("cape hem node {} is missing from the Lock", node.node_id),
                    )
                })?;
                if node.path != lock_node.path
                    || node.sha256 != lock_node.sha256
                    || hash_asset_file(&node.path).map_err(display_error)? != node.sha256
                    || node.verdict != ConsistencyVerdict::GameReady
                {
                    return Err((
                        "direction_grid_import_provenance_invalid".into(),
                        format!("cape hem evidence changed for {}", node.node_id),
                    ));
                }
            }
        }
        None => {
            if lock.cape_hem_contract.is_some()
                || lock.cape_hem_report_path.is_some()
                || lock.cape_hem_report_sha256.is_some()
                || record
                    .artifacts
                    .iter()
                    .any(|artifact| artifact.kind == "cape_hem_consistency_report")
            {
                return Err((
                    "direction_grid_import_provenance_invalid".into(),
                    "unexpected cape hem evidence is not bound by the import recipe".into(),
                ));
            }
        }
    }
    let source_lock: DirectionGridLockV1 = serde_json::from_slice(
        &fs::read(
            request
                .source_job_dir
                .join("source/direction-grid-lock.json"),
        )
        .map_err(io_error)?,
    )
    .map_err(json_error)?;
    for node in &consistency.nodes {
        let lock_node = lock.node(&node.node_id).ok_or_else(|| {
            (
                "direction_grid_import_provenance_invalid".into(),
                format!(
                    "consistency node {} is not in the imported Lock",
                    node.node_id
                ),
            )
        })?;
        let source_lock_node = source_lock.node(&node.node_id).ok_or_else(|| {
            (
                "direction_grid_import_provenance_invalid".into(),
                format!("consistency source node {} is missing", node.node_id),
            )
        })?;
        if node.source_path != source_lock_node.path
            || node.source_sha256 != source_lock_node.sha256
            || hash_asset_file(&node.source_path).map_err(display_error)? != node.source_sha256
            || node.candidate_path != lock_node.path
            || node.candidate_sha256 != lock_node.sha256
            || hash_asset_file(&node.candidate_path).map_err(display_error)?
                != node.candidate_sha256
            || matches!(
                node.verdict,
                ConsistencyVerdict::Blocked | ConsistencyVerdict::Regenerate
            )
        {
            return Err((
                "direction_grid_import_provenance_invalid".into(),
                format!("consistency evidence changed for {}", node.node_id),
            ));
        }
    }
    let alignment: serde_json::Value =
        serde_json::from_slice(&fs::read(&alignment_path).map_err(io_error)?)
            .map_err(json_error)?;
    if alignment.get("profile").and_then(serde_json::Value::as_str) != Some("onion-skin@1.0.0")
        || alignment.get("verdict").and_then(serde_json::Value::as_str) != Some("game_ready")
        || alignment
            .get("maxSourceScaleDrift")
            .and_then(serde_json::Value::as_f64)
            .is_none_or(|value| value > 0.20)
        || alignment
            .get("maxNormalizedCenterDriftPx")
            .and_then(serde_json::Value::as_f64)
            .is_none_or(|value| value > 2.0)
        || alignment
            .get("maxNormalizedFootDriftPx")
            .and_then(serde_json::Value::as_f64)
            .is_none_or(|value| value > 2.0)
    {
        return Err((
            "hard_failure_not_reviewable".into(),
            "current imported pixels fail the shared DirectionGrid alignment contract".into(),
        ));
    }
    let authority_alignment: serde_json::Value =
        serde_json::from_slice(&fs::read(&authority_alignment_path).map_err(io_error)?)
            .map_err(json_error)?;
    if authority_alignment
        .get("profile")
        .and_then(serde_json::Value::as_str)
        != Some("direction-grid-authority-alignment@1.0.0")
        || authority_alignment
            .get("verdict")
            .and_then(serde_json::Value::as_str)
            != Some("game_ready")
        || authority_alignment
            .get("frames")
            .and_then(serde_json::Value::as_array)
            .is_none_or(|frames| {
                frames.len() != 4
                    || frames.iter().any(|frame| {
                        frame
                            .get("widthRatio")
                            .and_then(serde_json::Value::as_f64)
                            .is_none_or(|value| !(0.90..=1.10).contains(&value))
                            || frame
                                .get("heightRatio")
                                .and_then(serde_json::Value::as_f64)
                                .is_none_or(|value| !(0.90..=1.10).contains(&value))
                            || frame
                                .get("centerDriftPx")
                                .and_then(serde_json::Value::as_f64)
                                .is_none_or(|value| value > 2.0)
                            || frame
                                .get("footBaselineDriftPx")
                                .and_then(serde_json::Value::as_f64)
                                .is_none_or(|value| value > 2.0)
                    })
            })
    {
        return Err((
            "hard_failure_not_reviewable".into(),
            "current imported pixels fail source-relative DirectionGrid alignment".into(),
        ));
    }
    let usage: serde_json::Value =
        serde_json::from_slice(&fs::read(&usage_path).map_err(io_error)?).map_err(json_error)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).map_err(io_error)?).map_err(json_error)?;
    let usage_is_zero = |value: &serde_json::Value| {
        [
            "requests",
            "generatedImages",
            "generatedVideos",
            "editedVideos",
            "privateFileUploads",
        ]
        .into_iter()
        .all(|field| {
            value
                .pointer(&format!("/usage/{field}"))
                .and_then(serde_json::Value::as_u64)
                == Some(0)
        })
    };
    if !usage_is_zero(&usage)
        || !usage_is_zero(&manifest)
        || manifest
            .get("generationMethod")
            .and_then(serde_json::Value::as_str)
            != Some("external_direction_grid_import")
        || manifest
            .get("providerRequestOccurred")
            .and_then(serde_json::Value::as_bool)
            != Some(false)
        || manifest
            .pointer("/importConsistency/profile")
            .and_then(serde_json::Value::as_str)
            != Some(DIRECTION_GRID_IMPORT_CONSISTENCY_PROFILE)
        || manifest
            .pointer("/importConsistency/sha256")
            .and_then(serde_json::Value::as_str)
            != Some(consistency_sha256.as_str())
        || manifest
            .pointer("/producer/kind")
            .and_then(serde_json::Value::as_str)
            != Some("external_import")
        || manifest
            .pointer("/downstreamBinding/providerId")
            .and_then(serde_json::Value::as_str)
            != Some(lock.provider_id.as_str())
    {
        return Err((
            "direction_grid_import_provenance_invalid".into(),
            "external DirectionGrid import no longer has zero-request Provider evidence".into(),
        ));
    }
    Ok(lock)
}

fn sanitize_cli_id(value: &str) -> String {
    forge_core::asset_project::safe_id(value)
        .trim_end_matches(".gsfpack")
        .to_string()
}

fn read_request<T: serde::de::DeserializeOwned>(
    input: &RequestInput,
) -> Result<T, (String, String)> {
    if input.request.is_some() == input.stdin {
        return Err((
            "invalid_arguments".into(),
            "choose exactly one of --request or --stdin".into(),
        ));
    }
    let bytes = if let Some(path) = &input.request {
        fs::read(path).map_err(io_error)?
    } else {
        let mut bytes = Vec::new();
        io::stdin().read_to_end(&mut bytes).map_err(io_error)?;
        bytes
    };
    serde_json::from_slice(&bytes).map_err(json_error)
}

fn spawn_worker(record: &JobRecord) -> Result<(), (String, String)> {
    let executable = env::current_exe().map_err(io_error)?;
    let stdout =
        fs::File::create(record.job_dir.join("logs/worker.stdout.log")).map_err(io_error)?;
    let stderr =
        fs::File::create(record.job_dir.join("logs/worker.stderr.log")).map_err(io_error)?;
    let child = ProcessCommand::new(executable)
        .arg("__worker")
        .arg("--job-id")
        .arg(&record.job_id)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(io_error)?;
    let store = job_store()?;
    store
        .update_record(&record.job_id, |record| {
            if !matches!(
                record.lifecycle_state,
                forge_core::job::JobLifecycleState::Succeeded
                    | forge_core::job::JobLifecycleState::Failed
                    | forge_core::job::JobLifecycleState::Cancelled
            ) {
                record.worker_pid = Some(child.id());
            }
        })
        .map_err(display_error)?;
    Ok(())
}

fn open_forge_job(job_id: &str) -> Result<(), (String, String)> {
    let record = job_store()?.read_record(job_id).map_err(display_error)?;
    let status = ProcessCommand::new("/usr/bin/open")
        .arg(&record.job_dir)
        .status()
        .map_err(io_error)?;
    status.success().then_some(()).ok_or_else(|| {
        (
            "job_reveal_failed".into(),
            format!(
                "could not reveal job directory: {}",
                record.job_dir.display()
            ),
        )
    })
}

fn asset_records(record: JobRecord) -> Vec<AssetRecord> {
    let name = record
        .recipe
        .as_ref()
        .and_then(|recipe| recipe.pointer("/request/metadata/name"))
        .and_then(|name| name.as_str())
        .map(str::to_string);
    record
        .artifacts
        .into_iter()
        .filter(|artifact| artifact.kind == "gsfpack")
        .map(|artifact| AssetRecord {
            job_id: record.job_id.clone(),
            asset_id: record.asset_id.clone(),
            name: name.clone(),
            artifact,
        })
        .collect()
}

fn job_store() -> Result<JobStore, (String, String)> {
    if let Some(root) = env::var_os("FORGE_JOB_STORE") {
        JobStore::new(root).map_err(display_error)
    } else {
        JobStore::default_app_store().map_err(display_error)
    }
}

fn authorization_store_root() -> Result<PathBuf, (String, String)> {
    if let Some(root) = env::var_os("FORGE_AUTHORIZATION_STORE") {
        return Ok(PathBuf::from(root));
    }
    let jobs = job_store()?;
    let parent = jobs.root().parent().ok_or_else(|| {
        (
            "authorization_store_unavailable".into(),
            "JobStore root has no parent for the authorization store".into(),
        )
    })?;
    Ok(parent.join("authorizations"))
}

#[allow(clippy::too_many_arguments)]
fn create_provider_authorization(
    provider: &str,
    profile: &str,
    id: &str,
    targets: &[String],
    max_requests_per_target: u32,
    max_requests: Option<u32>,
    max_provider_operations: Option<u32>,
    max_cost_ticks: u64,
    cost_reservation_ticks_per_request: u64,
    models: Vec<String>,
    source_job: Option<&str>,
    recipe_hash: Option<String>,
    input_fingerprint: Option<String>,
    expires_minutes: i64,
) -> Result<(), (String, String)> {
    if expires_minutes <= 0 || expires_minutes > 24 * 60 {
        return Err((
            "invalid_authorization".into(),
            "expires-minutes must be between 1 and 1440".into(),
        ));
    }
    let mut unique = std::collections::BTreeSet::new();
    if targets.iter().any(|target| !unique.insert(target.clone())) {
        return Err((
            "invalid_authorization".into(),
            "authorization targets must be unique".into(),
        ));
    }
    let allowed_targets = targets
        .iter()
        .map(|target_id| AuthorizedTargetV1 {
            target_id: target_id.clone(),
            max_requests: max_requests_per_target,
            cost_reservation_ticks_per_request,
        })
        .collect::<Vec<_>>();
    let target_request_limit = max_requests_per_target.saturating_mul(targets.len() as u32);
    let max_total_requests = max_requests.unwrap_or(target_request_limit);
    let source_lineage_root_job_id = source_job
        .map(|job_id| {
            let record = job_store()?.read_record(job_id).map_err(display_error)?;
            Ok::<_, (String, String)>(
                record
                    .lineage_root_job_id
                    .unwrap_or_else(|| record.job_id.clone()),
            )
        })
        .transpose()?;
    let now = chrono::Utc::now();
    let manifest = AuthorizationManifestV1 {
        schema_version: AUTHORIZATION_MANIFEST_SCHEMA_VERSION.into(),
        authorization_id: id.into(),
        provider_id: provider.into(),
        profile_id: profile.into(),
        allowed_models: models,
        created_at: now,
        expires_at: now + chrono::Duration::minutes(expires_minutes),
        max_total_requests,
        max_total_provider_operations: max_provider_operations,
        max_total_cost_ticks: max_cost_ticks,
        allowed_targets,
        source_lineage_root_job_id,
        recipe_hash,
        input_fingerprint,
    };
    let store = AuthorizationStore::create(authorization_store_root()?, &manifest)
        .map_err(display_error)?;
    success(&serde_json::json!({
        "manifest": store.manifest().map_err(display_error)?,
        "ledger": store.ledger().map_err(display_error)?,
        "secretMaterialStored": false,
    }))
}

fn attach_authorization(
    store: &JobStore,
    record: &JobRecord,
    plan: &AutomationPlan,
    authorization_id: &str,
) -> Result<JobRecord, (String, String)> {
    let authorization = AuthorizationStore::open(authorization_store_root()?, authorization_id)
        .map_err(display_error)?;
    let (manifest, manifest_sha256) = authorization.manifest_snapshot().map_err(display_error)?;
    if let AutomationOperation::GenerateCharacterPack(request) = &plan.operation {
        if request.workflow.id == "topdown-grid"
            && matches!(
                request.workflow.version.as_str(),
                "9.3.0" | "9.4.0" | "9.5.0"
            )
        {
            let model = plan.estimate.model.as_deref().ok_or_else(|| {
                (
                    "asymmetric_gait_authorization_model_missing".into(),
                    "V9.3 plan has no resolved image model".into(),
                )
            })?;
            let lineage_root = record
                .lineage_root_job_id
                .as_deref()
                .unwrap_or(&record.job_id);
            validate_v93_authorization_manifest(
                &manifest,
                request,
                model,
                lineage_root,
                &plan.recipe_hash,
                &plan.input_fingerprint,
            )?;
            authorization
                .ensure_active_and_pristine()
                .map_err(|_| {
                    (
                        "asymmetric_gait_authorization_not_fresh".into(),
                        "topdown-grid@9.3.0 authorization expired or its ledger is not physically empty"
                            .into(),
                    )
                })?;
        }
        if request.workflow.id == "topdown-cycle"
            && request.workflow.version == "10.0.0"
            && request.provider_id != "fixture"
        {
            let model = plan.estimate.model.as_deref().ok_or_else(|| {
                (
                    "topdown_cycle_v10_authorization_model_missing".into(),
                    "V10 plan has no resolved video model".into(),
                )
            })?;
            let lineage_root = record
                .lineage_root_job_id
                .as_deref()
                .unwrap_or(&record.job_id);
            validate_v10_authorization_manifest(
                &manifest,
                request,
                model,
                lineage_root,
                &plan.recipe_hash,
                &plan.input_fingerprint,
            )?;
            authorization
                .ensure_active_and_pristine()
                .map_err(|_| {
                    (
                        "topdown_cycle_v10_authorization_not_fresh".into(),
                        "topdown-cycle@10.0.0 authorization expired or its ledger is not physically empty"
                            .into(),
                    )
                })?;
        }
    }
    if let Some(provider) = &plan.estimate.provider_id {
        if &manifest.provider_id != provider {
            return Err((
                "authorization_provider_mismatch".into(),
                format!(
                    "authorization provider {} does not match plan provider {provider}",
                    manifest.provider_id
                ),
            ));
        }
    }
    if let Some(profile) = &plan.estimate.profile_id {
        if &manifest.profile_id != profile {
            return Err((
                "authorization_profile_mismatch".into(),
                format!(
                    "authorization profile {} does not match plan profile {profile}",
                    manifest.profile_id
                ),
            ));
        }
    }
    if manifest
        .recipe_hash
        .as_ref()
        .is_some_and(|hash| hash != &plan.recipe_hash)
        || manifest
            .input_fingerprint
            .as_ref()
            .is_some_and(|hash| hash != &plan.input_fingerprint)
    {
        return Err((
            "authorization_plan_mismatch".into(),
            "authorization recipe/input binding does not match this plan".into(),
        ));
    }
    let lineage_root = record
        .lineage_root_job_id
        .as_deref()
        .unwrap_or(&record.job_id);
    if manifest
        .source_lineage_root_job_id
        .as_deref()
        .is_some_and(|root| root != lineage_root)
    {
        return Err((
            "authorization_lineage_mismatch".into(),
            "authorization is bound to a different Job lineage".into(),
        ));
    }
    let ledger = authorization.ledger().map_err(display_error)?;
    let remaining_requests = manifest
        .max_total_requests
        .saturating_sub(ledger.consumed_request_count());
    if plan.estimate.provider_request_estimate > 0 && remaining_requests == 0 {
        return Err((
            "authorization_request_budget_exceeded".into(),
            "authorization has no Provider requests remaining".into(),
        ));
    }
    let allowed_targets = manifest
        .allowed_targets
        .iter()
        .map(|target| target.target_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for target in operation_authorization_targets(&plan.operation)? {
        if !allowed_targets.contains(target.as_str()) {
            return Err((
                "authorization_target_not_allowed".into(),
                format!("authorization does not permit plan target {target}"),
            ));
        }
    }
    let (_, current_manifest_sha256) = authorization.manifest_snapshot().map_err(display_error)?;
    if current_manifest_sha256 != manifest_sha256 {
        return Err((
            "authorization_manifest_changed".into(),
            "authorization manifest changed while it was being attached to the Job".into(),
        ));
    }
    store
        .update_record(&record.job_id, |record| {
            record.authorization_id = Some(authorization_id.to_string());
            record.authorization_manifest_sha256 = Some(manifest_sha256.clone());
        })
        .map_err(display_error)
}

fn operation_authorization_targets(
    operation: &AutomationOperation,
) -> Result<Vec<String>, (String, String)> {
    let mut targets = match operation {
        AutomationOperation::CreateStyleLock(_) => vec!["style_board".into()],
        AutomationOperation::CreateSubjectLock(request) => {
            if request.is_local_import() {
                Vec::new()
            } else {
                let spec = forge_core::subject::read_subject_spec(&request.spec_path)
                    .map_err(display_error)?;
                vec![format!("subject:{}", spec.id)]
            }
        }
        AutomationOperation::CreateCollectionLock(request) => {
            let spec = forge_core::collection::read_collection_spec(&request.spec_path)
                .map_err(display_error)?;
            if spec.anchor_image.is_some() {
                Vec::new()
            } else {
                vec![format!("collection:{}", spec.id)]
            }
        }
        AutomationOperation::GenerateStaticAssetSet(request) => {
            if request.consistency_recheck_only {
                Vec::new()
            } else if request.portrait_phase == PortraitGenerationPhaseV1::BaseOnly {
                vec!["neutral".into()]
            } else if request.retry_item_ids.is_empty() {
                request
                    .asset
                    .items
                    .iter()
                    .map(|item| item.id.clone())
                    .collect()
            } else {
                request.retry_item_ids.clone()
            }
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            let animations = if request.validation_only {
                request
                    .validation_animations
                    .iter()
                    .map(String::as_str)
                    .collect()
            } else if request.retry_animations.is_empty() {
                vec!["idle", "walk_up", "walk_right", "walk_down"]
            } else {
                request
                    .retry_animations
                    .iter()
                    .map(String::as_str)
                    .collect()
            };
            if request.workflow.id == "topdown-cycle" && request.workflow.version == "10.0.0" {
                animations
                    .into_iter()
                    .map(|animation| format!("{animation}:video"))
                    .collect()
            } else if request.workflow.id == "topdown-spritesheet" {
                let local_only = !request.retry_animations.is_empty()
                    && request.retry_animations.iter().all(|animation| {
                        request.retry_stages.get(animation).is_some_and(|stage| {
                            matches!(
                                stage,
                                CharacterRetryStage::Loop
                                    | CharacterRetryStage::Matting
                                    | CharacterRetryStage::Consistency
                            )
                        })
                    });
                if local_only {
                    Vec::new()
                } else {
                    animations.into_iter().map(str::to_string).collect()
                }
            } else if request.workflow.id == "topdown-frames" {
                let local_only = !request.retry_animations.is_empty()
                    && request.retry_animations.iter().all(|animation| {
                        request.retry_stages.get(animation).is_some_and(|stage| {
                            matches!(
                                stage,
                                CharacterRetryStage::Loop
                                    | CharacterRetryStage::Matting
                                    | CharacterRetryStage::Consistency
                            )
                        })
                    });
                if local_only {
                    Vec::new()
                } else {
                    let mut targets = request
                        .reuse_from_job_dir
                        .is_none()
                        .then(|| "direction_lock".to_string())
                        .into_iter()
                        .collect::<Vec<_>>();
                    for animation in animations {
                        if let Some(frames) = request.retry_frames.get(animation) {
                            targets.extend(
                                frames
                                    .iter()
                                    .map(|frame| format!("{animation}:frame:{frame}")),
                            );
                        } else {
                            targets.push(animation.to_string());
                        }
                    }
                    targets
                }
            } else if request.workflow.id == "topdown-direction-motion"
                && request.workflow.version == "8.0.0"
            {
                let local_only = !request.retry_animations.is_empty()
                    && request.retry_animations.iter().all(|animation| {
                        request.retry_stages.get(animation).is_some_and(|stage| {
                            matches!(
                                stage,
                                CharacterRetryStage::Loop
                                    | CharacterRetryStage::Matting
                                    | CharacterRetryStage::Consistency
                            )
                        })
                    });
                if local_only {
                    Vec::new()
                } else {
                    match request.direction_motion_stage {
                        DirectionMotionGenerationStageV1::ImageLocks => {
                            let nodes: Vec<&str> = if request.retry_animations.is_empty() {
                                vec![
                                    "front_idle",
                                    "back_idle",
                                    "right_idle",
                                    "left_idle",
                                    "front_walk",
                                    "back_walk",
                                    "right_walk",
                                    "left_walk",
                                ]
                            } else {
                                DirectionMotionLockV1::image_retry_closure(
                                    &request.retry_animations,
                                )
                                .map_err(|error| ("invalid_retry_scope".into(), error))?
                            };
                            nodes.into_iter().map(str::to_string).collect()
                        }
                        DirectionMotionGenerationStageV1::Complete => {
                            let animations: Vec<&str> = if request.validation_only {
                                request
                                    .validation_animations
                                    .iter()
                                    .map(String::as_str)
                                    .collect()
                            } else {
                                vec!["walk_down", "walk_up", "walk_right", "walk_left"]
                            };
                            animations
                                .into_iter()
                                .map(|animation| format!("{animation}:video"))
                                .collect()
                        }
                    }
                }
            } else if request.workflow.id == "topdown-grid" && request.workflow.version == "9.0.0" {
                match request.direction_motion_stage {
                    DirectionMotionGenerationStageV1::ImageLocks => vec!["direction_grid".into()],
                    DirectionMotionGenerationStageV1::Complete => {
                        if request.retry_frames.is_empty() {
                            let animations: Vec<&str> = if request.validation_only {
                                request
                                    .validation_animations
                                    .iter()
                                    .map(String::as_str)
                                    .collect()
                            } else {
                                vec!["walk_down", "walk_up", "walk_right", "walk_left"]
                            };
                            animations
                                .into_iter()
                                .map(|animation| format!("{animation}:action_grid"))
                                .collect()
                        } else {
                            request
                                .retry_frames
                                .iter()
                                .flat_map(|(animation, frames)| {
                                    frames
                                        .iter()
                                        .map(|frame| format!("{animation}:frame:{frame}"))
                                        .collect::<Vec<_>>()
                                })
                                .collect()
                        }
                    }
                }
            } else if request.workflow.id == "topdown-grid"
                && matches!(
                    request.workflow.version.as_str(),
                    "9.1.0" | "9.2.0" | "9.3.0" | "9.4.0" | "9.5.0"
                )
            {
                if request.retry_frames.is_empty() {
                    let actions: Vec<&str> = if request.validation_only {
                        request
                            .validation_animations
                            .iter()
                            .map(String::as_str)
                            .collect()
                    } else {
                        vec!["walk_down", "walk_up", "walk_right", "walk_left"]
                    };
                    actions
                        .into_iter()
                        .flat_map(|action| {
                            (0..4).map(move |frame| format!("{action}:frame:{frame}"))
                        })
                        .collect()
                } else {
                    request
                        .retry_frames
                        .iter()
                        .flat_map(|(animation, frames)| {
                            frames
                                .iter()
                                .map(move |frame| format!("{animation}:frame:{frame}"))
                        })
                        .collect()
                }
            } else if request.workflow.id == "topdown-direction-poses"
                && request.workflow.version == "7.0.0"
            {
                let local_only = !request.retry_animations.is_empty()
                    && request.retry_animations.iter().all(|animation| {
                        request.retry_stages.get(animation).is_some_and(|stage| {
                            matches!(
                                stage,
                                CharacterRetryStage::Loop
                                    | CharacterRetryStage::Matting
                                    | CharacterRetryStage::Consistency
                            )
                        })
                    });
                if local_only {
                    Vec::new()
                } else if request.validation_only {
                    request
                        .validation_animations
                        .iter()
                        .flat_map(|animation| match animation.as_str() {
                            "walk_up" => vec![
                                "direction_rear".into(),
                                "walk_up:pose".into(),
                                "walk_up:video".into(),
                            ],
                            "walk_right" => vec![
                                "direction_right".into(),
                                "walk_right:pose".into(),
                                "walk_right:video".into(),
                            ],
                            "walk_down" => {
                                vec!["walk_down:pose".into(), "walk_down:video".into()]
                            }
                            _ => Vec::new(),
                        })
                        .collect()
                } else if request.retry_animations.is_empty() {
                    vec![
                        "direction_rear".into(),
                        "direction_right".into(),
                        "walk_up:pose".into(),
                        "walk_right:pose".into(),
                        "walk_down:pose".into(),
                        "walk_up:video".into(),
                        "walk_right:video".into(),
                        "walk_down:video".into(),
                    ]
                } else {
                    request
                        .retry_animations
                        .iter()
                        .flat_map(|animation| {
                            let stage = request
                                .retry_stages
                                .get(animation)
                                .copied()
                                .unwrap_or(CharacterRetryStage::Auto);
                            match stage {
                                CharacterRetryStage::Auto | CharacterRetryStage::Video => {
                                    vec![format!("{animation}:video")]
                                }
                                CharacterRetryStage::Still => Vec::new(),
                                CharacterRetryStage::Loop
                                | CharacterRetryStage::Matting
                                | CharacterRetryStage::Consistency
                                | CharacterRetryStage::Frame => Vec::new(),
                            }
                        })
                        .collect()
                }
            } else if matches!(
                request.workflow.id.as_str(),
                "topdown-video-locked" | "topdown-video-cycle"
            ) {
                let local_only = !request.retry_animations.is_empty()
                    && request.retry_animations.iter().all(|animation| {
                        request.retry_stages.get(animation).is_some_and(|stage| {
                            matches!(
                                stage,
                                CharacterRetryStage::Loop
                                    | CharacterRetryStage::Matting
                                    | CharacterRetryStage::Consistency
                            )
                        })
                    });
                if local_only {
                    Vec::new()
                } else {
                    let mut targets = request
                        .reuse_from_job_dir
                        .is_none()
                        .then(|| "direction_lock".to_string())
                        .into_iter()
                        .collect::<Vec<_>>();
                    targets.extend(
                        animations
                            .into_iter()
                            .map(|animation| format!("{animation}:video")),
                    );
                    targets
                }
            } else if matches!(
                request.workflow.id.as_str(),
                "topdown-keyframes" | "topdown-keyposes"
            ) {
                let frame_count = if request.workflow.id == "topdown-keyposes" {
                    4
                } else {
                    8
                };
                animations
                    .into_iter()
                    .flat_map(|animation| {
                        let frames = request
                            .retry_frames
                            .get(animation)
                            .cloned()
                            .unwrap_or_else(|| (0..frame_count).collect());
                        frames
                            .into_iter()
                            .map(move |frame| format!("{animation}:frame:{frame}"))
                    })
                    .collect()
            } else {
                video_character_authorization_targets(
                    &animations,
                    &request.retry_stages,
                    request.reuse_from_job_dir.is_none() && request.workflow.id != "topdown-video",
                )
            }
        }
        AutomationOperation::CreateEnvironmentLock(request) => {
            let spec: forge_core::world::EnvironmentSpecV1 =
                serde_json::from_slice(&fs::read(&request.spec_path).map_err(io_error)?)
                    .map_err(json_error)?;
            vec![format!("environment:{}", spec.id)]
        }
        // These profiles have target labels derived from material/module
        // internals. The network boundary still enforces them; their CLI
        // target planner is added with the corresponding public milestone.
        AutomationOperation::GenerateTerrainSet(_)
        | AutomationOperation::GenerateBuildingKit(_)
        | AutomationOperation::BuildProject(_) => Vec::new(),
        _ => Vec::new(),
    };
    targets.sort();
    targets.dedup();
    Ok(targets)
}

fn video_character_authorization_targets(
    animations: &[&str],
    retry_stages: &std::collections::BTreeMap<String, CharacterRetryStage>,
    include_subject_reference: bool,
) -> Vec<String> {
    let mut targets = include_subject_reference
        .then(|| "subject_reference".to_string())
        .into_iter()
        .collect::<Vec<_>>();
    for animation in animations {
        match retry_stages
            .get(*animation)
            .copied()
            .unwrap_or(CharacterRetryStage::Still)
        {
            CharacterRetryStage::Auto | CharacterRetryStage::Still => {
                targets.push(format!("{animation}:still"));
                targets.push(format!("{animation}:video"));
            }
            CharacterRetryStage::Video => {
                targets.push(format!("{animation}:video"));
            }
            CharacterRetryStage::Loop
            | CharacterRetryStage::Matting
            | CharacterRetryStage::Consistency
            | CharacterRetryStage::Frame => {}
        }
    }
    targets
}

fn plan_store() -> Result<PlanStore, (String, String)> {
    if let Some(root) = env::var_os("FORGE_PLAN_STORE") {
        PlanStore::new(root).map_err(display_error)
    } else {
        PlanStore::default_app_store().map_err(display_error)
    }
}

fn locate_godot() -> Option<PathBuf> {
    if let Some(path) = env::var_os("FORGE_GODOT_PATH").map(PathBuf::from) {
        if path.is_file() {
            return Some(path);
        }
    }
    [
        PathBuf::from("/Applications/Godot.app/Contents/MacOS/Godot"),
        PathBuf::from("/Applications/Godot_mono.app/Contents/MacOS/Godot"),
    ]
    .into_iter()
    .find(|path| path.is_file())
    .or_else(|| {
        ["godot4", "godot"].into_iter().find_map(|name| {
            let output = ProcessCommand::new("/usr/bin/which")
                .arg(name)
                .output()
                .ok()?;
            output
                .status
                .success()
                .then(|| PathBuf::from(String::from_utf8_lossy(&output.stdout).trim().to_string()))
        })
    })
}

fn godot_version(path: &Path) -> Option<String> {
    let output = ProcessCommand::new(path).arg("--version").output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn display_error(error: impl std::fmt::Display) -> (String, String) {
    ("operation_failed".into(), error.to_string())
}

fn io_error(error: io::Error) -> (String, String) {
    ("io_error".into(), error.to_string())
}

fn json_error(error: serde_json::Error) -> (String, String) {
    ("invalid_json".into(), error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "subject-import")]
    #[test]
    fn subject_import_requires_neither_provider_resolution_nor_authorization_targets() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "create_subject_lock",
            "request": {
                "schemaVersion": "1",
                "projectPath": "/fixture/project",
                "specPath": "/fixture/subject.json",
                "providerId": "xai",
                "profileId": "default",
                "canonicalImportPath": "/fixture/approved.png",
                "importApprovalNote": "approved existing canonical"
            }
        }))
        .unwrap();
        assert!(operation_provider_id(&operation).is_none());
        assert!(operation_authorization_targets(&operation)
            .unwrap()
            .is_empty());
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn grid_generation_resolves_image_model_without_locking_a_video_model() {
        let mut operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "metadata": {
                    "name": "Grid Model Resolution",
                    "defaultAnimation": "idle_down",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-grid", "version": "9.0.0" },
                "generation": { "targetFrameCount": 4 }
            }
        }))
        .unwrap();

        resolve_operation_models(&mut operation).unwrap();
        let AutomationOperation::GenerateCharacterPack(request) = operation else {
            unreachable!();
        };
        assert!(request.generation.image_model.is_some());
        assert!(request.generation.video_model.is_none());
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn grid_direction_lock_retry_accepts_the_typed_whole_sheet_item() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "directionMotionStage": "image_locks",
                "metadata": {
                    "name": "Grid Direction Retry",
                    "defaultAnimation": "idle_down",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-grid", "version": "9.0.0" },
                "generation": { "targetFrameCount": 4 },
                "reuseFromJobDir": "/fixture/source-job",
                "retryAnimations": ["direction_grid"],
                "retryStages": { "direction_grid": "still" }
            }
        }))
        .unwrap();
        let AutomationOperation::GenerateCharacterPack(request) = &operation else {
            unreachable!();
        };
        assert!(is_grid_direction_lock_retry(request, "direction_grid"));
        assert!(!is_grid_direction_lock_retry(request, "idle_down"));
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["direction_grid"]
        );
    }

    #[test]
    fn grid_validation_authorizes_only_the_selected_action_grid() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "directionMotionStage": "complete",
                "validationOnly": true,
                "validationAnimations": ["walk_down"],
                "metadata": {
                    "name": "Grid Walk Down Probe",
                    "defaultAnimation": "walk_down",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-grid", "version": "9.0.0" },
                "generation": { "targetFrameCount": 4 }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["walk_down:action_grid"]
        );
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn grid_v91_authorization_is_frame_exact_for_probe_full_and_child_retry() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "directionMotionStage": "complete",
                "validationOnly": true,
                "validationAnimations": ["walk_down"],
                "metadata": {
                    "name": "Grid V9.1 Walk Down Probe",
                    "defaultAnimation": "walk_down",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-grid", "version": "9.1.0" },
                "generation": { "targetFrameCount": 4, "poseGuidance": "disabled" }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            (0..4)
                .map(|frame| format!("walk_down:frame:{frame}"))
                .collect::<Vec<_>>()
        );

        let mut full = operation.clone();
        let AutomationOperation::GenerateCharacterPack(request) = &mut full else {
            unreachable!();
        };
        request.validation_only = false;
        request.validation_animations.clear();
        request.metadata.default_animation = "idle_down".into();
        assert_eq!(operation_authorization_targets(&full).unwrap().len(), 16);

        let mut retry = operation;
        let AutomationOperation::GenerateCharacterPack(request) = &mut retry else {
            unreachable!();
        };
        request.reuse_from_job_dir = Some(PathBuf::from("/fixture/v91-source-job"));
        request.retry_animations = vec!["walk_down".into()];
        request
            .retry_stages
            .insert("walk_down".into(), CharacterRetryStage::Frame);
        request.retry_frames.insert("walk_down".into(), vec![2]);
        assert_eq!(
            operation_authorization_targets(&retry).unwrap(),
            vec!["walk_down:frame:2"]
        );
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn grid_v92_authorization_is_walk_down_frame_exact() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "directionMotionStage": "complete",
                "validationOnly": true,
                "validationAnimations": ["walk_down"],
                "metadata": {
                    "name": "Grid V9.2 Walk Down Probe",
                    "defaultAnimation": "walk_down",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-grid", "version": "9.2.0" },
                "generation": { "targetFrameCount": 4, "poseGuidance": "grayscale" }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            (0..4)
                .map(|frame| format!("walk_down:frame:{frame}"))
                .collect::<Vec<_>>()
        );

        let mut retry = operation;
        let AutomationOperation::GenerateCharacterPack(request) = &mut retry else {
            unreachable!();
        };
        request.reuse_from_job_dir = Some(PathBuf::from("/fixture/v92-failed-source-job"));
        request.retry_animations = vec!["walk_down".into()];
        request
            .retry_stages
            .insert("walk_down".into(), CharacterRetryStage::Frame);
        request.retry_frames.insert("walk_down".into(), vec![2]);
        assert_eq!(
            operation_authorization_targets(&retry).unwrap(),
            vec!["walk_down:frame:2"]
        );
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn image2_direction_grid_candidate_authorization_is_exactly_one_edit() {
        let request: GenerateCharacterPackRequest = serde_json::from_value(serde_json::json!({
            "schemaVersion": "3",
            "providerId": "xai",
            "profileId": "default",
            "character": { "prompt": "fixture" },
            "cameraProfile": "topdown-three-quarter@1.0.0",
            "equipment": { "kind": "none" },
            "equipmentExplicit": true,
            "directionMotionStage": "image_locks",
            "metadata": {
                "name": "Image 2 DirectionGrid Candidate",
                "defaultAnimation": "idle_down",
                "creator": "Forge",
                "license": "private"
            },
            "workflow": { "id": "topdown-grid", "version": "9.0.0" },
            "generation": {
                "maxAttemptsPerAnimation": 1,
                "targetFrameCount": 4,
                "imageModel": "grok-imagine-image-2.0",
                "poseGuidance": "enabled"
            },
            "reuseFromJobDir": "/fixture/approved-v9-source",
            "retryAnimations": ["direction_grid"],
            "retryStages": { "direction_grid": "still" }
        }))
        .unwrap();
        let operation = AutomationOperation::GenerateCharacterPack(request.clone());
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["direction_grid"]
        );

        let now = chrono::Utc::now();
        let canonical = AuthorizationManifestV1 {
            schema_version: AUTHORIZATION_MANIFEST_SCHEMA_VERSION.into(),
            authorization_id: "image2-direction-grid".into(),
            provider_id: "xai".into(),
            profile_id: "default".into(),
            allowed_models: vec![XAI_IMAGE_2_CANDIDATE_MODEL.into()],
            created_at: now,
            expires_at: now + chrono::Duration::minutes(30),
            max_total_requests: 1,
            max_total_provider_operations: Some(1),
            max_total_cost_ticks: XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS,
            allowed_targets: vec![AuthorizedTargetV1 {
                target_id: "direction_grid".into(),
                max_requests: 1,
                cost_reservation_ticks_per_request: XAI_IMAGE_2_CANDIDATE_MAX_COST_TICKS,
            }],
            source_lineage_root_job_id: Some("lineage-root".into()),
            recipe_hash: Some("recipe-hash".into()),
            input_fingerprint: Some("input-hash".into()),
        };
        assert!(validate_image2_candidate_authorization_manifest(
            &canonical,
            &request,
            "lineage-root",
            "recipe-hash",
            "input-hash",
        )
        .is_ok());

        let mut invalid = Vec::new();
        let mut value = canonical.clone();
        value.allowed_models = vec!["grok-imagine-image-quality".into()];
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_requests = 2;
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_provider_operations = None;
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_targets[0].target_id = "front_idle".into();
        invalid.push(value);
        let mut value = canonical.clone();
        value.source_lineage_root_job_id = Some("other".into());
        invalid.push(value);
        let mut value = canonical.clone();
        value.recipe_hash = Some("other".into());
        invalid.push(value);
        let mut value = canonical;
        value.input_fingerprint = Some("other".into());
        invalid.push(value);

        for manifest in invalid {
            let error = validate_image2_candidate_authorization_manifest(
                &manifest,
                &request,
                "lineage-root",
                "recipe-hash",
                "input-hash",
            )
            .unwrap_err();
            assert_eq!(error.0, "xai_image2_candidate_authorization_scope_mismatch");
        }
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn front_authoritative_grid_authorization_is_exactly_one_edit() {
        let request: GenerateCharacterPackRequest = serde_json::from_value(serde_json::json!({
            "schemaVersion": "3",
            "providerId": "xai",
            "profileId": "default",
            "character": { "prompt": "fixture" },
            "cameraProfile": "topdown-three-quarter@1.0.0",
            "equipment": { "kind": "none" },
            "equipmentExplicit": true,
            "directionMotionStage": "image_locks",
            "directionGridCapeContract": "front_authoritative_no_skirt_hem",
            "metadata": {
                "name": "Front Authority DirectionGrid",
                "defaultAnimation": "idle_down",
                "creator": "Forge",
                "license": "private"
            },
            "workflow": { "id": "topdown-grid", "version": "9.0.0" },
            "generation": {
                "maxAttemptsPerAnimation": 1,
                "targetFrameCount": 4,
                "imageModel": "grok-imagine-image-quality"
            },
            "reuseFromJobDir": "/fixture/approved-v9-source",
            "retryAnimations": ["direction_grid"],
            "retryStages": { "direction_grid": "still" }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&AutomationOperation::GenerateCharacterPack(
                request.clone()
            ))
            .unwrap(),
            vec!["direction_grid"]
        );
        let now = chrono::Utc::now();
        let canonical = AuthorizationManifestV1 {
            schema_version: AUTHORIZATION_MANIFEST_SCHEMA_VERSION.into(),
            authorization_id: "front-authority-grid".into(),
            provider_id: "xai".into(),
            profile_id: "default".into(),
            allowed_models: vec!["grok-imagine-image-quality".into()],
            created_at: now,
            expires_at: now + chrono::Duration::minutes(30),
            max_total_requests: 1,
            max_total_provider_operations: Some(1),
            max_total_cost_ticks: FRONT_AUTHORITATIVE_DIRECTION_GRID_MAX_COST_TICKS,
            allowed_targets: vec![AuthorizedTargetV1 {
                target_id: "direction_grid".into(),
                max_requests: 1,
                cost_reservation_ticks_per_request:
                    FRONT_AUTHORITATIVE_DIRECTION_GRID_MAX_COST_TICKS,
            }],
            source_lineage_root_job_id: Some("lineage-root".into()),
            recipe_hash: Some("recipe-hash".into()),
            input_fingerprint: Some("input-hash".into()),
        };
        assert!(validate_front_authoritative_grid_authorization_manifest(
            &canonical,
            &request,
            "grok-imagine-image-quality",
            "lineage-root",
            "recipe-hash",
            "input-hash",
        )
        .is_ok());
        for manifest in [
            AuthorizationManifestV1 {
                max_total_requests: 2,
                ..canonical.clone()
            },
            AuthorizationManifestV1 {
                max_total_provider_operations: None,
                ..canonical.clone()
            },
            AuthorizationManifestV1 {
                allowed_models: vec!["grok-imagine-image-2.0".into()],
                ..canonical.clone()
            },
            AuthorizationManifestV1 {
                source_lineage_root_job_id: Some("other".into()),
                ..canonical
            },
        ] {
            assert_eq!(
                validate_front_authoritative_grid_authorization_manifest(
                    &manifest,
                    &request,
                    "grok-imagine-image-quality",
                    "lineage-root",
                    "recipe-hash",
                    "input-hash",
                )
                .unwrap_err()
                .0,
                "front_authoritative_direction_grid_authorization_scope_mismatch"
            );
        }
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn topdown_cycle_v10_authorization_is_exactly_one_video_operation() {
        let request: GenerateCharacterPackRequest = serde_json::from_value(serde_json::json!({
            "schemaVersion": "3",
            "providerId": "xai",
            "profileId": "default",
            "character": { "prompt": "fixture" },
            "cameraProfile": "topdown-three-quarter@1.0.0",
            "equipment": { "kind": "none" },
            "equipmentExplicit": true,
            "directionMotionStage": "complete",
            "validationOnly": true,
            "validationAnimations": ["walk_down"],
            "motionProfile": "biped_walk@1.0.0",
            "metadata": {
                "name": "V10 Walk Down Probe",
                "defaultAnimation": "walk_down",
                "creator": "Forge",
                "license": "private"
            },
            "workflow": { "id": "topdown-cycle", "version": "10.0.0" },
            "generation": {
                "maxAttemptsPerAnimation": 1,
                "targetFrameCount": 12,
                "videoDurationSeconds": 4,
                "imageModel": "grok-imagine-image-quality",
                "videoModel": "grok-imagine-video-1.5",
                "poseGuidance": "disabled"
            },
            "reuseFromJobDir": "/fixture/approved-v9-source"
        }))
        .unwrap();
        let operation = AutomationOperation::GenerateCharacterPack(request.clone());
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["walk_down:video"]
        );

        let now = chrono::Utc::now();
        let canonical = AuthorizationManifestV1 {
            schema_version: AUTHORIZATION_MANIFEST_SCHEMA_VERSION.into(),
            authorization_id: "v10-exact".into(),
            provider_id: "xai".into(),
            profile_id: "default".into(),
            allowed_models: vec!["grok-imagine-video-1.5".into()],
            created_at: now,
            expires_at: now + chrono::Duration::minutes(30),
            max_total_requests: 1,
            max_total_provider_operations: Some(1),
            max_total_cost_ticks: V10_WALK_DOWN_MAX_COST_TICKS,
            allowed_targets: vec![AuthorizedTargetV1 {
                target_id: "walk_down:video".into(),
                max_requests: 1,
                cost_reservation_ticks_per_request: V10_WALK_DOWN_MAX_COST_TICKS,
            }],
            source_lineage_root_job_id: Some("lineage-root".into()),
            recipe_hash: Some("recipe-hash".into()),
            input_fingerprint: Some("input-hash".into()),
        };
        assert!(validate_v10_authorization_manifest(
            &canonical,
            &request,
            "grok-imagine-video-1.5",
            "lineage-root",
            "recipe-hash",
            "input-hash",
        )
        .is_ok());

        let mut invalid = Vec::new();
        let mut value = canonical.clone();
        value.provider_id = "fixture".into();
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_models.push("grok-imagine-video".into());
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_requests = 2;
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_provider_operations = Some(2);
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_cost_ticks += 1;
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_targets[0].target_id = "walk_up:video".into();
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_targets.push(AuthorizedTargetV1 {
            target_id: "walk_up:video".into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: V10_WALK_DOWN_MAX_COST_TICKS,
        });
        invalid.push(value);
        let mut value = canonical.clone();
        value.source_lineage_root_job_id = Some("other-lineage".into());
        invalid.push(value);
        let mut value = canonical.clone();
        value.recipe_hash = Some("other-recipe".into());
        invalid.push(value);
        let mut value = canonical.clone();
        value.input_fingerprint = Some("other-input".into());
        invalid.push(value);

        for manifest in invalid {
            let error = validate_v10_authorization_manifest(
                &manifest,
                &request,
                "grok-imagine-video-1.5",
                "lineage-root",
                "recipe-hash",
                "input-hash",
            )
            .unwrap_err();
            assert_eq!(error.0, "topdown_cycle_v10_authorization_scope_mismatch");
        }

        let mut local = request;
        local.workflow.version = "10.1.0".into();
        local.reuse_from_job_dir = Some(PathBuf::from("/fixture/failed-v10-source"));
        local.retry_animations = vec!["walk_down".into()];
        local
            .retry_stages
            .insert("walk_down".into(), CharacterRetryStage::Loop);
        assert!(
            operation_authorization_targets(&AutomationOperation::GenerateCharacterPack(local))
                .unwrap()
                .is_empty()
        );
    }

    #[cfg(feature = "grid-generation")]
    #[test]
    fn grid_v93_authorization_manifest_is_exactly_one_operation_and_one_frame() {
        let request: GenerateCharacterPackRequest = serde_json::from_value(serde_json::json!({
            "schemaVersion": "3",
            "providerId": "xai",
            "profileId": "default",
            "character": { "prompt": "fixture" },
            "cameraProfile": "topdown-three-quarter@1.0.0",
            "equipment": { "kind": "none" },
            "equipmentExplicit": true,
            "directionMotionStage": "complete",
            "validationOnly": true,
            "validationAnimations": ["walk_down"],
            "metadata": {
                "name": "Grid V9.3 Walk Down Probe",
                "defaultAnimation": "walk_down",
                "creator": "Forge",
                "license": "private"
            },
            "workflow": { "id": "topdown-grid", "version": "9.3.0" },
            "generation": {
                "targetFrameCount": 4,
                "poseGuidance": "grayscale",
                "imageModel": "grok-imagine-image-quality"
            },
            "reuseFromJobDir": "/fixture/v92-source",
            "retryAnimations": ["walk_down"],
            "retryStages": { "walk_down": "frame" },
            "retryFrames": { "walk_down": [2] }
        }))
        .unwrap();
        let now = chrono::Utc::now();
        let canonical = AuthorizationManifestV1 {
            schema_version: AUTHORIZATION_MANIFEST_SCHEMA_VERSION.into(),
            authorization_id: "v93-exact".into(),
            provider_id: "xai".into(),
            profile_id: "default".into(),
            allowed_models: vec!["grok-imagine-image-quality".into()],
            created_at: now,
            expires_at: now + chrono::Duration::minutes(30),
            max_total_requests: 1,
            max_total_provider_operations: Some(1),
            max_total_cost_ticks: V93_ASYMMETRIC_MAX_COST_TICKS,
            allowed_targets: vec![AuthorizedTargetV1 {
                target_id: "walk_down:frame:2".into(),
                max_requests: 1,
                cost_reservation_ticks_per_request: V93_ASYMMETRIC_MAX_COST_TICKS,
            }],
            source_lineage_root_job_id: Some("lineage-root".into()),
            recipe_hash: Some("recipe-hash".into()),
            input_fingerprint: Some("input-hash".into()),
        };
        assert!(validate_v93_authorization_manifest(
            &canonical,
            &request,
            "grok-imagine-image-quality",
            "lineage-root",
            "recipe-hash",
            "input-hash",
        )
        .is_ok());

        let mut invalid = Vec::new();
        let mut value = canonical.clone();
        value.provider_id = "fixture".into();
        invalid.push(value);
        let mut value = canonical.clone();
        value.profile_id = "other".into();
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_models.clear();
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_models.push("extra-model".into());
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_requests = 2;
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_provider_operations = None;
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_provider_operations = Some(2);
        invalid.push(value);
        let mut value = canonical.clone();
        value.max_total_cost_ticks -= 1;
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_targets[0].target_id = "walk_down:frame:1".into();
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_targets[0].max_requests = 2;
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_targets[0].cost_reservation_ticks_per_request -= 1;
        invalid.push(value);
        let mut value = canonical.clone();
        value.allowed_targets.push(AuthorizedTargetV1 {
            target_id: "walk_down:frame:3".into(),
            max_requests: 1,
            cost_reservation_ticks_per_request: V93_ASYMMETRIC_MAX_COST_TICKS,
        });
        invalid.push(value);
        let mut value = canonical.clone();
        value.source_lineage_root_job_id = None;
        invalid.push(value);
        let mut value = canonical.clone();
        value.recipe_hash = None;
        invalid.push(value);
        let mut value = canonical;
        value.input_fingerprint = Some("other-input".into());
        invalid.push(value);

        for manifest in invalid {
            let error = validate_v93_authorization_manifest(
                &manifest,
                &request,
                "grok-imagine-image-quality",
                "lineage-root",
                "recipe-hash",
                "input-hash",
            )
            .unwrap_err();
            assert_eq!(error.0, "asymmetric_gait_authorization_scope_mismatch");
        }
    }

    #[test]
    fn reused_video_character_authorization_declares_only_paid_retry_stages() {
        let mut stages = std::collections::BTreeMap::new();
        stages.insert("walk_up".into(), CharacterRetryStage::Still);
        assert_eq!(
            video_character_authorization_targets(&["walk_up"], &stages, false),
            vec!["walk_up:still", "walk_up:video"]
        );

        stages.insert("walk_up".into(), CharacterRetryStage::Video);
        assert_eq!(
            video_character_authorization_targets(&["walk_up"], &stages, false),
            vec!["walk_up:video"]
        );

        stages.insert("walk_up".into(), CharacterRetryStage::Consistency);
        assert!(video_character_authorization_targets(&["walk_up"], &stages, false).is_empty());
    }

    #[test]
    fn new_video_character_authorization_includes_subject_and_all_media_targets() {
        let targets = video_character_authorization_targets(
            &["idle", "walk_up"],
            &std::collections::BTreeMap::new(),
            true,
        );
        assert_eq!(
            targets,
            vec![
                "subject_reference",
                "idle:still",
                "idle:video",
                "walk_up:still",
                "walk_up:video"
            ]
        );
    }

    #[test]
    fn topdown_video_v2_reuses_subject_lock_without_subject_reference_authorization() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-orthographic@2.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "validationOnly": true,
                "validationAnimations": ["walk_right"],
                "metadata": {
                    "name": "Walk Right Video Validation",
                    "defaultAnimation": "walk_right",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-video", "version": "2.0.0" }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["walk_right:still", "walk_right:video"]
        );
    }

    #[test]
    fn keyframe_direction_validation_authorizes_only_selected_frames() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "validationOnly": true,
                "validationAnimations": ["walk_right"],
                "metadata": {
                    "name": "Walk Right Validation",
                    "defaultAnimation": "walk_right",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-keyframes", "version": "2.3.0" }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            (0..8)
                .map(|frame| format!("walk_right:frame:{frame}"))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn spritesheet_authorization_is_one_target_per_selected_animation() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "validationOnly": true,
                "validationAnimations": ["walk_right"],
                "metadata": {
                    "name": "Walk Right Sheet Validation",
                    "defaultAnimation": "walk_right",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-spritesheet", "version": "3.0.0" },
                "generation": { "targetFrameCount": 4 }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["walk_right"]
        );

        let mut local_replay = operation;
        let AutomationOperation::GenerateCharacterPack(request) = &mut local_replay else {
            unreachable!();
        };
        request.validation_only = false;
        request.validation_animations.clear();
        request.retry_animations = vec!["walk_right".into()];
        request
            .retry_stages
            .insert("walk_right".into(), CharacterRetryStage::Consistency);
        assert!(operation_authorization_targets(&local_replay)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn locked_frames_authorization_separates_direction_sheet_action_and_frame_retry() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "validationOnly": true,
                "validationAnimations": ["walk_right"],
                "metadata": {
                    "name": "Walk Right Locked Frames Validation",
                    "defaultAnimation": "walk_right",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-frames", "version": "4.0.0" },
                "generation": { "targetFrameCount": 4 }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["direction_lock", "walk_right"]
        );

        let mut frame_retry = operation;
        let AutomationOperation::GenerateCharacterPack(request) = &mut frame_retry else {
            unreachable!();
        };
        request.reuse_from_job_dir = Some(PathBuf::from("/fixture/source-job"));
        request.retry_animations = vec!["walk_right".into()];
        request
            .retry_stages
            .insert("walk_right".into(), CharacterRetryStage::Frame);
        request.retry_frames.insert("walk_right".into(), vec![2]);
        assert_eq!(
            operation_authorization_targets(&frame_retry).unwrap(),
            vec!["walk_right:frame:2"]
        );
    }

    #[test]
    fn locked_video_authorization_uses_direction_sheet_and_video_only() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "validationOnly": true,
                "validationAnimations": ["walk_up"],
                "metadata": {
                    "name": "Locked Video Validation",
                    "defaultAnimation": "walk_up",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-video-locked", "version": "5.0.0" },
                "generation": { "targetFrameCount": 8 }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["direction_lock", "walk_up:video"]
        );
    }

    #[test]
    fn video_cycle_authorization_uses_direction_sheet_and_video_only() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "validationOnly": true,
                "validationAnimations": ["walk_up"],
                "metadata": {
                    "name": "Video Cycle Validation",
                    "defaultAnimation": "walk_up",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-video-cycle", "version": "6.0.0" },
                "generation": { "targetFrameCount": 8 }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec!["direction_lock", "walk_up:video"]
        );

        let mut local_retry = operation;
        let AutomationOperation::GenerateCharacterPack(request) = &mut local_retry else {
            unreachable!();
        };
        request.reuse_from_job_dir = Some(PathBuf::from("/fixture/source-job"));
        request.retry_animations = vec!["walk_up".into()];
        request
            .retry_stages
            .insert("walk_up".into(), CharacterRetryStage::Loop);
        assert!(operation_authorization_targets(&local_retry)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn direction_motion_authorization_is_split_between_images_and_videos() {
        let image_stage: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "one complete test subject" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "directionMotionStage": "image_locks",
                "metadata": {
                    "name": "Direction Motion Images",
                    "defaultAnimation": "idle_down",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-direction-motion", "version": "8.0.0" },
                "generation": { "targetFrameCount": 12 }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&image_stage).unwrap(),
            [
                "back_idle",
                "back_walk",
                "front_idle",
                "front_walk",
                "left_idle",
                "left_walk",
                "right_idle",
                "right_walk",
            ]
        );

        let mut image_retry = image_stage.clone();
        let AutomationOperation::GenerateCharacterPack(request) = &mut image_retry else {
            unreachable!();
        };
        request.reuse_from_job_dir = Some(PathBuf::from("/fixture/image-lock-job"));
        request.retry_animations = vec!["idle_up".into()];
        request
            .retry_stages
            .insert("idle_up".into(), CharacterRetryStage::Still);
        assert_eq!(
            operation_authorization_targets(&image_retry).unwrap(),
            ["back_idle", "back_walk"]
        );

        let mut video_stage = image_stage;
        let AutomationOperation::GenerateCharacterPack(request) = &mut video_stage else {
            unreachable!();
        };
        request.direction_motion_stage = DirectionMotionGenerationStageV1::Complete;
        request.reuse_from_job_dir = Some(PathBuf::from("/fixture/approved-image-lock-job"));
        let video_stage = video_stage.clone();
        assert_eq!(
            operation_authorization_targets(&video_stage).unwrap(),
            [
                "walk_down:video",
                "walk_left:video",
                "walk_right:video",
                "walk_up:video",
            ]
        );

        let mut video_stage = video_stage;
        let AutomationOperation::GenerateCharacterPack(request) = &mut video_stage else {
            unreachable!();
        };
        request.validation_only = true;
        request.validation_animations = vec!["walk_right".into()];
        request.metadata.default_animation = "walk_right".into();
        assert_eq!(
            operation_authorization_targets(&video_stage).unwrap(),
            ["walk_right:video"]
        );
    }

    #[test]
    fn direction_pose_authorization_separates_direction_pose_and_video_targets() {
        let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
            "kind": "generate_character_pack",
            "request": {
                "schemaVersion": "3",
                "providerId": "xai",
                "profileId": "default",
                "character": { "prompt": "fixture" },
                "cameraProfile": "topdown-three-quarter@1.0.0",
                "equipment": { "kind": "none" },
                "equipmentExplicit": true,
                "metadata": {
                    "name": "Direction Pose Character",
                    "defaultAnimation": "idle",
                    "creator": "Forge",
                    "license": "private"
                },
                "workflow": { "id": "topdown-direction-poses", "version": "7.0.0" },
                "generation": { "targetFrameCount": 8 }
            }
        }))
        .unwrap();
        assert_eq!(
            operation_authorization_targets(&operation).unwrap(),
            vec![
                "direction_rear",
                "direction_right",
                "walk_down:pose",
                "walk_down:video",
                "walk_right:pose",
                "walk_right:video",
                "walk_up:pose",
                "walk_up:video",
            ]
        );

        let mut validation = operation;
        let AutomationOperation::GenerateCharacterPack(request) = &mut validation else {
            unreachable!();
        };
        request.validation_only = true;
        request.validation_animations = vec!["walk_right".into()];
        request.metadata.default_animation = "walk_right".into();
        assert_eq!(
            operation_authorization_targets(&validation).unwrap(),
            vec!["direction_right", "walk_right:pose", "walk_right:video"]
        );
    }
}
