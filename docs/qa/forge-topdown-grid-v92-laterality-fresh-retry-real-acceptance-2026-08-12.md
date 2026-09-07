# Forge V9.2 laterality fresh-retry real acceptance

Date: 2026-08-12  
Workflow: `topdown-grid@9.2.0`  
Provider/model: `xai` / `grok-imagine-image-quality`  
Verdict: **probe executed; generated cycle rejected**

## Independent authorization

The user independently authorized exactly one real `walk_down` validation
probe after the laterality fresh-retry offline gate passed. Expired pending
Plan `7666cc4c-93ef-46ce-abc8-944bcc4c4b68` was not executed. A new equivalent
single-use Plan, `0910d1ec-1133-47f7-b6ee-9e7e108a3bfb`, estimated and capped
execution at one image request.

Durable authorization `ayla-v92-frame2-fresh-child-probe-20260812` allowed:

- target `walk_down:frame:2` only;
- model `grok-imagine-image-quality` only;
- one request for the target and one request in total;
- `1,400,000,000` reserved and maximum cost ticks;
- lineage root `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`.

The child used failed structured-gait Job
`0048ce3b-a0ea-4a63-8ed9-9063d01e9043` as its immutable source. That source's
hash-bound `grid-keyframe-action-report@1.1.0` and
`gait-laterality@1.0.0` reports both recommended only frame 2. The approved
Direction Grid remained source Job `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`
with source Lock SHA
`9cf407379b2f8032d2c1abb3daecd79bb16c00a39f0ee556df98e1cbab1f1d78`.

## Execution result

Child Job: `d080b43c-bb35-4fc0-8f46-ba33aa1a8062`.

The runner byte-reused frames 0, 1, and 3 and submitted one fresh frame-2
request. The selected Provider node had exactly two immutable inputs:
`DirectionAnchor` and transparent grayscale `PoseStructure`. It contained no
failed `EditTarget`, `inputFrameSha256`, or `replacesFrameSha256`. The Action
Report records attempt 3 as `laterality_fresh_retry` under
`grid-keyframe-action-report@1.2.0`.

Actual usage was:

- one settled `edit_image` request on `walk_down:frame:2`;
- one generated image and no second attempt;
- `700,000,000` observed cost ticks;
- zero generated or edited videos and zero private uploads.

The Job ended with stable error `walk_laterality_not_alternating`. The
authorization was exhausted at 1/1, so no additional Provider request was
attempted.

## Automatic gates

Equipment and hand evidence remained `game_ready`: no equipment, grip,
long-thin protrusion, or hand-region drift was detected.

The explicit laterality gate rejected the fresh output:

| Frame | Phase | Expected side | Observed side | Left / right bottom Y | Result |
| --- | --- | --- | --- | --- | --- |
| 0 | left contact | screen-left | screen-left | `240 / 230` | pass |
| 1 | left passing | screen-left | screen-left | `240 / 230` | pass |
| 2 | right contact | screen-right | screen-left | `240 / 226` | **fail** |
| 3 | right passing | screen-right | screen-right | `228 / 240` | pass |

The opposite-contact delta was `0.019555923`, below the required `0.04`.
Motion semantics independently blocked the cycle with
`walk_phase_order_invalid`; its phase-order score fell to `0.3983861`, below
the required `0.45`. The four rasters were distinct and upper-body flicker
remained within threshold, but those facts do not override the incorrect
contact side and phase ordering.

## Native-size review

Review covered the new 1024 x 1024 Provider source, its 256 x 256 normalized
frame, the reused normalized frames, and the frame-2 PoseStructure.

- Ayla's face, warm-brown skin, auburn hair, green hood, full cape, amber
  scarf, jerkin, pouches, trousers, boots, front-facing direction, and empty
  hands remain coherent.
- There are no sheet dividers or grayscale-guide leaks.
- The new frame is sharper than the rejected diagnostic edit, confirming that
  removing the failed raster avoids its softness and bad-pixel inheritance.
- The anatomical correction still failed: the screen-left boot is visibly the
  lower support/contact boot while the screen-right boot is raised. It repeats
  frame 0's contact side instead of forming the requested opposite contact.
- The deterministic laterality result therefore matches native visual review;
  this is not a detector false positive.

Evidence:

- [new native frame-2 source](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/d080b43c-bb35-4fc0-8f46-ba33aa1a8062/source/provider/walk_down/frame-02/attempt-3/source.png)
- [frame 0 — reused left contact](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/d080b43c-bb35-4fc0-8f46-ba33aa1a8062/grid-keyframes/walk_down/frame-00.png)
- [frame 1 — reused left passing](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/d080b43c-bb35-4fc0-8f46-ba33aa1a8062/grid-keyframes/walk_down/frame-01.png)
- [frame 2 — rejected right contact](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/d080b43c-bb35-4fc0-8f46-ba33aa1a8062/grid-keyframes/walk_down/frame-02.png)
- [frame 3 — reused right passing](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/d080b43c-bb35-4fc0-8f46-ba33aa1a8062/grid-keyframes/walk_down/frame-03.png)

## Integrity and scope closure

- All 15 child Job artifact SHA-256 values revalidated against their files.
- The failed source Job JSON and Action Report hashes remained unchanged.
- The approved source DirectionGridLock SHA remained unchanged.
- WorkflowGraph records one non-cache Provider node, for frame 2 only; all
  other frame nodes are cache hits and byte reuse.
- The authorization ledger contains one settled request and no other target.
- Credential, bearer-token, API-key, and refresh/access-token scans returned
  zero matching files.
- The probe produced no `walk_up`, `walk_right`, or `walk_left`, no video, no
  `.gsfpack`, and no Godot resource.
- The project directory had zero files modified during the probe.

## Decision

Do not approve this cycle, expand to other directions, regenerate the approved
Direction Grid, or chain another laterality child from this result. The fresh
retry successfully isolated the request and preserved appearance, but the real
model did not obey the grayscale PoseStructure's viewer-space leg side.

The next correction must be offline and should change the control signal, not
spend another prompt-only retry. Evaluate a more explicit asymmetric structure
guide whose contact boot and raised boot have visually distinct grayscale
values or shapes, and verify that the provider actually follows that guide in
a frozen fixture/real-A-B contract. Keep the deterministic laterality gate and
the 1/1 frame-only budget. A further real probe requires new authorization.
