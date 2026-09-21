# Developer verification

Run commands from the repository root. Select checks for the behavior changed;
the rows below are entrypoints, not a requirement to run every suite.
Use an absolute path to the freshly built CLI (account for `CARGO_TARGET_DIR`),
and isolated `FORGE_JOB_STORE` / `FORGE_PLAN_STORE` directories for manual fixtures.
Native checks use a separately installed Godot 4.6.x or 4.7.x executable. Install Python
dependencies from `scripts/requirements-local-assets.txt` in a virtual environment;
CI uses Python 3.12 and synthetic media without Provider credentials.

## Source checks

For Rust changes, start with `cargo fmt --all -- --check` and focused tests.
Use `cargo test --locked -p pack` when changing Pack formats or processing output.
The following names are `packages/core/tests/` integration-test targets; run each
relevant group with `cargo test --locked -p core --test NAME [--test NAME ...]`.

| Changed contract | Core test targets |
| --- | --- |
| Resource identity, revisions and transfer | `content_digest_tests`, `asset_library_tests`, `asset_library_transfer_tests` |
| Resource reviews and acceptance | `asset_library_review_tests` |
| PNG preparation and native static delivery | `prepare_static_tests`, `static_delivery_tests` |
| Matting and layered Packs | `source_matte_tests`, `layered_pack_tests` |
| Animation, pixels, anchors and timing | `animation_delivery_tests`, `animation_pixel_quality_tests`, `anchor_tests` |
| Local audio | `audio_plan_tests`, `audio_processing_tests` |
| Source identity, receipts and install rollback | `source_lock_tests`, `image_contract_tests`, `delivery_audit_tests`, `godot_install_transaction_tests` |

Catalog changes also have internal tests:

```sh
cargo test --locked -p core --lib catalog::tests
cargo test --locked -p core --lib library::
```

For broader CLI integration, use
`FORGE_BINARY=/absolute/path/to/forge bash scripts/test-cli-product.sh`.
Optional world-asset changes additionally use `cargo test --locked -p forge-cli
--features world-assets` and `bash scripts/test-world-assets.sh`.
Use `bash scripts/test-cli-signing-contract.sh` for signing changes.

## CLI and native Godot checks

Run the relevant Python scripts below with `python3 scripts/SCRIPT` and the listed
arguments. `FORGE`, `GODOT` and `OUTPUT` stand for absolute paths; `OUTPUT` is a new
temporary evidence directory, not a consumer project. Read a script's `--help`
for additional modes. On Windows use `python` and native executable paths.

| Area | Script and arguments |
| --- | --- |
| Catalog commands | `test-asset-library-cli.py --forge FORGE` |
| Review records and browser previews | `test-asset-library-review-cli.py --forge FORGE` |
| Local transfer roundtrip | `test-asset-library-portability-cli.py --forge FORGE --mode roundtrip --godot GODOT --output OUTPUT` |
| Static PNG and native resources | `test-local-static-cli.py --forge FORGE --godot GODOT` |
| Image contracts | `test-image-contract-cli.py --forge FORGE` |
| Matting | `test-static-native-matte-cli.py --forge FORGE` |
| Existing animation frames | `test-local-animation-delivery.py --forge FORGE --godot GODOT --output OUTPUT` |
| Layered delivery | `test-layered-cli.py --forge FORGE --godot GODOT --output OUTPUT` |
| Layered preview UI | `test-godot-preview-ui.py --project PREVIEW_PROJECT --godot GODOT --output OUTPUT` |
| Unified layered playback | `test-godot-unified-player.py --godot GODOT --controller scripts/godot/runtime/layered-player-v1.gd` |
| Native install/update/rollback (both platforms) | `test-native-godot-transactions.py --godot GODOT --output OUTPUT` |
| External animation clock | `test-godot-external-clock.py --godot GODOT` |
| Audio processing, receipts and caches | `test-local-audio-cli.py --forge FORGE --godot GODOT` |
| Agent audio/static delivery and recovery | `experiments/agent_supporting_resources.py --forge FORGE --godot GODOT --output OUTPUT` (Python 3.10+) |
| Native audio resources | `test-godot-audio-delivery.py --godot GODOT` |
| Source locks, stable targets, receipts and audits | `test-sword-feedback-cli.py --forge FORGE --godot GODOT` |
| Storage diagnostics and complete pinned local delivery | `test-survival-feedback-cli.py --forge FORGE --godot GODOT --output OUTPUT` |

