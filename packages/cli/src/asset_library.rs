use std::path::{Path, PathBuf};

use clap::Args;
use forge_core::{catalog::CatalogError, library};

#[derive(Args)]
pub struct MigrateArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, conflicts_with = "apply")]
    dry_run: bool,
    #[arg(long, requires = "expected_sha256")]
    apply: bool,
    #[arg(long, requires = "apply")]
    expected_sha256: Option<String>,
    #[command(flatten)]
    json: crate::JsonFlag,
}

pub fn error(error: CatalogError) -> (String, String) {
    (error.code().into(), error.to_string())
}

pub fn initialize(path: &Path, name: &str) -> Result<(), (String, String)> {
    crate::success(&library::initialize(path, name).map_err(error)?)
}

pub fn migrate(args: MigrateArgs) -> Result<(), (String, String)> {
    if !args.apply {
        return crate::success(&library::migration_preview(&args.project).map_err(error)?);
    }
    let name = args.name.unwrap_or_else(|| {
        forge_core::asset_project::read_project(&args.project)
            .map(|project| project.name)
            .unwrap_or_else(|_| "Local assets".into())
    });
    let expected = args.expected_sha256.as_deref().ok_or_else(|| {
        (
            "invalid_arguments".into(),
            "--apply requires --expected-sha256 from the preview".into(),
        )
    })?;
    crate::success(&library::migrate(&args.project, &name, expected).map_err(error)?)
}

#[derive(Args)]
pub struct ScanArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[command(flatten)]
    json: crate::JsonFlag,
}
#[derive(Args)]
pub struct RegisterArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    input: PathBuf,
    #[command(flatten)]
    json: crate::JsonFlag,
}
#[derive(Args)]
pub struct SearchArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    query: Option<String>,
    #[arg(long)]
    kind: Option<String>,
    #[arg(long)]
    tag: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, default_value_t = 0)]
    offset: usize,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[command(flatten)]
    json: crate::JsonFlag,
}
#[derive(Args)]
pub struct HistoryArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    id: String,
    #[command(flatten)]
    json: crate::JsonFlag,
}
pub fn scan(args: ScanArgs) -> Result<(), (String, String)> {
    library::read_catalog(&args.project).map_err(error)?;
    let report = library::intake::scan(&args.root).map_err(error)?;
    // Create-new keeps a prior reviewed scan plan or an existing source intact.
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args.out)
        .map_err(|e| ("scan_output_failed".into(), e.to_string()))?;
    file.write_all(&serde_json::to_vec_pretty(&report).map_err(crate::display_error)?)
        .map_err(crate::display_error)?;
    file.sync_all().map_err(crate::display_error)?;
    crate::success(&report)
}
pub fn register(args: RegisterArgs) -> Result<(), (String, String)> {
    let bytes = std::fs::read(args.input).map_err(crate::display_error)?;
    let mut value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(crate::display_error)?;
    if let Some(object) = value.as_object_mut() {
        object.remove("issues");
    }
    let batch = serde_json::from_value(value).map_err(crate::display_error)?;
    crate::success(&library::intake::register(&args.project, &batch).map_err(error)?)
}
pub fn search(args: SearchArgs) -> Result<(), (String, String)> {
    let filter = library::intake::SearchFilter {
        query: args.query,
        kind: args.kind,
        tag: args.tag,
        status: args.status,
        offset: args.offset,
        limit: args.limit,
    };
    crate::success(&library::intake::search(&args.project, &filter).map_err(error)?)
}
pub fn history(args: HistoryArgs) -> Result<(), (String, String)> {
    crate::success(&library::intake::history(&args.project, &args.id).map_err(error)?)
}

pub fn resolve_binding(binding: &mut Option<library::finalize::ProjectBinding>, root: &Path) {
    if let Some(binding) = binding {
        if binding.project_path.is_relative() {
            binding.project_path = root.join(&binding.project_path);
        }
    }
}
#[derive(Args)]
pub struct RecoverArgs {
    #[arg(long)]
    input: PathBuf,
    #[command(flatten)]
    json: crate::JsonFlag,
}
pub fn recover(args: RecoverArgs) -> Result<(), (String, String)> {
    crate::success(
        &serde_json::json!({"catalogPath":library::finalize::recover(&args.input).map_err(error)?}),
    )
}

#[derive(Args)]
pub struct VersionArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    id: String,
    #[arg(long)]
    revision: String,
    #[command(flatten)]
    json: crate::JsonFlag,
}
impl VersionArgs {
    fn reference(&self) -> library::delivery::VersionRef {
        library::delivery::VersionRef {
            asset_id: self.id.clone(),
            revision: self.revision.clone(),
        }
    }
}
#[derive(Args)]
pub struct LockArgs {
    #[command(flatten)]
    version: VersionArgs,
    #[arg(long)]
    out: PathBuf,
}
pub fn retain(args: VersionArgs) -> Result<(), (String, String)> {
    crate::success(&library::delivery::retain(&args.project, &args.reference()).map_err(error)?)
}
pub fn select(args: VersionArgs) -> Result<(), (String, String)> {
    crate::success(&library::delivery::select(&args.project, &args.reference()).map_err(error)?)
}
pub fn lock(args: LockArgs) -> Result<(), (String, String)> {
    crate::success(
        &library::delivery::write_lock(&args.version.project, &args.version.reference(), &args.out)
            .map_err(error)?,
    )
}
pub fn installations(args: HistoryArgs) -> Result<(), (String, String)> {
    let catalog = library::read_catalog(&args.project).map_err(error)?;
    crate::success(
        &library::read_asset(&args.project, &catalog, &args.id)
            .map_err(error)?
            .installations,
    )
}
