# Forge CLI

[中文 README](README.zh-CN.md)

Forge is an open-source, agent-first command-line pipeline for generating
consistent 2D game assets and installing them into Godot 4.6.x projects.
Codex, Claude, scripts, and CI can call the same stable JSON protocol.

The current macOS Apple Silicon release supports:

- immutable project Style Locks;
- consistent top-down Character Packs with `idle`, `walk_up`, `walk_right`, and
  `walk_down` animations;
- consistent icon sets and prop sets derived from one style board and anchor;
- direct xAI REST generation through API Key or Preview OAuth, without Grok
  Build CLI;
- deterministic matting, normalization, consistency gates, targeted item
  and stage-level retry, Loop Selection V2, provenance, and `.gsfpack` validation;
- atomic Godot installation with external textures, small native resources,
  usage metadata, ownership checks, and rollback.

## Install

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

Open a new terminal and verify the installation:

```bash
forge doctor --json
```

Forge installs `forge`, `ffmpeg`, and `ffprobe` into a versioned user directory and
exposes only `forge` on `PATH`. The installer verifies both the release archive and
its per-file SHA-256 manifest before switching versions. The first CLI release is
unsigned and not notarized. Godot is not bundled; install Godot 4.6.x before using
engine delivery.

## Five-minute xAI to Godot flow

Authenticate without putting an API key in shell history:

```bash
forge provider login --provider xai --method api-key
```

Create an asset project:

```bash
forge project init --path "$PWD/game-assets" --name "My Game"
```

Create `game-assets/specs/style.json`:

```json
{
  "schemaVersion": "1",
  "prompt": "compact jewel-tone pixel art with dark outlines",
  "referenceImages": [],
  "perspective": "topdown",
  "lighting": "upper_left",
  "outline": "dark",
  "background": "transparent",
  "sampling": "nearest",
  "characterCanvasSize": 256,
  "iconCanvasSize": 128,
  "propCanvasSize": 256
}
```

Lock the style:

```bash
forge style create \
  --project "$PWD/game-assets" \
  --spec "$PWD/game-assets/specs/style.json" \
  --wait --json
```

Create `game-assets/specs/ranger.json`:

```json
{
  "schemaVersion": "1",
  "kind": "character",
  "id": "forest-ranger",
  "name": "Forest Ranger",
  "prompt": "a compact forest ranger with a green hood",
  "license": "private"
}
```

Generate the Character Pack:

```bash
forge generate character \
  --project "$PWD/game-assets" \
  --spec "$PWD/game-assets/specs/ranger.json" \
  --wait --json
```

Inspect the returned `.gsfpack`, then prepare and execute the separate Godot
write plan:

```bash
forge godot plan-install \
  --pack /absolute/path/Forest-Ranger.gsfpack \
  --project /absolute/path/my-godot-game \
  --asset-key forest_ranger --json

forge plan execute --token <returned-token> --wait --json
```

## Icon and prop sets

Icon and prop specs use the same shape:

```json
{
  "schemaVersion": "1",
  "kind": "icon_set",
  "id": "inventory-icons",
  "name": "Inventory Icons",
  "items": [
    { "id": "potion", "name": "Potion", "prompt": "a red healing potion" },
    { "id": "key", "name": "Key", "prompt": "a small brass key" }
  ],
  "license": "private"
}
```

```bash
forge generate icon-set --project "$PWD/game-assets" --spec /absolute/icons.json --json
forge generate prop-set --project "$PWD/game-assets" --spec /absolute/props.json --json
forge job report --id <job-id> --json
forge job retry --id <job-id> --item potion --wait --json

# Re-score existing icon/prop pixels against the current Style Lock without a Provider call.
forge job retry --id <static-job> --stage consistency --wait --json

# Character-only retries: local loop/matting reruns do not call the Provider.
forge job retry --id <character-job> --item walk_right --stage loop --wait --json
forge job report --id <new-job-id> --json
```

Generation defaults to an asynchronous durable job. Add `--wait` for a
synchronous result. All public commands write one JSON envelope to stdout;
diagnostics and interactive authorization stay on stderr/TTY.

