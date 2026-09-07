# Character reference isolation and equipment contract V2.2

Date: 2026-08-08  
Status: implemented and accepted offline; bounded real-xAI revalidation remains separate.

## Problem

The `topdown-keyframes@2.1.0` real `walk_right` probe proved that an API request
cannot make an image "style only" by attaching Forge's internal
`ReferenceRole::Style` label. xAI receives an ordered image list, so the opaque
multi-object Style board was also usable as content. Its staff, background, props,
and faceless ranger leaked into Character frames. The old white, horizontal-arm Pose
guide separately encouraged panels, T-poses, and colored guide pixels.

The Subject Lock did not declare a staff. The previous hand/equipment gate validated
only equipment inferred as expected from a prompt, so persistent equipment copied from
another reference was not a hard failure.

## V2.2 contract

`topdown-keyframes@2.2.0` keeps old Jobs readable but uses a new paid request boundary:

```text
StyleLock board
  -> character-style-descriptor@1.0.0 (text, palette, provenance only)

anchor frames 0/2/4/6
  -> Reference 1: exact Subject canonical as edit_target
  -> Reference 2: optional equipment_identity
  -> final reference: transparent topdown-poses@1.1.0 pose_structure

in-between frames 1/3/5/7
  -> previous approved anchor
  -> next approved anchor
  -> transparent pose_structure
```

The Style board SHA remains in the descriptor and WorkflowGraph for reproducibility,
but the board path is absent from every V2.2 Provider image-edit request. Style intent
is sent as normalized prompt metadata: rendering prompt, perspective, lighting,
outline, and dominant palette.

`topdown-poses@1.1.0` has a transparent canvas, compact vertical body, arms below the
shoulders, grounded feet, and rare cyan guide ink. The old V1 guide remains only for
historical workflow replay.

## Explicit equipment

`CharacterAssetSpecV2.equipment` is a versioned generation input:

```json
{ "kind": "none" }
```

or:

```json
{
  "kind": "staff_like",
  "referenceImage": "references/ayla-staff.png"
}
```

- `none` forbids an equipment image and instructs the Provider not to add a staff,
  wand, spear, weapon, tool, glow, or detached object.
- `staff_like` requires a clean local PNG that is fingerprinted during planning and
  sent as `equipment_identity` without changing Provider/model.
- Older Character workflows reject an explicit equipment reference so callers cannot
  assume a lock that the workflow will silently ignore.
- The durable request records `equipmentExplicit: true`. V2.2 planning rejects a
  missing flag even when a low-level caller bypasses the public asset schema; the
  default exists only to deserialize historical Job recipes.

`hand-equipment-contact@1.1.0` adds a non-overridable
`unexpected_held_equipment` result when a Character declared as unarmed contains a
persistent staff-like silhouette. The report schema continues to accept frozen V1.0
reports.

## Retry, cache, and release behavior

- A validation animation requires V2.2, plans eight edits, and caps the run at 16.
- Each failed frame may be retried once; accepted sibling hashes remain immutable.
- `matting`, `consistency`, local graph replay, and Godot installation make no Provider
  request.
- Cache keys include workflow/provider implementation, Provider/model, Style and
  Subject revisions, equipment kind/reference hash, pose hash, descriptor hash, and
  all dependent frame hashes.
- A validation-only direction never exports a partial Pack or updates the Catalog.
- A hard content, Alpha, silhouette, background, pose, or unexpected-equipment failure
  cannot be promoted by manual review.

## Compatibility and real gate

- `topdown-keyframes@2.0.0` and `2.1.0` remain deserializable for historical Jobs and
  provenance-compatible replay.
- New paid single-direction acceptance is allowed only with `2.2.0`.
- V2.2 remains experimental until a fresh bounded xAI `walk_right` run proves visual
  identity, anatomy, palette, transparent background, and equipment state. Offline
  fixture success is not represented as real-model success.
