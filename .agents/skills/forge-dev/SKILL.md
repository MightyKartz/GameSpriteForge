---
name: forge-dev
description: Use when developing or auditing Forge's Rust CLI, asset processing, Pack validation, Godot delivery, release packaging, or project documentation.
---

# Forge Dev

## Product and source boundaries

The public product is the Rust `forge` CLI. The workspace contains `packages/cli`,
`packages/core`, `packages/pack`, and `packages/providers`. The retired desktop
application is available in Git history. Use existing package and script
patterns. Do not copy external project source into Forge.

Read the [CLI protocol](../../../docs/automation/forge-cli.md) for requests, Jobs,
Packs, and Godot installation, and [CONTRIBUTING](../../../CONTRIBUTING.md) for
source builds. For asset preparation without changing Forge, use
[forge-use](../forge-use/SKILL.md) and the [local asset guide](../../../docs/automation/codex-local-assets.md).
Record development findings in `docs/qa/` and retain appropriate evidence under
`docs/qa/artifacts/`; keep private source media and generated Job stores out of Git.

## Establish the actual build

Inspect the checkout, dirty state, Cargo features, and executable before claiming a
capability works. `target/debug/forge` can be stale, even when the checkout is current.
Build the selected checkout, use its absolute binary path, and record the source
commit, build features, and binary SHA-256 alongside QA evidence. `--version` alone
does not distinguish a development binary from a released binary with the same version.

v0.3.0's doctor output carries compiled build identity and stable capability IDs.
When adding a local contract, update capabilities alongside its implementation.
Release verification uses `scripts/verify-cli-build.py` to match commit, version,
default features and packaged `BUILD_INFO.json`; source archives report unknown
Git values as `null`.

From the selected repository root, a default-feature verification starts with:

```bash
cargo build --locked -p forge-cli --no-default-features
git rev-parse HEAD
git status --short
./target/debug/forge --version
./target/debug/forge plan --help
./target/debug/forge doctor --json
shasum -a 256 ./target/debug/forge
```

Resolve the actual target directory if `CARGO_TARGET_DIR` is configured. Use default
features when assessing release behavior; enable optional features only for the
requested development workflow. `packages/cli/Cargo.toml` declares `default = []`.

Capability checkpoint, checked 2026-09-08; recheck Git and command help when using it:

- v0.3.0's default CLI includes local static PNG intake, static rendering/ground
  anchors, and alpha-bound controls. This is the stable PNG → static Pack → Godot
  path; no optional source feature is required.
- The earlier v0.2.1 supports existing animation preparation and Godot delivery,
  but predates `plan prepare-static` and the new local animation request fields.
  Do not infer development-build availability from the version string or silently
  change another project's pinned CLI. Verify consumer contracts before updating
  its lock for future imports; preserve existing asset receipts and source history.
  Actual re-imports produce new receipts.
- v0.3.0 also includes local `preserve_source`, rendering/timing and
  whole-sheet preprocessing, with repair preserving the requested coordinates.
  Use `scripts/test-local-animation-delivery.py` with Pillow and Godot for the
  synthetic CLI/Pack/native-resource contract. Animation remains experimental.
- Subject/Character V2, world assets, and game-art manifest commands are optional
  source-build capabilities. Read feature gates and the selected command's help.

## Embedded guide and optional product skill

From v0.3.1 the default CLI embeds `.agents/skills/forge-use/`. Maintain this as
one self-contained source, with runtime links inside the bundle. The explicit
file list lives in `packages/cli/src/skill.rs`; update it when adding resources.
From v0.3.2 `guide [RESOURCE] [--json]` reads that same content without skill
installation. Keep relative links usable in an installed bundle and provide
`guide` commands for references/examples read from an executable. Do not create
a second guide source. Plain output must preserve the original resource bytes;
JSON must identify the selected file and the same bundle/build as `skill show`.
Keep `embedded_usage_guide` in the build capabilities alongside the contract.
Guide reads are offline and read-only, without Provider credentials, Plans,
Jobs or Codex configuration. A CLI upgrade changes its embedded guide but does
not update separately installed skill files or a consumer's toolchain lock.

The build watches the skill directory, and skill-only edits must trigger CI.
Run `scripts/test-cli-skill.py --forge /absolute/path/to/forge` for isolated
guide reads, installation, content identity, update backups and modification/symlink protection.
Use the public launcher for packaged checks. Never install into a real consumer
or personal skill directory as a test. Codex discovery, Godot availability and
image generation remain separate from successful file installation. Codex can
read the guide without discovering a skill; installing only the CLI does not
register the embedded bundle with Codex.

## Local processing and generation

Codex's built-in image generation can supply source PNGs to Forge. It is a separate
generation step, not an installed Forge Provider. Local import preserves that
distinction in provenance; do not invent Provider or Style Lock evidence.

On a build with `plan prepare-static`, import transparent PNGs as `icon_set` or
`prop_set` through a single-use plan, execute the Job, inspect its contact sheet and
report, validate the Pack, and use a separate Godot install plan. This path needs no
Provider or Style Lock and reports zero Provider requests. It currently has no asset
catalog registration or targeted retry; prepare a new local request for revisions.
Its `game_ready` verdict covers structural checks, not visual or style approval.

Local static normalization preserves source copies and alpha without chroma-key
matting. `foregroundAlphaThreshold` selects subject bounds; `edgePaddingPx` retains
nearby soft edges. They default to 1/0; adjust them only after inspecting the
source and normalized output. Props use a ground
origin and icons a center origin. Godot UI consumers of icon textures must apply the
rendering contract themselves; the installed prop scenes carry it directly.

`generate`, `style create`, and generation-stage retries can make Provider requests.
Use the fixture Provider for offline tests, and real Providers only within the
requested scope. Read plan estimates and `job report` usage evidence rather than
inferring cost from the command's name. `doctor` and `provider list` deliberately
avoid credential reads; `provider doctor` performs an explicit authentication check.
A local-path pass does not establish real-Provider generation quality.

## Focused verification

Select checks for the changed behavior. Use isolated `FORGE_JOB_STORE` and
`FORGE_PLAN_STORE` directories for manual
fixtures so QA does not use the normal asset history.

```bash
cargo fmt --all -- --check
cargo test -p core --test <relevant_integration_test>
cargo test -p pack
```

For a checkout containing local static delivery changes:

```bash
cargo test -p core --test prepare_static_tests --test static_delivery_tests
cargo test -p core --test static_delivery_tests -- --ignored --nocapture
python3 scripts/test-local-static-cli.py --forge /absolute/path/to/forge --godot /absolute/path/to/Godot
```

The ignored test loads saved resources in real Godot 4.6.x, including legacy static
Packs. For a nonstandard Godot location, set both `FORGE_GODOT_PATH` (Forge installer)
and `GODOT_BIN` (the Rust test's final scene check). The Python script's `--godot`
selects both its install Jobs and final scene check.
Keep evidence clear about fixture resource validation versus reviewed artwork.

For broader CLI integration, use `FORGE_BINARY=/absolute/path/to/forge bash
scripts/test-cli-product.sh`; for a release, follow `.github/workflows/release-cli.yml`
and the existing packaging/verification scripts. v0.3.0 retains unsigned,
unnotarized distribution. Local development signing does not change release status.

## Verify installed release artifacts

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
