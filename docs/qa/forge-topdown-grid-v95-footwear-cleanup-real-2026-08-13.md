# Forge V9.5 footwear cleanup real execution and deterministic acceptance

Date: 2026-08-13  
Verdict: **accepted through deterministic local cleanup after one rejected real-model edit**

## Scope and source

The user rejected V9.4 Job
`c96788ba-e9de-43d6-8f30-c11611282f42` because `walk_down` frame 2 contained
a gray-white matte/guide residue beside the viewer-right boot. Forge recorded
that rejection and implemented `topdown-grid@9.5.0` as a single-frame cleanup
child. The source direction lock, identity, three unaffected frames, video,
Pack, catalog and Godot state remained outside the mutation scope.

- source workflow/target: `topdown-grid@9.4.0` / `walk_down:frame:2`
- real provider/profile/model: `xai` / `default` /
  `grok-imagine-image-quality`
- expected/maximum real requests: `1 / 1`
- maximum Provider operations: `1`
- maximum/reserved cost: `1,400,000,000` ticks
- videos, Pack, catalog, Godot: `0`

Forge did not silently switch to a newer model. The one-frame recovery stayed
bound to the model reviewed in the Plan and durable authorization.

## Transport recovery

The first V9.5 execution encountered an x.ai TLS transport failure before a
local generated image was materialized. Its Job
`6593deca-9a23-4be1-bb7c-c1b12f2756c7` recorded zero local Provider usage; the
corresponding durable request remained conservatively `ambiguous` and was
never reused. A second local-only preflight Job
`5f1f953a-6def-4e20-9997-e0cf23fc2281` also failed before any request because
the initial recovery implementation read the zero-output parent as a material
source. Both immutable failures were retained.

The recovery closure was then tightened to recursively bind zero-output
failure parents while sourcing direction locks and frames only from the
validated V9.4 material Job. Plan, execution, child-scope and authorization
lineage remained bound to the newest failure node.

## Real-model execution

Job `1ce7ade1-9a8e-40a2-8d11-cdf745f9ba50` settled exactly one
`edit_image` operation:

- requests / generated images: `1 / 1`
- observed cost: `700,000,000` ticks
- generated/edited video and private uploads: `0`
- frames 0, 1 and 3: byte-reused
- frame 2 generation method: `footwear_cleanup_edit`

The result was rejected automatically and visually. It retained a 50-pixel
neutral-gray connected component and changed frame 2 back to screen-left
contact. The Job ended with `grid_keyframe_action_quality_failed`; its
laterality and footwear reports were both `blocked`. The settled authorization
was not reused and no automatic Provider retry occurred.

## Deterministic local cleanup

Because the V9.4 frame already had the correct screen-right contact pose,
Forge added a zero-Provider `job cleanup-footwear` path. It accepts only the
hash-closed, manually rejected V9.4 frame-2 source that uniquely reproduces
the neutral-gray footwear leak. It creates an immutable child and modifies
only the assessed component plus one directly adjacent antialiasing ring; it
does not recursively enter boot shadows and does not synthesize or repaint the
pose.

Accepted Job: `4175b3ea-7181-47c0-a910-f8c567f8f042`

- Provider requests/cost: `0 / 0`
- source frame 2 SHA-256:
  `c709dfc8a2de65dfa365c68574321d07b4a7d3863e8c22adb3a79bea7e24e2dc`
- output frame 2 SHA-256:
  `e64cf819c8ac3b7ee6ab78578cbff4fce199a60dec1bfba48385a4abb4cb4c40`
- modified pixels: `87`
- repair bounds: `[133, 208, 141, 222]`
- all non-mask pixels: byte-identical
- frames 0/1/3: source SHA-256 values preserved exactly
- motion / laterality / equipment / footwear: all `game_ready`
- Pack, export, catalog, Godot: none

The acceptance path re-read all four PNGs, recomputed every bound artifact
hash, re-ran the four quality gates, and rechecked the zero-request usage
report before writing `review-decision.json`.

## Native review

Accepted at the native 2×2 sheet and 8× nearest-neighbor frame view. The
gray-white residue is gone; the brown viewer-right boot silhouette remains
natural and planted; the screen-left boot remains raised. Identity, auburn
fringe, green hood, full cape, amber scarf, leather clothing, scale and empty
hands match the three byte-reused frames.

This accepts the validation frames only. It does not authorize Pack export,
other directions, video generation, catalog changes or Godot installation.

## Isolated Godot follow-up

The accepted four-frame `walk_down` sequence subsequently passed an isolated
Godot 4.6.3 `AnimatedSprite2D` smoke at 4 FPS. Godot imported and retained all
four frames as ordered external PNG resources, played `0→1→2→3→0`, preserved
transparent borders and measured `0 px` baseline drift. This was not a Pack
export or a formal-project install. See
[`forge-topdown-grid-v95-godot-smoke-2026-08-13.md`](forge-topdown-grid-v95-godot-smoke-2026-08-13.md).

## Evidence

- [accepted contact sheet](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042/grid-keyframe-validation/walk_down-contact-sheet.png)
- [accepted frame 2](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042/grid-keyframes/walk_down/frame-02.png)
- [local cleanup manifest](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042/source/local-footwear-cleanup/local-cleanup-manifest.json)
- [repair report](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042/source/local-footwear-cleanup/footwear-neutral-gray-repair.json)
- [laterality report](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042/source/local-footwear-cleanup/walk_down-laterality.json)
- [footwear report](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042/source/local-footwear-cleanup/walk_down-footwear.json)
- [review decision](../../generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042/review-decision.json)
