# Forge Character Camera, Framing, FX, and Assembly Plan

Date: 2026-08-07

Status: implemented and accepted with existing real xAI media

## Decision

The stable Character workflow locks one gameplay view:

- `topdown-3q-orthographic@1.0.0`: fixed three-quarter orthographic camera, approximately
  45-degree pitch, no perspective, zoom, yaw, or roll drift.
- `body-framing@1.0.0`: the character body, rather than a held staff or low-alpha chroma noise,
  defines center, scale, top margin, and foot anchor.
- `equipment-effect-consistency@1.1.0`: both detached effects and translucent halos attached to
  bright equipment are forbidden in ordinary locomotion unless baked emission is explicit.
- `character-multi-source-assembly@1.0.0`: four direction Jobs in one immutable lineage may be
  assembled locally into one Pack without another Provider request.

This does not reinterpret `walk_up` as an oblique front view. `idle` and `walk_down` are front
views, `walk_up` is the rear view, and `walk_right` is the right profile. Godot derives left by
horizontal flip.

## Pipeline

```text
four SHA-256 verified sibling Jobs
→ multi-source assembly manifest
→ candidate extraction and loop@2.0.0
→ chroma cleanup with an effective alpha floor
→ shared 256×256 body/foot-anchor normalization
→ Camera/Framing/Direction/FX hard gates
→ Character Pack
→ external-texture Godot installation
```

The normalizer measures a dense body footprint that rejects narrow held equipment, but it also
fits the complete robust foreground inside the canvas. It applies one shared scale to all 32
frames, aligns the body center and foot anchor, clears low-alpha chroma residue, and records every
transform in `character-framing-report.json`.

The halo detector builds a bounded mask around opaque high-luminance equipment cores. Only
translucent pixels outside the two-pixel antialiasing edge are candidates for deterministic
cleanup. The final semantic gate runs after cleanup; opaque equipment, body pixels, and requested
permanent emission are not silently deleted.

## CLI contract

```bash
forge job assemble-character \
  --base <job-id> \
  --source idle=<job-id> \
  --source walk_up=<job-id> \
  --source walk_right=<job-id> \
  --source walk_down=<job-id> \
  --wait --json
```

The command requires exactly four directions. Every source must share the base lineage, asset,
Provider/profile, models, prompt, and reference lock. Source paths must remain inside their Job,
and recorded SHA-256 values must still match. The child Provider manifest reports zero usage and
records `multi_source_assembly:<sourceJobId>` for every direction.

## Export gates

- Body top margin must be at most 18% and may drift at most 8 px from idle.
- Body height may differ from idle by at most 7%.
- Body center may drift at most 6 px; foot anchor may drift at most 3 px.
- Any unrequested detached emissive component or attached translucent halo blocks export.
- A `game_ready` loop selection replaces the stale first/last-frame loop verdict; it does not
  override anchor, alpha, canvas, media-integrity, or semantic failures.
- Review cannot override Camera, direction, framing, clipping, or FX hard failures.

## Acceptance evidence

The existing real xAI `idle`, `walk_up`, `walk_right`, and `walk_down` siblings were assembled
without changing the authorization ledger. The final Pack contains all four new videos, all four
animations are `game_ready`, Godot 4.6.3 imports and loads the showcase scene, and the installed
resources contain no embedded Image or `PackedByteArray` data.

See `docs/qa/forge-character-camera-fx-framing-real-2026-08-07.md`.
