# Forge V9.5 accepted `walk_down` Godot 4.6 smoke

Date: 2026-08-13  
Verdict: **pass — isolated runtime validation only**

## Scope

This smoke consumed the four accepted `walk_down` PNGs from Job
`4175b3ea-7181-47c0-a910-f8c567f8f042`. It did not call a Provider, export a
Pack, install into the formal Forge Godot project, mutate Catalog state, or
authorize other directions.

The source Job was already accepted by
[`forge-topdown-grid-v95-footwear-cleanup-real-2026-08-13.md`](forge-topdown-grid-v95-footwear-cleanup-real-2026-08-13.md).

## Godot execution

Godot `4.6.3.stable.official.7d41c59c4` imported all four external PNGs as
`CompressedTexture2D`, created a native `SpriteFrames` resource, created and
saved an `AnimatedSprite2D` scene, reloaded both resources, and played the
animation headlessly.

Runtime contract:

- animation: `walk_down`
- frame order: `0, 1, 2, 3`
- playback: `4 FPS`, looping
- observed runtime sequence: `0, 1, 2, 3, 0, 1`
- filtering: nearest-neighbor
- centered anchor and integer `2×` scale
- all four frames: `256×256`, transparent canvas border
- alpha-footprint baseline: `y=239` for every frame
- baseline drift: `0 px`

The saved `.tres` is 672 bytes and binds four external PNG resources. The
saved `.tscn` is 401 bytes and binds the external `.tres`. Neither resource
contains `Image`, `PackedByteArray`, `ImageTexture`, or embedded raster data.

## Visual review

The Godot-loaded 2×2 contact sheet and `0→1→2→3→0` playback strip were
reviewed at native pixels. The repaired frame 2 has a clean brown boot
silhouette with no gray-white matte/guide residue. Identity, hood, full cape,
scarf, clothing, empty hands and scale remain consistent. The four frames
alternate the intended contact/passing phases without vertical baseline
jitter.

## Integrity and boundaries

The accepted Job tree was hash-checked before and after the smoke and remained
unchanged:

```text
before: c14fe5cc50957ecdb71a8664ac5fe40e086a094bbc57eb0a13ae68de25821faf
after:  c14fe5cc50957ecdb71a8664ac5fe40e086a094bbc57eb0a13ae68de25821faf
```

The run used an isolated temporary project. Its portable evidence copy is
under `docs/qa/artifacts/forge-topdown-grid-v95-godot-smoke-20260813`; it is
not a production Godot install and is not a `.gsfpack`.

## Reproduction

```bash
scripts/run-v95-godot-walk-smoke.sh \
  /Users/kartz/Development/Forge/generated-assets/forge-topdown-grid-v9-real-20260811/jobs/4175b3ea-7181-47c0-a910-f8c567f8f042 \
  /Users/kartz/Development/Forge/docs/qa/artifacts/forge-topdown-grid-v95-godot-smoke-20260813
```

The runner unsets real Provider environment variables before launching Godot.
It rejects a non-accepted source, a source-frame SHA mismatch, missing external
texture bindings, embedded image payloads, a non-looping or misordered
animation, baseline drift above three pixels, or any source Job mutation.

## Evidence

- [runtime report](artifacts/forge-topdown-grid-v95-godot-smoke-20260813/output/godot-smoke-report.json)
- [Godot-loaded contact sheet](artifacts/forge-topdown-grid-v95-godot-smoke-20260813/output/walk_down-runtime-contact-sheet.png)
- [Godot-loaded playback strip](artifacts/forge-topdown-grid-v95-godot-smoke-20260813/output/walk_down-runtime-strip.png)
- [native SpriteFrames](artifacts/forge-topdown-grid-v95-godot-smoke-20260813/output/walk_down.spriteframes.tres)
- [native AnimatedSprite2D scene](artifacts/forge-topdown-grid-v95-godot-smoke-20260813/output/walk_down_smoke.tscn)
- [source integrity report](artifacts/forge-topdown-grid-v95-godot-smoke-20260813/source-integrity.json)
- [Godot runtime log](artifacts/forge-topdown-grid-v95-godot-smoke-20260813/godot-runtime.log)

## Decision

The accepted V9.5 `walk_down` frames are technically usable as a Godot 4.6
`AnimatedSprite2D` loop. This does not establish a complete four-direction
character Pack: `walk_up`, `walk_left`, `walk_right`, idle mappings, formal
Pack export and integration into the real game remain separate work.
