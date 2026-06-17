# Forge UI/UX Next Polish Architecture Note

Date: 2026-06-17

## Purpose

This pass refines the existing local Forge workbench instead of introducing a
new layout, design system, or product direction. The goal is to make dense
desktop workflow state easier to verify at a glance:

- right rail stages expose source, frame, quality, and output readiness in a
  stable checklist;
- the timeline exposes selected-frame, visible-thumbnail, total-frame, and loop
  range evidence in text, not color alone;
- shared CSS hooks keep small labels readable and keyboard focus visible during
  real macOS QA.

## Source Boundaries

The implementation stays inside the current React/Tauri surface:

- `apps/mac/src/routes/ForgeRoute.tsx` owns workbench stage derivation and right
  rail action semantics.
- `apps/mac/src/components/FrameTimeline.tsx` owns timeline density and selected
  frame evidence without changing public props.
- `apps/mac/src/styles/app.css` owns the visual treatment for checklist rows,
  density chips, readable small text, and focus evidence.
- `scripts/test-workflow-focus-source.mjs` guards the new structural contracts.

## Right Rail Checklist

`StageActionPanel` now renders one compact checklist with stable
`data-stage-check` keys:

```text
source
frames
quality
output
```

Each row exposes `role="listitem"` and a `data-state` value. The empty workbench
still leaves import selection to the central launcher, so the right rail does
not duplicate the primary source CTA.

## Timeline Density

`FrameTimeline` keeps its existing props but adds readable evidence around the
existing thumbnail strip:

- `data-timeline-density` classifies the compression state;
- `data-visible-thumbnails` and `data-total-frames` expose density metadata;
- `timeline-selected-summary` repeats selected frame, visible thumbnail count,
  total frame count, and loop range in text;
- selected thumbnails reference the summary with `aria-describedby`.

## QA Contract

The source guard is intentionally structural. It prevents future UI edits from
removing:

- right rail checklist semantics;
- timeline density and selected-frame text evidence;
- the quieter empty-stage panel;
- readable small-text and strong focus hooks.

Visual and native behavior remains verified through the QA ledger and artifacts
under:

```text
docs/qa/forge-ui-ux-next-polish-2026-06-17.md
docs/qa/artifacts/forge-ui-ux-next-polish-20260617/
```

## Risks

- The source guard proves structural affordances, not pixel-perfect contrast.
  Screenshot and keyboard QA still need final human review.
- Native macOS focus evidence can be hard to see in screenshots, so accepted
  artifacts should include notes about the active control or be paired with
  browser/CDP focus evidence when needed.
- The timeline intentionally limits visible thumbnails to the existing cap; this
  pass improves evidence around density but does not add zooming or virtualized
  timeline behavior.
