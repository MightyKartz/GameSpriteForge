use std::path::{Path, PathBuf};

use clap::Args;
use forge_core::{catalog::CatalogError, library};

#[derive(Args)]
pub struct MergeArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    base: PathBuf,
    #[arg(long)]
    ours: PathBuf,
    #[arg(long)]
    theirs: PathBuf,
    #[arg(long, requires = "expected_sha256")]
    apply: bool,
    #[arg(long, requires = "apply")]
    expected_sha256: Option<String>,
    #[command(flatten)]
    json: crate::JsonFlag,
}
pub fn merge(args: MergeArgs) -> Result<(), (String, String)> {
    crate::success(
        &library::merge::run(
            &args.project,
            &args.base,
            &args.ours,
            &args.theirs,
            args.expected_sha256.as_deref(),
        )
        .map_err(error)?,
    )
}

#[derive(Args)]
pub struct AuditArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    rebuild_index: bool,
    #[command(flatten)]
    json: crate::JsonFlag,
}
#[derive(Args)]
pub struct BindRootArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    root_id: String,
    #[arg(long)]
    path: PathBuf,
    #[command(flatten)]
    json: crate::JsonFlag,
}
pub fn audit(args: AuditArgs) -> Result<(), (String, String)> {
    let report = library::audit::verify(&args.project).map_err(error)?;
    let index = args
        .rebuild_index
        .then(|| library::index::rebuild(&args.project))
        .transpose()
        .map_err(error)?;
    crate::success(&serde_json::json!({"audit": report, "index": index}))
}
pub fn bind_root(args: BindRootArgs) -> Result<(), (String, String)> {
    crate::success(
        &library::audit::bind_root(&args.project, &args.root_id, &args.path).map_err(error)?,
    )
}

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
    purpose: Option<String>,
    #[arg(long, value_parser = ["candidate", "selected", "discarded"])]
    disposition: Option<String>,
    #[arg(long)]
    status: Option<String>,
    #[arg(long, requires = "review_verdict")]
    review_domain: Option<String>,
    #[arg(long, requires = "review_domain")]
    review_verdict: Option<String>,
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
        purpose: args.purpose,
        disposition: args.disposition,
        review_domain: args.review_domain,
        review_verdict: args.review_verdict,
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

#[derive(Args)]
pub struct ReviewArgs {
    #[command(flatten)]
    version: VersionArgs,
    #[arg(long, value_parser = ["technical", "visual", "auditory", "license"])]
    domain: String,
    #[arg(long, value_parser = ["approved", "rejected", "needs_review", "unknown"])]
    verdict: String,
    #[arg(long)]
    statement: String,
    #[arg(long)]
    reviewer: String,
    #[arg(long)]
    evidence: PathBuf,
}
#[derive(Args)]
pub struct PreviewArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long, conflicts_with_all = ["query", "kind", "tag"])]
    id: Vec<String>,
    #[arg(long)]
    query: Option<String>,
    #[arg(long)]
    kind: Option<String>,
    #[arg(long)]
    tag: Option<String>,
    #[arg(long, default_value_t = 20)]
    limit: usize,
    #[arg(long, default_value_t = 0)]
    offset: usize,
    /// Repeat to compare exact versions; defaults to this asset's complete history (up to 100).
    #[arg(long, requires = "id")]
    revision: Vec<String>,
    #[arg(long)]
    out: PathBuf,
    #[command(flatten)]
    json: crate::JsonFlag,
}
#[derive(Args)]
pub struct AnnotateArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    id: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long, conflicts_with = "clear_purpose")]
    purpose: Option<String>,
    #[arg(long)]
    clear_purpose: bool,
    #[arg(long, conflicts_with = "clear_tags")]
    tag: Vec<String>,
    #[arg(long)]
    clear_tags: bool,
    #[command(flatten)]
    json: crate::JsonFlag,
}
pub fn review(args: ReviewArgs) -> Result<(), (String, String)> {
    let request = library::review::ReviewRequest {
        reference: args.version.reference(),
        domain: args.domain,
        verdict: args.verdict,
        statement: args.statement,
        reviewer: args.reviewer,
        evidence: args.evidence,
    };
    crate::success(&library::review::record(&args.version.project, &request).map_err(error)?)
}
pub fn reviews(args: VersionArgs) -> Result<(), (String, String)> {
    let catalog = library::read_catalog(&args.project).map_err(error)?;
    let asset = library::read_asset(&args.project, &catalog, &args.id).map_err(error)?;
    library::read_revision(&args.project, &asset, &args.revision).map_err(error)?;
    crate::success(
        &library::review::read_reviews(&args.project, &asset, &args.revision).map_err(error)?,
    )
}
pub fn preview(args: PreviewArgs) -> Result<(), (String, String)> {
    let (references, selection) = if args.id.is_empty() {
        let found = library::intake::search(
            &args.project,
            &library::intake::SearchFilter {
                query: args.query,
                kind: args.kind,
                tag: args.tag,
                limit: args.limit,
                offset: args.offset,
                ..Default::default()
            },
        )
        .map_err(error)?;
        let selection =
            serde_json::json!({"total":found.total,"offset":found.offset,"limit":found.limit});
        (
            found
                .items
                .into_iter()
                .map(|hit| library::delivery::VersionRef {
                    asset_id: hit.asset_id,
                    revision: hit.revision,
                })
                .collect::<Vec<_>>(),
            selection,
        )
    } else {
        if !args.revision.is_empty() && args.id.len() != 1 {
            return Err((
                "invalid_arguments".into(),
                "explicit revisions require exactly one --id".into(),
            ));
        }
        let mut references = vec![];
        for id in args.id {
            let revisions = if args.revision.is_empty() {
                library::intake::history(&args.project, &id)
                    .map_err(error)?
                    .into_iter()
                    .map(|hit| hit.revision)
                    .collect()
            } else {
                args.revision.clone()
            };
            references.extend(revisions.into_iter().map(|revision| {
                library::delivery::VersionRef {
                    asset_id: id.clone(),
                    revision,
                }
            }));
        }
        let selection =
            serde_json::json!({"total":references.len(),"offset":0,"limit":references.len()});
        (references, selection)
    };
    let report = library::preview::create(&args.project, &references, &args.out).map_err(error)?;
    let mut value = serde_json::to_value(report).map_err(crate::display_error)?;
    value["selection"] = selection;
    crate::success(&value)
}
pub fn annotate(args: AnnotateArgs) -> Result<(), (String, String)> {
    let tags = if args.clear_tags || !args.tag.is_empty() {
        Some(args.tag)
    } else {
        None
    };
    crate::success(
        &library::review::annotate_metadata(
            &args.project,
            &args.id,
            args.name,
            tags,
            if args.clear_purpose {
                Some(None)
            } else {
                args.purpose.map(Some)
            },
        )
        .map_err(error)?,
    )
}

