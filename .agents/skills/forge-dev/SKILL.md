---
name: forge-dev
description: Develop and validate Forge implementation changes, including its Rust CLI, 2D asset pipeline, and Godot delivery. Use for code maintenance and engineering QA, not routine asset creation or installation.
---

# Forge Dev

## Product boundary

Forge is a local-first, agent-oriented CLI. Codex, Claude, scripts, and CI use its single-JSON stdout protocol to create versioned image assets and install them into Godot. The desktop and MCP source may remain in the repository for history, but they are not part of the default workspace, release surface, or current implementation route.

Forge owns image generation orchestration, immutable locks, deterministic processing, quality/consistency gates, `.gsfpack`, provenance, project audit, and Godot delivery. Game code and gameplay logic belong to Codex/Claude and the engine project.

Identify the affected workflow and Cargo features before editing. Use the
[workflow and release boundaries](../../../docs/architecture/forge-workflow-boundaries.md)
for current capability scope, and check it against the source. Keep dated acceptance
results in QA reports rather than copying release status or test counts into this skill.

## Core rules

- Preserve the CLI JSON envelope, durable JobStore, single-use plan tokens, stable error codes, and async-by-default behavior.
- Keep providers model-independent. A Job locks one Provider/profile/model; never switch silently.
- Never put API keys, OAuth tokens, Device Codes, Authorization headers, or temporary media URLs in jobs, packs, normal logs, CLI JSON, or Godot projects.
- Keep real-provider execution behind explicit acceptance and request/cost limits. Fixture tests must remain fully offline.
- Provider output must be materialized inside the Job workspace, validated, and SHA-256 hashed before deterministic processing.
- Retry only the failed animation, frame, or static item. Local matting/loop/consistency/replace replay must make zero Provider requests.
- Never mutate a source Job or source Pack. Retry, replay, and replace create child Jobs with a source chain.
- Godot output uses external PNG/atlas resources. `.tres/.tscn` must stay below 1 MiB and contain no embedded Image, `PackedByteArray`, or `ImageTexture.create_from_image`.
- Preserve unrelated user changes and excluded desktop/MCP source.

## Targeted frame retry

Preserve unselected frames and the source Job/Pack. Keep appearance/direction authority
references distinct from previous-frame continuity references; continuity must not
override the requested action phase. Initial fixture generation and repair must model
the same pose semantics. Do not make a regression pass by relaxing quality thresholds.

When changing frame retry, verify the workflow's request scope, reference roles, source
hashes, and byte reuse of unselected frames. The focused V4 contract is
`cargo test -p providers --test locked_frames_generation_contract`; its particular
action, frame count, and request count are not universal rules for other workflows.

## Godot installation transaction

- Validate the engine version and stage source textures before replacing installed assets.
- Resources, the project manifest, and the optional catalog share a commit/rollback
  boundary. Any ordinary error after target modification, including registration or
  final JobStore writes, restores their prior state; a failed first install leaves no
  partial target or new registration. Preserve recovery evidence and report rollback errors.
- Hold registration locks from snapshot through commit or rollback. Use a consistent
  lock order, and do not call an API that reacquires a lock already held by the transaction.
- Refuse replacement when the existing target cannot be safely and completely backed up.
  Ordinary-error rollback does not establish recovery after process termination or a crash.
- Keep Rust orchestration and Godot resource generation within the same contract. For
  installation changes, run `cargo test -p core --lib godot_install` and the applicable
  CLI/asset delivery checks; mocked engine failures do not replace actual Godot validation.

## Repository routes

Paths below are relative to the repository root.

- CLI: `packages/cli`
- deterministic processing, locks, Jobs, audit: `packages/core`
- Pack validation: `packages/pack`
- Provider adapters and fixture: `packages/providers`
- Godot installation orchestration and rollback: `packages/core/src/automation/runner/godot_install.rs`
- Godot native resource generation: `scripts/godot/install_forge_pack.gd`
- Project registration and catalog locking: `packages/core/src/project/mod.rs`, `packages/core/src/catalog.rs`
- schemas: `schemas`
- QA evidence: `docs/qa`
- implementation plans: `docs/architecture`

## Verification

Choose the smallest meaningful test while iterating. For Rust/CLI changes, run the
applicable default gates below. Documentation/instruction-only changes need content,
path/link, and skill-format validation, not a full Rust rebuild or test run.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/test-cli-product.sh
```

Build/test the exact affected feature surface as well; a default build does not exercise
gated CLI commands. Use this routing table and
[CONTRIBUTING.md](../../../CONTRIBUTING.md) for the relevant checks:

| Changed surface | Verification entry |
| --- | --- |
| Full existing release matrix, including Consistency/World contracts | `bash scripts/test-v03-release-matrix.sh` |
| GameArtManifest / project build / Stage 3 collection assets | `FORGE_EXPERIMENTAL_SUITE=project-assets bash scripts/test-experimental-feature-matrix.sh` |
| Subject import / Grid generation | `FORGE_EXPERIMENTAL_SUITE=subject-grid bash scripts/test-experimental-feature-matrix.sh` |
| Pixel delivery / identity metric processing | `FORGE_EXPERIMENTAL_SUITE=processing bash scripts/test-experimental-feature-matrix.sh` |
| Focused World changes | `cargo test -p forge-cli --features world-assets`; `cargo test -p providers --test world_generation_contract`; `bash scripts/test-world-assets.sh` |

The experimental script accepts only the three named suites; `release` and `world`
are not suite values. These are fixture/offline contracts. Use `CARGO_NET_OFFLINE=true`
when dependencies are cached; an absent cache is an environment issue, not a passing check.

All three experimental suites require jq. Project-assets and subject-grid also require
FFmpeg, ffprobe, and Godot. Their preflight
uses `FORGE_GODOT_PATH` or `/Applications/Godot.app/Contents/MacOS/Godot`; subject-grid
also requires that fixed application path for its current integration tests. Processing
does not require media tools. The delivery suites enforce `FORGE_REQUIRE_GODOT=1`.
Follow the workflow's provisioning on CI and report missing local tools explicitly.

Run feature-switching CLI scripts sequentially when sharing `target/debug/forge`.
Verify that filtered tests actually run. Report failed, missing, ignored, and unexecuted
checks accurately; none establish acceptance. Some media tests return early when tools
are absent even though the Rust summary reports success; confirm prerequisites and
actual engine/media execution before claiming that coverage. Read the experimental
JSON reports' unverified-evidence entries for real-provider and calibration scope.
Unit tests and evaluator compilation do not prove human calibration accuracy.

Real-provider acceptance is never implied by fixture success. Record fixture and real-provider gates separately under `docs/qa/`, including request estimates, actual usage, Pack/Godot results, credential scans, and any manual native-size review.
