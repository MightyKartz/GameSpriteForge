# Transparent-Gutter Sprite Split Evidence

Date: 2026-06-11

## Scope

This evidence covers Forge's local automatic sprite sheet split mode for sheets
with transparent gutters. It does not use or copy external project code.

## Commands

```bash
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

## Result

```text
Rust transparent split tests: pass
Rust format check: pass
frontend build: pass
script source guards: pass
MVP UI smoke: pass
```

Fresh command output recorded on 2026-06-11:

```text
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check: pass
cargo test sprite_sheet_transparent: 3 passed; 0 failed
npm --workspace apps/mac run build: pass
npm run test:scripts: pass
npm --workspace apps/mac run smoke:ui:mvp: pass
MVP screenshot: apps/mac/smoke-output/forge-workbench-mvp-en-US.png
MVP source dump: apps/mac/smoke-output/forge-workbench-mvp-source-en-US.txt
MVP visible text dump: apps/mac/smoke-output/forge-workbench-mvp-visible-text-en-US.txt
```

## Manual Notes

```text
Manual grid import remains available.
Transparent-gutter import writes raw frames into the same job raw directory.
Downstream normalization, quality report, and export use the existing pipeline.
Non-transparent sheets should use fixed grid mode.
```

## Launcher Behavior

```text
The central empty-workbench Sprite Sheet launcher now selects the file and keeps
the user in Import workflow instead of importing immediately. Once the path is
selected, the side ImportPanel appears and users can choose Grid or Transparent
gutters before clicking Import Selected.
```

## Smoke Guard Note

```text
The internal boundary strings `IMPORT-ONLY MVP` and `Segment Range` are
preserved as non-visible app-shell data attributes for source-level MVP smoke
checks. They are not restored to visible i18n copy.
```
