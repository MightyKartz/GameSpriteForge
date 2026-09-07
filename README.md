# Forge

English | [简体中文](./README.zh-CN.md)

**Create 2D game assets, prepare your sprites, and bring them into Godot.**

Forge is a CLI for generating icon and prop sets, processing existing images and animation, and delivering assets to your game. Use it from the terminal, or let Codex and Claude handle the workflow with you.

[Latest release](https://github.com/MightyKartz/GameSpriteForge/releases/latest) ·
[Get started](#install) ·
[CLI guide](docs/automation/forge-cli.md) ·
[Examples](examples/cli)

## Generate icons and props in a shared style

Create inventory icons, pickups, and scene props from descriptions. A reusable style guide helps keep each set visually coordinated, and you can retry an individual item while keeping the rest.

![Illustrative forest icon and prop set](docs/media/showcase/gallery.png)

*Forest-themed artwork generated separately with AI for this showcase, then prepared with Forge.*

Start with the [icon set](examples/cli/icons.json), [prop set](examples/cli/props.json), and [style](examples/cli/style.json) examples.

## Turn existing media into usable sprites

Bring your own video, PNG sequence, or sprite sheet. Remove backgrounds, align frames, choose animation loops, and build sprite sheets locally—so existing artwork can move into your game workflow.

![Existing media prepared as transparent sprite frames](docs/media/showcase/processing.png)

*One sprite from the source sheet, before and after Forge background removal.*

## Bring assets into Godot

Deliver textures, atlases, and native animation resources to a Godot project. Forge preserves frame timing and rendering settings, so you can preview the result in the engine and refine it for your game.

![Godot demo scene assembled with the processed PNG sprites](docs/media/showcase/godot.png)

*A demo scene assembled in Godot using the PNGs prepared by Forge.*

### Work at your own pace

Check task progress, inspect results, and retry the items that need another pass. Forge works with terminal commands, scripts, and coding agents, making it practical to prepare a few assets or repeat a larger batch.

## Install

Current release: [v0.2.1](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.2.1) for **macOS Apple Silicon**. Install **Godot 4.6.x** separately for engine delivery. The current binaries are unsigned and not notarized.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

Open a new terminal, then check the installation:

```bash
forge --version
forge doctor --json
```

For your first set, follow the [CLI guide](docs/automation/forge-cli.md) with the [icon](examples/cli/icons.json) or [prop](examples/cli/props.json) example. Online generation uses your provider account and may incur charges; local asset processing does not require new generation.

## In development

**Character animation is still being developed and tested.** Generation, animation reuse, and directional workflows need visual review. The experimental reuse helper is available in the source repository; it is not installed with the CLI.

Advanced character consistency and world-asset workflows also require optional source-build features. See the [release notes](docs/releases/v0.2.1.md) for what is included in the current download.

## Documentation

- [CLI guide](docs/automation/forge-cli.md) — commands, generation, processing, and Godot delivery.
- [Example specifications](examples/cli) — starting points for your own assets.
- [Release notes](docs/releases/v0.2.1.md) — platform support and release scope.
- [Showcase assets](docs/media/showcase/README.md) — image sources and the reproducible Godot demo.
- [Contributing](CONTRIBUTING.md) — source builds and development checks.

## License

[MIT](LICENSE). Bundled FFmpeg tools have separate LGPL notices and source distributions; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
