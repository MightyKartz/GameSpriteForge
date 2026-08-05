# Install and troubleshoot Forge Assets

## Build

From the Forge repository root, run:

```bash
scripts/package-forge-plugin.sh
```

This creates `target/release/forge-cli` and refreshes `plugins/forge-assets/mcp/server.bundle.mjs`. CLI and MCP are developer tools; they are not included in the Forge app or DMG.

## CLI resolution

The MCP searches the repository release build, then debug build, then `forge-cli` on `PATH`. An explicit path takes priority:

```bash
export FORGE_CLI="/absolute/path/to/Forge/target/release/forge-cli"
```

Run `forge-cli doctor --json` directly when `check_environment` cannot start the CLI. Confirm that its stdout is a single JSON envelope and that `godotPath` points to Godot 4 before installing into a project.

## Codex loading

The repository does not mutate a plugin marketplace automatically. Install the packaged `forge-assets` plugin through the local marketplace configured by the user, then start a new Codex task so the Skill and MCP inventory reload.

## Common recovery

- Missing CLI: rebuild the package or set `FORGE_CLI`.
- Expired/used token: prepare a new plan; never reuse the old token.
- `awaiting_review`: call `analyze_repair`. When it returns safe changes, review them and use `plan_repair_job`; otherwise call `open_job` and revise the original request from the review result.
- Repair limit reached: stop automatic retries after three attempts, open the latest job, and ask for a semantic decision such as loop range, source trimming, background mode, or anchor placement.
- Godot target rejected: choose a path under `addons/forge_assets` and do not replace a non-Forge-owned directory.
- Project manifest rejected: keep `<project>/.forge` and `.forge/assets.json` as real paths rather than symbolic links, then retry with a new plan.
- Asset identity collision: call `inspect_project`, choose whether to update the existing stable `assetKey` or install under a new key, and prepare a new plan.
- Video: use the Forge GUI. For multiple animations, use a schema V2 request with `plan_prepare_character_pack`.
