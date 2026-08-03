# `forge` CLI automation protocol

`forge` is the public Forge product and the source of truth for asset generation,
quality evidence, durable jobs, Pack export, and Godot installation. Desktop and MCP
clients are not part of the CLI release.

## Output contract

Every command invoked with `--json` writes exactly one JSON value to stdout:

```json
{
  "schemaVersion": "1",
  "ok": true,
  "data": {}
}
```

Errors set `ok` to `false`, include stable `code` and `message` fields, and exit
non-zero. Diagnostics go to stderr. Credentials and authorization responses never
appear in either stream.

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

All jobs lock one Provider, Profile, model selection, and Style revision. Forge rejects
missing capabilities instead of silently switching Provider or degrading to unrelated
text-to-image calls.

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

Official macOS binaries use the fixed code-signing identifier
`dev.gamespriteforge.cli` and one Developer ID team across releases.

## Godot 4.6.x delivery

Godot installation is a separate single-use plan. Forge copies PNG textures first,
runs a headless import, and then creates resources with `ResourceLoader`. It rejects
text `.tres`/`.tscn` files at or above 1 MiB and any embedded Image
`PackedByteArray`.

- Characters receive external atlas textures, `SpriteFrames`, an
  `AnimatedSprite2D` scene, and directional playback metadata.
- Icon sets receive one external PNG per item and an item-to-`res://` mapping.
- Prop sets receive one external PNG and one `Sprite2D` scene per item.

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

## Character release gate

`v0.2.0-cli.1` must not be tagged until three clean xAI Style → Character → Pack →
Godot runs finish without review, all four actions are `game_ready`, provenance and
usage can be reconstructed, and credential/temporary-URL scans are clean. The offline
fixture and CI contracts are necessary but do not replace this real-model gate.

## Development store overrides

```bash
export FORGE_JOB_STORE="/absolute/test/jobs"
export FORGE_PLAN_STORE="/absolute/test/plans"
```
