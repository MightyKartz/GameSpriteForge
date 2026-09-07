# Forge V9.1 independent-keyframe real acceptance

Date: 2026-08-12  
Workflow: `topdown-grid@9.1.0`  
Provider/model: `xai` / `grok-imagine-image-quality`  
Verdict: **rejected**

## Authorized scope

The user explicitly authorized one real `walk_down` validation probe. The
single-use Plan estimated 4 image edits and capped execution at 8. Durable
authorization `ayla-v91-walk-down-keyframe-probe-20260812` allowed exactly two
requests for each of:

- `walk_down:frame:0`;
- `walk_down:frame:1`;
- `walk_down:frame:2`;
- `walk_down:frame:3`.

The authorization capped total cost at `11,200,000,000` ticks and bound the
requests to lineage root `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`. The probe
reused approved Direction Grid Job `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`;
its approved source Lock SHA remained
`9cf407379b2f8032d2c1abb3daecd79bb16c00a39f0ee556df98e1cbab1f1d78`.

## Execution result

Probe Job: `7097f67e-0e85-4ead-b5bc-c6c908f4abda`.

Forge generated all four independent first-attempt frames, then retried only
frames 1, 2, and 3 using each failed frame as an immutable edit target plus the
approved `front_idle` direction anchor. Frame 0 was not retried. The Job ended
with `grid_keyframe_action_quality_failed` and preserved all raw images,
cleanup reports, action reports, Provider usage, WorkflowGraph, and hashes.

Actual usage:

- 7 settled `edit_image` requests: four first attempts plus three targeted
  retries;
- `4,500,000,000` observed cost ticks;
- 7 generated images;
- 0 generated/edited videos and 0 private uploads;
- ledger targets contain only the four authorized `walk_down:frame:*` values.

One possible frame-0 retry remained unused. It cannot repair the cycle-wide
laterality failure and was not spent.

## Deterministic gates

Static consistency, geometry, alpha, single-subject, equipment-none, and hand
checks passed for every accepted frame attempt. The action equipment report is
`game_ready`: no staff, quiver, arrow, glove drift, or other held equipment was
detected.

The final motion report measured four pixel-distinct poses and a nominal phase
order score of `0.6274672`, but still blocked the action for
`lower_body_edge_ghost` on frames 2 and 3. This automatic reason is not a
complete diagnosis: the metric can distinguish silhouettes but does not prove
that the two anatomical sides alternate.

## Native-size review

Native review covered all seven 1024 x 1024 Provider outputs, the four cleaned
256 x 256 final frames, and the approved `front_idle` anchor.

- Ayla's face, warm-brown skin, auburn hair, green hood and full cape, amber
  scarf, jerkin, pouches, trousers, boots, front-facing camera, and empty hands
  remain coherent.
- There are no Action Grid divider lines because V9.1 generates independent
  images.
- The walk cycle is not valid: all four final frames place the same viewer-left
  boot forward while the viewer-right boot remains the lifted/rear leg. The
  requested `left contact -> left passing -> right contact -> right passing`
  semantics collapsed to variants of one side.
- Attempts 2 for frames 1-3 change stride magnitude and fine detail but do not
  reverse the contacting/forward leg.

Final normalized frames:

- [frame 0 — declared left contact](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/7097f67e-0e85-4ead-b5bc-c6c908f4abda/grid-keyframes/walk_down/frame-00.png)
- [frame 1 — declared left passing](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/7097f67e-0e85-4ead-b5bc-c6c908f4abda/grid-keyframes/walk_down/frame-01.png)
- [frame 2 — declared right contact, visually still the same side](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/7097f67e-0e85-4ead-b5bc-c6c908f4abda/grid-keyframes/walk_down/frame-02.png)
- [frame 3 — declared right passing, visually still the same side](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/7097f67e-0e85-4ead-b5bc-c6c908f4abda/grid-keyframes/walk_down/frame-03.png)

This is a real model failure, not a threshold-only rejection. Lowering or
removing the edge-ghost threshold would incorrectly admit a non-alternating
cycle.

## Closure and safety

- Every Job artifact SHA-256 in `job.json` was revalidated against its file.
- Credential, bearer-token, API-key, and temporary signed-URL scanning returned
  no matches.
- No `walk_up`, `walk_right`, or `walk_left` Provider output exists.
- No video, `.gsfpack`, Pack export, or Godot mutation was produced.
- The approved Direction Grid source Lock SHA was unchanged after the run.

## Decision

Do not expand V9.1 to the remaining directions and do not retry the same text
phase contract. V9.1 proved that independent frames avoid sheet layout defects
and retain appearance/equipment, but xAI did not obey left/right gait
laterality. Before another paid probe, Forge needs an explicit anatomical-side
gate and a pose-control method whose opposite-contact frame is structurally
conditioned rather than identified only by text. The existing approved
DirectionGridLock remains valid and must not be regenerated.
