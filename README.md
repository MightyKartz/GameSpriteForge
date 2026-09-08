# Forge

English | [简体中文](./README.zh-CN.md)

**Turn your PNG artwork into game assets for Godot.**

Forge is a CLI for preparing 2D icons and props. Create artwork with Codex or your preferred image tool, bring the transparent PNGs into Forge, and deliver the resulting asset set to your game. Use it from the terminal, or let Codex and Claude handle the workflow with you.

[Latest release](https://github.com/MightyKartz/GameSpriteForge/releases/latest) ·
[Get started](#install) ·
[CLI guide](docs/automation/forge-cli.md) ·
[Examples](examples/cli)

## Bring artwork from your preferred tools

Use AI-generated artwork, hand-drawn sprites, or images you already have. Import one transparent PNG per inventory icon, pickup, or scene prop. Local preparation needs no Forge Provider account or generated style guide.

![Illustrative forest icon and prop set](docs/media/showcase/gallery.png)

*Forest-themed artwork generated separately with AI for this showcase, then prepared with Forge.*

Start with the [PNG → Forge → Godot guide](docs/automation/codex-local-assets.md).

## Prepare a set for your game

Prepare transparent icons and props on consistent canvases, choose the texture sampling that suits your artwork, and inspect the results before delivery. Forge keeps the source originals so you can revise the set. Background-removal tools are also available for artwork that needs cleanup.

![Existing media prepared as transparent sprite frames](docs/media/showcase/processing.png)

*One sprite from the source sheet, before and after Forge background removal.*

## Bring assets into Godot

Deliver a static asset Pack to your Godot project, with textures, ready-to-use prop scenes, and placement and rendering settings. Preview your assets in the engine and adjust their scale and behavior for your game.

![Godot demo scene assembled with the processed PNG sprites](docs/media/showcase/godot.png)

*A demo scene assembled in Godot using the PNGs prepared by Forge.*

### Work at your own pace

Check task progress, inspect results, and submit a revised set when artwork changes. Forge works with terminal commands, scripts, and coding agents, making it practical to prepare a few assets or repeat a larger batch.

## Install

Stable version: [v0.3.0](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.3.0) for **macOS Apple Silicon**. Install **Godot 4.6.x** separately for engine delivery. The binaries are unsigned and not notarized.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

Open a new terminal, then check the installation:

```bash
forge --version
forge doctor --json
```

For your first set, follow the [local PNG guide](docs/automation/codex-local-assets.md). It covers importing transparent images, checking the Pack, and installing it into Godot.

Forge can also generate icon and prop sets through an online provider. See the [CLI guide](docs/automation/forge-cli.md) and [example specifications](examples/cli). Online generation uses your provider account and may incur charges; local preparation uses the artwork you already have.

## In development

**Character animation is still being developed and tested.** Generation, animation reuse, and directional workflows need visual review. The experimental reuse helper is available in the source repository; it is not installed with the CLI.

Advanced character consistency and world-asset workflows also require optional source-build features. See the [v0.3.0 release notes](docs/releases/v0.3.0.md) for the release scope.

## Documentation

- [CLI guide](docs/automation/forge-cli.md) — commands, generation, processing, and Godot delivery.
- [PNG artwork workflow](docs/automation/codex-local-assets.md) — prepare PNGs from Codex or other tools and deliver them to Godot.
- [Agent usage skill](.agents/skills/forge-use/SKILL.md) — guidance for preparing and delivering assets with Codex.
- [Example specifications](examples/cli) — starting points for your own assets.
- [Release notes](docs/releases/v0.3.0.md) — platform support and release scope.
- [Showcase assets](docs/media/showcase/README.md) — image sources and the reproducible Godot demo.
- [Contributing](CONTRIBUTING.md) — source builds and development checks.

## License

[MIT](LICENSE). Bundled FFmpeg tools have separate LGPL notices and source distributions; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
