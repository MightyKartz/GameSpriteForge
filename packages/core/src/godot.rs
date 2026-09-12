//! Shared native Godot discovery for the CLI doctor and delivery Jobs.
use std::{
    env,
    path::{Path, PathBuf},
};

pub fn locate_godot() -> Option<PathBuf> {
    if let Some(path) = env::var_os("FORGE_GODOT_PATH")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
    {
        return Some(path);
    }
    #[cfg(target_os = "macos")]
    for path in [
        "/Applications/Godot.app/Contents/MacOS/Godot",
        "/Applications/Godot_mono.app/Contents/MacOS/Godot",
    ] {
        if Path::new(path).is_file() {
            return Some(path.into());
        }
    }
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|directory| find_in_directory(&directory))
    })
}

fn find_in_directory(directory: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    let names = ["godot4.exe", "godot.exe", "godot_console.exe"];
    #[cfg(not(windows))]
    let names = ["godot4", "godot", "godot_console"];
    names
        .into_iter()
        .map(|name| directory.join(name))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_discovery_handles_platform_suffixes_and_spaces() {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("engine tools");
        std::fs::create_dir(&directory).unwrap();
        let engine = directory.join(if cfg!(windows) {
            "godot4.exe"
        } else {
            "godot4"
        });
        std::fs::write(&engine, b"fixture").unwrap();
        assert_eq!(find_in_directory(&directory), Some(engine));
    }
}
