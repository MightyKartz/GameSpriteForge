# Forge V17 24-frame review delivery plan

Date: 2026-08-19

Status: implemented and verified; human review remains pending.

## Scope

1. Bind a human-review record to the exact v17 source, prompt, replay summary,
   24 PNG hashes, and 24 native durations without fabricating approval.
2. Extend source-cycle, manifest, candidate Pack, Pack validation, and Godot
   installation contracts to preserve a reviewed 24-frame cycle.
3. Exercise the installed asset in a real `CharacterBody2D` movement scene
   with a shared pivot, collision shape, ground reference, and exact timing
   assertions.

## Guardrails

- v17 frames remain immutable;
- review status stays `pending` until the user explicitly supplies all checks;
- a pending review may create only a candidate Pack with
  `productionEligible=false`;
- 8/10/12 source-cycle reports remain backward compatible;
- 24-frame reports use `source-cycle-sampling@1.1.0`;
- Godot resources use external PNG textures and remain below 1 MiB;
- no provider request, image edit, per-frame translation, scaling, or Pack
  promotion occurs in this work.

## Implemented route

`v17 replay → animation-human-review@1.0.0 (pending) → 24-frame candidate
.gsfpack → Godot installer → CharacterBody2D movement review`.

Promotion is intentionally out of scope until motion, placement, edge,
laterality, pivot, and collision checks are explicitly approved by the user.

