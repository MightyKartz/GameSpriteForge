# Forge Top-down Continuous Cycle V10 offline acceptance — 2026-08-13

Result: accepted offline. Real Provider requests were not executed by this
report. On 2026-08-14 a separate product contract enabled only one exact
`xai/default` Video 1.5 `walk_down` validation probe.

## Accepted behavior

- `topdown-cycle@10.0.0` reuses one approved V9 `DirectionGridLock`; source
  direction hashes remain unchanged.
- validation-only `walk_down` estimates and performs exactly 1/1 fixture video
  request, selects one complete 8/10/12-frame source cycle, writes
  `character-scale-lock@1.0.0`, and exports no partial Pack.
- the complete fixture path performs exactly 4/4 video requests and no still
  generation, then exports `idle_down`, `idle_up`, `idle_right`, `idle_left`,
  `walk_down`, `walk_up`, `walk_right`, and `walk_left`.
- one shared scale and per-animation translation anchor are used; static V9
  idles are canvas-matched once before the shared pass. Independent per-frame
  resizing is not used.
- Pack validation accepts explicit four-direction gait evidence and retains the
  portable V9 DirectionGridLock, its approval, the V10 DirectionLock, complete
  source-cycle timing, and the character scale lock.
- the generated Pack installs into Godot with external PNG resources and opens
  successfully in Godot headless editor mode.
- a modified approved V9 node fails during Plan preparation before any media
  request.
- non-fixture V10 requests fail closed except the exact independently
  authorized 480p × 4-second Video 1.5 `walk_down` validation route; its
  offline boundary test proves one video request, no Pack, and mandatory
  native review.

## Focused evidence

```text
cargo test -p core character_cycle --lib
  3 passed, including the 4px accepted / 5px rejected center-drift boundary

cargo test -p providers --features grid-generation --test grid_generation_contract \
  fixture_topdown_cycle_v10_reuses_approved_v9_and_selects_one_scaled_walk_cycle -- --exact
  1 passed

cargo test -p providers --features grid-generation --test grid_generation_contract \
  fixture_topdown_cycle_v10_full_pack_has_eight_animations_and_godot_contract -- --exact
  1 passed; Pack validation, Godot install and Godot headless load passed

cargo test -p providers --features grid-generation --test grid_generation_contract \
  fixture_topdown_cycle_v10_plan_rejects_tampered_v9_anchor_before_video_request -- --exact
  1 passed

cargo test -p providers --all-features --test grid_generation_contract \
  fixture_topdown_cycle_v10_real_boundary_is_exact_one_video_and_awaits_review -- --exact
  1 passed; exact xAI scope proof, one video target/request, zero-request
  unvalidated-Provider rejection, mandatory awaiting_review and no Pack

cargo test --workspace --all-features
  passed; Core 293/293, Grid 30/30, legacy V7/V8/V9, Pack and Godot contracts green

cargo clippy --workspace --all-targets --all-features -- -D warnings
  passed
```

## Explicit non-claims

This report does not claim that Grok Imagine Image 2.0, Grok Imagine Video 1.5,
or PixelLab animation is visually accepted for this workflow. Model choice must
be decided by a separately authorized one-direction real probe; fixture success
does not authorize network access or paid generation. The separately authorized
2026-08-14 Video 1.5 probe was subsequently rejected for anchor drift; see
`docs/qa/forge-topdown-cycle-v10-real-acceptance-2026-08-14.md`.
