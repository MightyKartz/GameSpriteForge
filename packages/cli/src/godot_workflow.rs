//! Optional Godot setup and bounded, isolated project acceptance.
use clap::{Args, Subcommand};
use forge_core::godot_environment::{self as environment, Engine, ToolchainLock};
use serde_json::{json, Value};
use sha2::{Digest, Sha512};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, String>;

#[derive(Subcommand)]
pub enum SetupCommand {
    Godot(SetupArgs),
}

#[derive(Args)]
pub struct SetupArgs {
    /// Explicit executable or macOS .app. Save to Forge's machine-local configuration.
    #[arg(long, conflicts_with = "download")]
    path: Option<PathBuf>,
    /// Download a checksum-pinned official standard editor (default: 4.7.2).
    #[arg(long)]
    download: bool,
    /// Select a pinned engine version; only used with --download.
    #[arg(long, requires = "download", value_parser = ["4.6.3", "4.7.2"])]
    version: Option<String>,
    /// Also download the matching official export templates (large download).
    #[arg(long)]
    templates: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
pub struct LockArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    godot: Option<PathBuf>,
    /// Explicitly replace an existing project requirement after verification.
    #[arg(long)]
    update: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
pub struct CheckArgs {
    #[arg(long)]
    project: PathBuf,
    #[arg(long)]
    godot: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
pub struct VerifyArgs {
    #[arg(long)]
    project: PathBuf,
    /// New evidence directory outside the source project; contains an isolated copy.
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    godot: Option<PathBuf>,
    /// Capture the rendered main viewport after the specified frames (requires GPU).
    #[arg(long)]
    screenshot: bool,
    #[arg(long, default_value_t = 30, value_parser = clap::value_parser!(u32).range(1..=3600))]
    frames: u32,
    #[arg(long, default_value_t = 120, value_parser = clap::value_parser!(u64).range(1..=3600))]
    timeout: u64,
    /// Optional SceneTree test script, relative to project, which must print FORGE_ACCEPTANCE_OK.
    #[arg(long)]
    acceptance_script: Option<PathBuf>,
    /// Creating this file requests cancellation; checked throughout child execution.
    #[arg(long)]
    cancel_file: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
pub struct ExportArgs {
    #[arg(long)]
    project: PathBuf,
    /// Existing desktop export preset; export is performed in an isolated project copy.
    #[arg(long)]
    preset: String,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    godot: Option<PathBuf>,
    #[arg(long, default_value_t = 300, value_parser = clap::value_parser!(u64).range(1..=3600))]
    timeout: u64,
    /// Launch the exported native program for a bounded smoke test.
    #[arg(long)]
    run: bool,
    #[arg(long)]
    cancel_file: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

pub fn error(message: String) -> (String, String) {
    let code = if message.contains("toolchain_mismatch") {
        "toolchain_mismatch"
    } else {
        "godot_workflow_failed"
    };
    (code.into(), message)
}

fn project_root(path: &Path) -> Result<PathBuf> {
    let root = path.canonicalize().map_err(|e| e.to_string())?;
    if !root.join("project.godot").is_file() {
        return Err("Expected a Godot project containing project.godot".into());
    }
    Ok(root)
}

pub fn lock(args: LockArgs) -> Result<Value> {
    let project = project_root(&args.project)?;
    let engine = environment::resolve(args.godot.as_deref(), None)?;
    let lock = ToolchainLock {
        schema_version: 1,
        forge_version: env!("CARGO_PKG_VERSION").into(),
        godot_version: engine.version,
    };
    let path = project.join(environment::LOCK_FILE);
    if fs::symlink_metadata(project.join(".forge")).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Project .forge may not be a symlink".into());
    }
    environment::write_json(&path, &lock, args.update)?;
    Ok(json!({"lockPath":path,"lock":lock,"machinePathsIncluded":false}))
}

pub fn check(args: CheckArgs) -> Result<Value> {
    let project = project_root(&args.project)?;
    let engine = environment::resolve(args.godot.as_deref(), Some(&project))?;
    Ok(
        json!({"engine":engine,"lock":environment::read_lock(&project)?,"templates":template_status(&engine),"exportPresetsPresent":project.join("export_presets.cfg").is_file(),"runtime":"not_run","visualReview":"not_assessed"}),
    )
}

fn templates_root(version: &str) -> Result<PathBuf> {
    dirs_path().map(|p| {
        p.join(if cfg!(any(windows, target_os = "macos")) {
            "Godot/export_templates"
        } else {
            "godot/export_templates"
        })
        .join(version)
    })
}

fn template_version(engine: &Engine) -> String {
    let parts: Vec<_> = engine.version.split('.').collect();
    let count = if parts.get(2).is_some_and(|p| p.parse::<u32>().is_ok()) {
        4
    } else {
        3
    };
    let mut version = parts
        .iter()
        .take(count)
        .copied()
        .collect::<Vec<_>>()
        .join(".");
    if parts.contains(&"mono") {
        version.push_str(".mono");
    }
    version
}

fn dirs_path() -> Result<PathBuf> {
    // Godot's documented per-user data directory; keep config selection separate.
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or("APPDATA unavailable".into())
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME")
            .map(|p| PathBuf::from(p).join("Library/Application Support"))
            .ok_or("HOME unavailable".into())
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
            .ok_or("User data directory unavailable".into())
    }
}

fn template_status(engine: &Engine) -> Value {
    let version = template_version(engine);
    match templates_root(&version) {
        Ok(root) => {
            json!({"version":version,"directory":root,"windowsRelease":root.join("windows_release_x86_64.exe").is_file(),"macos":root.join("macos.zip").is_file(),"note":"Per-user templates for the selected engine; export resolves self-contained and custom templates separately"})
        }
        Err(error) => json!({"error":error}),
    }
}

pub fn setup(args: SetupArgs) -> Result<Value> {
    let engine = if args.download {
        install_engine(managed_release(
            args.version.as_deref().unwrap_or(DEFAULT_GODOT_VERSION),
        )?)?
    } else {
        environment::resolve(args.path.as_deref(), None)?
    };
    if args.templates {
        let release = MANAGED_RELEASES.iter().find(|r| r.matches(&engine)).ok_or(
            "Managed templates require a standard Godot 4.6.3 or 4.7.2 build; install matching templates separately for other engines"
        )?;
        install_templates(release)?;
    }
    let config = setup_file_operation("save Godot configuration", || environment::save(&engine))?;
    Ok(
        json!({"engine":engine,"configPath":config,"templates":template_status(&engine),"environmentModified":false,"next":"forge godot check --project PATH --json"}),
    )
}

const DEFAULT_GODOT_VERSION: &str = "4.7.2";

/// Windows scanners can hold a newly extracted executable or template file for
/// a moment. Retry only that OS error; retain the stage in all failures.
fn setup_file_operation<T>(stage: &str, mut operation: impl FnMut() -> Result<T>) -> Result<T> {
    let attempts = if cfg!(windows) { 6 } else { 1 };
    for attempt in 0..attempts {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if cfg!(windows) && error.contains("os error 5") && attempt + 1 < attempts =>
            {
                thread::sleep(Duration::from_millis(100u64 << attempt));
            }
            Err(error) => return Err(format!("{stage}: {error}")),
        }
    }
    unreachable!("the last attempt returns its result")
}

struct ManagedRelease {
    version: &'static str,
    windows_sha512: &'static str,
    macos_sha512: &'static str,
    templates_sha512: &'static str,
}

impl ManagedRelease {
    fn matches(&self, engine: &Engine) -> bool {
        engine
            .version
            .starts_with(&format!("{}.stable.", self.version))
            && !engine.version.split('.').any(|part| part == "mono")
    }
}

// SHA512-SUMS.txt from the corresponding official godot-builds release.
const MANAGED_RELEASES: &[ManagedRelease] = &[
    ManagedRelease {
        version: "4.6.3",
        windows_sha512: "d44ea7ef5bab754cacd49d581b6062836b2eea12a82e1d183aebfad9cd8c7db2bd82513337bd657d6d2d5c04d46239c0570b029faf1343e81e8a2fa7b85dd83b",
        macos_sha512: "0155bbc8dcf179edab4ef08e3dbbd4480d7434ab1990cb3fb25517d453b1b65787c7fe6dfc8824b15ff25e1fdcec59e8584f765054106894fee00311868c3964",
        templates_sha512: "da606b61c10157844f8300172df374472665f95015495cb1a7cd132c40ede404faa96cc1016a4b9662db9909ddea69632c4948b2cd11163438dad4808881fb68",
    },
    ManagedRelease {
        version: "4.7.2",
        windows_sha512: "83decd58fdf67b9d657958a1ae6bf1929c20785315a81effe245874cdc57acb709bf868e00778a96984338c1b29dafdb453c6847747694621c6ecf5da2259993",
        macos_sha512: "38aa16e5bba2083941fc5b3e54be0089bd4cc35e32415f5b9fd9a8a6a7b9818255d44532ea8ef94b5aef56c4b407c2d634fa4f657e4ebe681ebbf59b7bac69ca",
        templates_sha512: "ca4d71c4d7b81dfc15d1a98baa07534aa95b03fdda78a0075b06672e1648d2e5f40980c9adc28d23e1b92e732ee7bf3461997aa804af74ec2fcd7a93ccb84079",
    },
];

fn managed_release(version: &str) -> Result<&'static ManagedRelease> {
    MANAGED_RELEASES
        .iter()
        .find(|r| r.version == version)
        .ok_or_else(|| format!("No pinned Godot download for {version}; choose 4.6.3 or 4.7.2"))
}

