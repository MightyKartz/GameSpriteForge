# Forge automation request schemas

## Prepare a PNG sequence

```json
{
  "schemaVersion": "1",
  "input": {
    "kind": "png_sequence",
    "paths": ["/absolute/frame_001.png", "/absolute/frame_002.png"]
  },
  "metadata": {
    "name": "Knight Idle",
    "animation": "idle",
    "fps": 12,
    "loop": true,
    "creator": "Game Sprite Forge",
    "license": "private"
  },
  "matting": { "mode": "preserve_alpha" }
}
```

## Prepare a fixed-grid sprite sheet

```json
{
  "schemaVersion": "1",
  "input": {
    "kind": "sprite_sheet",
    "path": "/absolute/knight.png",
    "split": {
      "mode": "fixed_grid",
      "frameWidth": 64,
      "frameHeight": 64,
      "columns": 4,
      "rows": 2
    }
  },
  "metadata": { "name": "Knight Walk", "animation": "walk" }
}
```

For transparent gutters, replace `split` with:

```json
{ "mode": "transparent_gutters", "alpha_threshold": 0, "min_gap_px": 1 }
```

## Reuse a `.gsfpack`

```json
{
  "schemaVersion": "1",
  "input": { "kind": "gsfpack", "path": "/absolute/Knight.gsfpack" },
  "metadata": { "name": "Knight" }
}
```

## Prepare an exact video clip

`video_clip` works in schema V1 and as an animation input in schema V2. Bounds are integer milliseconds and `targetFrameCount` must be 2 through 24.

```json
{
  "schemaVersion": "1",
  "input": {
    "kind": "video_clip",
    "path": "/absolute/knight-walk.mp4",
    "startTimeMs": 250,
    "endTimeMs": 1750,
    "targetFrameCount": 8
  },
  "metadata": { "name": "Knight Walk", "animation": "walk", "fps": 12 }
}
```

## Prepare a versioned Character Workflow (automation schema V2)

Call `list_character_workflows` first. Copy its exact `id` and `version`; use the returned FPS and loop values as defaults unless the user or source timing deliberately overrides them. The example below selects the Platformer contract and therefore includes `idle`, `walk`, and `jump`.

```json
{
  "schemaVersion": "2",
  "metadata": {
    "name": "Knight",
    "defaultAnimation": "idle",
    "creator": "Game Sprite Forge",
    "license": "private"
  },
  "workflow": {
    "id": "platformer",
    "version": "1.0.0"
  },
  "animations": [
    {
      "name": "idle",
      "input": {
        "kind": "png_sequence",
        "paths": ["/absolute/idle_01.png", "/absolute/idle_02.png"]
      },
      "fps": 8,
      "loop": true,
      "matting": { "mode": "preserve_alpha" }
    },
    {
      "name": "walk",
      "input": {
        "kind": "sprite_sheet",
        "path": "/absolute/walk.png",
        "split": {
          "mode": "fixed_grid",
          "frameWidth": 64,
          "frameHeight": 64,
          "columns": 4,
          "rows": 1
        }
      },
      "fps": 12,
      "loop": true,
      "matting": { "mode": "preserve_alpha" }
    },
    {
      "name": "jump",
      "input": {
        "kind": "png_sequence",
        "paths": ["/absolute/jump_01.png", "/absolute/jump_02.png"]
      },
      "fps": 12,
      "loop": false,
      "matting": { "mode": "preserve_alpha" }
    }
  ],
  "quality": { "requireGameReady": true }
}
```

Call `plan_prepare_character_pack` with this request. Forge validates the selected workflow and required clips, processes each animation independently, normalizes all frames against one shared canvas and anchor, then exports one flat atlas with global frame indexes. Use `{ "id": "custom", "version": "1.0.0" }` only for a genuinely free-form contract. Existing `.gsfpack` inputs are not valid animation sources in Character Pack V1. Continue to Godot installation only from a `succeeded` prepare job: take the exact artifact path whose kind is `gsfpack`, validate it with `inspect_asset`, and use that path as `packPath`.

## Generate a top-down Character Pack (automation schema V3)

Call `list_providers` and `check_provider` first. OAuth login is deliberately unavailable through MCP; when xAI is unauthenticated, the user runs `forge-cli provider login --provider xai --method oauth` in a terminal. Use `fixture` for deterministic offline tests.

```json
{
  "schemaVersion": "3",
  "providerId": "xai",
  "profileId": "default",
  "character": {
    "prompt": "A compact forest ranger with a green hood and a short bow"
  },
  "metadata": {
    "name": "Forest Ranger",
    "defaultAnimation": "idle",
    "creator": "Game Sprite Forge",
    "license": "private"
  },
  "workflow": { "id": "topdown", "version": "1.0.0" },
  "generation": {
    "maxAttemptsPerAnimation": 2,
    "targetFrameCount": 8,
    "videoDurationSeconds": 4
  },
  "quality": { "requireGameReady": true }
}
```

An optional absolute PNG can be supplied as `character.referenceImagePath`. Optional `generation.imageModel` and `generation.videoModel` pin explicit model IDs; omit them to use the Provider Profile defaults. Forge locks the job to the one selected Provider, generates `idle`, `walk_up`, `walk_right`, and `walk_down`, and reuses `walk_right` with horizontal flip for left playback. It never silently switches Provider or model. Each action receives at most two attempts across Provider errors and quality regeneration. The resulting `.gsfpack` is exported only when every required action reaches `game_ready`.

## Install into an existing Godot project

```json
{
  "schemaVersion": "1",
  "packPath": "/absolute/Knight.gsfpack",
  "projectPath": "/absolute/my-godot-game",
  "target": "addons/forge_assets/knight",
  "assetKey": "knight",
  "providerRefs": [
    {
      "provider": "spritecook",
      "assetId": "spritecook-character-id",
      "label": "Knight source"
    }
  ]
}
```

`target` is project-relative. `assetKey` is a stable engine-safe identity; when omitted it is derived from the target folder name. `providerRefs` is optional provenance and must never contain credentials or secret URLs. Include only real provider IDs; if none are known, omit the field and report that provenance was not recorded rather than inferring an ID from a filename.

Forge writes textures, `forge_sprite_frames.tres`, `forge_animated_sprite.tscn`, `.forge-owned.json`, and `forge_usage.json` below the target. It atomically registers the asset in `<project>/.forge/assets.json`. Existing non-Forge-owned directories are never replaced. Call `inspect_project` before and after installation to discover the current asset/revision map.

## Repair an awaiting-review job

Repair tools take the durable job ID rather than a replacement request:

```json
{ "id": "<awaiting-review-job-id>" }
```

Call `analyze_repair` first. It returns the baseline quality evidence, `changes` with explicit before/after values, `manualActions`, `attempt`, and `canAutoRepair`. Only when the safe changes are acceptable, call `plan_repair_job` with the same ID, review the returned effects, and execute its token normally.

The new job contains a `repair` context linking it to the source and emits `repair-comparison.json`. Read that local JSON artifact path directly, then verify the affected animation's verdict, alpha coverage, notes, recommendations, and preview; `improved: true` alone is not proof that foreground recovery succeeded. Automatic changes are limited to processing parameters; loop ranges, frame selection, background-mode selection, and unresolved anchor decisions remain manual. A repair chain is capped at three attempts. A manually rebuilt V1/V2 request starts an unlinked job in Repair V1, so retain and report its originating job ID separately.
