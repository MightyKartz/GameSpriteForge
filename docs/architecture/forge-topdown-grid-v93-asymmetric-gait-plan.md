# Forge `topdown-grid@9.3.0` asymmetric-gait remediation plan

Date: 2026-08-12  
Status: implemented; offline accepted; real-source Plan/auth preflight accepted; real execution not authorized  
Real Provider scope: **forbidden in this implementation**

## Decision

Keep the approved V9 DirectionGridLock and the accepted bytes for
`walk_down` frames 0, 1, and 3. Remediate only frame 2 from the original V9.2
failed Job `0048ce3b-a0ea-4a63-8ed9-9063d01e9043`. Do not chain from child Job
`d080b43c-bb35-4fc0-8f46-ba33aa1a8062`, because it already consumed V9.2's
single laterality fresh retry.

V9.3 is a new workflow version, rather than a silent V9.2 prompt change. The
real V9.2 evidence showed two independent control failures:

1. the Provider prompt began with the ambiguous anatomical phrase
   `right contact` before specifying viewer-space `SCREEN-RIGHT`;
2. both guide legs used the same grayscale/shape vocabulary, with only a small
   vertical difference that the model could ignore.

## V9.3 control contract

```text
approved front DirectionAnchor
├── reused frame 0 + legacy PoseStructure V1.0
├── reused frame 1 + legacy PoseStructure V1.0
├── fresh frame 2
│   ├── Reference 1: immutable DirectionAnchor
│   └── Reference 2: asymmetric PoseStructure V1.1
│       ├── screen-right grounded: bright / thick / wide horizontal sole
│       └── screen-left raised: dark / thin / compact lifted boot
└── reused frame 3 + legacy PoseStructure V1.0
    ↓
identity / direction / equipment / guide-leak gates
    ↓
motion-semantics + gait-laterality (unchanged thresholds)
    ↓
native review only; no Pack or Godot
```

Provider text uses only `screen_left_contact`, `screen_left_support`,
`screen_right_contact`, and `screen_right_support`. It defines screen side by
image x-position around the torso center and never emits the anatomical phase
label. Grayscale values encode motion roles only and may not be copied into
the rendered sprite.

`grid-pose-structure@1.1.0` is deterministic, transparent, grayscale, and has
an exact ink palette. Its validator requires the correct viewer-space role
sides, a visibly raised swing boot, a wide grounded sole, no opaque panel, no
unknown ink, and no floor-like divider. V9.2 continues to generate the legacy
`grid-pose-structure@1.0.0`; historical evidence is not reinterpreted.

## Evidence and compatibility

New V9.3 Action Reports use `grid-keyframe-action-report@1.3.0` and record
`poseStructureProfile` per frame. This is deliberately per-frame because the
one-frame child mixes three byte-reused V1.0 guides with one newly generated
V1.1 guide. V9.2 `@1.1.0` / `@1.2.0` reports remain readable and must not gain
new fields retroactively.

The selected WorkflowGraph node uses
`grid-structured-keyframe@1.2.0`, includes ordered anchor/guide SHA-256 inputs,
and records one Provider request. Reused nodes use the new auditable local-copy
implementation `grid-byte-reuse@1.1.0` while preserving the source sprite bytes
and legacy PoseStructure profile. Their ordered inputs are source sprite
path/SHA, DirectionAnchor, and the child-local legacy PoseStructure; their
cache key binds every ordered input path and hash, and they set `cacheHit=true`
and `providerRequest=false`. Provider manifest and
cache metadata carry the same per-frame profile and hashes.

Source frames and PoseStructures must stay inside the immutable source Job and
match their recorded SHA-256 values at Plan time. A changed guide fails before
any Provider request. The fixture Provider derives leg side from the actual
PoseStructure pixels; it no longer passes the contract merely by parsing a
phase number from the prompt.

## Request and delivery boundary

- source: original V9.2 failed Job with the unique recommendation `[2]`;
- retry target: exactly `walk_down:frame:2`;
- expected / maximum Provider requests: `1 / 1`;
- maximum Provider operations: exactly `1`;
- exact authorization: one model, one `walk_down:frame:2` target, one request,
  `1.4B` total/reserved cost ticks, source lineage root, exact pending-Plan
  recipe/input hashes, exact reviewed manifest SHA-256, empty ledger;
- references: two images, ordered DirectionAnchor then PoseStructure;
- failed frame as input: forbidden;
- additional automatic retry: forbidden;
- videos / private uploads / Pack / catalog / Godot: `0` / prohibited;
- real calls during implementation: `0`.

The Provider session derives authorization policy from one immutable manifest
snapshot. The Job records that snapshot's SHA-256, and any durable-manifest
replacement before reservation, during HTTP response handling, or before a
ledger transition fails closed. In particular, a reviewed one-operation grant
cannot regain transparent 401/429 retries or URL-fallback GET through a
mid-flight manifest edit.

CLI preparation is explicit through `job retry --asymmetric-gait`; execution
uses the resulting pending Plan token with a newly created hash-bound
authorization. An ordinary V9.2 retry keeps historical behavior. A source already marked
`laterality_fresh_retry` or `asymmetric_guide_fresh_retry` cannot create
another fresh child. Staging atomically reserves a source-scoped slot, so
separately prepared or concurrent Plans can create at most one V9.3 child.

V9.3 has no initial 4/8 generation route. Plan validation requires a legacy
V9.2 source and the explicit frame-2 child shape before any request estimate is
issued; this prevents a fresh four-frame V9.3 Job from spending requests and
then failing its one-asymmetric-plus-three-reuse report contract.

## Release gate

1. all four V1.1 guides pass exact palette, side, lift, sole, transparency and
   no-floor-line validation and have distinct hashes;
2. fixture success follows actual guide pixels and reaches review with exactly
   one request and three byte-reused frames;
3. a fixture that ignores the guide remains wrong-sided and fails with stable
   `walk_laterality_not_alternating`;
4. source guide tampering fails during Plan preparation with zero requests;
5. xAI two-reference serialization preserves exact anchor-then-guide order;
6. V9.0/V9.1/V9.2 Grid regression, Core/Providers/CLI tests, product scripts,
   formatting, Clippy, schema/example JSON, credential scan and diff check pass;
7. a read-only real-source preflight creates a 1/1 pending Plan and stops at
   acceptance without creating a Job or changing any authorization ledger.

The read-only source preflight completed on 2026-08-13 against the eligible
V9.2 failure source. It retained identical source tree and Job-record aggregate
hashes, left 10 source Jobs and no source claim registry, and created only an
isolated pending Plan plus an empty-ledger, exact-scope grant. It did not create
or stage a Job, start a worker, mutate a real authorization store, or make a
Provider/network call. See
[`forge-topdown-grid-v93-real-source-preflight-2026-08-13.md`](../qa/forge-topdown-grid-v93-real-source-preflight-2026-08-13.md).

The final offline release pass also completed the full workspace all-feature
test suite, warnings-as-errors Clippy, the 26-test Grid contract suite, and the
CLI product contract. No real Provider call was introduced by those gates.

Only a later, independent user authorization may execute that pending scope.
Passing a future frame-2 probe permits native review of `walk_down`; it does
not authorize another direction or approve production delivery.
