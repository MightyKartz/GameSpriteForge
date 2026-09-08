use super::*;
use std::process::Output;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FIXTURE: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    directory: PathBuf,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let directory = env::temp_dir().join(format!(
            "forge-build-identity-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        let root = directory.join("repo");
        fs::create_dir_all(root.join("packages/cli/src")).unwrap();
        fs::create_dir_all(root.join("packages/core/src")).unwrap();
        fs::write(root.join(".gitignore"), "/target/\n").unwrap();
        fs::write(root.join("packages/core/src/lib.rs"), "// source\n").unwrap();
        let fixture = Self { directory, root };
        fixture.run_git(&["init", "-q", "-b", "main"]);
        fixture
    }

    fn run_git(&self, args: &[&str]) -> Vec<u8> {
        let mut all_args = vec![
            "-c",
            "user.name=Build Identity Test",
            "-c",
            "user.email=build-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=/dev/null",
        ];
        all_args.extend_from_slice(args);
        git(&self.root, &all_args).unwrap_or_else(|| panic!("git {args:?} failed"))
    }

    fn commit(&self) {
        self.run_git(&["add", "."]);
        self.run_git(&["commit", "-q", "--allow-empty", "-m", "fixture"]);
    }

    fn source(&self) -> PathBuf {
        self.root.join("packages/core/src/lib.rs")
    }

    fn add_cargo_probe(&self) {
        fs::write(
            self.root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"packages/cli\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        fs::write(
            self.root.join("packages/cli/Cargo.toml"),
            "[package]\nname = \"build-identity-probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n[features]\noptional-test = []\n",
        )
        .unwrap();
        fs::write(
            self.root.join("packages/cli/build.rs"),
            include_str!("../../build.rs"),
        )
        .unwrap();
        fs::write(
            self.root.join("packages/cli/src/main.rs"),
            r#"include!(concat!(env!("OUT_DIR"), "/build_metadata.rs"));
fn main() {
    println!("{GIT_COMMIT:?}\n{DIRTY:?}\n{TARGET}\n{PROFILE}\n{FEATURES:?}");
}
"#,
        )
        .unwrap();
        self.cargo(&["generate-lockfile", "--offline"]);
    }

    fn cargo(&self, args: &[&str]) -> Output {
        let output = Command::new("cargo")
            .args(args)
            .current_dir(&self.root)
            .env("CARGO_TARGET_DIR", self.root.join("target"))
            .env("CARGO_INCREMENTAL", "0")
            .output()
            .expect("run minimal Cargo probe");
        assert!(
            output.status.success(),
            "cargo {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn probe(&self) -> String {
        self.cargo(&["build", "--offline", "--quiet"]);
        let output = Command::new(self.binary()).output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }

    fn binary(&self) -> PathBuf {
        self.root
            .join("target/debug")
            .join(format!("build-identity-probe{}", env::consts::EXE_SUFFIX))
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn clean_dirty_and_failed_status_are_distinct() {
    let fixture = Fixture::new();
    fixture.commit();
    let clean = git_identity(&fixture.root);
    assert_eq!(clean.dirty, Some(false));
    assert_eq!(clean.commit.as_ref().map(String::len), Some(40));

    fs::write(fixture.source(), "// changed\n").unwrap();
    assert_eq!(git_identity(&fixture.root).dirty, Some(true));

    fs::write(fixture.root.join(".git/index"), "broken index").unwrap();
    let broken = git_identity(&fixture.root);
    assert_eq!(broken.commit, clean.commit);
    assert_eq!(broken.dirty, None);
}

#[test]
fn source_archive_does_not_inherit_its_parent_repository() {
    let fixture = Fixture::new();
    fixture.commit();
    let archive = fixture.root.join("source-archive");
    fs::create_dir_all(archive.join("packages/cli/src")).unwrap();
    assert_eq!(git_identity(&archive), GitIdentity::default());
    fs::write(archive.join(".git"), "gitdir: nonexistent\n").unwrap();
    assert_eq!(git_identity(&archive), GitIdentity::default());
}

#[test]
fn worktrees_watch_their_own_head_and_shared_refs() {
    let fixture = Fixture::new();
    fixture.commit();
    let linked = fixture.directory.join("linked");
    fixture.run_git(&[
        "worktree",
        "add",
        "--quiet",
        "-b",
        "linked",
        linked.to_str().unwrap(),
        "HEAD",
    ]);
    assert_eq!(git_identity(&linked), git_identity(&fixture.root));
    let paths = watched_paths(&linked);
    assert!(paths.contains(&linked.join(".git")));
    let git_root = fs::canonicalize(fixture.root.join(".git")).unwrap();
    assert!(paths.contains(&git_root.join("worktrees/linked/HEAD")));
    assert!(paths.contains(&git_root.join("refs")));
    fs::write(linked.join("packages/core/src/lib.rs"), "// changed\n").unwrap();
    assert_eq!(git_identity(&linked).dirty, Some(true));
    assert_eq!(git_identity(&fixture.root).dirty, Some(false));
}

#[test]
fn cargo_refreshes_source_and_git_changes_but_ignores_generated_target_files() {
    let fixture = Fixture::new();
    fixture.add_cargo_probe();
    fixture.commit();
    let clean = fixture.probe();
    assert!(clean.contains("\nSome(false)\n"), "{clean}");
    assert!(clean.contains("\ndebug\n[]\n"), "{clean}");

    let binary_time = fs::metadata(fixture.binary()).unwrap().modified().unwrap();
    fs::write(fixture.root.join("target/generated-output"), "generated\n").unwrap();
    assert_eq!(fixture.probe(), clean);
    assert_eq!(
        fs::metadata(fixture.binary()).unwrap().modified().unwrap(),
        binary_time,
        "generated target files must not rebuild the CLI"
    );

    // The CLI source is untouched: an edit in another package must still update
    // the embedded dirty flag, and restoring it must reset that flag.
    fs::write(fixture.source(), "// changed\n").unwrap();
    assert!(fixture.probe().contains("\nSome(true)\n"));
    fixture.run_git(&["restore", "packages/core/src/lib.rs"]);
    assert_eq!(fixture.probe(), clean);

    let added = fixture.root.join("packages/core/src/added.rs");
    fs::write(&added, "// new source\n").unwrap();
    assert!(fixture.probe().contains("\nSome(true)\n"));
    fs::remove_file(added).unwrap();
    assert_eq!(fixture.probe(), clean);

    // An empty commit changes only the ref, not any compiled source.
    fixture.commit();
    let committed = fixture.probe();
    assert_ne!(committed, clean);
    assert!(committed.contains("\nSome(false)\n"));
    fixture.run_git(&["pack-refs", "--all", "--prune"]);
    assert_eq!(fixture.probe(), committed);
    fixture.commit();
    assert_ne!(fixture.probe(), committed);

    fixture.cargo(&[
        "build",
        "--offline",
        "--quiet",
        "--features",
        "optional-test",
    ]);
    let output = Command::new(fixture.binary()).output().unwrap();
    let feature_build = String::from_utf8(output.stdout).unwrap();
    assert!(feature_build.contains("\n[\"optional-test\"]\n"));
}
