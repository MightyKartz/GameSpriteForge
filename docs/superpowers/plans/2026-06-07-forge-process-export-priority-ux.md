# Forge Process And Export Priority UX Plan

Date: 2026-06-07

## Goal

When processing and quality checks are complete, the right inspector should stop feeling like a quality-debug screen and become an export-ready panel. Users should be able to confirm the output location, pack name, animation name, and export action without scrolling.

## UX Decisions

1. Keep the existing dark desktop workbench visual language and component density.
2. Collapse quality details after a usable quality report exists; keep the verdict and warning count visible.
3. Promote export readiness, output location, pack name, animation name, and the primary export action above secondary target/status content.
4. Keep advanced metadata and sheet settings behind the existing disclosure.
5. Preserve all existing export behavior and validation actions.

## Implementation Steps

1. Add a compact mode to the quality inspector for processed/export-ready states.
2. Add an export-ready priority layout to the inspector panel and export panel.
3. Reorder export panel content so key settings and the primary export action appear before target/status details.
4. Add focused i18n copy and source-level regression assertions.
5. Run script tests, production build, smoke test, local install, signing verification, and real UI verification.

## Acceptance Criteria

- After processing completes, quality details are summarized and no longer dominate the right column.
- `导出资源包`, `默认导出位置`, `资源包名称`, and `动画名称` are visible without scrolling in the right column.
- Export behavior, re-export behavior, output-folder choosing, and validation remain wired.
- Chinese UI copy stays concise in the narrow right inspector.
