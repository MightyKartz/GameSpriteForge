# Forge Real UI Full QA Post-Fix Notes

Date: 2026-06-07 CST

## Fixed

- Settings and Exports routes now expose main landmarks, focus entry, labels, and exact owner screenshot capture support.
- Import state separates selected source from live frames, so sample 64-frame placeholders are not treated as an active Green Box workspace.
- Export and re-import validation state are separated; exported-but-unvalidated copy stays pending until validation succeeds.
- Advanced export metadata remains user-controlled instead of auto-opening during the main flow.
- First-run sample story now uses Green Box placeholders and recent export re-import context.
- Release build scripts no longer inject the failed launcher workaround.
- Release HTML is inlined after `#root` to avoid local WKWebView external-script execution problems.

## Verification

- `npm run test:scripts`
- `npm --workspace apps/mac run build`
- `npm --workspace apps/mac run smoke:ui`
- `FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui`
- `npm run install:mac`
- `codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"`

## Remaining Concern

`/Applications/Game Sprite Forge.app` is installed and signed, but the local automation stack still cannot reliably inspect the real Tauri window:

- Computer Use returned `remoteConnection`.
- Exact-owner `screencapture -l` captured a blank window.
- A temporary diagnostic run before cleanup showed the WebView DOM did mount: `rootChildren=1`, `textLength=945`, and root/app-shell geometry matched the visible viewport.

This means the product source and browser smoke path are verified, but final installed-App visual verification still needs a human visual pass or a different macOS WebView inspection method.

## Notarization

Gatekeeper assessment still reports `Unnotarized Developer ID` because notarization credentials were not present in the build environment for this run. Code signing itself verified successfully.
