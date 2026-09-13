# Forge

English | [简体中文](./README.zh-CN.md)

**A CLI for 2D game art and audio.**

Bring assets from your preferred creative tools, manage them in a project library, and deliver them to Godot. Forge is built in Rust and works with Codex, Claude, terminal commands and scripts.

[Latest release](https://github.com/MightyKartz/GameSpriteForge/releases/latest) · [Install](#install) · [Features](#what-you-can-do) · [CLI guide](docs/automation/forge-cli.md)

![Props and a synthetic sound cue delivered by Forge v0.4.0 and played in Godot](docs/media/showcase/v040/native-delivery.gif)

*Real v0.4.0 output in a dedicated Godot demonstration scene. [Watch with sound](docs/media/showcase/v040/native-delivery.mp4) · [Sources and reproduction](docs/media/showcase/v040/README.md).*

## Install

[v0.4.0](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.4.0) supports **macOS Apple Silicon** and offers an **experimental Windows x64 portable package**. Install **Godot 4.6.x** separately for native delivery and preview. Packages are unsigned; the macOS package is not notarized.

On macOS:

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

Open a new terminal, then run:

```bash
forge --version
forge doctor --json
forge guide
```

On Windows, follow the [portable installation guide](docs/releases/windows-portable.md). For an existing game, verify the upgrade before changing its pinned CLI.

## What you can do

| Workflow | Capabilities |
| --- | --- |
| **Resource library** | Find assets by name or tag, preview media, compare versions, retain reusable files and transfer selected resources between macOS and Windows. |
| **Images** | Batch-process PNG icons and props; configure backgrounds, canvases, anchors and sampling. Generate sets through xAI with your own account when needed. |
| **Animation and layers** | Prepare existing frames or sprite sheets, preserve source coordinates and timing, and package aligned layers with transform and opacity tracks. |
| **Audio** | Import WAV music, sound effects and ambience; trim, adjust gain, add fades and prepare loops. |
| **Godot delivery** | Validate Packs and install native textures, scenes, animations and audio streams. Lock chosen versions, retain delivery receipts and roll back failed updates. |

## Use with Codex

Open your game project in Codex and ask:

> Use Forge with this game's source images, animation frames and WAV audio. Run `forge guide` first, preserve the originals, and show me the prepared results for review before installing them into Godot.

The embedded guide works offline. Read a topic with `forge guide project-assets`, `forge guide static`, `forge guide animation`, `forge guide audio` or `forge guide delivery`. An optional [Codex skill installation](docs/automation/forge-cli.md#optional-bundled-codex-skill) enables discovery on macOS/Linux.

## In use: Sword

Sword is a cultivation-themed survival game prototype. Codex created its source artwork; Forge processed and delivered these spell and enemy animations to Godot.

![Sword spells: fire, frost and lightning](docs/media/showcase/sword-spells.gif)

![Sword enemies: Wisp, Stone Golem, Vine Spirit and a Guardian slam](docs/media/showcase/sword-enemies.gif)

*Existing resources imported with earlier Forge builds, replayed in a dedicated asset showcase. [Source and capture details](docs/media/showcase/README.md).*

## Scope

Character animation remains experimental; advanced character and world-asset commands require optional source-build features. Check `forge doctor --json` for your executable's capabilities.

Music and sound effects are generated in external applications. ACE-Step and Stable Audio 3 have optional read-only source-directory detection; Forge does not install or run their models. Online xAI requests use your own account and may incur charges.

Technical validation does not replace visual review, listening or license checks. Review assets in your game before use. See the [v0.4.0 release notes](docs/releases/v0.4.0.md) for the verified release scope.

## Documentation

- [CLI guide](docs/automation/forge-cli.md) — commands and automation.
- [Resource library](docs/automation/project-asset-library.md) — registration, versions, review and reuse.
- [Local artwork](docs/automation/codex-local-assets.md), [audio](.agents/skills/forge-use/references/audio.md) and [layered Packs](docs/automation/layered-packs.md) — preparation workflows.
- [Example specifications](examples/cli) — starting points for asset requests.
- [Contributing](CONTRIBUTING.md) — source builds and development checks.

## License

[MIT](LICENSE). Bundled FFmpeg tools have separate LGPL notices and source distributions; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
