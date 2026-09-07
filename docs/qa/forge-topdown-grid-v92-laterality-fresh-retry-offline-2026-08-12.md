# Forge V9.2 laterality fresh-retry offline acceptance

Date: 2026-08-12  
Workflow: `topdown-grid@9.2.0`  
Provider: fixture only  
Real requests in this offline gate: **0**  
Verdict: **offline accepted; separate real child later executed and rejected**

## Real failure addressed

Real Job `0048ce3b-a0ea-4a63-8ed9-9063d01e9043` reached
`motionVerdict: game_ready` and kept identity, cape, hood, empty hands, and four
distinct poses. Its explicit laterality report recommended only frame 2. The
frame-2 diagnostic edit nevertheless kept the screen-left boot forward and
was visibly softer because the rejected raster was supplied as `EditTarget`.

This remediation does not regenerate the approved DirectionGridLock and does
not alter the gait thresholds. It changes only the retry input policy for a
laterality-only failure.

## Implemented contract

- `LateralityFreshRetry` serializes as `laterality_fresh_retry`.
- Request references are exactly `DirectionAnchor`, `PoseStructure`.
- The prompt states that the rejected frame is intentionally absent and the
  named screen-side phase must be constructed anew.
- No `edit-target.png`, `inputFrameSha256`, or `replacesFrameSha256` is written.
- WorkflowGraph inputs contain only the anchor and PoseStructure hashes.
- Structured-keyframe implementation version is
  `grid-structured-keyframe@1.1.0`.
- New Action reports use `grid-keyframe-action-report@1.2.0`; `@1.1.0` remains
  readable only for frozen evidence compatibility.

Generic motion failures still use the existing diagnostic edit. Fresh retry
is chosen only when generic motion is `game_ready` and
`gait-laterality@1.0.0` owns the blocked verdict.

## Child validation boundary

The initial `walk_down` probe remains four expected / eight maximum requests.
A child validation is accepted only if all of the following are true:

- the source Job failed with stable `walk_laterality_not_alternating`;
- its Action Report is a V9.2 structured-gait profile, hash-bound in `job.json`;
- motion and equipment are `game_ready`, laterality is not;
- the immutable laterality report contains the stable reason;
- Action and laterality reports recommend the same single frame;
- the requested frame is exactly that recommendation;
- all four source frames and PoseStructures remain inside the source Job and
  match their SHA-256 values;
- source Provider manifest and WorkflowGraph exist.

The child Plan is one expected / one maximum image request and exposes one
authorization target. Non-selected frames and PoseStructures are copied into
the child and revalidated byte-for-byte. The selected frame gets one fresh
request; no automatic second request is reserved. A source frame already
marked `laterality_fresh_retry` cannot create another child, so the allowance
is single-use even if the fresh output remains wrong-sided.

## Offline evidence

The dedicated fixture reproduces the real failure shape: generic motion score
`0.6398972` is `game_ready`, equipment is `game_ready`, frame 2 has the wrong
screen-side contact, and laterality recommends exactly `[2]`.

- choosing frame 3 is rejected during Plan preparation;
- choosing frames 2 and 3 is rejected during Plan preparation;
- choosing frame 2 yields a 1/1 Plan and one `walk_down:frame:2` request;
- request roles are `DirectionAnchor`, `PoseStructure`;
- frames 0, 1, and 3 are byte-reused with child-local PoseStructure copies;
- frame 2 has no failed-frame provenance and is marked
  `laterality_fresh_retry`;
- the repaired fixture cycle reaches `AwaitingReview` with no Pack.
- if the one fresh output fails static quality, the child still spends exactly
  one request, preserves the Action Report, Provider manifest, WorkflowGraph,
  and raw evidence, stops with `grid_keyframe_action_quality_failed`, and may
  not be chained as another laterality child.

Focused V9.2 contracts passed 4/4. The complete Grid Provider contract passed
20/20, including V9.0/V9.1 Pack, Godot, Sheet diagnostic, and frame-retry
regressions. Additional gates passed:

- Core with `grid-generation`: 279/279 unit tests and all integration suites;
- CLI with `grid-generation`: 16/16;
- `scripts/test-grid-generation.sh`;
- default `scripts/test-cli-product.sh`;
- Core/Providers/CLI all-target Clippy with `grid-generation` and `-D warnings`;
- `cargo fmt --all -- --check`.

A read-only preflight against frozen real failed Job
`0048ce3b-a0ea-4a63-8ed9-9063d01e9043` successfully produced pending Plan
`7666cc4c-93ef-46ce-abc8-944bcc4c4b68` with one expected / one maximum request,
retry scope `walk_down:frame:2`, and workflow `topdown-grid@9.2.0`. Execution
stopped at the real-provider acceptance gate. The existing authorization
ledger remained at six settled requests and `4,300,000,000` ticks; this
implementation made zero real requests.

## Release decision

Do not expand directions and do not regenerate the approved DirectionGridLock.
At closure of this offline gate, the only permissible next paid gate was a
separately authorized one-request frame-2 child validation sourced from real
failed Job `0048ce3b-a0ea-4a63-8ed9-9063d01e9043`, followed by native-size
review. This offline result did not imply real-model acceptance.

## Real follow-up

The separately authorized 1/1 child was subsequently executed as Job
`d080b43c-bb35-4fc0-8f46-ba33aa1a8062`. The request boundary and fresh-input
contract worked, but the model again produced screen-left contact for the
screen-right frame. See
`docs/qa/forge-topdown-grid-v92-laterality-fresh-retry-real-acceptance-2026-08-12.md`.
