# Forge Character Four-Direction Real xAI Acceptance

> Superseded for the final Character Pack by
> `docs/qa/forge-character-silhouette-temporal-v2-real-2026-08-07.md`. This report
> remains the historical real-generation evidence.

Date: 2026-08-07

Authorization: `character-four-directions-20260807`

Source Job: `1e927928-0ec5-45f1-abce-c0e44fbf417a`

## Result

### Final combined acceptance

The interim limitation described below has been resolved. Zero-cost
`character-multi-source-assembly@1.0.0` assembled all four sibling Jobs into final Job
`66b7596e-5e17-4bff-b76c-40ee1aa551f2`. Its Pack contains all four new real videos, passes
Camera/Framing/Direction/FX and loop gates, and is installed in Godot 4.6.3. The authorization
ledger remains at 11 entries.

Final evidence and Pack path are recorded in
`docs/qa/forge-character-camera-fx-framing-real-2026-08-07.md`.

### Original per-direction acceptance

All four directions produced new real xAI media. Each selected first video passes the local
direction, detached-effect, consistency, and loop gates:

| Animation | Job | Semantic | Loop score | Detached effects |
| --- | --- | --- | ---: | ---: |
| `idle` | `23e3de1d-9b3d-4563-9e0f-afcd332847f6` | `game_ready` | 0.908 | 0 / 8 |
| `walk_up` | `851999e7-e7ab-46f1-9e16-3fd98bde3ca1` | `game_ready` | 0.961 | 0 / 8 |
| `walk_right` | `9a984573-9b15-4b88-a5e1-a47b4b02d946` | `game_ready` | 0.919 | 0 / 8 |
| `walk_down` | `cbbc4d09-0fed-41af-99bd-caa763bb637b` | `game_ready` | 0.961 | 0 / 8 |

`walk_up` has zero visible-face frames and uses the paid rear-facing still. The final up Job
exports a schema-valid Pack, but that Pack reuses the source Pack's other three animations. The
three new sibling videos are therefore preserved as acceptance evidence and are not falsely
described or installed as one combined four-direction Pack.

At this interim checkpoint Godot installation was intentionally skipped until a zero-cost
multi-source assembly workflow could create one Pack containing all four newly generated siblings.
That workflow and installation have now passed, as documented above.

## Accounting

- Ledger entries: 11.
- Settled cost: 25,600,000,000 ticks (about USD 2.56).
- One initial `walk_up:still` transport failure remains conservatively `ambiguous`, reserving
  5,000,000,000 ticks.
- Conservative accounted total: 30,600,000,000 ticks, below the 55,000,000,000 cap.
- Two entries are zero-cost private video uploads. No request targeted another asset or
  `subject_reference`.

## Defects found and repaired during acceptance

- Reused Character retries no longer demand a `subject_reference` authorization.
- A new still-retry child receives its own 1–2 attempt window while the durable ledger retains
  the cross-Job cap.
- Targeted retries cannot auto-repair an unselected animation.
- Video authorization uses stable animation IDs rather than prompt text.
- `direction-quality@1.1.0` requires meaningful eye/mouth feature density and no longer mistakes
  rear-view scarf folds or hands for a face.
- A failed Job can reuse paid, hash-verified still media and continue video generation without
  another image request; missing animations are resolved only through the verified parent lineage.

## Security and Pack evidence

- Credential and token scan: passed.
- Successful up Pack validation: passed.
- Pack: `generated-assets/forge-core-real-revalidation-20260807/jobs/851999e7-e7ab-46f1-9e16-3fd98bde3ca1/exports/validation-ranger/Validation-Ranger.gsfpack`
- Machine summary: `docs/qa/artifacts/forge-character-four-directions-real-20260807/summary.json`
