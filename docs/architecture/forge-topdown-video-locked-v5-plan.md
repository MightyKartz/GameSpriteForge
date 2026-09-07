# Forge `topdown-video-locked@5.0.0` implementation plan

Status: implemented behind the existing `consistency-v2` feature; experimental
until real-provider acceptance.

Date: 2026-08-09

## Decision

V5 keeps the strongest parts of the two preceding experiments:

- V4 owns one immutable four-view DirectionLock created from SubjectLock plus
  text-only Style metadata.
- The video model owns continuous motion from one direction anchor.
- Forge owns background cleanup, candidate extraction, loop selection, shared
  normalization, quality, Pack provenance, and native Godot delivery.

V5 does not generate per-direction stills, use reference-to-video, edit a failed
video, install a Godot extension, create a rig, split the character into parts,
or use ControlNet.

## Workflow

```text
StyleLock text metadata + SubjectLock image
→ one 2×2 DirectionLock sheet
→ fixed split: front / rear / right / left
→ deterministic background and thin ground-line cleanup
→ one shared DirectionLock scale + translation-only foot alignment
→ immutable DirectionLock
→ action mapping: front→idle/down, rear→up, right→right
→ exact chroma-green local composite
→ one image-to-video request per selected action
→ ≤12 FPS / ≤96 full-clip candidates
→ chroma matting + candidate cleanup
→ loop@2.0.0 closed-range selection
→ exactly eight frames from [start, end-boundary)
→ one Character-wide scale + per-frame translation-only body/foot anchor
→ semantic, equipment, silhouette, loop and hard geometry gates
→ `.gsfpack` V2
→ external PNGs + native Godot `SpriteFrames` / `AnimatedSprite2D`
```

The left runtime direction remains a horizontal flip of `walk_right`. The left
DirectionLock entry is retained for symmetry review and future asymmetric gear.

## Provider boundary

The DirectionLock request uses `EditImage`. Each action uses `ImageToVideo` with
exactly one local first-frame image. The first-frame image is produced locally by
alpha-compositing the cleaned direction anchor onto `#00FF00`; this operation does
not resize, redraw, or call a Provider.

The action prompt requires one subject, fixed camera/facing/framing, a flat green
background, an in-place closed cycle, no ground/floor/shadow, no duplicate limbs
or smear, and no new emission or effect. Idle has a separate subtle-breathing
contract and is not asked for gait contact poses.

V5 output resolution is 480p because the final canvas is 64–512px and Forge owns
the final shared resampling. The input is square and the default duration remains
four seconds.

## Ground-line cleanup

`keyframe-background-cleanup@1.2.0` detects thin, wide lower-canvas foreground
rows whose pixels are predominantly unsupported vertically. It removes only the
unsupported 1–3px band, protects boots/capes with vertical foreground support,
and blocks residual ground lines. The same cleanup is applied when upgrading a
compatible V4 DirectionLock and to V5 candidate/selected frames.

## Loop and alignment contract

Forge does not choose eight timestamps across the whole video. It extracts the
whole clip, mattes and provisionally aligns all candidates, searches for a closed
start/end-boundary pair, excludes the boundary frame, and samples eight indices
inside the selected interval. `outputFrameIndices` and output SHA-256 values must
match the exported PNGs exactly.

Normalization computes one scale that fits every selected frame in every action.
All frames receive that same scale. X uses the body center and Y uses the body/foot
anchor; only translation varies. Canvas top and bottom are fixed, while legitimate
head/body bob remains visible. Per-frame resize, stretch, crop, or hidden Godot
offsets are forbidden.

## Requests and retry

| Scope | Expected | Maximum |
| --- | ---: | ---: |
| New full Character | 5 | 10 |
| New one-action validation | 2 | 4 |
| Full Character with compatible reused DirectionLock | 4 | 8 |
| One video retry with reused DirectionLock | 1 | 2 |
| Loop/matting/consistency replay | 0 | 0 |

Authorization targets are `direction_lock` and `<animation>:video`. There is no
`<animation>:still` target. V5 rejects `still` and `frame` retry stages because
the DirectionLock is immutable. Provider/model/profile never switch silently.

## Release gate

Offline fixture success does not promote V5. Promotion requires separately
authorized real xAI probes for `walk_up` and `walk_right`, followed by a full
four-action run. All actions must be `game_ready`, export 32 external PNG frames,
pass Pack validation, install/load in Godot 4.6.x, show correct direction and
natural motion at native size, and pass credential/temporary-URL scans. Until
then `topdown-video@2.0.0` remains stable and V5 remains explicit opt-in.
