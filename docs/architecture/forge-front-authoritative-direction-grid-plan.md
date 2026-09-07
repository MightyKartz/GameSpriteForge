# Front-authoritative DirectionGrid remediation

Status: implemented and accepted offline; real execution requires a separately
reviewed 1/1 authorization.

## Decision

Four independently generated direction images are not a production route.
They permit identity, scale, silhouette, cape length and decorative trim to
drift independently. The approved `front_idle` is instead the sole visual and
garment-topology authority.

The remediation remains `topdown-grid@9.0.0` and is selected explicitly by
`directionGridCapeContract=front_authoritative_no_skirt_hem`. Keeping the
existing workflow version is safe because requests without this optional
contract retain the prior behavior and evidence profiles.

## Generation contract

1. Source is a succeeded, explicitly approved V9 DirectionGrid Job.
2. Source Job, Lock, approval and `front_idle` bytes are validated before a
   Provider request; the source is never mutated.
3. The Provider receives exactly one image reference with role
   `DirectionAnchor`: approved `front_idle`. A Style image is not sent.
4. One request produces one square 2×2 sheet: front, rear, screen-right,
   screen-left. Rear and side cells are rotations, not redesigns.
5. The front authority has no skirt-like cape lower hem. Rotations must not add
   a flared cape skirt, apron, tunic flare, lower gold piping or decorative
   border absent from the front.
6. The reference file is re-hashed after the Provider returns. A concurrent
   change fails before Lock/report evidence is written.
7. The Job stops at `awaiting_review`. It cannot create a Pack, catalog entry or
   Godot asset.

## Evidence and gates

The child writes `front-authoritative-direction-grid@1.0.0` evidence binding
the source Job, source Lock, authority path/SHA, one-reference order, cape
contract and request maximum. `DirectionGridLockV1` binds the typed Provider
producer, downstream Provider/profile/model, cape report path/SHA and four
final node hashes.

`cape-hem-consistency@1.0.0` now has two explicit contracts:

- `continuous_gold_lower_hem`: every view must meet the minimum lower gold trim
  ratio.
- `front_authoritative_no_skirt_hem`: every view must remain at or below the
  maximum lower gold-to-cape adjacency ratio of 0.004.

Appearance, direction, equipment and background gates remain unchanged. Cape
failure is a hard `direction_grid_regeneration_required` result and cannot be
overridden by manual approval. Reuse re-hashes the cape report and every bound
node.

## Budget and authorization

The Plan estimate and maximum are both one image request. Real xAI execution
requires a new, empty-ledger grant with exactly one model, target
`direction_grid`, one request, one Provider operation, a 1.4B-tick ceiling,
the source lineage and exact Plan recipe/input hashes. No retry or model
fallback is reserved.

## Acceptance sequence

1. Offline unit and fixture contracts.
2. Plan-only against the intended approved real source.
3. Separate user authorization for one real request.
4. Native review of the four extracted full-resolution nodes and cape report.
5. Only after explicit approval may a later animation Job reuse the Lock.
