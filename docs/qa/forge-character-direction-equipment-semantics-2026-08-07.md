# Forge Character Direction and Equipment-Effect Acceptance

Date: 2026-08-07

Implementation verdict: **passed**

Existing Character asset verdict: **quarantined; targeted real repair still required**

## Scope

This acceptance validates the new local Character semantic gates against fixtures and the existing
real xAI `Validation Ranger`. It does not authorize or perform any new real Provider request.

Profiles:

- `direction-quality@1.0.0`
- `equipment-effect-consistency@1.0.0`
- Character collection consistency: `consistency@1.6.0`

## Existing real asset replay

Source Job: `1e927928-0ec5-45f1-abce-c0e44fbf417a`

Zero-cost semantic replay Job: `a59c2efe-a639-472c-9204-7d5961d2f0aa`

The child Job reused the existing normalized frames and made no Provider calls:

| Metric | Result |
| --- | ---: |
| Provider requests | 0 |
| Generated images | 0 |
| Generated videos | 0 |
| Edited videos | 0 |
| Private file uploads | 0 |

The authorization ledger remained at 11 requests and 32,600,000,000 observed cost ticks, the same
lineage total recorded before this local replay.

## Semantic result

| Animation | Expected | Face-visible frames | Detached-effect frames | Verdict |
| --- | --- | ---: | ---: | --- |
| `idle` | down/front | 8 / 8 | 0 / 8 | `game_ready` |
| `walk_down` | down/front | 6 / 8 | 0 / 8 | `game_ready` |
| `walk_right` | right/side | 8 / 8 | 0 / 8 | `game_ready` |
| `walk_up` | up/rear | 6 / 8 | 4 / 8 | `blocked` |

`walk_up` failed with both machine reasons:

- `walk_up_face_visible`
- `unexpected_detached_emissive_effect`

The detector found as many as three detached emissive components in an affected frame. This
matches the visual observation that the supposed upward animation remains front-facing and that
staff sparks appear only in part of the cycle.

## Enforcement result

- The child Job produced no `.gsfpack`.
- `forge job review --accept` returned `hard_failure_not_reviewable` and created no decision file.
- The project catalog marks `validation-ranger` as `gameReady: false`, with
  `qualityVerdict: character_semantic_recheck_failed` and review status `quarantined`.
- The project audit intentionally reports two errors until repair: `catalog_asset_quarantined` and
  `quarantined_asset_installed`.
- The existing Godot files remain installed only as inspectable evidence; they are no longer a
  valid game-ready catalog asset.

## Regression evidence

- `cargo test --workspace --all-features`: passed.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: passed.
- `scripts/test-cli-product.sh`: passed.
- `scripts/test-stage3-static.sh`: passed.
- Character fixture contracts cover correct directions, rejection before video, video-introduced
  detached effects, Pack provenance, and Godot usage provenance.
- `character-semantic-quality-report@1.0.0` is available through `forge schema` in consistency-v2
  builds and validates the machine report.

Machine summary:
[`summary.json`](artifacts/forge-character-direction-equipment-semantics-20260807/summary.json)

Raw semantic report:
`generated-assets/forge-core-real-revalidation-20260807/jobs/a59c2efe-a639-472c-9204-7d5961d2f0aa/character-semantic-quality-report.json`

## Required next real acceptance

Only `walk_up` should be regenerated. The expected path is one rear-facing direction-still image
request plus one image-to-video request; the maximum is two attempts of each (four media requests).
After separate user authorization, Forge should run zero-cost semantic/loop/consistency replay,
export a new Pack only if all four animations are `game_ready`, reinstall Godot, and require a clean
project audit. No other Character, Icon, Prop, or world asset should be generated.
