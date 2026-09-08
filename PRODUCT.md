# Product

**A 2D game asset pipeline for AI-assisted development.**

Forge helps independent developers and small teams turn artwork into reusable
game resources. Its public product is a Rust CLI that people, coding agents and
scripts can use throughout asset production, from local preparation and quality
checks to traceable Packs and native Godot delivery.

## Who it serves

Developers working with Codex, other image tools or existing artwork need more
than a collection of generated images. They need assets with consistent framing,
intentional placement, usable engine resources and a record of how each result
was produced. Forge connects that creative work to the game project.

## What Forge provides

- Prepare transparent PNG icons and props on consistent canvases with suitable
  texture sampling and placement anchors.
- Inspect processing results and quality reports, then validate the asset Pack.
  Structural checks and visual approval remain separate decisions.
- Preserve source originals, hashes and processing evidence so revisions can be
  traced back to their inputs instead of replacing the asset's history.
- Deliver textures, prop scenes and usage metadata into Godot, with stable asset
  identities, owned installation targets and recovery after failed installs.
- Reuse existing animation frames through experimental local preparation that
  can preserve shared drawing coordinates, anchors and frame timing through
  Pack and Godot SpriteFrames delivery.

## Artwork and generation

The primary route starts with artwork created in Codex or another image tool,
or transparent PNGs the developer already has. Image creation is a separate
creative step; Forge processes the local files without a Provider account.

Forge also supports controlled online icon and prop generation through a
selected Provider and Style. This route uses the developer's Provider account,
records generation provenance and request usage, and supports targeted retries.
Provider requests can incur charges; local processing has separate usage evidence.

## Working with an AI assistant

Install the CLI and ask the assistant to run `forge guide` before preparing the
assets. The guide and request examples travel with the executable and work
offline. Keep a game's verified Forge version when continuing an existing project.
The workflow supports reviewing and delivering an asset set the game can consume,
with source history and delivery evidence available for later revisions and reuse.

## Current release scope

[v0.3.2](docs/releases/v0.3.2.md) supports macOS Apple Silicon. The stable route
is local transparent PNG preparation, static Packs and Godot delivery. Install
Godot 4.6.x separately. Current CLI binaries are unsigned and not notarized.

Character animation remains in development and testing. Existing animation
processing and reviewed project examples do not establish that automatic animation
generation is ready for general production use. Advanced character and world-asset workflows require
optional source-build features and are outside the default release workflow.

Product communication should make actual capabilities, results and review needs
clear. It should help developers create and reuse assets without promising that
generated artwork is automatically ready for every game.

## Historical context

Earlier Forge work produced a Tauri + React desktop MVP focused on local media
import and animation processing. Its retained application code, early 0.1.0
packages and signing evidence are historical; the desktop and MCP clients are
outside the current default CLI build and release. See
[Contributing](CONTRIBUTING.md) for the current development boundaries.
