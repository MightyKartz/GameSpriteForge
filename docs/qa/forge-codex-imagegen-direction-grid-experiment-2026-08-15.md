# Forge Codex built-in imagegen DirectionGrid experiment — 2026-08-15

Result: visually promising, rejected for native-alpha delivery.

## Scope

The experiment used the approved V9 DirectionGrid contact sheet from Job
`9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad` as the sole visual reference. It did
not modify that Job, its DirectionGrid Lock, the Forge Provider configuration,
Pack output or a Godot project.

Codex built-in `image_gen` was intentionally treated as an external preview
generator. Its interface does not expose a stable Forge Provider model ID,
durable authorization ledger or API cost record, so this experiment does not
claim to evaluate a named OpenAI API model.

## Attempts

Attempt 1 regenerated one 2×2 sheet with front, back, strict screen-right and
strict screen-left idle views. The four views are directionally correct, empty
handed and visually close in scale and baseline. The green hood, full cape,
yellow scarf and brown leather outfit are present. Some face and clothing
details were redrawn, so strict approved-identity equivalence remains
unproven.

The generated PNG was `1254×1254` RGB with no alpha channel. The visible
white/light-gray checkerboard was baked into the pixels rather than being a
viewer transparency preview.

Attempt 2 made one targeted edit: remove only the checkerboard and preserve all
character pixels. It again returned a `1254×1254` RGB PNG without alpha and
retained the baked checkerboard. Per the frozen stop condition, no additional
generation was attempted.

## Gate result

`native_alpha_missing` and `checkerboard_baked_into_pixels` are hard failures.
The experiment therefore stopped before Forge frame extraction, DirectionGrid
Lock, Pack or Godot delivery. A deterministic background-removal experiment
could evaluate whether the character drawing is salvageable, but such a result
must be recorded as Forge post-processing and must not be presented as native
model transparency.

Evidence is stored in
`generated-assets/experiments/codex-imagegen-direction-grid-20260815/` with
input/output hashes and both untouched generated PNGs.

## Decision

Do not replace the current Forge image Provider with Codex built-in
`image_gen`. Keep it as a fast external concept and comparison tool. The next
useful implementation, if desired, is an explicit external-candidate import
Job with deterministic matting, SHA-256 provenance and the existing
DirectionGrid gates. A production OpenAI route should instead use a named API
model behind the normal Forge Provider authorization and usage contracts.

## Follow-up

The external-candidate import and deterministic matting route was subsequently
implemented. Job `01447a14-791d-4ced-8f5f-990076e6b200` successfully converted
the second raw output to real alpha with zero Provider requests and stopped at
native DirectionGrid review. See
`docs/qa/forge-external-direction-grid-import-real-2026-08-15.md`.
