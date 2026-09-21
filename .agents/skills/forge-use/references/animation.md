# Existing animation (experimental)

Character animation remains in development and testing. The default v0.3.0 CLI
can process local animation and preserve coordinates/timing; availability does not
establish visual quality. For a still character or unrelated items, use
[local static preparation](local-static.md) (`"$FORGE_BIN" guide static`).

Verify `local_animation_import`, `preserve_source_coordinates`,
`local_animation_timing`, and, when needed, `whole_sheet_source_transform` in the
selected executable's capabilities. v0.2.1 predates these newer local request
fields. Respect the [toolchain and Job checks](../SKILL.md)
(`"$FORGE_BIN" guide overview`). These `guide` commands require a verified CLI
with `embedded_usage_guide`; installed links work with older pinned executables.

## Preserve intentional drawing coordinates

Before cropping a sheet into separate files, inspect the entire cleaned sheet
with its actual cell dimensions. Record edge contact per cell and review extended
weapons, boots, hair and soft effects. An opaque candidate is not made transparent
by choosing `preserve_source`; reject a baked background or explicitly prepare a
reviewed matte. A cell-edge warning is evidence to inspect, not permission to
erase pixels or independently recenter frames. Keep failed candidates and bind
the chosen bytes with `sourceLocks` (`forge guide delivery`).

Use `plan prepare-asset` for one action. This v1 request assumes three already
aligned 64×64 PNG frames, saved relative to an asset-spec file. New builds resolve
paths relative to that file; use absolute paths with older pinned animation CLIs:

```json
{
  "schemaVersion": "1",
  "input": {
    "kind": "png_sequence",
    "paths": ["../sources/idle-0.png", "../sources/idle-1.png", "../sources/idle-2.png"]
  },
  "metadata": {
    "name": "Character idle", "animation": "idle", "fps": 10,
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

Choose the anchor from the actual art. `preserve_source` retains frame dimensions
and drawing positions without scaling, trimming or per-frame translation. Do not
independently crop/recenter aligned frames; that can erase intended motion.
All actions must share the same frame canvas and anchor, and margins must be zero.
Fractional anchors require `pixelSnap:false`.

Prefer a clean transparent RGBA sheet over a color-keyed source. Keep every
visible and faint glow pixel inside its declared cell, inspect the grid with
`source inspect --frame-width W --frame-height H`, and use
`matting:{"mode":"preserve_alpha"}` so the original soft edges survive. If an
existing sheet needs more room, expand the canvas or move each frame before
planning; do not erase faint pixels just to satisfy the boundary gate.

When explicit chroma matting is needed as a fallback, supported builds accept
optional `backgroundScope:"border_connected"` and `edgeColorRecovery:true`
inside the request's matting parameters. Border-connected mode protects enclosed
details that match the key, and edge recovery estimates soft-edge RGB from that
key. Review a contact sheet on dark and light backgrounds; neither option
approves animation quality.

For multiple actions use `plan prepare-character`, `schemaVersion:"2"`,
`metadata.defaultAnimation`, and at least two `animations[]` entries instead of
the single-action `input`/`metadata.animation` form. Each action has `name`,
`input`, `fps`, `loop` and optional `frameDurationsMs`. Keep normalization and
rendering at the request level. Each duration array needs one positive integer
per frame; the default animation's timing must survive action reordering.

```bash
"$FORGE_BIN" plan prepare-asset --request /absolute/asset-specs/idle.json --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_PLAN --wait --json
"$FORGE_BIN" job report --id JOB_FROM_EXECUTION --json
```

Inspect the plan's zero Provider request estimates and the completed Job's quality
result. `requireGameReady:false` is appropriate only for an explicitly intended
prototype and cannot waive a blocked result. Preserve the prototype decision and
review status; do not disable a failed gate to label the result successful.

## Three-action task

On builds whose guide index includes `animation-example`, retrieve the
[three-action request](../examples/local-character.json) (`forge guide animation-example`).
It expects three aligned 64×64 transparent PNGs for each of idle, walk and attack,
under `sources/` next to the request's `asset-specs/` directory. Paths resolve
relative to the request file. Adjust paths, shared anchor and timing to the actual
art; do not rescale or recenter frames to imitate the example.

This example deliberately selects prototype review (`requireGameReady:false`).
Choose the intended quality gate before execution and retain the result; a
blocked result still requires correction. It is not an artistic approval.
For a regular sheet, replace one action's input with `kind:"sprite_sheet"`, a
`path`, and `split:{"mode":"fixed_grid","frameWidth":64,"frameHeight":64,
"columns":3,"rows":1}`. Its frame order is row-major; timings are milliseconds.

```bash
"$FORGE_BIN" guide animation-example > /absolute/asset-specs/new-character.json
"$FORGE_BIN" plan prepare-character --request /absolute/asset-specs/new-character.json --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_PLAN --wait --json
"$FORGE_BIN" job report --id JOB_FROM_EXECUTION --json
"$FORGE_BIN" pack validate --path PACK_FROM_JOB --json
"$FORGE_BIN" godot plan-install --pack PACK_FROM_JOB --project /absolute/game --asset-key hero --target addons/forge_assets/hero --json
"$FORGE_BIN" plan execute --token TOKEN_FROM_INSTALL_PLAN --wait --json
"$FORGE_BIN" godot verify-install --project /absolute/game --asset-key hero --pack PACK_FROM_JOB --json
```

For repeated delivery after source review, add the reviewed `sourceLocks` and use
`forge guide local-delivery-example` with `--operation prepare-character`; that
existing script retains progress, the Pack and portable receipts. Follow
[delivery evidence](delivery.md) (`forge guide delivery`) for its invocation and
recovery. After a timeout, inspect the recorded Job before starting another one.

With `godot_lossless_sprite_import`, flat animation/character and icon/prop PNGs
are imported losslessly without alpha-border RGB rewriting, premultiplication,
mipmaps or downscaling. Both nearest and linear scene sampling retain the Pack's
RGBA texels; filtering/blending during rendering remains intentional. The
installer checks saved native pixels against the Pack and rolls back on a
mismatch; the verification phase records `verifiedSpriteTextures`. Its `.import` settings are managed inside the asset target, including
reinstallation. This policy does not change layered or world assets, existing
installations until explicitly reinstalled, or a game's pinned executable.
Older builds may alter faint RGB during Godot import despite preserving Pack PNGs.
Review linear-filtered edges on the actual game background when upgrading.

Invalid multi-action inputs identify the action; missing sequence files and
preserved-canvas errors include the zero-based frame index and source path.
A duration-count error reports expected and supplied counts; a zero duration
identifies `frameDurationsMs[index]`. Correct that input and create a new plan.
These diagnostics and structural checks do not establish natural motion.

## Diagnose and revise one frame

Builds with `animation_frame_issues` add `pixelDiagnostics.issues` to each
animation's existing quality report. Read `job report --id JOB --json` and locate
`reports.animation_quality_report.animations[]` by `name` (single-action Jobs use
`reports.quality_report`). Indices are zero-based **within that normalized action**, not the
reordered atlas. PNG sequences and unchanged fixed grids retain their declared
order. For a selected video loop, use its existing loop-selection provenance to
map output indices back; never treat them as raw video frame indices. `evidencePath` is a JSON pointer relative to that action's report;
it is not a filesystem path. The request's input and Job artifacts locate the PNGs.

- `empty_frame`, `foreground_below_threshold`, `frame_size_mismatch`: deterministic
  errors; inspect source/matting/thresholds or restore the shared canvas.
- `canvas_edge_contact`: review required; contact is measurable but clipping is
  uncertain. Keep intentional edge contact, or explicitly replace the source / agree
  a larger shared canvas. Forge cannot reconstruct missing pixels.
- `identical_visible_frames`: informational; consecutive visible pixels match.
  Keep an intentional hold. Hidden RGB under alpha=0 is excluded from this hint.

Each issue carries severity, certainty, frame index, optional related frame,
evidence pointer and correction options. Existing position/size and loop metrics
remain measurements; they do not infer foot contact or character identity.
An intentional jump must not be recentered merely to improve a score. The existing
quality gate remains explicit; a blocked frame cannot be approved or exported.
Older reports may omit `issues`; use the same selected toolchain to validate new
Packs because older strict report schemas may reject the added field.

Use the existing request as the minimal correction recipe:

1. Preserve the original request, source PNGs, Job, Pack and review evidence. Copy
   the request beside the original so relative paths keep their meaning.
2. For an individual frame sequence, change only the selected action's
   `input.paths[FRAME]` to a **new** replacement PNG. Keep its shared canvas and
   anchor. To change timing explicitly, edit only that action's
   `frameDurationsMs[FRAME]`. Do not copy a job acceptance into this new candidate.
3. For a fixed-grid sheet, keep an immutable revised sheet with the same cell
   layout, or explicitly switch that action to ordered individual frame files.
   Compare every unaffected cell; never silently infer order or drop frames.
4. If `sourceLocks` are present, first verify every unchanged hash. After reviewing
   the replacement source, replace only its corresponding lock; the locks must
   cover the new request's complete input set, exactly once. A shared source may
   still be used by another action and must retain its old lock. Do not recompute
   unchanged hashes to silently accept unrelated edits.
5. Create and execute a new `plan prepare-character` / `prepare-asset`. Read its
   new report, validate the Pack, and compare named actions' pixels and durations.
   Only the explicitly selected frames/timings may differ. Export builds the whole
   Pack again; this is not an in-place patch or a partial cache execution.
6. Review the candidate, then use the ordinary install/verify sequence above with
   the stable asset key and target. Forge owns that complete target; keep gameplay
   configuration outside it. No source or consumer toolchain pin is auto-upgraded.

The Agent may create a corrected PNG locally or use an external generation tool.
That generation is separate from Forge; retain the actual source/provenance and
never claim that the local replacement called a Provider.

For retained before/after candidates, reuse the optional library binding
`assetProject:{"projectPath":"../library","assetId":"hero"}` and exact revision
reviews (`forge guide project-assets`). A new revision starts without old approvals.
On builds with `synchronized_animation_review`, compare two exact revisions:

```bash
"$FORGE_BIN" asset preview --project /absolute/library --id hero --revision BEFORE --revision AFTER --out /absolute/new-comparison --json
```

The offline page uses original PNGs at 1× by default, frame stepping, light/dark
backgrounds and anchor guides. Enable **Synchronize PNG players** to select a
shared action, play/pause or seek together. All versions use the same elapsed
milliseconds, each with its own native durations and loop policy; differing
speeds are not hidden by matching frame indices or stretching the cycle. Non-loop
versions stop at their endpoint. Previewing records no approval. Browser refresh
and normal alpha blending still differ from native engine rendering; use the
Godot verification and actual background for delivery review.

## Whole-sheet preprocessing and repair

Within a `fixed_grid` split, supported builds accept `sourcePaddingRightPx`,
`sourcePaddingBottomPx`, `sourceOffsetX` and `sourceOffsetY` (all default zero).
Padding adds transparent pixels to the whole sheet; offsets translate the whole
original, not individual frames. Derived dimensions must match the declared grid.
Discarding any nontransparent source pixel, including alpha 1, is rejected. This
cannot repair inconsistent pose placement within separate cells.

Keep the Job's `source_transform` sidecar when this preprocessing is used. It
records original/derived hashes, dimensions, parameters and zero discarded
nontransparent pixels. The full sidecar stays in the Job store, so retain it with
the import receipt before cleaning that store.

Automatic repair preserves `preserve_source` coordinates. Canvas/anchor changes
remain manual actions; the plan can still contain other supported corrections.
Inspect both changes and remaining manual actions before executing a repair.

Validate the Pack, then follow [Godot delivery](local-static.md#install-into-godot)
(`"$FORGE_BIN" guide static`) with a stable asset key and target. Animation installs
native `SpriteFrames` and
`AnimatedSprite2D` resources. Inspect nonuniform `frameDurationsMs` in Pack/Godot
resources and review actual playback in-engine. Older GIF exports may use uniform
FPS even when native timing differs.
Keep original source hashes, request, source-transform evidence and receipt;
record structural results separately from visual approval.

## Effects and preview timing

With `effect_quality_profile`, local animation requests can select
`"quality":{"profile":"effect","requireGameReady":true}`. The default
`character` profile retains its foot/center stability checks. The effect profile
allows natural smoke, fire and magic deformation while checking actual missing
frames and canvas consistency. It records alpha bounds, brightness and
premultiplied RGB/alpha differences between adjacent and first/last frames, both
after matting and after normalization. These measurements do not approve motion.

For intentional disappearance at the end of a non-looping effect, explicitly add
`"allowTransparentTail":true`. An all-empty animation or missing interior frame
still fails, as does a tail lost during normalization. Looping effects and
character requests cannot use that exception. Single-action non-looping requests
do not receive a loop-trimming recommendation just because their endpoints differ.

With `preview_timing_diagnostics`, each exported GIF has a `.timing.json` sidecar
beside it. It records encoded GIF duration, nominal FPS and actual native timing.
For example, 8 fps requests 125 ms but GIF's centisecond resolution stores 130 ms;
eight frames preview as 1040 ms instead of 1000 ms. Nonuniform native durations
also remain authoritative. Keep the sidecar when sharing the preview.

Builds with `pack_mp4_preview` fix GIF disposal and encode per-frame delays rounded
to 10 ms. Transparent GIF uses an alpha threshold of 128; soft transparency and
full PNG colors cannot be retained. Existing Pack GIFs are not rewritten.

## Sharing a video preview

With `pack_mp4_preview`, export a flat animation/character Pack directly from its
original PNGs, without using its GIF:

```bash
forge pack preview --path ./effect.gsfpack --out ./strike.mp4 --animation strike --background dark --cache-dir ./preview-cache --json
```

`--animation` defaults to the first manifest animation. Background choices are
`dark` (default), `light` and `checkerboard`. The MP4 contains one cycle, with a
baked background and normal alpha composition; it is not a transparent game
asset. Timing uses a 60 fps grid (very short frames may be skipped); JSON reports
native/encoded duration, encoder, source/video hashes and cache identity. Odd
dimensions are padded right/bottom, never scaled. Maximum canvas: 4096 × 4096;
maximum cycle: five minutes. Engine blend modes require native Godot review.

FFmpeg must provide libx264, h264_videotoolbox or h264_mf. Forge uses its existing
bundled/PATH helper discovery, and never downloads an encoder automatically.
Missing or unusable encoders fail explicitly. `--out` must be a new `.mp4` path
outside the Pack. `--cache-dir` is optional and must also be outside the Pack;
it keys on source content, animation, background, encoding profile and FFmpeg
binary hash. Cached bytes are verified before reuse. A corrupt entry is rejected;
remove that cache-key directory and retry. Cache files are derivatives, not
catalog revisions or delivery receipts.

For precise transparent review, builds with `project_asset_png_animation_preview`
use PNG playback in `forge asset preview`, including old flat Packs. The player
offers animation choice, pause, frame stepping and background selection. Display
refresh and browser scheduling still affect live playback. GIF remains an internal
compatibility artifact required by existing v1/v2 Pack contracts.

For retained source-transform reports and immutable delivery receipts, read
[delivery evidence](delivery.md) (`"$FORGE_BIN" guide delivery`). A source checkout
also contains `examples/godot/forge-external-clock`, a generic SpriteFrames sampler
that handles pause, loop completion and per-frame timing. Gameplay clock policy
remains the consuming game's responsibility.

## Native preview and common player

Builds advertising `godot_native_preview` can create an isolated project with
`forge godot preview --pack asset.gsfpack --output new-preview --godot PATH --json`.
The output directory must be new. Add `--launch` to open Godot playback controls:
clip selection, play/pause, time seek, speed and finished state. The command runs
the ordinary transactional install plus a native scene check and keeps its Jobs
inside the preview directory. It accepts animation, character and layered Packs.

With `godot_unified_playback`, the delivered scene root exposes `play(clip, restart)`,
`pause()`, `seek(seconds)`, `set_speed(nonnegative_speed)`, `state()` and `reset_pose()`.
Nonuniform SpriteFrames durations remain authoritative. Completion stops a
non-looping clip and emits `completed` once during advancement; seeking to the
end stops without emitting completion. For an external game clock, disable the
root's `_process` with `set_process(false)` and call `advance(delta_seconds)`;
do not run two clocks. Speed applies once inside `advance`.

With `effect_blend_modes`, animation `rendering.blendMode` accepts `normal`, `add`
and `multiply`. Normal remains the default. Preview on the intended game
background because additive and multiplicative effects depend on that background.

## Registered layers

With `layered_pack_v1`, `forge asset prepare-layered --request layers.json
--output character.gsfpack --json` accepts a shared `canvas:{width,height,origin:[0,0]}`,
`sampling`, and ordered `layers`. Every layer supplies `id`, `name`, local `path`,
exact `sha256` and source-pixel `pivot`; optional `transform` defaults to identity
and `blend` to normal. Package identity fields are `schemaVersion:"1"`, `id`,
`name`, and `license`. Layer PNGs retain their original bytes and common canvas.
This step does not infer layers or perform background removal.

Optional `clips` carry `id`, `durationMs`, `loop`, and per-layer `tracks` with
`layerId` and strictly ordered `keyframes`. Keyframes include both endpoints and
contain complete `transform:{position:[x,y],rotationDegrees,scale:[x,y],opacity}`.
Transforms interpolate linearly around each pivot. There are no bones, meshes,
parent hierarchies or automatic occlusion repair in V1. Static composition and
motion appearance still require artwork-specific review.
