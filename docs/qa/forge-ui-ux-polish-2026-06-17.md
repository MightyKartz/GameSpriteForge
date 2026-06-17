# Forge UI/UX Polish QA Evidence Ledger

Date: 2026-06-17 CST

## Scope

This document is the QA and accessibility evidence structure for the current
Forge UI/UX polish pass. It is intentionally a ledger, not a pass report.

Covered surfaces:

```text
text size and hierarchy
focus-visible states
keyboard-only workflow paths
real UI smoke screenshots
failed or blocked evidence items
handoff items for final integration
```

Artifact root:

```text
docs/qa/artifacts/forge-ui-ux-polish-20260617
```

## Final Verification Summary

These checks were run by the main agent after the UI, CSS, and source-test
changes were integrated:

| Check | Result | Evidence |
| --- | --- | --- |
| Dependency restore | Passed with warning | `npm install` added 131 packages. `npm audit` reported 2 high severity vulnerabilities; no forced dependency upgrade was applied in this UI pass. |
| Web build | Passed | `npm --workspace apps/mac run build` exited 0 after `tsc --noEmit && vite build`. |
| Script tests | Passed | `npm run test:scripts` exited 0, including `PASS workflow focus source test` and `PASS notarization preflight test`. |
| en-US MVP smoke | Passed | `FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp` generated `01-smoke-mvp-en-US.png` and `03-smoke-visible-text-en-US.txt`. |
| zh-CN MVP smoke | Passed | `FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp` generated `02-smoke-mvp-zh-CN.png` and `04-smoke-visible-text-zh-CN.txt`. |
| Responsive smoke | Passed | `FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive` generated `05`, `06`, and `07` responsive screenshots. |
| Workspace debug app build | Passed with notarization warning | `npm --workspace apps/mac run tauri -- build --debug --bundles app` built `/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app`; notarization was skipped because Apple notarization env vars were not present. |
| Workspace debug app screenshot | Passed | `10-real-app-empty-workbench.png` was captured from the workspace debug app, not `/Applications`. |
| Keyboard focus path | Partially passed | CDP keyboard Tab capture produced `14-browser-keyboard-focus-import-button.png` and a focus trail through workflow tab, nav, toolchain, sample, and import buttons. Native macOS Tab screenshots were captured, but the focus ring was not visually conclusive at captured scale. |

## Starting State

Agent C branch check during the documentation slice:

```bash
git status -sb
```

Observed output:

```text
## codex/forge-ui-ux-polish
 M scripts/test-workflow-focus-source.mjs
```

The modification to `scripts/test-workflow-focus-source.mjs` was owned by the
implementation slices, not by Agent C's documentation pass. The main agent later
verified the final diff and reran the full script suite.

Product Design saved-context preflight, run from the Product Design
`skills/user-context` directory:

```bash
python3 scripts/user_context_preflight.py
```

Result:

```text
status: missing
entries: []
```

This run is therefore grounded in the Forge repository and the current user
brief, not in saved Product Design context.

## Existing Evidence Reviewed

Relevant QA documentation patterns reviewed:

```text
docs/qa/forge-ui-ux-workflow-focus-evidence-2026-06-11.md
docs/qa/forge-ui-ux-review-2026-06-11.md
docs/qa/transparent-gutter-sprite-real-ui-qa-2026-06-11.md
docs/qa/forge-real-ui-full-qa-post-fix-2026-06-07.md
docs/architecture/post-release-backlog.md
```

Relevant smoke and capture tooling reviewed:

```text
apps/mac/scripts/smoke-ui.mjs
scripts/capture-forge-ui-screenshot.mjs
scripts/test-workflow-focus-source.mjs
package.json
apps/mac/package.json
```

Useful existing conventions:

```text
Record app path, build command, UI driver, fixture path, result, and limits.
Keep QA findings under docs/qa and generated evidence under docs/qa/artifacts.
Do not reuse old screenshots as current-run evidence.
Inspect screenshots before accepting them as evidence.
Keep source/script smoke results separate from real macOS app click-throughs.
```

## Evidence Status

