# Forge UI/UX Polish Architecture Note

Date: 2026-06-17

## Goal

Tighten the Forge workbench flow without changing the product model: make the
timeline, preview inspection state, and next-step rail easier to scan while
preserving the local-first Tauri workflow.

## Implementation Boundaries

- `FrameTimeline.tsx` now exposes timeline evidence as semantic list items,
  stable `data-evidence` keys, loop start/end markers, and selected-frame
  state through `aria-current`, `title`, and stable frame data attributes.
- `CanvasPreview.tsx` now exposes the active preview mode, before/after
  processing state, inspection overlay state, overlay parts, and inspection
  toggle control relationship through stable data and ARIA attributes.
- `ForgeRoute.tsx` keeps the central import launcher as the only empty-state
  primary CTA. The right rail remains the stage explainer and no longer
  duplicates the source-import button in the empty state.
- `app.css` keeps the existing dark desktop workbench language, but increases
  dense preview/timeline state text to readable 12px targets and adds visual
  treatment for inspection mode, loop evidence, loop markers, and selected
  frame state.
- `scripts/test-workflow-focus-source.mjs` guards these source-level UI
  invariants so future refactors do not silently remove the accessibility and
  workflow evidence hooks.

## QA Evidence

Primary evidence lives in:

```text
docs/qa/forge-ui-ux-polish-2026-06-17.md
docs/qa/artifacts/forge-ui-ux-polish-20260617
```

The final pass covered build, script tests, MVP smoke in English and Chinese,
responsive smoke, workspace debug app launch, and a keyboard Tab focus capture
against the built UI.

## Known Limits

- Native macOS focus screenshots were captured but were not visually conclusive
  enough to claim full keyboard coverage.
- Automated color contrast ratios were not measured in this pass.
- `npm install` reported 2 high severity audit items; dependency remediation
  should be handled separately from this UI PR.
