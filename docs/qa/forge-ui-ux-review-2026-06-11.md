# Forge UI/UX Review

Date: 2026-06-11

## Scope

This review uses the installed local app screenshots provided during manual
testing. It records UI/UX issues only; it does not implement fixes.

Screenshots reviewed:

```text
/Users/kartz/Desktop/截屏2026-06-11 14.29.32.png
/Users/kartz/Desktop/截屏2026-06-11 14.29.39.png
/Users/kartz/Desktop/截屏2026-06-11 14.29.43.png
/Users/kartz/Desktop/截屏2026-06-11 14.29.47.png
```

Skill used:

```text
ui-ux-pro-max
```

Design-system lookup:

```bash
python3 /Users/kartz/.codex/skills/ui-ux-pro-max/scripts/search.py \
  "desktop sprite animation editor local developer tool dark dense workflow" \
  --design-system -p "Game Sprite Forge" -f markdown
```

Relevant guidance:

```text
Product type: desktop developer/creative tool
Style: dark professional utility
Priority: accessibility, interaction clarity, performance/loading feedback, layout hierarchy
Color direction: code-dark surfaces, cyan/blue inspection accents, green only for completed positive actions
```

## Overall Finding

Forge now has the functionality of a capable local sprite pipeline, but the UI
still feels like a diagnostic cockpit. The main issue is not visual polish alone;
it is that too many panels explain too many workflow states at the same time.

The next UI slice should focus on:

```text
single obvious next action
one workflow progress model
right rail as the current-step action panel
clear preview mode labels
timeline evidence that helps decide whether extraction is correct
progressive disclosure for advanced/export metadata
stronger contrast and keyboard accessibility
```

## Findings

### P1: Workflow State Is Repeated In Too Many Places

Observed:

```text
Top workflow tabs, left run summary, right quality/export panel, bottom action bar,
and footer status all report process state at the same time.
```

Impact:

Users must compare multiple state indicators to decide what to do next. This is
especially visible after video import, where the app shows "source selected",
"waiting for frames", "next step", bottom actions, and workflow tabs together.

Recommendation:

Keep one canonical progress model. Prefer top workflow tabs plus one stage CTA.
Convert the bottom bar into compact status only, or remove duplicate workflow
buttons from it.

### P1: Primary Action Is Not Singular

Observed:

At different moments, the user can see several competing actions:

```text
抽取帧
处理并检查质量
导出资源包
验证重新导入
打开导出文件夹
更改位置
```

Impact:

The correct next action is present, but it is not always visually dominant. This
slows down manual testing and will confuse new users.

Recommendation:

Each stage should have exactly one primary CTA. Secondary actions should remain
available but visually subordinate and placed after the primary path.

### P1: Right Rail Is Doing Too Much

Observed:

The right rail mixes quality status, export readiness, output folder selection,
pack naming, export formats, validation, and generated output details.

Impact:

The right rail becomes a dense warehouse rather than a decision panel. Export
settings appear before users have enough context to use them.

Recommendation:

Make the right rail stage-aware:

```text
Import stage: source guidance and sample path
Frames stage: extraction summary and next action
Process stage: quality gate and cleanup action
Export stage: format, name, output location, export button
Post-export stage: open folder, validate, Godot project export
```

### P1: Timeline Does Not Yet Answer "Is 24 Frames Correct?"

Observed:

The timeline shows many markers and thumbnails, but the extraction decision is
not summarized in one readable place.

Impact:

Users can see that 24 frames exist, but not why that is correct for the selected
range, FPS, and interval.

Recommendation:

Add an extraction evidence strip:

```text
Target: 24 frames
Actual: 24 frames
Range: 00:00.00-00:01.00
Sampling: every 1 frame
Loop: start frame 1, end frame 24
```

Also make selected thumbnails larger or provide a hover/keyboard preview.

### P2: Preview Inspection Overlays Compete With The Character

Observed:

The bbox, anchor line, and center line are valuable, but they are visually strong
and can dominate the actual sprite.

Impact:

The overlay helps technical inspection but can make the character harder to
judge visually, especially after processing.

Recommendation:

Add preview modes:

```text
Raw
Processed
Normalized
Inspection
```

Only show strong bbox/anchor/center overlays in Inspection mode. In other modes,
use softer overlay opacity or hide overlays by default.

### P2: Export Formats Look Like Disabled Cards Before Export

Observed:

PNG + JSON and Godot helper cards are visible even when export is not ready.

Impact:

Users may interpret the format cards as choices that are currently broken or
disabled, rather than output types that will be available later.

Recommendation:

Move format selection into the Export stage. Before export, show a simpler
"Export unlocks after quality" empty state.

### P2: Advanced Metadata Appears Too Close To The Main Path

Observed:

Pack metadata, sheet columns, padding, margin, and loop settings can appear in
the primary rail.

Impact:

New users see implementation details before they have completed the main
pipeline. The export step feels more complicated than it needs to be.

Recommendation:

Keep advanced metadata collapsed unless:

```text
the user explicitly opens it
validation fails because of metadata
the user is on an Advanced export subpanel
```

### P2: Dark UI Contrast And Text Size Need A Pass

Observed:

Several secondary labels are very small and low contrast on dark surfaces.

Impact:

The UI feels dense and can become tiring to read. It also risks failing
accessibility checks for normal text contrast.

Recommendation:

Audit text tokens:

```text
body text >= 12px minimum for dense desktop panels
primary text contrast >= 4.5:1
secondary text contrast >= 3:1
disabled state visibly different from actionable secondary state
focus ring visible on all keyboard targets
```

## Recommended Priority

```text
P1: single primary action model
P1: stage-aware right rail
P1: timeline extraction evidence
P2: preview mode and softer overlays
P2: export format progressive disclosure
P2: contrast, focus, and keyboard pass
```

## Non-Goals

Do not use this UI/UX slice to add:

```text
AI generation
cloud workflows
marketplace publishing
online registry
MCP product surface
new engine exporters beyond current Godot project support
```
