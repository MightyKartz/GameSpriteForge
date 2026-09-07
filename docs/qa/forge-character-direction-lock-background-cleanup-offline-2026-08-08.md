# Forge Character DirectionLock/background cleanup offline acceptance

Date: 2026-08-08

## Verdict

`topdown-keyframes@2.3.0` passes the offline implementation gate for the two
defects exposed by the V2.2 real run. It is not promoted as real-xAI ready: no
network model request was authorized or made during this acceptance.

Machine evidence: [summary](artifacts/forge-character-direction-lock-background-cleanup-20260808/summary.json),
[DirectionLock](artifacts/forge-character-direction-lock-background-cleanup-20260808/reports/direction-lock.json),
[Provider manifest](artifacts/forge-character-direction-lock-background-cleanup-20260808/reports/keyframe-provider-manifest.json),
[WorkflowGraph](artifacts/forge-character-direction-lock-background-cleanup-20260808/reports/workflow-graph.json),
[portable Pack lock](artifacts/forge-character-direction-lock-background-cleanup-20260808/pack/character-direction-lock.json),
and [fixture preview](artifacts/forge-character-direction-lock-background-cleanup-20260808/pack/preview.gif).

## Contract evidence

| Gate | Result | Evidence |
| --- | --- | --- |
| Versioned direction lock | Pass | Three `game_ready` entries: front=`idle:0`, rear=`walk_up:0`, right=`walk_right:0` |
| No extra direction request | Pass | Plan 32 / maximum 64; direction anchors reuse frame-0 targets |
| Per-frame raw/clean boundary | Pass | 32 Provider nodes, 32 cleanup nodes, 32 normalized frame nodes |
| Downstream cleaned references | Pass | In-between Provider inputs match adjacent normalized output SHA-256 values, not raw hashes |
| Opaque background removal | Pass | Fixture raw green backgrounds have distinct raw/cleaned hashes and transparent output borders |
| Direction failure cost stop | Pass | Wrong-right fixture stops after two frame-0 requests with `direction_lock_required` and durable partial evidence |
| Single-frame retry | Pass | One selected frame creates one Provider request and reuses all unaffected outputs |
| Local replay | Pass | Consistency replay creates zero Provider requests |
| V2.2 compatibility | Pass | Historical V2.2 single-direction execution succeeds without V2.3 nodes or DirectionLock |
| Pack | Pass | Pack V2 validates with portable `character-direction-lock.json`; no host path remains |
| Godot 4.6 | Pass | Headless install/load passes; external resources and `forge_usage.json` provenance verified |
| Credential scan | Pass | Fixture JobStore contains no bearer/API-key/token/device-code patterns |

## Commands run

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p forge-cli --all-targets --features consistency-v2 -- -D warnings
cargo test --workspace --no-fail-fast
cargo test -p providers --test keyframe_generation_contract -- --nocapture
bash scripts/test-consistency-v2.sh
bash scripts/test-cli-product.sh
```

Results:

- Core unit tests: 183 passed.
- Core integration suites: all passed.
- Pack tests: 20 passed.
- Provider unit tests: 32 passed.
- Character video contract: 5 passed.
- V2.3 keyframe/Pack/Godot contract: passed.
- Stage 3 static contract: 4 passed.
- Static and world contracts: passed.
- CLI consistency V2 and default product scripts: passed.
- Clippy with warnings denied: passed for the workspace and exact
  `consistency-v2` CLI feature surface.

## Remaining real gate

A new paid acceptance must be explicitly authorized before calling xAI. The
recommended minimum is one direction at a time, starting with `walk_up` and
`walk_right`, with native-size review of:

- rear/profile facing on all eight frames;
- absence of opaque scene/background pixels before and after cleanup;
- identity, scarf/skin/equipment stability;
- feet/hand anatomy and temporal outline stability;
- request/cost ledger, Pack, Godot and credential scans.

Only after those directions pass should a bounded full 32-frame V2.3 run be
considered. Fixture success is not evidence of real-model visual quality.

