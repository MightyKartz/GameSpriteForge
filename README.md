# Forge CLI

[中文 README](README.zh-CN.md)

Forge is an open-source, agent-first command-line pipeline for generating
consistent 2D game assets and installing them into Godot 4.6.x projects.
Codex, Claude, scripts, and CI can call the same stable JSON protocol.

The current macOS Apple Silicon release supports:

- immutable project Style Locks;
- consistent top-down Character Packs with `idle`, `walk_up`, `walk_right`, and
  `walk_down` animations;
- consistent icon sets and prop sets derived from one style board and anchor;
- direct xAI REST generation through API Key or Preview OAuth, without Grok
  Build CLI;
- deterministic matting, normalization, consistency gates, targeted item
  and stage-level retry, Loop Selection V2, provenance, and `.gsfpack` validation;
- Godot installation with external textures, small native resources, usage
  metadata, ownership checks, and transactional rollback.

## Current source and release scope

The default CLI builds with `default = []`. Character V2/Grid, project builds,
Collection/Portrait assets, project audit, and world generation require explicit
features. The production Provider resolver supports xAI and the offline fixture;
PixelLab is currently an offline loopback experiment.

Start with the installation and default workflows below. The bilingual
[workflow and release boundaries](docs/architecture/forge-workflow-boundaries.md)
map optional features, verified examples, and remaining release gates. The
[implementation status](docs/qa/forge-complete-visual-implementation-status.md)
tracks the roadmap. Later V18 acceptance covers one reviewed `walk_right`
delivery and does not establish a general multi-action CLI release.

## Install

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

Open a new terminal and verify the installation:

```bash
forge doctor --json
```

Forge installs `forge`, `ffmpeg`, and `ffprobe` into a versioned user directory and
exposes only `forge` on `PATH`. The installer verifies both the release archive and
its per-file SHA-256 manifest before switching versions. The first CLI release is
unsigned and not notarized. Godot is not bundled; install Godot 4.6.x before using
engine delivery.

## Five-minute xAI to Godot flow

Authenticate without putting an API key in shell history:

```bash
forge provider login --provider xai --method api-key
```

Create an asset project:

```bash
forge project init --path "$PWD/game-assets" --name "My Game"
```

Create `game-assets/specs/style.json`:

```json
{
  "schemaVersion": "1",
  "prompt": "compact jewel-tone pixel art with dark outlines",
  "referenceImages": [],
  "perspective": "topdown",
  "lighting": "upper_left",
  "outline": "dark",
  "background": "transparent",
  "sampling": "nearest",
  "characterCanvasSize": 256,
  "iconCanvasSize": 128,
  "propCanvasSize": 256
}
```

Lock the style:

```bash
forge style create \
  --project "$PWD/game-assets" \
  --spec "$PWD/game-assets/specs/style.json" \
  --wait --json
```

Create `game-assets/specs/ranger.json`:

```json
{
  "schemaVersion": "1",
  "kind": "character",
  "id": "forest-ranger",
  "name": "Forest Ranger",
  "prompt": "a compact forest ranger with a green hood",
  "license": "private"
}
```

Generate the Character Pack:

```bash
forge generate character \
  --project "$PWD/game-assets" \
  --spec "$PWD/game-assets/specs/ranger.json" \
  --wait --json
```

Inspect the returned `.gsfpack`, then prepare and execute the separate Godot
write plan:

```bash
forge godot plan-install \
  --pack /absolute/path/Forest-Ranger.gsfpack \
  --project /absolute/path/my-godot-game \
  --asset-key forest_ranger --json

forge plan execute --token <returned-token> --wait --json
```

## Icon and prop sets

Icon and prop specs use the same shape:

```json
{
  "schemaVersion": "1",
  "kind": "icon_set",
  "id": "inventory-icons",
  "name": "Inventory Icons",
  "items": [
    { "id": "potion", "name": "Potion", "prompt": "a red healing potion" },
    { "id": "key", "name": "Key", "prompt": "a small brass key" }
  ],
  "license": "private"
}
```

```bash
forge generate icon-set --project "$PWD/game-assets" --spec /absolute/icons.json --json
forge generate prop-set --project "$PWD/game-assets" --spec /absolute/props.json --json
forge job report --id <job-id> --json
forge job retry --id <job-id> --item potion --wait --json

# Re-score existing icon/prop pixels against the current Style Lock without a Provider call.
forge job retry --id <static-job> --stage consistency --wait --json

# Character-only retries: local loop/matting reruns do not call the Provider.
forge job retry --id <character-job> --item walk_right --stage loop --wait --json
forge job report --id <new-job-id> --json
```

