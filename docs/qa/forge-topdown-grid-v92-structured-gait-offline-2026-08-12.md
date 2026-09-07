# Forge V9.2 structured-gait offline acceptance

Date: 2026-08-12  
Workflow: `topdown-grid@9.2.0`  
Provider: fixture only  
Real requests: **0**  
Verdict: **offline accepted; real acceptance pending**

## Outcome

V9.2 fixes the failure demonstrated by rejected V9.1 real Job
`7097f67e-0e85-4ead-b5bc-c6c908f4abda`: four raster-distinct poses could all
place the same viewer-left boot at the contact extreme. The approved V9.0
DirectionGridLock remains immutable and was not regenerated or modified.

The first V9.2 surface accepts only a validation-only `walk_down` against an
approved V9.0 Direction Grid. It cannot export a Pack, install into Godot,
generate another direction, or authorize a video target.

## Gait laterality gate

`gait-laterality@1.0.0` records viewer-space boot-baseline extrema, contact
signal, expected and observed side, confidence, opposite-pair deltas, verdict,
stable reasons, and exact retry frames.

- frames 0/1 require screen-left contact/support;
- frames 2/3 require screen-right contact/support;
- opposite pairs require opposite signed signals and a minimum delta;
- ambiguity fails closed;
- same-side collapse returns `walk_laterality_not_alternating` and recommends
  only frames 2 and 3.

The real V9.1 rejected frames are an optional local paid-pixel regression
fixture. All four have the same positive screen-left signal and are rejected
with retry frames `[2, 3]`. Synthetic alternating, ambiguous, and same-side
fixtures are repository-independent mandatory tests. Lower-half area remains
diagnostic only because Ayla's full cape can dominate it; the hard gate uses
boot baseline extrema.

## Grayscale PoseStructure

Every attempt creates a hash-bound, transparent, grayscale-only structure
image with four distinct phase geometries. It contains no cyan/magenta colors,
labels, floor line, cell border, opaque panel, or appearance content.

Reference order is contract-tested:

- fresh: `DirectionAnchor`, `PoseStructure`;
- diagnostic: `EditTarget`, `DirectionAnchor`, `PoseStructure`.

The Provider must advertise at least three image references before any
request. Unknown or smaller capacity fails locally. PoseStructure path and
SHA-256 are recorded in each frame, Job artifacts, Provider manifest, cache
input, and WorkflowGraph. Literal guide copying is a hard static failure.

V9.1 continues to write `grid-keyframe-action-report@1.0.0`. Only V9.2 writes
`@1.1.0`, which requires laterality and PoseStructure evidence. V9.1 child
retry rejects V9.2 reports, preventing cross-version evidence reuse.

## Plan, execution, and failure closure

- Plan estimate: 4 expected / 8 maximum image edits.
- Authorization: exactly `walk_down:frame:0..3`.
- Successful fixture: 4 requests, Job stops at `AwaitingReview`, no Pack.
- Same-side fixture: four first attempts plus only frame 2/3 retries, 6 total
  requests, stable error `walk_laterality_not_alternating`.
- Failed Job preserves raw attempts, cleanup, PoseStructures, motion,
  equipment, laterality, action report, usage, manifest, and WorkflowGraph.
- Full generation, `walk_up`, non-grayscale guidance, and unrestricted child
  retry expansion were rejected during this initial Plan gate. A later
  remediation now permits only the unique evidence-selected laterality frame
  under a 1/1 child validation contract; see
  `forge-topdown-grid-v92-laterality-fresh-retry-offline-2026-08-12.md`.

## Deterministic pair-transform spike

The zero-request lower-body mirror spike can invert contact while preserving
every pixel above its split:

```json
{
  "sourceContactSignal": 0.05238095,
  "transformedContactSignal": -0.05238095,
  "upperBodyChangedPixels": 0,
  "lowerBodyChangedRatio": 0.6338889,
  "boundarySeamChangedRatio": 0.42962962
}
```

The 42.96% changed-pixel ratio across the four-row split band is an
unacceptable garment/leg seam risk. The spike remains disabled and is not a
production fallback or Provider-frame replacement.

## Verification

- Grid Provider contracts: 19/19 passed, including V9.0/V9.1 Pack and Godot
  regressions plus three V9.2 contracts.
- Core with `grid-generation`: 279/279 unit tests and all integration suites
  passed.
- CLI with `grid-generation`: 16/16 passed.
- `scripts/test-grid-generation.sh`: passed.
- default `scripts/test-cli-product.sh`: passed.
- `cargo fmt --all -- --check`: passed.
- Core/Providers/CLI all-target Clippy with `grid-generation` and
  `-D warnings`: passed.
- Real Provider environment was never enabled; 0 paid requests and 0 ticks.

## Release boundary

Offline acceptance does not imply xAI acceptance. Do not authorize another
direction, Pack, video, or Godot output. The only permissible next paid gate
is a separately authorized V9.2 `walk_down` validation probe with 4 expected /
8 maximum image requests followed by native-size review. The approved
DirectionGridLock must remain frozen.