fn download_extract(
    release: &ManagedRelease,
    name: &str,
    checksum: &str,
    parent: &Path,
) -> Result<tempfile::TempDir> {
    setup_file_operation("create Godot download directory", || {
        fs::create_dir_all(parent).map_err(|e| e.to_string())
    })?;
    let temp = setup_file_operation("create Godot download workspace", || {
        tempfile::tempdir_in(parent).map_err(|e| e.to_string())
    })?;
    let archive = temp.path().join("download.zip");
    let url = format!(
        "https://github.com/godotengine/godot-builds/releases/download/{}-stable/{name}",
        release.version
    );
    let mut command = Command::new(if cfg!(windows) { "curl.exe" } else { "curl" });
    command
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "--fail",
            "--location",
            "--retry",
            "3",
            "--connect-timeout",
            "30",
            "--max-time",
            "1800",
            "--output",
        ])
        .arg(&archive)
        .arg(url);
    if let Err(error) = run_process(&mut command, temp.path(), "download", 1900, None, false) {
        let evidence = temp.keep();
        return Err(format!(
            "{error}; retained download diagnostics: {}",
            evidence.display()
        ));
    }
    let mut file = setup_file_operation("open downloaded Godot archive", || {
        fs::File::open(&archive).map_err(|e| e.to_string())
    })?;
    let mut hash = Sha512::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = setup_file_operation("hash downloaded Godot archive", || {
            file.read(&mut buffer).map_err(|e| e.to_string())
        })?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    drop(file);
    if format!("{:x}", hash.finalize()) != checksum {
        return Err("Official Godot archive checksum mismatch; nothing installed".into());
    }
    if let Err(error) = extract(&archive, &temp.path().join("unpacked"), temp.path()) {
        let evidence = temp.keep();
        return Err(format!(
            "{error}; retained setup diagnostics: {}",
            evidence.display()
        ));
    }
    Ok(temp)
}

