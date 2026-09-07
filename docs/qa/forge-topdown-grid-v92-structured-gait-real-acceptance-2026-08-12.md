# Forge V9.2 structured-gait real acceptance

Date: 2026-08-12  
Workflow: `topdown-grid@9.2.0`  
Provider/model: `xai` / `grok-imagine-image-quality`  
Verdict: **rejected**

## Authorized scope

The user explicitly authorized one real `walk_down` validation probe after
the V9.2 offline gate passed. Single-use Plan
`13a642bf-462e-49e3-8d5a-6f4382d2628c` estimated four image edits and capped
execution at eight. Durable authorization
`ayla-v92-walk-down-structured-gait-probe-20260812` allowed at most two
requests for each of:

- `walk_down:frame:0`;
- `walk_down:frame:1`;
- `walk_down:frame:2`;
- `walk_down:frame:3`.

Both the process budget and durable authorization capped the run at eight
requests and `11,200,000,000` cost ticks. The authorization was bound to
lineage root `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`. The probe reused
approved V9.0 Direction Grid Job
`9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`; the source Lock SHA remained
`9cf407379b2f8032d2c1abb3daecd79bb16c00a39f0ee556df98e1cbab1f1d78`.

## Execution result

Probe Job: `0048ce3b-a0ea-4a63-8ed9-9063d01e9043`.

Forge generated the four independent first-attempt frames. Frame 0 received
one static fresh retry after its per-frame gates. The cycle-wide laterality
gate then selected only frame 2 for a diagnostic edit using immutable inputs
in the required order: failed `EditTarget`, approved `DirectionAnchor`, and
transparent grayscale `PoseStructure`.

The Job ended with stable error `walk_laterality_not_alternating`. Actual
usage was:

- six settled `edit_image` requests;
- `4,300,000,000` observed cost ticks;
- six generated images;
- zero generated or edited videos and zero private uploads;
- ledger targets limited to the four authorized `walk_down:frame:*` values.

The two unused request allowances were not spent. No child retry was created.

## Deterministic gates

Motion semantics and equipment/hand evidence were `game_ready`:

- four raster-distinct poses;
- motion phase-order score `0.63535243`;
- stable upper-body flicker ratio maximum `0.033140656`;
- no held equipment, staff, quiver, arrow, glove drift, or grip contact.

The explicit `gait-laterality@1.0.0` gate correctly rejected the cycle:

| Frame | Declared phase | Expected side | Observed side | Contact signal |
| --- | --- | --- | --- | ---: |
| 0 | left contact | screen-left | screen-left | `0.048076924` |
| 1 | left passing | screen-left | screen-left | `0.048076924` |
| 2 | right contact | screen-right | screen-left | `0.028985508` |
| 3 | right passing | screen-right | screen-right | `-0.057971016` |

The frame 0/2 opposite-contact delta was only `0.019091416`, below the
required `0.04`, and frame 2 still had the wrong sign after its one targeted
retry. The report recommended only frame 2, and the runner spent no request on
the already-correct frame 3.

## Native-size review

Review covered all six 1024 x 1024 Provider outputs, all four cleaned 256 x
256 final frames, and the frame-2 grayscale PoseStructure.

- Ayla's face, warm-brown skin, auburn hair, green hood and full cape, amber
  scarf, jerkin, pouches, trousers, boots, front-facing camera, and empty hands
  remain recognizable and coherent.
- Independent frames contain no sheet divider lines and no grayscale-guide
  leakage.
- V9.2 improves on V9.1: frame 3 visibly places the screen-right boot at the
  contact/support extreme, so structured guidance can influence laterality.
- The cycle is still invalid. Frame 2, declared right contact, continues to
  extend the screen-left boot after both the first generation and diagnostic
  edit.
- Frame-2 attempt 2 is visibly softer and less detailed than the other native
  frames. Carrying the failed raster as `EditTarget` preserved its wrong-side
  geometry and degraded sharpness instead of correcting the anatomical side.

Final normalized frames:

- [frame 0 — left contact](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/0048ce3b-a0ea-4a63-8ed9-9063d01e9043/grid-keyframes/walk_down/frame-00.png)
- [frame 1 — left passing](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/0048ce3b-a0ea-4a63-8ed9-9063d01e9043/grid-keyframes/walk_down/frame-01.png)
- [frame 2 — rejected right contact](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/0048ce3b-a0ea-4a63-8ed9-9063d01e9043/grid-keyframes/walk_down/frame-02.png)
- [frame 3 — right passing](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/0048ce3b-a0ea-4a63-8ed9-9063d01e9043/grid-keyframes/walk_down/frame-03.png)

## Integrity and scope closure

- All 35 Job artifact SHA-256 values revalidated against their files.
- Final frame, motion, equipment, and laterality report hashes revalidated.
- WorkflowGraph contains four logical Provider nodes, all for `walk_down`;
  diagnostic frame 2 records `EditTarget -> DirectionAnchor -> PoseStructure`.
- Credential, bearer-token, API-key, and temporary signed-URL scans returned
  no matches.
- No `walk_up`, `walk_right`, or `walk_left` output exists.
- No video, `.gsfpack`, Pack export, catalog entry, or Godot mutation exists.
- The approved Direction Grid source Lock SHA was unchanged after execution.

## Decision and next correction

Do not expand V9.2 to the other directions and do not spend the unused frame-2
allowance. The gate worked as designed and prevented a false positive, but
the current diagnostic edit strategy is the remaining weak point.

Before another paid probe, change a laterality-only failed frame retry from
`EditTarget + DirectionAnchor + PoseStructure` to a fresh generation from
`DirectionAnchor + PoseStructure`. Preserve diagnostic editing for failures
that benefit from local pixel correction, but do not feed wrong-side leg
geometry back to the model. Add an offline contract that frame-2 laterality
retry has two references, is recorded as a fresh retry, and never reuses the
failed-frame SHA. Then authorize at most one new frame-2 request against a new
child/validation Job. The approved DirectionGridLock remains valid and must
not be regenerated.
