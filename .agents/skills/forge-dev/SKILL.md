---
name: forge-dev
description: Use when developing or auditing Forge's Rust CLI, asset processing, Pack validation, Godot delivery, release packaging, or project documentation. Includes conditional guidance for the retained desktop app.
---

# Forge Dev

## Product and source boundaries

The public product is the Rust `forge` CLI. The workspace contains `packages/cli`,
`packages/core`, `packages/pack`, and `packages/providers`; retained desktop and MCP
code are outside the default build and release. Use existing package and script
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

- Published v0.2.1 supports existing animation preparation and Godot delivery, but
  predates `plan prepare-static`.
- The current default source build includes local static PNG intake, static
  rendering/ground anchors, and alpha-bound controls, originally verified at
  `c1f448082c34f531738126e38e41f5a9d66ca1a7`. The published v0.2.1 predates these
  capabilities. Do not infer availability from the version string or silently
  change another project's pinned CLI.
- The current source also includes local `preserve_source`, rendering/timing and
  whole-sheet preprocessing, with repair preserving the requested coordinates.
  Use `scripts/test-local-animation-delivery.py` with Pillow and Godot for the
  synthetic CLI/Pack/native-resource contract. Animation remains experimental.
- Subject/Character V2, world assets, and game-art manifest commands are optional
  source-build capabilities. Read feature gates and the selected command's help.

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
nearby soft edges. They default to 1/0. The Sword evidence used 16/16 for distant alpha
residue; inspect the actual source before reusing those values. Props use a ground
origin and icons a center origin. Godot UI consumers of icon textures must apply the
rendering contract themselves; the installed prop scenes carry it directly.

`generate`, `style create`, and generation-stage retries can make Provider requests.
Use the fixture Provider for offline tests, and real Providers only within the
requested scope. Read plan estimates and `job report` usage evidence rather than
inferring cost from the command's name. `doctor` and `provider list` deliberately
avoid credential reads; `provider doctor` performs an explicit authentication check.
A local-path pass does not establish real-Provider generation quality.

## Focused verification

Select checks for the changed behavior. CLI/core changes do not require a desktop
build. Use isolated `FORGE_JOB_STORE` and `FORGE_PLAN_STORE` directories for manual
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
and the existing packaging/verification scripts. The published v0.2.1 binaries are
unsigned and not notarized. Local development signing does not change release status.

## Retained desktop work only

When the task changes `apps/mac`, preserve sprite sheet intake as: choose file,
configure `固定网格` or `透明间隔`, then import. Keep automation mechanics out of the
user-facing UI. Build and test the workspace app, avoiding a stale installed app
with the same bundle ID:

```bash
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
npm --workspace apps/mac run tauri -- build --debug --bundles app
```

Launch `<checkout>/target/debug/bundle/macos/Game Sprite Forge.app`, accounting for
any configured target directory. Close stale `/Applications/Game Sprite Forge.app`
windows or explicitly target the workspace bundle. Record the exact app path, UI
driver, fixture, and observed result in the QA note.