Style Locks use the versioned `style-baseline@2.3.0` foreground-aware palette.
Recreating a Style Lock after a baseline upgrade preserves the old immutable revision and
reuses its verified style board when possible, so migration does not require regeneration.

Character generation evaluates the full video, selects a real closed cycle, and
exports only the selected `[start, end)` frames. The matching boundary frame is kept
as quality evidence but is not duplicated in the animation. `job report` exposes the
selected indices, score components, retry method, and whether the retry made a paid
Provider request.

The Character release gate is complete: three consecutive clean real-xAI Character →
Pack → Godot runs passed all four actions without manual review. The evidence and
provider-cost/retry audit are recorded in
[`docs/qa/forge-character-loop-v2-2026-08-03.md`](docs/qa/forge-character-loop-v2-2026-08-03.md).
The remaining `v0.2.0-cli.1` gate is a clean installation from the published GitHub Release.

## Unreleased Character consistency V2

The source tree contains an opt-in `consistency-v2` build for the next CLI release. It adds
immutable Subject Locks, semantic image-reference roles, 8-frame-per-action keyframe generation,
typed WorkflowGraph replay, a content-addressed cache, and `.forge/catalog.json`. These commands
are deliberately absent from the default `v0.2.0-cli.1` binary until the real-xAI acceptance gate
is complete. The signed SAM/DINO/LPIPS component remains unpublished until license and calibration
review; `forge component install` never substitutes unreviewed weights.
The implemented offline contracts, Godot verification, and remaining external gates are recorded in
[`docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md`](docs/qa/forge-consistency-v2-and-world-implementation-2026-08-03.md).

## Unreleased world pipeline

The next CLI milestones are implemented behind unreleased world build features and the
V3 Pack contract. These commands are intentionally absent from the `v0.2.0-cli.1`
release binary. Terrain, Building, and Map ship separately after Character consistency V2:

- immutable top-down Environment Locks;
- deterministic 16/32 px dual-grid Terrain Sets built from two Provider-generated
  material plates;
- modular exterior Building Kits with fixed roof, wall, door, and window modules;
- a Provider-free JSON Map Compiler that produces a self-contained Godot world.

Create the Environment Lock and world art:

```bash
forge environment create --project /absolute/assets --spec /absolute/environment.json --wait --json
forge generate terrain-set --project /absolute/assets --spec /absolute/terrain.json --wait --json
forge generate building-kit --project /absolute/assets --spec /absolute/buildings.json --wait --json
```

Maps accept JSON only. Forge does not call a text model or translate natural language;
Codex, Claude, or a user writes `MapSpecV1`, then Forge validates and deterministically
compiles it:

```bash
forge map schema --json
forge map compile --project /absolute/assets --spec /absolute/map.json --wait --json
forge map validate --pack /absolute/Forest-Village.gsfpack --json
```

The V1 world scope is top-down outdoor maps, dual-grid Terrain Sets, rectangular
3×3–8×6 exterior buildings with south-facing entrances, and Godot 4.6.x. It does
not include indoor, isometric, platformer, 3D, Tiled, Unity, or Unreal output.
Runnable JSON examples live under [`examples/cli/world`](examples/cli/world).

## Security and provenance

- Provider output is materialized, format-checked, and SHA-256 hashed before
  local processing.
- A job locks one Provider, profile, model selection, and Style revision.
- Credentials are stored in Keychain and never enter jobs, packs, logs, or
  normal JSON output.
- OAuth is Preview. API Key is the stable commercial authentication path.
- Godot writes are confined to `addons/forge_assets` and replace only
  Forge-owned output.

## Development

Contributor build and test instructions live in [CONTRIBUTING.md](CONTRIBUTING.md).
Architecture and automation contracts live under `docs/architecture` and
`docs/automation`.

## License

Forge is licensed under the [MIT License](LICENSE). Bundled FFmpeg helpers have
separate LGPL notices and corresponding source published with each release;
see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
