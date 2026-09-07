# topdown-grid@9.4.0 platform-safe gait remediation

Status: implemented; offline and single-frame real validation accepted. Pack,
other directions, video and Godot remain outside the authorized scope.

## Decision

The real V9.3 `walk_down` probe preserved identity, clothing, empty hands and gait laterality, but native review rejected frame 2 because the bright wide V1.1 sole control became a gray platform/skate beneath the planted boot. V9.3 and its real evidence remain immutable.

V9.4 is a single child remediation of that rejected V9.3 candidate:

- byte-reuse frames 0, 1 and 3;
- generate only `walk_down:frame:2` from the approved `front_idle` DirectionAnchor and PoseStructure V1.2;
- never send the rejected V9.3 frame to the Provider;
- use exactly one image edit, no retry, video, Pack or Godot delivery;
- reserve a new source-scoped child slot and a new exact independent authorization.

## PoseStructure V1.2

`grid-pose-structure@1.2.0` preserves screen-side support/swing control through bright/thick versus dark/thin leg ink. The planted endpoint is a compact dot/vertical tick. A deterministic validator rejects a reintroduced horizontal contact bar. The prompt explicitly forbids platforms, boards, skates, slabs, floors and grayscale guide materials.

## Platform hard gate

`footwear-platform@1.0.0` is color-independent. For each screen half it measures a thin bottom shelf against the leg stem above it. A shelf is blocked when it is at least 24% of body width, at least twice its stem width, projects by at least 12 pixels and remains vertically thin. The real rejected frame measures 46 px against a 16 px stem (2.875×); the other three real frames remain ready.

V9.4 persists the per-frame/action metrics as `walk_down-footwear-platform.json`, binds its SHA and verdict into the Action Report, Provider manifest, WorkflowGraph, Job artifacts and action-quality implementation profile.

## Source closure

The source must be the immutable V9.3 validation candidate with:

- lifecycle `awaiting_review` and code `grid_keyframe_validation_review_required`;
- workflow `topdown-grid@9.3.0`, `walk_down`, frame-stage retry `[2]`;
- Action Report `@1.3.0`, frame 2 `asymmetric_guide_fresh_retry`, PoseStructure `@1.1.0`;
- hash-closed Job artifacts, Provider manifest and WorkflowGraph;
- current platform gate blocking exactly frame 2 and no other frame.

Any source, frame, guide, manifest or graph mutation fails before a Provider request.