fn extract(archive: &Path, output: &Path, logs: &Path) -> Result<()> {
    let mut command;
    if cfg!(windows) {
        command = Command::new("powershell.exe");
        // Values are environment data, never interpolated into PowerShell code.
        command.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", "[Console]::OutputEncoding = [Text.UTF8Encoding]::new(); $ErrorActionPreference='Stop'; Expand-Archive -LiteralPath $env:FORGE_EXTRACT_INPUT -DestinationPath $env:FORGE_EXTRACT_OUTPUT"]);
        command
            .env("FORGE_EXTRACT_INPUT", archive)
            .env("FORGE_EXTRACT_OUTPUT", output);
    } else if cfg!(target_os = "macos") {
        command = Command::new("ditto");
        command.args(["-x", "-k"]).arg(archive).arg(output);
    } else {
        return Err("Managed downloads currently support Windows x64 and macOS Apple Silicon; use --path on other platforms".into());
    }
    run_process(&mut command, logs, "extract", 600, None, false)?;
    Ok(())
}

fn install_engine(release: &ManagedRelease) -> Result<Engine> {
    let version = release.version;
    let (archive, checksum, relative) = if cfg!(all(windows, target_arch = "x86_64")) {
        (
            format!("Godot_v{version}-stable_win64.exe.zip"),
            release.windows_sha512,
            format!("Godot_v{version}-stable_win64_console.exe"),
        )
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        (
            format!("Godot_v{version}-stable_macos.universal.zip"),
            release.macos_sha512,
            "Godot.app/Contents/MacOS/Godot".into(),
        )
    } else {
        return Err(
            "Managed Godot download supports Windows x64 and macOS Apple Silicon; use --path"
                .into(),
        );
    };
    let root = environment::config_root()?.join("tools");
    let target = root.join(format!("godot-{version}-standard"));
    if target.exists() {
        let engine = setup_file_operation("probe installed Godot", || {
            environment::probe(&target.join(&relative))
        })?;
        let saved: Engine = serde_json::from_slice(&setup_file_operation(
            "read installed Godot manifest",
            || fs::read(target.join("forge-engine.json")).map_err(|e| e.to_string()),
        )?)
        .map_err(|e| e.to_string())?;
        if saved.sha256 != engine.sha256
            || saved.companion_sha256 != engine.companion_sha256
            || saved.version != engine.version
            || !release.matches(&engine)
        {
            return Err(
                "Managed engine was modified; select another installation explicitly".into(),
            );
        }
        return Ok(engine);
    }
    let temp = download_extract(release, &archive, checksum, &root)?;
    let unpacked = setup_file_operation("probe extracted Godot", || {
        environment::probe(&temp.path().join("unpacked").join(&relative))
    })?;
    if !release.matches(&unpacked) {
        return Err("Downloaded engine does not match the selected release".into());
    }
    setup_file_operation("publish extracted Godot", || {
        fs::rename(temp.path().join("unpacked"), &target).map_err(|e| e.to_string())
    })?;
    let engine = setup_file_operation("probe published Godot", || {
        environment::probe(&target.join(&relative))
    })?;
    setup_file_operation("write Godot manifest", || {
        environment::write_json(&target.join("forge-engine.json"), &engine, false)
    })?;
    Ok(engine)
}

