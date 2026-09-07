# Character reference isolation and equipment V2.2 offline acceptance

Date: 2026-08-08

## Verdict

The implementation is accepted at the focused fixture-contract level. No real xAI
request was made, so this report does **not** promote the keyframe workflow to stable
and does not claim that the visual defect is fixed in real model output.

## Implemented change

- V2.2 never sends the multi-object Style board as a Provider image reference.
- Anchors use the exact Subject canonical as `edit_target`; in-betweens use adjacent
  approved frames.
- Style becomes `character-style-descriptor@1.0.0` prompt/JSON metadata and remains in
  WorkflowGraph provenance and cache inputs.
- `topdown-poses@1.1.0` uses a transparent compact guide with arms close to the body.
- Character V2 requires an explicit `none` or `staff_like` equipment contract for the
  repaired workflow; a staff requires a fingerprinted local reference image.
- Both the asset schema and the durable request reject an omitted V2.2 equipment
  declaration; compatibility defaults are limited to historical Job deserialization.
- `hand-equipment-contact@1.1.0` blocks persistent undeclared staff-like content as
  `unexpected_held_equipment`.
- Single-direction validation requires V2.2 and still guarantees no partial Pack or
  Catalog registration.

## Focused evidence

The complete fixture keyframe contract passed. It covers 32 typed frame nodes, actual
Provider reference observations, eight-frame `walk_right` validation, Pack/Catalog,
single-frame retry, zero-request local consistency replay, and review-only candidate
behavior. Provider observations prove that V2.2 keyframe edits contain no Style role;
the validation anchor is exactly `edit_target + pose_structure`.

The exact fixture keyframe Pack was installed through Forge's Godot plan/execute path
into Godot 4.6.3. External texture loading and headless scene load passed; generated
`.tres`/`.tscn` files stayed below 1 MiB and contained no embedded Image or
`PackedByteArray`.

Equipment tests cover a stable staff, malformed finger-like protrusion, a real unarmed
silhouette, and persistent undeclared staff content. All four passed.

The real xAI plan-only probe returned:

- workflow/model: `topdown-keyframes@2.2.0` / `grok-imagine-image-quality`
- animation: `walk_right`
- estimate/maximum: 8/16 image edits
- cache hits: 0
- real requests executed: 0
- partial Pack/Catalog: prohibited by the operation contract

Machine evidence is in
[`artifacts/forge-character-reference-isolation-offline-20260808/summary.json`](artifacts/forge-character-reference-isolation-offline-20260808/summary.json).

## Full regression gates

The final tree passed:

```text
git diff --check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/test-cli-product.sh
scripts/test-stage3-static.sh
```

This includes 180 Core unit tests, 20 Pack tests, 32 Provider unit tests, CLI tests,
Character video and V2.2 keyframe contracts, Stage 3 static assets, Icon, World, Pack,
Godot, retry, cache, authorization, and OAuth contract coverage. The retained offline
evidence contains no credential/Bearer marker, temporary media URL, or embedded Godot
image data.

## Remaining gate

Run a fresh, separately authorized, bounded V2.2 `walk_right` real-xAI validation.
Promotion requires all eight frames to pass identity, palette, Alpha/background,
silhouette/anatomy, pose-leakage, and equipment-state checks. A correct block remains a
valid safety result but is not a usable animation. Only a visual pass should authorize
the later 32-frame Character run.

## Follow-up real result

The user subsequently authorized a complete four-direction V2.2 run. It used 61 image
edits and was blocked with only 8/32 frames `game_ready`. Style-board staff leakage was
eliminated, but direction locking, transparent-background reliability, identity, and
lower-body consistency still failed. See
[`forge-character-keyframes-v22-real-acceptance-2026-08-08.md`](forge-character-keyframes-v22-real-acceptance-2026-08-08.md).
