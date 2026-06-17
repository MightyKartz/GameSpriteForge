# Forge UI/UX Next Polish QA Ledger

Date: 2026-06-17 CST

## Scope

This ledger tracks QA, accessibility, and evidence planning for the Forge UI/UX
next-polish pass. It is a verification checklist and handoff document, not a
claim that the final integrated UI has passed.

Covered evidence areas:

```text
build and source/script guards
en-US and zh-CN MVP smoke
responsive smoke
workspace macOS debug bundle
native keyboard focus path
screenshots and visible text dumps
failures, blockers, and residual risk
```

Artifact root:

```text
docs/qa/artifacts/forge-ui-ux-next-polish-20260617
```

## Current Status

| Area | Status | Evidence state |
| --- | --- | --- |
| Build verification | Passed | Web build and debug app bundle command exited 0. |
| Script verification | Passed | Source guards and script suite exited 0 after the smoke CDP stability fix. |
| en-US MVP smoke | Passed | Fresh screenshot and visible-text dump copied into artifact root. |
| zh-CN MVP smoke | Passed | Fresh screenshot and visible-text dump copied into artifact root. |
| Responsive smoke | Passed | 1120, 1280, and 1568 width screenshots copied into artifact root. |
| macOS debug bundle | Partial | Bundle build exited 0, but workspace `.app` rendered a blank WebView in capture. |
| Native keyboard focus path | Blocked | Real app content was blank, so native focus screenshots were rejected. |
| Screenshot review | Partial | Browser/smoke screenshots accepted; native app screenshots rejected as blank. |
| Failure log | Open | Native debug bundle blank-window issue remains for follow-up. |
| Residual risk | Open | Native app QA and contrast ratios need follow-up beyond this UI polish pass. |

No build, smoke, test, or real macOS app command is marked passed in this
document unless the command, observed result, and evidence location are recorded
here.

## Verification Command Slots

Fill this table during final integration. Keep raw pass/fail claims tied to the
actual command output and artifact paths.

| Check | Command | Status | Evidence / notes |
| --- | --- | --- | --- |
| Web build | `npm --workspace apps/mac run build` | Passed | Exit 0; `tsc --noEmit && vite build`; 1601 modules transformed. |
| Script suite | `npm run test:scripts` | Passed | Exit 0; all source guard scripts passed, including `workflow focus source test`. |
| en-US MVP smoke | `FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:mvp` | Passed | `01-smoke-mvp-en-US.png`, `03-smoke-visible-text-en-US.txt`. |
| zh-CN MVP smoke | `FORGE_SMOKE_LOCALE=zh-CN npm --workspace apps/mac run smoke:ui:mvp` | Passed | `02-smoke-mvp-zh-CN.png`, `04-smoke-visible-text-zh-CN.txt`. |
| Responsive smoke | `FORGE_SMOKE_LOCALE=en-US npm --workspace apps/mac run smoke:ui:responsive` | Passed | `05-smoke-responsive-1120-en-US.png`, `06-smoke-responsive-1280-en-US.png`, `07-smoke-responsive-1568-en-US.png`. |
| macOS debug bundle | `npm --workspace apps/mac run tauri -- build --debug --bundles app` | Partial | Exit 0; app bundle created at workspace path; notarization skipped because Apple credential env vars were absent. |
| Real app screenshot | `FORGE_CAPTURE_OWNER="Game Sprite Forge" FORGE_CAPTURE_ACTIVATE=1 node scripts/capture-forge-ui-screenshot.mjs <artifact>` | Failed | Captures from workspace app were blank and rejected: `10-real-app-next-polish.png`, `10-real-app-next-polish-retry.png`, `12-real-app-terminal-launch.png`, `13-real-app-with-preview-server.png`. |

Workspace debug app path expected by Forge QA conventions:

```text
/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app
```

## MVP Smoke Slots

Expected source outputs from the smoke scripts:

```text
apps/mac/smoke-output/forge-workbench-mvp-en-US.png
apps/mac/smoke-output/forge-workbench-mvp-zh-CN.png
apps/mac/smoke-output/forge-workbench-mvp-visible-text-en-US.txt
apps/mac/smoke-output/forge-workbench-mvp-visible-text-zh-CN.txt
apps/mac/smoke-output/forge-workbench-responsive-1120-en-US.png
apps/mac/smoke-output/forge-workbench-responsive-1280-en-US.png
apps/mac/smoke-output/forge-workbench-responsive-1568-en-US.png
```

Recommended artifact filenames:

