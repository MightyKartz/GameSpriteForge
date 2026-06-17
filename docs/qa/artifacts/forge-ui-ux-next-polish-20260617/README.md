# Forge UI/UX Next Polish Artifacts - 2026-06-17

This folder is reserved for current-run QA evidence for:

```text
docs/qa/forge-ui-ux-next-polish-2026-06-17.md
```

Only add artifacts generated or captured during the 2026-06-17 next-polish
verification pass. Do not reuse old screenshots, stale smoke output, or captures
from an installed app when the workspace debug bundle is required.

## Expected Smoke Artifacts

```text
01-smoke-mvp-en-US.png
02-smoke-mvp-zh-CN.png
03-smoke-visible-text-en-US.txt
04-smoke-visible-text-zh-CN.txt
05-smoke-responsive-1120-en-US.png
06-smoke-responsive-1280-en-US.png
07-smoke-responsive-1568-en-US.png
```

## Expected macOS App Artifacts

```text
10-real-app-empty-workbench-en-US.png
11-real-app-empty-workbench-zh-CN.png
12-real-app-focus-import-controls.png
13-real-app-focus-timeline-controls.png
14-real-app-focus-preview-inspection-toggle.png
15-real-app-focus-export-advanced-collapsed.png
16-real-app-settings-focus-language.png
17-real-app-route-navigation-focus.png
```

Workspace debug app path to record for accepted real-app screenshots:

```text
/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app
```

## Accepted In This Run

```text
01-smoke-mvp-en-US.png
source: FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
locale: en-US
viewport: 1568x1003
proves: empty workbench, right rail checklist, central import CTA, timeline empty state
limits: browser smoke evidence, not native WebView evidence

02-smoke-mvp-zh-CN.png
source: FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
locale: zh-CN
viewport: 1568x1003
proves: localized empty workbench labels fit at MVP size
limits: browser smoke evidence, not native WebView evidence

03-smoke-visible-text-en-US.txt
source: en-US MVP smoke visible text dump
locale: en-US
proves: visible text includes expected English workbench copy
limits: text-only, no layout evidence

04-smoke-visible-text-zh-CN.txt
source: zh-CN MVP smoke visible text dump
locale: zh-CN
proves: visible text includes expected Chinese workbench copy
limits: text-only, no layout evidence

05-smoke-responsive-1120-en-US.png
source: FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
locale: en-US
viewport: 1120x760
proves: narrow desktop layout remains readable
limits: browser smoke evidence, not native WebView evidence

06-smoke-responsive-1280-en-US.png
source: FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
locale: en-US
viewport: 1280x820
proves: mid-width desktop layout remains stable
limits: browser smoke evidence, not native WebView evidence

07-smoke-responsive-1568-en-US.png
source: FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive
locale: en-US
viewport: 1568x1003
proves: wide desktop layout remains aligned
limits: browser smoke evidence, not native WebView evidence
```

## Rejected In This Run

```text
10-real-app-next-polish.png
10-real-app-next-polish-retry.png
11-real-app-keyboard-focus-tab.png
12-real-app-terminal-launch.png
13-real-app-with-preview-server.png
reason: workspace debug app window rendered blank content, so these cannot prove UI or keyboard focus behavior.
next action: investigate native bundle/WebView runtime before accepting native keyboard QA evidence.
```

## Rejection Rules

Reject and record artifacts that are:

```text
blank
still loading
cropped in a way that hides the tested surface
from a stale installed app window
from the wrong locale
from an older implementation state
missing a visible focus state when used as keyboard evidence
```

## Main Integration Checklist

Before final signoff, this folder should contain or explicitly defer:

```text
en-US MVP smoke screenshot
zh-CN MVP smoke screenshot
en-US visible text dump
zh-CN visible text dump
responsive smoke screenshots
workspace debug app first-screen screenshot
native keyboard focus screenshots or equivalent evidence
notes for any rejected captures
```