Generation defaults to an asynchronous durable job. Add `--wait` for a
synchronous result. All public commands write one JSON envelope to stdout;
diagnostics and interactive authorization stay on stderr/TTY.

Style Locks use the versioned `style-baseline@2.3.0` foreground-aware palette.
Recreating a Style Lock after a baseline upgrade preserves the old immutable revision and
reuses its verified style board when possible, so migration does not require regeneration.

Character generation evaluates the full video, selects a real closed cycle, and
exports only the selected `[start, end)` frames. The matching boundary frame is kept
as quality evidence but is not duplicated in the animation. `job report` exposes the
selected indices, score components, retry method, and whether the retry made a paid
Provider request.

The v0.2 Character release gate completed on 2026-08-03: three consecutive clean real-xAI Character →
Pack → Godot runs passed all four actions without manual review. The evidence and
provider-cost/retry audit are recorded in
[`docs/qa/forge-character-loop-v2-2026-08-03.md`](docs/qa/forge-character-loop-v2-2026-08-03.md).
`v0.2.0-cli.1` is published on GitHub Releases (2026-08-03, unsigned and not
notarized, with SBOM and Artifact Attestation). The final release-operation check —
a clean-account installation from that published release — is not yet recorded
under `docs/qa/`.

## Unreleased Character consistency V2

Build with `consistency-v2` to expose this optional command surface; Grid workflows
also need `grid-generation`. These versions remain separate from the default release.
The history below records individual contracts and their dated acceptance results.

<details>
<summary>Character workflow history and version-specific evidence</summary>

`topdown-video@2.0.0` defines a versioned Character V2 contract. A spec must choose
`topdown-orthographic@2.0.0` or `topdown-three-quarter@1.0.0`; Forge validates the
front/rear/right direction still, body scale, top margin, center, foot baseline,
SubjectLock-relative framing, and crop safety before making the image-to-video
request. It then samples the complete local clip, selects a closed cycle, and
exports external PNG/atlas textures plus native Godot `SpriteFrames`. The legacy
`topdown@1.0.0` route remains readable for Character V1 Jobs. See the
[implementation plan](docs/architecture/forge-topdown-video-v2-plan.md) and
[offline acceptance](docs/qa/forge-topdown-video-v2-offline-2026-08-09.md).

The opt-in build can bound paid keyframe acceptance to one direction before a
complete 32-frame Character run:

```bash
forge generate character --project /absolute/assets --spec character-v2.json \
  --validation-animation walk_right --plan-only --json
```

For `topdown-video@2.0.0`, this mode plans one still edit and one image-to-video
request, with a maximum of four requests across two attempts. It writes local
direction, framing, loop, quality, and playback evidence and never exports or
catalogs a partial Character Pack.

The first repaired contract was `topdown-keyframes@2.2.0`: it never sends the
multi-object Style board as an image reference, uses a transparent compact Pose
guide, and requires an explicit `none` or `staff_like` equipment declaration.
The [offline acceptance report](docs/qa/forge-character-reference-isolation-equipment-offline-2026-08-08.md)
records the fixture, CLI-plan, Godot, and remaining real-model gates.
The subsequent [four-direction real-xAI result](docs/qa/forge-character-keyframes-v22-real-acceptance-2026-08-08.md)
was blocked: the staff-reference leak was fixed, but direction, Alpha background,
identity, and lower-body consistency were not yet release-ready.

The follow-up `topdown-keyframes@2.3.0` keeps 2.2 Jobs readable and addresses the
two propagation defects found by that run. Frame 0 establishes a versioned front,
rear, or right DirectionLock without adding paid requests; every later frame is
checked against that direction-local baseline. Every Provider image is also passed
through `keyframe-background-cleanup@1.3.0` before normalization or reuse as an
image reference. Raw and cleaned SHA-256 values remain separately auditable. The
[V2.3 offline QA report](docs/qa/forge-character-direction-lock-background-cleanup-offline-2026-08-08.md)
covers the fixture/Pack/Godot gate; real xAI promotion still requires a new,
separately authorized acceptance run.

