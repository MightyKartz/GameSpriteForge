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
`AnimatedSprite2D` resources. GIF previews use uniform FPS: inspect nonuniform
`frameDurationsMs` in Pack/Godot resources and review actual playback in-engine.
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
