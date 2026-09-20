# Forge

English | [简体中文](./README.zh-CN.md)

**A CLI for 2D game art and audio.**

Bring assets from your preferred creative tools, manage them in a project library, and deliver them to Godot. Forge is built in Rust and works with Codex, Claude, terminal commands and scripts.

[Latest release](https://github.com/MightyKartz/GameSpriteForge/releases/latest) · [Install](#install) · [Features](#what-you-can-do) · [CLI guide](docs/automation/forge-cli.md)

![Thunder spirits and lightning playing in Godot](docs/media/showcase/thunder/godot-demo.gif)

*Created with Codex, prepared and managed with Forge, delivered to Godot. [Watch the replay](docs/media/showcase/thunder/godot-demo.mp4) · [Source sheets and production notes](docs/media/showcase/thunder/README.md).*

## Install

[v0.6.3](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.6.3) supports **macOS Apple Silicon** and offers an **experimental Windows x64 portable package**. Install **Godot 4.6.x or 4.7.x** separately for native delivery and preview. Packages are unsigned; the macOS package is not notarized.

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

Download the single `forge-windows-installer.zip` from the latest release,
extract it, and run the included installer. It discovers and verifies the bundled
archive automatically:

```powershell
Expand-Archive .\forge-windows-installer.zip .\forge-windows
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\forge-windows\install-windows.ps1
& "$env:LOCALAPPDATA\GameSpriteForge\bin\forge.cmd" doctor --json
```

## What you can do

| Workflow | Capabilities |
| --- | --- |
| **Resource library** | Find assets by name or tag, preview media, compare versions, retain reusable files and transfer selected resources between macOS and Windows. |
| **Images** | Batch-process PNG icons and props; configure backgrounds, canvases, anchors and sampling. Generate sets through xAI with your own account when needed. |
| **Animation and layers** | Prepare existing frames or sprite sheets, preserve source coordinates and timing, and package aligned layers with transform and opacity tracks. |
| **Audio** | Import WAV music, sound effects and ambience; trim, adjust gain, add fades and prepare loops. |
| **Godot delivery** | Validate Packs and install native textures, scenes, animations and audio streams. Lock chosen versions, retain delivery receipts and roll back failed updates. |

**New in v0.6.3:** flat animation Packs can be reviewed from
original PNGs with frame stepping and background selection, or exported on demand
as H.264 MP4. Windows packages include Media Foundation H.264 encoding. Check
`doctor --json` for `pack_mp4_preview`. See the [preview guide](.agents/skills/forge-use/references/animation.md#sharing-a-video-preview)
for commands and timing/transparency limits.

### Audio delivery

Forge prepares local WAV files and delivers native audio resources to Godot. This example plays a synthesized three-note chime in a Godot demonstration scene.

![Godot playback of a synthetic sound cue delivered by Forge](docs/media/showcase/v040/native-delivery.gif)

*The GIF is silent. [Watch with sound](docs/media/showcase/v040/native-delivery.mp4) · [Sources and reproduction](docs/media/showcase/v040/README.md).*

### Godot setup and acceptance

Builds with `godot_environment_setup` provide `forge setup godot --path PATH`
(or `--download` for the pinned official engine), persistent machine configuration,
`forge godot lock/check`, and isolated `forge godot verify/export` commands.
Acceptance uses a fresh per-run `user://` profile and resolves relative custom
export templates against the original project without changing its preset.
Read `forge guide godot-workflow` for examples, screenshots, explicit template
installation and native exported-program checks. These commands are included in
v0.5.0; check the selected executable's capabilities. Runtime/screenshot evidence does not establish
visual or gameplay approval. [Workflow guide](.agents/skills/forge-use/references/godot-workflow.md).

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

**New in v0.6.0:** builds with `godot_version_selection` support Godot 4.7.x alongside 4.6.x, with selectable 4.7.2 and 4.6.3 downloads and matching export templates. The new default download is 4.7.2; existing project locks remain unchanged. See the [Godot workflow guide](.agents/skills/forge-use/references/godot-workflow.md).

**New in v0.6.1:** Windows users download a single `forge-windows-installer.zip` containing the verified package, checksum and installer; extraction plus one script installs everything. Capabilities are unchanged from v0.6.0.

**New in v0.6.2:** optional `border_connected` chroma matting protects enclosed details that match the key color, and `edgeColorRecovery` restores soft edge colors from a flat background. Guides now recommend clean transparent RGBA sources and `preserve_alpha` first, with matting as a reviewed fallback.

Character animation remains experimental; advanced character and world-asset commands require optional source-build features. Check `forge doctor --json` for your executable's capabilities.

Music and sound effects are generated in external applications. ACE-Step and Stable Audio 3 have optional read-only source-directory detection; Forge does not install or run their models. Online xAI requests use your own account and may incur charges.

Technical validation does not replace visual review, listening or license checks. Review assets in your game before use. See the [v0.6.3 release notes](docs/releases/v0.6.3.md) for the verified release scope.

## Documentation

- [CLI guide](docs/automation/forge-cli.md) — commands and automation.
- [Resource library](docs/automation/project-asset-library.md) — registration, versions, review and reuse.
- [Local artwork](docs/automation/codex-local-assets.md), [audio](.agents/skills/forge-use/references/audio.md) and [layered Packs](docs/automation/layered-packs.md) — preparation workflows.
- [Example specifications](examples/cli) — starting points for asset requests.
- [Contributing](CONTRIBUTING.md) — source builds and development checks.

## License

[MIT](LICENSE). Bundled FFmpeg tools have separate LGPL notices and source distributions; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
