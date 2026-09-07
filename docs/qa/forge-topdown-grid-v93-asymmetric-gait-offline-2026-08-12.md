# Forge `topdown-grid@9.3.0` asymmetric-gait offline acceptance

Date: 2026-08-12  
Verdict: **offline accepted; real Provider execution not performed**

## Scope and decision

V9.3 preserves the user-approved DirectionGridLock and the accepted bytes for
`walk_down` frames 0, 1, and 3. It remediates only frame 2 from original failed
V9.2 Job `0048ce3b-a0ea-4a63-8ed9-9063d01e9043`. Child
`d080b43c-bb35-4fc0-8f46-ba33aa1a8062` is not an eligible source because it
already consumed the ordinary laterality-fresh attempt. This acceptance used
fixture Providers only and made zero network or real Provider calls.

## Implemented contract

- Workflow: `topdown-grid@9.3.0`, validation-only `walk_down`.
- Target: exactly `walk_down:frame:2`; Plan estimate and maximum are `1 / 1`.
- Provider references: exact order `DirectionAnchor`, then
  `grid-pose-structure@1.1.0`; no EditTarget, rejected raster,
  `inputFrameSha256`, or `replacesFrameSha256`.
- Frame 2 method/profile: `asymmetric_guide_fresh_retry` /
  `grid-pose-structure@1.1.0`; screen-right support is bright, thick, and has a
  wide horizontal sole; screen-left swing is dark, thin, and visibly lifted.
- Frames 0/1/3: byte-identical source sprites plus legacy V1.0 guides.
- Evidence: Action Report `grid-keyframe-action-report@1.3.0`; selected graph
  node `grid-structured-keyframe@1.2.0`; reused nodes
  `grid-byte-reuse@1.1.0` with an independently recomputable cache key over
  ordered source-sprite/anchor/guide paths and hashes,
  `cacheHit=true`, and `providerRequest=false`; manifest records Action Report
  profile/path/SHA, per-frame reuse and PoseStructure profiles.
- Authorization: a new independent empty-ledger grant is required before
  staging; exact one model, one target, one model request, one Provider
  operation, `1.4B` total/reserved ticks, source lineage root, and the pending
  Plan's exact recipe/input hashes. The Job also persists the SHA-256 of the
  exact authorization-manifest bytes reviewed at attachment; Provider binding,
  reservation, ledger transitions, send/retry policy, and response fallback
  fail closed if that manifest changes.
- Network boundary: an exact-one-operation xAI grant disables automatic
  401/429 resend and URL-fallback GET; edited references are read once, checked
  against their SHA-256, and those same bytes are encoded for the sole POST.
- Execution: Job run atomically claims `Queued -> Running`; a Job cannot run
  twice. JobStore atomically permits only one V9.3 child per source/frame scope.

## Positive and negative evidence

The fixture success path issued exactly one image edit, preserved three frame
and guide hashes, passed structure/motion/equipment/laterality gates, stopped
at native review, and wrote no Pack or Godot output.

The independent negative path deliberately ignored the asymmetric guide. It
issued one request and remained blocked with stable
`walk_laterality_not_alternating`, proving the gate does not pass merely from
prompt text. Source PoseStructure tampering fails during Plan preparation with
zero Provider calls. A second sibling from the same source fails at staging;
a V9.3 result cannot chain another fresh child. The xAI adapter contract keeps
the two-reference anchor-then-guide payload order.

Backward compatibility is explicit: historical V9.2 Action Reports @1.1/@1.2
remain readable without retroactive profile fields; V9.3 @1.3 requires the
mixed per-frame profiles and exactly one frame-2 asymmetric retry plus three
byte reuses.

## Offline gates

The release evidence is produced by:

```text
scripts/test-grid-generation.sh
cargo test -p core
cargo test -p providers --features grid-generation
cargo test -p forge-cli --features grid-generation
cargo clippy -p core -p providers -p forge-cli --all-targets --features grid-generation -- -D warnings
cargo fmt --all -- --check
git diff --check
```

The focused contracts additionally cover concurrent single execution,
concurrent source-scope claiming, the exact authorization mutation matrix,
deterministic guide validation, leak rejection, schema aliases and Provider
reference serialization.

The authorization race regressions also replace the manifest after exact
scope validation and while a 401 response is in flight. The first case fails
before reservation with an empty ledger; the second performs exactly one POST,
does not refresh credentials or retry, and leaves the submitted reservation
conservatively consumed.

Final release-gate revalidation on 2026-08-13 also passed
`cargo test --workspace --all-features` (including the historical V7/V8/V9
and targeted-frame compatibility contracts),
`cargo clippy --workspace --all-targets --all-features -- -D warnings`, the
full 26-test Grid contract suite, and `scripts/test-cli-product.sh`. Two
feature-union regressions found during that run were fixed without weakening
quality thresholds: a targeted legacy frame now keeps the same shared-sheet
resampling contract as its three immutable peers, and V7 static-idle canvas
matching no longer performs a second palette quantization after the approved
DirectionPoseLock has already crossed the pixel-delivery boundary.

## Real-source preflight boundary

The separate read-only preflight has prepared (not claimed or executed) a
fresh V9.3 Plan against Job `0048ce3b-a0ea-4a63-8ed9-9063d01e9043`. It proved
the `1 / 1` estimate, unchanged source/lock hashes, no new Job, zero
request-ledger entries, and zero Provider/network calls. Its source hashes,
exact Plan/auth bindings, and non-action evidence are recorded in
[`forge-topdown-grid-v93-real-source-preflight-2026-08-13.md`](forge-topdown-grid-v93-real-source-preflight-2026-08-13.md).
No real probe is authorized by this offline acceptance or preflight.
