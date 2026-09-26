# Forest Courier: three views and one-direction movement

One generated three-view character, a rejected movement candidate, and a runnable
Godot turning test are included. **The user rejected the source action: it does
not read as sustained running.** The retained prototype is technical evidence,
not a completed run animation. Right uses the video-derived
`move_right` clip; left mirrors it without resetting phase. Up/down still use
standing reference images and are not animated.

已完成三视图与 Godot 转向技术测试；用户指出原视频没有跑动起来，当前动作候选已拒绝，单方向跑动动画尚未完成。
右方向播放视频提取的动画，左方向镜像并保留动画进度；上下方向仍为站姿参考。
原片动作更接近缓慢迈步；两倍播放与引擎平移不能证明跑步动作合格。当前资源仅保留作失败候选和技术测试证据。

![Three views](three-view-preview.png)

## Run

Open `godot/project.godot` in Godot 4.7.2, or from the repository root:

```sh
godot --path examples/three-view-run-pilot/godot --editor --import --quit
godot --path examples/three-view-run-pilot/godot
```

Arrow keys / WASD move. Space toggles automatic turning. Hold Shift for half
speed. The scene shows the shared foot origin and collision circle. A stopped
lateral actor freezes its current frame; no idle animation was generated.

The automated review visits right/left repeatedly and up/down once. It runs for
786 physics ticks (13.1 seconds), checking all 54 frames, exact installed timing,
cycle completion, actual velocity, stable anchors/collision and retained phase
at every turn:

```sh
godot --headless --path examples/three-view-run-pilot/godot -- \
  --test --report=/absolute/new-normal-report.json
godot --headless --path examples/three-view-run-pilot/godot -- \
  --test --slow --report=/absolute/new-slow-report.json
```

Use fresh report paths. For native recording, omit `--headless` and add
`--write-movie /absolute/new-preview.avi --fixed-fps 60` before `--`.
The saved [QA evidence](../../docs/qa/three-view-run-pilot-20260926.md) includes
native recording samples, measured reports and a clean-copy test.

## Source and preparation

A single Codex image-generation call produced right/back/front standing views.
Forge prepared the three static references. The complete original image and
videos remain in local task storage; this example contains selected derived
media and portable installed resources.

The first Q2 Pro subject-reference candidate cost 0 credits and earned the
20-credit subject-task reward (balance 1 → 21). It was rejected because boots
were cropped at the video boundary. Its evidence is retained in the QA record.

For the second candidate, the existing side crop was reduced and placed on a
1536×1024 magenta canvas with generous head/foot margins. The user approved one
Q2 image-to-video request at 10 credits (21 → 11). The exact submitted prompt is
[video-prompt-margin-v2.txt](video-prompt-margin-v2.txt). The generated video is
1764×1176, 24 fps, 122 frames, approximately 5.083 seconds.

The selected interval is source frames **47–100 inclusive**. Frame 101 supplies
a same-phase comparison and is excluded from the loop. Every intermediate frame
is retained. Shared crop `[480,160,800,800]` excludes the distant watermark and
keeps the complete character; no foreground watermark repainting was needed.
Forge `source matte` removes the sampled magenta background. A documented
whole-figure horizontal translation removes video drift; Y and all joint
relationships remain unchanged. A common 0.5 scale produces 400×400 RGBA frames.

The source interval is 2.25 seconds. The review plays it at 2× speed, yielding a
1.125-second, 54-frame loop with explicit 20/21 ms durations. Half-speed mode
restores the original source cadence. All source indices, hashes, translations
and durations are in the [cycle manifest](../../docs/qa/artifacts/three-view-run-pilot-20260926/cycle-manifest.json).
There is no skeleton, per-limb repositioning or synthetic pose generation.

Forge's strict GameReady export stopped at `prototype_usable`: bottom drift
9 px, bounding-box center-X variation 64.5 px, loop-match score 0.89246.
A separate review-only request exported the unchanged sequence; the stricter
failure remains recorded. Pack validation and Godot install verification passed
with zero Forge Provider requests. Vidu generated motion; Forge processed it.

## Review limits

Normal and half-speed native tests passed, including all frames, repeated turns
and exact timing. These are technical checks, not proof of natural foot contact
or artistic approval. Some source boot-color shimmer/soft edges and the loop
seam remain review items. Vertical views are nearly eye-level, not a verified
orthographic overhead projection. Mirroring is authorized for this empty-handed
character; it is not a rule for asymmetric or weapon-bearing characters.

The committed resources are a portable snapshot of Forge installation. Private
ownership records, stores and caches are excluded. For a new managed install,
use the retained Pack and receipts in a fresh project rather than treating this
snapshot as a registered installation on another machine.
