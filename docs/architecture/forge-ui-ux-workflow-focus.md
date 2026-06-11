# Forge UI/UX Workflow Focus

Date: 2026-06-11

## Purpose

Forge's local media pipeline is now capable enough that the interface should
shift from "show every internal state" to "guide the next correct user decision."

This document defines the UI/UX direction for the next local-app slice.

## Product Type

Forge is a local desktop creative/developer tool:

```text
creative input: video, PNG sequence, sprite sheet, .gsfpack
technical processing: frame extraction, matting, normalization, quality metrics
developer output: .gsfpack, PNG/JSON, Godot project helper
```

The interface should feel like a precise editor, not a marketing page and not a
debug dashboard.

## Design Principles

### 1. One Current Job

Every screen state should answer:

```text
What do I have?
What should I do next?
What will happen if I click the primary action?
```

Avoid showing several equivalent "next" buttons in different regions.

### 2. Stage-Aware Right Rail

The right rail should be the current stage's action panel. It should not always
show every possible quality and export concern.

Target stages:

```text
Import: choose source, sample path, recent pack import
Frames: selected range, target frames, extraction evidence
Process: background removal, anchor/position readiness, quality gate
Sheet: sheet preview and layout only after processed frames exist
Export: format, name, output location, export
Post-export: open folder, validate re-import, export Godot project
```

### 3. Timeline As Evidence

The timeline should help users validate frame selection, not just list frames.

It should expose:

```text
target frame count
actual extracted count
selected time range
sampling interval
loop start and loop end
selected frame index
```

Thumbnails can stay compact, but the currently selected frame and loop endpoints
need stronger visual priority.

### 4. Preview Modes

The central preview should clearly name what is being viewed:

```text
Raw frame
Processed frame
Normalized frame
Inspection overlay
Export preview
```

Data overlays should be mode-specific. In non-inspection modes, the sprite
itself must remain dominant.

### 5. Progressive Disclosure

Advanced settings belong behind stable disclosure controls:

```text
sheet columns
sheet padding
sheet margin
license
creator name
loop export details
Godot helper/project internals
```

The main flow should remain:

```text
Import -> Extract -> Process -> Review -> Export -> Validate
```

### 6. Accessible Dark Utility

Forge can keep the dark professional theme, but the token system should make
readability measurable.

Target token direction:

```text
background: near-black slate
surface: dark slate with clear borders
primary inspection accent: cyan/blue
success action: green only for completed/positive actions
warning: amber with icon/text
error: red with icon/text
primary text: high-contrast off-white
secondary text: lighter slate, not charcoal-on-charcoal
```

## Target Layout Model

```text
Left rail: product navigation and compact current source summary
Top bar: workflow progress and current route context
Center: preview/editor canvas
Bottom: timeline and frame evidence
Right rail: current step action panel
Footer: compact status only
```

## UX State Model

Recommended canonical states:

```text
empty
source_selected
frames_ready
processed_ready
quality_ready
export_ready
exported_unvalidated
validated
godot_project_ready
blocked
running
```

All visible summaries should derive from this state model rather than separately
guessing from partial component props.

## Implementation Update

Implemented on 2026-06-11:

```text
apps/mac/src/forgeViewModel.ts defines WorkbenchStage and deriveWorkbenchStage.
apps/mac/src/routes/ForgeRoute.tsx derives workbenchStage once and gates the right rail from it.
StageActionPanel owns early-stage primary actions.
FrameTimeline renders timeline-evidence-strip for target, actual, range, sampling, loop, and selected frame.
CanvasPreview labels preview mode and exposes an inspection overlay toggle.
The footer shows compact stage status instead of duplicate workflow action buttons.
scripts/test-workflow-focus-source.mjs guards the core UI/UX constraints.
docs/qa/forge-ui-ux-workflow-focus-evidence-2026-06-11.md records verification.
```

Remaining design follow-ups:

```text
make loop endpoints visually stronger on the timeline track itself
complete keyboard tab-order QA in the installed app
record full installed-app local video screenshots from import through Godot export
```

## Acceptance Criteria

A UI/UX hardening slice is ready when:

```text
only one primary CTA is visible per workflow stage
right rail content changes by workflow stage
timeline shows target vs actual extraction evidence
preview mode label is always visible
inspection overlays can be toggled or are limited to inspection mode
advanced metadata remains collapsed unless user opens it or it needs attention
keyboard users can reach all primary and secondary actions
dark-mode contrast meets the documented text targets
real installed-app screenshots are recorded under docs/qa
```

## References

```text
docs/qa/forge-ui-ux-review-2026-06-11.md
docs/superpowers/plans/2026-06-11-forge-ui-ux-workflow-focus.md
```
