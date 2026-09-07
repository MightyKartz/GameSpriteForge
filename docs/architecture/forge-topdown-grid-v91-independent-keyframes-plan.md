# Forge `topdown-grid@9.1.0` independent-keyframe implementation plan

Date: 2026-08-12  
Status: implemented and offline-accepted  
Real Provider scope: **forbidden in this implementation goal**

## Decision

Keep the approved `topdown-grid@9.0.0` DirectionGridLock as the immutable
multi-view source of truth, but replace the unproven 2x2 Action Grid stage with
four independently generated keyframes per direction.

`topdown-grid@9.1.0` is a new workflow version. It does not change V9.0 Job,
Lock, report, replay, or Pack semantics and it cannot generate a Direction
Grid. The first stage must be an explicitly approved V9.0 Direction Grid Job.

```text
approved topdown-grid@9.0.0 DirectionGridLock
├── front_idle -> walk_down frames 0..3
├── back_idle  -> walk_up frames 0..3
├── right_idle -> walk_right frames 0..3
└── left_idle  -> walk_left frames 0..3
```

The four phases are fixed and ordered:

1. left contact;
2. left passing;
3. right contact;
4. right passing.

## Generation contract

- Every first-attempt frame is an independent edit of its direction idle.
  No generated walk frame is the appearance parent of another frame.
- The DirectionAnchor is the only image reference for a fresh frame. Style,
  phase, camera, equipment, and background constraints remain text metadata.
- No pose-guide image is supplied. This prevents guide colors or skeleton
  geometry from leaking into the sprite.
- A static hard failure retries fresh from the DirectionAnchor and never uses
  the rejected pixels as an edit target.
- After four statically valid frames exist, `motion-semantics@1.1.0` checks the
  ordered cycle. A motion-only retry may edit only recommended failed frames,
  with the failed frame as `EditTarget` and the immutable DirectionAnchor as
  `DirectionAnchor`.
- Each Provider output is materialized inside its exact attempt directory,
  validated, cleaned, normalized, SHA-256 bound, and quality checked before it
  becomes an accepted frame.

## Public and budget contract

- Feature: existing unreleased `grid-generation`; default release surface is
  unchanged.
- Workflow: `topdown-grid@9.1.0`.
- Only `--direction-motion-stage complete` is valid.
- `--image-lock-job` must name an approved V9.0 Direction Grid Job.
- Validation-only requires exactly one walk direction and never exports a
  partial Pack.
- One-direction validation: 4 expected / 8 maximum image edits.
- Full four-direction generation: 16 expected / 32 maximum image edits.
- Video requests are always zero.
- Authorization targets are frame-specific:
  `<animation>:frame:<0..3>`.

## Reports and replay

`grid-keyframe-action-report@1.0.0` records:

- the action and immutable direction anchor hash;
- four frame indices, phases, attempts, source method, paths, and hashes;
- Provider-request and reuse state;
- motion-semantics and equipment report paths/hashes/verdicts;
- the recommended and actually retried frames.

The Job writes a typed `WorkflowGraphV1`. A frame retry creates a child Job,
reuses every unselected accepted frame byte-for-byte, and invalidates the
selected frame plus action-level quality and downstream Pack/Godot nodes.
Source Jobs and Packs remain immutable.

## Pack and Godot

A complete V9.1 Job reuses the four approved idle PNGs and delivers four
four-frame walk animations through the existing Character Pack and Godot
pipelines. Portable direction Lock/approval, per-action reports, and the
sanitized Provider manifest are copied into the Pack as provenance. Godot
continues to use external PNG/atlas resources; no embedded image bytes are
introduced.

## Offline acceptance

The implementation goal is complete only when all of the following pass
without credentials or real Provider requests:

1. V9.1 rejects missing, unapproved, tampered, cross-Subject, cross-Style,
   cross-equipment, cross-camera, cross-model, and non-V9.0 Direction locks.
2. Plan estimates are exactly 4/8 for one direction and 16/32 for a full run;
   authorization targets are frame-specific and contain no video target.
3. Each fresh frame references only the matching DirectionAnchor; frames do
   not reference one another.
4. Four ordered phases pass; collapsed/out-of-order phases fail closed.
5. Static retry starts fresh; motion retry changes only recommended frames.
6. Explicit child frame retry reuses the other three frames byte-for-byte.
7. Validation-only stops for review and exports no Pack.
8. Full fixture Pack validates and installs in Godot 4.6.x.
9. Workflow graph, Pack provenance, hashes, credentials scan, temporary URL
   scan, and embedded-image scan pass.
10. Default-feature CLI help and all V1-V9 focused contracts remain unchanged.

Real xAI acceptance is a later, separately authorized goal. Its first paid
gate must be `walk_down` validation-only with 4 expected / 8 maximum image
requests. No other direction may be purchased before native-size review passes.
