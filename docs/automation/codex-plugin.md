# Forge Assets Codex plugin

Forge automation is intentionally split from the macOS app package:

- `forge-cli` owns the JSON protocol, plan tokens, jobs, image pipeline, and Godot writes.
- `forge-assets` MCP is a TypeScript stdio adapter that invokes the CLI.
- The Codex Skill teaches the safe inspect → prepare → execute → poll/open → register workflow.
- The Forge app remains the human review and recovery surface.

## Build the local package

From the repository root:

```bash
scripts/package-forge-plugin.sh
```

This builds `target/release/forge-cli` and copies the single-file MCP bundle into `plugins/forge-assets/mcp/server.bundle.mjs`. The plugin also accepts an explicit CLI path through `FORGE_CLI`.

## Validate

```bash
python3 ~/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py plugins/forge-assets
python3 ~/.codex/skills/.system/skill-creator/scripts/quick_validate.py plugins/forge-assets/skills/forge-assets
```

## Install locally

This repository does not change a marketplace automatically. To use the packaged plugin, add this repository as a local marketplace according to your Codex setup, then install `forge-assets` from that marketplace. If your Codex process cannot resolve the repository build, launch it with:

```bash
export FORGE_CLI="/absolute/path/to/Forge/target/release/forge-cli"
```

Start a new Codex task after installing or updating the plugin so the Skill and MCP tool inventory is reloaded.

## Supported workflows

- Schema V1 / `plan_prepare_asset`: one animation from a PNG sequence, fixed-grid sheet, transparent-gutter sheet, or existing `.gsfpack`.
- Schema V2 / `plan_prepare_character_pack`: a versioned Platformer, Top-down, Isometric, or Custom Character Workflow normalized to one canvas and foot anchor, with per-animation FPS and loop behavior.
- Schema V3 / `plan_generate_character_pack`: lock one `xai` or offline `fixture` Provider to a `topdown@1.0.0` job, generate four directional clips, and export only after every clip is `game_ready`.
- `list_providers` / `check_provider`: inspect capabilities and authentication without exposing credentials. OAuth login remains an explicit user-only CLI action.
- `list_character_workflows`: return the current machine-checkable contracts before preparing a guided character.
- `plan_install_godot`: install either pack shape as neutral `SpriteFrames` and `AnimatedSprite2D` resources, then register a stable asset identity in the project manifest.
- `inspect_project`: read `<project>/.forge/assets.json` so Codex can rediscover generated scenes, animations, revisions, and optional provider source IDs in a later task.
- `analyze_repair`: turn an awaiting-review job's quality evidence into explicit safe parameter changes and manual decision IDs without writing anything.
- `plan_repair_job`: create a normal single-use plan that executes the reviewed repair as a new linked job and emits before/after quality comparison evidence.

Character Pack exports include a default preview, per-animation previews, aggregate and per-animation quality evidence, and a backward-compatible `.gsfpack` manifest. Godot installs include `forge_usage.json` beside the generated scene. CLI/MCP accept exact-range `video_clip` inputs; merging existing `.gsfpack` files into a Character Pack is deferred.

The MCP reports `repair_comparison` as a local artifact path; Codex reads that JSON file directly. Its `improved` flag is a routing hint rather than sufficient visual proof, so the Skill also checks the affected animation's verdict, alpha coverage, notes, recommendations, and preview. Manual replacement requests are intentionally outside automatic repair lineage in V1; retain the originating job ID in the task handoff.
