# Forge external DirectionGrid import — real local acceptance — 2026-08-15

Result: implementation accepted; imported visual candidate awaiting user
review. Real Provider requests: 0.

## Implementation result

The new command is:

```bash
forge job import-direction-grid \
  --id <approved-v9-direction-grid-job> \
  --sheet /absolute/external-sheet.png \
  --generator codex_builtin_image_gen \
  --note "external candidate; native review required" \
  --wait --json
```

Plan estimate and maximum are both zero. The operation is represented by a
distinct immutable recipe, clears inherited authorization, hashes the external
sheet and complete source closure, and never resolves a media Provider.

## Real local candidate

- approved source Job: `9ea9cae9-dd94-4b4a-a277-4b5d0345b2ad`;
- imported child Job: `01447a14-791d-4ced-8f5f-990076e6b200`;
- input SHA-256:
  `2f75b4dd007f784502b86614601ed4a36abe332367e699000a83b95b6c0b35df`;
- lifecycle: `awaiting_review`;
- authorization: none;
- Provider usage: 0 requests, 0 generated images, 0 videos and 0 uploads;
- Pack/Godot artifacts: 0.

Checkerboard matting removed 1,278,326 background pixels and 1,551 exposed
neutral fringe pixels. Cleaned alpha coverage is `0.18609604`, border opaque
ratio is `0`, significant subject count is `4`, neutral edge residual is `0`,
and the matting verdict is `game_ready`.

Shared alignment reports maximum source body-scale drift `0.005405426`, center
drift `0.5 px` and foot-baseline drift `0.3999939 px`; all are below their hard
limits. Four directions, empty hands, neutral stance, hood, scarf, armor and
full cape are visibly present. No baked checkerboard, gray platform or obvious
edge halo remains.

The existing appearance report remains `awaiting_review` with
`direction_grid_undeclared_object_evidence`, driven by front/side hand and
exterior elongation heuristics. Native review does not reveal a weapon or prop,
but the regenerated face, front chest strap and some clothing details drift
from the approved V9 identity. The Job was deliberately not approved.

The approved source tree stayed byte-identical at
`785da20140de9aff17be792f870dd010ec564f289e8baa60eef9eb7fe5caddbc`.

## Verification

```text
cargo test -p core checkerboard
  passed
cargo test -p core saturated_border_residual_fails_closed
  passed
cargo test -p providers --features grid-generation --test grid_generation_contract \
  external_direction_grid_import_mattes_checkerboard_with_zero_provider_requests -- --exact
  passed
cargo clippy -p core -p providers -p forge-cli --all-targets \
  --features grid-generation -- -D warnings
  passed
scripts/test-grid-generation.sh
  passed; Grid contract 33/33
cargo test --workspace --all-features -q
  passed
cargo clippy --workspace --all-targets --all-features -- -D warnings
  passed
scripts/test-cli-product.sh
  PASS Forge CLI product contract
```

The integration contract also rejects sheet mutation, source Lock mutation,
missing direction subjects and colored border residue before any Provider
request. An isolated review-contract run accepted an intact imported child and
wrote a DirectionGridApproval, while a changed matting report was rejected with
`direction_grid_import_provenance_invalid`; the real candidate remained
unapproved.
