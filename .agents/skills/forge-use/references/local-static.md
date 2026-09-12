# Local PNGs → static Pack → Godot

Use this route for Codex-generated artwork and existing static PNGs.
Follow the [toolchain and Job checks](../SKILL.md) (`"$FORGE_BIN" guide overview`
when reading the embedded guide) before running the workflow.

## Preserve and inspect sources

If new artwork or a creative edit is requested, use the available image-generation
tool, then inspect its saved PNG output. Keep the original unchanged with its
SHA-256, source/tool identity and rights information. Do not invent a model name,
Forge Provider or Style Lock in provenance. Image generation may consume Codex
usage; local Forge preparation should report **zero Forge Provider requests**.

Prefer one transparent PNG per item. Use `icon_set` for centered UI items and
`prop_set` for objects with a ground anchor; a still character can be a prop.
For an existing collection sheet, preserve the sheet and record how each separate
item source was derived. Do not treat unrelated props as consecutive frames.
Inspect actual transparency, subject edges and framing before import. Forge
normalizes alpha-bearing PNGs; it does not remove an opaque background or apply
chroma-key matting in the default static workflow. Source builds can explicitly
remove a flat background with `source matte`, or retain a reviewed rectangular
RGB/RGBA canvas with `canvasPolicy:"preserve_source"`, as described below.

## Prepare a request

Copy [the local static example](../examples/local-static.json) from an installed
skill into the game's asset-spec directory, or retrieve it from a verified CLI
with `embedded_usage_guide` into a new request file:

```bash
"$FORGE_BIN" guide static-example > /absolute/asset-specs/local-static.json
```

Confirm the command succeeded before editing or consuming the file; a failed
command can leave an empty redirected file. Change IDs, names, paths and `license`
for the real sources; the example's `private` label grants no rights. Its paths
assume this layout:

```text
asset-specs/local-static.json
sources/lantern.png
sources/rock.png
```

Paths resolve relative to the request file, or to the working directory with
`--stdin`. The example is `schemaVersion:"1"`, `prop_set`, with painted (`linear`)
sampling. For UI icons, use `kind:"icon_set"`; for pixel art choose `nearest`.
Keep item IDs stable across revisions. IDs start with an ASCII letter or digit,
contain only ASCII letters, digits, `_` or `-`, and are at most 80 bytes. Item IDs
must be unique even ignoring case. A set accepts 1–64 items. Default normalized PNGs need an alpha
channel, transparent background and visible foreground, at most 4096 pixels per
dimension and 32 MiB per file. Canvas sizes are 64, 128, 256 or 512.

`foregroundAlphaThreshold` (1–255, default 1) selects the subject bounds;
`edgePaddingPx` (0–64, default 0) expands the crop in source pixels. Start at
`1/0`. Change them only after inspecting faint distant alpha residue or soft
edges. The threshold is not an alpha cutoff: pixels inside the crop retain their
alpha before resampling. Inspect the normalized result on light and dark grounds.

```bash
export FORGE_JOB_STORE="/absolute/asset-work/jobs"
export FORGE_PLAN_STORE="/absolute/asset-work/plans"
"$FORGE_BIN" plan prepare-static --request /absolute/asset-specs/local-static.json --json
```

Inspect `data.estimate.providerRequestEstimate` and
`data.estimate.maximumProviderRequests`; both must be zero for this route. Read
`data.token`, then execute that token once:

```bash
"$FORGE_BIN" plan execute --token TOKEN_FROM_PLAN --wait --json
"$FORGE_BIN" job report --id JOB_FROM_EXECUTION --json
```

Require a successful envelope and `data.lifecycle_state:"succeeded"` from the
execution. Its `data.job_id` identifies the Job. The report must have
`data.providerRequestOccurred:false` and `data.providerRequestCount:0`.
Find the Pack in the execution/Job's `data.artifacts` entry with `kind:"gsfpack"`;
use the returned path instead of guessing a file or archive location.

```bash
"$FORGE_BIN" pack validate --path PACK_FROM_JOB_ARTIFACTS --json
"$FORGE_BIN" asset inspect --pack PACK_FROM_JOB_ARTIFACTS --json
```

## Review normalization

The Job preserves source copies, source/normalized hashes, fingerprints, a contact
sheet and quality evidence. Review those artifacts against the originals. The
cropped region fits 82% of the canvas's longest extent. `nearest` resizes with
nearest neighbor; `linear` uses Lanczos resampling and linear engine filtering.
Icons are centered. Props use the ground origin
`(canvasSize / 2, canvasSize - canvasSize / 16)`. Padding belongs to the normalized
extent, so the visible subject can sit slightly above that origin. Normalization
does not promise pixel-identical source output.

