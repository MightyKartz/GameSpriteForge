//! Optional upstream audio tools are a metadata-only catalog, not Forge Providers.
//! Keep this module free of process execution, file-content reads and installation.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};
use serde_json::{json, Value};

type Result<T> = std::result::Result<T, (String, String)>;

#[derive(Subcommand)]
pub enum AudioToolsCommand {
    /// List optional external audio tools and upstream information, offline.
    List(AudioToolsListArgs),
    /// Observe known source files in an explicit directory without running a tool.
    Doctor(AudioToolsDoctorArgs),
}

#[derive(Args)]
pub struct AudioToolsListArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Args)]
pub struct AudioToolsDoctorArgs {
    #[arg(long, value_enum)]
    pub tool: AudioTool,
    /// Existing upstream checkout directory. Omit to report not_configured.
    #[arg(long)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum AudioTool {
    Acestep,
    StableAudio,
}

struct ToolInfo {
    id: &'static str,
    name: &'static str,
    repository_url: &'static str,
    code_license_url: &'static str,
    model_license_label: &'static str,
    model_license_url: &'static str,
    markers: &'static [&'static str],
}

impl AudioTool {
    fn info(self) -> ToolInfo {
        // Known source layouts, checked against the official repositories on 2026-09-12.
        // These names are metadata hints. They do not establish code identity or version.
        match self {
            Self::Acestep => ToolInfo {
                id: "acestep",
                name: "ACE-Step 1.5",
                repository_url: "https://github.com/ace-step/ACE-Step-1.5",
                code_license_url: "https://github.com/ace-step/ACE-Step-1.5/blob/main/LICENSE",
                model_license_label: "Review the selected upstream model card and license",
                model_license_url: "https://huggingface.co/ACE-Step/acestep-v15-turbo",
                markers: &[
                    "pyproject.toml",
                    "cli.py",
                    "acestep/acestep_v15_pipeline.py",
                ],
            },
            Self::StableAudio => ToolInfo {
                id: "stable-audio",
                name: "Stable Audio 3",
                repository_url: "https://github.com/Stability-AI/stable-audio-3",
                code_license_url:
                    "https://github.com/Stability-AI/stable-audio-3/blob/main/LICENSE",
                model_license_label: "Stability AI Community License; review current model terms",
                model_license_url: "https://stability.ai/license",
                markers: &["pyproject.toml", "run_gradio.py", "stable_audio_3/cli.py"],
            },
        }
    }
}

fn catalog_entry(tool: AudioTool) -> Value {
    let info = tool.info();
    json!({
        "id": info.id,
        "name": info.name,
        "optional": true,
        "bundled": false,
        "provider": false,
        "repositoryUrl": info.repository_url,
        "license": {
            "code": {"label": "MIT", "infoUrl": info.code_license_url},
            "models": {"label": info.model_license_label, "infoUrl": info.model_license_url},
            "localLicenseVerified": false,
            "eligibility": "not_checked"
        },
        "sourceLayout": {
            "kind": "upstream_checkout",
            "requiredFiles": info.markers,
            "catalogCheckedAt": "2026-09-12"
        }
    })
}

fn policy() -> Value {
    json!({
        "mode": "filesystem_metadata_only",
        "pathSelection": "explicit_path_only",
        "automaticInstallation": false,
        "generationSupported": false,
        "note": "External tools are optional. Install or run them separately only after explicit opt-in. Forge does not install dependencies, download weights, authenticate, or generate audio with these tools."
    })
}

pub fn run(command: AudioToolsCommand) -> Result<Value> {
    match command {
        AudioToolsCommand::List(_) => Ok(json!({
            "tools": [catalog_entry(AudioTool::Acestep), catalog_entry(AudioTool::StableAudio)],
            "policy": policy()
        })),
        AudioToolsCommand::Doctor(args) => doctor(args.tool, args.path.as_deref()),
    }
}

fn path_error(code: &str, path: &Path, error: impl std::fmt::Display) -> (String, String) {
    (
        code.into(),
        format!("cannot inspect audio tool path {}: {error}", path.display()),
    )
}

