# Forge consistency V2 and world implementation QA — 2026-08-03

## Scope

This record covers the implementation foundation for the unreleased Character consistency V2,
external vision-component protocol, and versioned world assets while preserving the default
`v0.2.0-cli.1` release surface.

The already completed three-run real-xAI video Character gate remains recorded in
`forge-character-loop-v2-2026-08-03.md`. This run did not claim new real-xAI keyframe acceptance:
the local environment does not expose `XAI_API_KEY`, and no additional Keychain credential read was
triggered during unattended QA.

## Implemented contracts

- `SubjectSpecV1` and immutable `SubjectLockV1`, including 0–2 identity inputs, Style revision,
  Provider/model identity, canonical image and mask hashes, and filesystem-safe revision storage.
- `CharacterAssetSpecV2` with explicit Subject revision and `topdown-keyframes@2.0.0` workflow.
- Semantic Provider image-reference roles: subject identity, style, pose structure, edit target,
  start keyframe, and end keyframe.
- Experimental 32-frame Character workflow: four actions × eight independent frame edits, three
  ordered references per edit, two attempts per frame, single-frame retry, Provider/model locking,
  and no collage fallback.
- `WorkflowGraphV1` with typed dependencies, implementation versions, input/output hashes, cache
  keys, Provider-request markers, and invalidation ranges. Legacy stage manifests remain exported.
- Content-addressed cache with output SHA-256 revalidation, Provider/model isolation, and corrupt
  cache rejection.
- Project asset catalog at `.forge/catalog.json`; Godot installation records remain separate in
  `.forge/assets.json` and can be linked back after a successful install.
- Gray keyframe consistency results export only a `candidate_gsfpack` and require explicit review.
  `regenerate` and `blocked` results never reach Pack export.
- `VisionComponentProtocolV1` fixture plus external stdio contract. Installed manifests use real
  Ed25519 verification, traversal-safe file paths, executable/model SHA-256 checks, and license
  metadata. The real component remains fail-closed and unpublished.
- Split world feature gates: `terrain-assets`, `building-assets`, and `map-compiler`.
- `dual-grid@2.0.0`: two material samples, all 15 non-empty masks, four deterministic variants per
  mask, and cross-variant seam validation.
- `topdown-exterior@2.0.0`: versioned module semantics, anchors, collision, occlusion, entrance,
  Y-sort metadata, and eight deterministic examples.
- `map-compiler@2.0.0`: evaluates all 20 deterministic candidates and selects the highest score
  using the published 25/20/20/15/10/10 rubric; ties choose the lowest candidate index.
- Release workflow manual preflight (`workflow_dispatch`) performs build, tests, FFmpeg license
  checks, packaging, per-file hashes, and SBOM generation without creating a GitHub Release. Tag
  mode validates the `forge-cli` Cargo version before publishing an unsigned archive with Artifact
  Attestation.

## Verification results

All commands below passed from `/Users/kartz/Development/Forge`:

| Gate | Result |
| --- | --- |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --workspace --no-fail-fast` | PASS |
| `bash scripts/test-cli-product.sh` | PASS |
| `bash scripts/test-consistency-v2.sh` | PASS |
| `bash scripts/test-cli-installer.sh` | PASS |
| `bash scripts/test-cli-signing-contract.sh` | PASS |
| `bash scripts/test-world-assets.sh` | PASS, Godot 4.6.3 headless |

The keyframe graph contains 32 Provider frame nodes, four matting nodes, four provisional-alignment
nodes, one collection-consistency node, four loop-quality nodes, shared normalization, and Pack.
Replaying from `collection_consistency` completed with zero Provider requests; retrying one explicit
frame completed with exactly one Provider request.

The default CLI product test confirms Subject, Schema, Component, and World commands do not leak
into the `v0.2.0-cli.1` binary. The feature-enabled consistency test confirms a 32/64 request
estimate, valid Pack/graph/catalog output, and exactly one Provider request for an explicit
single-frame retry. The world test confirms V3 Pack validation and a Godot 4.6.3 headless load.

## Release policy

Apple signing and notarization are not GitHub Release requirements and are not hard gates for the
first CLI version. `v0.2.0-cli.1` is intentionally published as unsigned and not notarized. The
installer requires the external archive SHA-256 and the package's per-file `MANIFEST.sha256`; the
workflow also publishes an SBOM and GitHub Artifact Attestation. Developer ID signing remains a
future commercial-distribution hardening step. The published archive must pass the README curl
installer in a clean Apple Silicon user account.

The `v0.3` promotion gates also remain intentionally open: three fresh real-xAI keyframe
Character→Pack→Godot runs, followed by the frozen 20-character/5-style benchmark. The preview
vision signing key must be replaced by a CI-controlled release key before any component manifest is
published, and SAM 2/DINOv2/LPIPS weights must pass commercial redistribution review.
