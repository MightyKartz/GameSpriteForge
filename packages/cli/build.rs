use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Default, PartialEq, Eq)]
struct GitIdentity {
    commit: Option<String>,
    dirty: Option<bool>,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    // The CLI lives at packages/cli, including in a source archive. Never search
    // ancestors for Git: an archive may have been unpacked inside another repo.
    let root = manifest_dir.parent().unwrap().parent().unwrap();
    let identity = git_identity(root);
    for path in watched_paths(root) {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    let features: BTreeSet<String> = env::vars()
        .filter_map(|(key, _)| {
            key.strip_prefix("CARGO_FEATURE_")
                .filter(|name| *name != "DEFAULT")
                .map(|name| name.to_ascii_lowercase().replace('_', "-"))
        })
        .collect();
    let metadata = format!(
        "pub const GIT_COMMIT: Option<&str> = {:?};\n\
         pub const DIRTY: Option<bool> = {:?};\n\
         pub const TARGET: &str = {:?};\n\
         pub const PROFILE: &str = {:?};\n\
         pub const FEATURES: &[&str] = &{:?};\n",
        identity.commit.as_deref(),
        identity.dirty,
        env::var("TARGET").unwrap(),
        env::var("PROFILE").unwrap(),
        features.into_iter().collect::<Vec<_>>(),
    );
    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("build_metadata.rs");
    if fs::read_to_string(&output).ok().as_deref() != Some(metadata.as_str()) {
        fs::write(output, metadata).expect("write CLI build metadata");
    }
}

fn git(root: &Path, args: &[&str]) -> Option<Vec<u8>> {
    let output = Command::new("git")
        .args(["--no-optional-locks", "-C"])
        .arg(root)
        .args(args)
        // Inherited Git overrides must not attribute this source to another repo.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .output()
        .ok()?;
    output.status.success().then_some(output.stdout)
}

fn git_text(root: &Path, args: &[&str]) -> Option<String> {
    String::from_utf8(git(root, args)?)
        .ok()
        .map(|text| text.trim().to_owned())
}

fn own_repository(root: &Path) -> bool {
    root.join(".git").exists()
        && git_text(root, &["rev-parse", "--show-toplevel"])
            .and_then(|path| fs::canonicalize(path).ok())
            .zip(fs::canonicalize(root).ok())
            .is_some_and(|(actual, expected)| actual == expected)
}

fn git_identity(root: &Path) -> GitIdentity {
    if !own_repository(root) {
        return GitIdentity::default();
    }
    GitIdentity {
        commit: git_text(root, &["rev-parse", "--verify", "HEAD^{commit}"]).filter(|commit| {
            matches!(commit.len(), 40 | 64) && commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        }),
        dirty: git(
            root,
            &[
                "status",
                "--porcelain=v1",
                "--untracked-files=normal",
                "--ignore-submodules=none",
            ],
        )
        .map(|status| !status.is_empty()),
    }
}

fn watched_paths(root: &Path) -> BTreeSet<PathBuf> {
    let mut paths = BTreeSet::new();
    // Watch source directories to catch newly added files as well as edits.
    // Never watch the workspace root or .git recursively: target/ and Git's
    // changing object database would invalidate otherwise unchanged builds.
    for relative in [
        "Cargo.toml",
        "Cargo.lock",
        "packages/cli/Cargo.toml",
        "packages/cli/build.rs",
        "packages/cli/src",
        "packages/core/Cargo.toml",
        "packages/core/src",
        "packages/pack/Cargo.toml",
        "packages/pack/src",
        "packages/providers/Cargo.toml",
        "packages/providers/src",
        "schemas",
        "profiles",
        "scripts/godot",
    ] {
        add_existing(&mut paths, root.join(relative));
    }
    if root.join(".git").is_file() {
        add_existing(&mut paths, root.join(".git"));
    }
    if !own_repository(root) {
        return paths;
    }
    for argument in ["--absolute-git-dir", "--git-common-dir"] {
        if let Some(directory) = git_text(root, &["rev-parse", argument]) {
            let directory = root.join(directory);
            for name in ["HEAD", "index", "commondir", "packed-refs", "refs"] {
                add_existing(&mut paths, directory.join(name));
            }
        }
    }
    // A dirty dependency, schema, or other tracked file must refresh the CLI's
    // identity even when its own Rust sources have not changed. Track existing
    // untracked files too, so their removal can restore a clean identity.
    if let Some(files) = git(
        root,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    ) {
        for file in files
            .split(|byte| *byte == 0)
            .filter(|file| !file.is_empty())
        {
            if let Ok(file) = std::str::from_utf8(file) {
                add_existing(&mut paths, root.join(file));
            }
        }
    }
    paths
}

fn add_existing(paths: &mut BTreeSet<PathBuf>, path: PathBuf) {
    if path.exists() {
        paths.insert(path);
    }
}

#[cfg(test)]
#[path = "tests/support/build_identity_tests.rs"]
mod tests;