#[derive(Args)]
pub struct ExportArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(
        long,
        required_unless_present = "asset_lock",
        conflicts_with = "asset_lock",
        requires = "revision"
    )]
    id: Option<String>,
    #[arg(long, requires = "id")]
    revision: Vec<String>,
    #[arg(long, conflicts_with = "id")]
    asset_lock: Option<PathBuf>,
    #[arg(long)]
    out: PathBuf,
    #[command(flatten)]
    json: crate::JsonFlag,
}
#[derive(Args)]
pub struct VerifyBundleArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    expected_sha256: Option<String>,
    #[command(flatten)]
    json: crate::JsonFlag,
}
#[derive(Args)]
pub struct ImportArgs {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    path: PathBuf,
    #[arg(long)]
    expected_sha256: String,
    #[command(flatten)]
    json: crate::JsonFlag,
}
pub fn export(args: ExportArgs) -> Result<(), (String, String)> {
    let references = if let Some(lock) = &args.asset_lock {
        let document: library::delivery::ResourceLock =
            serde_json::from_slice(&std::fs::read(lock).map_err(crate::display_error)?)
                .map_err(crate::display_error)?;
        document
            .assets
            .keys()
            .map(|id| library::delivery::locked_reference(&args.project, id, lock))
            .collect::<Result<Vec<_>, _>>()
            .map_err(error)?
    } else {
        let id = args.id.expect("clap requires selection");
        args.revision
            .into_iter()
            .map(|revision| library::delivery::VersionRef {
                asset_id: id.clone(),
                revision,
            })
            .collect()
    };
    crate::success(
        &library::transfer::export(
            &args.project,
            &references,
            &args.out,
            crate::receipt::identity()?,
            args.asset_lock.as_deref(),
        )
        .map_err(error)?,
    )
}
pub fn verify_bundle(args: VerifyBundleArgs) -> Result<(), (String, String)> {
    crate::success(
        &library::transfer::verify(&args.input, args.expected_sha256.as_deref()).map_err(error)?,
    )
}
pub fn import(args: ImportArgs) -> Result<(), (String, String)> {
    crate::success(
        &library::transfer::import(&args.input, &args.path, &args.expected_sha256)
            .map_err(error)?,
    )
}

#[derive(Args)]
pub struct DispositionArgs {
    #[command(flatten)]
    version: VersionArgs,
    #[arg(long, value_parser = ["candidate", "discarded"])]
    state: String,
}
pub fn disposition(args: DispositionArgs) -> Result<(), (String, String)> {
    crate::success(
        &library::review::set_disposition(
            &args.version.project,
            &args.version.reference(),
            &args.state,
        )
        .map_err(error)?,
    )
}
