# Forge `topdown-frames@4.0.0` implementation plan

Status: implemented behind the existing `consistency-v2` build feature; experimental until real-provider acceptance.

Date: 2026-08-09

## Decision

Forge V4 uses only an image-edit Provider and Godot's native frame animation resources. It does not use image-to-video, ControlNet, a rig, character parts, a Godot extension, or model-generated animation metadata.

The Provider authors complete raster sprites. Forge owns the cardinal view contract, timeline order, deterministic slicing, background cleanup, shared-scale alignment, quality gates, retry boundaries, Pack provenance, and Godot delivery.

## Workflow

```text
StyleLock text metadata + SubjectLock image
→ one 2×2 DirectionLock sheet
→ fixed split: front / rear / right / left
→ cleanup + one shared scale + foot-anchor alignment
→ direction, framing, identity and equipment gates
→ immutable DirectionLock
→ one 2×2 action sheet for each selected action
→ fixed split: phase 0 / 1 / 2 / 3
→ cleanup + one shared scale + translation-only alignment
→ onion-skin overlays and report
→ direction / consistency / geometry / motion / equipment / silhouette gates
→ `.gsfpack` V2
→ external PNGs + Godot `SpriteFrames` + `AnimatedSprite2D`
```

The four animation timelines are fixed:

- `idle`: neutral, inhale, neutral, exhale; 4 FPS.
- `walk_up`, `walk_right`, `walk_down`: left contact, passing, right contact, passing; 6 FPS.
- Godot continues to derive the left-facing runtime animation by horizontally flipping `walk_right`.

## Reference isolation

The first request creates the four DirectionLock views from:

- `subject_identity`
- optional `equipment_identity`
- StyleLock as text metadata only

Each action request uses:

- exactly one `direction_anchor` selected by Forge
- optional `equipment_identity`
- StyleLock as text metadata only

The Style board image is deliberately excluded from both action requests and targeted frame repair. This prevents a style illustration containing a staff, glow, portrait crop, or three-quarter camera from becoming an accidental semantic subject reference. xAI currently receives ordered images, so Forge must make the image set itself unambiguous rather than assume that internal reference-role labels are interpreted by the model.

## Request budget

| Operation | Expected | Maximum |
|---|---:|---:|
| New full Character | 5 | 10 |
| One-action validation with new DirectionLock | 2 | 4 |
| Child action retry with reused DirectionLock | 1 | 2 |
| Child single-frame retry | 1 | 2 |
| Matting / consistency / loop replay | 0 | 0 |

A request is authorized by a concrete target:

- `direction_lock`
- `idle`, `walk_up`, `walk_right`, or `walk_down`
- `<animation>:frame:<0-3>` for targeted repair

## Shared-scale and onion-skin contract

The initial four cells are never independently resized. Forge measures the four foreground bodies, computes one median-derived scale, applies that scale to the whole collection, and then uses only translation to align the center and foot baseline.

`onion-skin@1.0.0` records:

- the shared scale;
- source body-height ratios;
- per-frame translation;
- normalized center and foot drift;
- output hashes;
- four previous/current cyan overlays.

It blocks empty foreground, cropping, source scale drift above 20%, center drift above 2 px, or foot drift above 2 px. These checks are additive to the existing hard geometry, direction, equipment, motion, silhouette, Alpha, consistency, and Pack gates.

## Targeted frame repair

`forge job retry --id <job> --item <animation> --frame <0-3> --stage frame --json` creates a child Job. It reuses the immutable DirectionLock and all source frames, sends only the selected cell plus its previous accepted frame as an onion-skin continuity reference, and then replays local gates.

Unselected PNGs are copied byte-for-byte. Source Jobs are never modified. Provider usage, authorization target, input hashes, changed-frame indices and lineage remain reconstructable from the child manifest and JobStore.

## Pack and Godot

The Character Pack remains V2 for backward compatibility. It adds optional files and metadata:

- `animation-sheet-report.json`, including onion-skin reports;
- `character-direction-lock.json`;
- `assets/direction-lock/{front,rear,right,left}.png`;
- workflow and Provider provenance.

DirectionLock paths are rewritten to Pack-relative paths. No Job path, token, authorization header, Device Code, temporary URL, or credential is portable.

Godot installation uses the existing external PNG import path and native `SpriteFrames`/`AnimatedSprite2D`. Text resources remain below 1 MiB and contain no embedded `Image`, `PackedByteArray`, or `ImageTexture.create_from_image` payload.

## Compatibility and release gate

- `topdown-spritesheet@3.0.0` remains unchanged and readable.
- Video, keyframe and keypose workflows remain readable and runnable.
- `topdown-frames@4.0.0` is experimental and is not the default workflow.
- Real xAI acceptance needs separate user authorization and budget.
- Promotion requires multiple real Characters with all four actions `game_ready`, zero hard-failure Pack exports, visual review of direction and motion, and Godot playback acceptance.

