# Forge V9 Direction Grid equipment and hand remediation QA

Date: 2026-08-11

## Outcome

Offline remediation is complete. No real Provider request was made.

The observed defects were confirmed as contract failures, not merely drawing quality:

- `front_idle` contains an undeclared staff and does not present a reliable empty-hand idle;
- the four views introduce staff/quiver/arrow inventory despite `equipment.kind = none`;
- side views contain inconsistent hand-covering evidence;
- the old structural reports did not evaluate these semantics.

The existing real Job remains unapproved and has no action Grid, Pack, or Godot output.

## Real failed-pixel recheck

Source Job: `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`.

The new local evaluator used the immutable Ayla Subject canonical plus the four already-paid node PNGs. It made zero Provider requests and did not access credentials.

```json
{
  "profile": "direction-grid-appearance@1.0.0",
  "equipmentKind": "none",
  "verdict": "awaiting_review",
  "reasons": ["direction_grid_undeclared_object_evidence"],
  "unexpectedElongatedNodeCount": 4,
  "nodes": {
    "front_idle": 3.7272727,
    "back_idle": 3.7142856,
    "right_idle": 7.428571,
    "left_idle": 4.8333335
  }
}
```

The exterior evidence intentionally remains review-only because capes, wings, tails, sleeves, and non-human anatomy can produce similar geometry. The user's visual review supplies the final conclusion that these particular structures are undeclared equipment. A high-confidence hand/equipment detector still produces a hard failure when it recognizes a held or detached prop.

## Retry preflight

The real legacy Job was accepted as a compatible retry source by a Plan-only run:

```json
{
  "workflow": "topdown-grid@9.0.0",
  "expectedProviderRequests": 1,
  "maximumProviderRequests": 2,
  "provider": "xai",
  "model": "grok-imagine-image-quality"
}
```

The Plan was not executed. The existing authorization ledger is unchanged:

- authorization: `ayla-v9-direction-grid-real-20260811`;
- `nextSequence`: `2`;
- settled requests: `1`;
- observed cost: `700,000,000` ticks;
- authorization cap: `2` requests / `2,800,000,000` ticks;
- remaining scope: one `direction_grid` request under the existing authorization.

## Offline gates

