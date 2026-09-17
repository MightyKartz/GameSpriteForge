# Real generated assets in Godot 4.7.2 — PR #50 runtime QA

**Date:** 2026-09-17
**Scope:** PR #50 (`codex/godot-47-selection`, commit `0d2a040e36c0ff9f7f49eaf9933abf9e01b88a8e`), default features, release profile.
**Question:** Can the PR's actual installed executable process real generated art and deliver it to the newest supported official Godot?

This is macOS acceptance evidence only. It is not a Windows pass, not a release signing pass, not license approval, and not final art approval.

## Toolchain and identity

- Checkout: clean at `0d2a040`; local `cargo build --locked --release -p forge-cli --no-default-features`.
- Installed executable: SHA-256 `32adac744a780bef7087464b878de7dc9360cff370a8f3d3364ede56bb0ef1ae`.
- `doctor --json`: CLI 0.5.0, clean build, default features, target `aarch64-apple-darwin`; includes `godot_version_selection`, `project_asset_output_registration`, local static/animation import, Pack validation, installation and evidence capabilities.
- Packaged payload was reconstructed from the published v0.5.0 layout/checksum evidence and installed through `install.sh` in isolated test mode with the PR binary substituted. `scripts/verify-cli-build.py` passed with `packagedPayloadChecked: true`. This checks layout and identity; it is not an upstream signed artifact claim.
- Godot: official `4.7.2.stable.official.ed1daf0bf`, SHA-256 `c7cccbf8fb143e34e02fd6521e09be2c2b974f0d5db080b19071c9c570718ccf`; selected through `forge setup godot --path`.
- Godot export template archive: official 4.7.2 `.tpz`, SHA-512 checked against the pinned manifest before extracting `templates/macos.zip`.

Evidence root: `/Users/kartz/Development/Forge-local-archive/pr50-real-assets-20260917/evidence/`.

## Real asset processing

Sources were created by Codex image generation during this run and copied with hashes into the QA workspace; no Forge Provider was used and no model identity was invented.

1. Thunderstone golem (`monster.thunderstone`)
   - Matched PNG color-keyed through Forge `source matte`.
   - Prepared with `plan prepare-static`, `prop_set`, 512px canvas, `game_ready`, and output registration.
   - Job `5ddcbf61-eca4-4167-9498-8f45902ead74` succeeded; Pack SHA-256 `2360e0990e9d6b9577c658cee5ce0e6aa0b60572dec4f51ed0ce2485b265bb57`.
   - The report records zero Provider requests and a distinct `visual_review_required` result.
2. Cyan lightning (`fx.lightning`)
   - The first real generated sheet was correctly stopped at `awaiting_review` (`NeedsCleanup`, `increase_canvas_margin`) because semi-transparent effect pixels touched the declared cells' edges. This was treated as the gate working, not bypassed.
   - A revised source was prepared with explicit `manual_color` matting and the effect profile. Job `d6b3903d-f4c4-47c2-a3ab-55d321efae4a` passed; revision `2c7cfef0…` was installed, then a local edge-cleanup revision (`threshold=110`, `softness=90`, `halo=1`) passed and was retained/selected as revision `cc9e6169…`.
   - The request preserved four shared 627×627 coordinates, one ground anchor and nonuniform durations `[90, 100, 140, 180]` ms. No quality gate was disabled.
3. Source catalog
   - Raw and derived sources were scanned and registered as local assets with origin notes. The output Packs also registered with execution provenance.
   - The golem and final lightning revisions were retained, selected and resource-locked separately before installation.

## Godot delivery and runtime

The throwaway Godot 4.7.2 project was machine-local under the QA root and carried a Forge toolchain lock. Installation used the library/resource-lock interface (`godot plan-install` + one-time `plan execute`) into `addons/forge_assets/monster` and `addons/forge_assets/lightning`.

The project scene instantiates the delivered `Sprite2D` monster and `AnimatedSprite2D` lightning twice. It supports Space/click strike triggers and `P` for auto-play. A Godot acceptance script checked:

- four actual frames and native 510 ms nonuniform duration;
- exact frames at 90/100/140/180 ms boundaries;
- completion only once after 510 ms;
- pause, seek, replay and double-clock-safe manual advancement;
- save/reload of the delivered scene with resources intact;
- monster texture native size;
- an injected Space key triggering the playable scene once.

`forge godot verify --acceptance-script tests/acceptance.gd --screenshot --frames 8` passed in an isolated project/user profile. The screenshot was inspected; edge cleanup remains a visual, not structural, claim. A separate visible Godot window was controlled with keyboard and mouse: the `P` toggle, Space and click each printed/updated an additional strike trigger, reaching counter 30.

## Native export

Initial export correctly reported missing isolated-profile template configuration. After using the checksum-verified official `templates/macos.zip` as a custom release template and enabling the project setting for ETC2/ASTC imports required by the macOS universal/arm64 preset, `forge godot export --preset Desktop --run` passed. It produced `game.zip` SHA-256 `e76863ae7d2f64bdd22f0ce8455fcb3be3f3c55a186f5fd00b09bd26211d02e1` and completed the bounded headless startup smoke check. This proves native export and startup, not signing/notarization, device graphics or gameplay balance.

## Result

The PR behavior is locally accepted for this macOS real-asset path: installed PR binary, real generated image sources, local matting/normalization/effect-quality evidence, library revisions/locks, transactional Godot 4.7.2 delivery, native resource loading/playback/interaction, and a native macOS export with startup.

Remaining boundaries: Windows native verification still belongs on the Windows machine/CI; animation quality remains experimental and requires art review; the final art's source rights were not assessed.

## Follow-up commit

`6b445ce` adds this record. The PR page carries the same acceptance summary and links this file.
