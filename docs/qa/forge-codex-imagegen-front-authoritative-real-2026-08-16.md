# Codex front-authoritative DirectionGrid generation — 2026-08-16

Verdict: generated but rejected for production. No xAI content generation was
used after the user explicitly switched production to Codex built-in image
generation.

## Scope

- Authority: approved V9 `front_idle`, SHA-256
  `08e34f0e9ad5702e93a9112993da45920b778939534d95850653792aa93f15d0`.
- Initial generation: one Codex built-in 2×2 sheet, not four independent
  direction generations.
- Targeted corrections: one side-width correction and one final identity/
  costume correction. No further generation after the final hard failure.
- Codex output remained 1254×1254 RGB with baked checkerboard pixels. Forge
  performed deterministic checkerboard matting and Alpha reconstruction with
  zero media-Provider requests.

Workspace candidates are under
`generated-assets/experiments/codex-front-authority-direction-grid-v3-20260816/`.
The last raw sheet is `direction-grid-identity-corrected.png`; named v3 files
are the four ordered cells.

## Deterministic validation

Initial local import Job `e7541a10-80c3-407f-98fd-a8544784b1da` passed
checkerboard matting and Alpha/halo gates but rejected both side silhouettes as
about 11.5% narrower than the approved same-direction authority.

Width-corrected Job `b27c4a87-19c6-4407-bd9a-c2228b1ab775` passed geometry and
materialized transparent nodes. It was rejected for new torso linear detail,
side exterior objects and the explicit cape topology report.

Final identity-correction Job `149a96d2-df71-4c78-ada1-f7d0442d9560` also
materialized four transparent nodes and a review contact sheet. It remained
`direction_grid_regeneration_required` because all four directions contained
source-relative torso detail drift, rear/side nodes contained new exterior
silhouettes, and the front cape report measured 0.00998 against the 0.004
no-skirt maximum. Native review independently confirmed the face/costume drift
and overlong rear/side cape.

## Delivery boundary

The candidate was not approved and produced no Pack, catalog entry or Godot
asset. Forge provider usage for all three local imports was zero. The approved
source Job and Lock remain unchanged.

## 2026-08-17 minimal-edit optimization

After native review called the 2×2 result poor, the route was narrowed again:
the approved front remained immutable; approved back and right nodes were each
sent to Codex built-in image editing with the front as a supporting plain-hem
reference; left was derived by deterministic mirroring of right.

Codex successfully removed the visible gold cape edging, but still regenerated
the complete sprites. Back changed canvas to 1199×1312 RGBA and altered scale/
brush detail; right changed to 1254×1254 RGB with baked checkerboard and altered
body proportions. Local checkerboard cleanup then hit
`alpha_edge_dark_outline_discontinuity` in Jobs
`a66849f6-a601-4a34-9c53-c81b9a8e3f9c` and
`5121ef71-1b16-47a6-a0d3-58efb860a809`. Mixed-background assembly Job
`e3623126-856d-4e76-b5b0-f6c313583722` also failed the matting contract.

Raw optimization candidates are retained under
`generated-assets/experiments/codex-minimal-cape-edit-v4-20260817/`. They are
not approved production assets. No xAI request, Pack, catalog or Godot mutation
occurred in this optimization round.
