# Forge external DirectionGrid identity remediation plan

Status: Phase 1 implemented and offline accepted; Phase 2 zero-request replay
completed and correctly rejected the sheet candidate; Phase 3 was separately
authorized and completed with four independent candidates, then blocked by the
Alpha edge gate. Phase 4 remains separately authorized future work.

## Decision

Keep the external DirectionGrid import route, but do not approve or regenerate
from the current candidate yet. The checkerboard matting and four-direction
alignment are already usable. The next work must close two narrower gaps:

1. judge identity and equipment drift relative to the approved frame for the
   same direction, instead of relying only on absolute silhouette heuristics;
2. distinguish the pixels' actual producer from the Provider/model binding
   reserved for later action generation.

Global appearance thresholds must not be loosened to make the current candidate
pass. The current import Job
`01447a14-791d-4ced-8f5f-990076e6b200` remains `awaiting_review` and unapproved.

## Implementation outcome

- External imports now write `direction-grid-lock@1.2.0` with typed
  `external_import` producer provenance and a separate `downstreamBinding`.
- `direction-grid-import-consistency@1.0.0` binds every source/candidate
  direction pair and records identity, regional detail, silhouette, torso-line,
  body scale, center, and foot-baseline evidence.
- One shared source-relative scale plus per-direction translation corrects the
  previously hidden whole-character size/baseline mismatch before assessment.
- Checkerboard cleanup now writes `checkerboard-sheet-matting@1.1.0`, soft Alpha
  decontamination, final `alpha-edge-halo@1.0.0`, three background composites,
  and a four-row native review package.
- Sheet and named four-file input modes are both supported and hash-bound.
- Final local replay Job `a045e4fc-b457-4618-b5fe-2aaf3bc33ac0` made zero
  Provider requests, used no authorization, preserved source tree hash
  `785da20140de9aff17be792f870dd010ec564f289e8baa60eef9eb7fe5caddbc`,
  and wrote no approval, Pack, or Godot artifact.
- The replay corrected all four body heights to 0.990–1.010×, widths to
  0.967–0.989×, centers to at most 0.5 px, and foot baselines to 0 px. It still
  rejected front/rear because the generated candidate contains new linear torso
  details; this is the intended gate result, not a threshold failure.
- The separately authorized Phase 3 experiment completed exactly four built-in
  image generations, each from its approved same-direction frame. All four raw
  outputs were RGB with a baked checkerboard. Four-file import Job
  `8cff3609-1117-424b-8e70-8b499929480d` made zero Forge Provider requests and
  stopped at `alpha_edge_dark_outline_discontinuity` before identity approval,
  Pack, or Godot. No fifth generation or targeted replacement was attempted.

## Observed boundary

The current imported sheet already satisfies the local structural contract:

- four fixed front/rear/right/left directions;
- empty hands and neutral idle poses;
- transparent border with no remaining checkerboard or gray platform;
- approximately 0.54% source-scale drift;
- sub-pixel center and foot-baseline drift;
- zero Provider requests and an unchanged approved source Job.

It is not ready for approval because the candidate changes identity-bearing
details: the face/expression differs, the front view introduces or changes a
diagonal chest strap, and belt, sleeve, cape, and clothing details are not fully
stable. The current `direction_grid_undeclared_object_evidence` result also
conflates legitimate cape/side silhouettes with newly introduced props.

## Scope

### P0 — Same-direction relative consistency

Add `direction-grid-import-consistency@1.0.0`. For each imported frame, compare
it with the corresponding approved DirectionGrid frame:

| Candidate | Approved authority |
| --- | --- |
| front | front |
| rear | rear |
| right | right |
| left | left |

The report must bind the source and candidate path/SHA for every pair and record:

- identity metric and palette deltas;
- silhouette delta after the existing shared scale/baseline normalization;
- face/hair, hood, scarf, chest strap, belt/pouch, sleeve, cape, hand, and
  exterior-object evidence;
- the absolute appearance result and the source-relative interpretation;
- a per-direction verdict plus aggregate verdict.

Hard-block newly introduced permanent equipment, straps, props, missing
coverings, hand occupation, or direction mismatch. Use `awaiting_review` for
small facial or decorative-detail drift that remains visually ambiguous.
Existing absolute checks stay in force. A silhouette feature already present in
the approved same-direction source, such as a cape edge, must not be reported as
a newly introduced object solely because it is elongated.

### P0 — Producer provenance separation

Evolve the DirectionGrid Lock to a new backward-readable profile with two
separate concepts:

- `producer`: `external_import`, generator name, original/matted sheet SHA,
  import-evidence SHA, and `providerRequestOccurred=false`;
- `downstreamBinding`: the Provider/profile/model that may be used only after
  DirectionGrid approval for later action generation.

The approved source's xAI binding must never imply that xAI produced imported
pixels. Existing Lock profiles remain readable; new imports must write the
separated fields and review must validate them fail-closed.

### P1 — Soft-alpha and halo evidence

