# Forge xAI Image 2.0 candidate plan

Status: implemented and accepted offline on 2026-08-14. No paid Image 2.0
generation has been authorized or executed by this plan.

## Decision

`grok-imagine-image-2.0` is an explicit candidate, not the Forge image
default. `grok-imagine-image-quality` remains the incumbent until a bounded
real comparison and native review justify a route change.

The candidate is admitted only as one `topdown-grid@9.0.0` DirectionGrid
comparison child:

- source is a succeeded, explicitly approved `xai/default` V9 DirectionGrid
  produced by the Image Quality incumbent;
- source Job recipe, DirectionGridLock, approval, Provider manifest, four
  direction PNGs, StyleLock and SubjectLock are hash-bound and revalidated;
- when the incumbent source was itself a reviewed semantic retry, its
  hash-bound correction-code evidence is reused verbatim so incumbent and
  candidate receive the same deterministic prompt contract; an initial source
  keeps the initial no-correction prompt;
- target is exactly `direction_grid` at `still` stage;
- model is exactly `grok-imagine-image-2.0`;
- expected/maximum requests are 1/1, attempts are fixed to one, and model
  fallback is forbidden;
- the approved source is never mutated and its consumed authorization is not
  inherited;
- output stops at native DirectionGrid review and cannot create a Pack,
  catalog entry, or Godot installation.

Any fresh Character run, action frame, expanded direction, two-attempt route,
different Provider/profile, or full production use of Image 2.0 fails Plan
validation with `xai_image2_candidate_scope_forbidden`.

## Authorization boundary

Real execution requires a new physically empty durable authorization bound to
the approved source lineage and exact pending Plan hashes. Its complete scope
is:

- provider/profile: `xai/default`;
- model: `grok-imagine-image-2.0` only;
- target: `direction_grid` only;
- request count: 1;
- Provider model operations: 1;
- conservative cost ceiling/reservation: 1,400,000,000 ticks.

CLI preflight and Core Runner independently verify the model, target, counts,
cost, lineage, recipe hash, input fingerprint, authorization ID and immutable
manifest digest. A Provider that merely reports `id=xai` but cannot prove the
scope fails before its first request.

## Evaluation policy

The candidate grid must be compared against its approved incumbent source at
native resolution. Minimum review covers front/rear/left/right direction,
identity and face, hood/full cape, empty hands/equipment, planted neutral
stance, scale/baseline consistency, transparent/background cleanup, and new
semantic residue. Passing one grid permits only a further canary proposal; it
does not switch the default automatically.
