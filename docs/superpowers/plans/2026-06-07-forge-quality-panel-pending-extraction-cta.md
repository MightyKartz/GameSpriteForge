# Forge Quality Panel Pending Extraction CTA Plan

Date: 2026-06-07

## Goal

When a video source is imported but frames have not been extracted yet, the right quality panel should not compete with the left extraction control. The left rail owns the primary "Extract frames" action; the quality panel should explain what will happen after frames exist.

## UX Decisions

1. Keep the left `Extract about N frames` button as the only primary CTA in the pre-extraction state.
2. Change the quality panel title from a generic no-report state to a waiting-for-frames state.
3. Remove the right-side `Extract & Quality Check` button before frames exist.
4. Keep the quality checklist preview so users understand what will be checked later.
5. Preserve direct processing controls after raw frames exist.

## Implementation Steps

1. Add pending-extraction copy for the quality panel.
2. Gate the quality panel action button so it is hidden while source extraction is pending.
3. Add a compact waiting note for the pending-extraction quality panel.
4. Update source-level regression tests to prevent the competing CTA from returning.
5. Build, install, and verify with the real UI after video import.

## Acceptance Criteria

- Before extraction, the right panel does not show `Extract & Quality Check`.
- Before extraction, the right panel says quality is waiting for frames.
- After extraction, `Process & Quality` remains available.
- The left extraction CTA remains the clear primary next action.
