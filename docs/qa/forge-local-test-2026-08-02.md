# Forge local test — 2026-08-02

## Environment

- Workspace: `/Users/kartz/Development/Forge`
- Rust: `rustc 1.96.0`, `cargo 1.96.0`
- Node/npm: `v24.4.1` / `11.6.0`
- Godot: `4.6.3.stable.official.7d41c59c4`
- Native app: `/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app`
- UI drivers: Chromium DevTools Protocol for scripted smoke tests; Quartz window enumeration/capture for the native workspace bundle.

## Results

| Check | Result | Evidence |
| --- | --- | --- |
| Formatting | Pass | `cargo fmt --all -- --check` |
| Rust workspace | Pass | 97 tests passed; no failures. Includes media, quality, Character Pack, repair lineage, project manifest, pack validation, and Tauri tests. |
| Automation/MCP | Pass | Automation/project/repair tests passed; MCP 0.4.0 build, TypeScript check, and 2 bundle tests passed. |
| Script regression | Pass | All source, fixture, export, recovery, UI, FFmpeg, Godot, release-preflight, and notarization-preflight script checks passed. |
| Frontend build | Pass | TypeScript and Vite production build completed; 1,603 modules transformed. |
| UI smoke | Pass | MVP, Character Pack, repair en-US, repair zh-CN, and 1280px responsive suites passed. |
| Native app build | Pass | Debug `.app` bundled and signed; `codesign --verify --deep --strict` passed. Notarization was not requested for this debug build. |
| Native window | Pass | Quartz found and captured the workspace app's `Game Sprite Forge` 1440×923 window. The local workbench, navigation, import controls, timeline, preview, and status bar rendered. |
| Godot pack import | Pass | Generated an 8-frame pack, imported it with Godot 4.6.3, and saved `Godot_Smoke_Walk.spriteframes.tres` plus `Godot_Smoke_Walk.tscn`. |
| Plugin/Skill | Pass | Forge plugin and Skill validators passed. |
| Diff hygiene | Pass | `git diff --check` reported no whitespace errors. Existing uncommitted work was preserved. |

## Native evidence

![Workspace debug app](artifacts/forge-local-test-native-2026-08-02.png)

The window was initially occluded by the current full-screen macOS space, so an Accessibility-only window count was temporarily zero. Quartz still reported the correct workspace PID and window bounds; direct window capture confirmed that the app had created and rendered the window.

## Godot fixture

- Root: `docs/qa/artifacts/godot-pack-smoke-20260802-094025`
- Generated pack: `exports/godot_smoke_walk/Godot-Smoke-Walk.gsfpack`
- Imported frames: 8
- Generated Godot resources: `res://imported/Godot_Smoke_Walk/Godot_Smoke_Walk.spriteframes.tres`, `res://Godot_Smoke_Walk.tscn`

## Remaining scope

This run covers automated import/export pipelines, generated fixtures, browser UI behavior, native rendering, and Godot import. It does not replace a manual test using a user-selected production video, a notarized release build, or distribution testing on a clean Mac.
