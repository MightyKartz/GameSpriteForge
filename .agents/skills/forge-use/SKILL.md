---
name: forge-use
description: Use Forge CLI to prepare existing game art, generate icon or prop sets, inspect asset jobs, and deliver Packs to Godot. Includes Codex image generation followed by local Forge processing.
---

# Use Forge for game assets

Use the CLI for production work. Start from the game's asset specs, toolchain lock,
and import receipts when they exist. Forge source development belongs to `forge-dev`.

## Select the actual toolchain

Record the absolute CLI path, `--version`, and `doctor --json`. For development
builds, also check the pinned source commit and binary SHA-256: different binaries
currently report `0.2.1` while accepting different request fields. Do not replace
a game's pinned executable with a convenient `forge` on PATH.

- Published v0.2.1 supports the existing local animation, Pack and Godot workflows.
- `plan prepare-static` is included in the current default source build. The
  published v0.2.1 predates it; use a verified source binary until the new release.
- Local `preserve_source`, request-level rendering/timing, and whole-sheet
  padding/offset are also included in the current source build. Published v0.2.1
  does not accept those local request forms. Animation remains experimental.

Check `plan --help` for commands. Request fields also need a known source revision
and a successful plan; the presence of `prepare-asset` alone does not establish
support. See the [local asset guide](../../../docs/automation/codex-local-assets.md)
for contracts and availability. If a required build is unavailable, report that
specific gap rather than dropping fields or claiming a stable upgrade supplies it.

## Choose a workflow

- **Codex image generation + local Forge:** use the available image-generation
  tool for new artwork or creative edits. Inspect its output, retain the original
  PNG and source hash, and pass local files to Forge. This is an external source
  workflow, not a Forge OpenAI Provider. It needs no Forge Provider login or Style
  Lock; zero Forge Provider requests does not mean image generation has no usage.
- **Existing transparent static items:** use `plan prepare-static` for an
  `icon_set` or `prop_set`. Each item has its own source path and stable ID.
  Unrelated icons/props are not consecutive animation frames.
- **Existing animation:** use `plan prepare-asset` for one action or
  `plan prepare-character` for multiple actions. With a supported build, use
  `preserve_source` when source frames already share intentional coordinates.
  Do not independently crop/recenter such frames; that can erase intended motion.
- **Forge Provider generation:** follow the [CLI guide](../../../docs/automation/forge-cli.md)
  and [example specs](../../../examples/cli) for Style → icon/prop/character
  generation. Character animation remains experimental. Reusing a local PNG does
  not require creating a paid Style Lock or invoking `generate`.

## Prepare, inspect, and deliver

Use dedicated `FORGE_JOB_STORE` and `FORGE_PLAN_STORE` locations. Plan first and
inspect request bounds; a local processing plan should estimate zero Provider
requests. Execute its token once with `--wait`. Check both the JSON envelope and
the Job lifecycle: an accepted command or existing Pack is not sufficient evidence
that this job succeeded.

For static art, choose `linear` for painted edges or `nearest` for pixel art.
`foregroundAlphaThreshold` selects crop bounds, not an alpha cutoff;
`edgePaddingPx` retains surrounding source pixels. The Sword values `16/16` are
one tested choice, not a universal preset. Inspect the normalized result.

For preserved animation, all actions need the same frame canvas and common anchor;
margins must be zero. Fractional anchors need `pixelSnap:false`. Whole-sheet
padding/offset is explicit and must not discard any nontransparent pixels. Retain
the `source_transform` artifact when used. Use native Godot resources to review
nonuniform frame timing; the preview GIF still uses uniform FPS.

Read `job report`, find the Pack in Job artifacts, run `pack validate`, then prepare
and execute a Godot install plan. Supply stable `--asset-key` **and** `--target`
(`addons/forge_assets/<stable-id>`); the display name should not determine ownership.
Consume paths, IDs, anchors and sampling from `forge_usage.json`. Icon textures
need the consuming node to apply the declared filter; installed prop scenes apply
it directly. Keep game-specific
scale, behavior and offsets in wrapper scenes instead of editing Forge-owned files.

Retain source/normalized hashes, request, CLI identity, Job IDs, Pack hash, installed
usage and relevant processing evidence in an import receipt. Check resources in
Godot at gameplay scale. Local static `game_ready` is a structural result, not art review;
`prototype_usable` stays a prototype. Do not disable a quality gate or submit a
human acceptance on behalf of an unperformed review. If the user explicitly wants
a prototype, record that intent and its remaining review status separately.

Automatic repair preserves `preserve_source` coordinates: canvas/anchor changes
are returned as manual actions. A repair plan may still apply other supported
corrections; inspect its changes and remaining manual actions before execution.

To revise a local static set, prepare a new request; the initial local importer has
no targeted retry or Forge asset-project catalog integration. Provider-generated
static assets have different retry behavior. Preserve that distinction.