`topdown-keyposes@2.4.0` is the next opt-in repair. It authors four explicit poses
per action at 6 FPS (contact, passing, opposite contact, opposite passing), keeps
one immutable DirectionLock, and removes the two-neighbor AI interpolation that
produced flicker and residual feet. `motion-semantics@1.1.0` blocks packs whose
frames only recolor, lack a real gait, have an invalid contact order, or contain
lower-edge ghosts/extra foot lobes. Existing packs can be checked locally with
`forge pack audit-motion --path <pack> --json`; see the
[offline report](docs/qa/forge-topdown-keyposes-v24-motion-semantics-offline-2026-08-08.md).

The real V2.4 gate showed that the previous accepted frame still dominated the
new Pose guide: `walk_right` remained almost static and the chained reference
propagated drift. `topdown-keyposes@2.5.0` keeps V2.4 reproducible but derives
frames 1–3 independently from the immutable DirectionLock plus a two-color
left/right semantic Pose guide. Downstream motion hard failures now keep a
`failed` lifecycle and can no longer be overwritten by a reviewable consistency
verdict. See the [V2.5 remediation report](docs/qa/forge-topdown-keyposes-v25-isolation-offline-2026-08-08.md).

`topdown-spritesheet@3.0.0` remains an experimental low-cost comparison path:
the locked image Provider creates one complete 2×2 four-phase sheet per action,
then Forge deterministically slices, cleans, aligns and gates it. A full Character
uses four expected/eight maximum image edits; a single-direction probe uses one/two.
Repeated or merely flickering cells fail `motion-semantics@1.1.0`. The real xAI
acceptance showed that identity and framing were strong but locomotion phases
collapsed into near-duplicate poses, so it is not the default movement authoring
workflow. Godot receives
only external textures and native `AnimatedSprite2D`/`SpriteFrames`/`AtlasTexture`
resources—no extension, rig, parts, or ControlNet. See the
[design](docs/architecture/forge-topdown-spritesheet-v3-plan.md) and
[offline acceptance](docs/qa/forge-topdown-spritesheet-v3-offline-2026-08-08.md).

`topdown-frames@4.0.0` is the next experimental image-only path. It first creates
one validated 2×2 DirectionLock sheet in Forge-owned order (front, rear, right,
left), then uses the selected direction image as the sole appearance/camera
anchor for each four-phase action sheet. The Style board is text metadata only,
which prevents props, effects, crops, or camera angles in a style illustration
from leaking into the Character. A full run plans five image edits and caps at
ten. Shared-scale, translation-only foot alignment and `onion-skin@1.0.0` keep
real drift visible to quality gates. `job retry --frame 0-3` replaces only that
cell and reuses the other three PNGs byte-for-byte. The offline fixture, Pack,
CLI, security and Godot 4.6.3 gates pass; real xAI visual acceptance still needs
separate authorization. See the [V4 design](docs/architecture/forge-topdown-frames-v4-plan.md)
and [offline acceptance](docs/qa/forge-topdown-frames-v4-offline-2026-08-09.md).

`topdown-video-locked@5.0.0` is the experimental hybrid successor. It keeps the
successful V4 DirectionLock, removes thin generated floor/baseline marks locally,
and composites each selected direction anchor onto exact chroma green before using
it directly as the first frame of an image-to-video request. No per-direction still
edit and no video-edit fallback is used. Forge samples the full clip at at most 12
FPS, selects eight frames from a closed `[start, boundary)` interval, excludes the
duplicate boundary frame, and applies one collection-wide scale plus translation-only
body/foot anchoring. A new full run plans five media requests (one DirectionLock plus
four videos) and caps at ten; a reused DirectionLock needs four/eight. It remains
experimental until separately authorized real-xAI `walk_up` and `walk_right` probes,
then a full four-action Pack → Godot run, pass. See the
[V5 design](docs/architecture/forge-topdown-video-locked-v5-plan.md).

`topdown-video-cycle@6.0.0` is the experimental V5 correction. It separates a
512px-or-larger DirectionLock generation master from the 256px delivery sprite,
uses 720p image-to-video, requests repeated constant-cadence motion, and only
exports a walk after proving two opposing contacts plus two passing poses in a
700–1200ms source cycle. The eight exported frames share one runtime cadence:
800ms for every walk and 1600ms for idle. Debug GIF, Pack, and Godot timing come
from the same `frameDurationsMs`. See the
[V6 design](docs/architecture/forge-topdown-video-cycle-v6-plan.md); this path
remains experimental until offline frozen-video and separately authorized real
xAI acceptance gates pass.