`PREVIEW_PROJECT` is the `preview/project` produced by the layered CLI test.
The Survival feedback runner also accepts `--storage-root EXISTING_DIRECTORY
--expect-storage-unsupported` for an explicit native external-volume probe. It
creates and removes its own subdirectory and preserves caller files. Omit the
expectation flag on supported storage. Keep ExFAT evidence separate from ordinary
macOS/Windows CI and do not infer full installation support from a probe.
Use separate output directories for each script. Native static regression checks
also include `cargo test --locked -p core --test static_delivery_tests -- --ignored
--nocapture`. For a nonstandard Godot location, set both `FORGE_GODOT_PATH` (Forge
installer) and `GODOT_BIN` (the Rust test's final scene check). The local-static
Python script's `--godot` selects both its install Jobs and final scene check.

The native transaction runner requires all five named real-Godot tests to be
discovered and pass, and retains discovery/execution logs and a JSON summary.
It supplies `FORGE_GODOT_PATH` explicitly and runs the tests serially. Its
failure-injection executables are compiled by the active `rustc` on both macOS
and Windows. Only the three separate tests using Unix Python/shebang stubs
remain Unix-only. A successful Cargo exit with zero tests is not acceptance;
run `python3 scripts/test-native-godot-gate.py` to check the gate's rejection cases.

A local transfer roundtrip does not establish cross-platform portability. Follow
[the quality matrix](../../../../.github/workflows/v03-quality.yml): produce
fixtures on macOS and Windows, then consume the other platform's fixture using
`--mode consume --input FIXTURE --require-foreign --output OUTPUT`. Keep the
selected `--forge` and `--godot` arguments. Retain both producers' and consumers'
evidence. Fixture media checks do not establish visual or listening approval.

## Embedded guide versus developer documentation

The optional ready-PNG comparison is an experiment, not a required performance
gate: follow [the protocol](../../../../docs/qa/asset-delivery-comparison.md).
When changing its runner/checker, run
`python3 scripts/experiments/test-static-delivery-comparison.py -v` and the native
pilot with isolated projects. Do not infer human productivity from its timings.

Changes to `.agents/skills/forge-use/` change the compiled payload. Rebuild with
`cargo build --locked -p forge-cli --no-default-features`, then run
`python3 scripts/test-cli-skill.py --forge /absolute/path/to/forge`.
The explicit bundle file list is in `packages/cli/src/skill.rs`. Test isolated
installation/update and exact guide content; never use personal skill directories.
Windows currently uses `--guide-only`; full installation tests run on macOS.

Root instructions and `forge-dev` are not embedded. For documentation-only edits,
check local links, command/target existence and `git diff --check`; use the
available skill-creator validator for changed skills. A full Rust/native matrix
is unnecessary unless compiled inputs or executable behavior also changed.

## Release and platform evidence

Follow [release-cli.yml](../../../../.github/workflows/release-cli.yml) for paired
macOS/Windows releases and [windows-portable.yml](../../../../.github/workflows/windows-portable.yml)
for native Windows package checks. The [Windows portable guide](../../../../docs/releases/windows-portable.md)
describes the public `forge.cmd` launcher, helpers and installation paths.
Windows source compilation alone does not verify a portable package. A macOS
run cannot replace Windows execution. CLI binaries are unsigned; macOS builds
are not notarized. Local development signing does not change release status.

## Verify installed macOS release artifacts

Exercise fresh installation, same-version reinstall and old-to-new upgrade with
the actual release archives and installer. Use the installer's public
`<bin-dir>/forge` symlink for CLI calls, including `scripts/verify-cli-build.py`.
Pass an absolute path without resolving that symlink before execution. Do not
prepend the payload's `bin` directory to `PATH`: either shortcut can hide a
launcher or bundled-helper discovery bug.

Disable external FFmpeg/FFprobe discovery during installed-package checks. Point
the search override at an empty temporary directory and disable macOS default
tool directories; keep Godot available separately:

```bash
forge_empty_tool_dir="$(mktemp -d)"
GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS="$forge_empty_tool_dir" \
GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1 \
  bash scripts/test-cli-release-artifact.sh \
    /absolute/current-release TAG /absolute/previous-release PREVIOUS_TAG
```

Compare compiled version/commit with the expected tag using the identity
verifier's `--version`, `--commit` and `--release` options. Supply `--build-info`
from that installation's payload and check that reported `ffmpegPath` and
`ffprobePath` point into the same payload. Resolving returned helper paths for
comparison is appropriate; resolving the public launcher before calling it is
not. Retain installation and identity results as release evidence; source-build
or fixture-only passes do not replace checks of the packaged binaries.