```text
01-smoke-mvp-en-US.png
02-smoke-mvp-zh-CN.png
03-smoke-visible-text-en-US.txt
04-smoke-visible-text-zh-CN.txt
05-smoke-responsive-1120-en-US.png
06-smoke-responsive-1280-en-US.png
07-smoke-responsive-1568-en-US.png
```

Acceptance checklist:

| Locale / viewport | Required review | Status |
| --- | --- | --- |
| en-US MVP | Main empty workbench, import actions, stage rail, labels, and no obvious clipping | Accepted: `01-smoke-mvp-en-US.png` |
| zh-CN MVP | Chinese labels, button fit, panel copy, and no obvious clipping | Accepted: `02-smoke-mvp-zh-CN.png` |
| 1120 en-US | Narrow desktop scan for overflow, toolbar wrapping, and right-rail density | Accepted: `05-smoke-responsive-1120-en-US.png` |
| 1280 en-US | Mid-width desktop scan for stable panel rhythm and readable text | Accepted: `06-smoke-responsive-1280-en-US.png` |
| 1568 en-US | Wide desktop scan for excessive empty space and aligned workbench regions | Accepted: `07-smoke-responsive-1568-en-US.png` |

## macOS Debug Bundle And Screenshots

Before accepting real app evidence:

```text
Close stale /Applications/Game Sprite Forge.app windows, or explicitly record
why an installed app window is the intended target. Prefer launching the
workspace debug bundle path listed above.
```

Suggested screenshot slots:

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

For each accepted screenshot, record:

```text
command or driver
workspace app path
locale
window size or viewport
what the artifact proves
known limits
```

## Native Keyboard Focus Path

Use the real workspace debug app for this pass unless final integration records
a reason to use browser-level keyboard evidence as a supplement.

| Step | Keyboard path | Expected evidence | Status |
| --- | --- | --- | --- |
| 01 | Launch the workspace debug app and press Tab from the initial workbench | Focus starts predictably and is visible. | Blocked: app content blank in native capture. |
| 02 | Reach primary import controls by keyboard | Import Video, Import PNG Sequence, and Import Sprite Sheet have visible focus states. | Blocked: app content blank in native capture. |
| 03 | Navigate route controls | Forge, Exports, and Settings are reachable and route changes do not strand focus. | Blocked: app content blank in native capture. |
| 04 | Open and close disclosure controls | Summary/disclosure controls show focus and respond to Enter or Space. | Blocked: app content blank in native capture. |
| 05 | Navigate timeline controls | Previous/Next frame, selected frame state, and loop markers are understandable without pointer input. | Blocked: app content blank in native capture. |
| 06 | Toggle preview inspection state | Toggle exposes focus and pressed/active state with a visible UI change. | Blocked: app content blank in native capture. |
| 07 | Reach export and advanced settings controls | Collapsed content is not unexpectedly in the tab path; expanded fields are reachable. | Blocked: app content blank in native capture. |
| 08 | Reach Settings language/tool path controls | Controls are reachable and labels fit in en-US and zh-CN. | Blocked: app content blank in native capture. |

## Accessibility Review Slots

| Area | Evidence needed | Status |
| --- | --- | --- |
| Focus-visible states | Screenshot or active-element trail for import, timeline, preview, export, settings, and route navigation. | Partial: CSS hooks/source guard present; native focus screenshots blocked by blank app. |
| Text size | Source/computed note for dense labels and screenshots showing readable labels at smoke sizes. | Passed: `--readable-small` added; smoke screenshots reviewed at MVP and responsive sizes. |
| Text contrast | Token or computed-color note for primary, secondary, and disabled/action text. | Partial: visual smoke review only; no automated contrast ratio measurement. |
| Button text fit | en-US and zh-CN screenshots reviewed for clipping and overlap. | Passed for smoke captures. |
| State communication | Selected frame, preview mode, inspection state, and failure states do not rely only on color. | Passed structurally through source guards for checklist, density summary, and selected frame text evidence. |
| Reduced ambiguity | Empty, loading, blocked, and failed states are distinguishable in visible text. | Passed for empty workbench smoke; native blank-window failure remains separate. |

## Failure And Blocker Log

Add a row for every failure, blocker, inconclusive screenshot, flaky command, or
deferred accessibility concern. Do not delete failures after fixes; close them
with the command or artifact that proves the fix.

