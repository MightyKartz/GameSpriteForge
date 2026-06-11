# Forge Real UI QA Issue List

Date: 2026-06-06 16:41 CST

Scope: real macOS UI testing through Computer Use. No code changes were made during this pass.

## Tested Builds

```text
/Applications/Game Sprite Forge.app
/Users/kartz/Development/Forge/target/release/bundle/macos/Game Sprite Forge.app
```

Important environment finding: the installed `/Applications` app is older than the latest development bundle. The installed app opened in English, had no Language selector in Settings, and still exposed raw quality recommendation ids. The latest development bundle opened in Simplified Chinese and included the language selector and localized quality recommendations.

## Evidence

```text
docs/qa/screenshots/forge-real-ui-sample-complete-2026-06-06.png
docs/qa/screenshots/forge-real-ui-latest-exports-2026-06-06.png
```

Latest successful export checked on disk:

```text
/Users/kartz/Game Sprite Forge/Exports/Green-Box-Character-Pack-1780735213055/
```

The output folder contains frame PNGs, `sprite_sheet.png`, `preview.gif`, `atlas.json`, `manifest.json`, `quality-report.json`, `godot_import.json`, and the `.gsfpack` folder, so the export pipeline produced files even when UI previews failed to render them.

## Passed In Real UI

- Latest bundle launches and exposes an accessible macOS window.
- Simplified Chinese shell loads in Automatic language mode.
- Settings shows Language, ffmpeg path, ffprobe path, default output folder, default FPS, and default sheet size.
- Language pop-up shows 跟随系统, English, 简体中文.
- Running the bundled sample completes import, process, quality, export, and validate re-import.
- Frame navigation changes selected frame state from 1/24 to 2/24.
- Background, Anchor, Sheet, and Export workflow tabs become unlocked after sample processing.
- Apply Anchor does not crash and recomputes quality.
- Export Pack writes a new output folder.
- Validate Re-import succeeds.
- Exports route lists the latest exported pack.
- Open Folder opens Finder for the pack folder.

## Issues

### P0: Processed Image Previews Render As Broken/Blank

Repro:

1. Open `/Users/kartz/Development/Forge/target/release/bundle/macos/Game Sprite Forge.app`.
2. Click `运行示例流程`.
3. Wait for the full pipeline to pass.
4. Observe the main frame preview, timeline thumbnails, and exported sprite sheet preview.

Observed:

The app shows bbox/anchor overlays, timeline frame buttons, and an exported sprite sheet image element, but the visual content is missing or displayed as a broken image icon. This also happens in the installed `/Applications` build.

Expected:

The processed character frame, each timeline thumbnail, and the exported sprite sheet should render visibly so users can inspect background removal, anchor placement, frame consistency, and sheet output.

Impact:

This breaks the core visual inspection loop of the product. The files are exported correctly on disk, but the user cannot trust or inspect the result from the app UI.

Suggested fix:

Audit Tauri asset URL conversion for generated local files. Verify frame PNG, processed PNG, and exported sprite sheet paths are converted into WebView-safe URLs consistently after import, process, export, reprocess, and validate.

### P1: Installed App Is Stale Compared With Latest Development Bundle

Repro:

1. Launch by app name or `/Applications/Game Sprite Forge.app`.
2. Open Settings.

Observed:

The installed app opens in English, lacks the Language selector, and still shows raw recommendation ids such as `use_shorter_clip`.

Expected:

The installed app should match the latest verified build or the test instructions should make it impossible to accidentally test a stale bundle.

Impact:

Manual QA and user testing can report old bugs as current bugs. It also makes localization validation unreliable.

Suggested fix:

Add a clear install/update step after building the latest bundle, or add a visible build identifier in Settings/About so QA can confirm which bundle is running.

### P1: Exports Library Promises More Actions Than It Provides

Repro:

1. Open the latest bundle.
2. Run the sample pipeline.
3. Click `导出` in the left navigation.

Observed:

The page subtitle says it can validate, inspect, open, and re-import local `.gsfpack` exports, but each pack card only exposes `打开文件夹`.

Expected:

Each pack card should provide visible actions for inspect, validate, re-import, and open, or the subtitle should only promise the actions currently available.

Impact:

Users cannot complete important library workflows from the library page despite the page claiming those workflows exist.

Suggested fix:

Add per-pack `检查`, `验证`, `重新导入`, and `打开文件夹` actions, or revise the copy until those actions are implemented.

### P1: Quality And Status Messaging Can Contradict Warnings

Repro:

1. Run the sample pipeline.
2. Open Anchor.
3. Click `Apply Anchor` / `应用锚点`.

Observed:

Quality can show warnings such as `Cell Risk` / `单元格风险` and multiple warning recommendations, while the bottom status still says `All checks passed` / `全部检查通过`. Export readiness remains green.

Expected:

If warnings remain, the status should say something like `可导出，但有 2 项建议` rather than `全部检查通过`.

Impact:

Users may ignore important quality warnings because the global status says everything passed.

Suggested fix:

Separate hard blockers from warnings in the status model: use `blocked`, `ready_with_warnings`, and `ready_clean` rather than only pass/fail.

### P2: Sheet And Export Workflow Tabs Have Weak Distinct Value

Repro:

1. Complete the sample pipeline.
2. Click `精灵表`.
3. Click `导出`.

Observed:

Both tabs largely keep the same central frame inspection area and rely on the right export panel. There is no dedicated sheet inspection/editing surface, and Export does not feel different from the general workspace.

Expected:

`精灵表` should foreground sprite sheet layout, page preview, columns, padding, margin, split behavior, and sheet validation. `导出` should foreground export targets, output path, pack metadata, export history, and validation.

Impact:

The workflow tabs imply a step-by-step editor, but later steps do not yet provide step-specific focus.

Suggested fix:

Give Sheet and Export dedicated center panels, or remove/disable the tabs until their content differs enough to justify separate steps.

### P2: Chinese UI Still Has Residual English In Metadata Toggle

Repro:

1. Open latest bundle in Simplified Chinese.
2. Look at `资源包元数据和精灵表设置`.

Observed:

The summary control shows `Show` or `Hide` in English.

Expected:

Use `显示` / `隐藏`.

Impact:

Small but visible localization polish issue in a frequently used export settings area.

Suggested fix:

Move the details summary state text into the i18n dictionary.

### P2: Left Run Summary And File Names Are Too Narrow

Repro:

1. Run the sample pipeline.
2. Look at the left `运行摘要`.

Observed:

Source and export names are heavily truncated, making it hard to confirm which source/export is active. In the installed English build, several summary labels also appear cramped.

Expected:

The user should be able to confirm source, export, quality, and validation without guessing from short fragments.

Impact:

This reduces confidence after long-running operations and makes screenshots harder to interpret during QA.

Suggested fix:

Use tooltips, a wider summary layout, two-line wrapping for filenames, or a collapsible detail panel.

### P3: Settings Page Wastes Large Horizontal Space

Repro:

1. Open Settings in the latest bundle.

Observed:

The settings form occupies the upper-left area while most of the window is empty.

Expected:

The page could use the available space for toolchain status, detected tool versions, output folder preview, language explanation, or a compact QA diagnostics panel.

Impact:

Not a functional blocker, but the page feels unfinished compared with the dense workbench.

Suggested fix:

Add a right-side status panel or reduce the settings route width so the composition feels intentional.

## UX Health Summary

```text
Primary task success: partial pass
Core pipeline: pass
Visual inspection loop: fail
Localization: mostly pass on latest bundle
Installed app parity: fail
Exports library completeness: partial
Overall real-UI readiness: not ready for broad user testing until image rendering is fixed
```

## Recommended Next Fix Order

1. Fix WebView-safe local image rendering for processed frames, timeline thumbnails, and exported sprite sheet previews.
2. Align installed app with latest build or add a visible build/version identifier.
3. Make quality status distinguish clean pass from ready-with-warnings.
4. Add or remove Exports Library actions to match the page promise.
5. Give Sheet and Export tabs dedicated content, or simplify the workflow tabs.
6. Finish the remaining Chinese UI polish (`Show` / `Hide`) and improve sidebar summary truncation.
