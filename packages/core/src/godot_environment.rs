//! Machine-local engine selection and portable project requirements.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub const LOCK_FILE: &str = ".forge/toolchain.lock.json";
pub type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Engine {
    pub path: PathBuf,
    pub version: String,
    pub sha256: String,
    /// Windows console launchers delegate to a sibling executable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion_sha256: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolchainLock {
    pub schema_version: u32,
    pub forge_version: String,
    pub godot_version: String,
}

pub fn config_root() -> Result<PathBuf> {
    if let Some(path) = env::var_os("FORGE_CONFIG_DIR") {
        return Ok(PathBuf::from(path));
    }
    dirs_next::config_dir()
        .map(|p| p.join("GameSpriteForge"))
        .ok_or("Cannot locate user configuration directory; set FORGE_CONFIG_DIR".into())
}

pub fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

pub fn executable(path: &Path) -> PathBuf {
    if path.extension().is_some_and(|e| e == "app") {
        path.join("Contents/MacOS/Godot")
    } else {
        path.to_path_buf()
    }
}

/// Version probes have a deadline even when the configured executable is broken.
pub fn probe(path: &Path) -> Result<Engine> {
    let path = executable(path)
        .canonicalize()
        .map_err(|e| format!("Godot path unavailable: {e}; run forge setup godot --path PATH"))?;
    let log = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut child = Command::new(&path)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(|e| e.to_string())?)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Godot version probe timed out".into());
        }
        thread::sleep(Duration::from_millis(20));
    };
    use std::io::{Seek, SeekFrom};
    let mut log = log;
    log.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut version = String::new();
    log.take(8192)
        .read_to_string(&mut version)
        .map_err(|e| e.to_string())?;
    let version = version.trim().to_string();
    if !status.success() || !version.starts_with("4.6.") || version.contains('\n') {
        return Err(format!(
            "Unsupported Godot version {version:?}; select Godot 4.6.x"
        ));
    }
    let companion_sha256 = path
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix("_console.exe"))
        .map(|name| digest(&path.with_file_name(format!("{name}.exe"))))
        .transpose()?;
    Ok(Engine {
        sha256: digest(&path)?,
        companion_sha256,
        path,
        version,
    })
}

pub fn configured() -> Result<Option<Engine>> {
    let path = config_root()?.join("godot.json");
    if !path.exists() {
        return Ok(None);
    }
    serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
        .map(Some)
        .map_err(|e| format!("Invalid {}: {e}", path.display()))
}

pub fn save(engine: &Engine) -> Result<PathBuf> {
    let path = config_root()?.join("godot.json");
    write_json(&path, engine, true)?;
    Ok(path)
}

pub fn write_json(path: &Path, value: &impl Serialize, replace: bool) -> Result<()> {
    let parent = path.parent().ok_or("Configuration has no parent")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("Refusing to replace a symlink".into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temp.write_all(&serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    temp.as_file().sync_all().map_err(|e| e.to_string())?;
    if replace {
        temp.persist(path).map_err(|e| e.to_string())?;
    } else {
        temp.persist_noclobber(path).map_err(|e| {
            format!("Already locked or cannot write: {e}; use --update to replace explicitly")
        })?;
    }
    Ok(())
}

pub fn candidates() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(paths) = env::var_os("PATH") {
        roots.extend(env::split_paths(&paths));
    }
    if let Some(home) = dirs_next::home_dir() {
        roots.extend([home.join("Downloads"), home.join("Applications")]);
    }
    #[cfg(windows)]
    {
        roots.extend([
            PathBuf::from("C:/Tools/Godot"),
            PathBuf::from("C:/Program Files/Godot"),
        ]);
        if let Some(local) = env::var_os("LOCALAPPDATA") {
            roots.push(PathBuf::from(local).join("Microsoft/WinGet/Links"));
        }
    }
    #[cfg(target_os = "macos")]
    roots.push(PathBuf::from("/Applications"));
    let mut candidates = Vec::new();
    for root in roots {
        for name in [
            "godot",
            "godot4",
            "godot.exe",
            "godot4.exe",
            "godot_console.exe",
            "Godot.app",
            "Godot_mono.app",
        ] {
            let path = executable(&root.join(name));
            if path.is_file() {
                candidates.push(path);
            }
        }
        if let Ok(entries) = fs::read_dir(&root) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.starts_with("godot_v")
                    && name.ends_with("_console.exe")
                    && entry.path().is_file()
                {
                    candidates.push(entry.path());
                }
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

/// An explicit/configured selection never silently falls back on failure.
pub fn resolve(explicit: Option<&Path>, project: Option<&Path>) -> Result<Engine> {
    let engine = if let Some(path) = explicit {
        probe(path)?
    } else if let Some(path) = env::var_os("FORGE_GODOT_PATH") {
        probe(Path::new(&path))?
    } else if let Some(saved) = configured()? {
        let current = probe(&saved.path)?;
        if current.sha256 != saved.sha256
            || current.companion_sha256 != saved.companion_sha256
            || current.version != saved.version
        {
            return Err("Configured Godot changed; rerun forge setup godot --path PATH to select it explicitly".into());
        }
        current
    } else {
        let required = project.map(read_lock).transpose()?.flatten();
        candidates().iter().filter_map(|p| probe(p).ok()).find(|e| required.as_ref().is_none_or(|l| l.godot_version == e.version)).ok_or("Godot not found; run forge setup godot --path PATH or forge setup godot --download")?
    };
    if let Some(project) = project {
        validate_lock(project, &engine)?;
    }
    Ok(engine)
}

pub fn read_lock(project: &Path) -> Result<Option<ToolchainLock>> {
    let path = project.join(LOCK_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let lock: ToolchainLock = serde_json::from_slice(&fs::read(&path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Invalid toolchain lock: {e}"))?;
    if lock.schema_version != 1 {
        return Err("Unsupported toolchain lock schema".into());
    }
    Ok(Some(lock))
}

pub fn validate_lock(project: &Path, engine: &Engine) -> Result<()> {
    if let Some(lock) = read_lock(project)? {
        if lock.forge_version != env!("CARGO_PKG_VERSION") || lock.godot_version != engine.version {
            return Err(format!("toolchain_mismatch: project requires Forge {} / Godot {}; selected Forge {} / Godot {}. Select the required tools or explicitly update the lock after verification", lock.forge_version, lock.godot_version, env!("CARGO_PKG_VERSION"), engine.version));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_lock_has_no_machine_path_and_refuses_implicit_update() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(LOCK_FILE);
        let lock = ToolchainLock {
            schema_version: 1,
            forge_version: env!("CARGO_PKG_VERSION").into(),
            godot_version: "4.6.3.stable.official.test".into(),
        };
        write_json(&path, &lock, false).unwrap();
        assert!(write_json(&path, &lock, false).is_err());
        assert_eq!(read_lock(dir.path()).unwrap(), Some(lock));
        assert!(!fs::read_to_string(path).unwrap().contains("path"));
        assert!(validate_lock(
            dir.path(),
            &Engine {
                path: "other-machine/godot".into(),
                version: "4.6.2".into(),
                sha256: "a".repeat(64),
                companion_sha256: None
            }
        )
        .is_err());
    }
}