Retain the deterministic checkerboard flood boundary, but reconstruct or
preserve soft edge alpha instead of treating fringe removal as sufficient.
Record an `alpha-edge-halo@1.0.0` report produced from composites over white,
black, and high-chroma backgrounds. Reject exposed checker tiles, neutral fringe,
dark outline discontinuities, or materially jagged silhouettes.

The evidence must state that transparency is deterministic post-processing. It
must never describe the source model output as natively transparent when the
input was an opaque checkerboard.

### P1 — Direction-specific external inputs

Extend the local import operation to accept either:

- one fixed 2×2 sheet; or
- four explicitly named files: `front`, `rear`, `right`, and `left`.

The modes are mutually exclusive. The four-file form preserves declared order,
path, and SHA in the Plan fingerprint and import evidence. This enables a future
experiment in which each approved direction is the direct identity anchor for
one independently generated direction. Do not chain one generated direction
into the next, and do not generate walk poses before the new DirectionGrid is
approved.

### P1 — Native review package

Produce one deterministic review contact sheet containing, per direction:

- approved source;
- imported candidate;
- alpha composite;
- silhouette overlay/difference.

The review checklist must explicitly cover face/hair, hood, scarf, chest strap,
belt/pouches, sleeves, cape shape/trim, hands/props, scale/baseline, and edge
alpha. Acceptance must re-hash all source, candidate, report, provenance, and
graph artifacts before writing approval.

## Implementation sequence

### Phase 0 — Freeze the current evidence (completed)

- Keep Job `01447a14-791d-4ced-8f5f-990076e6b200` unapproved.
- Do not mutate its source Job, Lock, generated frames, reports, or contact sheet.
- Do not export a Pack or invoke Godot from this candidate.

### Phase 1 — Offline contracts and gates (completed)

- Add the source-relative report types, schema, typed validation, and CLI schema
  registry entries.
- Add producer/downstream binding fields with legacy read compatibility.
- Add halo/composite analysis and the deterministic review package.
- Extend the external import request with the mutually exclusive four-file form.
- Re-run formatting, clippy, workspace tests, grid contracts, and CLI product
  tests without a Provider credential or network request.

### Phase 2 — Zero-request replay (completed)

Replay the existing external sheet through the new local gates into a new child
Job. Expected Provider request estimate and maximum are `0 / 0`; authorization
must be absent. The original import Job and approved source remain immutable.

The replay is expected to remain `awaiting_review` or become rejected because of
the face, chest-strap, and clothing-detail drift. That result is useful
calibration evidence and must not be converted to success by relaxing gates.

### Phase 3 — Optional new image experiment (completed and rejected)

Only after separate user authorization, create four independent direction
candidates, each anchored to its approved same-direction frame. Assemble and
import them locally. The experiment stops after one candidate per direction and,
if explicitly authorized, one targeted replacement for a failed direction.

Codex built-in image generation remained an external preview producer rather
than a hidden Forge Provider. The authorized run completed four requests with
no retries, but all four results baked the checkerboard into RGB pixels and the
local Alpha edge report blocked the assembled import. If production OpenAI API
generation is desired, add a named Provider adapter with the ordinary Plan,
authorization, request ledger, model pinning, cost cap, and immutable evidence
contract.

### Phase 4 — Approval and downstream work (pending)

Approve only when automatic gates and native review both pass. Action-frame,
video, Pack, and Godot work require a later, separate Plan and authorization.
They are not part of this remediation.

## Required regression coverage

- Byte-identical approved frames pass the relative comparison.
- An added diagonal chest strap fails.
- A changed face or missing hood/cape cannot be marked `game_ready`.
- A cape edge present in both source and candidate does not become a false prop.
- A genuinely new weapon, platform, or hand-held object fails.
- Swapped right/left or front/rear inputs fail.
- Four-file input path, order, or SHA tampering fails before processing.
- Opaque checkerboard, colored border, residual neutral fringe, and halo cases
  fail deterministically; clean native alpha remains supported.
- New producer provenance cannot be interpreted as a Provider request.
- Legacy Lock profiles remain readable, while new Lock tampering fails review.
- Re-review after any source or candidate byte change fails before approval.
- Import and replay keep Provider usage at zero and leave the source tree hash
  unchanged.

## Acceptance criteria

The remediation is complete only when:

1. all four directions match their approved directional authority without new
   permanent identity or equipment details;
2. absolute and source-relative appearance reports are both acceptable;
3. matting, alpha-edge, scale, center, and baseline gates pass;
4. producer provenance is explicit and downstream model binding is separate;
5. the review bundle is complete and hash-bound;
6. import/replay records zero Provider requests and no authorization;
7. the approved source is unchanged; and
8. no Pack or Godot artifact exists before explicit DirectionGrid approval.

## Recommended next action

Do not approve either rejected candidate. Phase 3 proved that independent
same-direction anchoring fixes the direction dependency but did not preserve
native Alpha or strict clothing identity. Do not issue a fifth built-in request
or generate actions, Pack, or Godot output. A different generation strategy or
named production Provider route requires a new reviewed Plan and authorization;
it must still pass these gates and native review.