### Keep a reviewed native canvas

For native-sized backgrounds, overlays, and already-framed images, set
`canvasPolicy:"preserve_source"`, omit `canvasSize`, and leave `edgePaddingPx`
at zero. This opt-in accepts 8-bit RGB/RGBA PNGs, including opaque backgrounds,
within the same 4096-pixel and 32-MiB input limits. All items in one Pack must
have identical source dimensions; separate different canvases into separate Packs.
Rectangles and non-power-of-two sizes are valid. The existing plan/execute/install
commands apply unchanged.

Native item textures and retained frames preserve original PNG bytes, pixel
coordinates, and alpha. Atlas/preview images remain derived evidence. The report
records `sourceBytesPreserved`, dimensions, source/output hashes, and
`canvasPolicy`; opaque images truthfully report `transparentBackground:false`.
The helper uses real width/height and a custom `(0,0)` anchor. Generated prop
scenes have an uncentered `Sprite2D` at `(0,0)`; icon consumers apply the origin
and filter to their own nodes. Default requests still use normalized square
canvases and the previous center/ground anchors.

### Matte one flat-background source locally

Run `"$FORGE_BIN" source matte --request /absolute/matte.json --json` with:

```json
{
  "schemaVersion":"1",
  "input":"sources/portrait-on-white.png",
  "output":"derived/portrait-matte.png",
  "parameters":{
    "keyMode":"manual", "manualKeyColor":"#FFFFFF",
    "threshold":24, "softness":32, "despillStrength":0.0, "haloPixels":0
  }
}
```

Paths resolve against the request file. The explicit output must be a new PNG;
the source and any existing output are protected. Processing retains original
dimensions and coordinates, applies the local chroma algorithm, and clears RGB
where alpha becomes zero. Inputs are single still 8-bit RGB/RGBA PNGs (up to
128 MiB and 33,554,432 pixels). Threshold/softness are 0–255, despill strength
0–2, and halo erosion 0–4 pixels. Omitted parameters select `auto_corners` with
threshold 48, softness 18, despill 0.5, and halo 0.

Matting has no Provider calls or Job store. Retain its report with input/output
hashes, dimensions, parameters, resolved key color, alpha statistics, and
`visualReviewRequired:true`. Inspect soft edges and matching foreground colors;
chroma keying does not understand the subject. An all-transparent result is
rejected. Review the output before importing it with either canvas policy.

The local report records `styleConsistencyEvaluated:false` and
`visualReviewRequired:true`: its `game_ready` verdict covers structural checks.
Review framing, edges, style and readability at gameplay scale separately.

Local static intake has no targeted retry or asset-project catalog registration.
To revise a set, prepare a new request with stable IDs and keep the prior receipt.
Install this kind of Pack without `--catalog-project`.

## Install into Godot

Use the intended Godot 4.6.x project. If discovery chooses the wrong engine, set
`FORGE_GODOT_PATH` to the verified Godot executable. Forge prepares resources in an
existing project; it does not bundle Godot or package the game for a platform.

```bash
"$FORGE_BIN" godot plan-install \
  --pack PACK_FROM_JOB_ARTIFACTS --project /absolute/game \
  --asset-key forest_props --target addons/forge_assets/forest_props --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_INSTALL_PLAN --wait --json
```

Supply both a stable `--asset-key` and `--target`, even if the display name changes
or is not ASCII. Inspect the install plan's destination and ownership, then check
the install Job succeeded. Read the installed `forge_usage.json` and the project's
`.forge/assets.json`; consume returned resource paths and item IDs instead of
deriving filenames from display names. Forge updates only its owned target and
restores the previous installation if an install fails.

Static usage exposes `texturePaths`, `anchor` and `rendering.textureFilter`.
Props also have scenes with a `Node2D` root and a `Sprite2D` child. The child
applies the anchor through its position and sets the texture filter; inspect the
scene tree before binding game scripts. Icon texture consumers must apply the
declared filter on their own Godot nodes. Use wrapper
scenes for game-specific scale, offsets and behavior so subsequent Forge installs
can continue managing the generated resources.

## Keep an import receipt

Retain the source originals/hashes, generation or derivation history, actual
rights statement, request, CLI path/version/build/hash, plan and Job IDs,
source/normalized hashes, Pack hash or fingerprint, usage path and relevant
processing reports. Save evidence before cleaning Job stores. Preserve existing
receipts; a later import gets a new one linked to its source and predecessor.

Load the installed resources in Godot at the game's actual scale and record visual
approval separately from the immutable import result. Successful structural
validation and headless import do not establish art quality or device performance.
