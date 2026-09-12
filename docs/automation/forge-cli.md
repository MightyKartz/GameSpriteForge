# `forge` CLI automation protocol

`forge` is the public Forge product and the source of truth for asset generation,
quality evidence, durable jobs, Pack export, and Godot installation. The repository
builds and distributes the CLI; agents and scripts use its command/JSON protocol.

For **Codex-generated PNGs → local Forge processing → Godot**, read the
[local asset guide](codex-local-assets.md). The v0.3.0 default CLI includes the
stable local PNG → static Pack → Godot workflow. Character animation remains
in development and testing. Codex's built-in image generation is an external
source workflow, not a Forge Provider.

For direct Codex use, v0.3.2 adds `guide`: agents can read the bundled workflow
and examples without installing a skill or checking out Forge. Select the game's
verified absolute executable as `FORGE_BIN` and use it for guide reads and asset
commands alike. Do not replace an existing pin with whichever `forge` is on PATH.

## Output contract

Successfully parsed commands invoked with `--json` write one JSON value to stdout:

```json
{
  "schemaVersion": "1",
  "ok": true,
  "data": {}
}
```

Handled runtime errors set `ok` to `false`, include stable `code` and `message`
fields, and exit non-zero. CLI parser failures (such as an unavailable subcommand
or an invalid flag) instead print usage diagnostics to stderr and may have no JSON
stdout. Check the exit status before decoding. Credentials and authorization
responses are not diagnostic output.

### Compiled build identity

Starting with v0.3.0, `doctor --json` includes `data.build` with `gitCommit`,
`dirty`, `target`, `profile` and `features`, plus `data.capabilities` containing
stable IDs. `gitCommit`/`dirty` are `null` when the build has no trustworthy Git
identity (for example, a source archive). These describe the executable at compile
time, not the current checkout around it. `features` lists enabled named Cargo
features, omitting the empty `default` marker. Runtime Godot/FFmpeg availability
remains in the existing doctor fields.

The default v0.3.0 capabilities are `local_static_import`,
`local_animation_import`, `preserve_source_coordinates`, `local_animation_timing`,
`whole_sheet_source_transform`, `pack_validation` and `godot_install`. A capability
does not certify source artwork quality. Consumers should lock a tested binary
hash and check the capabilities their requests require.

## Product commands

```text
forge doctor --json
forge guide [RESOURCE] [--json]
forge skill show --json
forge skill install --project /absolute/game --json
forge skill check --project /absolute/game --json
forge provider list --json
forge provider login --provider xai --method api-key
forge provider login --provider xai --method oauth

forge project init --path /absolute/assets --name "My Game"
forge style create --project /absolute/assets --spec style.json --wait --json
forge style inspect --project /absolute/assets --json

forge generate character --project /absolute/assets --spec ranger.json [--wait] --json
forge generate icon-set --project /absolute/assets --spec icons.json [--wait] --json
forge generate prop-set --project /absolute/assets --spec props.json [--wait] --json

forge job get --id JOB --json
forge job report --id JOB --json
forge job cancel --id JOB --json
forge job retry --id JOB --item ITEM_OR_ANIMATION \
  --stage auto|still|video|loop|matting [--wait] --json
forge job review --id JOB --accept --reason "visual review" --json

forge pack validate --path /absolute/Pack.gsfpack --json
forge asset verify-images --root /absolute/project --lock tools/asset-lock.json --scan game --json
forge source inspect --path /absolute/source.png --json
forge receipt export --job JOB [--install-job INSTALL_JOB] --out receipt.json --json
forge receipt verify --path receipt.json [--pack /absolute/retained.gsfpack] --json
forge godot plan-install --pack /absolute/Pack.gsfpack --project /absolute/game --json
forge godot verify-install --project /absolute/game --asset-key stable_id --json
forge plan execute --token TOKEN [--wait] --json
```

`generate` prepares and immediately consumes the same fingerprinted single-use plan
used by low-level automation. Without `--wait` it returns a Job ID and a detached
worker continues the job.

### Embedded usage guide

v0.3.2 adds the compiled capability `embedded_usage_guide`. `guide [RESOURCE]`
reads one resource from the same self-contained bundle used by `skill show` and
`skill install`; there is one maintained content source. Omitting `RESOURCE`
selects `overview` (`SKILL.md`). The accepted topics and exact bundle-relative paths are:

| Topic | Path |
| --- | --- |
| `overview` | `SKILL.md` |
| `static` | `references/local-static.md` |
| `provider` | `references/provider.md` |
| `animation` | `references/animation.md` |
| `delivery` | `references/delivery.md` (new source builds) |
| `static-example` | `examples/local-static.json` |
| `provider-example` | `examples/provider-icons.json` |

Plain output is the selected file's exact UTF-8 content without a heading or
wrapper. `--json` returns the standard envelope with these `data` fields:

- `name`, `schemaVersion`, `cliVersion`, `build`, `contentHash`: the same bundle
  and compiled CLI identity as `skill show --json`.
- `path`, `sha256`, `content`: the selected resource and its original content.
- `resources`: all available resources, each with `topic`, `path`, `mediaType`.

Only the listed resources are accepted; unknown topics and other paths return
`guide_resource_not_found`. This command does not read arbitrary filesystem
paths. It is offline and read-only, without Provider credential
access, Plans, Jobs, skill installation or Codex configuration changes. Plain
JSON examples can be saved with `"$FORGE_BIN" guide static-example > request.json`.
Choose a new output path and check command success before consuming it: a failed
command can leave an empty redirected file. Adapt the example before planning.

Reading a guide from an upgraded CLI returns that version's embedded content.
Separately installed skill files still require explicit updates. Keep an older
game pin when using older contracts: v0.3.1 exposes its complete bundle with
`skill show --json`; v0.3.0 has neither command and needs matching file
documentation. Check capabilities and help before invoking a newer command.

### Optional bundled Codex skill

v0.3.1 adds `bundled_forge_use_skill` to the compiled capability list and includes
the self-contained `forge-use` bundle. `skill install` and `skill check` require
exactly one scope: `--project PATH` or `--user`. They do not use Plans, Jobs or
Provider credentials. `skill show` displays the bundled entrypoint; with `--json`
its data includes `name`, `schemaVersion`, `cliVersion`, `build`, `contentHash`
and `files` (`path`, `sha256`, `content`).

`skill check --json` returns `data.status` as `missing`, `current`, `outdated`,
`modified` or `unmanaged`, along with the target and issues. A completed inspection
has `ok: true` even when the skill is missing or needs updating. Installation
returns `action` (`installed`, `unchanged`, `updated`); updates report a
`backupPath`. Runtime failures use the standard error envelope. See
[Codex usage and optional setup](codex-skill.md) for the content identity, update
and conflict rules. Installing the CLI alone does not register a Codex skill;
`guide` provides the default workflow without skill installation.

## Plans and jobs

### Source-build delivery additions

The current source build adds capabilities `reviewed_source_hashes`, `local_request_relative_paths`,
`source_png_inspection`, `effect_quality_profile`, `preview_timing_diagnostics`,
`delivery_receipts`, `godot_install_verification`, `transactional_godot_install`,
`godot_import_cache_integrity` and `project_image_contract_verification`.
These are not a claim about the existing v0.3.2 release archive. Check the actual
binary/build hash before selecting them. Read the maintained
[delivery guide](../../.agents/skills/forge-use/references/delivery.md) or
`forge guide delivery` for commands, source locks, receipt relocation/trust and
installation audit limits; [animation guidance](../../.agents/skills/forge-use/references/animation.md)
covers opt-in effect semantics and GIF quantization.

`asset verify-images` checks an existing project's declared PNG hashes,
dimensions, alpha constraints and exact scanned file set without writing a lock,
using Jobs or starting Godot. The report is limited to image contracts and lists
unverified metadata; it does not establish installation or export provenance.
See [project image contracts](image-contracts.md) for the schema, explicit scan
scope and mismatch results.

Local preparation paths and `sourceLocks` resolve relative to `--request`'s file;
stdin paths resolve against cwd. This corrects older animation commands that
used cwd even with a request file. Absolute paths keep their existing meaning.
`sourceLocks` is optional and backward compatible; when nonempty it binds the
complete local source closure before plan creation and is rechecked on execution.