fn install_templates(release: &ManagedRelease) -> Result<()> {
    let version = format!("{}.stable", release.version);
    let target = templates_root(&version)?;
    if target.exists() {
        if [
            "windows_release_x86_64.exe",
            "windows_debug_x86_64.exe",
            "macos.zip",
        ]
        .iter()
        .all(|name| target.join(name).is_file())
            && target.join("version.txt").is_file()
            && setup_file_operation("read installed template version", || {
                fs::read_to_string(target.join("version.txt")).map_err(|e| e.to_string())
            })?
            .trim()
                == version
        {
            return Ok(());
        }
        return Err(
            "Existing template directory is incomplete or unrecognized; it was preserved".into(),
        );
    }
    let archive = format!("Godot_v{}-stable_export_templates.tpz", release.version);
    let temp = download_extract(
        release,
        &archive,
        release.templates_sha512,
        target.parent().ok_or("Invalid templates path")?,
    )?;
    let source = temp.path().join("unpacked/templates");
    if setup_file_operation("read extracted template version", || {
        fs::read_to_string(source.join("version.txt")).map_err(|e| e.to_string())
    })?
    .trim()
        != version
    {
        return Err("Unexpected export template version".into());
    }
    setup_file_operation("publish Godot templates", || {
        fs::rename(&source, &target).map_err(|e| e.to_string())
    })?;
    Ok(())
}

/// File-backed stdout/stderr avoid pipe deadlocks. Timeout/cancellation kill the child.
fn run_process(
    command: &mut Command,
    logs: &Path,
    name: &str,
    timeout: u64,
    cancel: Option<&Path>,
    engine_errors: bool,
) -> Result<Value> {
    let stdout = logs.join(format!("{name}.stdout.log"));
    let stderr = logs.join(format!("{name}.stderr.log"));
    if cancel.is_some_and(Path::exists) {
        return Err("cancelled: cancellation file exists".into());
    }
    command
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout).map_err(|e| e.to_string())?)
        .stderr(fs::File::create(&stderr).map_err(|e| e.to_string())?);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot start {name}: {e}"))?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        let cancelled = cancel.is_some_and(Path::exists);
        let oversized = [&stdout, &stderr]
            .iter()
            .any(|p| fs::metadata(p).is_ok_and(|m| m.len() > 16 * 1024 * 1024));
        if cancelled || oversized || start.elapsed().as_secs() >= timeout {
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                let _ = Command::new("taskkill.exe")
                    .args(["/PID", &child.id().to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{}: {name}; logs: {}",
                if cancelled {
                    "cancelled"
                } else if oversized {
                    "log_limit_exceeded"
                } else {
                    "timeout"
                },
                logs.display()
            ));
        }
        thread::sleep(Duration::from_millis(30));
    };
    if [&stdout, &stderr]
        .iter()
        .any(|p| fs::metadata(p).is_ok_and(|m| m.len() > 16 * 1024 * 1024))
    {
        return Err(format!(
            "log_limit_exceeded: {name}; inspect {}",
            logs.display()
        ));
    }
    let text = format!(
        "{}\n{}",
        fs::read_to_string(&stdout).unwrap_or_default(),
        fs::read_to_string(&stderr).unwrap_or_default()
    );
    if !status.success()
        || (engine_errors
            && text.lines().any(|l| {
                let l = l.trim_start();
                l.starts_with("ERROR:")
                    || l.starts_with("SCRIPT ERROR:")
                    || l.contains("Parse Error:")
            }))
    {
        return Err(format!(
            "{name} failed (exit {:?}); inspect {} and {}",
            status.code(),
            stdout.display(),
            stderr.display()
        ));
    }
    Ok(
        json!({"status":"passed","exitCode":status.code(),"stdout":stdout,"stderr":stderr,"elapsedMs":start.elapsed().as_millis()}),
    )
}

fn inventory(root: &Path) -> Result<BTreeMap<PathBuf, String>> {
    fn walk(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, String>) -> Result<()> {
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if [".git", ".godot"].contains(&entry.file_name().to_string_lossy().as_ref()) {
                continue;
            }
            let meta = fs::symlink_metadata(&path).map_err(|e| e.to_string())?;
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err(format!(
                        "Project snapshot refuses reparse point: {}",
                        path.display()
                    ));
                }
            }
            if meta.file_type().is_symlink() {
                return Err(format!(
                    "Project snapshot refuses symlink: {}",
                    path.display()
                ));
            }
            if meta.is_dir() {
                walk(root, &path, files)?;
            } else if meta.is_file() {
                files.insert(
                    path.strip_prefix(root)
                        .map_err(|e| e.to_string())?
                        .to_path_buf(),
                    environment::digest(&path)?,
                );
            } else {
                return Err(format!("Unsupported project entry: {}", path.display()));
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    walk(root, root, &mut files)?;
    Ok(files)
}

fn snapshot(
    project: &Path,
    output: &Path,
) -> Result<(PathBuf, PathBuf, BTreeMap<PathBuf, String>)> {
    let parent = output
        .parent()
        .ok_or("Output must have a parent")?
        .canonicalize()
        .map_err(|e| format!("Create the output parent first: {e}"))?;
    let output = parent.join(output.file_name().ok_or("Output needs a directory name")?);
    if output.starts_with(project) {
        return Err("Evidence output must be outside the source project".into());
    }
    fs::create_dir(&output).map_err(|e| format!("Evidence directory must be new: {e}"))?;
    let files = inventory(project)?;
    let copied = output.join("project");
    fs::create_dir(&copied).map_err(|e| e.to_string())?;
    for relative in files.keys() {
        let dest = copied.join(relative);
        fs::create_dir_all(dest.parent().ok_or("Invalid snapshot path")?)
            .map_err(|e| e.to_string())?;
        fs::copy(project.join(relative), dest).map_err(|e| e.to_string())?;
    }
    if files != inventory(&copied)? || files != inventory(project)? {
        return Err("Source changed while copying; rerun with a new evidence directory".into());
    }
    environment::write_json(&output.join("source-inventory.json"), &files, false)?;
    Ok((output, copied, files))
}

