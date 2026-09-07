# Forge Character keyframe, timing, and equipment remediation

Status: implemented offline on 2026-08-08; real xAI acceptance remains separately gated.

## Decision

Forge keeps `topdown@1.0.0` as the compatible video workflow, but no longer treats a
uniform 12 FPS export as authoritative timing. Armed production characters should use
the opt-in `topdown-keyframes@2.1.0` workflow until benchmark evidence justifies a
default change.

The implementation has four boundaries:

1. `animation-timing@1.0.0` chooses the shortest strong fundamental cycle using closure
   and self-similarity prominence, then safely retimes its relative intervals into a
   versioned cadence profile. It records source timestamps and one playback duration
   per output frame; retiming outside 0.5–2.0 blocks export.
2. `hand-equipment-contact@1.0.0` creates an `EquipmentLockV1` for staff-like held
   equipment and blocks missing grip contact, side flips, grip drift, hand-region area
   outliers, and long thin finger-like protrusions.
3. Durable Provider authorization distinguishes model generation from transport. A
   private upload is auditable but cannot consume the second video model attempt.
4. `topdown-keyframes@2.1.0` generates four phase anchors before four in-betweens.
   In-betweens reference approved start/end frames plus a pose guide; frame retry is
   source-linked and immutable.

## Compatibility

- Existing `loop@2.0.0` reports deserialize because `timing` is additive and optional.
- Existing manifests without `frameDurationsMs` retain their FPS behavior.
- Existing authorization grants omit `maxTotalProviderOperations` and receive a
  conservative default of four Provider operations per model attempt.
- `topdown-keyframes@2.0.0` remains accepted and preserves the independent-frame
  reference contract.
- Pack V2 remains unchanged; the hand/equipment report is an optional Pack asset.

## Offline acceptance

- Fundamental period beats a longer harmonic under `walk` cadence.
- Non-uniform sampling durations sum exactly to the selected period.
- GIF centiseconds are derived from the same frame durations used by Godot.
- Stable staff grip passes; a one-frame thin finger protrusion blocks; unarmed prompts
  do not receive an equipment failure.
- One logical video repair can reserve upload + edit atomically while consuming one
  model attempt and two Provider operations.
- Fixture keyframe V2.1 creates 32 frames, records start/end dependencies, exports a
  Pack, updates the catalog, and retries one in-between with one Provider request.

## Remaining real gate

No real Provider request is authorized by this implementation. The first paid gate
should target only `walk_right` with `topdown-keyframes@2.1.0`, inspect the native-size
hand/staff crop, and verify exact Godot playback duration before expanding to four
directions.
