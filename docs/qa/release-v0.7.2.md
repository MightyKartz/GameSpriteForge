# v0.7.2 release verification

Scope: default-feature macOS Apple Silicon and experimental Windows x64 packages,
carrying the cross-platform skill installation change (#70) onto v0.7.1.
Preparation baseline: `c4b5e20aa3134a2b7453d32d22af4c6386140df7` (main after #70).
No consumer projects, toolchain locks, source media or historical receipts change.

## Feature gates and evidence

| Gate | Evidence source |
| --- | --- |
| Windows no-replace commit (`MoveFileExW` without the replace flag) | `skill::tests::commit_rename_never_replaces_a_concurrently_created_empty_directory`, now compiled and run on Windows |
| Full install/update/backup/protection contract on Windows | `cargo test -p forge-cli` skill suite (28 tests), `scripts/test-cli-skill.py` full mode (24/24 cases) on a Windows 11 x86_64-pc-windows-gnu host |
| Manifest key separator identity | `skill::tests::install_is_idempotent_and_preserves_a_complete_old_bundle_outside_discovery` passing on Windows |
| Symlink protection with privilege probing | Rust tests and the CLI script probe; unprivileged sessions record an explicit skip, privileged CI runners exercise the full checks |
| Workspace lint parity with CI | `cargo clippy --locked --workspace --all-targets -- -D warnings` clean (the exact godot-workflow gate) |
| CI on the merged head | Godot agent workflow (macOS/Windows x Godot 4.6.3/4.7.2), Forge v0.3 Quality Matrix, Windows portable package: all success on `c16ef03` |

## Environment notes

- The pre-existing local failures `game_art::diff::tests::pack_directory_hash_matches_runner_algorithm`
  and `build_script::tests::worktrees_watch_their_own_head_and_shared_refs` reproduce on the clean
  baseline on this Windows host and are outside the changed contracts.
- The recurring transient `Pinned download SHA-256 mismatch` in the FFmpeg build step is a runner
  download issue; affected runs pass on rerun. Tracked as release hygiene, unrelated to this patch.
- Before merging this preparation PR, run `release-cli.yml` with `workflow_dispatch`, version
  `v0.7.2`, on the PR branch as the package preflight. After merge, the tag workflow repeats the
  gates for the exact merged commit; only its successful packages are published.
