# Forge Vidu Q2 fixed-scale canary — real run

Date: 2026-08-19

Status: one bounded browser-provider request completed; source rejected before
Godot because both the source-height and production-sampling gates failed.

## Authorization and cost

- user confirmed the final Vidu submission action;
- provider/model: Vidu web / Vidu Q2;
- task: `3429047626647115`;
- credits: `25 → 15` (10 spent, no reward expected or received);
- request count: 1 of the authorized maximum 1;
- automatic retry: not authorized and not performed;
- cash purchase: not authorized and not performed.

## Locked inputs and source

- identity anchor SHA-256:
  `98d7d948640616be9e79630b37b14a0fff6f2d0bab443a1686ac679d19302cae`;
- prompt SHA-256:
  `12af233da258cc99aa1eaf9ce610d7a1b65fbe2098d703a5dfd0c1d181c54ca1`;
- source video SHA-256:
  `27e0a0171f000cf2593f14f14621f36e3367aeaa90181fcb7e260061767b717f`;
- source: 1440×1440, 24fps, 122 native frames, 5.083333 seconds.

The revised prompt made head top, belt/pelvis center, support-foot ground line,
and ≤1% body-height change explicit. It also prohibited scale, zoom, vertical
root movement, perspective change, and reframing.

## Gate results

Gait selection found a complete game-ready walk window (native frames 38–94),
but the source still failed the two gates required before Godot:

1. Translation-only height gate:
   - observed relative drift: 2.2242%;
   - maximum allowed: 1.5%;
   - selected 24-frame body-height range: 48 source pixels;
   - previous candidate relative drift: 2.9570%;
   - verdict: blocked.
2. Production sampling:
   - 12-frame normalized reconstruction error: 15.1677%;
   - maximum allowed: 12%;
   - verdict: blocked.

The prompt improved the relative scale deviation but did not make Vidu Q2 obey
the fixed-raster-height contract. Translation cannot repair this while keeping
both the head and support foot fixed.

## Decision

No Godot project and no `.gsfpack` were written for this candidate. The source
is retained as immutable evidence, and the run stops without a blind retry.
Further work should change the motion-source method rather than spend another
Q2 call on prompt-only variation.
