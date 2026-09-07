# Forge

English | [简体中文](./README.zh-CN.md)

**An agent-first CLI for generating, processing, and reusing 2D game assets, with provenance and Godot delivery.**

Forge gives Codex, Claude, scripts, and CI a common JSON interface for building game
assets. Generate characters, icons, and props; process existing media locally; or
reuse approved animation frames without generating new video. Deliver the results
as inspectable `.gsfpack` assets and native Godot resources.

[Latest release](https://github.com/MightyKartz/GameSpriteForge/releases/latest) ·
[CLI reference](docs/automation/forge-cli.md) ·
[Examples](examples/cli) ·
[Contributing](CONTRIBUTING.md)

## What Forge does

- **Consistent assets:** immutable Style Locks guide character, icon-set, and prop-set generation.
- **Local processing:** matting, frame normalization, loop selection, sprite sheets, and Pack validation.
- **Existing-animation reuse:** preserve approved source pixels, frame order, native durations, and provenance while assembling a separate directional review candidate.
- **Inspectable jobs:** durable jobs, structured reports, plan/execute operations, and targeted retries.
- **Godot delivery:** external PNG/atlas textures, native animation resources, installation ownership checks, and usage metadata.
- **Agent integration:** one JSON envelope on stdout, with diagnostics and interactive authentication on stderr/TTY.

Forge handles visual assets and engine delivery. Gameplay and game logic belong to
the project consuming those assets.

## Install

The published CLI release targets **macOS Apple Silicon**. Install Godot **4.6.x**
separately if you need engine delivery.

```bash
curl --proto '=https' --tlsv1.2 -LsSf https://raw.githubusercontent.com/MightyKartz/GameSpriteForge/main/install.sh | sh
```

Open a new terminal, then check the installation:

```bash
forge --version
forge doctor --json
```

The installer verifies SHA-256 manifests and installs `forge`, `ffmpeg`, and
`ffprobe` in a versioned user directory. Only `forge` is exposed on `PATH`.
The current published release is
[`v0.2.0-cli.1`](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.2.0-cli.1);
it is unsigned and not notarized. Changes on `main` may require a source build.

## Generate your first character

Use the [style](examples/cli/style.json) and [character](examples/cli/character.json)
specifications as starting points. Save your own copies and replace the absolute
spec paths below. API Key is the stable xAI authentication path; OAuth is Preview.
Provider-backed generation may incur charges. Review the plan and request bounds
before execution; see the [CLI reference](docs/automation/forge-cli.md).

```bash
forge provider login --provider xai --method api-key
forge project init --path "$PWD/game-assets" --name "My Game"

# Plan the Style Lock, then execute the returned token.
forge style create --project "$PWD/game-assets" \
  --spec /absolute/style.json --plan-only --json
forge plan execute --token STYLE_PLAN_TOKEN --wait --json

# Plan the character, then execute its separate token.
forge generate character --project "$PWD/game-assets" \
  --spec /absolute/character.json --plan-only --json
forge plan execute --token CHARACTER_PLAN_TOKEN --wait --json
```

Replace each token placeholder with the value from its plan response. Generation
normally runs as a durable asynchronous job; `--wait` waits for completion. Read the
job's artifacts to find the actual output Pack:

```bash
forge job report --id JOB_ID --json
forge pack validate --path /absolute/Character.gsfpack --json
```

For inventory assets, use `forge generate icon-set` or `forge generate prop-set`
with the corresponding [icon](examples/cli/icons.json) or
[prop](examples/cli/props.json) specification.

## Deliver to Godot

Use an existing Godot project and the Pack path returned by the completed job:

```bash
forge godot plan-install \
  --pack /absolute/Character.gsfpack \
  --project /absolute/my-godot-game \
  --asset-key my_character --json

forge plan execute --token INSTALL_PLAN_TOKEN --wait --json
```

Forge installs into `addons/forge_assets`, uses external textures, and tracks
Forge-owned output. Validate the installed resources and review the animation in
Godot before using it in your game.

## Reuse animation without new video

The source repository includes an experimental
[directional reuse workflow](docs/architecture/forge-existing-animation-reuse-plan.md)
(documentation in Chinese). It combines existing approved right/down/up recovery
Packs in a standalone Godot review project, preserves source frames and native
timing, and applies a fixed scale per direction. The installation plans must have
explicit zero Provider request bounds.

The helper requires Python 3.9+, Pillow, a current source-built Forge CLI, Godot
4.6.x, and the expected source approvals and geometry evidence. It is scoped to the
existing recovered three-direction corpus; the full source media is not bundled.

See the [integration checks](docs/qa/forge-directional-reuse-main-integration-2026-09-07.md)
and [human approval record](docs/qa/forge-directional-human-review-2026-09-07.md).
Approval is bound to that specific candidate. Each changed candidate needs its own
review; the helper does not synthesize missing left-facing or idle animations.

## Release and source-build scope

The [CLI feature definitions](packages/cli/Cargo.toml) are the source of truth.
The default build uses `default = []`.

| Surface | Availability |
| --- | --- |
| Style Locks, character/icon/prop generation, local asset preparation, jobs, Pack validation, Godot installation | Default CLI |
| Directional reuse helper and native per-frame timing/rendering support | Available on `main`; separate from the published CLI release |
| Subject Locks and Character consistency V2 | Opt-in `consistency-v2` source build |
| Environment, Terrain, Building, Map workflows | Opt-in world features; `world-assets` enables the group |
| Manifest-driven project diff and build planning | Opt-in `game-art-manifest` source build |

Optional features and fixture checks do not establish real-provider or visual
acceptance. The CLI and Rust workspace are the current product focus; retained
desktop/MCP code is outside the default release surface.

## Development

```bash
cargo build -p forge-cli
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/character/test_prepare_directional_reuse.py -v
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup and applicable checks, the
[quality workflow](.github/workflows/v03-quality.yml) for CI coverage, and the
[QA artifact policy](docs/qa/forge-qa-artifact-policy.md) for evidence handling.
Keep credentials and temporary media URLs out of commits. Provider output should
be materialized and hashed before processing, and source Jobs/Packs remain
traceable across retries and derived outputs.

## License

[MIT](LICENSE). Bundled FFmpeg helpers have separate LGPL notices and corresponding
source distributions; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
