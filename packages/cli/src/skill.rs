//! An offline usage guide and optional installer backed by one embedded bundle.
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const NAME: &str = "forge-use";
const MANIFEST: &str = ".forge-skill-manifest.json";
const MANAGER: &str = "forge-cli";
const SCHEMA_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);
type Result<T> = std::result::Result<T, (String, String)>;

struct SourceFile {
    resource: GuideResource,
    content: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct GuideResource {
    topic: &'static str,
    path: &'static str,
    media_type: &'static str,
}

const SOURCE_FILES: &[SourceFile] = &[
    SourceFile {
        resource: GuideResource {
            topic: "overview",
            path: "SKILL.md",
            media_type: "text/markdown",
        },
        content: include_str!("../../../.agents/skills/forge-use/SKILL.md"),
    },
    SourceFile {
        resource: GuideResource {
            topic: "static",
            path: "references/local-static.md",
            media_type: "text/markdown",
        },
        content: include_str!("../../../.agents/skills/forge-use/references/local-static.md"),
    },
    SourceFile {
        resource: GuideResource {
            topic: "provider",
            path: "references/provider.md",
            media_type: "text/markdown",
        },
        content: include_str!("../../../.agents/skills/forge-use/references/provider.md"),
    },
    SourceFile {
        resource: GuideResource {
            topic: "animation",
            path: "references/animation.md",
            media_type: "text/markdown",
        },
        content: include_str!("../../../.agents/skills/forge-use/references/animation.md"),
    },
    SourceFile {
        resource: GuideResource {
            topic: "delivery",
            path: "references/delivery.md",
            media_type: "text/markdown",
        },
        content: include_str!("../../../.agents/skills/forge-use/references/delivery.md"),
    },
    SourceFile {
        resource: GuideResource {
            topic: "static-example",
            path: "examples/local-static.json",
            media_type: "application/json",
        },
        content: include_str!("../../../.agents/skills/forge-use/examples/local-static.json"),
    },
    SourceFile {
        resource: GuideResource {
            topic: "provider-example",
            path: "examples/provider-icons.json",
            media_type: "application/json",
        },
        content: include_str!("../../../.agents/skills/forge-use/examples/provider-icons.json"),
    },
];

#[derive(Args)]
pub struct GuideArgs {
    /// Short name or exact bundled path. Use --json to discover all resources.
    #[arg(default_value = "overview", value_name = "RESOURCE")]
    resource: String,
    /// Return content, build identity, hashes, and the resource index as JSON.
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
pub enum SkillCommand {
    /// Show the embedded SKILL.md, or every bundle file with --json.
    Show {
        #[arg(long)]
        json: bool,
    },
    /// Optionally install the bundled skill for Codex discovery, preserving local changes.
    Install(ScopeArgs),
    /// Inspect an installation without changing files.
    Check(ScopeArgs),
}

#[derive(Args)]
#[group(skip)]
pub struct ScopeArgs {
    /// Existing project directory; no Godot project or Git repository is required.
    #[arg(long, required_unless_present = "user", conflicts_with = "user")]
    project: Option<PathBuf>,
    /// Install for this user under ~/.agents/skills/forge-use.
    #[arg(long, required_unless_present = "project", conflicts_with = "project")]
    user: bool,
    #[arg(long, global = false)]
    json: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BundleIdentity {
    name: String,
    schema_version: u32,
    cli_version: String,
    build: serde_json::Value,
    content_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    #[serde(flatten)]
    identity: BundleIdentity,
    managed_by: String,
    files: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct BundleFile {
    path: String,
    sha256: String,
    content: String,
}

#[derive(Serialize)]
struct Bundle {
    #[serde(flatten)]
    identity: BundleIdentity,
    files: Vec<BundleFile>,
}

#[derive(Debug, Serialize)]
struct Guide<'a> {
    #[serde(flatten)]
    identity: &'a BundleIdentity,
    #[serde(flatten)]
    file: &'a BundleFile,
    resources: Vec<GuideResource>,
}

impl Bundle {
    fn embedded() -> Self {
        Self::new(
            &SOURCE_FILES
                .iter()
                .map(|source| (source.resource.path, source.content))
                .collect::<Vec<_>>(),
        )
    }

