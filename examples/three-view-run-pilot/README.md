# Forest Courier: three views and one-direction movement

The current Godot example plays a **29-frame running review prototype** extracted
from the late part of Vidu candidate v3. The user confirmed that this part runs
and requested processing. Right uses the original sequence; left mirrors it
without restarting the animation. Up/down remain standing references.

当前示例已换成最新视频后半段的 29 帧跑动循环，保持原速。左右方向共用动画，
向左时镜像；上下方向仍为站姿参考。用户认可原视频后半段跑起来并授权处理，
处理后的边缘、循环接缝与游戏内脚步接触仍待审阅，并非正式品质批准。

![Three views](three-view-preview.png)

## Run

Open `godot/project.godot` in Godot 4.7.2, or from the repository root:

```sh
godot --path examples/three-view-run-pilot/godot --editor --import --quit
godot --path examples/three-view-run-pilot/godot
```

Arrow keys / WASD move; Space toggles the automatic turn test. Hold Shift for
half speed. A stopped actor freezes the lateral frame; no idle animation is
claimed. The visual parent mirrors around the shared pivot while the collision
shape remains unchanged.

The automated test runs 786 physics ticks (13.1 seconds), visiting left/right
repeatedly and up/down once. It checks all 29 frames, exact installed durations,
complete cycles, measured velocity, pivot/collision stability and preserved
animation progress through turns:

```sh
godot --headless --path examples/three-view-run-pilot/godot -- \
  --test --report=/absolute/new-normal-report.json
godot --headless --path examples/three-view-run-pilot/godot -- \
  --test --slow --report=/absolute/new-slow-report.json
```

For native recording, omit `--headless` and add
`--write-movie /absolute/new-preview.avi --fixed-fps 60` before `--`.
Normal/half-speed native recording and a clean-copy import/runtime check passed.
See [QA evidence](../../docs/qa/three-view-run-pilot-20260926.md).

## Current source and processing

One Codex image-generation call created right/back/front standing views. The
existing padded side reference drove Vidu Q2 candidate v3 with the
[short video prompt](video-prompt-v3.txt). The user authorized its 10-credit
submission (11 → 1) and subsequently requested processing its late running part.
This local processing iteration made no additional generation request.

Source frames **91–119 inclusive**, at 24 fps, form the selected consecutive
cycle. Frame 120 is the same-phase boundary comparison and is excluded. The
standing lead-in is discarded; no interior frames are omitted. The native cycle
lasts 1208.333 ms, represented as 1208 ms through cumulative timestamp rounding
into 41/42 ms frame durations. Playback is **1× source speed**, not the rejected
previous candidate's 2× playback.

All frames use crop `[550,160,800,800]` and a common 0.5 scale to 400×400 RGBA.
Forge removes the sampled magenta background; the crop excludes the remote
watermark without painting on the character. No per-frame translation, warping,
bone animation or limb edits are applied. The original torso view variation is
retained. [Frame hashes, source indices and timing](../../docs/qa/artifacts/three-view-run-pilot-20260926/run-v3-cycle-manifest.json).

Forge 0.6.3 prepared and installed an explicitly labelled review prototype at
`addons/forge_assets/courier_run_v3`. Pack validation and installation verification
passed with zero Forge Provider requests. Its quality verdict is
`prototype_usable`: loop-match 0.511799, bottom variation 20 px, center-X variation
27.5 px. These diagnostics do not grant artistic approval. The scene uses a
shared pivot (200,360), scale 0.7, horizontal speed 140 px/s and collision radius
16. Half-speed mode slows both animation and movement.

## Review limits and retained attempts

Review the source torso rotation, boot-color shimmer, soft/key-colored edges,
loop seam and foot contact in the native previews. The user accepted continued
work on the late source motion, not every property of the final sprite. Up/down
locomotion is not implemented. The reference projection is nearly eye-level;
this is not verified overhead movement. Horizontal mirroring is appropriate to
this authorized empty-handed character, not a rule for asymmetric designs.

Earlier attempts remain documented in QA: the free Q2 Pro clip cropped boots;
the next Q2 clip was rejected for slow stepping rather than running. Its old
installed target remains for provenance but is not used by the current scene.

Only selected derived media and portable installed resources are committed.
Original video, full-resolution input, Job/Plan stores, private ownership files
and `.godot` caches remain local. The committed project is a runnable snapshot;
use the retained Pack in a fresh project for a new Forge-managed installation.