// Scope these variables to child processes. Project settings and exported PCKs
// remain unchanged, so acceptance paths cannot leak into delivered games.
struct RunProfile {
    root: PathBuf,
}

impl RunProfile {
    fn new(output: &Path) -> Result<Self> {
        let root = PathBuf::from(godot_path(&output.join("user-profile"))?);
        for directory in ["home", "data", "config", "cache", "temp"] {
            fs::create_dir_all(root.join(directory)).map_err(|e| e.to_string())?;
        }
        Ok(Self { root })
    }

    fn apply<'a>(&self, command: &'a mut Command) -> &'a mut Command {
        command
            .env("HOME", self.root.join("home"))
            .env("APPDATA", self.root.join("data"))
            .env("LOCALAPPDATA", self.root.join("cache"))
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_CACHE_HOME", self.root.join("cache"))
            .env("TMPDIR", self.root.join("temp"))
            .env("TMP", self.root.join("temp"))
            .env("TEMP", self.root.join("temp"))
    }
}

fn engine_command(engine: &Engine, project: &Path, profile: &RunProfile) -> Command {
    let mut command = Command::new(&engine.path);
    command.arg("--path").arg(project);
    profile.apply(&mut command);
    command
}

fn finish(output: &Path, report: &Value, result: Result<()>) -> Result<Value> {
    let mut report = report.clone();
    match result {
        Ok(()) => {
            report["status"] = json!("passed");
            environment::write_json(&output.join("report.json"), &report, false)?;
            Ok(report)
        }
        Err(error) => {
            for key in [
                "import",
                "runtime",
                "interactionTests",
                "export",
                "exportedRuntime",
            ] {
                if report
                    .get(key)
                    .and_then(|phase| phase.get("status"))
                    .and_then(Value::as_str)
                    == Some("running")
                {
                    report[key] = json!({"status":"failed","error":error});
                }
            }
            report["status"] = json!("failed");
            report["error"] = json!(&error);
            environment::write_json(&output.join("report.json"), &report, false)?;
            Err(format!(
                "{error}; report: {}",
                output.join("report.json").display()
            ))
        }
    }
}

pub fn verify(args: VerifyArgs) -> Result<Value> {
    let project = project_root(&args.project)?;
    let engine = environment::resolve(args.godot.as_deref(), Some(&project))?;
    let (output, copied, before) = snapshot(&project, &args.output)?;
    let mut report = json!({"schemaVersion":"1","operation":"verify","sourceProject":project,"snapshot":copied,"engine":engine,"forgeVersion":env!("CARGO_PKG_VERSION"),"forgeBuild":crate::build_info::current(),"visualReview":"not_assessed","interactionTests":{"status":"not_run"},"import":{"status":"not_run"},"runtime":{"status":"not_run"},"screenshot":{"status":"not_requested"}});
    let result = (|| {
        let profile = RunProfile::new(&output)?;
        report["userData"] = json!({"isolation":"per_run_profile","root":profile.root});
        report["import"] = json!({"status":"running"});
        report["import"] = run_process(
            engine_command(&engine, &copied, &profile).args(["--headless", "--import"]),
            &output,
            "import",
            args.timeout,
            args.cancel_file.as_deref(),
            true,
        )?;
        let script = output.join("acceptance.gd");
        fs::write(
            &script,
            include_str!("../../../scripts/godot/forge_project_acceptance.gd"),
        )
        .map_err(|e| e.to_string())?;
        let mut command = engine_command(&engine, &copied, &profile);
        if !args.screenshot {
            command.arg("--headless");
        }
        command
            .arg("--script")
            .arg(&script)
            .args(["--", "--forge-frames"])
            .arg(args.frames.to_string());
        let screenshot = output.join("screenshot.png");
        if args.screenshot {
            command.arg("--forge-screenshot").arg(&screenshot);
        }
        report["runtime"] = json!({"status":"running"});
        report["runtime"] = run_process(
            &mut command,
            &output,
            "runtime",
            args.timeout,
            args.cancel_file.as_deref(),
            true,
        )?;
        if !fs::read_to_string(output.join("runtime.stdout.log"))
            .map_err(|e| e.to_string())?
            .lines()
            .any(|l| l == "FORGE_PROJECT_READY")
        {
            report["runtime"]["status"] = json!("failed");
            return Err("Runtime exited without completion marker".into());
        }
        if args.screenshot {
            let image = forge_core::source_inspect::inspect_png(&screenshot, None, None);
            image.map_err(|e| format!("Screenshot missing or invalid: {e}"))?;
            report["screenshot"] = json!({"status":"captured","path":screenshot,"sha256":environment::digest(&screenshot)?});
        }
        if let Some(relative) = &args.acceptance_script {
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| !matches!(c, std::path::Component::Normal(_)))
            {
                return Err(
                    "Acceptance script must be a project-relative path without traversal".into(),
                );
            }
            let script = copied.join(relative);
            report["interactionTests"] = json!({"status":"running"});
            report["interactionTests"] = run_process(
                engine_command(&engine, &copied, &profile)
                    .args(["--headless", "--script"])
                    .arg(&script),
                &output,
                "interaction",
                args.timeout,
                args.cancel_file.as_deref(),
                true,
            )?;
            if !fs::read_to_string(output.join("interaction.stdout.log"))
                .map_err(|e| e.to_string())?
                .lines()
                .any(|l| l == "FORGE_ACCEPTANCE_OK")
            {
                report["interactionTests"]["status"] = json!("failed");
                return Err("Interaction test did not print FORGE_ACCEPTANCE_OK".into());
            }
        }
        if before != inventory(&project)? {
            return Err("Source changed during verification; evidence is stale".into());
        }
        Ok(())
    })();
    finish(&output, &report, result)
}