Receipt v1 is described by [delivery-receipt.schema.json](../../schemas/delivery-receipt.schema.json).
Its producer evidence is captured by the executing CLI, separately from the
exporter's identity. Legacy Jobs report absent producer evidence explicitly.
Verifying a receipt requires the retained Pack and, when included, installation;
it requires neither original source paths nor the Job store. It verifies source
hash records, not the continued existence of the original source files. Record
the receipt SHA separately and pass `--expected-sha256` to bind that evidence.
`verified:true` confirms the evidence checks; read `visualReview` separately,
which can still be `pending`, `rejected` or `not_recorded`.

Installation now performs dependency checks before changing its target, uses a
project lock and restores the previous target, registry and catalog link on
execution errors or cancellation, including the target textures' imported cache
bytes. A structured native verification report is a Job artifact. Snapshot v2 in
`.forge-install.json` records stable installed files and separate import/cache
evidence, hash-bound to `.forge/assets.json`. Missing recorded caches or import
sidecars are reported as `not_materialized`; an empty cache baseline is
`not_recorded`. Both have `materialized:false` and do not establish readiness
for native loading. Changed existing caches are rejected. The
read-only audit does not reload or render the resources. Serialize non-Forge
Godot processes sharing a project's cache separately.

### Local animation contracts

