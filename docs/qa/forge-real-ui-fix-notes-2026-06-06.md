# Forge Real UI Issue Fix Notes - 2026-06-06

## Fixed in this pass

- Replaced `convertFileSrc` preview rendering with a scoped `read_preview_image` Tauri command that returns data URLs for generated preview images.
- Updated canvas preview, frame timeline thumbnails, and exported sprite-sheet preview to use `usePreviewImage`.
- Added recent-export card actions for Inspect, Validate Pack, Re-import Pack, and Open.
- Split quality state into blocked, all-passed, and exportable-with-suggestions states.
- Added Sheet and Export workflow focus panels.
- Localized the export metadata Show/Hide disclosure labels through i18n instead of CSS literals.
- Expanded Settings into a two-column local runtime layout.
- Added `npm run install:mac` and `scripts/install-local-mac-app.sh` to keep `/Applications/Game Sprite Forge.app` synced with the latest local build.

## Verification

- `node scripts/test-real-ui-issue-fixes-source.mjs`
- `npm run test:scripts`
- `npm run build`
- `cargo test`
- `npm --workspace apps/mac run tauri -- build --bundles app`
- `scripts/install-local-mac-app.sh`
- `codesign --verify --deep --strict --verbose=2 /Applications/Game Sprite Forge.app`

## Remaining manual note

Computer Use returned only `remoteConnection` for this desktop session, and macOS screenshots of the Tauri window were not reliable enough to complete a visual click-through. The installed app is signed and synced, but Gatekeeper assessment still reports `Unnotarized Developer ID` until notarization credentials are exported for the build shell.