| ID | Severity | Status | Evidence | Owner / next action |
| --- | --- | --- | --- | --- |
| QA-NEXT-20260617-001 | Info | Closed | Ledger created before final verification. | Main integration filled command results and artifacts. |
| QA-NEXT-20260617-002 | Medium | Closed | Initial smoke failed because Chrome 149 did not expose CDP with `--headless=new`, then CDP after CLI screenshot could not restart reliably. | Fixed `apps/mac/scripts/smoke-ui.mjs` to extract visible text before screenshots, use `--headless=chrome`, create the target page over CDP, and retry temp profile cleanup. |
| QA-NEXT-20260617-003 | High | Open | Workspace debug `.app` builds but renders a blank window in native screenshots, including direct terminal launch and launch with preview server running. | Follow up in packaging/native runtime work; do not treat rejected native screenshots as passing UI evidence. |

## Residual Risk

| Risk | Current note | Mitigation before signoff |
| --- | --- | --- |
| Native workspace app blank window | Debug `.app` rendered a blank WebView in current environment even after a successful bundle build. | Track separately in packaging/native runtime follow-up before claiming native keyboard QA is complete. |
| Keyboard evidence is browser-backed only | Source hooks and smoke UI pass, but real macOS keyboard path could not be validated because native content was blank. | Re-run native focus QA after the blank-window issue is fixed. |
| Contrast is not proven by screenshots alone | Visual inspection is useful but not a ratio measurement. | Add computed contrast checks if accessibility confidence is required. |
| Locale clipping may appear after copy changes | en-US and zh-CN labels can stress different controls. | Re-run both MVP smoke locales after future copy changes. |
| Concurrent implementation changes may invalidate evidence | Other agents or follow-up commits may update UI after these captures. | Use only fresh artifacts from the final integrated state for release signoff. |

## Agent C Documentation Commands

Commands run by Agent C for this documentation slice:

```bash
sed -n '1,220p' /Users/kartz/Development/Forge/.agents/skills/forge-dev/SKILL.md
git status --short --branch
ls -la docs docs/qa docs/qa/artifacts docs/architecture
sed -n '1,260p' docs/qa/forge-ui-ux-polish-2026-06-17.md
find docs/qa/artifacts/forge-ui-ux-polish-20260617 -maxdepth 2 -type f -print | sort
find docs/qa/artifacts -maxdepth 2 -name README.md -print | sort
sed -n '1,220p' docs/architecture/forge-ui-ux-polish-2026-06-17.md
sed -n '260,520p' docs/qa/forge-ui-ux-polish-2026-06-17.md
sed -n '1,260p' docs/qa/artifacts/forge-ui-ux-polish-20260617/README.md
test -e docs/qa/forge-ui-ux-next-polish-2026-06-17.md
test -e docs/qa/artifacts/forge-ui-ux-next-polish-20260617/README.md
test -e docs/architecture/forge-ui-ux-next-polish-2026-06-17.md
mkdir -p docs/qa/artifacts/forge-ui-ux-next-polish-20260617
git diff --check -- docs/qa/forge-ui-ux-next-polish-2026-06-17.md docs/qa/artifacts/forge-ui-ux-next-polish-20260617/README.md
rg -n "[[:blank:]]$" docs/qa/forge-ui-ux-next-polish-2026-06-17.md docs/qa/artifacts/forge-ui-ux-next-polish-20260617/README.md
git status --short -- docs/qa/forge-ui-ux-next-polish-2026-06-17.md docs/qa/artifacts/forge-ui-ux-next-polish-20260617/README.md docs/architecture/forge-ui-ux-next-polish-2026-06-17.md docs/superpowers/plans/2026-06-17-forge-ui-ux-next-polish.md
sed -n '1,280p' docs/qa/forge-ui-ux-next-polish-2026-06-17.md
sed -n '1,240p' docs/qa/artifacts/forge-ui-ux-next-polish-20260617/README.md
```

Command result summary:

```text
Branch was codex/forge-ui-ux-next-polish.
One unrelated untracked plan file existed outside this documentation scope.
Existing QA and architecture docs were reviewed for format and conventions.
The next-polish ledger and artifact README did not exist before this slice.
The optional next-polish architecture note did not exist and was not created.
`git diff --check` returned clean for the two new QA files.
Trailing-whitespace scan returned no matches; `rg` exited 1 as expected for an empty result.
No build, script suite, smoke test, or real macOS app command was run by Agent C.
```

## Main Integration Follow-Up

Completed in this pass:

```text
web build result and warnings
script-test result and passing script names
fresh en-US MVP smoke screenshot and visible text dump
fresh zh-CN MVP smoke screenshot and visible text dump
fresh responsive smoke screenshots
workspace debug bundle build result and app path
accepted browser/smoke screenshot notes
rejected native screenshot notes
closed or deferred failure log items
final residual risk assessment
```

Deferred:

```text
native keyboard focus path evidence, blocked by blank workspace debug app window
```
