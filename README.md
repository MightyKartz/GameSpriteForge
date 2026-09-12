# Forge

English | [简体中文](./README.zh-CN.md)

**A 2D game asset pipeline for AI-assisted development.**

Forge connects artwork creation to native Godot delivery. It combines batch asset processing, quality checks, traceable asset Packs and engine integration in a Rust CLI that Codex, Claude and your own scripts can drive.

Build icon and prop libraries from an art brief, bring in artwork from your preferred tools, or explore existing animation frames in a game prototype. Forge carries the work through processing, review and delivery while preserving the source artwork.

[Latest release](https://github.com/MightyKartz/GameSpriteForge/releases/latest) ·
[Get started](#install) ·
[CLI guide](docs/automation/forge-cli.md) ·
[Examples](examples/cli)

## Build assets with Codex

After [installing the CLI](#install), open your game project in Codex and ask:

> Use Forge and an available image tool to build forest-themed inventory icons and scene props for this Godot game. Run `forge guide` first, keep the source artwork, check the results, and deliver the assets into the project.

Codex creates the source art with its image tool; Forge handles asset processing, validation and Godot delivery. The CLI includes workflow guidance, request examples and structured JSON results, so coding agents can plan work, inspect progress and check the outcome.

## Build asset sets around your art direction

Start with AI-generated artwork, hand-drawn sprites or an existing library. Batch-process transparent PNGs into icon and prop sets for inventories, pickups and game scenes. Local processing runs on your machine without a Forge Provider account.

For generation through Forge, use a fixed style reference to guide an online provider. Consistency reports help you inspect palette, scale and other deviations; retry selected icons or props as you refine the set. Provider generation uses your own account and may incur charges.

![Illustrative forest icon and prop set](docs/media/showcase/gallery.png)

*A forest-themed icon and prop collection. Source art generated separately with Codex; sprites processed locally with Forge.*

Start with the [local artwork workflow](docs/automation/codex-local-assets.md), or explore [provider generation examples](examples/cli).

## Control how your sprites appear in game

Split sprite sheets, clean up keyed backgrounds and normalize sprite canvases. Use centered origins for icons and ground anchors for props, with crisp pixel sampling or smooth filtering to suit the artwork. Preview the results and inspect quality reports before delivery.

For existing animation frames, the experimental workflow can preserve drawing coordinates and individual frame durations through export. Source originals and processing records remain available for later revisions.

![Existing media prepared as transparent sprite frames](docs/media/showcase/processing.png)

*From a keyed source sheet to a transparent sprite: Forge's local background-removal workflow.*

## Validate assets and deliver native Godot resources

Forge packages textures and asset metadata into verifiable Packs, then installs them into your Godot project. Icon sets deliver PNG textures; prop sets add reusable scenes with anchor and rendering settings. Animation Packs provide native `SpriteFrames` resources and `AnimatedSprite2D` scenes. Updates replace Forge-managed assets and restore the previous installation if delivery fails.

Input hashes, processing settings and job reports let you trace an asset back to its source and inspect what changed between iterations. Pack validation checks the delivery's structure and integrity; visual review determines whether the art suits your game.

### In use: Sword

Sword is a cultivation-themed survival game prototype using Forge for asset processing and delivery. Its spell effects and enemy animations started as Codex-generated artwork, were processed and delivered by Forge, and are replayed below in a dedicated Godot asset showcase.

![Sword prototype spells: fire, frost and lightning](docs/media/showcase/sword-spells.gif)

*Fire, frost and lightning — existing spell frames delivered to Godot.*

![Sword prototype enemies: Wisp, Stone Golem, Vine Spirit and a Guardian slam attack](docs/media/showcase/sword-enemies.gif)

*Wisp, Stone Golem, Vine Spirit and a Guardian slam — prototype enemy animations.*

## Install

Stable version: [v0.3.2](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.3.2) for **macOS Apple Silicon**. Install **Godot 4.6.x** separately for engine delivery. The binaries are unsigned and not notarized. For an existing game, retain its pinned CLI until you have verified an upgrade.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

Open a new terminal, then check the installation:

```bash
forge --version
forge doctor --json
forge guide
```

`forge guide` provides the workflow instructions and request examples bundled with your CLI version, even offline.

For your first set, follow the [local artwork workflow](docs/automation/codex-local-assets.md): prepare your images, review the results, validate the Pack and install it into Godot.

## Current scope

Local icon and prop processing, Pack validation and Godot delivery are the stable foundation. **Character animation remains in development and testing**, including generation, reuse and directional workflows. The Sword previews demonstrate specific prototype assets; review animation results before using them in your game. See the [v0.3.2 release notes](docs/releases/v0.3.2.md) for the release scope.

## Documentation

- [CLI guide](docs/automation/forge-cli.md) — commands, generation, processing, and Godot delivery.
- [Local artwork workflow](docs/automation/codex-local-assets.md) — take artwork from Codex or other tools through processing and Godot delivery.
- [Delivery evidence](.agents/skills/forge-use/references/delivery.md) — source-build additions for reviewed hashes, portable receipts and installed-resource audits; check the selected binary's capabilities.
- [Example specifications](examples/cli) — starting points for your own assets.
- [Release notes](docs/releases/v0.3.2.md) — platform support and release scope.
- [Showcase assets](docs/media/showcase/README.md) — artwork sources and Godot previews.
- [Contributing](CONTRIBUTING.md) — source builds and development checks.

## License

[MIT](LICENSE). Bundled FFmpeg tools have separate LGPL notices and source distributions; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