struct NativePreset {
    options_section: String,
    custom_template: Option<PathBuf>,
    architecture: String,
}

fn native_preset(project: &Path, name: &str) -> Result<NativePreset> {
    let text = fs::read_to_string(project.join("export_presets.cfg")).map_err(|e| e.to_string())?;
    let mut sections: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut section = String::new();
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].into();
        } else if let Some((key, value)) = line.split_once('=') {
            if let Ok(value) = serde_json::from_str::<String>(value.trim()) {
                sections
                    .entry(section.clone())
                    .or_default()
                    .insert(key.trim().into(), value);
            }
        }
    }
    let selected: Vec<_> = sections
        .iter()
        .filter(|(section, values)| {
            !section.ends_with(".options") && values.get("name").is_some_and(|value| value == name)
        })
        .collect();
    if selected.len() != 1 {
        return Err("Export preset not found or ambiguous; choose an existing uniquely named desktop preset".into());
    }
    let (section, values) = selected[0];
    let expected = if cfg!(windows) {
        "Windows Desktop"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    if values.get("platform").is_none_or(|value| value != expected) {
        return Err(format!("This acceptance command exports the host platform ({expected}); use Godot directly for cross-target export"));
    }
    let options_section = format!("{section}.options");
    let options = sections.get(&options_section);
    let architecture = options
        .and_then(|v| v.get("binary_format/architecture"))
        .cloned()
        .unwrap_or_else(|| "x86_64".into());
    let custom = options
        .and_then(|v| v.get("custom_template/release"))
        .filter(|v| !v.is_empty());
    let custom_template = custom
        .map(|path| {
            let path = if let Some(relative) = path.strip_prefix("res://") {
                project.join(relative)
            } else {
                let p = PathBuf::from(path);
                if p.is_absolute() {
                    p
                } else {
                    project.join(p)
                }
            };
            path.canonicalize()
                .map_err(|e| format!("Custom export template is unavailable: {e}"))
        })
        .transpose()?;
    Ok(NativePreset {
        options_section,
        custom_template,
        architecture,
    })
}

// Resolve before changing the child's profile, where standard templates are no
// longer visible. Match Godot's versioned desktop template names and self-contained
// editor layout; keep companion libraries beside the original template.
fn standard_release_template(engine: &Engine, architecture: &str) -> Result<PathBuf> {
    if !["x86_64", "x86_32", "arm64", "arm32", "universal"].contains(&architecture) {
        return Err("Unsupported desktop export architecture".into());
    }
    let version = template_version(engine);
    let mut editor_dir = engine
        .path
        .parent()
        .ok_or("Invalid engine path")?
        .to_path_buf();
    if cfg!(target_os = "macos") && editor_dir.ends_with("Contents/MacOS") {
        editor_dir = editor_dir.join("../../..");
    }
    let root = if ["._sc_", "_sc_"]
        .iter()
        .any(|name| editor_dir.join(name).is_file())
    {
        editor_dir.join("editor_data/export_templates")
    } else {
        dirs_path()?.join(if cfg!(any(windows, target_os = "macos")) {
            "Godot/export_templates"
        } else {
            "godot/export_templates"
        })
    };
    let filename = if cfg!(windows) {
        format!("windows_release_{architecture}.exe")
    } else if cfg!(target_os = "macos") {
        "macos.zip".into()
    } else {
        format!("linux_release.{architecture}")
    };
    let path = root.join(version).join(filename);
    path.canonicalize().map_err(|e| format!("Release export template unavailable at {}: {e}; install matching templates or set custom_template/release", path.display()))
}

