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
