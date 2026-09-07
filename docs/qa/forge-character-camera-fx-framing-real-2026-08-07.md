# Forge Character Camera, Framing, FX, and Multi-Source Real Acceptance

> Superseded for the final Character Pack by
> `docs/qa/forge-character-silhouette-temporal-v2-real-2026-08-07.md`. This report
> remains the historical camera/FX/framing evidence.

Date: 2026-08-07

Result: passed

## Inputs and accounting

- Source lineage: `323a1c5a-8af3-467a-95de-b093d5004edd`
- Real direction Jobs:
  - `idle`: `23e3de1d-9b3d-4563-9e0f-afcd332847f6`
  - `walk_up`: `851999e7-e7ab-46f1-9e16-3fd98bde3ca1`
  - `walk_right`: `9a984573-9b15-4b88-a5e1-a47b4b02d946`
  - `walk_down`: `cbbc4d09-0fed-41af-99bd-caa763bb637b`
- Final assembly Job: `66b7596e-5e17-4bff-b76c-40ee1aa551f2`
- Provider request ledger before and after assembly: 11 entries.
- Assembly Provider usage: 0 image, video, edit, upload, and total requests.

## Gate results

| Animation | Direction | Body top | Body height | Foot Y | Loop | Verdict |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| `idle` | down | 8.6% | 85.0% | 239.5 | 0.908 | `game_ready` |
| `walk_up` | up/rear | 7.8% | 85.9% | 240.0 | 0.961 | `game_ready` |
| `walk_right` | right | 11.3% | 82.2% | 240.0 | 0.919 | `game_ready` |
| `walk_down` | down | 6.6% | 87.1% | 240.0 | 0.961 | `game_ready` |

- Camera: `topdown-3q-orthographic@1.0.0`
- Framing: `body-framing@1.0.0`
- Direction: `direction-quality@1.1.0`
- Equipment/FX: `equipment-effect-consistency@1.1.0`
- Canvas: 256×256, shared scale `0.54028434`, effective alpha threshold 48.
- Detached effect frames: 0/32.
- Attached halo frames after deterministic cleanup: 0/32.
- The staff crystal remains opaque; the visible low-alpha halo is absent from the exported frames.

## Pack and Godot

- Pack:
  `generated-assets/forge-core-real-revalidation-20260807/jobs/66b7596e-5e17-4bff-b76c-40ee1aa551f2/exports/validation-ranger/Validation-Ranger.gsfpack`
- Pack SHA-256: `9a956da27c81db608e3d2a33a3c7d8a6e9357ee025631cb01af2e780dc9b86fb`
- Pack schema validation: passed.
- Godot install Job: `a37d9569-9d80-4243-90fc-2df2cd290d12`
- Godot version: `4.6.3.stable.official.7d41c59c4`
- Headless editor import: passed with empty stderr.
- Headless showcase scene load: passed with empty stderr.
- `forge_sprite_frames.tres`: 6,717 bytes.
- `forge_animated_sprite.tscn`: 6,996 bytes.
- No `PackedByteArray`, embedded Image, or `ImageTexture.create_from_image` in the installed asset.
- Source catalog ↔ Godot install link: passed.
- `project-audit@1.1.0`: clean, 4 Packs audited, 3 installed assets, 0 errors, 0 warnings.

## Provenance and security

`character-assembly-manifest.json` records every source Job and still/video SHA-256. Pack source
metadata and Godot `forge_usage.json` preserve the Camera, Framing, direction, FX, loop, and retry
profiles. Credential, authorization-header, device-code, token, and temporary-media-URL scans are
release gates; none of these values are required by or copied into the local assembly.

Machine summary:
`docs/qa/artifacts/forge-character-camera-fx-framing-20260807/summary.json`
