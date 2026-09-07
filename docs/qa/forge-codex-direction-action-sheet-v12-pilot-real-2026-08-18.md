# Codex direction action sheet V12 pilot — real generation

Date: 2026-08-18

Verdict: **the one-direction-per-2x2 method is materially better than four
independent image requests, but the best candidate is still prototype-only and
must not be promoted to a production Pack.** No `.gsfpack` was emitted and no
production Godot project or approved Direction Grid Lock was modified.

## Scope and authority

- Generation surface: Codex subscription built-in image generation; no Image
  API, xAI request, or hidden Forge media Provider.
- Primary direction authority: the existing no-gold-trim `idle_right` frame
  already reviewed in the V11 Godot prototype.
- Supporting identity authority: the existing V11 front authority.
- Sheet shape: exactly one `2x2` `walk_right` action grid, read top-left,
  top-right, bottom-left, bottom-right.
- Delivery path: deterministic magenta removal, `512x512` frame extraction,
  shared scale, feet alignment, fringe decontamination, V11 external-keyframe
  intake, and Godot 4.6.3 `AnimatedSprite2D` playback.
- Forge Plan estimates and maxima were `0/0` Provider requests for both executed
  Jobs.

Experiment root:

`generated-assets/experiments/codex-direction-action-sheet-v12-pilot-20260818/`

Bound hashes:

- right direction anchor: `98d7d948640616be9e79630b37b14a0fff6f2d0bab443a1686ac679d19302cae`;
- front identity authority: `ec2ae86d11730be2b2c867c0bdffa8c6b48f788404890f56e7013a48ff26747c`;
- attempt 1 raw sheet: `57e22034995a260e767e83b9c5c4e3c5ab7fdec6233abd1adca9f284575a1477`;
- rejected attempt 2 raw sheet: `56d63fadf351a0fe1084652a1148a80e9fde9d209cdf73721162b674734d15a7`;
- pose-conditioned attempt 3 raw sheet: `3df4c4388467e9acd34d4ebe4dda7bd64ac81bb4da41b8e3e99ba5a61962b2e1`.

## Attempts

### Attempt 1 — direction anchor + text phases + layout guide

One built-in generation created four screen-right poses on solid `#FF00FF`.
All cells passed extraction containment, single-component and edge-touch checks.
Deterministic fringe cleanup changed 5,356 RGB pixels, changed zero Alpha
pixels, left zero connected magenta-fringe pixels and passed.

Forge Job: `dc040e1b-af53-45a5-9cf5-7d0ae593a64c`.

This is the best candidate. Compared with the independently generated V11
`walk_right`:

| Metric | V11 independent frames | 2x2 attempt 1 | Change |
| --- | ---: | ---: | ---: |
| phase-order score | 0.40657 | 0.60522 | passed the 0.45 minimum |
| stable upper-body flicker | 0.41106 | 0.08045 | 80.4% reduction; 0.00045 above budget |
| body-center step | 10.5 px | 4.0 px | 61.9% reduction |
| body-width variation | 0.20202 | 0.04348 | 78.5% reduction |
| foot-anchor step | 0.0 px | 0.0 px | stable |

It still failed `walk_right` production gates for:

- maximum foot-lobe count 3, budget 2;
- upper-body flicker just above the `0.08` budget;
- upper-body contour, unsupported core-edge and temporal edge-color drift;
- body-center step of 4 px;
- the existing orange-costume false-positive
  `unexpected_attached_emissive_halo`.

Forge recommended retrying frames 0 and 3. The Job remained failed and Pack
export stayed blocked.

### Attempt 2 — edit attempt 1 in place

One narrowly scoped built-in edit asked only for compact passing poses and a
locked upper body. It turned the bottom-left cell to screen-left and changed
cape/body presentation. The raw result was rejected before splitting or Forge
execution. This confirms that editing a failed sheet is not a safe gait-repair
route.

### Attempt 3 — fresh sheet with accepted V11 pose structure

A fresh generation used the right-facing anchor for identity and a deterministic
2x2 V11 phase contact sheet plus grayscale silhouette sheet for pose order.
All four outputs faced screen-right and the passing poses were visibly compact.

Forge Job: `3571da5c-fc18-457f-9169-9ac2b7deac9e`.

The stronger pose conditioning increased action amplitude but also reintroduced
large redraw drift:

| Metric | 2x2 attempt 1 | pose-locked attempt 3 |
| --- | ---: | ---: |
| phase-order score | 0.60522 | 0.42919 |
| stable upper-body flicker | 0.08045 | 0.25962 |
| body-center step | 4.0 px | 9.5 px |
| body-width variation | 0.04348 | 0.22727 |

Attempt 3 was rejected and no further generation was made.

## Godot 4.6 result for the best candidate

Attempt 1 was imported as four external `512x512` RGBA textures into native
Godot 4.6.3 `AnimatedSprite2D` + `SpriteFrames`, looped at 4 FPS with linear
filtering and `pixelSnap=false`.

- rendered movie frames: 68 at 30 FPS;
- duration: 2.2667 seconds;
- foot baseline drift: 0.0 px;
- horizontal bbox-center drift: 0.5 px;
- subject-height drift: 2.0 px;
- runtime geometry gate: passed.

The headless dummy renderer crashed only when MovieWriter attempted to access a
dummy texture. After normal Godot import, the macOS OpenGL Compatibility run
completed and wrote the valid movie and runtime capture. This is a QA-runner
display-mode issue, not an asset import failure.

## Decision

Adopt `one direction -> one 2x2 action sheet -> deterministic split` as the
preferred next-generation experiment topology. Do not yet make it a production
V12 workflow and do not expand it to `walk_up` or `walk_down` from this pilot.

The first-sheet result proves that a shared canvas substantially reduces
cross-request identity and center drift. It does not solve pixel-level upper-body
locking or reliable anatomical phase control. The next production design must
separate those concerns: a locked reusable upper-body layer or rig plus a
lower-body gait representation, rather than asking a raster generator to redraw
the entire character in every cell. Until that exists, keep attempt 1 only as a
Godot-usable prototype candidate and preserve the current V11 hard gates.

## Artifacts

- best raw sheet: `raw/walk-right-2x2.png`;
- rejected edit: `raw/walk-right-2x2-attempt2-rejected.png`;
- rejected pose-conditioned sheet:
  `raw/walk-right-2x2-attempt3-pose-locked.png`;
- best processed frames: `frames-clean/walk_right/`;
- best Forge reports: `qa/forge-job-dc040e1b/`;
- pose-conditioned Forge reports: `qa/forge-job-3571da5c/`;
- Godot project and runtime output: `godot/walk-right-qa/`.
