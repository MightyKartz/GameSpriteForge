# Forge Real UI Follow-up Fix Notes - 2026-06-06

## Fixed in this pass

- Kept the Forge workbench mounted while visiting Settings or Exports so route changes and locale re-renders do not reset the active workspace.
- Disabled Forge keyboard shortcuts while the workbench route is hidden.
- Used the imported `.gsfpack` manifest name as the active pack name while keeping the file basename as source identity.
- Replaced the hardcoded `v1.2.0` footer with `APP_VERSION`, sourced from the root package version at build time.
- Added app version to Settings and removed developer-facing `npm run install:mac` copy from product UI.
- Made the top header route-aware: Settings and Exports no longer show the workbench workflow tabs.
- Reworked the local pack library source to use stable newest-first sorting and a selected pack detail panel after library refresh.
- Changed preview alt text from knight-specific to generic selected sprite frame copy.
- Hid collapsed export metadata controls from keyboard and accessibility trees.
- Changed the visible workbench subtitle from internal MVP wording to user-facing local import workbench copy.
- Aligned the default pack name with the bundled Green Box sample.

## Automated verification

```text
node --check scripts/test-real-ui-followup-fixes-source.mjs
node scripts/test-real-ui-followup-fixes-source.mjs
npm --workspace apps/mac run build
npm run test:scripts
npm run install:mac
```

## Real UI verification

Tested installed app:

```text
/Applications/Game Sprite Forge.app
```

Passed:

- Initial installed UI shows `本地导入工作台` instead of `仅导入 MVP`.
- Initial export pack name defaults to `Green Box Character Pack`.
- Main preview accessibility label is generic: `选中的精灵帧`.
- Collapsed export metadata no longer exposes hidden sheet steppers in the accessibility tree.
- Footer version shows `v0.1.0`.
- `运行示例流程` completes and produces a live `Green Box Character Pack` workspace with 24 frames, exported sheet preview, export output, and validation result.
- Visiting Settings shows route-aware `页面状态 / 设置`, no workbench workflow tabs, app version `v0.1.0`, and no developer install command.
- Returning from Settings to Workbench preserves the live 24-frame workspace instead of resetting to the sample/no-source state.
- Exports route shows route-aware `页面状态 / 资源包库`.

Evidence screenshots:

```text
docs/qa/screenshots/forge-real-ui-followup-library-2026-06-06.png
docs/qa/screenshots/forge-real-ui-followup-library-refreshed-2026-06-06.png
```

## Remaining verification note

Computer Use became stuck reporting the macOS Window menu accessibility tree after a mis-click during export-library refresh verification. The installed app UI remained usable visually, but the refresh-to-detail-panel interaction was not conclusively verified through Computer Use in this pass. The source regression test covers the detail panel and stable sorting implementation.

## Distribution note

`codesign --verify --deep --strict --verbose=2 /Applications/Game Sprite Forge.app` passed.

`spctl --assess --type execute --verbose=4 /Applications/Game Sprite Forge.app` still reports `source=Unnotarized Developer ID` for the freshly installed local build because `npm run install:mac` built without notarization environment variables in this shell.
