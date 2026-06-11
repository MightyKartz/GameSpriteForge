# UI Language Localization Evidence

Date: 2026-06-06

Scope: verify Forge follows system language by default, supports English and Simplified Chinese overrides in Settings, and keeps the MVP local-only without adding CLI, MCP, cloud, website, marketplace, or account surfaces.

## Automated Evidence

Commands run:

```bash
npm --workspace apps/mac run build
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:responsive
npm --workspace apps/mac run tauri -- build --bundles app
npm run test:scripts
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Result:

```text
frontend build: pass
English MVP UI smoke: pass
Simplified Chinese MVP UI smoke: pass
English responsive UI smoke: pass
Simplified Chinese responsive UI smoke: pass
Tauri .app bundle build: pass
script test suite: pass
Rust tests: pass
```

Smoke screenshots:

```text
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-mvp-en-US.png
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-mvp-zh-CN.png
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-responsive-1120-en-US.png
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-responsive-1280-en-US.png
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-responsive-1568-en-US.png
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-responsive-1120-zh-CN.png
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-responsive-1280-zh-CN.png
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-responsive-1568-zh-CN.png
```

Runtime visible-text dumps:

```text
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-mvp-visible-text-en-US.txt
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-mvp-visible-text-zh-CN.txt
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-visible-text-en-US.txt
/Users/kartz/Development/Forge/apps/mac/smoke-output/forge-workbench-visible-text-zh-CN.txt
```

Current zh-CN smoke guards reject visible English leftovers in the source panel, frame timeline, sprite sheet preview, sample assets, processed-frame summary, output folder, frame navigation, pending checks, import workspace, frame inspection, and processed-pixels controls. The source guard also requires every schema-defined quality recommendation id to map through i18n before it can appear in the Quality Report.

## Computer Use Evidence

The installed-app observations below were captured earlier in the UI language localization pass. During the later multi-agent hardening rerun, `/Users/kartz/Development/Forge/target/release/bundle/macos/Game Sprite Forge.app` was rebuilt and launched, but the macOS desktop was on the lock screen, so Computer Use could not inspect or click the Forge window for a fresh dynamic-flow rerun.

Real app opened:

```text
/Users/kartz/Development/Forge/target/release/bundle/macos/Game Sprite Forge.app
```

Observed Automatic mode:

```text
Settings language value: 跟随系统
App shell: 本地工作台, 仅导入 MVP
Navigation: 工作台, 导出, 设置
Workflow tabs: 导入, 帧, 背景, 锚点, 精灵表, 导出
Quality panel: 质量报告, 等待处理, 还没有质量报告
Export panel: 生成输出, 导出前还缺少, 资源包名称, 导出资源包
```

Observed English override:

```text
Settings -> language changed to English through the real pop-up button.
App shell switched to Local Workbench and IMPORT-ONLY MVP.
Navigation switched to Forge, Exports, Settings.
Forge panel showed Import Sources, Load Sample Path, Quality Report, Generated Outputs, Export Pack, and Validate Re-import.
```

Observed restore to Automatic:

```text
Settings -> language changed back to Automatic through the real pop-up button.
The local macOS language resolved back to Simplified Chinese.
Settings language value returned to 跟随系统.
```

Observed Exports route:

```text
Automatic Chinese Exports route showed 本地资源包库, 刷新资源包库, 打开文件夹, and frame counts as 帧.
Local pack entries remained local filesystem paths under /Users/kartz/Game Sprite Forge/Exports.
```

Observed Run Sample Pipeline:

```text
Clicked 运行示例流程 in the real app.
The pipeline completed and produced a live workspace.
Status bar: Full pipeline passed: 24 frames exported and re-imported.
Quality panel: 可用于游戏, 边框稳定性, 背景移除, 脚底锚点漂移, 循环匹配, 帧一致性.
Export panel: 可以导出, 已有处理后的帧和质量报告, 资源包, 帧数, 精灵表页数, Godot 辅助文件.
```

This earlier dynamic observation predates the later status-bar hardening. The current source guard requires the zh-CN dictionary to include `完整流程已通过`, and the current smoke dump covers visible static MVP copy after the hardening pass.

## Product Boundary

No CLI or MCP product surface was added. Computer Use and smoke scripts were used only for local QA evidence. The shipped app remains a local macOS workbench with local settings and local files.

## Accepted Terminology

The Simplified Chinese UI intentionally preserves professional or product tokens where translation would reduce clarity: Game Sprite Forge, PNG, JSON, .gsfpack, FFmpeg, FPS, Alpha, BBox, Godot, CC0, CC BY, filesystem paths, and sample asset names such as `sample_walk`.

## Current Follow-Ups

Native macOS file/dialog titles and some backend diagnostic note ids are still outside the typed UI dictionary. They should be localized only if they become frequent user-facing copy in manual QA.
