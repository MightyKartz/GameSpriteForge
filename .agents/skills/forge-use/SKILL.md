---
name: forge-use
description: Use Forge CLI to prepare existing game art, generate icon or prop sets, inspect asset jobs, and deliver Packs to Godot. Includes Codex image generation followed by local Forge processing.
---

# Use Forge for game assets

Use the CLI for production work. Start from the game's asset specs, toolchain lock,
and import receipts when they exist. Forge source development belongs to `forge-dev`.

## Select the actual toolchain

Use v0.3.0 for the stable local PNG → static Pack → Godot workflow. Record the
absolute CLI path, `--version`, `doctor --json` and binary SHA-256. Development
builds can share a version string while accepting different request fields, so
also check their pinned source commit. Do not replace a game's pinned executable
with a convenient `forge` on PATH.

v0.3.0 adds `doctor.data.build` and `doctor.data.capabilities`. Use those compiled
fields to check the expected commit, source state and required capabilities; an
unknown Git identity is `null`, not a clean-build claim. Continue verifying the
consumer's binary SHA-256. Runtime tool availability is reported separately.

- v0.3.0 includes `plan prepare-static`, Pack validation and Godot installation
  in the default CLI; no source build or optional feature is needed for this path.
- The earlier v0.2.1 supports existing local animation, Pack and Godot workflows,
  but predates `prepare-static` and the new local animation request fields.
- Local `preserve_source`, request-level rendering/timing, and whole-sheet
  padding/offset are included in v0.3.0. Character animation remains in development
  and testing; availability is not production or visual approval.

Check `plan --help` for commands. Request fields also need a known source revision
and a successful plan; the presence of `prepare-asset` alone does not establish
support. See the [local asset guide](../../../docs/automation/codex-local-assets.md)
for contracts and availability. If a required build is unavailable, report that
specific gap rather than dropping fields. When upgrading a pinned consumer,
verify its required contracts before updating its lock for future imports. Keep
existing asset receipts and source history unchanged; create new receipts only
for actual re-imports.

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
`edgePaddingPx` retains surrounding source pixels. Start with their defaults,
`1/0`, and adjust after source inspection. Inspect the normalized result.

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