`topdown-direction-motion@8.0.0` is the experimental correction for direction
lineage and lost source motion. It creates one canonical `front_idle`, derives
three direction idles, derives one motion pose per direction, stops for an
approval bound to all eight image hashes, and only then allows four separately
authorized videos. V8.1 retains native decoded PTS up to 24 FPS / 120 frames,
selects a complete closed source cycle over the full clip, and only then
reduces it deterministically to 8, 10, or 12 original poses. The closure frame
is not duplicated and exact source-time durations flow into preview, Pack, and
Godot. Godot receives four explicit idles and four explicit walks, including
real left-facing assets. See the [V8 design and CLI
contract](docs/architecture/forge-character-direction-motion-v8-plan.md).

The source tree contains an opt-in `consistency-v2` build for the v0.3 CLI release line. It adds
immutable Subject Locks, semantic image-reference roles, explicit keyframe/key-pose generation,
typed WorkflowGraph replay, a content-addressed cache, and `.forge/catalog.json`. These commands
are deliberately absent from the default release binary until the real-xAI acceptance gate
is complete. The v0.3 fixture/contract matrix passed all six gates on 2026-08-04
([`docs/qa/forge-v03-test-matrix.md`](docs/qa/forge-v03-test-matrix.md)); the real-model
identity promotion gate remains open. The signed SAM/DINO/LPIPS component remains unpublished until license and calibration
review; `forge component install` never substitutes unreviewed weights.
The implemented offline contracts, Godot verification, and remaining external gates are recorded in
[`docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md`](docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md).

A bounded real-xAI `walk_right` keyframe acceptance on 2026-08-08 was blocked
correctly after 14 image edits. It exposed semantic content leakage from the
opaque multi-object Style board into Character frames; no Pack was exported.
See [`docs/qa/forge-character-walk-right-keyframe-real-2026-08-08.md`](docs/qa/forge-character-walk-right-keyframe-real-2026-08-08.md).

</details>

## Unreleased Stage 3 static asset system

The `collection-assets` source feature adds immutable Collection Locks and
project-level consistency for Icon/Prop V2, Portrait, Equipment V1, and Decal
sets. It is not part of the published v0.2 binary. A one-style, three-Pack real-xAI probe completed on
2026-08-05 and did **not** pass the release gate: it exposed Collection anchor,
report-closure, Portrait framing, review/Catalog, and Godot kind-mapping blockers.

The later [remediation report](docs/qa/forge-stage3-blocker-remediation-2026-08-05.md)
records acceptance of the repaired three-Pack scope, including Portrait and project
audit. This scoped result does not establish the full frozen five-style matrix or
promote `collection-assets` into the default release.

<details>
<summary>Static asset contracts, optional commands, and acceptance history</summary>

Portrait framing is now an explicit offline contract. Existing Portrait V1 specs keep
`dialogue_bust@1.0.0`; Portrait V2 selects either that profile or
`full_body@1.0.0`. Full-body candidates are checked before crop/scale normalization,
must use a feet-grounded Collection Lock, and cannot use dialogue-bust reframing.
Obvious missing lower-body silhouettes are blocked before Pack export. The report calls
its deterministic measurement `lowerBodyPresenceProxy`; it does not claim semantic leg
verification.

Portrait V2 now treats a qualified `neutral` image as an immutable Portrait Base. The
remaining expressions are independent edits of that one base, and Forge deterministically
restores every pixel outside a versioned face scope. Local skin-tone and protected-cheek
artifact gates catch defects that whole-image palette metrics miss. The implementation was
replayed against the frozen 2026-08-06 xAI outputs with zero Provider requests: clothing,
scarf, body, and leg drift were eliminated; the observed hurt-face marks and surprised skin
drift were rejected before Pack export.

New Portrait V2 CLI runs are two-phase. `--phase base` spends only on neutral (estimated
1, maximum 2), then stops without a Pack until `forge job review --accept` writes a
hash-bound approval. `--phase expressions --base-job <id>` spends only on the four
expressions (estimated 4, maximum 8). The default `subject-style` reference policy gives
identity precedence and removes the potentially conflicting Collection anchor image from
the neutral request.

`portrait-local@1.1.0` makes skin review expression-aware: expanded eyes and mouths are
excluded from skin-tone sampling, matched skin uses a trimmed mean, and a separate skin
correspondence ratio catches invalid recoloring. Non-severe skin-only findings may pause
for explicit review; body-pixel changes, severe marks, identity drift, and other hard
defects remain non-overridable.

