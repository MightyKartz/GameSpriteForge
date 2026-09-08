#![cfg(unix)]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct InstalledCli {
    root: PathBuf,
}

impl InstalledCli {
    fn new() -> Self {
        let fixture = Self {
            root: std::env::temp_dir().join(format!(
                "forge-bundled-helpers-{}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed),
            )),
        };
        fs::create_dir_all(fixture.payload_bin()).unwrap();
        fs::create_dir_all(fixture.public_bin()).unwrap();
        fs::create_dir(fixture.root.join("empty-helper-search")).unwrap();
        fs::copy(
            env!("CARGO_BIN_EXE_forge"),
            fixture.payload_bin().join("forge"),
        )
        .unwrap();
        // Doctor checks helper discovery without executing these placeholders.
        for helper in ["ffmpeg", "ffprobe"] {
            fs::write(fixture.payload_bin().join(helper), []).unwrap();
        }
        // Avoid invoking a host Godot installation during this discovery test.
        fs::write(fixture.root.join("non-executable-godot"), []).unwrap();
        symlink(
            "../../payload/bin/forge",
            fixture.public_bin().join("forge"),
        )
        .unwrap();
        symlink("forge", fixture.public_bin().join("forge-chain")).unwrap();
        fixture
    }

    fn payload_bin(&self) -> PathBuf {
        self.root.join("payload/bin")
    }

    fn public_bin(&self) -> PathBuf {
        self.root.join("public/bin")
    }

    fn doctor(&self, launcher: &str) -> Value {
        // Keep the public entry point intact: resolving it in the test would
        // conceal macOS current_exe() retaining the launcher's symlink path.
        let output = Command::new(self.public_bin().join(launcher))
            .args(["doctor", "--json"])
            .current_dir(&self.root)
            .env("FORGE_JOB_STORE", self.root.join("jobs"))
            .env("FORGE_PLAN_STORE", self.root.join("plans"))
            .env("FORGE_GODOT_PATH", self.root.join("non-executable-godot"))
            .env(
                "GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS",
                self.root.join("empty-helper-search"),
            )
            .env("GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "doctor through {launcher} failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(envelope["ok"], true, "{envelope}");
        envelope["data"].clone()
    }
}

impl Drop for InstalledCli {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn public_relative_and_chained_launchers_find_payload_helpers() {
    let fixture = InstalledCli::new();
    for launcher in ["forge", "forge-chain"] {
        let data = fixture.doctor(launcher);
        for (field, helper) in [("ffmpegPath", "ffmpeg"), ("ffprobePath", "ffprobe")] {
            let actual = data[field]
                .as_str()
                .unwrap_or_else(|| panic!("{launcher} did not discover {helper}: {data}"));
            assert_eq!(
                Path::new(actual).canonicalize().unwrap(),
                fixture.payload_bin().join(helper).canonicalize().unwrap(),
                "{launcher} must use the installed payload's {helper}",
            );
        }
    }
}

#[test]
fn incomplete_payload_does_not_use_helpers_beside_the_public_launcher() {
    let fixture = InstalledCli::new();
    fs::remove_file(fixture.payload_bin().join("ffprobe")).unwrap();
    // The public directory is not the bundle, even if it contains a full pair.
    for helper in ["ffmpeg", "ffprobe"] {
        fs::write(fixture.public_bin().join(helper), []).unwrap();
    }
    for launcher in ["forge", "forge-chain"] {
        let data = fixture.doctor(launcher);
        for field in ["ffmpegPath", "ffprobePath"] {
            assert_eq!(
                data.get(field),
                Some(&Value::Null),
                "{launcher} incorrectly accepted an incomplete payload: {data}",
            );
        }
    }
}
