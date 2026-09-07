# Forge `topdown-grid@9.4.0` platform-safe gait real execution and rejection

Date: 2026-08-13  
Verdict: **deterministic gates passed, but later native review rejected a neutral-gray footwear/matting residue**

> Superseded on 2026-08-13. The initial native review below missed a compact
> gray-white component on the inner edge of frame 2's viewer-right boot. The
> user rejected that result after inspecting the native frame. Job
> `c96788ba-e9de-43d6-8f30-c11611282f42` now contains the immutable
> `manual_rejected` decision. The accepted replacement is documented in
> `forge-topdown-grid-v95-footwear-cleanup-real-2026-08-13.md`.

## Authorized scope

The user explicitly requested one real generation after the V9.4 offline
remediation was accepted. Forge derived a new Plan from immutable rejected
V9.3 Job `54b278d7-fdf3-4d39-9516-83ec5964ad63` and created V9.4 Job
`c96788ba-e9de-43d6-8f30-c11611282f42`.

- workflow/target: `topdown-grid@9.4.0` / `walk_down:frame:2`
- provider/profile/model: `xai` / `default` /
  `grok-imagine-image-quality`
- recipe/input: `a7dc53e5e18ccb9fe332b1cbaa745837e646440e54c2f39dec1757e47e22bfea`
  / `89e0dde997005f034cb701a539a279ed30da28268d85e79e922989bb13fc6ccd`
- expected / maximum requests: `1 / 1`
- maximum Provider operations: `1`
- maximum/reserved cost: `1,400,000,000` ticks
- videos, Pack, catalog, Godot: `0`

The new authorization ledger was physically empty before execution and bound
to lineage root `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`. A first CLI attempt
without the environment request cap failed before Plan claim, Job creation,
authorization reservation or network activity. Execution resumed only after
setting the independently reviewed environment caps to one request and 1.4B
ticks.

## Execution and provenance

Exactly one `edit_image` operation settled:

- requests / Provider operations: `1 / 1`
- generated images: `1`
- observed cost: `700,000,000` ticks
- generated/edited videos and private uploads: `0`

The Job stopped at `awaiting_review` with
`grid_keyframe_validation_review_required`; this is the validation workflow's
expected review boundary. It exported no Pack.

Frames 0, 1 and 3 were byte-reused:

- frame 0: `4ad2946571e3352de04904693e3d9eb30605ebe37ca30f3ec1aade2bc5f32ca8`
- frame 1: `edd6795e1f9a37a9a52b315dad68170e7b6ba0da0dba7d00758c495bef977d8a`
- frame 3: `d389b3732a42220bc73574c1c875c231f7bac3bbe69dd1a5c7b12f6897ed80a0`

Frame 2 used `platform_safe_guide_fresh_retry` and
`grid-pose-structure@1.2.0`; its delivered SHA-256 is
`c709dfc8a2de65dfa365c68574321d07b4a7d3863e8c22adb3a79bea7e24e2dc`.
The source V9.3 tree and `job.json` remained unchanged at
`f58db9e9e0a6bff55cef9f6246a7acacdfc84e98ae64dff4a805e981c9af1891`
and `6a1bb28c8d372060cc1b290b06f607b839a835b260cd3d083196a1aef01e1310`.
All 17 bound child artifacts matched their Job-record SHA-256 values.

## Deterministic gates

- motion semantics: game-ready, four distinct poses, phase-order score
  `0.8610755`, opposing-contact change ratio `0.18408664`;
- gait laterality: all four expected viewer-space sides matched with confidence
  `1.0`; frame 2 is screen-right contact;
- equipment/empty hands and consistency/cleanup: game-ready;
- footwear platform gate: game-ready for every frame. Frame 2 viewer-right
  shelf/stem is now `21 / 16 = 1.3125×`, versus the rejected V9.3 result
  `46 / 16 = 2.875×`;
- credential/temporary URL scan: zero matches.

## Native review

Rejected after user re-review. The wide platform/board silhouette was gone and
the viewer-right contact pose was correct, but frame 2 still contained a
compact gray-white component on the inner boot/ankle edge. The historical
`footwear-platform@1.0.0` report did not model neutral-gray connected
components, so its `game_ready` result was insufficient. The source Job was
not overwritten; the rejection was recorded and subsequent work used child
Jobs.

This acceptance records the validation result only; it does not authorize a
Pack export, other directions, video generation, catalog mutation or Godot
installation.

## Evidence

- [native contact sheet](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/c96788ba-e9de-43d6-8f30-c11611282f42/grid-keyframe-validation/walk_down-contact-sheet.png)
- [rejected frame 2](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/c96788ba-e9de-43d6-8f30-c11611282f42/grid-keyframes/walk_down/frame-02.png)
- [PoseStructure V1.2](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/c96788ba-e9de-43d6-8f30-c11611282f42/source/provider/walk_down/frame-02/attempt-4/pose-structure.png)
- [footwear report](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/c96788ba-e9de-43d6-8f30-c11611282f42/source/grid-keyframe-actions/walk_down-footwear-platform.json)
- [Action Report](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/c96788ba-e9de-43d6-8f30-c11611282f42/source/grid-keyframe-actions/walk_down.json)
- [WorkflowGraph](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/c96788ba-e9de-43d6-8f30-c11611282f42/workflow-graph.json)
