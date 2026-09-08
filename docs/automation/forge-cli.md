# `forge` CLI automation protocol

`forge` is the public Forge product and the source of truth for asset generation,
quality evidence, durable jobs, Pack export, and Godot installation. Desktop and MCP
clients are not part of the CLI release.

For **Codex-generated PNGs → local Forge processing → Godot**, read the
[local asset guide](codex-local-assets.md). It distinguishes published v0.2.1
capabilities from the static/animation development builds used by Sword. Codex's
built-in image generation is an external source workflow, not a Forge Provider.

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
forge godot plan-install --pack /absolute/Pack.gsfpack --project /absolute/game --json
forge plan execute --token TOKEN [--wait] --json
```

`generate` prepares and immediately consumes the same fingerprinted single-use plan
used by low-level automation. Without `--wait` it returns a Job ID and a detached
worker continues the job.

## Plans and jobs

### Local animation contracts

Use `plan prepare-asset --request ... --json` for one action and
`plan prepare-character --request ... --json` for multiple actions. The current
source build accepts `normalize.mode: preserve_source`, request-level `rendering`,
and per-action `frameDurationsMs`, plus explicit whole-sheet padding/offset in
fixed-grid inputs. These local request forms are newer than published v0.2.1.
They preserve intentional drawing coordinates and timing through Pack/Godot
delivery; they do not establish visual quality. Automatic repair leaves preserved
coordinates unchanged and returns canvas/anchor issues for manual review. See the
[local asset guide](codex-local-assets.md#preserve-intentional-animation-coordinates-development-build)
for the request shapes and limits.

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
  "id": "sword-props",
  "name": "Sword props",
  "license": "private",
  "sampling": "linear",
  "canvasSize": 128,
  "foregroundAlphaThreshold": 16,
  "edgePaddingPx": 16,
  "items": [
    { "id": "jade_blade", "name": "Jade blade", "path": "sources/jade-blade.png" }
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
that crop in source pixels before normalization. Values of 16 and 16 work for the
Sword concept-derived source set. Pixels inside the padded crop retain their
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

This entry point is included in the current default source build; the published
v0.2.1 binary predates it. Until the new release, use a verified absolute source CLI
path. See the [integration record](../qa/forge-local-assets-integration-2026-09-08.md).

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

The published v0.2.1 binaries are **unsigned and not notarized**. Local development
signing can use the fixed identifier `dev.gamespriteforge.cli`; that configuration
does not establish release signing. See [the release notes](../releases/v0.2.1.md)
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
