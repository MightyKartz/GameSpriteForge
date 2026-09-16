//! Shared native Godot discovery.
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;

pub fn locate_godot() -> Option<PathBuf> {
    crate::godot_environment::resolve(None, None)
        .ok()
        .map(|engine| engine.path)
}

#[cfg(test)]
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
