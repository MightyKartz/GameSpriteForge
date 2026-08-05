---
name: forge-assets
description: Generate, prepare, inspect, validate, repair, install, and track Game Sprite Forge 2D assets through the local Forge MCP. Use for model-independent xAI or fixture generation, single animations, versioned Platformer/Top-down/Isometric/Custom Character Workflows, quality-gated awaiting-review jobs, video clips, PNG sequences, sprite sheets, existing .gsfpack directories, durable Forge jobs, Godot 4 AnimatedSprite2D installation, or inspection of a Godot project's .forge/assets.json asset manifest.
---

# Forge Assets

## Overview

Turn supported 2D image inputs into validated `.gsfpack` directories, then install neutral `SpriteFrames` and `AnimatedSprite2D` resources into an existing Godot 4 project. Forge records each installed asset in `.forge/assets.json` and writes a colocated `forge_usage.json` so later Codex tasks can rediscover the exact scene, animations, revision, and source provenance. All mutations use a two-stage plan token and durable job records.

## Workflow

1. Call `check_environment` before the first mutation in a task. If `forge-cli` is unavailable, stop and read [install-and-troubleshoot.md](references/install-and-troubleshoot.md).
2. Choose the workflow:
   - One animation or an existing `.gsfpack`: use `plan_prepare_asset` with schema V1.
   - A gameplay-ready character: call `list_character_workflows`, choose the closest versioned contract, and use `plan_prepare_character_pack` with schema V2. Include every required animation exactly once. Treat returned FPS and loop values as recommended defaults; preserve them unless the user or source timing supplies a deliberate override.
   - A Provider-generated top-down character: call `list_providers`, then `check_provider`. Use `plan_generate_character_pack` with schema V3 and exactly one `providerId`. If xAI is unauthenticated, ask the user to run `forge-cli provider login --provider xai --method oauth`; never attempt login through MCP or request a credential.
3. For each raw input, select a supported source:
   - `png_sequence`: two or more absolute PNG paths in playback order.
   - `sprite_sheet`: one absolute PNG plus `fixed_grid` dimensions or `transparent_gutters` settings.
   - `gsfpack`: one absolute `.gsfpack` directory.
   - `video_clip`: one absolute video path with an integer millisecond range and exact target frame count.
4. Inspect and summarize the plan's effects, expiry, input fingerprint, and recipe hash.
5. Call `execute_plan` only when the user's request already authorizes the described writes. Otherwise ask for confirmation and retain the plan token only until its expiry.
6. Poll `get_job`. Treat `awaiting_review` as a quality decision point, not an infrastructure failure.
7. When a prepare job reaches `awaiting_review`, use the executable repair loop:
   - Call `analyze_repair` and present every proposed before/after parameter change plus `manualActions`.
   - If `canAutoRepair` is true and the task authorizes those writes, call `plan_repair_job`, summarize its effects, then `execute_plan` and poll the new linked job.
   - Read the local JSON file at the new job's `repair_comparison` artifact path. Do not treat `improved: true` as sufficient proof: compare the affected animation's verdict, alpha coverage, notes, recommendations, and preview, and check that unaffected animations did not regress. Repeat only when it remains `awaiting_review` and Forge offers another safe attempt; Forge caps the chain at three attempts.
   - If no safe change exists, call `open_job`. Apply the human review decision to a revised schema V1/V2 request, then prepare a new plan manually. Repair V1 does not link this manual replacement job automatically, so include the source job ID in the handoff/report for auditability.
   - Never install into Godot until a prepare or repair job reaches `succeeded` and `inspect_asset` passes.
8. After a prepare job reaches `succeeded`, read the artifact whose kind is `gsfpack`; use that exact absolute path with `inspect_asset`. Never infer `packPath` from the job directory.
9. Before installing into Godot, call `inspect_project` to understand existing Forge-owned assets and avoid accidental identity collisions.
10. Use `plan_install_godot` with a stable `assetKey`. Include `providerRefs` when the source came from SpriteCook or another provider. Execute and poll with the same plan sequence.
11. After success, call `inspect_project` again. Report the registered asset key, revision, generated scene, `forge_usage.json`, and `.forge/assets.json` paths.

