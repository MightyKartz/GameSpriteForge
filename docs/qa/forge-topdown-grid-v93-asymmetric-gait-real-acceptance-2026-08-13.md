# Forge `topdown-grid@9.3.0` asymmetric-gait real acceptance

Date: 2026-08-13  
Verdict: **rejected at native review; one real request completed; no retry**

## Authorized scope

The user independently authorized one real `walk_down` validation generation.
Forge derived the request from immutable V9.2 failure Job
`0048ce3b-a0ea-4a63-8ed9-9063d01e9043` and created V9.3 Job
`54b278d7-fdf3-4d39-9516-83ec5964ad63`.

- workflow: `topdown-grid@9.3.0`
- target: exactly `walk_down:frame:2`
- provider/profile/model: `xai` / `default` /
  `grok-imagine-image-quality`
- expected / maximum requests: `1 / 1`
- maximum Provider operations: `1`
- maximum/reserved cost: `1,400,000,000` ticks
- videos, Pack, catalog, Godot: `0`

The new authorization was bound to recipe
`2b8e04dc368c19ed02ab2dba62b5c31bc24da5155ffdaba9c49efa24269dabbc`,
input fingerprint
`bca54aa06b655280adc1a4afb2127dc9bc3566529b3db41a524af8212bc72406`,
and lineage root `833cbbf2-c8c8-4191-bcc6-4a0215d43b64`. Its initial ledger was empty.

## Execution result

The single `edit_image` operation settled successfully. Provider usage was:

- requests: `1`
- generated images: `1`
- observed cost: `700,000,000` ticks
- generated/edited videos and private uploads: `0`

The Job stopped at `awaiting_review` with
`grid_keyframe_validation_review_required`. It exported no Pack and performed
no Godot mutation. Source V9.2 tree SHA-256 remained unchanged at
`79dc39bb62c90b6afd2c08ef04c9a40f55f522a0c1c89dc95f2dddf897ae2622`.
Frames 0, 1, and 3 were byte-identical reuse; their SHA-256 values remained:

- frame 0: `4ad2946571e3352de04904693e3d9eb30605ebe37ca30f3ec1aade2bc5f32ca8`
- frame 1: `edd6795e1f9a37a9a52b315dad68170e7b6ba0da0dba7d00758c495bef977d8a`
- frame 3: `d389b3732a42220bc73574c1c875c231f7bac3bbe69dd1a5c7b12f6897ed80a0`

Frame 2 used `asymmetric_guide_fresh_retry` with
`grid-pose-structure@1.1.0`. Its delivered SHA-256 is
`b394996098073003f5f68e395c7b7c34e9e15d45abfc9536fd9794703ef219c4`.

## Deterministic gates

The generated four-frame sequence passed automated gates:

- gait laterality: all four expected viewer-space sides matched; frame 2 was
  correctly classified as screen-right contact;
- motion semantics: four distinct poses, phase-order score `0.8158823`, and
  opposing-contact change ratio `0.18408664`;
- identity/style consistency, geometry, background cleanup, equipment and
  empty-hand evidence: `game_ready`;
- no additional Provider request was attempted.

## Native review rejection

Native-size review rejected frame 2. The viewer-right boot stands on a wide,
flat gray platform with a light top and dark rim, more than twice the apparent
boot width. The other three frames contain no such object. Its shape directly
echoes the V1.1 guide's bright, thick, wide horizontal sole, so this is a
semantic PoseStructure leak rather than a valid brown boot sole.

The artifact creates a conspicuous platform/skate appearance and breaks the
foot silhouette, ground contact, and temporal continuity. Identity, face,
auburn hair, green hood, amber scarf, leather clothing, full cape, and empty
hands otherwise remained acceptable. One polluted keyframe is sufficient to
reject the whole action.

The Job was deliberately left unapproved in `awaiting_review`. The sole V9.3
child scope is consumed and this report does not authorize another request.

The follow-up remediation is versioned separately as `topdown-grid@9.4.0`.
V9.4 removes the wide horizontal guide sole, introduces a compact V1.2
contact marker and hard-gates platform/skate silhouettes. The V9.3 Job and
this rejection evidence remain immutable.

## Evidence

- [native contact sheet](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/54b278d7-fdf3-4d39-9516-83ec5964ad63/grid-keyframe-validation/walk_down-contact-sheet.png)
- [rejected frame 2](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/54b278d7-fdf3-4d39-9516-83ec5964ad63/grid-keyframes/walk_down/frame-02.png)
- [raw Provider output](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/54b278d7-fdf3-4d39-9516-83ec5964ad63/source/provider/walk_down/frame-02/attempt-3/source.png)
- [V1.1 PoseStructure](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/54b278d7-fdf3-4d39-9516-83ec5964ad63/source/provider/walk_down/frame-02/attempt-3/pose-structure.png)
- [Action Report](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/54b278d7-fdf3-4d39-9516-83ec5964ad63/source/grid-keyframe-actions/walk_down.json)
- [WorkflowGraph](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/54b278d7-fdf3-4d39-9516-83ec5964ad63/workflow-graph.json)
