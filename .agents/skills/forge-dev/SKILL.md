---
name: forge-dev
description: Use when working on Forge, the local Tauri macOS sprite animation workbench, especially import, sprite sheet, export, real UI QA, release packaging, or project documentation tasks.
---

# Forge Dev

## Overview

Forge is a local-first Tauri macOS app for turning video, PNG sequences, sprite sheets, and `.gsfpack` folders into game-ready sprite animation assets. Keep product behavior local, inspectable, and verified through the real app when UI behavior changes.

## Core Rules

- Prefer existing Forge patterns in `apps/mac/src`, `packages/core`, `scripts`, and `docs/qa`.
- Do not copy external project source. External tools may inform product research only; Forge behavior must be implemented in Forge's own local Rust/Tauri architecture.
- Keep sprite sheet intake as: choose file, configure `固定网格` or `透明间隔`, then import.
- Keep MCP/CLI tooling out of the product UI; use it only for development and QA evidence.
- Write QA findings under `docs/qa/`; put screenshots and generated fixtures under `docs/qa/artifacts/`.

## Real UI QA

When testing the macOS app UI, make sure you are testing the workspace build, not a stale installed app with the same bundle id.

Preferred flow:

```bash
npm --workspace apps/mac run tauri -- build --debug --bundles app
```

Launch and test:

```text
/Users/kartz/Development/Forge/target/debug/bundle/macos/Game Sprite Forge.app
```

Before relying on Computer Use evidence, close stale `/Applications/Game Sprite Forge.app` windows or explicitly target the workspace bundle path. Record the app path, UI driver, fixture path, and result in the QA note.

## Verification

Choose the smallest reliable set for the change, but UI/import/export work usually needs:

```bash
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

For Rust media-processing changes, add the focused Cargo test, for example:

```bash
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
```

For release work, use the existing packaging and verification scripts instead of hand-rolling signing or notarization steps.