## Safety Contract

- Plan tokens expire after 15 minutes, are single-use, and recheck input fingerprints at execution.
- Never invent or shorten paths. Use absolute input, pack, and Godot project paths.
- Godot targets must remain project-relative and cannot contain `..`.
- Forge only replaces target directories containing `.forge-owned.json`; a non-owned directory is a hard stop.
- Do not bypass `requireGameReady` silently. Open the job in Forge when quality is below `game_ready`.
- Video is an automation input. Keep clip bounds in integer milliseconds and `targetFrameCount` between 2 and 24.
- A schema V3 job is locked to one Provider. Never silently switch Provider or model after planning, including on rate-limit or entitlement errors.
- Never call OAuth login from an agent, read a system credential store, return tokens, or place credentials, device codes, temporary signed URLs, or authorization headers in a request, job, pack, Provider reference, log, or response.
- Generated V3 packs currently require `topdown@1.0.0`, `idle` as the default, and Forge-managed `idle`, `walk_up`, `walk_right`, and `walk_down`. Left is the horizontally flipped right animation.
- Character Pack animation names must be unique and engine-safe. All animations share one normalized canvas and foot anchor.
- A selected Character Workflow's `id` and `version` must match `list_character_workflows`; do not invent workflow versions or omit required clips.
- Non-looping actions such as `attack` and `death` should set `loop: false`; do not force loop cleanup advice onto them.
- Character Pack feature V1 uses automation schema V2 and accepts PNG sequences and sprite sheets. Merging animations out of existing `.gsfpack` files is not yet supported.
- Treat `assetKey` as a stable project identity, not a display label. Reinstalling changed content under the same key increments its manifest revision; reinstalling unchanged content preserves it.
- Provider references are optional identifiers only. Include real provider IDs when known; otherwise omit them and disclose that provenance was not recorded. Never infer IDs from filenames or place provider tokens, signed URLs, prompts containing secrets, or credentials in `providerRefs`.
- Repair jobs never overwrite their source job. Automatic repair may adjust only reversible processing parameters such as chroma threshold, canvas margin, or normalization mode.
- Treat loop trimming, shorter clips, animation semantics, background-mode selection, and persistent anchor drift as manual decisions. Do not manufacture an ad hoc patched request when `analyze_repair` marks them manual.
- Respect the three-attempt repair cap. After that, open the job and ask for human direction instead of cycling.
- The comparison score is a routing hint, not visual proof. Verify the affected report fields and preview before claiming that foreground or animation quality improved.

## Request Shapes

Read [request-schema.md](references/request-schema.md) when constructing plan requests. Preserve `schemaVersion: "1"` for single assets and Godot installation, `schemaVersion: "2"` for prepared Character Packs, and `schemaVersion: "3"` for Provider-generated top-down Character Packs. Use the exact workflow version returned by `list_character_workflows`.

## Job Interpretation

- `queued` / `running`: poll; offer cancellation only if the user changes intent.
- `succeeded`: report `.gsfpack` and preview artifacts, or for Godot jobs report the scene, usage contract, and project manifest artifacts.
- `awaiting_review`: call `analyze_repair`; execute only reviewed safe changes through `plan_repair_job`, otherwise call `open_job` and rebuild the request from the human decision.
- `failed`: report `error_code`, `error_summary`, whether it is recoverable, and the provided `next_actions`.
- `cancelled`: do not retry without preparing a new plan.

## Boundaries

The MCP is a thin wrapper over `forge-cli`; do not duplicate image-processing logic in prompts or ad hoc scripts. The Forge macOS app is the review/recovery surface, while CLI and MCP remain independent developer tools and are not bundled into the app DMG.
