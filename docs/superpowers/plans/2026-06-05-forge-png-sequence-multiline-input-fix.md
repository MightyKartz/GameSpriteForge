# Forge PNG Sequence Multiline Input Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the PNG sequence import field truly support newline-separated paths in the real macOS UI while preserving comma-separated import.

**Architecture:** Keep parsing behavior unchanged in `ForgeRoute`; it already splits PNG paths on newlines or commas. Fix the UI contract at the source by letting `ImportPanel` render the PNG sequence path control as a textarea while keeping other path controls as single-line inputs.

**Tech Stack:** Tauri 2, React 18, TypeScript, Vite, source-level Node smoke test, `computer-use` MCP for real UI verification.

---

### Task 1: Add Source-Level Regression Test

**Files:**
- Create: `scripts/test-import-panel-source.mjs`
- Modify: `package.json`

- [ ] **Step 1: Write the failing source test**

Create `scripts/test-import-panel-source.mjs`:

```js
import { readFileSync } from "node:fs";

const source = readFileSync("apps/mac/src/components/ImportPanel.tsx", "utf8");

function assertContains(needle, message) {
  if (!source.includes(needle)) {
    throw new Error(message);
  }
}

assertContains(
  'detail="newline or comma separated PNGs"',
  "ImportPanel must keep the PNG sequence affordance text.",
);
assertContains(
  "label=\"Import PNG Sequence\"",
  "ImportPanel must keep the PNG sequence import action.",
);
assertContains(
  "multiline",
  "PNG sequence PathAction must opt into multiline input.",
);
assertContains(
  "<textarea",
  "PathAction must render a textarea for multiline path input.",
);
assertContains(
  "className=\"path-input path-input-multiline\"",
  "Multiline path input must use the multiline path-input class.",
);

console.log("PASS import panel source test");
```

- [ ] **Step 2: Wire the test script**

Add the source test to root `package.json` `test:scripts` after the manual QA fixture test:

```json
"test:scripts": "node --check scripts/prepare-manual-qa-fixtures.mjs && node --check scripts/test-manual-qa-fixtures.mjs && node --check scripts/test-import-panel-source.mjs && node scripts/test-manual-qa-fixtures.mjs && node scripts/test-import-panel-source.mjs && bash -n scripts/verify-release-package.sh scripts/package-release-candidate.sh scripts/notarization-preflight.sh scripts/test-notarization-preflight.sh scripts/run-pre-manual-pipeline-evidence.sh && bash scripts/test-notarization-preflight.sh"
```

- [ ] **Step 3: Verify RED**

Run:

```bash
node scripts/test-import-panel-source.mjs
```

Expected: FAIL with `PNG sequence PathAction must opt into multiline input.`

### Task 2: Implement Multiline PNG Sequence Field

**Files:**
- Modify: `apps/mac/src/components/ImportPanel.tsx`
- Modify: `apps/mac/src/styles/app.css`

- [ ] **Step 1: Add multiline prop to PNG sequence PathAction**

In `apps/mac/src/components/ImportPanel.tsx`, add `multiline` to the PNG sequence `PathAction`:

```tsx
        <PathAction
          detail="newline or comma separated PNGs"
          disabled={disabled}
          icon={<Images size={17} />}
          label="Import PNG Sequence"
          multiline
          onAction={onImportFrames}
          onChoose={onChooseFrameFiles}
          onValueChange={onFrameSequenceInputChange}
          placeholder="/path/frame_001.png"
          value={frameSequenceInput}
        />
```

- [ ] **Step 2: Render textarea only for multiline PathAction**

Extend the `PathAction` props with `multiline?: boolean`, default it to `false`, and render this controlled textarea when `multiline` is true:

```tsx
        {multiline ? (
          <textarea
            className="path-input path-input-multiline"
            disabled={disabled}
            onChange={(event) => onValueChange(event.target.value)}
            placeholder={placeholder}
            rows={3}
            spellCheck={false}
            value={value}
          />
        ) : (
          <input
            className="path-input"
            disabled={disabled}
            onChange={(event) => onValueChange(event.target.value)}
            placeholder={placeholder}
            value={value}
          />
        )}
```

- [ ] **Step 3: Style multiline input without layout churn**

In `apps/mac/src/styles/app.css`, keep `.path-input` shared and add:

```css
.path-input-multiline {
  min-height: 58px;
  height: auto;
  padding-block: 6px;
  line-height: 1.35;
  resize: vertical;
}
```

### Task 3: Verify and Reinstall

**Files:**
- None beyond Task 1 and Task 2.

- [ ] **Step 1: Verify GREEN**

Run:

```bash
node scripts/test-import-panel-source.mjs
npm run test:scripts
npm --workspace apps/mac run build
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
```

Expected: all pass.

- [ ] **Step 2: Build and install the updated app**

Run:

```bash
npm --workspace apps/mac run tauri -- build
```

Then install from:

```text
/Users/kartz/Development/Forge/target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
```

### Task 4: Real UI Regression with computer-use

**Files:**
- None.

- [ ] **Step 1: Confirm textarea in real UI**

Use `computer-use` `get_app_state` for `Game Sprite Forge`. Expected: `Import PNG Sequence` field appears as a multiline/settable text field or textarea-like accessibility field.

- [ ] **Step 2: Import newline-separated PNG paths**

Paste these newline-separated paths into the PNG sequence field:

```text
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_001.png
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_002.png
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_003.png
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_004.png
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_005.png
/Users/kartz/Development/Forge/examples/inputs/manual-qa/png-sequence/frame_006.png
```

Click `Import Selected` for PNG sequence. Expected: `Imported 6 PNG frames ...`.

- [ ] **Step 3: Process, export, validate**

Click `Process & Quality`, then `Export Pack`, then `Validate Re-import`.

Expected:

```text
Processed 6 frames with Game Ready verdict.
Exported <pack>.gsfpack.
Validated and re-imported <pack> with 6 frames.
```

- [ ] **Step 4: Verify exported files**

Check the latest export directory contains:

```text
frames/
sprite_sheet.png
atlas.json
manifest.json
quality-report.json
preview.gif
godot_import.json
*.gsfpack/
```

---

## Self-Review

- Spec coverage: The plan fixes the observed mismatch between newline-support copy and real UI behavior, and preserves comma-separated paths because parsing remains unchanged.
- Placeholder scan: No placeholder tasks; every step has exact files and commands.
- Type consistency: The new `multiline?: boolean` prop is local to `PathAction` and only used by the PNG sequence action.
