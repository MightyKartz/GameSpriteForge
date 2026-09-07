# Forge Topdown Grid V9 Offline Gate

Date: 2026-08-11

Status: P1 implemented and fixture/offline gates passed. **Real xAI acceptance
was not executed and this document must not be read as a real-model gate
pass.** `grid-generation` is disabled by default.

## Workflow

`topdown-grid@9.0.0` now implements:

```text
prompt + StyleLock + SubjectLock
└── direction_grid  1 request / max 2, 2x2 front/back/right/left
    ├── explicit approval bound to Job, lock SHA-256, and all four node hashes
    └── action_grid × 4  4 requests / max 8, one 2x2 four-phase sheet per walk
```

No video requests are used. Full two-stage cost is 5 expected / 10 maximum
image requests. The two stages are separately authorized.

## Implementation evidence

- Direction grid uses the existing fixed 2×2 direction sheet inspector and
  deterministic slicing.
- Action grids use the existing 2×2 animation sheet inspector, background
  cleanup, onion-skin alignment, `motion-semantics@1.1.0`, and equipment gate.
- Pose guidance is controlled by `poseGuidance: enabled|disabled`; the enabled
  path sends `PoseStructure`, the disabled path sends only identity and
  direction references.
- `job retry --frame 0-3` regenerates the target cell and byte-reuses the
  other three cells in a child Job.
- Missing or tampered approval evidence fails closed before Provider calls.
- CLI model resolution locks only the image model for V9; no video model is
  resolved, persisted, or required by either stage.
- The `grid-generation` CLI feature includes the SubjectLock command surface,
  so a feature build can execute Project -> Style -> Subject -> Grid without a
  hidden prerequisite from another feature.
- The default CLI rejects `topdown-grid@9.0.0`; `--help` does not expose
  `topdown-grid` or `grid-generation`.

## Files

- `packages/core/src/character_grid.rs`
- `packages/core/src/automation/plan.rs`
- `packages/core/src/automation/runner.rs`
- `packages/cli/src/main.rs`
- `packages/providers/tests/grid_generation_contract.rs`
- `examples/cli/character-v2-grid.json`
- `schemas/character-direction-grid-lock.schema.json`
- `schemas/character-direction-grid-approval.schema.json`
- `schemas/grid-action-report.schema.json`
- `scripts/test-grid-generation.sh`

## Offline verification

`scripts/test-grid-generation.sh` completed successfully:

- Feature-off gate test: passed.
- Feature-on gate test: passed.
- CLI default and feature builds: passed.
- Provider grid contract: 3/3 passed.
  - two-stage direction/action generation;
  - 5 expected / 10 maximum request accounting;
  - zero video requests;
  - Pack validation and Godot 4.6 load when available;
  - targeted frame retry reuses three cells byte-for-byte;
  - tampered approval fails before Provider requests;
  - collapsed four-phase fixture is blocked by `motion-semantics`;
  - pose-guidance disabled sends only two references.
  - Pack and Godot text artifacts contain no Authorization header, Bearer token,
    access/refresh token, temporary URL, or absolute JobStore path.
- Default `--help` contains neither `topdown-grid` nor `grid-generation`.
- Feature-build CLI model-resolution test proves V9 leaves `videoModel` empty.
- Feature-build help exposes `forge subject create`; the default build remains
  free of the experimental grid workflow.

## Boundaries

- V1–V8 workflows were not removed or reordered.
- No real Provider key, OAuth token, request, or temporary URL was used.
- No threshold was lowered.
- This is fixture evidence only; real xAI acceptance requires a separate
  budget and approval.
