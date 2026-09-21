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

Follow [product priorities](../../../PRODUCT.md#current-investment-priorities)
for new scope. Prioritize repeated local processing, native delivery and recovery.
Before adding a command or expanding an optional workflow, identify the recurring
consumer task and check whether a guide or reusable script already solves it.
Preserve compatibility; measure adoption and maintenance costs with the
[comparison protocol](../../../docs/qa/asset-delivery-comparison.md).

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

The doctor output carries compiled build identity and stable capability IDs.
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

Use `doctor --json` capabilities and command help from the selected executable
instead of inferring support from a historical version checkpoint. Subject/Character
V2, world assets and game-art manifest commands require optional source features.
Preserve consumer pins and receipts unless the task includes a verified upgrade.

## Embedded guide and optional product skill

The default CLI embeds `.agents/skills/forge-use/`. Maintain this as
one self-contained source, with runtime links inside the bundle. The explicit
file list lives in `packages/cli/src/skill.rs`; update it when adding resources.
`guide [RESOURCE] [--json]` reads that same content without skill
installation. Keep relative links usable in an installed bundle and provide
`guide` commands for references/examples read from an executable. Do not create
a second guide source. Plain output must preserve the original resource bytes;
JSON must identify the selected file and the same bundle/build as `skill show`.
Keep `embedded_usage_guide` in the build capabilities alongside the contract.
Guide reads are offline and read-only, without Provider credentials, Plans,
Jobs or Codex configuration. A CLI upgrade changes its embedded guide but does
not update separately installed skill files or a consumer's toolchain lock.

The build watches `forge-use`, and edits to that bundle must trigger CI.
`forge-dev` is developer documentation and is not embedded in the executable.
Run `scripts/test-cli-skill.py --forge /absolute/path/to/forge` for isolated
guide reads, installation, content identity, update backups and modification/symlink protection.
Use the public launcher for packaged checks. Never install into a real consumer
or personal skill directory as a test. Codex discovery, Godot availability and
image generation remain separate from successful file installation. Codex can
read the guide without discovering a skill; installing only the CLI does not
register the embedded bundle with Codex.

## Delivery and catalog contracts

Keep new execution provenance separate from legacy Job history and the current
receipt exporter identity. Portable receipts embed JSON report bytes and full
Pack inventories; verify them after relocating the Job store. Never fabricate
historical evidence when importing or migrating existing assets.

Read the [asset library contract](../../../docs/automation/project-asset-library.md)
when changing catalog behavior. Preserve immutable content-addressed objects,
atomic head replacement and inventory rechecks under the catalog lock. Migration
requires an explicit preview/apply with the expected digest; reads do not migrate.
New output does not automatically select itself for delivery or update consumer
locks. Keep machine-local roots separate from portable metadata, and remember
that metadata alone does not back up media. Versioned delivery retains the
whole-Pack installation and rollback boundary.

Godot installation success requires explicit completion evidence and saved
resource checks, not only process status. Ordinary warnings alone are not failure.
Serialize imports/exports sharing the same Godot cache; Forge's project install
lock does not lock unrelated editor processes. Keep structural validation, native
load, visual review and device testing as distinct evidence.

## Local processing and generation

Codex's built-in image generation can supply source PNGs to Forge. It is a separate
generation step, not an installed Forge Provider. Local import preserves that
distinction in provenance; do not invent Provider or Style Lock evidence.

On a build with `plan prepare-static`, import transparent PNGs as `icon_set` or
`prop_set` through a single-use plan, execute the Job, inspect its contact sheet and
report, validate the Pack, and use a separate Godot install plan. This path needs no
Provider or Style Lock and reports zero Provider requests. Builds with
`project_asset_output_registration` support explicit `assetProject` binding for
catalog registration. It has no targeted retry; prepare a new local request for revisions.
Its `game_ready` verdict covers structural checks, not visual or style approval.

Local static normalization preserves source copies and alpha without chroma-key
matting. `foregroundAlphaThreshold` selects subject bounds; `edgePaddingPx` retains
nearby soft edges. They default to 1/0; adjust them only after inspecting the
source and normalized output. Props use a ground
origin and icons a center origin. Godot UI consumers of icon textures must apply the
rendering contract themselves; the installed prop scenes carry it directly.

For existing animation frames, preserve the requested shared coordinates, anchors
and timing through whole-sheet preprocessing and repair when using
`preserve_source`. Animation preparation remains experimental; fixture delivery
checks do not establish production animation quality.

`generate`, `style create`, and generation-stage retries can make Provider requests.
Use the fixture Provider for offline tests, and real Providers only within the
requested scope. Read plan estimates and `job report` usage evidence rather than
inferring cost from the command's name. `doctor` and `provider list` deliberately
avoid credential reads; `provider doctor` performs an explicit authentication check.
A local-path pass does not establish real-Provider generation quality.

## Audio boundaries

Local audio uses `audio import` / `plan prepare-audio`, a v4 audio Pack and the
existing Godot install transaction. `audio tools` only observes explicitly
selected source directories; it must not start Python, load models, download
weights or read credentials. Maintain its guide and example in `forge-use`.
Audio metadata describes technical validation and user-asserted origin; it does
not establish listening or license approval. Native delivery uses binary
`AudioStreamWAV` resources and tracks WAV `.sample` caches alongside image caches.
Preserve another project's existing audio and toolchain pins.

## Verification and release

Read [references/verification.md](references/verification.md) for the checks that
match the changed behavior: catalog/review/transfer, static art, animation/layers,
audio, Godot delivery, embedded guides or installed release artifacts. It also
links the native Windows and paired-release workflows. Do not run every suite
for an unrelated documentation edit or claim native coverage from fixture tests.
