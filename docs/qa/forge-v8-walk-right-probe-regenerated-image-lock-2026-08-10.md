# Forge V8 Walk Right Probe Regenerated Image Lock

Date: 2026-08-10

Status: regenerated, then rejected by human visual review. No video request was
authorized or made. No Pack was exported.

## Result

- Image-lock Job: `ede37df3-013d-4300-b11e-0f9130fa7aa7`
- Workflow: `topdown-direction-motion@8.0.0`
- Image model: `grok-imagine-image-quality`
- Image requests: 8 expected, 8 actual, 0 retries
- Video requests: 0
- Observed image cost: 5,100,000,000 cost ticks
- Approval file: absent until explicit review acceptance

## Gate evidence

All eight nodes are `game_ready` under the repaired V8 image-lock feedback.
The canonical `front_idle` passed the new stable-stance gate. Direction idles
and walk poses retain one foreground component and high parent occupancy
similarity:

- `back_idle`: 0.9268
- `right_idle`: 0.8655
- `left_idle`: 0.8499
- `front_walk`: 0.9260
- `back_walk`: 0.9298
- `right_walk`: 0.9151
- `left_walk`: 0.8914

## Evidence

- Contact sheet:
  `generated-assets/forge-v8-walk-right-probe-20260810/jobs/ede37df3-013d-4300-b11e-0f9130fa7aa7/direction-motion-lock/contact-sheet.png`
- Lock:
  `generated-assets/forge-v8-walk-right-probe-20260810/jobs/ede37df3-013d-4300-b11e-0f9130fa7aa7/source/direction-motion-lock.json`
- Usage:
  `generated-assets/forge-v8-walk-right-probe-20260810/jobs/ede37df3-013d-4300-b11e-0f9130fa7aa7/provider-usage.json`

## Next gate

Human review found that `back_idle` does not preserve the permanent hood from
`front_idle`. The rejection is recorded in the source Job. The next paid gate
is a targeted `idle_up` image-lock retry from Job
`ede37df3-013d-4300-b11e-0f9130fa7aa7`, estimated 2 and maximum 4 image edits;
it must regenerate `back_idle` and dependent `back_walk` only. Video remains
forbidden until the new eight-image lock is reviewed and approved.

## Targeted retry result

- Retry Job: `41598630-e5f4-4af8-b32b-a71f623ec182`
- Source Job: `ede37df3-013d-4300-b11e-0f9130fa7aa7`
- Scope: `idle_up` / `still`
- Regenerated nodes: `back_idle`, `back_walk`
- Image requests: 2 expected, 2 actual, 0 retries
- Video requests: 0
- Observed retry cost: 1,500,000,000 cost ticks
- Status: awaiting human review; no approval and no Pack

Both regenerated nodes are `game_ready` under the structural gates. The contact
sheet is:

`generated-assets/forge-v8-walk-right-probe-20260810/jobs/41598630-e5f4-4af8-b32b-a71f623ec182/direction-motion-lock/contact-sheet.png`

## Targeted retry review

Human review rejected the retry because `back_idle` still does not clearly
preserve both the permanent hood and shoulder cape. The Job is recorded as
`manual_rejected`; no further paid retry should run until the direction-idle
prompt and structural gate are strengthened. Video remains forbidden.

## Follow-up repair

The direction-idle prompt now explicitly preserves permanent head and
shoulder/body coverings when present, while rejected prior images are marked
as negative evidence. New `parent_head_covering_loss` and
`parent_shoulder_covering_loss` feedback reasons are converted into specific
repair instructions for a subsequent edit rather than serving only as terminal
rejections. This repair has offline test coverage; no real xAI retry has been
run after the repair.

The repair was extended after the full regeneration review: V8 now checks the
lower-body extent and hemline of permanent coverings, and every image node is
materialized through deterministic background cleanup. The cleanup removes
background-colored residue while protecting opaque green components whose
color is materially different from the sampled background, so legitimate green
clothing is not erased with the canvas. This extension is covered by focused
offline tests and the V8 fixture contract; no real xAI retry has been run after
the extension.

## Full-covering cleanup regeneration

A new full eight-image Job was run after the full-covering and cleanup repair:

- Job: `479e3e7d-7c7c-42a1-81a9-e7a5b418c4f9`
- Image requests: 13 actual
- Video requests: 0
- Observed cost: 8,200,000,000 cost ticks
- All 13 background cleanup reports are `game_ready` with zero border opaque
  residue
- Status: awaiting human review; no approval and no Pack

The lock is not ready for video because several nodes remain `awaiting_review`:

- `front_idle`: `canonical_idle_not_stable_stance` after two attempts
- `right_idle`: head, shoulder, and lower-covering feedback after two attempts
- `left_idle`: head and lower-covering feedback after two attempts
- `right_walk`: parent foreground-structure feedback after two attempts

`back_idle` and `back_walk` are `game_ready`. The contact sheet is:

`generated-assets/forge-v8-walk-right-probe-20260810/jobs/479e3e7d-7c7c-42a1-81a9-e7a5b418c4f9/direction-motion-lock/contact-sheet.png`

## Covering-guided targeted retry

After the prompt and feedback repair, one explicitly authorized retry was run
from rejected Job `41598630-e5f4-4af8-b32b-a71f623ec182`:

- Retry Job: `92312efb-10aa-4ed8-b57a-82163fc1677e`
- Scope: `idle_up` / `still`
- Regenerated nodes: `back_idle`, `back_walk`
- Image requests: 2 expected, 2 actual, 0 retries
- Video requests: 0
- Observed retry cost: 1,500,000,000 cost ticks
- Status: awaiting human review; no approval and no Pack

The new `back_idle` used the accepted `front_idle` as edit target and the
rejected prior `back_idle` as negative evidence. The resulting lock is:

`generated-assets/forge-v8-walk-right-probe-20260810/jobs/92312efb-10aa-4ed8-b57a-82163fc1677e/direction-motion-lock/contact-sheet.png`

## Full eight-image regeneration after covering retry rejection

Human review rejected the covering-guided retry because `back_idle` still did
not accurately preserve the cape. A full fresh image-lock Job was then run with
the repaired prompts and stance/covering feedback:

- Job: `f9a693ed-5d4d-4043-b4eb-ca56b312600c`
- Image requests: 9 actual
- `front_idle` attempts: 2; the first candidate was rejected by the stable
  stance feedback before any dependent direction was generated
- All other nodes: 1 attempt each
- Video requests: 0
- Observed cost: 5,600,000,000 cost ticks
- Status: awaiting human review; no approval and no Pack

Contact sheet:

`generated-assets/forge-v8-walk-right-probe-20260810/jobs/f9a693ed-5d4d-4043-b4eb-ca56b312600c/direction-motion-lock/contact-sheet.png`

## Full regeneration review

Human review rejected Job `f9a693ed-5d4d-4043-b4eb-ca56b312600c` for two
separate defects:

1. `back_idle` preserves only the shoulder portion of the cape instead of the
   complete permanent covering.
2. Several directions retain green background fringe around the sprite edges.

No further paid retry should run until V8 preserves the full extent of permanent
coverings and strengthens static-image edge cleanup. Video remains forbidden.
