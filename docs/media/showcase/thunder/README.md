# Thunder spirit and lightning

English | [简体中文](README.zh-CN.md)

Codex created the source artwork. Forge v0.4.0 prepared the animation Packs,
registered them in a project resource library and delivered native resources
to Godot 4.6.3. The [6.6-second replay](godot-demo.mp4) shows those resources in
a small, separate demonstration scene; it is not Sword gameplay or Forge UI.
The README's [GIF](godot-demo.gif) is a silent, reduced-size excerpt of that capture.

## Source artwork

These PNGs retain the original dimensions and file bytes. Each sheet contains
four frames in a 2 × 2 grid. The prompts describe the generation request, not a
guarantee that the generated image satisfies every instruction.

### Thunder spirit

![Four-frame thunder spirit idle source sheet](sources/thunder-idle.png)

[Idle PNG](sources/thunder-idle.png) · [Movement PNG](sources/thunder-move.png) ·
[Idle prompt](sources/idle-prompt.txt) · [Movement prompt](sources/move-prompt.txt)

Forge preserves the sheets' shared drawing coordinates and alpha, preparing idle
at 5 FPS and movement at 7 FPS. Both loops have four frames. The character is a
demonstration prototype (`prototype_usable`), prepared with `requireGameReady:false`
from the outset. Similar poses and frame-to-frame variation remain; this is not
a claim of production-ready character animation.

### Lightning

![Four-frame lightning source sheet before cleanup](sources/lightning.png)

[Original PNG](sources/lightning.png) · [Cleaned PNG used by Forge](sources/lightning-clean.png) ·
[Generation prompt](sources/lightning-prompt-v2.txt)

The sequence contains charge, strike, shockwave and fade. Forge's source-matte
processing applied a one-pixel halo cleanup to edge noise before Pack preparation.
The cleaned result passed the unchanged `requireGameReady:true` gate. This is
technical validation, not a human visual approval. The first rejected generation
is not included in this public set.

## What the replay demonstrates

- Codex generated the monster, effect and arena artwork and authored the demo code.
- Forge prepared and validated the two Packs, registered their outputs and
  installed native SpriteFrames/scenes into Godot; these local Jobs made zero
  Provider requests.
- Godot supplies movement, targeting, health indicators and hit flashes. These
  behaviors are gameplay code, not additional generated animation frames.

The excerpt uses seconds 1.0–7.6 of an existing 18-second Godot Movie Maker
capture. MP4: 960 × 540, 30 FPS, no audio. GIF: 960 × 540, 15 FPS, looping.
No subtitles, framing overlays, speed changes or synthetic in-between frames
were added. The GIF loops the excerpt; the game state does not form a seamless loop.

[provenance.json](provenance.json) records source/output hashes, tool identity,
Job IDs and the earlier processing results. Raw stores and machine paths are
excluded. This README update reuses the completed capture; it does not rerun
generation or assert a new human review. Forge is free and open source; external
generation tools or accounts may have separate charges.
