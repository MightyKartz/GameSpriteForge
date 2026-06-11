# Forge Real UI QA Follow-up Issue List

Date: 2026-06-06 21:18 CST

Status: DONE_WITH_CONCERNS

Scope: real macOS UI operation through Computer Use on the installed local app. This pass validates the post-fix installed build rather than the earlier stale build noted in `docs/qa/forge-real-ui-issue-list-2026-06-06.md`.

## Tested App

```text
/Applications/Game Sprite Forge.app
Bundle ID: dev.gamespriteforge.desktop
Window: Game Sprite Forge
System language path tested: Automatic -> English -> Automatic
```

Version cross-check:

```text
package.json: 0.1.0
apps/mac/src-tauri/tauri.conf.json: 0.1.0
UI footer: v1.2.0
Local pack library cards: v0.1.0
```

## Evidence

```text
docs/qa/screenshots/forge-real-ui-sample-complete-2026-06-06.png
docs/qa/screenshots/forge-real-ui-latest-exports-2026-06-06.png
docs/qa/screenshots/forge-real-ui-settings-language-2026-06-06.png
docs/qa/screenshots/forge-real-ui-workbench-reset-after-language-2026-06-06.png
```

## Passed In This Pass

- The installed app launches as a real macOS app and exposes a usable accessibility tree.
- `运行示例流程` completes the bundled sample flow: import, process, quality, export, and validate re-import.
- The processed frame preview, timeline thumbnails, and exported sprite-sheet preview now render visible images instead of broken image placeholders.
- Workflow tabs unlock after the sample flow.
- The left `导出` route lists local `.gsfpack` exports and exposes `检查`, `验证资源包`, `重新导入资源包`, and `打开`.
- `检查` and `验证资源包` both return success messages in the resource library.
- `重新导入资源包` returns to the workbench with 24 frames available.
- `处理并检查质量`, `导出资源包`, and `验证重新导入` work after a resource-library re-import.
- Settings includes a language selector with `跟随系统`, `English`, and `简体中文`.
- Switching to English updates the visible UI; switching back to Automatic returns the visible UI to Simplified Chinese on this machine.

## Issues

### P1: Settings/Language Round-trip Resets The Active Workspace

Repro:

1. Run `运行示例流程`.
2. Re-import or export a pack so the workbench is in a live 24-frame state.
3. Open `设置`.
4. Change language to `English`.
5. Change language back to `Automatic`.
6. Click `工作台`.

Observed:

The workbench returns to the sample/no-source state. The top workflow still has `帧` selected, but the tab is disabled and the quality state is back to pending. The previous live pack, processed frames, exported sheet, and validation status are no longer visible.

Expected:

Changing language and visiting Settings should preserve the current workspace session. If a full UI reload is unavoidable, the app should restore the active workspace or show a clear "reload will reset current work" confirmation before switching language.

Impact:

Users can lose their current in-app working context while changing a display preference. The selected disabled workflow tab also makes the next step unclear.

Suggested fix:

Decouple `LocalSettings.languageMode` changes from the transient workspace model, or persist/rehydrate the active `ForgeRoute` state across route and locale changes. Add a regression smoke that runs sample, changes language, returns to workbench, and asserts the active frame count and quality/export state are still present.

### P1: Resource Pack Identity Becomes Inconsistent After Library Re-import

Repro:

1. Open `导出`.
2. Click `刷新资源包库`.
3. Choose a `Green Box Character Pack` row.
4. Click `重新导入资源包`.
5. Process and export from the workbench.

Observed:

The resource library row is `Green Box Character Pack`, but the workbench current-source card shows `Hero Knight Pack` with `Green-Box-Character-Pack.gsfpack`. The footer can say `已导入 Green Box Character Pack`, while the export action produces `Hero-Knight-Pack.gsfpack`.

Expected:

The library row title, imported pack metadata, current-source card, export form default name, footer status, and generated file name should agree, or the UI should explicitly distinguish "source file name" from "pack display name" and "new export name".

Impact:

Users cannot confidently tell which pack they re-imported or what name the next export will use.

Suggested fix:

Normalize the pack identity contract used by `ExportsRoute` and `ForgeRoute`. On re-import, preserve the pack manifest display name as the primary name and show the file basename as secondary metadata. If the user renames the pack for a new export, make that an explicit editable field state.

### P1: App Version Display Is Internally Inconsistent

Repro:

1. Launch the installed app.
2. Run the sample or open the workbench.
3. Compare the footer version with package metadata and resource-library card versions.

Observed:

The footer displays `v1.2.0`. `package.json` and `apps/mac/src-tauri/tauri.conf.json` are `0.1.0`, and the local pack library displays `v0.1.0`. Source reference: `apps/mac/src/routes/ForgeRoute.tsx:1109` hardcodes `v1.2.0`.

Expected:

All visible version surfaces should derive from one build/version source.

Impact:

QA, notarization, support screenshots, and release notes can refer to the wrong build.

Suggested fix:

Expose the Tauri/package version through a build-time constant or Tauri command, then render the same value in the footer, Settings/About, and pack metadata where appropriate.

### P2: Export Library Ordering Changes After Refresh

Repro:

1. Click `导出` after a fresh sample run.
2. Observe that recent exports appear at the top.
3. Click `刷新资源包库`.

Observed:

The list is rebuilt in a different order; older timestamped folders move to the top and the newest export drops below the fold.

Expected:

The library should keep a stable default sort, preferably newest first, with an explicit sort control if other orders are useful.

Impact:

Users may think the latest export disappeared, especially after a successful export followed by refresh.

Suggested fix:

Sort scanned packs by manifest/export timestamp descending and preserve that order for both initial route load and manual refresh.

### P2: `检查` In The Export Library Collapses The List To One Item Without A Clear Detail Mode

Repro:

1. Open `导出`.
2. Click `检查` on the first pack.

Observed:

The library list collapses from many rows to one checked item with a short `已检查 ...` message. The page has no visible "back to all" detail state; `刷新资源包库` is the only obvious way to restore the list.

Expected:

Inspecting a pack should either open a detail panel while keeping the list context, or show a clear filtered/detail state with a return action.

Impact:

The user loses browsing context immediately after inspection.

Suggested fix:

Use a two-pane library layout: left list, right selected-pack details. Keep `刷新资源包库` as a scan action, not as the only escape hatch from inspection.

### P2: Export Library Uses Only Half The Desktop Window

Repro:

1. Open `导出` on the 1440px-wide app window.

Observed:

Pack cards occupy the left half of the content area, while the right half is mostly empty. Long paths are forced into narrow cards and repeated action buttons stack vertically.

Expected:

On desktop, the route should use the available width for either a denser table/list plus right-side details or wider cards with clearer path wrapping.

Impact:

Scanning many local exports is slower than necessary, and long paths become visually noisy.

Suggested fix:

Convert the route to a master-detail layout with columns for pack name, frames, timestamp/version, and actions. Put full path, validation result, manifest summary, and re-import details in the right panel.

### P2: Sample Story Mixes Knight Demo Assets With Green-box Pipeline Output

Repro:

1. Launch the app in the no-source state.
2. Observe the knight demo preview and `Hero Knight Pack` defaults.
3. Click `运行示例流程`.

Observed:

The no-source demo uses a polished knight preview and `Hero Knight Pack` naming. The actual sample pipeline imports `green-box-character.mp4` and outputs simple white block frames / `Green Box Character Pack`. In later re-import flows this can combine with the identity issue above.

Expected:

The first-run demo preview, bundled sample source, output names, and accessibility labels should tell one coherent sample story.

Impact:

Users may think the sample flow produced the wrong visual output or downgraded the asset.

Suggested fix:

Either make the bundled sample use the knight asset end to end, or make the initial preview clearly match the green-box sample. Update the default pack name and sample labels accordingly.

### P2: Preview Image Accessibility Alt Text Remains Knight-specific For Non-knight Frames

Repro:

1. Run the sample pipeline or re-import a green-box pack.
2. Inspect the accessibility tree for the main preview image.

Observed:

The image alt remains `选中的骑士精灵帧` / `Selected knight sprite frame` even when the visible frame is a green-box sample or a re-imported pack frame.

Expected:

The alt text should be generic, such as `选中的精灵帧`, or should interpolate the current pack/source name.

Impact:

Screen reader users receive incorrect content, and automated UI checks can misidentify the active asset.

Suggested fix:

Replace `stage.selectedKnightAlt` with a generic i18n key, or derive a per-source accessible label in `CanvasPreview`.

### P2: Collapsed Export Metadata Still Exposes Hidden Steppers In The Accessibility Tree

Repro:

1. In the initial workbench state, keep `资源包元数据和精灵表设置` collapsed.
2. Inspect the accessibility tree.

Observed:

The collapsed disclosure still exposes stepper controls for sheet columns/padding/margins.

Expected:

Collapsed advanced controls should be hidden from keyboard and accessibility navigation until expanded.

Impact:

Screen reader and keyboard users may encounter invisible controls.

Suggested fix:

Ensure collapsed content uses native `details` behavior or applies `hidden` / `aria-hidden` / inert handling consistently to the advanced settings body when collapsed.

### P3: Developer-facing Install Command Appears In Product Settings

Repro:

1. Open `设置`.
2. Read `本地运行环境` / `安装版同步`.

Observed:

The Settings page tells the user to run `npm run install:mac` to replace `/Applications/Game Sprite Forge.app`.

Expected:

End-user product settings should explain runtime/toolchain state and app version, not developer install commands. Developer sync instructions belong in docs or a dev-only diagnostics panel.

Impact:

The installed local product feels like an internal build, and non-developer testers may be unsure whether terminal commands are required to use Forge.

Suggested fix:

Move `npm run install:mac` instructions to developer docs. In Settings, show app version, install location, output folder, ffmpeg/ffprobe status, and a "Check for local update" placeholder only if that workflow exists.

### P3: Global Header Workflow Is Confusing On Settings And Exports Routes

Repro:

1. Open `设置` or `导出`.
2. Look at the top workflow tabs and quality status.

Observed:

The header still shows workbench workflow tabs and a status like `打开工作台` while the user is on Settings or the export library.

Expected:

Settings and library pages should either hide the workbench workflow strip or replace it with route-relevant status.

Impact:

The top-level navigation looks like it belongs to a different page and adds cognitive noise.

Suggested fix:

Render workflow tabs only on the Forge workbench route, or use a route-aware header layout.

### P3: `仅导入 MVP` Reads Like Internal Release Jargon

Repro:

1. Launch the app.
2. Read the app subtitle under `本地工作台`.

Observed:

The subtitle says `仅导入 MVP`.

Expected:

Use user-facing product language such as `本地导入工作台`, `本地资源处理`, or `导入与导出工作台`. Keep MVP scope language in documentation and QA surfaces.

Impact:

The UI exposes internal development framing instead of explaining what users can do.

Suggested fix:

Update `app.workbench.subtitle` in both locales to user-facing copy while keeping the scope constraints in docs and smoke tests.

## Recommended Next Fix Order

1. Preserve workspace state across Settings/language changes.
2. Normalize pack identity after library re-import and export.
3. Replace hardcoded `v1.2.0` with a single build version source.
4. Stabilize export-library sorting and convert inspect into a detail panel.
5. Clean up sample story/naming and preview accessibility labels.
6. Hide collapsed advanced controls from the accessibility tree.
7. Remove developer install-command copy and make the header route-aware.
