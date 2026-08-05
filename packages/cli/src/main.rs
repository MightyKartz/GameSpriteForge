use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

use clap::{Args, Parser, Subcommand, ValueEnum};
use forge_core::asset_project::{
    export_static_pack, hash_file as hash_asset_file, init_project, read_project, read_style_lock,
    resolve_relative, CharacterAssetSpecV1, CharacterAssetSpecV2, ConsistencyReportV1,
    ConsistencyVerdict, StaticAssetSetSpecV1, StaticPackItem, FORGE_PROJECT_FILE, STYLE_LOCK_FILE,
};
#[cfg(feature = "map-compiler")]
use forge_core::automation::CompileMapRequest;
#[cfg(feature = "consistency-v2")]
use forge_core::automation::CreateSubjectLockRequest;
#[cfg(feature = "building-assets")]
use forge_core::automation::GenerateBuildingKitRequest;
use forge_core::automation::{
    analyze_repair, automation_profile, character_workflow_catalog, prepare_repair_plan,
    run_operation_with_provider, stage_plan_job, AutomationOperation, AutomationPlan,
    CharacterRetryStage, CreateStyleLockRequest, GenerateCharacterPackRequest,
    GenerateStaticAssetSetRequest, GodotInstallRequest, PlanStore, PrepareAssetRequest,
    PrepareCharacterPackRequest,
};
#[cfg(feature = "terrain-assets")]
use forge_core::automation::{CreateEnvironmentLockRequest, GenerateTerrainSetRequest};
#[cfg(feature = "consistency-v2")]
use forge_core::benchmark::{
    plan_character_benchmark, run_character_benchmark, summarize_character_benchmark,
    BenchmarkWorkflow, CharacterBenchmarkExecutionOptions, CharacterBenchmarkManifestV1,
    CharacterBenchmarkRunV1,
};
use forge_core::catalog::read_project_catalog;
#[cfg(feature = "consistency-v2")]
use forge_core::component::{
    inspect_component, install_component, list_components, FixtureVisionComponent, VisionComponent,
    VisionComponentRequestV1, VisionOperation,
};
#[cfg(feature = "game-art-manifest")]
use forge_core::game_art::{
    compute_build_plan, compute_project_diff, GameArtError, GameArtManifestV1,
    ProviderCapabilityInput,
};
use forge_core::job::{JobArtifactRecord, JobRecord, JobStore, JOB_WORKSPACE_JSON};
use forge_core::provider::{CredentialKind, ProviderHealth};
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
use forge_providers::{
    list_provider_health_noninteractive, resolve_image_model as resolve_provider_image_model,
    resolve_provider, resolve_video_model as resolve_provider_video_model, XAI_PROVIDER_ID,
};
use serde::Serialize;

const JSON_SCHEMA_VERSION: &str = "1";

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
    #[cfg(feature = "consistency-v2")]
    Subject {
        #[command(subcommand)]
        command: SubjectCommand,
    },
    #[cfg(feature = "consistency-v2")]
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
}

