# Cape hem and V10.1 offline acceptance — 2026-08-16

Result: implementation accepted for offline use; final Godot asset blocked.
No new image/video Provider request was made.

## Cape evidence

The four approved direction PNGs were copied outside the immutable source Job
and imported through the named four-file route with
`continuous_gold_lower_hem`.

- Job: `76172efc-4dd6-49cf-badc-0d9ddfea80c6`
- Provider requests: 0
- Result: `awaiting_review` / `direction_grid_regeneration_required`
- Exact failure: `front_idle:cape_hem_gold_trim_missing`
- back/right/left: game-ready for the hem contract
- no approval, action generation, Pack, catalog or Godot mutation

The measured adjacent-gold lower-hem ratios were front `0.003426`, rear
`0.034340`, right `0.014982`, and left `0.008821`, against a locked minimum
`0.004`. This reproduces the user's native observation without weakening any
identity or Alpha gate.

## V10.1 retained-video replay

The immutable failed V10 Job
`f80b6e47-9f58-4d8d-9dc3-4bdd239a9e6c` was replayed locally:

- final child Job: `ff618c4c-0332-4347-b82d-bbe387e95b06`
- workflow: `topdown-cycle@10.1.0`
- retained video SHA-256: `1d1e5d3741af8b78da55f152102aa0ee466c35056fd994d68b87ef59885c30b4`
- requests/images/videos/edits/uploads: all 0
- translation only: yes; maximum 6 px; clipped foreground 0
- residual center drift: 1 px (previously 9 px)
- residual foot drift: 0 px (previously 10 px)
- V10.1 scale lock: game-ready
- WorkflowGraph: one `anchor_stabilization` node with implementation
  `character-anchor-stabilization@1.0.0`, zero Provider request, and bound
  normalized-input / stabilized-output / report hashes

The replay correctly remained failed because the retained model pixels still
contain `stable_upper_body_flicker`, `upper_body_contour_drift`,
`unsupported_core_edge_drift`, and `temporal_edge_color_flicker`. This proves
that anchor stabilization fixes placement but cannot repair temporal costume or
upper-body redraw. No Pack/Godot asset was emitted.

## Offline checks

- `character_cycle` unit tests: 5/5 pass, including translation invariants and
  clipping/extreme-translation failure.
- cape hem unit tests: 2/2 pass.
- schemas parse as JSON and are exposed by the feature CLI.
- the exact V10.1 Plan reports 0 expected / 0 maximum Provider requests.
- Grid generation integration contracts: 34/34 pass.
- `cargo test --workspace --all-features`: pass (one explicitly ignored retained
  real-video replay remains ignored by design).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: pass.
- `cargo fmt --all -- --check` and `git diff --check`: pass.

## Next authorized work

1. Generate one corrected `front_idle` with the same narrow gold lower cape hem.
2. Re-import all four directions with the hem contract and approve only if all
   four automatic and native checks pass.
3. Do not reuse the rejected V10 video for production. Probe walks one direction
   at a time from the corrected anchors; stop at the first flicker/scale failure.
4. Export and open Godot only after the complete eight-animation set passes.
