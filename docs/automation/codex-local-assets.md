# Codex artwork → Forge → Godot

Use Codex's built-in image generation for the artwork, then Forge for reproducible
local processing, Pack validation and Godot installation. Sword uses this workflow
for its static props and prototype animation. Forge does not call Codex's image
model as a Provider: the handoff is a local image file.

## Availability: check before following these examples

Audited on **2026-09-08**, against main baseline `b44e1d3` and the two local
development branches below. The development capabilities are **not in the
published v0.2.1 download** and are not added to main by this documentation change.

| Workflow | Required implementation | Published v0.2.1 / audited main |
| --- | --- | --- |
| Existing local animation processing, Pack validation, Godot install | Default CLI | Available |
| Transparent PNG sets with `plan prepare-static`, static filtering/ground anchors, alpha-bound options | `codex/sword-static-delivery`, `c1f448082c34f531738126e38e41f5a9d66ca1a7` | Not available |
| Local request `preserve_source`, explicit rendering and frame timing | `codex/sword-animation-delivery`, `9efc15b2075464a0e8a16861331245a55187c645` | Not available in these local request forms |
| Whole-sheet transparent padding and translation | Same animation branch, `d4b18e2c2792db9877e5d5517ef390bbc254e2d2` | Not available |

The animation branch includes the static work. Sword deliberately retains separate
static and animation binary locks so animation development cannot silently change
existing static imports. These are local development references, not promised
remote branches or installable releases. Use an existing verified checkout if
available; otherwise integration/release of the required implementation remains
work to do. Do not silently fall back to the stable binary.

All three binaries can report `forge 0.2.1`. Record the absolute binary path,
source commit, feature set, SHA-256 and `doctor --json` output. Check `plan --help`
for `prepare-static`; newer fields on existing commands require the matching
implementation and a successful request plan as well. A source checkout's HEAD
does not identify a stale `target/debug/forge` left by an earlier build.

## Choose the source and asset type

- Keep image-generation originals unchanged, with their hashes, source/tool
  identity and asset rights information. Inspect transparency, subject edges and
  framing before importing. Forge processing creates derived images separately.
- Prefer one transparent PNG per static item. Use `icon_set` for centered UI items
  and `prop_set` for world objects with a ground anchor. A still character can be a
  static prop while its animation remains under development.
- A sheet of unrelated props is a static collection, not an animation. Do not
  route it through animation loop scoring just to get an export. For new artwork,
  request separate items; for an existing collection, preserve the sheet and record
  how the separate item sources were derived.
- Use PNG sequences or sprite sheets for actual animation frames. Frame order,
  common canvas, anchor, timing and loop intent belong in the request.

Local preparation requires no xAI credentials, Provider login or generated Style
Lock. The original image-generation action may consume Codex usage. Report
**zero Forge Provider requests**, not zero total generation cost. Provider-backed
`style create` / `generate` is a separate route in the [CLI guide](forge-cli.md).

## Import transparent static items (development build)

Set `FORGE_BIN` to the verified executable and use dedicated job/plan stores. For
Godot delivery, set `FORGE_GODOT_PATH` if automatic discovery selects the wrong
engine. Keep the stores long enough to retain source and processing evidence.

Save a request such as `asset-specs/local-props.json`:

```json
{
  "schemaVersion": "1",
  "kind": "prop_set",
  "id": "forest_props",
  "name": "Forest props",
  "license": "private",
  "sampling": "linear",
  "canvasSize": 256,
  "foregroundAlphaThreshold": 16,
  "edgePaddingPx": 16,
  "items": [
    { "id": "lantern", "name": "Stone lantern", "path": "../sources/lantern.png" },
    { "id": "rock", "name": "Moss rock", "path": "../sources/rock.png" }
  ]
}
```

Replace `license` with the actual rights/distribution statement for your sources;
the example's `private` label does not grant rights. Paths resolve relative to the
request file, or the working directory for `--stdin`. IDs must be unique,
engine-safe identifiers. A set accepts 1–64 PNGs, at most 4096 pixels per dimension
and 32 MiB per file. Canvas sizes are 64, 128, 256 or 512. Opaque backgrounds,
corrupt PNGs and missing visible foreground are rejected.

```bash
"$FORGE_BIN" plan prepare-static --request /absolute/asset-specs/local-props.json --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_PLAN --wait --json
"$FORGE_BIN" job report --id JOB_FROM_EXECUTION --json
"$FORGE_BIN" pack validate --path PACK_FROM_JOB_ARTIFACTS --json
```

Read `data.token` from the plan; the token expires in 15 minutes and is single-use.
Check `data.estimate.providerRequestEstimate` and `data.estimate.maximumProviderRequests`
are zero. After execution, require `data.lifecycle_state == "succeeded"`; inspect
other states before proceeding. Find the `gsfpack` artifact in `data.artifacts`
instead of guessing an output path. `job report` should report
`providerRequestOccurred:false` and `providerRequestCount:0`.

The importer preserves original source files, normalizes each item onto its own
canvas, and creates a static Pack. It does not apply chroma-key matting. `nearest`
uses nearest-neighbor resizing; `linear` uses Lanczos resizing. Installed prop
scenes apply the requested engine filter. Icons supply textures and usage metadata;
their consumer must set its node's texture filter accordingly. The cropped region
fits 82% of the canvas's longest extent. Icons are
centered; props use center X and a ground line at `canvasSize - canvasSize / 16`.

