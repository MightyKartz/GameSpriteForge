# Forge Export Footer Action Dedupe Plan

Date: 2026-06-07

## Goal

After a pack has exported, the bottom status bar should not show duplicated export actions that compete with the timeline and become clipped. Export result actions should live in the right export panel, while the footer remains a compact status surface.

## UX Decisions

1. Keep pre-export footer actions because they help users move through import, extract, and process.
2. Remove duplicate post-export footer buttons for open folder, validate re-import, and re-export.
3. Replace the post-export footer action cluster with a compact status chip.
4. Keep right-side export panel actions unchanged as the canonical place for export result actions.
5. Preserve footer pipeline steps, frame metadata, quality status, and app version.

## Implementation Steps

1. Replace the `exportOutput` footer action branch with a `footer-export-status` chip.
2. Add concise Chinese/English i18n for exported and validation-pending footer status.
3. Add CSS so the chip truncates cleanly and never overlaps the timeline.
4. Add source-level regression checks.
5. Build, install, and verify the exported state with the real macOS UI.

## Acceptance Criteria

- After export, the lower-left corner no longer shows clipped action buttons.
- The footer shows export state text only.
- Right panel still offers Open Export Folder, Validate Re-import, and Re-export.
- Tests, build, install, signing verification, and real UI verification pass.
