# Forge

English | [简体中文](./README.zh-CN.md)

**Help AI coding agents build Godot games.**

Forge is a **free, open-source CLI under the MIT license**, connecting Codex and other coding agents, creative tools, and Godot. Prepare and manage generated or existing 2D images, animations, and audio as reusable game assets, deliver them to your project, then check the game, capture screenshots, and export builds for your desktop platform.

[Latest release](https://github.com/MightyKartz/GameSpriteForge/releases/latest) · [Install](#install) · [Features](#what-you-can-do) · [CLI guide](docs/automation/forge-cli.md)

![Thunder spirits and lightning playing in Godot](docs/media/showcase/thunder/godot-demo.gif)

*Created with Codex, prepared and managed with Forge, delivered to Godot. [Watch the replay](docs/media/showcase/thunder/godot-demo.mp4) · [Source sheets and production notes](docs/media/showcase/thunder/README.md).*

## Install

[v0.5.0](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.5.0) supports **macOS Apple Silicon** and offers an **experimental Windows x64 portable package**. For Godot workflows, select an existing **Godot 4.6.x** installation or let Forge download the pinned official engine as described below. Packages are unsigned; the macOS package is not notarized.

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

To use an existing Godot installation, run `forge setup godot --path PATH`, replacing `PATH` with your engine location. If Godot is not installed, download and configure the pinned official Godot 4.6.3:

```bash
forge setup godot --download
```

Add `--templates` when you also need export templates; they are a separate download. Read `forge guide godot-workflow` for setup and export examples.

## What you can do

| Workflow | Capabilities |
| --- | --- |
| **Resource library** | Find assets by name or tag, preview media, compare versions, retain reusable files and transfer selected resources between macOS and Windows. |
| **Images** | Batch-process PNG icons and props; configure backgrounds, canvases, anchors and sampling. Generate sets through xAI with your own account when needed. |
| **Animation and layers** | Prepare existing frames or sprite sheets, preserve source coordinates and timing, and package aligned layers with transform and opacity tracks. |
| **Audio** | Import WAV music, sound effects and ambience; trim, adjust gain, add fades and prepare loops. |
| **Godot delivery** | Validate Packs and install native textures, scenes, animations and audio streams. Lock chosen versions, retain delivery receipts and roll back failed updates. |
| **Godot development** | Configure the engine, lock project tool versions, check game startup, capture screenshots and export/test builds for the desktop platform you are using. |

### Audio delivery

Forge prepares local WAV files and delivers native audio resources to Godot. This example plays a synthesized three-note chime in a Godot demonstration scene.

![Godot playback of a synthetic sound cue delivered by Forge](docs/media/showcase/v040/native-delivery.gif)

*The GIF is silent. [Watch with sound](docs/media/showcase/v040/native-delivery.mp4) · [Sources and reproduction](docs/media/showcase/v040/README.md).*

### Check and export your Godot game

After delivering assets, let your agent check the game in an isolated project copy, capture a screenshot for review, and inspect logs when something fails. Project toolchain locks help keep Forge and Godot versions consistent across machines.

Forge can also use an existing export preset to build for your current desktop platform and check that the exported game starts. You still review the visuals, sound and gameplay. See the [Godot workflow guide](.agents/skills/forge-use/references/godot-workflow.md) for commands and detailed verification behavior.

## Use with Codex

Open your game project in Codex and ask:

> Help me prepare and manage this game's source images, animation frames and WAV audio with Forge. Run `forge guide` first, preserve the originals, and register the assets in the project library. Show me the prepared results for review before installing them into Godot. Then use Forge to check the game's startup and capture a screenshot; report any errors and what still needs my review.

The embedded guide works offline. Read a topic with `forge guide project-assets`, `forge guide static`, `forge guide animation`, `forge guide audio` or `forge guide delivery`. An optional [Codex skill installation](docs/automation/forge-cli.md#optional-bundled-codex-skill) enables discovery on macOS/Linux.

## In use: Sword

Sword is a cultivation-themed survival game prototype. Codex created its source artwork; Forge processed and delivered these spell and enemy animations to Godot.

![Sword spells: fire, frost and lightning](docs/media/showcase/sword-spells.gif)

![Sword enemies: Wisp, Stone Golem, Vine Spirit and a Guardian slam](docs/media/showcase/sword-enemies.gif)

*Existing resources imported with earlier Forge builds, replayed in a dedicated asset showcase. [Source and capture details](docs/media/showcase/README.md).*

## Scope

Character animation remains experimental; advanced character and world-asset commands require optional source-build features. Check `forge doctor --json` for your executable's capabilities.

Music and sound effects are generated in external applications. ACE-Step and Stable Audio 3 have optional read-only source-directory detection; Forge does not install or run their models. Online xAI requests use your own account and may incur charges.

Technical validation does not replace visual review, listening or license checks. Review assets in your game before use. See the [v0.5.0 release notes](docs/releases/v0.5.0.md) for the verified release scope.

## Documentation

- [CLI guide](docs/automation/forge-cli.md) — commands and automation.
- [Resource library](docs/automation/project-asset-library.md) — registration, versions, review and reuse.
- [Godot workflow](.agents/skills/forge-use/references/godot-workflow.md) — engine setup, toolchain locks, game checks and desktop export.
- [Local artwork](docs/automation/codex-local-assets.md), [audio](.agents/skills/forge-use/references/audio.md) and [layered Packs](docs/automation/layered-packs.md) — preparation workflows.
- [Example specifications](examples/cli) — starting points for asset requests.
- [Contributing](CONTRIBUTING.md) — source builds and development checks.

## License

[MIT](LICENSE). Bundled FFmpeg tools have separate LGPL notices and source distributions; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
