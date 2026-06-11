# Game Sprite Forge

[中文 README](README.zh-CN.md)

Game Sprite Forge is a local macOS workbench for turning video clips, PNG frame sequences, sprite sheets, and `.gsfpack` folders into game-ready 2D sprite animation assets.

The current test version focuses on an import-first, local-only pipeline: choose source media, extract a controlled frame set, inspect frames, remove or normalize backgrounds, run quality checks, export a sprite asset pack, generate a Godot helper project, and validate the exported pack by re-importing it. It does not require cloud services, accounts, AI provider setup, marketplace flows, or online registries.

## Project Introduction

Game Sprite Forge is built for solo game developers, 2D asset makers, technical artists, and small teams who already have local animation source material and need a deterministic way to package it for game engines.

The app helps with:

- importing videos, PNG sequences, sprite sheets, and `.gsfpack` folders;
- extracting and inspecting animation frames;
- processing green-screen or transparent-frame sources;
- normalizing frames to a stable canvas and foot anchor;
- checking quality signals such as bounds, loops, alpha edges, and frame consistency;
- exporting PNG frames, sprite sheets, manifests, atlas data, preview material, and `.gsfpack` packages;
- exporting a minimal Godot project helper for a generated pack;
- validating exported packs through local re-import.

## Current Test Release

The current public test release is intended for macOS Apple Silicon.

Download the latest pre-release from the [GitHub Releases](https://github.com/MightyKartz/GameSpriteForge/releases) page.

Current test tag:

```text
v0.1.0-test.2
```

The release is a test build. Expect rough edges, but the core local import -> extract -> process -> quality -> export -> Godot helper -> validate loop is implemented.

## Features

- Local-first macOS desktop app built with Tauri, React, TypeScript, and Rust.
- Video intake through `ffmpeg`/`ffprobe`.
- Target-frame extraction controls, including a 24-frame flow for short animation clips.
- PNG sequence import with multiline path support.
- Sprite sheet import with fixed-grid and transparent-gutter splitting.
- `.gsfpack` import, export, validation, and re-import.
- Stage-aware workbench UI with one primary next action per workflow stage.
- Timeline evidence for target frames, actual frames, selected range, sampling interval, loop range, and selected frame.
- Preview modes for raw, normalized, inspection, and export review states.
- Quality report panel with actionable recovery paths.
- Godot project helper export for testing Forge output inside Godot.
- Source-level guard scripts and real UI QA evidence for release hardening.

## Repository Layout

```text
apps/mac/                 Tauri macOS app and React UI
packages/core/            Rust media-processing core
packages/pack/            .gsfpack schema and pack logic
schemas/                  JSON schemas for exported data
examples/                 Sample inputs and engine helper examples
scripts/                  QA, packaging, and release verification scripts
docs/                     Architecture notes, plans, and QA evidence
.agents/skills/forge-dev/ Project-specific agent development guidance
```

## Development

Prerequisites:

- macOS
- Node.js and npm
- Rust toolchain
- Tauri prerequisites for macOS
- `ffmpeg` and `ffprobe` for video workflows

Install dependencies:

```bash
npm install
```

Run the frontend dev server:

```bash
npm run dev
```

Run the Tauri app in development:

```bash
npm --workspace apps/mac run tauri -- dev
```

Build the macOS app bundle:

```bash
npm --workspace apps/mac run tauri -- build --debug --bundles app
```

## Verification

Common checks:

```bash
npm --workspace apps/mac run build
npm run test:scripts
npm --workspace apps/mac run smoke:ui:mvp
```

For Rust media-processing changes:

```bash
cargo fmt --manifest-path Cargo.toml --all -- --check
cargo test --manifest-path Cargo.toml sprite_sheet_transparent
```

For release packages, use the existing release verification scripts in `scripts/`.

## Notes

- Transparent-gutter sprite sheet intake is implemented inside Forge's local Rust/Tauri pipeline.
- The product intentionally avoids cloud or account-dependent features in the first test version.
- The current repository is source-first. Release candidate archives and DMGs are published as GitHub Release assets rather than committed to Git history.

## License

No open-source license has been selected yet. Until a license is added, all rights are reserved by the project owner.
