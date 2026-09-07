# Forge topdown-grid V9 equipment and hand remediation

Date: 2026-08-11

## Problem

The first real V9 Direction Grid preserved the four views and transparent delivery shape, but it invented a staff, quiver, arrows, decorative cape marks, and inconsistent hand coverings even though the immutable request declared `equipment.kind = none`. The structural Direction Sheet and background reports were therefore necessary but not sufficient.

The gate must improve generation without encoding one ranger costume as the only valid character. Knights, animals, robots, flying subjects, tails, wings, robes, and asymmetrical designs remain valid inputs.

## Generation contract

`topdown-grid@9.0.0` now treats references by role:

- Subject identity is the content authority.
- Style controls rendering only; it cannot contribute inventory, anatomy, garments, accessories, hand state, or pose.
- `equipment.kind = none` explicitly requires empty hands and forbids newly introduced weapons, staffs, quivers, arrows, handheld props, gloves, or gauntlets absent from the Subject.
- `equipment.kind = staff_like` requires one declared item, stable handedness, visible grip contact, and no duplication, floating, redesign, or effects.
- Every idle cell must keep every support point planted; it cannot start the walk pose.

These are prompt constraints, not a subject-class ontology.

## Deterministic report

Every generated Direction Grid writes `direction-grid-appearance-report.json` using `direction-grid-appearance@1.0.0`. It contains:

- the existing versioned hand/equipment contact result;
- a canonical-derived exposed-hand evidence baseline when that evidence is available;
- per-node exposed-hand counts and mirrored side-view consistency;
- canonical-relative elongated exterior foreground evidence;
- a typed verdict and reason codes.

High-confidence undeclared held/detached equipment is a hard failure. Hand-color evidence and elongated exterior geometry are review evidence unless multiple independent signals establish a hard failure. This avoids treating capes, wings, tails, sleeves, or non-human anatomy as weapons.

The report path and SHA-256 are bound into `direction-grid-lock@1.1.0`. The Lock also binds Subject ID/revision/canonical SHA-256, equipment kind, camera profile, image model, Style revision, Provider profile, sheet SHA-256, and the four node hashes.

## Review and retry

Direction Grid output is always materialized with its contact sheet before the mandatory review stop.

- `game_ready` and `awaiting_review` may be explicitly accepted after visual inspection.
- `regenerate` and `blocked` cannot be accepted or used by action generation.
- Lock and report integrity are validated before an accepted review decision is written.
- Action generation validates the complete Subject/Style/equipment/camera/model closure before any Provider request.

A rejected Grid can be retried as one typed child Job:

```text
forge job retry --id <job-id> --item direction_grid --stage still --wait --json
```

The child:

- estimates one request and allows at most two attempts;
- inherits parent/lineage provenance;
- validates the source Job recipe, Lock artifact SHA, Provider manifest SHA, Subject, Style, equipment, camera, model, asset, and node hashes before spending;
- locally re-evaluates the rejected pixels and records `direction-grid-retry-evidence.json`;
- puts only deterministic reason codes into the new prompt;
- does not use the rejected Grid as an image reference;
- uses the immutable Subject and Style references again.

Legacy `direction-grid-lock@1.0.0` Jobs may be used only as a validated retry source. They cannot enter the action stage because they do not contain the V1.1 semantic closure.

## Gates

- consistent bare-hand fixture passes;
- single-hand covering drift becomes explicit review evidence;
- undeclared held or detached equipment blocks action authorization;
- human, animal, faceless, or covered inputs without a reliable exposed-hand baseline remain reviewable;
- approval/report/Lock tampering fails before Provider use;
- cross-Subject, equipment, camera, or model reuse fails with zero Provider requests;
- whole-sheet child retry records one-request estimate, lineage, evidence, and manifest provenance;
- the real failed Ayla Grid is re-evaluated locally without credentials or Provider calls.

