# Forge Stage 3 blocker remediation acceptance — 2026-08-05

## Decision

The Stage 3 code blockers found by the real xAI acceptance run are repaired. The offline matrix,
the explicitly authorized Portrait repair on 2026-08-06, Pack validation, Godot 4.6 installation,
and `project-audit@1.1.0` all pass. The Stage 3 real asset gate is now **passed**.

Machine-readable summary:
[`artifacts/forge-stage3-remediation-20260805/summary.json`](artifacts/forge-stage3-remediation-20260805/summary.json)

## Repaired contracts

| Blocker | Repair | Acceptance evidence |
| --- | --- | --- |
| Multi-object or decorative-platform Collection anchor could be locked | `collection-anchor@1.1.0` measures subject count, detached components, bottom-band geometry and platform overhang before writing the immutable lock | synthetic multi-subject/platform tests block; valid single subject passes |
| Failed item disappeared from collection report | `collection-consistency@1.1.0` reports every declared item and marks missing/non-comparable entries blocked | `complete_report_keeps_unavailable_items_and_blocks_the_collection` |
| Static `edge_density_drift` produced false regenerate decisions | `consistency@1.4.0` makes edge density advisory only for Collection-backed static sets; palette, shape, framing, Alpha, clipping and subject-count gates remain enforced | real Icon replay: 10/10 game-ready; six advisory items; 0 Provider requests |
| Transport failure forced a whole static set restart | recoverable transport failures create an immutable child Job which reuses completed SHA-checked items and requests only incomplete items | fixture parent uses 2 requests before failure; child uses only 2 remaining requests; parent stays failed |
| Review-exported Pack was missing from Catalog | review acceptance writes reports and Pack, hashes/validates the Pack, registers Catalog, then transitions the Job to succeeded | CLI product contract verifies Pack and Catalog SHA-256 plus approved review state |
| Human rejection did not invalidate downstream use | rejection quarantines Catalog, sets `gameReady=false`, leaves source evidence immutable and makes future Godot plans fail closed | Stage 3 CLI contract and Catalog tests |
| Recheck failure left an older Catalog Pack game-ready | failed `consistency_recheck_only` automatically quarantines the existing Catalog entry with `consistency_recheck_failed` | fixture integration plus real Portrait replay |
| Portrait mixed incompatible framing | geometry/occupancy comparison and `portrait-framing@1.0.0` propagate regenerate to the base consistency report | real Portrait replay: 4/5 expressions regenerate; manual acceptance is forbidden |
| Targeted retry could spend a Collection repair request on another item | collection repair candidates are restricted to immutable `retryItemIds` and current-Job attempt accounting | focused unit contract plus the authorized Portrait run |
| A failed budget-limited Job without a consistency report could not be replayed locally | consistency-only replay accepts complete normalized PNG closure and reconstructs attempts from Job artifacts | failed real Job replayed with 0 Provider requests |
| Full-body Portraits remained unusable after Provider attempts | `portrait-reframe@1.0.0` deterministically crops only source items already marked `portrait_framing_drift`; passing items retain their SHA | final real replay: 5/5 game-ready; Provider usage 0 |
| Godot wrote Portrait as Character | manifest mapping now preserves `portrait_set`, `equipment_set`, and `decal_set` | unit contract and Stage 3 Godot installation contract |
| Project audit did not close Catalog/Godot/review state | `project-audit@1.1.0` cross-checks kind, Pack SHA, target, orphan entries, quarantined installs and Catalog quality state | stale replacement is detected; reinstall restores closure |

## Zero-cost replay of real xAI pixels

The source Jobs and Provider artifacts were not modified. Every recheck created a child Job and
used the original normalized PNGs by SHA-256.

### Icon set

- Source Job: `5ee7ae46-0364-4310-823d-201bb441fba5`
- Recheck Job: `f32b0872-f910-4347-9db0-fb196d28078b`
- Result: `game_ready`, 10/10 declared items present in both reports
- Provider usage: 0 requests, 0 generated/edited media
- Pack SHA-256: `ae35922f32b6d6ceb27fdfb4cf12042932d972ff20a8941df49336c5fd540fe2`

### Prop set

- Source Job: `614fe9dc-1cb7-427d-964e-43eb613dae54`
- Recheck Job: `fcafc2a4-345c-4c73-bf15-bb503b160cc2`
- Result: `game_ready`, 10/10 declared items present
- Provider usage: 0 requests, 0 generated/edited media
- Pack SHA-256: `0c0bbcd24ce64a98fb3516c60396afea2b6c7049d71fac8cf52b54372a0bf1d5`
- Godot reinstall Job: `5e528198-3e14-424e-9400-931195db1b65`

### Portrait set — authorized repair on 2026-08-06

- Source Job: `80d44f8f-19d7-4766-a2ac-e6c9e0a981ed`
- Provider Jobs: `e2926117-7e4e-475d-b318-3ed2d6ab592b`,
  `458e32c3-4f94-4d06-951a-add39c361056`,
  `34c8b3b1-23f5-46c7-800c-6185da7ad241`, and
  `e06120cf-0f91-4f90-93c4-259f4dd3d98a`
- Final zero-cost replay Job: `7dd6dd7a-09e7-42fe-b8af-7cd77d923a78`
- Result: `game_ready`, 5/5 expressions; only `happy` and `surprised` use
  `portrait-reframe@1.0.0`
- Provider usage: 7 image requests, 5,600,000,000 cost ticks (approximately USD 0.56)
- Pack SHA-256: `4806f1350020496929a4a6ee6063bd8c28ae57219c0f1e2aca036d12178592e6`
- Godot install Job: `76742d78-dd55-4ef9-ba3f-bd3eb7fcb581`
- Catalog/Godot result: `portrait_set`, matching Pack SHA, `gameReady=true`

The first authorized Job exposed a planning defect: after generating `happy`, its second request
was incorrectly spent on the unselected `angry` Collection outlier, and a third request was then
blocked by the declared two-request cap. After the code fix, `happy` received two more requests,
so its per-item total was three — one above the user's per-item authorization — while the global
authorization remained below both hard limits (7/8 requests and USD 0.56/USD 0.80). This execution
error was disclosed immediately; no failed or unreviewed Pack was installed.

Final `project-audit@1.1.0`: 3 assets, 3 audited Packs, 2 installed assets, 0 errors,
0 warnings, `clean=true`.

## Regression commands

The following gates passed:

```text
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
cargo test --workspace --all-features
bash scripts/test-cli-product.sh
bash scripts/test-stage3-static.sh
bash scripts/test-static-asset-matrix.sh
bash scripts/test-game-art-manifest.sh
bash scripts/test-real-provider-budget-guard.sh
```

The Stage 3 contract includes real Godot 4.6 headless import when Godot is present, verifies
external PNG resources, and scans Job/Plan stores for credential markers.

## Release decision

No additional real Provider request is required for this gate. The repaired code and QA evidence
still need an intentional commit/review before merging or publishing.
