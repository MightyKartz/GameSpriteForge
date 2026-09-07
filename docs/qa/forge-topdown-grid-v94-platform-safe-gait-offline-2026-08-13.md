# Forge `topdown-grid@9.4.0` platform-safe gait offline acceptance

Date: 2026-08-13  
Verdict: **offline implementation accepted; no real Provider request authorized or executed**

## Scope

V9.4 remediates only the rejected V9.3 `walk_down` frame 2. It preserves the
approved DirectionGridLock and byte-reuses frames 0, 1 and 3. No video, Pack,
catalog or Godot path is enabled.

## Implemented contract

- `grid-pose-structure@1.2.0` retains viewer-space bright/thick support and
  dark/thin swing legs, but the grounded endpoint is compact and contains no
  horizontal sole/floor/platform primitive.
- Its validator rejects unsupported ink, wrong screen side, insufficient
  lift, missing compact contact, a contact span wider than 1/16 canvas, and a
  floor-like line.
- `footwear-platform@1.0.0` measures each screen-half bottom shelf against the
  leg stem above it; it is color-independent and blocks a wide, projecting,
  vertically thin platform/skate silhouette.
- Action Report `grid-keyframe-action-report@1.4.0` requires exactly frame 2
  `platform_safe_guide_fresh_retry` with V1.2, exactly three byte-reuse frames,
  and a game-ready footwear report/hash.
- the selected WorkflowGraph node is `grid-structured-keyframe@1.3.0`;
  reused nodes remain `grid-byte-reuse@1.1.0` with source-sprite provenance.
- Plan, staging and Core execution use a new source-scoped child slot and the
  same exact independent 1/1/1.4B authorization boundary as V9.3.

## Real failure replay

Read-only replay against V9.3 Job
`54b278d7-fdf3-4d39-9516-83ec5964ad63` closed its Job/recipe/artifact,
Action Report, Provider manifest, frame/guide and WorkflowGraph hashes. The
new gate produced:

- frame 0: ready, shelf/stem max 1.20×;
- frame 1: ready, shelf/stem max 1.1875×;
- frame 2: **blocked**, viewer-right shelf 46 px / stem 16 px = **2.875×**;
- frame 3: ready, shelf/stem max 1.2143×.

Thus the remediation source recommendation is uniquely `[2]` without adding
mutable human-approval state to the historical Job.

## Offline fixture

The end-to-end fixture constructs a hash-closed V9.2 failure, produces one
V9.3 frame-2 semantic platform leak, then stages V9.4. It verifies:

- exactly one edit and ordered references `DirectionAnchor, PoseStructure`;
- prompt contains V9.4/no-sole/no-platform contract and omits the V1.1 wide
  sole legend;
- frames 0/1/3 match V9.3 SHA-256 values;
- frame 2 uses V1.2 and becomes platform-gate ready;
- footwear report is a Job artifact and no `.gsfpack` exists.

No real credential, network request, Provider operation, authorization ledger,
source Job, Pack or Godot project was modified by this acceptance run.
