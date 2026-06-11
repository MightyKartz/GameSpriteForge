# Transparent-Gutter Sprite Sheet Real UI QA

Date: 2026-06-11

Scope: real macOS UI test for the transparent-gutter sprite sheet intake path in Forge.

## Environment

- App under test: `/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app`
- Build command: `npm --workspace apps/mac run tauri -- build --debug --bundles app`
- UI driver: Computer Use against the real Tauri/macOS app window and native Open panel
- Fixture: `docs/qa/artifacts/forge-real-ui-transparent-gutter-sheet-fixture-2026-06-11.png`
- Evidence screenshot: `docs/qa/artifacts/forge-real-ui-transparent-sprite-processed-2026-06-11.png`

## Real UI Path Tested

1. Launched the debug macOS app bundle from the workspace.
2. In the empty workbench, clicked `导入精灵表`.
3. Used the native macOS Open panel to select a 23x11 PNG sprite sheet with transparent gutters.
4. Confirmed the selected sprite sheet surfaced in the central import launcher.
5. Switched the split mode from `固定网格` to `透明间隔`.
6. Clicked `导入所选精灵表`.
7. Confirmed the app moved to `帧` workflow, created a live workspace, and reported 6 original frames.
8. Ran `处理并检查质量`; the quality step completed and produced a blocked quality report for the intentionally tiny/color-shifting test fixture.

## Findings

### F-001: QA environment can target stale installed app

Status: documented

When `/Applications/Game Sprite Forge.app` was still running with the same bundle id as the workspace app, Computer Use initially attached to the installed app instead of the current debug bundle. This was not a product bug, but it can invalidate real UI test evidence.

Mitigation used: closed stale app instances, rebuilt the debug bundle, and launched `/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app` explicitly.

### F-002: Selected sprite sheet next step was visually hard to find

Status: fixed

Before the fix, choosing a sprite sheet from the central launcher filled the side import form, but the central launcher still looked like the original video import card. The key next action, `导入所选`, existed in the side form/accessibility tree but was not obvious from the primary viewport.

Fix:

- Added a central `配置精灵表导入` state after sprite sheet selection.
- Shows the selected file name, split mode buttons, grid/transparent parameters, and `导入所选精灵表` in the main launcher.
- Kept `更换精灵表` and alternate source actions available.
- Added source-level guard coverage so the central sprite sheet confirmation path is not accidentally removed.

Files changed:

- `apps/mac/src/routes/ForgeRoute.tsx`
- `apps/mac/src/i18n.ts`
- `apps/mac/src/styles/app.css`
- `scripts/test-import-panel-source.mjs`

### F-003: Toy fixture blocks export after quality check

Status: expected behavior

The tiny six-frame test PNG intentionally uses very small, color-shifting regions. After processing, the quality report blocks export with loop/background recommendations. This confirms the quality gate is active; it is not a transparent-gutter import failure.

## Verification

Passed:

- `npm --workspace apps/mac run build`
- `npm --workspace apps/mac run tauri -- build --debug --bundles app`
- real UI import regression via Computer Use
- `cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check`
- `cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent`
- `npm run test:scripts`
- `npm --workspace apps/mac run smoke:ui:mvp`

Result: transparent-gutter sprite sheet intake works in the real macOS UI, and the discovered central-launcher UX issue has been fixed and guarded.