    fn new(files: &[(&str, &str)]) -> Self {
        let files: Vec<_> = files
            .iter()
            .map(|(path, content)| BundleFile {
                path: (*path).to_owned(),
                sha256: digest(content.as_bytes()),
                content: (*content).to_owned(),
            })
            .collect();
        let hashes = files
            .iter()
            .map(|file| (file.path.clone(), file.sha256.clone()))
            .collect();
        Self {
            identity: BundleIdentity {
                name: NAME.into(),
                schema_version: SCHEMA_VERSION,
                cli_version: env!("CARGO_PKG_VERSION").into(),
                build: serde_json::to_value(crate::build_info::current())
                    .expect("build identity serializes"),
                content_hash: content_hash(&hashes),
            },
            files,
        }
    }

    fn manifest(&self) -> Manifest {
        Manifest {
            identity: self.identity.clone(),
            managed_by: MANAGER.into(),
            files: self
                .files
                .iter()
                .map(|file| (file.path.clone(), file.sha256.clone()))
                .collect(),
        }
    }
}

fn select_guide<'a>(bundle: &'a Bundle, resource: &str) -> Result<Guide<'a>> {
    // Resource arguments are identifiers, never filesystem paths to resolve.
    let source = SOURCE_FILES
        .iter()
        .find(|source| source.resource.topic == resource || source.resource.path == resource);
    let file = source.and_then(|source| {
        bundle
            .files
            .iter()
            .find(|file| file.path == source.resource.path)
    });
    let file = file.ok_or_else(|| {
        error(
            "guide_resource_not_found",
            format!(
                "unknown guide resource {resource:?}; run forge guide --json to list resources"
            ),
        )
    })?;
    Ok(Guide {
        identity: &bundle.identity,
        file,
        resources: SOURCE_FILES.iter().map(|source| source.resource).collect(),
    })
}

pub fn run_guide(args: GuideArgs) -> Result<()> {
    let bundle = Bundle::embedded();
    let guide = select_guide(&bundle, &args.resource)?;
    if args.json {
        crate::success(&guide)
    } else {
        print!("{}", guide.file.content);
        Ok(())
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum Status {
    Missing,
    Current,
    Outdated,
    Modified,
    Unmanaged,
}

impl Status {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Current => "current",
            Self::Outdated => "outdated",
            Self::Modified => "modified",
            Self::Unmanaged => "unmanaged",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Check {
    name: &'static str,
    scope: &'static str,
    target: PathBuf,
    status: Status,
    bundle: BundleIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    installed: Option<Manifest>,
    issues: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    action: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    backup_path: Option<PathBuf>,
}

struct Location {
    scope: &'static str,
    root: PathBuf,
}

impl Location {
    fn from_args(args: &ScopeArgs) -> Result<Self> {
        let (scope, root) = if let Some(path) = &args.project {
            ("project", path.clone())
        } else if args.user {
            (
                "user",
                env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
                    .ok_or_else(|| error("skill_invalid_scope", "HOME is not set"))?,
            )
        } else {
            return Err(error(
                "skill_invalid_scope",
                "choose --project PATH or --user",
            ));
        };
        let metadata = fs::symlink_metadata(&root).map_err(|cause| {
            error(
                "skill_invalid_scope",
                format!("{}: {cause}", root.display()),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(error(
                "skill_invalid_scope",
                format!(
                    "scope must be an existing directory, not a symlink: {}",
                    root.display()
                ),
            ));
        }
        // Canonicalizing the explicit scope allows system aliases such as /tmp.
        // Every path subsequently created inside that scope is checked separately.
        Ok(Self {
            scope,
            root: fs::canonicalize(root).map_err(crate::io_error)?,
        })
    }

    fn agents(&self) -> PathBuf {
        self.root.join(".agents")
    }
    fn target(&self) -> PathBuf {
        self.agents().join("skills").join(NAME)
    }

    fn prepare(&self, create: bool) -> Result<()> {
        checked_directory(&self.agents(), create)?;
        checked_directory(&self.agents().join("skills"), create)?;
        Ok(())
    }
}

pub fn run(command: SkillCommand) -> Result<()> {
    let bundle = Bundle::embedded();
    match command {
        SkillCommand::Show { json } => {
            if json {
                crate::success(&bundle)
            } else {
                print!("{}", SOURCE_FILES[0].content);
                Ok(())
            }
        }
        SkillCommand::Check(args) => {
            let location = Location::from_args(&args)?;
            location.prepare(false)?;
            output(&inspect(&location, &location.target(), &bundle)?, args.json)
        }
        SkillCommand::Install(args) => {
            let location = Location::from_args(&args)?;
            output(&install(&location, &bundle)?, args.json)
        }
    }
}

fn output(check: &Check, json: bool) -> Result<()> {
    if json {
        return crate::success(check);
    }
    println!(
        "{}: {} ({})",
        NAME,
        check.status.as_str(),
        check.target.display()
    );
    if let Some(action) = check.action {
        println!("Action: {action}");
    }
    if let Some(path) = &check.backup_path {
        println!("Previous skill preserved at {}", path.display());
    }
    for issue in &check.issues {
        println!("- {issue}");
    }
    Ok(())
}

fn checked_directory(path: &Path, create: bool) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(error(
            "skill_unsafe_path",
            format!(
                "expected a real directory, refusing symlink or other entry: {}",
                path.display()
            ),
        )),
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => {
            if create {
                fs::create_dir(path).map_err(crate::io_error)?;
            }
            Ok(())
        }
        Err(cause) => Err(crate::io_error(cause)),
    }
}

fn inspect(location: &Location, target: &Path, bundle: &Bundle) -> Result<Check> {
    let mut check = Check {
        name: NAME,
        scope: location.scope,
        target: target.to_owned(),
        status: Status::Missing,
        bundle: bundle.identity.clone(),
        installed: None,
        issues: Vec::new(),
        action: None,
        backup_path: None,
    };
    match fs::symlink_metadata(target) {
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(check),
        Err(cause) => return Err(crate::io_error(cause)),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => {
            check.status = Status::Unmanaged;
            check
                .issues
                .push("skill target is a symlink or is not a directory".into());
            return Ok(check);
        }
    }
    let manifest_path = target.join(MANIFEST);
    let manifest = match read_manifest(&manifest_path) {
        Ok(manifest) => manifest,
        Err(reason) => {
            check.status = Status::Unmanaged;
            check.issues.push(reason);
            return Ok(check);
        }
    };
    let expected_dirs: BTreeSet<_> = manifest
        .files
        .keys()
        .flat_map(|path| {
            Path::new(path)
                .ancestors()
                .skip(1)
                .filter(|p| !p.as_os_str().is_empty())
                .map(Path::to_path_buf)
                .collect::<Vec<_>>()
        })
        .collect();
    inspect_entries(
        target,
        Path::new(""),
        &manifest,
        &expected_dirs,
        &mut check.issues,
    )?;
    for path in manifest.files.keys() {
        if !target.join(path).try_exists().map_err(crate::io_error)? {
            check.issues.push(format!("missing managed file: {path}"));
        }
    }
    check.status = if !check.issues.is_empty() {
        Status::Modified
    } else if manifest.identity.content_hash == bundle.identity.content_hash {
        Status::Current
    } else {
        Status::Outdated
    };
    check.installed = Some(manifest);
    Ok(check)
}

fn read_manifest(path: &Path) -> std::result::Result<Manifest, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|cause| format!("no readable Forge manifest: {cause}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Forge manifest is not a regular file".into());
    }
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(MAX_MANIFEST_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|cause| format!("cannot read Forge manifest: {cause}"))?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err("Forge manifest is too large".into());
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|cause| format!("invalid Forge manifest: {cause}"))?;
    if manifest.managed_by != MANAGER
        || manifest.identity.name != NAME
        || manifest.identity.schema_version != SCHEMA_VERSION
    {
        return Err("manifest is not a supported Forge-managed forge-use installation".into());
    }
    if !manifest.files.contains_key("SKILL.md")
        || manifest.files.len() > 128
        || manifest.files.iter().any(|(path, hash)| {
            !safe_relative(path)
                || path == MANIFEST
                || hash.len() != 64
                || !hash
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        })
        || content_hash(&manifest.files) != manifest.identity.content_hash
    {
        return Err("manifest contains unsafe paths or inconsistent file hashes".into());
    }
    Ok(manifest)
}

fn inspect_entries(
    root: &Path,
    relative: &Path,
    manifest: &Manifest,
    directories: &BTreeSet<PathBuf>,
    issues: &mut Vec<String>,
) -> Result<()> {
    let mut entries = fs::read_dir(root.join(relative))
        .map_err(crate::io_error)?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(crate::io_error)?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = relative.join(entry.file_name());
        if path == Path::new(MANIFEST) {
            continue;
        }
        let key = path.to_string_lossy();
        let kind = entry.file_type().map_err(crate::io_error)?;
        if kind.is_symlink() {
            issues.push(format!("symlink in installed skill: {key}"));
        } else if kind.is_dir() {
            if directories.contains(&path) {
                inspect_entries(root, &path, manifest, directories, issues)?;
            } else {
                issues.push(format!("unmanaged directory: {key}"));
            }
        } else if kind.is_file() {
            match manifest.files.get(key.as_ref()) {
                Some(expected) if file_hash(&entry.path())? == *expected => {}
                Some(_) => issues.push(format!("modified managed file: {key}")),
                None => issues.push(format!("unmanaged file: {key}")),
            }
        } else {
            issues.push(format!("non-regular entry in installed skill: {key}"));
        }
    }
    Ok(())
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
        && Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// BTreeMap provides bytewise path order. Hash the compact UTF-8 JSON object of
// relative paths -> lowercase file SHA-256, allowing offline verification.
fn content_hash(files: &BTreeMap<String, String>) -> String {
    digest(&serde_json::to_vec(files).expect("string map serializes"))
}

fn file_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(crate::io_error)?;
    let mut hash = Sha256::new();
    let mut bytes = [0; 8192];
    loop {
        let read = file.read(&mut bytes).map_err(crate::io_error)?;
        if read == 0 {
            break;
        }
        hash.update(&bytes[..read]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn install(location: &Location, bundle: &Bundle) -> Result<Check> {
    location.prepare(true)?;
    let _lock = WriteLock::acquire(&location.agents())?;
    location.prepare(false)?;
    let before = inspect(location, &location.target(), bundle)?;
    match before.status {
        Status::Current => {
            return Ok(Check {
                action: Some("unchanged"),
                ..before
            })
        }
        Status::Modified | Status::Unmanaged => {
            return Err(error(
                if before.status == Status::Modified {
                    "skill_modified"
                } else {
                    "skill_unmanaged"
                },
                format!(
                    "preserving {}: {}",
                    before.target.display(),
                    before.issues.join("; ")
                ),
            ))
        }
        Status::Missing | Status::Outdated => {}
    }
    let staging_parent = location.agents().join(".forge-skill-staging");
    checked_directory(&staging_parent, true)?;
    let mut staging = Staging::new(&staging_parent)?;
    staging.write_bundle(bundle)?;
    location.prepare(false)?;
    checked_directory(&staging_parent, false)?;
    let current = inspect(location, &location.target(), bundle)?;
    if current.status != before.status
        || current.installed != before.installed
        || !current.issues.is_empty()
    {
        return Err(error(
            "skill_changed",
            "installation changed while staging; no existing skill was replaced",
        ));
    }
    let backup_path = if before.status == Status::Outdated {
        let backups = location.agents().join(".forge-skill-backups");
        checked_directory(&backups, true)?;
        let reserved = unique_directory(&backups)?;
        let backup = reserved.join(NAME);
        rename_new(&before.target, &backup).map_err(crate::io_error)?;
        Some(backup)
    } else {
        None
    };
    if let Err(cause) = rename_new(&staging.bundle_path(), &before.target) {
        if let Some(backup) = &backup_path {
            if fs::symlink_metadata(&before.target)
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
            {
                if let Err(restore) = rename_new(backup, &before.target) {
                    return Err(error("skill_update_failed", format!("update failed: {cause}; restore failed: {restore}; previous skill remains at {}", backup.display())));
                }
            } else {
                return Err(error("skill_update_failed", format!("update failed: {cause}; target appeared concurrently; previous skill preserved at {}", backup.display())));
            }
        }
        return Err(error(
            "skill_update_failed",
            format!("could not install staged skill: {cause}"),
        ));
    }
    staging.moved = true;
    Ok(Check {
        name: NAME,
        scope: location.scope,
        target: before.target,
        status: Status::Current,
        bundle: bundle.identity.clone(),
        installed: Some(bundle.manifest()),
        issues: vec![],
        action: Some(if backup_path.is_some() {
            "updated"
        } else {
            "installed"
        }),
        backup_path,
    })
}

struct WriteLock {
    path: PathBuf,
    token: Vec<u8>,
    file: File,
}

impl WriteLock {
    fn acquire(agents: &Path) -> Result<Self> {
        let path = agents.join(".forge-use.install.lock");
        let token = format!("{}\n", unique_id()).into_bytes();
        let file = OpenOptions::new().write(true).create_new(true).open(&path).map_err(|cause| {
            if cause.kind() == std::io::ErrorKind::AlreadyExists {
                error("skill_busy", format!("installation lock exists: {}; another install may be running; if interrupted, inspect this lock before removing it", path.display()))
            } else { crate::io_error(cause) }
        })?;
        let mut lock = Self { path, token, file };
        lock.file.write_all(&lock.token).map_err(crate::io_error)?;
        Ok(lock)
    }
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        let owned = {
            use std::os::unix::fs::MetadataExt;
            self.file
                .metadata()
                .ok()
                .zip(fs::symlink_metadata(&self.path).ok())
                .is_some_and(|(opened, current)| {
                    !current.file_type().is_symlink()
                        && opened.dev() == current.dev()
                        && opened.ino() == current.ino()
                })
        };
        #[cfg(not(unix))]
        let owned = fs::symlink_metadata(&self.path)
            .is_ok_and(|meta| meta.is_file() && !meta.file_type().is_symlink())
            && fs::read(&self.path)
                .ok()
                .is_some_and(|bytes| self.token.starts_with(&bytes));
        if owned {
            let _ = fs::remove_file(&self.path);
        }
    }
}

struct Staging {
    root: PathBuf,
    files: Vec<PathBuf>,
    directories: Vec<PathBuf>,
    moved: bool,
}

impl Staging {
    fn new(parent: &Path) -> Result<Self> {
        let root = unique_directory(parent)?;
        let mut staging = Self {
            root,
            files: vec![],
            directories: vec![],
            moved: false,
        };
        let bundle = staging.bundle_path();
        fs::create_dir(&bundle).map_err(crate::io_error)?;
        staging.directories.push(bundle);
        Ok(staging)
    }

    fn bundle_path(&self) -> PathBuf {
        self.root.join(NAME)
    }

    fn write_bundle(&mut self, bundle: &Bundle) -> Result<()> {
        for file in &bundle.files {
            if !safe_relative(&file.path) || file.path == MANIFEST {
                return Err(error(
                    "skill_invalid_bundle",
                    format!("unsafe bundled path: {}", file.path),
                ));
            }
            let mut parent = self.bundle_path();
            let relative = Path::new(&file.path);
            for part in relative.parent().unwrap().components() {
                parent.push(part);
                if !parent.exists() {
                    fs::create_dir(&parent).map_err(crate::io_error)?;
                    self.directories.push(parent.clone());
                }
                checked_directory(&parent, false)?;
            }
            self.write_file(relative, file.content.as_bytes())?;
        }
        let bytes = serde_json::to_vec_pretty(&bundle.manifest()).map_err(crate::json_error)?;
        self.write_file(Path::new(MANIFEST), &bytes)
    }

    fn write_file(&mut self, relative: &Path, bytes: &[u8]) -> Result<()> {
        let path = self.bundle_path().join(relative);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(crate::io_error)?;
        self.files.push(path);
        file.write_all(bytes).map_err(crate::io_error)?;
        file.sync_all().map_err(crate::io_error)
    }
}

impl Drop for Staging {
    fn drop(&mut self) {
        // Never recursively delete a directory: remove only entries created by
        // this invocation. Unexpected files keep the temporary directory intact.
        if !self.moved {
            for path in self.files.iter().rev() {
                let _ = fs::remove_file(path);
            }
            for path in self.directories.iter().rev() {
                let _ = fs::remove_dir(path);
            }
        }
        let _ = fs::remove_dir(&self.root);
    }
}

fn unique_id() -> String {
    format!(
        "{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn unique_directory(parent: &Path) -> Result<PathBuf> {
    for _ in 0..10 {
        let path = parent.join(unique_id());
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(cause) if cause.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(cause) => return Err(crate::io_error(cause)),
        }
    }
    Err(error(
        "skill_io",
        "could not reserve a unique staging directory",
    ))
}

fn error(code: &str, message: impl Into<String>) -> (String, String) {
    (code.into(), message.into())
}

/// Unlike std::fs::rename, never replace even an empty directory that appeared
/// between inspection and commit. Both paths are on the same scope filesystem.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn rename_new(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    let source = CString::new(source.as_os_str().as_bytes())?;
    let target = CString::new(target.as_os_str().as_bytes())?;
    // SAFETY: the C strings live throughout the call and flags prohibit replacement.
    let result = unsafe {
        #[cfg(target_os = "macos")]
        {
            libc::renameatx_np(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                target.as_ptr(),
                libc::RENAME_EXCL,
            )
        }
        #[cfg(target_os = "linux")]
        {
            libc::renameat2(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                target.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        }
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn rename_new(_source: &Path, _target: &Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "safe skill installation is not implemented on this platform",
    ))
}

#[cfg(test)]
mod guide_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn guide_defaults_to_overview_and_resolves_each_topic_and_exact_path() {
        let cli = crate::Cli::try_parse_from(["forge", "guide"]).unwrap();
        let crate::Command::Guide(args) = cli.command else {
            panic!("guide command was not parsed");
        };
        assert_eq!(args.resource, "overview");
        assert!(!args.json);

        let bundle = Bundle::embedded();
        let expected = [
            ("overview", "SKILL.md", "text/markdown"),
            ("static", "references/local-static.md", "text/markdown"),
            ("provider", "references/provider.md", "text/markdown"),
            ("animation", "references/animation.md", "text/markdown"),
            (
                "static-example",
                "examples/local-static.json",
                "application/json",
            ),
            ("delivery", "references/delivery.md", "text/markdown"),
            (
                "provider-example",
                "examples/provider-icons.json",
                "application/json",
            ),
        ];
        let guide = select_guide(&bundle, &args.resource).unwrap();
        assert_eq!(guide.resources.len(), expected.len());
        assert_eq!(guide.resources.len(), bundle.files.len());
        for (topic, path, media_type) in expected {
            assert!(guide.resources.contains(&GuideResource {
                topic,
                path,
                media_type,
            }));
            let by_topic = select_guide(&bundle, topic).unwrap();
            let by_path = select_guide(&bundle, path).unwrap();
            assert_eq!(by_topic.file.path, path);
            assert!(std::ptr::eq(by_topic.file, by_path.file));
            assert_eq!(
                by_topic.file.sha256,
                digest(by_topic.file.content.as_bytes())
            );
            if media_type == "application/json" {
                serde_json::from_str::<serde_json::Value>(&by_topic.file.content).unwrap();
            }
        }
    }

    #[test]
    fn guide_rejects_unknown_and_nonexact_paths_with_a_stable_error() {
        let bundle = Bundle::embedded();
        for resource in [
            "",
            "unknown",
            "Static",
            "../SKILL.md",
            "./SKILL.md",
            "/SKILL.md",
            "references/../SKILL.md",
            "references//local-static.md",
            "references\\local-static.md",
            ".agents/skills/forge-use/SKILL.md",
        ] {
            let cli = crate::Cli::try_parse_from(["forge", "guide", resource, "--json"])
                .expect("resource validation must reach the structured runtime error handler");
            let crate::Command::Guide(args) = cli.command else {
                panic!("guide command was not parsed");
            };
            assert!(args.json);
            let failure = select_guide(&bundle, &args.resource).unwrap_err();
            assert_eq!(failure.0, "guide_resource_not_found");
        }
    }

    #[test]
    fn guide_json_preserves_bundle_identity_and_selected_file_without_extra_contents() {
        let bundle = Bundle::embedded();
        let bundle_json = serde_json::to_value(&bundle).unwrap();
        let guide_json =
            serde_json::to_value(select_guide(&bundle, "static-example").unwrap()).unwrap();
        for field in [
            "name",
            "schemaVersion",
            "cliVersion",
            "build",
            "contentHash",
        ] {
            assert_eq!(guide_json[field], bundle_json[field], "{field}");
        }
        let file = bundle_json["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["path"] == "examples/local-static.json")
            .unwrap();
        for field in ["path", "sha256", "content"] {
            assert_eq!(guide_json[field], file[field], "{field}");
        }
        assert!(guide_json.get("files").is_none());
        for resource in guide_json["resources"].as_array().unwrap() {
            assert_eq!(resource.as_object().unwrap().len(), 3);
            assert!(resource["topic"].is_string());
            assert!(resource["path"].is_string());
            assert!(resource["mediaType"].is_string());
        }
    }
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    struct Fixture {
        location: Location,
    }

    impl Fixture {
        fn new() -> Self {
            let root = env::temp_dir().join(format!("forge-skill-test-{}", unique_id()));
            fs::create_dir(&root).unwrap();
            Self {
                location: Location {
                    scope: "project",
                    root: root.canonicalize().unwrap(),
                },
            }
        }

        fn target(&self) -> PathBuf {
            self.location.target()
        }
        fn manifest_bytes(&self) -> Vec<u8> {
            fs::read(self.target().join(MANIFEST)).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.location.root);
        }
    }

    fn old_bundle() -> Bundle {
        Bundle::new(&[("SKILL.md", "---\nname: forge-use\n---\nOld instructions\n")])
    }

    #[test]
    fn install_is_idempotent_and_preserves_a_complete_old_bundle_outside_discovery() {
        let fixture = Fixture::new();
        let old = old_bundle();
        let new = Bundle::embedded();
        fixture.location.prepare(false).unwrap();
        assert!(
            !fixture.location.agents().exists(),
            "check setup must not create directories"
        );
        assert_eq!(
            inspect(&fixture.location, &fixture.target(), &new)
                .unwrap()
                .status,
            Status::Missing
        );
        assert_eq!(
            install(&fixture.location, &old).unwrap().action,
            Some("installed")
        );
        let original_manifest = fixture.manifest_bytes();
        assert_eq!(
            install(&fixture.location, &old).unwrap().action,
            Some("unchanged")
        );
        assert_eq!(fixture.manifest_bytes(), original_manifest);
        assert_eq!(
            inspect(&fixture.location, &fixture.target(), &new)
                .unwrap()
                .status,
            Status::Outdated
        );
        let result = install(&fixture.location, &new).unwrap();
        assert_eq!(result.action, Some("updated"));
        let backup = result.backup_path.unwrap();
        assert!(backup.starts_with(fixture.location.agents().join(".forge-skill-backups")));
        assert!(!backup.starts_with(fixture.location.agents().join("skills")));
        assert_eq!(fs::read(backup.join(MANIFEST)).unwrap(), original_manifest);
        assert_eq!(
            fs::read_to_string(backup.join("SKILL.md")).unwrap(),
            old.files[0].content
        );
        assert_eq!(
            inspect(&fixture.location, &fixture.target(), &new)
                .unwrap()
                .status,
            Status::Current
        );
        assert_eq!(
            fs::read_dir(fixture.location.agents().join("skills"))
                .unwrap()
                .count(),
            1
        );
        let installed = fixture.manifest_bytes();
        assert_eq!(
            install(&fixture.location, &new).unwrap().action,
            Some("unchanged")
        );
        assert_eq!(fixture.manifest_bytes(), installed);
        assert_eq!(
            fs::read_dir(backup.parent().unwrap().parent().unwrap())
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn local_edits_extra_files_and_unmanaged_directories_are_preserved() {
        let fixture = Fixture::new();
        let old = old_bundle();
        let new = Bundle::embedded();
        install(&fixture.location, &old).unwrap();
        fs::write(fixture.target().join("SKILL.md"), "My local instructions").unwrap();
        assert_eq!(
            inspect(&fixture.location, &fixture.target(), &new)
                .unwrap()
                .status,
            Status::Modified
        );
        assert_eq!(
            install(&fixture.location, &new).unwrap_err().0,
            "skill_modified"
        );
        assert_eq!(
            fs::read_to_string(fixture.target().join("SKILL.md")).unwrap(),
            "My local instructions"
        );
        fs::write(fixture.target().join("SKILL.md"), &old.files[0].content).unwrap();
        fs::write(fixture.target().join("personal-notes.md"), "keep me").unwrap();
        assert_eq!(
            install(&fixture.location, &new).unwrap_err().0,
            "skill_modified"
        );
        assert_eq!(
            fs::read_to_string(fixture.target().join("personal-notes.md")).unwrap(),
            "keep me"
        );
        fs::remove_file(fixture.target().join(MANIFEST)).unwrap();
        assert_eq!(
            inspect(&fixture.location, &fixture.target(), &new)
                .unwrap()
                .status,
            Status::Unmanaged
        );
        assert_eq!(
            install(&fixture.location, &new).unwrap_err().0,
            "skill_unmanaged"
        );
        assert!(fixture.target().join("personal-notes.md").is_file());
    }

    #[test]
    fn unsafe_manifest_and_symlink_children_never_authorize_an_update() {
        let fixture = Fixture::new();
        let old = old_bundle();
        let new = Bundle::embedded();
        install(&fixture.location, &old).unwrap();
        let original = fixture.manifest_bytes();
        let mut forged = old.manifest();
        forged
            .files
            .insert("../../outside.txt".into(), digest(b"external"));
        forged.identity.content_hash = content_hash(&forged.files);
        fs::write(
            fixture.target().join(MANIFEST),
            serde_json::to_vec(&forged).unwrap(),
        )
        .unwrap();
        assert_eq!(
            install(&fixture.location, &new).unwrap_err().0,
            "skill_unmanaged"
        );
        fs::write(fixture.target().join(MANIFEST), original).unwrap();
        let external = fixture.location.root.join("outside.txt");
        fs::write(&external, &old.files[0].content).unwrap();
        fs::remove_file(fixture.target().join("SKILL.md")).unwrap();
        symlink(&external, fixture.target().join("SKILL.md")).unwrap();
        assert_eq!(
            inspect(&fixture.location, &fixture.target(), &new)
                .unwrap()
                .status,
            Status::Modified
        );
        assert_eq!(
            install(&fixture.location, &new).unwrap_err().0,
            "skill_modified"
        );
        assert_eq!(fs::read_to_string(external).unwrap(), old.files[0].content);
        assert!(fs::symlink_metadata(fixture.target().join("SKILL.md"))
            .unwrap()
            .file_type()
            .is_symlink());
    }

    #[test]
    fn symlink_parents_and_target_cannot_redirect_installation() {
        for relative in [
            ".agents",
            ".agents/skills",
            ".agents/skills/forge-use",
            ".agents/.forge-skill-staging",
        ] {
            let fixture = Fixture::new();
            let external = fixture.location.root.join("external");
            fs::create_dir(&external).unwrap();
            let link = fixture.location.root.join(relative);
            fs::create_dir_all(link.parent().unwrap()).unwrap();
            symlink(&external, &link).unwrap();
            let failure = install(&fixture.location, &Bundle::embedded()).unwrap_err();
            assert!(
                matches!(failure.0.as_str(), "skill_unsafe_path" | "skill_unmanaged"),
                "{failure:?}"
            );
            assert_eq!(fs::read_dir(&external).unwrap().count(), 0);
            assert!(fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink());
        }
        let fixture = Fixture::new();
        install(&fixture.location, &old_bundle()).unwrap();
        let external = fixture.location.root.join("external");
        fs::create_dir(&external).unwrap();
        symlink(
            &external,
            fixture.location.agents().join(".forge-skill-backups"),
        )
        .unwrap();
        let original = fixture.manifest_bytes();
        assert_eq!(
            install(&fixture.location, &Bundle::embedded())
                .unwrap_err()
                .0,
            "skill_unsafe_path"
        );
        assert_eq!(fixture.manifest_bytes(), original);
        assert_eq!(fs::read_dir(external).unwrap().count(), 0);
    }

    #[test]
    fn a_failed_staging_write_leaves_the_previous_installation_intact() {
        let fixture = Fixture::new();
        install(&fixture.location, &old_bundle()).unwrap();
        let original = fixture.manifest_bytes();
        // A file/directory collision fails after one staged file has been written.
        let invalid = Bundle::new(&[
            ("SKILL.md", "new"),
            ("references", "file"),
            ("references/guide.md", "guide"),
        ]);
        assert!(install(&fixture.location, &invalid).is_err());
        assert_eq!(fixture.manifest_bytes(), original);
        assert_eq!(
            fs::read_to_string(fixture.target().join("SKILL.md")).unwrap(),
            old_bundle().files[0].content
        );
        assert!(!fixture
            .location
            .agents()
            .join(".forge-skill-backups")
            .exists());
        assert_eq!(
            fs::read_dir(fixture.location.agents().join(".forge-skill-staging"))
                .unwrap()
                .count(),
            0
        );
        assert!(!fixture
            .location
            .agents()
            .join(".forge-use.install.lock")
            .exists());
    }

    #[test]
    fn an_existing_writer_lock_blocks_install_without_removing_its_lock() {
        let fixture = Fixture::new();
        fixture.location.prepare(true).unwrap();
        let lock = WriteLock::acquire(&fixture.location.agents()).unwrap();
        assert_eq!(
            install(&fixture.location, &Bundle::embedded())
                .unwrap_err()
                .0,
            "skill_busy"
        );
        assert_eq!(fs::read(&lock.path).unwrap(), lock.token);
        assert!(!fixture.target().exists());
        drop(lock);
        assert_eq!(
            install(&fixture.location, &Bundle::embedded())
                .unwrap()
                .status,
            Status::Current
        );
    }

    #[test]
    fn commit_rename_never_replaces_a_concurrently_created_empty_directory() {
        let fixture = Fixture::new();
        let source = fixture.location.root.join("source");
        let destination = fixture.location.root.join("destination");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("SKILL.md"), "keep staged").unwrap();
        fs::create_dir(&destination).unwrap();
        assert!(rename_new(&source, &destination).is_err());
        assert_eq!(fs::read_dir(destination).unwrap().count(), 0);
        assert_eq!(
            fs::read_to_string(source.join("SKILL.md")).unwrap(),
            "keep staged"
        );
    }
}
