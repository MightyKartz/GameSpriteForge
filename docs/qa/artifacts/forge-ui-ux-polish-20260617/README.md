# Forge UI/UX Polish Artifacts - 2026-06-17

This folder is reserved for current-run QA evidence for:

```text
docs/qa/forge-ui-ux-polish-2026-06-17.md
```

Do not place old screenshots here as current evidence. Add only artifacts that
were generated or captured during the 2026-06-17 UI/UX polish verification run
and inspected before acceptance.

## Expected Files

Recommended smoke artifacts:

```text
01-smoke-mvp-en-US.png
02-smoke-mvp-zh-CN.png
03-smoke-visible-text-en-US.txt
04-smoke-visible-text-zh-CN.txt
05-smoke-responsive-1120-en-US.png
06-smoke-responsive-1280-en-US.png
07-smoke-responsive-1568-en-US.png
```

Recommended real macOS app artifacts:

```text
10-real-app-empty-workbench-en-US.png
11-real-app-empty-workbench-zh-CN.png
12-real-app-focus-import-controls.png
13-real-app-focus-timeline-controls.png
14-real-app-focus-preview-inspection-toggle.png
15-real-app-focus-export-advanced-collapsed.png
16-real-app-settings-focus-language.png
```

## Accepted In This Run

Current-run smoke artifacts:

```text
01-smoke-mvp-en-US.png
02-smoke-mvp-zh-CN.png
03-smoke-visible-text-en-US.txt
04-smoke-visible-text-zh-CN.txt
05-smoke-responsive-1120-en-US.png
06-smoke-responsive-1280-en-US.png
07-smoke-responsive-1568-en-US.png
```

Current-run app and keyboard artifacts:

```text
10-real-app-empty-workbench.png
12-real-app-focus-import-controls.png
13-real-app-focus-central-controls.png
14-browser-keyboard-focus-import-button.png
```

The native focus screenshots are supporting evidence only; the browser keyboard
capture has the clearer focus ring and active-element trail.

## Acceptance Rules

For each accepted screenshot or text dump, update the QA ledger with:

```text
command used
app path or browser/smoke mode
locale
viewport or window size
what the artifact proves
known limits
```

Reject artifacts that are blank, loading, cropped, from the wrong app window,
or from a stale installed app when the workspace debug bundle is required.
