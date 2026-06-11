# Forge Formal Empty Workbench Evidence - 2026-06-07

## Scope

Formal app first-run behavior now treats the no-source state as an empty local workbench instead of a bundled sample workspace.

## Implemented

- Default source pill and summary now use empty-workspace copy.
- Default canvas shows a neutral empty placeholder, not sample preview art.
- Default asset rail shows an empty assets state, not sample assets.
- Default frame timeline shows a no-frames empty state, not sample tracks.
- Sprite sheet preview no longer shows a sample badge before export.
- Default pack metadata starts as `Untitled Sprite Pack` / `idle`; `Green Box Character Pack` / `walk` is only applied after an explicit sample action.
- The bundled sample pipeline remains available through explicit actions only:
  - First Run: Run Sample Pipeline
  - Import Sources: Load Sample Path
  - Quality Report: Run sample pipeline

## Verification

- `npm run test:scripts` passed.
- `npm --workspace apps/mac run build` passed.
- `npm --workspace apps/mac run smoke:ui` passed.
- `FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui` passed.
- `npm run install:mac` passed and installed `/Applications/Game Sprite Forge.app`.
- `codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"` passed.

## Smoke Artifacts

- English screenshot: `/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-1568-en-US.png`
- Chinese screenshot: `/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-1568-zh-CN.png`
- English visible text: `/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-visible-text-en-US.txt`
- Chinese visible text: `/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-visible-text-zh-CN.txt`

## Guardrail

The zh-CN/en-US visible-text dumps were checked for these default-state leftovers and none were found:

- `示例预览`
- `示例资源`
- `示例时间线`
- `Sample preview`
- `Sample Assets`
- `Sample timeline`
- `Demo preview`
- `Demo workspace`