| Area | Current status | Evidence required before final UI/UX signoff |
| --- | --- | --- |
| Text size and hierarchy | Partially verified | CSS now keeps preview mode controls at 12px and timeline evidence items at 12px; smoke screenshots were inspected for obvious clipping. Automated contrast ratios were not measured. |
| Focus-visible states | Partially verified | Source guard covers `summary`, `textarea`, and `[tabindex]` focus styles. CDP keyboard screenshot proves import focus ring. Native app focus screenshot was inconclusive. |
| Keyboard-only path | Partially verified | Tab path was captured through import-entry controls. Export/settings keyboard paths still need a fuller manual pass. |
| Real UI smoke screenshots | Verified | Fresh `en-US` and `zh-CN` MVP smoke screenshots and visible text dumps were copied into this artifact root. |
| Responsive smoke screenshots | Verified | Fresh `en-US` responsive smoke screenshots at 1120, 1280, and 1568 widths were copied into this artifact root. |
| Workspace debug app screenshots | Partially verified | Workspace debug app empty-workbench screenshot was captured. Native focus screenshot was captured but not accepted as strong focus evidence. |
| Failed item log | Updated below | Remaining limits are recorded as partial coverage rather than pass claims. |

No test, smoke, or accessibility check is marked as passed in this document
unless a command and observed result are recorded in this file.

## Text Size And Hierarchy Checklist

Use this for the final current-UI pass.

| Check | Target | Evidence slot | Status |
| --- | --- | --- | --- |
| Dense panel body text | Minimum 12px equivalent for normal readable body copy | Source/computed style note plus screenshot crop | Pending main agent verification |
| Primary text contrast | 4.5:1 or better for normal text where measurable | Token or computed color note | Pending main agent verification |
| Secondary text contrast | At least 3:1 for helper/metadata text; avoid disabled/actionable ambiguity | Token or computed color note | Pending main agent verification |
| Heading hierarchy | Stage title, panel title, and metadata labels scan in the intended order | Annotated screenshot or review note | Pending main agent verification |
| Button text fit | Labels fit in English and Simplified Chinese without clipping | en-US and zh-CN smoke screenshots | Pending main agent verification |
| Timeline density | Evidence strip and frame controls remain readable at smoke viewport sizes | smoke screenshot and visible-text dump | Pending main agent verification |

## Focus And Keyboard Checklist

Use the workspace debug app when recording real macOS UI evidence:

```text
/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app
```

Close stale `/Applications/Game Sprite Forge.app` windows before capturing
evidence, or explicitly record why the installed app is the intended target.

| Step | Keyboard path | Expected evidence | Status |
| --- | --- | --- | --- |
| 01 | Launch workspace app, press Tab through top navigation and first workbench controls | Visible focus order starts in a predictable place and does not skip primary actions | Pending main agent verification |
| 02 | Use keyboard to reach Import Video, Import PNG Sequence, Import Sprite Sheet, and Run Sample Pipeline controls | Each target has a visible focus indicator and readable label | Pending main agent verification |
| 03 | Open and close disclosure summaries with Enter or Space | Focus ring is visible on `summary` controls and focus returns predictably | Pending main agent verification |
| 04 | Navigate timeline frame controls and Previous/Next frame buttons | Selected frame state is exposed without relying only on color | Pending main agent verification |
| 05 | Toggle preview inspection overlay from keyboard | Toggle exposes pressed state and visible state change | Pending main agent verification |
| 06 | Reach export metadata fields only when the section is expanded | Collapsed advanced fields are not in the tab path | Pending main agent verification |
| 07 | Move between Forge, Exports, and Settings routes by keyboard | Route content receives a sensible focus entry after navigation | Pending main agent verification |
| 08 | In Settings, reach language selector and tool path choose buttons | Focus state and labels are visible in both locales | Pending main agent verification |

## Smoke Evidence Slots

Recommended command set for the final current-run evidence:

```bash
npm --workspace apps/mac run build
FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp
FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp
npm --workspace apps/mac run smoke:ui:responsive
```

Expected generated source locations from the smoke script:

```text
apps/mac/smoke-output/forge-workbench-mvp-en-US.png
apps/mac/smoke-output/forge-workbench-mvp-zh-CN.png
apps/mac/smoke-output/forge-workbench-mvp-visible-text-en-US.txt
apps/mac/smoke-output/forge-workbench-mvp-visible-text-zh-CN.txt
apps/mac/smoke-output/forge-workbench-responsive-1120-en-US.png
apps/mac/smoke-output/forge-workbench-responsive-1280-en-US.png
apps/mac/smoke-output/forge-workbench-responsive-1568-en-US.png
```

Copy only freshly generated, inspected artifacts into:

```text
docs/qa/artifacts/forge-ui-ux-polish-20260617
```

Suggested accepted filenames:

```text
01-smoke-mvp-en-US.png
02-smoke-mvp-zh-CN.png
03-smoke-visible-text-en-US.txt
04-smoke-visible-text-zh-CN.txt
05-smoke-responsive-1120-en-US.png
06-smoke-responsive-1280-en-US.png
07-smoke-responsive-1568-en-US.png
```

Accepted current-run smoke artifacts:

| File | Source command | Evidence |
| --- | --- | --- |
| `01-smoke-mvp-en-US.png` | `FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp` | Empty workbench with right-rail stage explanation and no duplicate right-rail source CTA. |
| `02-smoke-mvp-zh-CN.png` | `FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp` | Chinese empty workbench without obvious clipping in the central import launcher or right rail. |
| `03-smoke-visible-text-en-US.txt` | en-US MVP smoke | Visible English text dump used by the smoke guard. |
| `04-smoke-visible-text-zh-CN.txt` | zh-CN MVP smoke | Visible Chinese text dump used by the smoke guard. |
| `05-smoke-responsive-1120-en-US.png` | `FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive` | Narrow desktop responsive screenshot. |
| `06-smoke-responsive-1280-en-US.png` | responsive smoke | Mid-width desktop screenshot. |
| `07-smoke-responsive-1568-en-US.png` | responsive smoke | Wide desktop screenshot. |

## Real App Screenshot Slots

Preferred build and launch flow:

```bash
npm --workspace apps/mac run tauri -- build --debug --bundles app
open "/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app"
```

Screenshot helper:

```bash
FORGE_CAPTURE_OWNER="Game Sprite Forge" FORGE_CAPTURE_ACTIVATE=1 \
  node scripts/capture-forge-ui-screenshot.mjs \
  docs/qa/artifacts/forge-ui-ux-polish-20260617/10-real-app-empty-workbench.png
```

Acceptance rules:

```text
Reject blank, loading, wrong-window, stale-app, or cropped screenshots.
Record the app path and UI driver for every accepted screenshot.
For each screenshot, add a short note saying what keyboard or visual state it proves.
Do not treat a screenshot as proof of keyboard access unless the focus state is visible or the keyboard action was separately observed.
```

Suggested real app evidence names:

```text
10-real-app-empty-workbench-en-US.png
11-real-app-empty-workbench-zh-CN.png
12-real-app-focus-import-controls.png
13-real-app-focus-timeline-controls.png
14-real-app-focus-preview-inspection-toggle.png
15-real-app-focus-export-advanced-collapsed.png
16-real-app-settings-focus-language.png
```

Accepted current-run real app and keyboard artifacts:

| File | Source command | Evidence |
| --- | --- | --- |
| `10-real-app-empty-workbench.png` | `FORGE_CAPTURE_OWNER="Game Sprite Forge" FORGE_CAPTURE_ACTIVATE=1 node scripts/capture-forge-ui-screenshot.mjs ...` | Workspace debug app launched from `/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app`. |
| `12-real-app-focus-import-controls.png` | Native macOS `Tab` followed by screenshot capture | Native app remained active, but the focus ring was not visually conclusive at the captured scale. Kept as supporting, not decisive, evidence. |
| `13-real-app-focus-central-controls.png` | Six native macOS `Tab` events followed by screenshot capture | Same limitation as above: screenshot is valid, but focus ring is not strong enough to claim keyboard coverage by itself. |
| `14-browser-keyboard-focus-import-button.png` | Chrome DevTools Protocol Tab capture against the freshly built Vite preview | Stronger keyboard evidence. The recorded focus trail reached `导入`, `工作台`, `导出`, `设置`, `检查工具链`, `运行示例流程`, `选择视频文件`, `导入 PNG 序列`, and `导入精灵表`. |

## Failure And Blocker Log

| ID | Severity | Status | Evidence | Owner / next action |
| --- | --- | --- | --- | --- |
| QA-UX-20260617-001 | P2 | Closed | Current-run smoke screenshots were generated and copied into this artifact root. | No follow-up for MVP/responsive smoke. |
| QA-UX-20260617-002 | P2 | Partial | Keyboard Tab path was captured via CDP and import controls show a focus ring. Native macOS focus screenshots were inconclusive. | Future manual QA should record a higher-resolution native focus pass through export/settings. |
| QA-UX-20260617-003 | P2 | Partial | Source CSS and smoke screenshots cover text hierarchy and obvious clipping. Automated contrast ratios were not measured. | Add a contrast/style audit if the dark theme is prepared for accessibility certification. |
| QA-UX-20260617-004 | Info | Resolved | Main agent verified the final `scripts/test-workflow-focus-source.mjs` diff and reran `npm run test:scripts` successfully. | No follow-up. |
| QA-UX-20260617-005 | Info | Open | `npm install` reported 2 high severity dependency audit items. | Handle in a separate dependency/security pass; do not apply `npm audit fix --force` inside this UI PR. |

## Agent C Commands Run

Commands run for this documentation slice:

```bash
git status -sb
cat /Users/kartz/Development/Forge/.agents/skills/forge-dev/SKILL.md
cat /Users/kartz/.codex/plugins/cache/openai-curated-remote/product-design/0.1.46/skills/index/SKILL.md
cat /Users/kartz/.codex/plugins/cache/openai-curated-remote/product-design/0.1.46/skills/audit/SKILL.md
cat /Users/kartz/.codex/plugins/cache/openai-curated/superpowers/43313cc9/skills/verification-before-completion/SKILL.md
cat /Users/kartz/.codex/plugins/cache/openai-curated-remote/product-design/0.1.46/references/critical-overrides.md
cat /Users/kartz/.codex/plugins/cache/openai-curated-remote/product-design/0.1.46/skills/user-context/SKILL.md
cat /Users/kartz/.codex/plugins/cache/openai-curated-remote/product-design/0.1.46/skills/audit/references/design-audit-framework.md
cat /Users/kartz/.codex/plugins/cache/openai-curated-remote/product-design/0.1.46/references/communication-protocol.md
cat /Users/kartz/.codex/plugins/cache/openai-curated-remote/product-design/0.1.46/skills/user-context/scripts/user_context_preflight.py
python3 scripts/user_context_preflight.py
rg --files docs/qa
rg --files scripts
rg -n "smoke|smoke:ui|accessib|focus|keyboard|screenshot|QA|qa" package.json apps packages docs scripts
sed -n '1,240p' docs/qa/forge-ui-ux-workflow-focus-evidence-2026-06-11.md
sed -n '1,260p' docs/qa/forge-ui-ux-review-2026-06-11.md
sed -n '1,220p' docs/qa/transparent-gutter-sprite-real-ui-qa-2026-06-11.md
sed -n '1,220p' docs/qa/forge-real-ui-full-qa-post-fix-2026-06-07.md
rg --files scripts apps/mac/scripts -g '*smoke*' -g '*capture*' -g '*qa*' -g '*workflow-focus*'
sed -n '1,260p' apps/mac/scripts/smoke-ui.mjs
sed -n '260,620p' apps/mac/scripts/smoke-ui.mjs
sed -n '620,940p' apps/mac/scripts/smoke-ui.mjs
sed -n '1,220p' scripts/capture-forge-ui-screenshot.mjs
sed -n '1,160p' package.json
sed -n '1,220p' apps/mac/package.json
sed -n '1,220p' scripts/test-workflow-focus-source.mjs
sed -n '1,260p' docs/architecture/post-release-backlog.md
sed -n '260,520p' docs/architecture/post-release-backlog.md
find docs/qa/artifacts/forge-ui-ux-workflow-focus-20260611 -maxdepth 1 -type f -print
find docs/qa/artifacts -maxdepth 2 -name README.md -print
ls -la docs/qa
mkdir -p docs/qa/artifacts/forge-ui-ux-polish-20260617
git diff --check -- docs/qa/forge-ui-ux-polish-2026-06-17.md docs/qa/artifacts/forge-ui-ux-polish-20260617/README.md
rg -n "[[:blank:]]$" docs/qa/forge-ui-ux-polish-2026-06-17.md docs/qa/artifacts/forge-ui-ux-polish-20260617/README.md
forbidden-name scan over the two new QA docs
test -f docs/qa/forge-ui-ux-polish-2026-06-17.md
test -f docs/qa/artifacts/forge-ui-ux-polish-20260617/README.md
git ls-files --others --exclude-standard docs/qa/forge-ui-ux-polish-2026-06-17.md docs/qa/artifacts/forge-ui-ux-polish-20260617/README.md
```

Command results summary:

```text
Branch was codex/forge-ui-ux-polish.
Product Design saved context was missing.
Existing QA docs and smoke tooling were reviewed.
No UI smoke, build, test suite, or real macOS app screenshot command was run in this slice.
Direct trailing-whitespace scan returned no matches; `rg` exited 1 as expected for an empty result.
Forbidden-name scan returned no matches; the search exited 1 as expected for an empty result.
Both new QA files exist and are untracked pending integration.
```

## Final Handoff Checklist

Before closing the full UI/UX polish pass, the main agent should add:

```text
fresh en-US MVP smoke screenshot
fresh zh-CN MVP smoke screenshot
fresh en-US visible text dump
fresh zh-CN visible text dump
responsive smoke screenshots if layout density changed
real workspace debug app screenshot proving the current first screen
keyboard focus evidence for import, timeline, preview, export, settings, and route navigation
actual command outputs for build, smoke, and any source guards that the final pass claims as passing
notes for every failure, blocker, or deferred accessibility risk
```
