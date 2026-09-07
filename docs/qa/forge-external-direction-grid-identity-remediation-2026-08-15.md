# Forge external DirectionGrid identity remediation acceptance

Date: 2026-08-15  
Scope: Phase 1 implementation and Phase 2 zero-request replay only  
Result: implementation accepted; replayed candidate rejected as designed

## Boundary

This acceptance did not authorize or perform a Provider request, DirectionGrid
approval, Pack export, or Godot delivery. It used the approved source Job as an
immutable comparison authority and wrote a new local replay Job only.

- Approved source Job: `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`
- External sheet:
  `generated-assets/experiments/codex-imagegen-direction-grid-20260815/direction-grid-alpha-retry.png`
- Final replay Job: `a045e4fc-b457-4618-b5fe-2aaf3bc33ac0`
- Lifecycle: `awaiting_review`
- Stable error: `direction_grid_regeneration_required`
- Authorization: none
- Provider requests, generated images, generated videos, edited videos, and
  private uploads: all zero
- Approved source tree SHA-256 before and after:
  `785da20140de9aff17be792f870dd010ec564f289e8baa60eef9eb7fe5caddbc`
- Job-directory count: 22 before, 23 after
- Approval, `.gsfpack`, `.tres`, and `.tscn` artifacts: absent

## Implemented contracts

- `direction-grid-lock@1.2.0` separates the external pixel `producer` from the
  future `downstreamBinding`; imported pixels cannot be attributed to xAI.
- `direction-grid-import-consistency@1.0.0` binds each approved/candidate pair
  by ordered path and SHA and measures identity, regional detail, silhouette,
  exterior props, torso-line additions, scale, center, and foot baseline.
- `checkerboard-sheet-matting@1.1.0` reconstructs and decontaminates soft Alpha
  while truthfully recording deterministic post-processing as the transparency
  origin.
- `alpha-edge-halo@1.0.0` checks white, black, and magenta composites for
  exposed checker tiles, neutral halo, isolated dark fragments, and border
  opacity.
- `direction-grid-authority-alignment@1.0.0` uses one shared source-relative
  scale and per-direction translation. Named four-file input mode additionally
  supports exact authority-byte reuse.
- Native review is a fixed front/back/right/left four-row package with columns
  for approved source, candidate, magenta Alpha composite, and diff overlay.

## Replay evidence

Authority alignment passed:

| Direction | Width ratio | Height ratio | Center drift | Foot drift |
| --- | ---: | ---: | ---: | ---: |
| front | 0.988506 | 1.004808 | 0.5 px | 0 px |
| back | 0.987952 | 1.009662 | 0.5 px | 0 px |
| right | 0.967213 | 1.000000 | 0 px | 0 px |
| left | 0.983607 | 0.990476 | 0.5 px | 0 px |

Matting and final Alpha passed: 5,307 soft/decontaminated input-edge pixels,
6,420 final soft-edge pixels, zero opaque edge exposure, zero neutral halo,
one isolated dark discontinuity against a four-frame budget of 32, and zero
border opacity.

Relative consistency correctly rejected the sheet:

| Direction | Identity composite | Face-region change | Torso-region change | New torso lines | Verdict |
| --- | ---: | ---: | ---: | ---: | --- |
| front | 0.978408 | 0.151214 | 0.175219 | 3 | regenerate |
| back | 0.977314 | 0.066070 | 0.074091 | 3 | regenerate |
| right | 0.953545 | recorded | recorded | 0 | game_ready |
| left | 0.976784 | recorded | recorded | 0 | game_ready |

The hard reason for front and back is
`direction_grid_new_torso_linear_detail`. Native review agrees: scale and foot
baseline are now consistent and no checker halo or gray platform remains, but
the front/back candidate introduces identity-bearing clothing lines that do not
exist in the approved same-direction authorities. Thresholds were not loosened.

## Evidence hashes

- Native review package:
  `21098fe9c7161b737db5721ab66a6bd45b0b8f18f309bcef97f730a73bb5f9bc`
- DirectionGrid Lock:
  `d1f36ec3a93ebbe611c84786849e09fac1f4f98071c277b605c3fe640cc099b6`
- Relative consistency report:
  `88dc521e8f97dcabe07f7ee3a2f41e1cadbe6e4690e4d56f5d857112c1caad58`

## Release gates

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `scripts/test-grid-generation.sh`
- `scripts/test-cli-product.sh`
- JSON Schema parse and `git diff --check`

Phase 3 remains a separate future authorization: generate four independent
direction images from same-direction authorities and import them through the
named four-file route. No action generation or Godot delivery should start
until a new DirectionGrid passes this review contract and is explicitly
approved.