#[derive(Subcommand)]
enum PackCommand {
    Validate {
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
        #[command(flatten)]
        json: JsonFlag,
    },
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

#[cfg(feature = "consistency-v2")]
#[derive(Subcommand)]
enum SubjectCommand {
    Create(ProjectSpecInput),
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

#[cfg(feature = "consistency-v2")]
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
    Character(ProjectSpecInput),
    IconSet(ProjectSpecInput),
    PropSet(ProjectSpecInput),
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
    #[command(flatten)]
    json: JsonFlag,
}

#[derive(Subcommand)]
enum ProfileCommand {
    CharacterWorkflows(JsonFlag),
}

#[derive(Subcommand)]
enum ProviderCommand {
    List(JsonFlag),
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
        },
        Command::Pack { command } => match command {
            PackCommand::Validate { path, .. } => {
                forge_pack::validate_pack_layout(&path).map_err(display_error)?;
                success(&serde_json::json!({ "path": path, "valid": true }))
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
                                | "contact_sheet"
                                | "provider_manifest"
                                | "provider_usage"
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
                success(&serde_json::json!({
                    "job": record,
                    "reportArtifacts": report_artifacts,
                    "reports": reports,
                    "providerRequestOccurred": provider_requests > 0,
                    "providerRequestCount": provider_requests,
                    "providerCostInUsdTicks": provider_cost_ticks,
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
                wait,
                ..
            } => retry_job(&id, item.as_deref(), frame, stage, wait),
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
                project, manifest, ..
            } => project_plan_build(&project, &manifest),
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
                prepare_and_execute(AutomationOperation::CreateStyleLock(request), input.wait)
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
                };
                prepare_or_plan(
                    AutomationOperation::CreateSubjectLock(request),
                    input.wait,
                    input.plan_only,
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
        #[cfg(feature = "consistency-v2")]
        Command::Schema { command } => match command {
            SchemaCommand::List(_) => success(&serde_json::json!({
                "schemas": [
                    "asset@1.0.0",
                    "character@2.0.0",
                    "character-benchmark@1.0.0",
                    "style@1.0.0",
                    "subject@1.0.0",
                    "map@1.0.0",
                    "game-art-manifest@1.0.0"
                ]
            })),
            SchemaCommand::Show { id, .. } => {
                let source = match id.as_str() {
                    "asset@1.0.0" => include_str!("../../../schemas/asset-spec.schema.json"),
                    "character@2.0.0" => {
                        include_str!("../../../schemas/character-v2.schema.json")
                    }
                    "character-benchmark@1.0.0" => {
                        include_str!("../../../schemas/character-benchmark.schema.json")
                    }
                    "style@1.0.0" => include_str!("../../../schemas/style-spec.schema.json"),
                    "subject@1.0.0" => include_str!("../../../schemas/subject-spec.schema.json"),
                    "map@1.0.0" => include_str!("../../../schemas/map-spec.schema.json"),
                    "game-art-manifest@1.0.0" => {
                        include_str!("../../../schemas/game-art-manifest.schema.json")
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
                prepare_and_execute(AutomationOperation::CompileMap(request), input.wait)
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
                    catalog
                        .workflows
                        .retain(|workflow| workflow.id != "topdown-keyframes");
                }
                success(&catalog)
            }
        },
        Command::Provider { command } => match command {
            ProviderCommand::List(_) => success(&list_provider_health_noninteractive()),
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
            PlanCommand::Execute { token, wait, .. } => {
                let plan = plan_store()?.claim(&token).map_err(display_error)?;
                let store = job_store()?;
                let record = stage_plan_job(&store, &plan).map_err(display_error)?;
                if wait {
                    let result = run_plan_operation(&store, &record.job_id, &plan.operation)?;
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
            run_plan_operation(&store, &job_id, &plan.operation)?;
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
fn project_plan_build(project: &Path, manifest: &Path) -> Result<(), (String, String)> {
    let validated = GameArtManifestV1::load_validated(manifest).map_err(game_art_error)?;
    let diff = compute_project_diff(project, &validated).map_err(game_art_error)?;
    let capabilities = provider_capability_input(
        &validated.manifest.provider.id,
        &validated.manifest.provider.profile_id,
    )?;
    let plan =
        compute_build_plan(project, &validated, &diff, &capabilities).map_err(game_art_error)?;
    let plan_sha256 = plan.plan_sha256();
    // The automation layer pins canonical absolute paths; canonicalizing up
    // front also resolves any symlinked input paths it would reject.
    let canonical_project = project.canonicalize().map_err(io_error)?;
    let canonical_manifest = manifest.canonicalize().map_err(io_error)?;
    // Build the operation through its tagged JSON form so the CLI does not
    // depend on the core facade re-exporting the request type.
    let operation: AutomationOperation = serde_json::from_value(serde_json::json!({
        "kind": "build_project",
        "request": {
            "schemaVersion": "1",
            "projectPath": canonical_project,
            "manifestPath": canonical_manifest,
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
    profile_id: &str,
) -> Result<ProviderCapabilityInput, (String, String)> {
    let provider = resolve_provider(provider_id, profile_id).map_err(display_error)?;
    let capabilities = provider
        .capabilities()
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

fn run_plan_operation(
    store: &JobStore,
    job_id: &str,
    operation: &AutomationOperation,
) -> Result<JobRecord, (String, String)> {
    if let AutomationOperation::GenerateCharacterPack(request) = operation {
        ensure_character_workflow_enabled(request)?;
    }
    if let Some(provider_id) = operation_provider_id(operation) {
        ensure_real_provider_execution(provider_id)?;
    }
    let provider = match operation {
        #[cfg(feature = "game-art-manifest")]
        AutomationOperation::BuildProject(request) => {
            let validated = GameArtManifestV1::load_validated(&request.manifest_path)
                .map_err(game_art_error)?;
            let provider_id = validated.manifest.provider.id.as_str();
            ensure_real_provider_execution(provider_id)?;
            Some(
                resolve_provider(provider_id, &validated.manifest.provider.profile_id)
                    .map_err(display_error)?,
            )
        }
        AutomationOperation::GenerateCharacterPack(request) => Some(
            resolve_provider(&request.provider_id, &request.profile_id).map_err(display_error)?,
        ),
        AutomationOperation::CreateStyleLock(request) => Some(
            resolve_provider(&request.provider_id, &request.profile_id).map_err(display_error)?,
        ),
        AutomationOperation::CreateSubjectLock(request) => Some(
            resolve_provider(&request.provider_id, &request.profile_id).map_err(display_error)?,
        ),
        AutomationOperation::GenerateStaticAssetSet(request) => Some(
            resolve_provider(&request.provider_id, &request.profile_id).map_err(display_error)?,
        ),
        AutomationOperation::CreateEnvironmentLock(request) => Some(
            resolve_provider(&request.provider_id, &request.profile_id).map_err(display_error)?,
        ),
        AutomationOperation::GenerateTerrainSet(request) => Some(
            resolve_provider(&request.provider_id, &request.profile_id).map_err(display_error)?,
        ),
        AutomationOperation::GenerateBuildingKit(request) => Some(
            resolve_provider(&request.provider_id, &request.profile_id).map_err(display_error)?,
        ),
        _ => None,
    };
    run_operation_with_provider(store, job_id, operation, provider.as_deref())
        .map_err(|error| (error.code().into(), error.to_string()))
}

fn operation_provider_id(operation: &AutomationOperation) -> Option<&str> {
    match operation {
        AutomationOperation::GenerateCharacterPack(request) => Some(&request.provider_id),
        AutomationOperation::CreateStyleLock(request) => Some(&request.provider_id),
        AutomationOperation::CreateSubjectLock(request) => Some(&request.provider_id),
        AutomationOperation::GenerateStaticAssetSet(request) => Some(&request.provider_id),
        AutomationOperation::CreateEnvironmentLock(request) => Some(&request.provider_id),
        AutomationOperation::GenerateTerrainSet(request) => Some(&request.provider_id),
        AutomationOperation::GenerateBuildingKit(request) => Some(&request.provider_id),
        _ => None,
    }
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

fn ensure_character_workflow_enabled(
    request: &GenerateCharacterPackRequest,
) -> Result<(), (String, String)> {
    if request.workflow.id == "topdown-keyframes" && !cfg!(feature = "consistency-v2") {
        return Err((
            "feature_not_available".into(),
            "topdown-keyframes@2.0.0 requires a Forge v0.3 consistency-v2 build".into(),
        ));
    }
    Ok(())
}

fn prepare_and_execute(
    mut operation: AutomationOperation,
    wait: bool,
) -> Result<(), (String, String)> {
    resolve_operation_models(&mut operation)?;
    let plan = plan_store()?.prepare(operation).map_err(display_error)?;
    let claimed = plan_store()?.claim(&plan.token).map_err(display_error)?;
    let store = job_store()?;
    let record = stage_plan_job(&store, &claimed).map_err(display_error)?;
    if wait {
        let completed = run_plan_operation(&store, &record.job_id, &claimed.operation)?;
        success(&completed)
    } else {
        spawn_worker(&record)?;
        success(&record)
    }
}

fn prepare_or_plan(
    mut operation: AutomationOperation,
    wait: bool,
    plan_only: bool,
) -> Result<(), (String, String)> {
    resolve_operation_models(&mut operation)?;
    if plan_only {
        let plan = plan_store()?.prepare(operation).map_err(display_error)?;
        return success(&plan);
    }
    prepare_and_execute(operation, wait)
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
            request.generation.video_model = Some(
                resolve_provider_video_model(
                    &request.provider_id,
                    request.generation.video_model.as_deref(),
                )
                .map_err(display_error)?,
            );
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

fn generate_character(input: ProjectSpecInput) -> Result<(), (String, String)> {
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
    let request: GenerateCharacterPackRequest = if schema_version == "1" {
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
        let spec: CharacterAssetSpecV2 = serde_json::from_value(value).map_err(json_error)?;
        if spec.kind != "character"
            || spec.workflow.id != "topdown-keyframes"
            || spec.workflow.version != "2.0.0"
        {
            return Err((
                "invalid_asset_spec".into(),
                "Character V2 requires kind character and workflow topdown-keyframes@2.0.0".into(),
            ));
        }
        let subject_path =
            subject_lock_path(&input.project, &spec.subject.id, &spec.subject.revision);
        let subject = read_subject_lock(&subject_path).map_err(display_error)?;
        if subject.style_revision != revision {
            return Err((
                "subject_style_mismatch".into(),
                format!(
                    "SubjectLock uses style revision {}, but the project locks {}",
                    subject.style_revision, revision
                ),
            ));
        }
        serde_json::from_value(serde_json::json!({
            "schemaVersion": "3",
            "providerId": project.provider.id,
            "profileId": project.provider.profile_id,
            "projectPath": input.project,
            "assetId": spec.id,
            "styleLockPath": style_lock_path,
            "subjectLockPath": subject_path,
            "character": {
                "prompt": subject.prompt,
                "referenceImagePath": subject.canonical_path,
            },
            "metadata": {
                "name": spec.name,
                "defaultAnimation": "idle",
                "creator": "Game Sprite Forge",
                "license": spec.license,
            },
            "workflow": { "id": "topdown-keyframes", "version": "2.0.0" },
            "generation": { "maxAttemptsPerAnimation": 2, "targetFrameCount": 8, "videoDurationSeconds": 4 },
            "quality": { "requireGameReady": true }
        }))
        .map_err(json_error)?
    } else {
        return Err((
            "invalid_asset_spec".into(),
            "character spec requires schemaVersion 1 or 2".into(),
        ));
    };
    prepare_or_plan(
        AutomationOperation::GenerateCharacterPack(request),
        input.wait,
        input.plan_only,
    )
}

fn generate_static(input: ProjectSpecInput, expected_kind: &str) -> Result<(), (String, String)> {
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
        .join(revision)
        .join(STYLE_LOCK_FILE);
    let mut spec: StaticAssetSetSpecV1 =
        serde_json::from_slice(&fs::read(&input.spec).map_err(io_error)?).map_err(json_error)?;
    if spec.kind.as_str() != expected_kind || spec.schema_version != "1" {
        return Err((
            "invalid_asset_spec".into(),
            format!("asset spec kind must be {expected_kind}"),
        ));
    }
    let root = input.spec.parent().unwrap_or(Path::new("."));
    for item in &mut spec.items {
        if let Some(reference) = &item.reference_image {
            item.reference_image = Some(resolve_relative(root, reference));
        }
    }
    let request = GenerateStaticAssetSetRequest {
        schema_version: "4".into(),
        project_path: input.project,
        style_lock_path,
        provider_id: project.provider.id,
        profile_id: project.provider.profile_id,
        asset: spec,
        max_attempts_per_item: 2,
        image_model: None,
        reuse_from_job_dir: None,
        retry_item_ids: vec![],
        consistency_recheck_only: false,
    };
    prepare_and_execute(
        AutomationOperation::GenerateStaticAssetSet(request),
        input.wait,
    )
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
    prepare_and_execute(AutomationOperation::GenerateTerrainSet(request), input.wait)
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
    )
}

fn retry_job(
    id: &str,
    item: Option<&str>,
    frame: Option<u8>,
    stage: CharacterRetryStageArg,
    wait: bool,
) -> Result<(), (String, String)> {
    let source = job_store()?.read_record(id).map_err(display_error)?;
    let recipe = source.recipe.clone().ok_or_else(|| {
        (
            "recipe_missing".into(),
            "job has no immutable recipe".into(),
        )
    })?;
    let mut operation: AutomationOperation = serde_json::from_value(recipe).map_err(json_error)?;
    match &mut operation {
        AutomationOperation::GenerateStaticAssetSet(request) => {
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
                request.retry_item_ids = if let Some(item_id) = item {
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
                    vec![item_id.into()]
                } else {
                    request
                        .asset
                        .items
                        .iter()
                        .map(|candidate| candidate.id.clone())
                        .collect()
                };
                request.consistency_recheck_only = true;
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
                request.retry_item_ids = vec![item_id.into()];
            }
        }
        AutomationOperation::GenerateCharacterPack(request) if item.is_some() => {
            let is_keyframe =
                request.workflow.id == "topdown-keyframes" && request.workflow.version == "2.0.0";
            if stage == CharacterRetryStageArg::Consistency && !is_keyframe {
                return Err((
                    "unsupported_retry_stage".into(),
                    "Character --stage consistency requires topdown-keyframes@2.0.0".into(),
                ));
            }
            let animation = item.unwrap();
            if !matches!(animation, "idle" | "walk_up" | "walk_right" | "walk_down") {
                return Err((
                    "item_not_found".into(),
                    format!("job has no animation {animation}"),
                ));
            }
            if frame.is_some() && !is_keyframe {
                return Err((
                    "unsupported_retry_frame".into(),
                    "--frame is only available for topdown-keyframes@2.0.0".into(),
                ));
            }
            if stage == CharacterRetryStageArg::Frame && frame.is_none() {
                return Err((
                    "retry_frame_required".into(),
                    "--stage frame requires --frame <0-7>".into(),
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
            let selected_stage = if frame.is_some() {
                CharacterRetryStage::Frame
            } else {
                select_character_retry_stage(&source, animation, stage)
            };
            request.reuse_from_job_dir = Some(source.job_dir.clone());
            request.retry_animations = vec![animation.into()];
            request.retry_frames.clear();
            if let Some(frame) = frame {
                request.retry_frames.insert(animation.into(), vec![frame]);
            }
            request
                .retry_stages
                .insert(animation.into(), selected_stage);
        }
        AutomationOperation::GenerateCharacterPack(request) => {
            let is_keyframe =
                request.workflow.id == "topdown-keyframes" && request.workflow.version == "2.0.0";
            if stage == CharacterRetryStageArg::Consistency && is_keyframe {
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
            } else if stage != CharacterRetryStageArg::Auto {
                return Err((
                    "retry_item_required".into(),
                    "Character retry stages require --item <animation>".into(),
                ));
            }
        }
        _ => {
            return Err((
                "unsupported_retry".into(),
                "only generated character and static set jobs can be retried".into(),
            ))
        }
    }
    prepare_and_execute(operation, wait)
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
        "direction_still" => CharacterRetryStageArg::Still,
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
    retry_job(id, item, node.frame, stage, wait)
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
    if record.lifecycle_state != forge_core::job::JobLifecycleState::AwaitingReview {
        return Err((
            "job_not_awaiting_review".into(),
            "job is not awaiting review".into(),
        ));
    }
    let path = record.job_dir.join("review-decision.json");
    fs::write(&path, serde_json::to_vec_pretty(&serde_json::json!({
        "schemaVersion": "1", "accepted": accept, "reason": reason, "reviewedAt": chrono::Utc::now()
    })).map_err(json_error)?).map_err(io_error)?;
    let decision_sha256 = hash_asset_file(&path).map_err(display_error)?;
    if accept {
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
        let operation: AutomationOperation =
            serde_json::from_value(record.recipe.clone().ok_or_else(|| {
                (
                    "recipe_missing".into(),
                    "job has no immutable recipe".into(),
                )
            })?)
            .map_err(json_error)?;
        if let AutomationOperation::GenerateStaticAssetSet(request) = operation {
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
            fs::write(
                &report_path,
                serde_json::to_vec_pretty(&report).map_err(json_error)?,
            )
            .map_err(io_error)?;
            let report_sha256 = hash_asset_file(&report_path).map_err(display_error)?;
            let style = read_style_lock(&request.style_lock_path).map_err(display_error)?;
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
            let output = export_static_pack(
                &record.job_dir.join("exports"),
                &request.asset,
                &style,
                &request.provider_id,
                &items,
                &report,
            )
            .map_err(display_error)?;
            let updated = store
                .update_record(id, |record| {
                    if let Some(artifact) = record
                        .artifacts
                        .iter_mut()
                        .find(|artifact| artifact.kind == "consistency_report")
                    {
                        artifact.sha256 = Some(report_sha256.clone());
                    }
                    record.artifacts.push(JobArtifactRecord {
                        kind: "review_decision".into(),
                        path: path.clone(),
                        sha256: Some(decision_sha256.clone()),
                    });
                    record.artifacts.push(JobArtifactRecord {
                        kind: "gsfpack".into(),
                        path: output.pack_dir.clone(),
                        sha256: None,
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
    let updated = store
        .update_record(id, |record| {
            record.artifacts.push(JobArtifactRecord {
                kind: "review_decision".into(),
                path: path.clone(),
                sha256: Some(decision_sha256.clone()),
            });
            record.error_summary =
                Some("review rejected; targeted regeneration is required".into());
            record.next_actions = vec!["retry_item".into(), "job_report".into()];
        })
        .map_err(display_error)?;
    success(&updated)
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
