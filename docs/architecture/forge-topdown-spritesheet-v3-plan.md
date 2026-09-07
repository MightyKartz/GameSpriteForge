# Forge topdown-spritesheet V3

## Decision

`topdown-spritesheet@3.0.0` is the first deliberately simple replacement for the
unsuccessful independent-frame experiments. It does not use ControlNet, a rig,
separate body parts, video generation, a Godot extension, or an animation model.
The image Provider authors one complete 2×2 four-phase sheet per animation and
Forge/Godot own everything after that image response.

```text
StyleLock + SubjectLock (+ optional equipment reference)
→ one 2×2 Provider image for an action
→ fixed row-major split
→ deterministic green/Alpha cleanup
→ foot-anchor alignment and shared normalization
→ consistency + direction/effect + equipment + silhouette + motion gates
→ Character Pack V2
→ external atlas
→ Godot AnimatedSprite2D + SpriteFrames + AtlasTexture
```

The four cells are always `0` top-left, `1` top-right, `2` bottom-left, and `3`
bottom-right. A walk sheet represents left contact, passing, right contact, and
passing. An idle sheet contains four subtle breathing phases. `walk_left` remains
a horizontal flip of `walk_right` in the Godot usage mapping.

## Provider contract

- Capability: `edit_image`; video capabilities are not required.
- References: Subject identity, Style board, and optional equipment identity.
- One square `1:1` output, exactly four equal cells, no dividers or labels.
- Exactly one complete full-body subject per cell with equal scale and margins.
- Native Alpha is used when advertised; otherwise the prompt requests flat
  `#00FF00` and Core performs deterministic cleanup.
- Provider/model/Style/Subject remain locked for the complete Job.
- Authorization targets are animation names, not individual frames.

A full Job therefore estimates four image requests and permits at most eight.
Single-direction validation estimates one and permits two. Automatic retries
replace the complete failed action sheet. A child `--stage still` retry requests
only the named action and reuses the other three verified actions by SHA-256.
`matting`, `loop`, and `consistency` replay issue no Provider requests.

## Hard gates

`animation-sheet@1.0.0` blocks malformed/non-square grids, empty cells, and any
foreground touching a cell boundary. Existing Character gates then validate:

- exact canvas, Alpha, one subject, no clipping and stable foot anchor;
- palette/scale/edge consistency;
- front/rear/right direction and forbidden detached/baked effects;
- hand/equipment contact and equipment presence/absence;
- temporal silhouette stability and residual/duplicate limbs;
- `motion-semantics@1.0.0` gait energy, four distinct walk phases, phase order,
  upper-body flicker, lower-edge ghosts and excess foot lobes;
- loop and ordinary Character quality before Pack export.

Repeated cells therefore cannot pass merely because they look consistent: a
walk with four identical cells fails the motion hard gate and no Pack is written.

## Godot delivery

Forge exports external PNG/atlas files. The installer creates only native Godot
resources: `SpriteFrames`, `AnimatedSprite2D`, and one `AtlasTexture` region per
frame with `filter_clip = true`. Idle runs at 4 FPS and walks at 5 FPS in V3;
Godot also consumes `frameDurationsMs` when a later timing profile supplies
non-uniform durations. No image pixels may be embedded in `.tres`/`.tscn`, and
each text resource remains below 1 MiB.

## Release status

The workflow is experimental. Offline fixture, Pack, retry, duplicate-frame,
Godot install, resource-size, and headless-load contracts pass. It must remain
opt-in until a separately authorized real-xAI single-direction probe succeeds,
followed by a four-direction Character gate. No real Provider request is part of
this implementation change.