fn godot_path(path: &Path) -> Result<String> {
    // Godot's virtual filesystem does not consistently accept Windows verbatim
    // prefixes (notably in APPDATA), although Rust canonicalize returns them.
    let path = path.to_str().ok_or("Godot path is not UTF-8")?;
    Ok(path
        .strip_prefix(r"\\?\UNC\")
        .map(|p| format!("//{p}"))
        .unwrap_or_else(|| path.strip_prefix(r"\\?\").unwrap_or(path).to_owned())
        .replace('\\', "/"))
}

fn bind_release_template(project: &Path, section: &str, template: &Path) -> Result<()> {
    let path = project.join("export_presets.cfg");
    let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let template = godot_path(template)?;
    let binding = format!(
        "custom_template/release={}\n",
        serde_json::to_string(&template).map_err(|e| e.to_string())?
    );
    let mut output = String::new();
    let mut selected = false;
    let mut found = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            selected = &trimmed[1..trimmed.len() - 1] == section;
            output.push_str(line);
            output.push('\n');
            if selected {
                output.push_str(&binding);
                found = true;
            }
        } else if !(selected
            && trimmed
                .split_once('=')
                .is_some_and(|(key, _)| key.trim() == "custom_template/release"))
        {
            output.push_str(line);
            output.push('\n');
        }
    }
    if !found {
        output.push_str(&format!("\n[{section}]\n{binding}"));
    }
    fs::write(path, output).map_err(|e| e.to_string())
}

pub fn export(args: ExportArgs) -> Result<Value> {
    let project = project_root(&args.project)?;
    let engine = environment::resolve(args.godot.as_deref(), Some(&project))?;
    if !project.join("export_presets.cfg").is_file() {
        return Err(
            "Missing export_presets.cfg; create a desktop export preset in Godot first".into(),
        );
    }
    let preset = native_preset(&project, &args.preset)?;
    let (output, copied, before) = snapshot(&project, &args.output)?;
    let mut report = json!({"schemaVersion":"1","operation":"export","sourceProject":project,"snapshot":copied,"engine":engine,"forgeVersion":env!("CARGO_PKG_VERSION"),"forgeBuild":crate::build_info::current(),"preset":args.preset,"templates":template_status(&engine),"export":{"status":"not_run"},"exportedRuntime":{"status":"not_run"},"visualReview":"not_assessed"});
    let result = (|| {
        let profile = RunProfile::new(&output)?;
        report["userData"] = json!({"isolation":"per_run_profile","root":profile.root});
        let template = match &preset.custom_template {
            Some(path) => path.clone(),
            None => standard_release_template(&engine, &preset.architecture)?,
        };
        let template_hash = environment::digest(&template)?;
        bind_release_template(&copied, &preset.options_section, &template)?;
        report["releaseTemplate"] = json!({"path":template,"sha256":template_hash,"source":if preset.custom_template.is_some() { "custom" } else { "standard" }});
        report["customTemplate"] = if preset.custom_template.is_some() {
            json!({"path":template,"sha256":template_hash})
        } else {
            json!({"path":null,"sha256":null})
        };
        let artifacts = output.join("artifacts");
        fs::create_dir(&artifacts).map_err(|e| e.to_string())?;
        let target = artifacts.join(if cfg!(windows) {
            "game.exe"
        } else if cfg!(target_os = "macos") {
            "game.zip"
        } else {
            "game.x86_64"
        });
        report["export"] = json!({"status":"running"});
        report["export"] = run_process(
            engine_command(&engine, &copied, &profile)
                .args(["--headless", "--export-release"])
                .arg(&args.preset)
                .arg(&target),
            &output,
            "export",
            args.timeout,
            args.cancel_file.as_deref(),
            true,
        )?;
        if !target.is_file() || fs::metadata(&target).map_err(|e| e.to_string())?.len() == 0 {
            return Err(
                "Export produced no executable/archive; check preset and matching export templates"
                    .into(),
            );
        }
        report["artifact"] = json!({"path":target,"sha256":environment::digest(&target)?});
        report["artifactInventory"] =
            serde_json::to_value(inventory(&artifacts)?).map_err(|e| e.to_string())?;
        if args.run {
            let binary = if cfg!(target_os = "macos") {
                let expanded = output.join("exported");
                extract(&target, &expanded, &output)?;
                let app = fs::read_dir(&expanded)
                    .map_err(|e| e.to_string())?
                    .flatten()
                    .find(|e| e.path().extension().is_some_and(|x| x == "app"))
                    .ok_or("Export archive contains no app")?
                    .path();
                fs::read_dir(app.join("Contents/MacOS"))
                    .map_err(|e| e.to_string())?
                    .flatten()
                    .find(|e| e.path().is_file())
                    .ok_or("Exported app contains no executable")?
                    .path()
            } else {
                target
            };
            report["exportedRuntime"] = json!({"status":"running"});
            report["exportedRuntime"] = run_process(
                profile.apply(Command::new(binary).current_dir(&artifacts).args([
                    "--headless",
                    "--quit-after",
                    "30",
                ])),
                &output,
                "exported-runtime",
                args.timeout,
                args.cancel_file.as_deref(),
                true,
            )?;
            report["exportedRuntime"]["scope"] = json!("bounded_startup_smoke_only");
        }
        if before != inventory(&project)? {
            return Err("Source changed during export; evidence is stale".into());
        }
        if environment::digest(&template)? != template_hash {
            return Err("Release export template changed during export; evidence is stale".into());
        }
        Ok(())
    })();
    finish(&output, &report, result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_version_requires_explicit_download_and_known_pin() {
        use clap::Parser;
        for args in [
            vec!["forge", "setup", "godot", "--version", "4.7.2"],
            vec!["forge", "setup", "godot", "--download", "--version", "4.8"],
            vec!["forge", "setup", "godot", "--path", "godot", "--download"],
        ] {
            assert!(crate::Cli::try_parse_from(args).is_err());
        }
        for version in ["4.6.3", "4.7.2"] {
            assert!(crate::Cli::try_parse_from([
                "forge",
                "setup",
                "godot",
                "--download",
                "--version",
                version,
                "--templates"
            ])
            .is_ok());
        }
    }

    #[test]
    fn template_reporting_and_downloads_follow_selected_engine() {
        for (version, templates, pin) in [
            (
                "4.6.3.stable.official.fixture",
                "4.6.3.stable",
                Some("4.6.3"),
            ),
            (
                "4.7.2.stable.official.fixture",
                "4.7.2.stable",
                Some("4.7.2"),
            ),
            ("4.7.stable.official.fixture", "4.7.stable", None),
            (
                "4.7.2.stable.mono.official.fixture",
                "4.7.2.stable.mono",
                None,
            ),
            ("4.7.2.rc1.official.fixture", "4.7.2.rc1", None),
        ] {
            let engine = Engine {
                path: "godot".into(),
                version: version.into(),
                sha256: String::new(),
                companion_sha256: None,
            };
            assert_eq!(template_version(&engine), templates);
            assert_eq!(template_status(&engine)["version"], templates);
            assert_eq!(
                MANAGED_RELEASES
                    .iter()
                    .find(|r| r.matches(&engine))
                    .map(|r| r.version),
                pin
            );
        }
    }

    #[test]
    fn standard_template_uses_selected_engine_version_and_self_contained_root() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("._sc_"), "").unwrap();
        let templates = temp
            .path()
            .join("editor_data/export_templates/4.6.9.stable");
        fs::create_dir_all(&templates).unwrap();
        let filename = if cfg!(windows) {
            "windows_release_x86_64.exe"
        } else if cfg!(target_os = "macos") {
            "macos.zip"
        } else {
            "linux_release.x86_64"
        };
        let template = templates.join(filename);
        fs::write(&template, "fixture").unwrap();
        let engine = Engine {
            path: temp.path().join("godot"),
            version: "4.6.9.stable.official.fixture".into(),
            sha256: String::new(),
            companion_sha256: None,
        };
        assert_eq!(
            standard_release_template(&engine, "x86_64").unwrap(),
            template.canonicalize().unwrap()
        );
        assert!(standard_release_template(&engine, "../../escape").is_err());
    }

    #[test]
    fn template_binding_preserves_other_presets_and_resolves_relative_paths() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("game");
        fs::create_dir(&project).unwrap();
        let template = temp.path().join("release template 模板.zip");
        fs::write(&template, "fixture").unwrap();
        let platform = if cfg!(windows) {
            "Windows Desktop"
        } else if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Linux"
        };
        let path = project.join("export_presets.cfg");
        let other = "[preset.9.options]\ncustom_template/release=\"untouched\"\n";
        fs::write(&path, format!("[preset.2]\nname=\"Desktop\"\nplatform=\"{platform}\"\n[preset.2.options]\ncustom_template/release=\"../release template 模板.zip\"\napplication/bundle_identifier=\"test.preserved\"\n{other}")).unwrap();
        let preset = native_preset(&project, "Desktop").unwrap();
        assert_eq!(
            preset.custom_template,
            Some(template.canonicalize().unwrap())
        );
        bind_release_template(
            &project,
            &preset.options_section,
            preset.custom_template.as_ref().unwrap(),
        )
        .unwrap();
        let rebound = native_preset(&project, "Desktop").unwrap();
        assert_eq!(rebound.custom_template, preset.custom_template);
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.matches("custom_template/release=").count(), 2);
        assert!(text.contains(other));
        assert!(text.contains("application/bundle_identifier=\"test.preserved\""));
    }

    #[test]
    fn template_binding_creates_missing_options_and_accepts_res_paths() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path();
        let template = project.join("template.zip");
        fs::write(&template, "fixture").unwrap();
        let platform = if cfg!(windows) {
            "Windows Desktop"
        } else if cfg!(target_os = "macos") {
            "macOS"
        } else {
            "Linux"
        };
        let path = project.join("export_presets.cfg");
        let base = format!("[preset.0]\nname=\"Desktop\"\nplatform=\"{platform}\"\n");
        fs::write(&path, &base).unwrap();
        assert!(native_preset(project, "Desktop")
            .unwrap()
            .custom_template
            .is_none());
        bind_release_template(
            project,
            "preset.0.options",
            &template.canonicalize().unwrap(),
        )
        .unwrap();
        assert_eq!(
            native_preset(project, "Desktop").unwrap().custom_template,
            Some(template.canonicalize().unwrap())
        );
        fs::write(
            &path,
            format!("{base}[preset.0.options]\ncustom_template/release=\"res://template.zip\"\n"),
        )
        .unwrap();
        assert_eq!(
            native_preset(project, "Desktop").unwrap().custom_template,
            Some(template.canonicalize().unwrap())
        );
    }
}
