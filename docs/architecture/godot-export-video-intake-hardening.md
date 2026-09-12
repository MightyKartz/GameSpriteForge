# Godot Delivery and Animation Intake

Updated: 2026-09-12. This note retains the engine and media correctness decisions
from the 2026-06-11 desktop work and describes their current CLI entry points.

## Separate preparation from installation

Forge prepares local frames into a validated `.gsfpack`; Godot installation uses
its own fingerprinted, single-use plan. Use the
[CLI protocol](../automation/forge-cli.md#godot-46x-delivery) and
[local asset guide](../automation/codex-local-assets.md) for the maintained
request shapes and end-to-end commands.

```text
source input -> frames -> processing -> quality report -> Pack validation
validated Pack -> Godot install plan -> native resources -> engine verification
```

The CLI installs external PNG textures, runs headless Godot import, and creates
native resources with `ResourceLoader`. Animations receive atlas textures,
`SpriteFrames`, and an `AnimatedSprite2D` scene. Icons receive texture mappings;
props receive `Sprite2D` scenes. Installation records `forge_usage.json` and
`.forge/assets.json`, replaces only Forge-owned targets, and rolls back on
failure.

## Frame selection and position

A target frame count is an extraction parameter, not evidence of a correct
animation. Review the selected motion segment, extracted count, native frame
timing, first/last-frame transition, and requested loop range. Animation remains
experimental even when structural checks pass.

Position review should include foreground bounds, bottom and center drift,
anchor coordinates, and cell boundaries. Requests can preserve intentional
source coordinates and timing. Repair must keep the selected coordinate contract;
review actual playback after installation.

## Engine verification

Schema validation does not prove Godot compatibility. The engine gate should
load actual installed textures and resources, save and reload the scene, and
exercise animation playback. Static delivery additionally verifies rendering
filters and origin placement.

Current checks include:

```bash
bash scripts/run-godot-pack-smoke.sh
cargo test -p core --test animation_delivery_tests --test static_delivery_tests
```

Use the Godot-dependent checks documented in
[CONTRIBUTING](../../CONTRIBUTING.md) and the
[development skill](../../.agents/skills/forge-dev/SKILL.md) for full delivery
verification. New transient projects and Job stores belong under `target/qa/`
or an isolated temporary directory.

The [2026-06-11 pack smoke](../qa/godot-pack-smoke-2026-06-11.md) and
[implementation evidence](../qa/godot-export-video-intake-hardening-2026-06-11.md)
remain historical engine evidence. Their desktop UI results do not describe the
current product surface.