Paid runs can now use `forge provider authorize` plus `--authorization <id>`. Its
non-secret manifest and atomic request ledger enforce per-target, total-request, and
cost caps across detached workers, retries, and replay Jobs. `forge job report` exposes
the ledger for audit without credentials, prompts, headers, or temporary URLs. See
[`docs/automation/forge-cli.md`](docs/automation/forge-cli.md) and the full-body examples
in [`examples/cli/stage3`](examples/cli/stage3).
See the [real acceptance report](docs/qa/forge-stage3-real-acceptance-2026-08-05.md).

```bash
forge collection create --project /absolute/assets --spec /absolute/collection.json --wait --json
forge collection inspect --project /absolute/assets --id inventory --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json --phase base --wait --json
forge job review --id <base-job-id> --accept --reason "neutral approved" --json
forge generate portrait-set --project /absolute/assets --spec /absolute/portraits.json --phase expressions --base-job <base-job-id> --wait --json
forge generate equipment-set --project /absolute/assets --spec /absolute/equipment.json --json
forge generate decal-set --project /absolute/assets --spec /absolute/decals.json --json

forge asset export-editable --project /absolute/assets --id inventory-icons --output /absolute/editable --json
forge asset replace-item --id <source-job-id> --item potion --path /absolute/potion.png --wait --json
forge project audit --project /absolute/assets --scope all --json
```

`replace-item` creates a child Job and makes zero Provider requests. Project
audit checks Pack/hash completeness, Style/Subject/Collection revisions,
quality and collection outliers, license/provenance, Godot installs, embedded
images, credential markers, and temporary media URLs. Schemas are available
through `forge schema list/show`; examples live in
[`examples/cli/stage3`](examples/cli/stage3). The frozen implementation and
real-provider gates are in
[`docs/architecture/forge-stage3-static-assets-implementation-plan.md`](docs/architecture/forge-stage3-static-assets-implementation-plan.md).

</details>

## Unreleased world pipeline

The next CLI milestones are implemented behind unreleased world build features and the
V3 Pack contract. These commands are intentionally absent from the default release
binary and stay experimental; the v0.3 matrix records their fixture-level pass.
[Real engineering acceptance](docs/qa/forge-world-v1-real-acceptance-2026-08-03.md)
also passed, while terrain repetition and building art quality prevented release
promotion. Terrain, Building, and Map have separate release milestones.

<details>
<summary>Experimental world scope and commands</summary>


- immutable top-down Environment Locks;
- deterministic 16/32 px dual-grid Terrain Sets built from two Provider-generated
  material plates;
- modular exterior Building Kits with fixed roof, wall, door, and window modules;
- a Provider-free JSON Map Compiler that produces a self-contained Godot world.

Create the Environment Lock and world art:

```bash
forge environment create --project /absolute/assets --spec /absolute/environment.json --wait --json
forge generate terrain-set --project /absolute/assets --spec /absolute/terrain.json --wait --json
forge generate building-kit --project /absolute/assets --spec /absolute/buildings.json --wait --json
```

Maps accept JSON only. Forge does not call a text model or translate natural language;
Codex, Claude, or a user writes `MapSpecV1`, then Forge validates and deterministically
compiles it:

```bash
forge map schema --json
forge map compile --project /absolute/assets --spec /absolute/map.json --wait --json
forge map validate --pack /absolute/Forest-Village.gsfpack --json
```

The V1 world scope is top-down outdoor maps, dual-grid Terrain Sets, rectangular
3×3–8×6 exterior buildings with south-facing entrances, and Godot 4.6.x. It does
not include indoor, isometric, platformer, 3D, Tiled, Unity, or Unreal output.
Runnable JSON examples live under [`examples/cli/world`](examples/cli/world).

</details>

## Security and provenance

- Provider output is materialized, format-checked, and SHA-256 hashed before
  local processing.
- A job locks one Provider, profile, model selection, and Style revision.
- Credentials are stored in Keychain and never enter jobs, packs, logs, or
  normal JSON output.
- OAuth is Preview. API Key is the stable commercial authentication path.
- Godot writes are confined to `addons/forge_assets` and replace only
  Forge-owned output.

## Development

Contributor build and test instructions live in [CONTRIBUTING.md](CONTRIBUTING.md).
Architecture and automation contracts live under `docs/architecture` and
`docs/automation`.

## License

Forge is licensed under the [MIT License](LICENSE). Bundled FFmpeg helpers have
separate LGPL notices and corresponding source published with each release;
see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
