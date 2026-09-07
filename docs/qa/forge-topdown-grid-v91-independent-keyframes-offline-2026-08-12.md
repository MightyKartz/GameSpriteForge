# Forge V9.1 independent-keyframe offline acceptance

Date: 2026-08-12  
Workflow: `topdown-grid@9.1.0`  
Provider: fixture only  
Real requests: **0**

## Result

Implemented and accepted the V9.1 hybrid workflow. It reuses an explicitly
approved V9.0 DirectionGridLock and generates four independent ordered walk
keyframes from each mapped direction idle. It does not generate an Action Grid
or call a video model.

## Verified contracts

- One-direction validation is 4 expected / 8 maximum image edits, stops in
  `AwaitingReview`, and exports no Pack.
- Full generation is 16 expected / 32 maximum image edits and 0 videos.
- Every fresh frame references only its `DirectionAnchor`; no walk frame is a
  generation parent of another walk frame.
- Phase order is left contact, left passing, right contact, right passing.
- Static retry starts fresh from the DirectionAnchor. Motion-only retry stores
  an immutable edit-target snapshot plus the DirectionAnchor.
- Child frame retry authorizes only the selected frame, records the replaced
  frame hash, and reuses all other frame bytes unchanged.
- Failed motion preserves the action report, Provider manifest, WorkflowGraph,
  usage ledger, retry advice, and maximum-request evidence.
- Complete Pack provenance uses relative paths, includes portable Direction
  Grid Lock/approval/appearance report, four action reports, a sanitized
  Provider manifest, and validates every frame and node SHA-256.
- A one-pixel Pack mutation is rejected by the Pack validator.
- Eight named animations install through the existing explicit-four-direction
  Godot contract. Godot 4.6 headless import and project load passed when the
  local executable was available.
- Default builds do not expose the experimental grid workflow; feature builds
  expose the new report Schema and frame-exact authorization targets.

## Commands

```text
./scripts/test-grid-generation.sh
cargo test -p forge-cli --features grid-generation grid_v91_authorization_is_frame_exact_for_probe_full_and_child_retry -- --nocapture
cargo check --workspace --all-targets --features grid-generation,subject-import
cargo fmt --all -- --check
git diff --check
```

Observed acceptance results:

- Grid Provider contract: 16/16 passed.
- V9.1 focused Provider contracts: validation, child retry, failure evidence,
  full Pack/Godot, and tamper rejection passed.
- Core test suites: 331/331 passed.
- CLI feature test suite: 15/15 passed.
- Pack test suites: 42/42 passed.
- Workspace all-target check: passed.
- Default CLI product contract: passed.
- Real Provider environment variables were unset by the release script; no
  credentials or paid endpoints were used.

## Deliberate boundary

This is offline implementation acceptance, not visual acceptance of a real
model. The first later paid operation must remain a separately authorized
`walk_down` validation probe. It may spend at most 8 image requests, may not
generate another direction, video, Pack, or Godot install, and requires native
review before scope expands.