/// Check fixed relative components only. Symlinks below the selected root are
/// reported rather than followed, so a marker cannot redirect discovery elsewhere.
fn marker(root: &Path, relative: &str) -> Result<Value> {
    let mut path = root.to_path_buf();
    let mut components = Path::new(relative).components().peekable();
    while let Some(component) = components.next() {
        path.push(component);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(json!({"path": relative, "status": "missing"}));
            }
            Err(error) => {
                return Err(path_error("audio_tool_inspection_failed", &path, error));
            }
        };
        if metadata.file_type().is_symlink() {
            return Ok(json!({"path": relative, "status": "symlink_not_followed"}));
        }
        if components.peek().is_some() {
            if !metadata.is_dir() {
                return Ok(json!({"path": relative, "status": "parent_not_directory"}));
            }
        } else {
            return Ok(if metadata.is_file() {
                json!({"path": relative, "status": "observed", "bytes": metadata.len()})
            } else {
                json!({"path": relative, "status": "not_regular_file"})
            });
        }
    }
    unreachable!("catalog markers are nonempty relative paths")
}

fn doctor(tool: AudioTool, path: Option<&Path>) -> Result<Value> {
    let info = tool.info();
    let mut report = json!({
        "tool": catalog_entry(tool),
        "policy": policy(),
        "status": "not_configured",
        "path": null,
        "markers": [],
        "checks": {
            "runtime": "not_checked",
            "dependencies": "not_checked",
            "weights": "not_checked",
            "hardware": "not_checked",
            "authentication": "not_checked",
            "licenseEligibility": "not_checked",
            "generation": "not_checked"
        },
        "note": "No path selected. Pass --path to an existing upstream checkout to observe known source files. No installation paths were searched."
    });
    let Some(path) = path else {
        return Ok(report);
    };
    let metadata =
        fs::metadata(path).map_err(|error| path_error("audio_tool_path_invalid", path, error))?;
    if !metadata.is_dir() {
        return Err(path_error(
            "audio_tool_path_invalid",
            path,
            "--path must be an existing directory",
        ));
    }
    // Resolving an explicitly selected root allows user-managed checkout symlinks.
    // Nothing below this root is traversed apart from the fixed marker components.
    let root = fs::canonicalize(path)
        .map_err(|error| path_error("audio_tool_path_invalid", path, error))?;
    let markers = info
        .markers
        .iter()
        .map(|relative| marker(&root, relative))
        .collect::<Result<Vec<_>>>()?;
    let observed = markers
        .iter()
        .filter(|marker| marker["status"] == "observed")
        .count();
    report["status"] = json!(if observed == markers.len() {
        "source_files_observed"
    } else if observed == 0 {
        "missing"
    } else {
        "partial_source_files"
    });
    report["path"] = json!(root);
    report["markers"] = json!(markers);
    report["note"] = json!("Only known source-file names and filesystem metadata were inspected. This does not verify installation, source identity, version, runtime readiness, model weights, license eligibility, or generation quality.");
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "forge-audio-tools-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }

        fn file(&self, relative: &str) {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            // Deliberately invalid source: observation must not import or parse it.
            fs::write(path, b"metadata fixture; never run this file\xff").unwrap();
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn catalog_is_optional_and_has_separate_license_information() {
        let catalog = run(AudioToolsCommand::List(AudioToolsListArgs { json: true })).unwrap();
        let tools = catalog["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
        for tool in tools {
            assert_eq!(tool["optional"], true);
            assert_eq!(tool["bundled"], false);
            assert_eq!(tool["provider"], false);
            assert_eq!(tool["license"]["code"]["label"], "MIT");
            assert_eq!(tool["license"]["localLicenseVerified"], false);
            assert_eq!(tool["license"]["eligibility"], "not_checked");
        }
        assert_eq!(catalog["policy"]["automaticInstallation"], false);
        assert_eq!(catalog["policy"]["generationSupported"], false);
    }

    #[test]
    fn no_path_is_not_configured_and_makes_no_readiness_claim() {
        let report = doctor(AudioTool::Acestep, None).unwrap();
        assert_eq!(report["status"], "not_configured");
        assert!(report["path"].is_null());
        assert_eq!(report["markers"], json!([]));
        assert!(report["checks"]
            .as_object()
            .unwrap()
            .values()
            .all(|status| status == "not_checked"));
    }

    #[test]
    fn complete_fixture_layouts_observe_files_without_reading_source_or_loading_models() {
        for tool in [AudioTool::Acestep, AudioTool::StableAudio] {
            let fixture = Fixture::new();
            for relative in tool.info().markers {
                fixture.file(relative);
            }
            let report = doctor(tool, Some(&fixture.0)).unwrap();
            assert_eq!(report["status"], "source_files_observed");
            assert_eq!(report["checks"]["runtime"], "not_checked");
            assert_eq!(report["checks"]["weights"], "not_checked");
            for relative in tool.info().markers {
                assert_eq!(
                    fs::read(fixture.0.join(relative)).unwrap(),
                    b"metadata fixture; never run this file\xff"
                );
            }
            assert!(!fixture.0.join(".venv").exists());
            assert!(!fixture.0.join("checkpoints").exists());
            assert!(!fixture.0.join("__pycache__").exists());
        }
    }

    #[test]
    fn missing_and_partial_layouts_are_read_only_diagnostics() {
        let fixture = Fixture::new();
        let before = fs::metadata(&fixture.0).unwrap().modified().unwrap();
        let missing = doctor(AudioTool::StableAudio, Some(&fixture.0)).unwrap();
        assert_eq!(missing["status"], "missing");
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 0);
        assert_eq!(
            fs::metadata(&fixture.0).unwrap().modified().unwrap(),
            before
        );

        fixture.file("pyproject.toml");
        // A nested copy is intentionally outside the fixed lookup locations.
        fixture.file("other/stable_audio_3/cli.py");
        let partial = doctor(AudioTool::StableAudio, Some(&fixture.0)).unwrap();
        assert_eq!(partial["status"], "partial_source_files");
        assert_eq!(partial["markers"][2]["status"], "missing");
    }

    #[test]
    fn invalid_supplied_paths_fail_without_creating_them() {
        let fixture = Fixture::new();
        let missing = fixture.0.join("not-installed");
        let error = doctor(AudioTool::Acestep, Some(&missing)).unwrap_err();
        assert_eq!(error.0, "audio_tool_path_invalid");
        assert!(!missing.exists());
        fixture.file("file");
        let error = doctor(AudioTool::Acestep, Some(&fixture.0.join("file"))).unwrap_err();
        assert_eq!(error.0, "audio_tool_path_invalid");
        assert!(error.1.contains("existing directory"));
    }

    #[test]
    fn directories_and_non_directory_parents_do_not_count_as_source_files() {
        let fixture = Fixture::new();
        fs::create_dir(fixture.0.join("pyproject.toml")).unwrap();
        fixture.file("acestep");
        let report = doctor(AudioTool::Acestep, Some(&fixture.0)).unwrap();
        assert_eq!(report["status"], "missing");
        assert_eq!(report["markers"][0]["status"], "not_regular_file");
        assert_eq!(report["markers"][2]["status"], "parent_not_directory");
    }

    #[cfg(unix)]
    #[test]
    fn does_not_follow_marker_or_nested_directory_symlinks() {
        use std::os::unix::fs::symlink;
        let fixture = Fixture::new();
        let elsewhere = Fixture::new();
        elsewhere.file("pyproject.toml");
        elsewhere.file("cli.py");
        symlink(
            elsewhere.0.join("pyproject.toml"),
            fixture.0.join("pyproject.toml"),
        )
        .unwrap();
        symlink(&elsewhere.0, fixture.0.join("stable_audio_3")).unwrap();
        let report = doctor(AudioTool::StableAudio, Some(&fixture.0)).unwrap();
        assert_eq!(report["status"], "missing");
        assert_eq!(report["markers"][0]["status"], "symlink_not_followed");
        assert_eq!(report["markers"][2]["status"], "symlink_not_followed");
    }
}