- `scripts/test-grid-generation.sh`: passed, including 9/9 Grid contracts.
- Core library: 266/266 tests passed with `grid-generation`.
- CLI: 13/13 tests passed with `grid-generation`.
- Focused Core appearance/equipment tests: passed.
- Fixture whole-Grid child retry: passed with one observed request.
- Cross-Subject/equipment/camera/model Lock reuse: rejected with zero requests.
- Cross-Subject/equipment/camera/model retry source: rejected with zero requests.
- Core, Providers, and CLI Clippy with Grid features and `-D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

## Release status

The remediation is ready for one explicitly authorized real Direction Grid retry. It is not evidence that the next xAI image will pass visual review. Action Grid generation remains prohibited until the replacement `direction-grid-lock@1.1.0` is explicitly accepted.

## Real Direction Grid retry

The original authorization had expired before execution. After the user explicitly requested the documented next step, Forge created a narrower replacement authorization:

- authorization: `ayla-v9-direction-grid-remediation-real-20260811`;
- target: `direction_grid` only;
- model: `grok-imagine-image-quality`;
- maximum requests: `1`;
- maximum cost: `1,400,000,000` ticks;
- source lineage root: `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`.

The first execution attempt stopped before Job creation or Provider use because the legacy process-level acceptance environment was absent. The ledger remained empty. Execution was repeated with process limits exactly matching the durable authorization.

Result:

- child Job: `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`;
- parent Job: `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`;
- lifecycle: `awaiting_review`;
- requests: `1`;
- observed cost: `700,000,000` ticks;
- generated images: `1`;
- generated videos/video edits: `0`;
- Action Grid, Pack, and Godot output: none.

Native review of [the replacement contact sheet](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad/direction-grid/contact-sheet.png) confirms that the staff, quiver, arrows, and inconsistent glove were removed. `front_idle` is planted; the rear view retains the hood and full cape. Two side-view elongated-outline observations are the cape hem and remain review-only by design.

The new immutable closure passed:

- `direction-grid-lock@1.1.0` Lock SHA matches Job artifact and Provider manifest;
- appearance-report SHA matches Lock, Job artifact, and Provider manifest;
- all four node SHA-256 values match their PNG files;
- credential and temporary-URL scan returned no matches;
- no accepted review decision was written.

The user visually accepted the replacement as "perfect". Forge wrote
`review-decision.json` and `direction-grid-approval.json` at
`2026-08-11T14:39:24Z`; the approved Lock SHA is
`9cf407379b2f8032d2c1abb3daecd79bb16c00a39f0ee556df98e1cbab1f1d78`.

## Real Action Grid gate

The user separately authorized the four Action Grids with an expected/maximum
budget of `4/8` image requests and `11,200,000,000` ticks. The durable
authorization `ayla-v9-action-grid-real-20260811` permitted only:

- `walk_down:action_grid`;
- `walk_up:action_grid`;
- `walk_right:action_grid`;
- `walk_left:action_grid`.

The complete-stage child Job was
`e7c14d79-33a6-4c4c-bdfe-93d3ac6d15d8`, with approved Lock Job
`9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad` as its parent and lineage root
`833cbbf2-c8c8-4191-bcc6-4a0215d43b64`.

Forge stopped on `walk_down` after its two allowed attempts. No later target was
submitted. Both sheets passed structural extraction but failed
`motion-semantics@1.1.0` with:

- `stable_upper_body_flicker`;
- `walk_contact_poses_too_similar`;
- `walk_phase_order_invalid`;
- `walk_pose_diversity_missing`.

The native sheets corroborate the detector: each contains only two effective
pose families, with near-duplicate standing and stride cells rather than four
ordered walk phases. This is not a threshold-only failure.

- [attempt 1 sheet](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/e7c14d79-33a6-4c4c-bdfe-93d3ac6d15d8/source/provider/walk_down/attempt-1/sheet.png)
- [attempt 2 sheet](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/e7c14d79-33a6-4c4c-bdfe-93d3ac6d15d8/source/provider/walk_down/attempt-2/sheet.png)

Actual usage was two settled `edit_image` requests, both for
`walk_down:action_grid`, at `800,000,000` ticks each (`1,600,000,000` total).
The authorization ledger remains at `nextSequence: 3`; the remaining targets
were not consumed. Credential and temporary-URL scanning returned no matches.
No Pack, Godot resource, video, Subject, Style, or replacement Direction Grid
was generated.

Release conclusion: Action Grid real acceptance is blocked. Do not spend the
remaining authorization until the four-phase pose-control contract is revised
and passes a zero-cost fixture/frozen-image gate.

## Offline Action Grid four-phase remediation — 2026-08-12

The zero-cost remediation is complete. No xAI request, credential lookup, Pack
export from the failed real Job, or Godot mutation was performed.

Forge now encodes `left contact -> left passing -> right contact -> right
passing` as `grid-action-phases@1.0.0`. A failed first Action Grid is the
`EditTarget` of attempt 2; the immutable direction anchor and deterministic pose
guide remain the next two references. Motion and hand/equipment diagnostics are
combined into one correction request.

`grid-action-report@1.1.0` adds phase evidence and diagnostic-edit provenance.
Both legacy 1.0 and earlier 1.1 reports must pass the current local motion and
per-frame equipment checks before reuse. Targeted cell retry runs the same two
gates. Provider reference capacity, input SHA-256 and exact attempt output path
are checked before results can advance.

Failed transport, malformed sheets, hash mismatches and output-path violations
remain reconstructable through `action-attempts.json` and the failed
`grid-provider-manifest.json`.

Offline evidence:

- core Action Grid report tests: `6/6`;
- reference-capacity preflight: passed;
- Grid provider contract: `11/11`, including collapsed-first-attempt recovery,
  incorrect output path, per-frame equipment failure, Pack and Godot install;
- xAI reference-order contract: covered separately by the Provider unit gate;
- real Provider usage: `0` requests / `0` ticks.

The old authorization ledger remains unchanged at two settled
`walk_down:action_grid` requests (`1,600,000,000` ticks). Real acceptance is
still blocked pending a new, explicitly authorized single-direction probe.

## Real single-direction Action Grid probe — 2026-08-12

Before spending, Forge added and verified a V9 `validationOnly` path. It accepts
exactly one walk animation, estimates `1/2` requests, exposes only that
animation's authorization target, stops at native review, and cannot export a
partial Pack. The full Grid product gate passed `12/12` fixture contracts.

The old authorization was expired and its `walk_down` allowance was exhausted,
so Forge created the narrower authorization
`ayla-v9-walk-down-action-grid-probe-20260812`:

- target: `walk_down:action_grid` only;
- model: `grok-imagine-image-quality`;
- expected/maximum requests: `1/2`;
- maximum cost: `2,800,000,000` ticks;
- source lineage root: `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`.

Probe Job: `0eae6ee6-d635-4958-ae91-14223045f39c`.

Attempt 1 retained Ayla's identity, clothing colors and empty hands, and passed
the equipment gate. Motion remained blocked by
`walk_contact_poses_too_similar` and `walk_phase_order_invalid`.

- [attempt 1 Action Grid](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/0eae6ee6-d635-4958-ae91-14223045f39c/source/provider/walk_down/attempt-1/sheet.png)

Attempt 2 correctly used attempt 1 as its hashed `EditTarget`, but the model
introduced full-width black row and column divider lines. Those lines touch all
four cell boundaries, so `animation-sheet@1.0.0` rejected every cell before
motion acceptance.

- [attempt 2 diagnostic edit](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/0eae6ee6-d635-4958-ae91-14223045f39c/source/provider/walk_down/attempt-2/sheet.png)

Actual usage was two settled image edits at `800,000,000` ticks each
(`1,600,000,000` total). No up/right/left target, Subject, Style, Direction
Grid, video, Pack, or Godot resource was generated. The Job and authorization
contain complete attempt/usage provenance; credential and temporary-URL scans
passed.

Conclusion: the probe is rejected. The remaining issue is not identity drift;
it is the sheet-layout contract plus insufficiently distinct walk phases. Do
not authorize the other directions until diagnostic editing is constrained to
preserve a separator-free sheet canvas and passes a new offline fixture gate.
