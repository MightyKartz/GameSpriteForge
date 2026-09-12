# Godot playback and preview

`forge godot preview` creates a new isolated Godot 4.6 project, installs the Pack
through the same transaction and native checks used for consumer delivery, and
validates the generated preview scene. It does not install into a game's project.

```powershell
forge godot preview --pack ./character.gsfpack --output ./preview-character --json
forge godot preview --pack ./effect.gsfpack --output ./preview-effect --godot C:/Tools/Godot/godot.exe --launch --json
```

The output must be a new directory. `project/` is the runnable preview; `jobs/`,
`plans/`, `preview-report.json` and `preview-verify.log` retain installation evidence.
The window offers clip selection, play/pause, time seek, speed and completion
status. Without `--launch`, it creates and verifies the project without keeping
a window open. Use Godot's project manager or `godot --path PREVIEW/project` to
open it later. Supported Pack types are animation, character and layered.

## One playback interface

Both delivered animation roots and layered roots provide:

| Method/state | Contract |
|---|---|
| `play(clip="", restart=true)` | Select a known clip; default/first clip is used when appropriate. Returns false for an unknown clip without changing state. |
| `pause()` | Stops time accumulation while retaining pose and time. |
| `seek(seconds)` | Nonnegative finite seconds; clamps non-looping clips and wraps loops. Recomputes pose immediately. |
| `set_speed(value)` | Nonnegative finite multiplier; zero retains time. Applied once during advancement. |
| `advance(delta_seconds)` | Advance a playing clip by external delta; ignores invalid/overflowing time. |
| `reset_pose()` | Stop and restore initial layer transforms, or the first frame of the selected sequence. |
| `state()` | Clip, playing, finished, positionSeconds, durationSeconds and speed. |
| `completed(clip)` | Signal emitted once when normal playback reaches a non-looping end. Seeking to the end stops without emitting it. |

The player's `_process(delta)` owns time by default. Games with their own clock
must call `set_process(false)` on the root and use `advance(delta)` or `seek(time)`.
The child AnimatedSprite2D internal clock is paused so it cannot double-advance.
Actual SpriteFrames duration weights/FPS determine each frame, including
nonuniform timing. An absolute 1 ns tolerance handles floating-point boundary
roundoff. This player owns no Mahjong rules, action scheduling or save state.

Layered tracks are absolute transforms, not accumulated deltas: each frame first
restores untracked layers to their initial transform and then samples the selected
clip. Position and pivot use source pixels; rotation uses degrees; scale and
opacity interpolate linearly. Rotation follows numeric values, not an implicit
shortest-path rule. See the [layered contract](layered-packs.md).

## Effect blending

Sequence request `rendering.blendMode` and each layered `blend` accept `normal`,
`add` and `multiply`. Normal/add use Godot CanvasItemMaterial; multiply uses a
frozen alpha-aware shader: destination RGB is multiplied by
`mix(1, source RGB, source alpha × layer opacity)`, retaining destination alpha.
Transparent source pixels have no effect, including black RGB padding. Multiply
darkens existing content and does not add coverage to an empty transparent canvas.
Omitted mode preserves legacy normal blending. These are 2D texture blend modes, not particle systems,
procedural shaders or automatic effect generation. Native resource checks verify
the saved material; actual render comparisons verify exported appearance.

## Validation

`scripts/test-godot-unified-player.py` checks native state, timing, interpolation
and completion against the frozen V1 controller. `scripts/test-layered-cli.py`
checks request → Pack → install → native preview and can read a consumer rig via
explicit `--rig` and `--consumer` paths. Consumer PNGs and locks remain unchanged;
private artwork stays in the local QA output and is never required by public CI.
`scripts/test-local-animation-delivery.py` checks exported sequence materials,
external SpriteFrames and normal/add/multiply behavior alongside legacy defaults.

Structural checks, native loading, rendered output comparison and a person's
art-direction review are distinct evidence. A passing structural report does
not declare a character's motion or style approved.
