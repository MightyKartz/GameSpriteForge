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
    /// Download the checksum-pinned official Godot 4.6.3 standard editor.
    #[arg(long)]
    download: bool,
    /// Also download the official 4.6.3 export templates (large download).
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
        json!({"engine":engine,"lock":environment::read_lock(&project)?,"templates":template_status(),"exportPresetsPresent":project.join("export_presets.cfg").is_file(),"runtime":"not_run","visualReview":"not_assessed"}),
    )
}

fn templates_root() -> Result<PathBuf> {
    dirs_path().map(|p| p.join("Godot/export_templates/4.6.3.stable"))
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

fn template_status() -> Value {
    match templates_root() {
        Ok(root) => {
            json!({"version":"4.6.3.stable","directory":root,"windowsRelease":root.join("windows_release_x86_64.exe").is_file(),"macos":root.join("macos.zip").is_file(),"note":"Actual export resolves preset/custom templates; this checks the managed 4.6.3 location only"})
        }
        Err(error) => json!({"error":error}),
    }
}

pub fn setup(args: SetupArgs) -> Result<Value> {
    let engine = if args.download {
        install_engine()?
    } else {
        environment::resolve(args.path.as_deref(), None)?
    };
    if args.templates {
        if !engine.version.starts_with("4.6.3.stable.") || engine.version.contains("mono") {
            return Err("Managed templates require the standard Godot 4.6.3 build".into());
        }
        install_templates()?;
    }
    let config = environment::save(&engine)?;
    Ok(
        json!({"engine":engine,"configPath":config,"templates":template_status(),"environmentModified":false,"next":"forge godot check --project PATH --json"}),
    )
}

const BASE: &str = "https://github.com/godotengine/godot-builds/releases/download/4.6.3-stable";

fn download_extract(name: &str, checksum: &str, parent: &Path) -> Result<tempfile::TempDir> {
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = tempfile::tempdir_in(parent).map_err(|e| e.to_string())?;
    let archive = temp.path().join("download.zip");
    let url = format!("{BASE}/{name}");
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
    let mut file = fs::File::open(&archive).map_err(|e| e.to_string())?;
    let mut hash = Sha512::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
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

fn install_engine() -> Result<Engine> {
    let (archive, checksum, relative) = if cfg!(all(windows, target_arch = "x86_64")) {
        ("Godot_v4.6.3-stable_win64.exe.zip", "d44ea7ef5bab754cacd49d581b6062836b2eea12a82e1d183aebfad9cd8c7db2bd82513337bd657d6d2d5c04d46239c0570b029faf1343e81e8a2fa7b85dd83b", "Godot_v4.6.3-stable_win64_console.exe")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        ("Godot_v4.6.3-stable_macos.universal.zip", "0155bbc8dcf179edab4ef08e3dbbd4480d7434ab1990cb3fb25517d453b1b65787c7fe6dfc8824b15ff25e1fdcec59e8584f765054106894fee00311868c3964", "Godot.app/Contents/MacOS/Godot")
    } else {
        return Err(
            "Managed Godot download supports Windows x64 and macOS Apple Silicon; use --path"
                .into(),
        );
    };
    let root = environment::config_root()?.join("tools");
    let target = root.join("godot-4.6.3-standard");
    if target.exists() {
        let engine = environment::probe(&target.join(relative))?;
        let saved: Engine = serde_json::from_slice(
            &fs::read(target.join("forge-engine.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if saved.sha256 != engine.sha256 || saved.companion_sha256 != engine.companion_sha256 {
            return Err(
                "Managed engine was modified; select another installation explicitly".into(),
            );
        }
        return Ok(engine);
    }
    let temp = download_extract(archive, checksum, &root)?;
    environment::probe(&temp.path().join("unpacked").join(relative))?;
    fs::rename(temp.path().join("unpacked"), &target).map_err(|e| e.to_string())?;
    let engine = environment::probe(&target.join(relative))?;
    environment::write_json(&target.join("forge-engine.json"), &engine, false)?;
    Ok(engine)
}

fn install_templates() -> Result<()> {
    let target = templates_root()?;
    if target.exists() {
        if [
            "windows_release_x86_64.exe",
            "windows_debug_x86_64.exe",
            "macos.zip",
        ]
        .iter()
        .all(|name| target.join(name).is_file())
            && target.join("version.txt").is_file()
            && fs::read_to_string(target.join("version.txt"))
                .map_err(|e| e.to_string())?
                .trim()
                == "4.6.3.stable"
        {
            return Ok(());
        }
        return Err(
            "Existing template directory is incomplete or unrecognized; it was preserved".into(),
        );
    }
    let temp = download_extract("Godot_v4.6.3-stable_export_templates.tpz", "da606b61c10157844f8300172df374472665f95015495cb1a7cd132c40ede404faa96cc1016a4b9662db9909ddea69632c4948b2cd11163438dad4808881fb68", target.parent().ok_or("Invalid templates path")?)?;
    let source = temp.path().join("unpacked/templates");
    if fs::read_to_string(source.join("version.txt"))
        .map_err(|e| e.to_string())?
        .trim()
        != "4.6.3.stable"
    {
        return Err("Unexpected export template version".into());
    }
    fs::rename(source, target).map_err(|e| e.to_string())?;
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

fn engine_command(engine: &Engine, project: &Path) -> Command {
    let mut command = Command::new(&engine.path);
    command.arg("--path").arg(project);
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
    let mut report = json!({"schemaVersion":"1","operation":"verify","sourceProject":project,"snapshot":copied,"engine":engine,"forgeVersion":env!("CARGO_PKG_VERSION"),"forgeBuild":crate::build_info::current(),"visualReview":"not_assessed","interactionTests":"not_run","import":{"status":"not_run"},"runtime":{"status":"not_run"},"screenshot":{"status":"not_requested"}});
    let result = (|| {
        report["import"] = json!({"status":"running"});
        report["import"] = run_process(
            engine_command(&engine, &copied).args(["--headless", "--import"]),
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
        let mut command = engine_command(&engine, &copied);
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
                engine_command(&engine, &copied)
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

fn native_preset(project: &Path, name: &str) -> Result<Option<PathBuf>> {
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
    let custom = sections
        .get(&format!("{section}.options"))
        .and_then(|v| v.get("custom_template/release"))
        .filter(|v| !v.is_empty());
    custom
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
        .transpose()
}

pub fn export(args: ExportArgs) -> Result<Value> {
    let project = project_root(&args.project)?;
    let engine = environment::resolve(args.godot.as_deref(), Some(&project))?;
    if !project.join("export_presets.cfg").is_file() {
        return Err(
            "Missing export_presets.cfg; create a desktop export preset in Godot first".into(),
        );
    }
    let custom_template = native_preset(&project, &args.preset)?;
    let template_hash = custom_template
        .as_deref()
        .map(environment::digest)
        .transpose()?;
    let (output, copied, before) = snapshot(&project, &args.output)?;
    let mut report = json!({"schemaVersion":"1","operation":"export","sourceProject":project,"snapshot":copied,"engine":engine,"forgeVersion":env!("CARGO_PKG_VERSION"),"forgeBuild":crate::build_info::current(),"preset":args.preset,"templates":template_status(),"export":{"status":"not_run"},"exportedRuntime":{"status":"not_run"},"visualReview":"not_assessed"});
    let result = (|| {
        report["customTemplate"] = json!({"path":custom_template,"sha256":template_hash});
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
            engine_command(&engine, &copied)
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
                Command::new(binary).current_dir(&artifacts).args([
                    "--headless",
                    "--quit-after",
                    "30",
                ]),
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
        if custom_template
            .as_deref()
            .map(environment::digest)
            .transpose()?
            != template_hash
        {
            return Err("Custom export template changed during export; evidence is stale".into());
        }
        Ok(())
    })();
    finish(&output, &report, result)
}
