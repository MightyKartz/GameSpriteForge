# Character `walk_right` keyframe real acceptance — 2026-08-08

## Verdict

**Blocked correctly.** The bounded xAI run completed and remained within its
authorization, but the generated sequence is not a usable game animation. Forge
did not export a Pack or register an asset.

## Provider execution

- Job: `a1e0d861-d605-4e25-8f21-b8c21fa4a53e`
- Workflow: `topdown-keyframes@2.1.0`
- Provider/model: `xai` / `grok-imagine-image-quality`
- Authorization: `character-walk-right-keyframes-20260808`
- Scope: only `walk_right:frame:0` through `walk_right:frame:7`
- Estimate / maximum: 8 / 16 image edits
- Actual: 14 image edits
- Cost: 11,200,000,000 ticks, approximately USD 1.12
- Attempts: frames 3 and 4 passed their first local frame gate; the other six
  frames used the allowed second attempt.

All 14 ledger entries are settled at 800,000,000 ticks each. There were no
video, Subject, Style, icon, prop, portrait, terrain, or building requests.

## Visual findings

Only frames 3 and 4 reached the per-frame `game_ready` band. Frames 0, 1, 2,
5, 6, and 7 remained blocked. Native-size review confirms:

- the opaque beige/gray scene from the Style board leaks into frames 0 and 1;
- several frames contain T-pose or pointing silhouettes instead of a coherent
  rightward walk;
- frames contain transparent holes through the face, torso, cloak, and legs;
- the scarf changes between amber and blue-green;
- body width, pose, face, and equipment shift sharply between adjacent frames;
- the staff is copied inconsistently from the Style board even though the
  canonical Subject image contains no staff.

The deterministic reports agree with the visual review:

- `consistency@1.6.0`: blocked; edge, identity, background, and lower-body
  failures on six frames.
- `silhouette-temporal@2.0.0`: blocked; minimum adjacent core IoU 0.448,
  maximum contour distance 6.41 px, body-width variation 50.5%.
- `boundary-alpha-repair@1.0.0`: blocked; cleanup budget exceeded.
- `animation-quality@2.0.0`: blocked; no Pack export.

## Root cause

This is primarily a generation-contract failure, not a threshold calibration
problem. The three references carry conflicting semantic content:

1. Subject identity: clean transparent full-body Ayla, no staff.
2. Style board: opaque scene containing a faceless ranger with staff plus two
   unrelated props.
3. Pose guide: high-contrast skeletal/T-pose structure.

xAI treats all three as image-edit content. It has no API-level “style-only”
reference role, so Forge's logical `ReferenceRole::Style` label does not prevent
the board's background, staff, faceless head, or props from leaking into the
result. Odd-frame interpolation then propagates defects from the neighboring
anchors.

The hand/equipment report returned `equipmentKind: none` because the immutable
Subject prompt and canonical image do not declare a staff. This is internally
consistent, but it exposes a missing gate: Forge needs to detect *unexpected*
equipment copied from another reference, not only validate equipment explicitly
locked by the Subject.

## Godot and security

A QA-only Godot 4.6.3 project imports the eight external PNGs and plays them at
8 FPS (1,000 ms cycle). Headless import and scene load passed. It is deliberately
not a Pack installation or Catalog asset.

Credential and temporary-URL scans passed. No API key, OAuth token, Device Code,
Authorization header, temporary media URL, embedded Image, or `PackedByteArray`
was found in the Job or retained QA evidence.

## Required change before another paid run

Do not spend the two remaining requests. First replace the multi-object opaque
Style board reference with a content-neutral style swatch/texture reference, or
encode style into the prompt while using Subject identity plus a clean pose
target. Add an unexpected-equipment/content-leakage gate and reject pose-guide
pixel leakage before image retry. Then perform another bounded one-direction
acceptance under a new authorization.

## Offline implementation status

The requested change is now implemented as `topdown-keyframes@2.2.0`: Style is a
text/JSON descriptor rather than a Provider image reference, Pose uses a transparent
compact guide, equipment is explicit, and persistent undeclared staff content is a
hard failure. The fixture and zero-cost plan gates pass; a new real-xAI run has not
been performed. See
[`forge-character-reference-isolation-equipment-offline-2026-08-08.md`](forge-character-reference-isolation-equipment-offline-2026-08-08.md).