Use `plan prepare-asset --request ... --json` for one action and
`plan prepare-character --request ... --json` for multiple actions. v0.3.0
accepts `normalize.mode: preserve_source`, request-level `rendering`,
and per-action `frameDurationsMs`, plus explicit whole-sheet padding/offset in
fixed-grid inputs. These local request forms are newer than v0.2.1; character
animation remains experimental. They preserve drawing coordinates and timing through Pack/Godot
delivery; they do not establish visual quality. Automatic repair leaves preserved
coordinates unchanged and returns canvas/anchor issues for manual review. See the
[local asset guide](codex-local-assets.md#preserve-intentional-animation-coordinates-experimental)
for the request shapes and limits.

With `effect_quality_profile`, local PNG, sheet or video preparation can use
`"quality":{"profile":"effect","requireGameReady":true,"allowTransparentTail":true}`
for intentional disappearance at the end. `allowTransparentTail` belongs inside
`quality` and requires every animation in that request to be non-looping. Omit it
or set it to `false` otherwise. All-empty animations, interior empty frames and
tails made empty only by normalization remain invalid. The effect profile does
not grant visual approval; Pack-copy inputs and generated character workflows
do not accept this override.

### Local transparent PNG sets

`plan prepare-static` prepares a local `icon_set` or `prop_set` without a Provider
or Style Lock. It uses the same single-use plan, durable Job, Pack validation, and
Godot installation flow as other CLI operations:

```bash
forge plan prepare-static --request /absolute/static-items.json --json
forge plan execute --token TOKEN --wait --json
```

```json
{
  "schemaVersion": "1",
  "kind": "prop_set",
  "id": "forest_props",
  "name": "Forest props",
  "license": "private",
  "sampling": "linear",
  "canvasSize": 128,
  "foregroundAlphaThreshold": 1,
  "edgePaddingPx": 0,
  "items": [
    { "id": "lantern", "name": "Stone lantern", "path": "sources/lantern.png" }
  ]
}
```

Paths are relative to the request file, or the current directory with `--stdin`.
Set `sampling` explicitly to `linear` or `nearest`. Canvas size is 64, 128, 256,
or 512; a set contains 1–64 items with unique stable IDs. Input must be PNG with
transparent background and visible foreground, at most 4096 px per dimension and
32 MiB per file. Local import never applies chroma-key matting: it preserves the
source alpha, fits the foreground to 82% of the canvas, centers icons, and places
props on the ground line described below. Nearest uses nearest-neighbor resizing;
linear uses Lanczos resizing and linear engine filtering.

For images with faint distant alpha residue, `foregroundAlphaThreshold` (1–255,
default 1) chooses the subject bounds. `edgePaddingPx` (0–64, default 0) expands
that crop in source pixels before normalization. Choose these values after
inspecting the source and normalized result. Pixels inside the padded crop retain their
original alpha; the threshold is not an alpha cutoff. The quality report records
both the subject bounds and the padded crop with exclusive right/bottom coordinates.
Padding is included in the normalized extent, so the visible subject may sit a few
output pixels above the prop origin. Omitted fields preserve the original behavior.

The Job retains original PNGs, source and normalized SHA-256 values, recipe/input
fingerprints, and a local quality report. The static Pack records `import_frames`
provenance and zero Provider requests; its explicit `assetType` stays `icon_set`
or `prop_set`, and Godot receives textures or Sprite2D scenes. `game_ready` here
means structural PNG/canvas checks passed; style consistency is not evaluated.
The initial local importer does not register an asset-project catalog or support
targeted retries: prepare a new request to revise a local set, and install without
`--catalog-project`. Normal Godot install ownership, registry, and rollback apply.

This entry point is included in the v0.3.0 default CLI; the earlier v0.2.1 binary
predates it. Use the [local asset guide](codex-local-assets.md) for installation
and an end-to-end request example.

A plan validates and fingerprints local inputs without generating media or changing a
Godot project. Its token expires after 15 minutes, is consumed once, and refuses to
execute if an input changes. Execution creates an immutable recipe and a durable
JobStore record. Cancellation is cooperative between provider and processing steps.

`job retry --item` creates a new source-linked Job. A static retry calls the provider
only for that icon or prop and copies accepted siblings after checking their source
directory and SHA-256 evidence. A Character retry behaves the same way for
`idle`, `walk_up`, `walk_right`, or `walk_down`. Character stages have explicit
invalidation boundaries:

- `still` regenerates the direction still and invalidates video and all local stages;
- `video` reuses the still and prefers xAI video editing, with same-still
  image-to-video as the recorded fallback;
- `loop` reuses the video and reruns candidate extraction through Pack export with
  zero Provider requests;
- `matting` reuses the video and reruns background processing and downstream stages;
- `auto` selects the earliest stage implied by persisted consistency and loop evidence.

Static assets keep their existing `--item` retry semantics and reject Character-only
stages. Every retry creates a new source-linked Job; it never mutates its parent.

`job report` embeds Provider attempt/usage evidence and Loop Selection reports in the
JSON response. It includes `providerRequestOccurred` and the selected source frame
range, so an agent can distinguish free local reprocessing from a paid generation.

`job review` may promote only an `awaiting_review` gray-band result. It cannot bypass
missing/corrupt media, invalid Alpha, crop, canvas, frame, or other hard gates.

## Project, style, and asset contracts

- `ForgeProjectV1` locks the Provider Profile, output directory, and current immutable
  Style revision.
- `StyleSpecV1` accepts zero to three references and canvas/perspective/lighting intent.
- `StyleLockV1` records SHA-256 references, Provider/model identity, style board,
  palette, edge density, foreground scale, and background baseline.
- `AssetSpecV1` covers `character`, `icon_set`, and `prop_set`. Paths are resolved
  relative to the spec before planning.
- `ConsistencyReportV1` records each direction/item attempt, metrics, threshold profile,
  verdict, and review reasons.
- `.gsfpack` V2 adds `assetType`, static `items`, consistency evidence, and Style
  provenance. The reader remains compatible with V1 Character Packs.

Provider-generated jobs lock one Provider, Profile, model selection, and Style
revision. Local preparation and Godot installation use their input/recipe
fingerprints and do not require a Provider or generated Style Lock. Forge rejects
missing generation capabilities instead of silently switching Provider or degrading
to unrelated text-to-image calls.

## Consistency profile

`consistency@1.2.0` evaluates perceptually matched palette overlap, longest-extent
foreground scale, reference-normalized edge density, major-subject count, anchor
drift, and optional foreground identity similarity. Character direction palettes and
edges are compared to the canonical character reference; Style Lock still governs
generation, but a mixed character/icon/prop style board is not treated as the
character's literal color palette. It has three
outcomes: `game_ready`, `awaiting_review`, and `regenerate`/`blocked`. Each generated
direction or item gets at most two automatic attempts before the job pauses.

Characters derive one canonical reference, four direction stills, and four image-to-
video clips. Godot flips `walk_right` for left-facing playback. Icon and prop sets first
establish an anchor item, then derive the remaining items from the Style and anchor.

`loop@2.0.0` samples the complete generated video at no more than 12 FPS and 96
candidates. After matting and provisional alignment it searches a closed interval,
uses the boundary frame only as closure proof, and exports evenly sampled frames from
`[start, end)` without duplicating the first frame. Its fixed score combines Mask IoU
(30%), soft palette overlap (20%), edge overlap (20%), anchor closure (15%), and wrap
transition continuity (15%). Walk motion energy must be at least 1%; idle motion must
be at least 0.2%. `regenerate` and `blocked` loop results cannot be manually promoted.

Generated Character Jobs also write `workflow-stage-manifest.json`. Each stage records
its implementation version, input/output SHA-256 values, invalidated descendants, and
whether it made a Provider request. `.gsfpack` V2 carries additive
`quality/loops.json` evidence; older readers remain compatible.

## xAI authentication

API-key login reads a hidden TTY value and stores it in the operating system credential
store; an API key is never accepted as a CLI argument. Device Code OAuth is Preview.
Both modes feed the same direct xAI REST Provider; Forge has no Grok Build CLI
dependency. `fixture` implements the same contract offline for deterministic tests.

Forge stores a separate non-secret auth profile containing only the selected method and
storage backend, so generation reads one credential entry instead of probing API Key
and OAuth entries on every process launch. Production defaults to Keychain. Developers
whose rebuilt ad-hoc binaries would repeatedly trigger macOS approval can explicitly
use owner-only OAuth file storage:

```bash
forge provider login --provider xai --method oauth --credential-store file
```

The v0.3.0 binaries remain **unsigned and not notarized**. Local development
signing can use the fixed identifier `dev.gamespriteforge.cli`; that configuration
does not establish release signing. See [the release notes](../releases/v0.3.0.md)
and [development signing instructions](../../CONTRIBUTING.md).

## Godot 4.6.x delivery

Godot installation is a separate single-use plan. Forge copies PNG textures first,
runs a headless import, and then creates resources with `ResourceLoader`. It rejects
text `.tres`/`.tscn` files at or above 1 MiB and any embedded Image
`PackedByteArray`.

- Characters receive external atlas textures, `SpriteFrames`, an
  `AnimatedSprite2D` scene, and directional playback metadata.
- Icon sets receive one external PNG per item and an item-to-`res://` mapping.
- Prop sets receive one external PNG and one `Sprite2D` scene per item.

New static Packs preserve Style `sampling` as `rendering.textureFilter` (`nearest`
or `linear`) in the manifest, Pack source metadata, and Godot helper. Props use the
normalized ground line `(canvas / 2, canvas - canvas / 16)` as their local origin;
icons use the canvas center. `forge_usage.json` includes this rendering/anchor
metadata and a `texturePaths` map from item ID to installed `res://` texture for UI
consumers. Static Packs without the rendering contract retain their former centered
geometry and inherited Godot filtering.

Every install writes `forge_usage.json`, registers atomically in
`.forge/assets.json`, replaces only Forge-owned targets, and restores the previous
installation after failure. Character usage includes the loop profile, selected
boundaries/frame indices, and the recorded Provider retry method.

## Static consistency replay

```bash
forge style create --project /absolute/assets --spec /absolute/style.json --wait --json
forge job retry --id <icon-or-prop-job> --stage consistency --wait --json
```

`style-baseline@2.3.0` removes the dominant border-connected background before extracting
the Style palette and records a new immutable revision. If the prior revision was created
from the same spec, Provider, profile, and references, Forge copies its SHA-256-verified
style board into the new revision instead of generating another image. `--stage consistency`
copies the source Job's normalized static images, recalculates `consistency@1.3.0`, and makes
zero Provider requests. An optional `--item` limits the explicit target while all reused
items are also rechecked against the current baseline.

## Historical character-generation acceptance

The `v0.2.0-cli.1` generation work used a three-run xAI Style → Character → Pack →
Godot acceptance target, including four `game_ready` actions and reconstructable
provenance/usage. This is historical generation criteria, not an unfulfilled gate
on the already published v0.2.1. See the [v0.2.1 scope](../releases/v0.2.1.md)
and [release verification](../qa/forge-cli-v0.2.1-release-2026-09-07.md).
Offline fixtures and local-image checks do not establish real-model art quality.

## Development store overrides

```bash
export FORGE_JOB_STORE="/absolute/test/jobs"
export FORGE_PLAN_STORE="/absolute/test/plans"
```
