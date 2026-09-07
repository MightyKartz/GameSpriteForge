# Forge Identity Metric Calibration

Date: 2026-08-11

Status: P0-B implemented and offline gates passed. No real Provider requests
were made. `identity-metric-v2` is a core feature and is disabled by default.

## Calibration set

Manifest:

`docs/qa/artifacts/forge-identity-metric-calibration-2026-08-11/calibration.json`

The set contains 10 real V8 DirectionMotion image pairs copied from retained
Job artifacts:

- 4 human-accepted pass pairs.
- 5 human-rejected fail pairs.
- 1 gray pair that remains unreviewed and is excluded from pass/fail scoring.

Labels recover the human decisions already recorded in the V8 QA reports.
No labels were inferred from model output and no thresholds were lowered.

## Baseline pHash result

Current 64-bit pHash similarity was measured first:

- Pass mean: `0.8984`
- Fail mean: `0.9344`
- Separation: `-0.0359`
- ROC AUC: `0.1750`
- Best threshold confusion: TP 4 / FP 5 / TN 0 / FN 0

This confirms the documented failure mode: human-rejected identity drift can
score more “similar” than accepted direction changes.

## Ablation

Machine-readable report:

`docs/qa/artifacts/forge-identity-metric-calibration-2026-08-11/identity-metric-report.json`

| Signal | Separation | ROC AUC | Result |
| --- | ---: | ---: | --- |
| HOG | `-0.0084` | `0.2000` | inverted on this set |
| Regional palette | `+0.0656` | `1.0000` | useful |
| Silhouette IoU | `-0.1623` | `0.1000` | inverted by legitimate direction changes |
| Occupancy | `-0.0219` | `0.4250` | weak/inverted |
| pHash | `-0.0359` | `0.1750` | baseline failure |

## IdentityMetricV2

Weights are derived from this ablation and remain versioned in code and report:

- Regional palette EMD-style support: `0.80`
- HOG: `0.05`
- pHash: `0.05`
- Occupancy: `0.10`
- Silhouette IoU: `0.00`

Silhouette remains in the report for diagnosis but receives zero weight on this
calibration because legitimate front-to-side direction changes score lower
than human-rejected front-to-back structure loss.

Composite result:

- Pass mean: `0.9797`
- Fail mean: `0.9316`
- Separation: `+0.0481`
- ROC AUC: `1.0000`
- Best threshold confusion: TP 4 / FP 0 / TN 5 / FN 0

The composite is significantly better than the pHash baseline on this set.
This is not a claim that the metric is universally calibrated; the set is
small and intentionally preserves one gray pair. DINO/LPIPS remain optional,
unpublished component candidates and were not installed or used.

## Verification

`scripts/test-identity-metric.sh` completed successfully:

- `cargo test -p core quality::identity`: 2/2 passed.
- `cargo check -p core`: passed.
- `cargo check -p core --features identity-metric-v2`: passed.
- Calibration evaluator regenerated the report and enforced:
  - 4 pass / 5 fail / 1 gray;
  - composite separation greater than pHash;
  - composite ROC AUC greater than pHash and at least `0.99`;
  - regional palette ablation ROC AUC at least `0.99`.

`assess_consistency` behavior is unchanged in the default build. The new
metric is not invoked by existing V1–V8 workflows unless a future feature-gated
workflow explicitly consumes it.