`foregroundAlphaThreshold` (1–255, default 1) selects subject bounds.
`edgePaddingPx` (0–64, default 0) expands the crop in source pixels. Alpha is retained
inside that crop, including pixels below the threshold; resampling then creates
the normalized output. These controls are useful for distant faint alpha residue,
but `16/16` is only a tested Sword choice. Inspect the result on light and dark
backgrounds. Padding can leave the visible subject slightly above the nominal
ground anchor. Do not assume normalized output is pixel-identical to its source.

Local static quality explicitly records `styleConsistencyEvaluated:false` and
`visualReviewRequired:true`. Its `game_ready` verdict covers structural checks.
The initial importer does not register a Forge asset-project catalog or support
item retries. To revise the set, submit a new request; install without
`--catalog-project`. Keep stable IDs to update the same installed asset.

## Preserve intentional animation coordinates (development build)

Use `plan prepare-asset` for one action and `plan prepare-character` for multiple
actions. Example single-action request for an already aligned three-frame sequence:

```json
{
  "schemaVersion": "1",
  "input": {
    "kind": "png_sequence",
    "paths": ["/absolute/idle-0.png", "/absolute/idle-1.png", "/absolute/idle-2.png"]
  },
  "metadata": {
    "name": "Cultivator idle", "animation": "idle", "fps": 10,
    "loop": true, "frameDurationsMs": [70, 150, 230]
  },
  "normalize": {
    "mode": "preserve_source", "margin": 0, "marginBottom": 0,
    "alphaThreshold": 0,
    "manualAnchor": { "x": 32, "y": 52, "lockedByUser": true }
  },
  "rendering": { "textureFilter": "linear", "pixelSnap": false },
  "quality": { "requireGameReady": true }
}
```

Choose the anchor for the actual frame dimensions (this example assumes 64×64).
`preserve_source` keeps frame dimensions and drawing coordinates, without scaling,
trimming or per-frame translation. All actions must share the same canvas and
anchor; margins must be zero. Fractional anchors require `pixelSnap:false`.
For `prepare-character`, use `schemaVersion:"2"`, `metadata.defaultAnimation`,
and at least two `animations[]` entries instead of the single-action `input` and
`metadata.animation` form above. Each action has its own `name`, `input`, `fps`,
`loop` and optional `frameDurationsMs`; each duration array must contain one
positive integer per frame. Keep normalization/rendering at the request level.
Reordering the default action must not change its timing. Inspect the actual job
quality result before delivery.

If the source sheet does not fit its grid, the animation implementation supports
`sourcePaddingRightPx`, `sourcePaddingBottomPx`, `sourceOffsetX` and `sourceOffsetY`
inside a `fixed_grid` split. Defaults are zero. Padding adds transparent pixels to
the whole sheet; offset moves the whole original, not individual frames. The
derived dimensions must match the declared grid. Any discarded pixel with alpha
greater than zero causes rejection, even at alpha 1. This preserves source evidence;
it cannot repair inconsistent poses or bad placement within individual cells.

Retain the Job's `source_transform` sidecar when preprocessing is used. It records
original/derived hashes, dimensions, parameters and zero discarded nontransparent
pixels. This evidence remains in the Job store, rather than being copied in full
into the Pack. Preserve it alongside the import receipt if job stores are cleaned.

Character animation remains experimental. `requireGameReady:false` is appropriate
only for an explicitly intended prototype and cannot waive a blocked result.
Do not change a failed request to disable its quality gate without preserving that
decision and prototype status. GIF previews still use uniform FPS; inspect exact
`frameDurationsMs` in Pack/Godot resources and review the animation in-engine.

## Install and use the resources

```bash
"$FORGE_BIN" godot plan-install \
  --pack /absolute/Forest.gsfpack --project /absolute/game \
  --asset-key forest_props --target addons/forge_assets/forest_props --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_INSTALL_PLAN --wait --json
```

Use explicit stable keys and target directories, particularly when display names
contain Chinese or change over time. Check install completion, then read
`addons/forge_assets/forest_props/forge_usage.json` and `.forge/assets.json`.
For static assets, use `texturePaths`, `anchor` and `rendering`; props also get
`Sprite2D` scenes. Animations get native `SpriteFrames`/`AnimatedSprite2D` resources.
Use wrappers for game-specific scale/behavior and leave Forge-owned files managed
by subsequent installs. Headless import, ownership checks and rollback apply here
as in other Forge installs.

Keep a receipt linking CLI identity, input hashes, request, Job IDs, processing
sidecars, Pack hash and usage path. A successful install is a resource contract,
not visual approval or iPhone performance evidence. Review at gameplay scale and
record later visual approval separately from the immutable import result.

## Reuse in another Codex game project

The repo includes [forge-use](../../.agents/skills/forge-use/SKILL.md) for product
use and [forge-dev](../../.agents/skills/forge-dev/SKILL.md) for Forge development.
A sibling game project does not automatically discover Forge's repo-local skill.
To use it there, make the Forge skill available through that project's or your
personal skill directory, preserving its links to this source checkout. A symlink
to the skill directory keeps one maintained copy; do not overwrite an existing
skill with the same name. The skill guides tool selection and delivery and does
not install or upgrade the Forge executable.

For current verification and remaining integration work, see the
[Sword workflow audit](../qa/forge-sword-workflow-audit-2026-09-08.md).
