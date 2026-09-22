# v0.7.1 release verification

Scope: default-feature macOS Apple Silicon and experimental Windows x64 packages,
carrying the asset-discovery patch set (#65–#68) onto v0.7.0. Preparation
baseline: `1de8ce554797ce2e675184bca7fe449160abe6ed` (main after #68).
No consumer projects, toolchain locks, source media or historical receipts change.

## Feature gates and evidence

| Gate | Evidence source |
| --- | --- |
| Vocabulary and metadata-only search contracts | `asset_library_tests.rs` (`vocabulary_and_metadata_only_search_skip_source_byte_verification`, `vocabulary_counts_revisions_and_ignores_untagged_assets`), `scripts/test-asset-library-cli.py` check `vocabulary_and_metadata_search` |
| Preview media manifest incl. animation player frames | `asset_library_review_tests.rs` manifest assertions, `scripts/test-asset-library-review-cli.py` check `media_manifest`, `scripts/test-animation-preview-cli.py` manifest frame bytes |
| Requirements reconciliation (four statuses, templates, read-only, invalid input rejection) | `asset_library_tests.rs` (`requirements_reconciliation_reports_each_status_and_stays_read_only`, `requirements_validate_batch_shape_before_reading`), `scripts/test-asset-library-cli.py` check `requirements_reconciliation` with `schemas/asset-requirements.schema.json` and `examples/asset-library/requirements.json` validation |
| Capability identity | `doctor --json` capabilities asserted in the CLI scripts above |
| Zero Provider requests / no writes from new commands | the same offline CLI scripts assert store inventories unchanged and no Job/Plan stores are created |

## Local verification (Windows 11, x86_64-pc-windows-gnu, Rust 1.98.1)

- `cargo fmt --check`, `cargo clippy -p core -p forge-cli --no-default-features`: clean.
- `cargo test -p core --no-default-features`: all library integration suites pass;
  the lib suite shows one pre-existing environment failure
  (`game_art::diff::tests::pack_directory_hash_matches_runner_algorithm`) that
  reproduces identically on the clean v0.7.0 baseline on this machine; the CLI
  package shows the pre-existing `worktrees_watch_their_own_head_and_shared_refs`
  environment failure, also reproduced on the baseline. Neither is touched by
  this patch set and both are outside the changed contracts.
- `scripts/test-asset-library-cli.py`, `scripts/test-asset-library-review-cli.py`,
  `scripts/test-animation-preview-cli.py` (with bundled helper discovery),
  `scripts/test-cli-skill.py --guide-only`: pass on the merged commits.

## CI and packaged gates

- PR branch runs: Windows portable package (fresh install, reinstall, historical
  upgrades, installed asset-library workflows, bundled FFmpeg/FFprobe) passed on
  the #66, #67 and #68 heads; Forge v0.3 Quality Matrix and Godot agent workflow
  passed on the #66 head and subsequent runs. Two transient failures during the
  campaign (a pinned zlib download SHA mismatch in the FFmpeg build step, and a
  pre-rebase run missing the then-new `animation_preview_tests` target) were
  resolved by rerun and by rebasing onto current main respectively; final runs
  on the rebased commits are green.
- Before merging this preparation PR, run `release-cli.yml` with
  `workflow_dispatch`, version `v0.7.1`, on the PR branch as the package
  preflight. After merge, the tag workflow repeats the gates for the exact
  merged commit; only its successful packages are published.

Visual, listening and license approval remain separate evidence. This release
makes no real-project productivity claims.
