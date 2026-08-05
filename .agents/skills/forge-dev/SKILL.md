---
name: forge-dev
description: Use when working on Forge, the agent-first Rust CLI for consistent game assets and Godot delivery, including Providers, Style Locks, packs, engine installation, release packaging, or project documentation.
---

# Forge Dev

## Overview

Forge is a local-first Rust CLI for generating and processing characters, icon sets, prop sets, and `.gsfpack` assets for Godot. Keep behavior agent-readable, inspectable, model-neutral, and verified through the real CLI and Godot headless installation.

## Core Rules

- Prefer existing Forge patterns in `packages/cli`, `packages/core`, `packages/providers`, `packages/pack`, `scripts`, and `docs/qa`.
- Do not copy external project source. External tools may inform product research only; Forge behavior must be implemented in Forge's own local Rust architecture.
- Keep the public command name `forge`, one-JSON-value stdout contract, diagnostics on stderr, durable jobs, and single-use write plans.
- Keep Providers locked per job; never put credentials into plans, jobs, packs, logs, or ordinary output.
- Keep desktop and MCP sources out of default builds and releases. Do not delete their retained sources without explicit authorization.
- Write QA findings under `docs/qa/`; put screenshots and generated fixtures under `docs/qa/artifacts/`.

## CLI and Godot QA

Use the workspace binary at `/Users/kartz/Development/Forge/target/debug/forge` or `target/release/forge`, not a stale installed command. Redirect JobStore and PlanStore for tests. Verify generated packs with `forge pack validate`, install into a temporary Godot 4.6.x project, and reject embedded `PackedByteArray` image resources or `.tres/.tscn` files at or above 1 MiB.

## Verification

Choose the smallest reliable set for the change. Product and release work usually needs:

```bash
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml
bash scripts/test-cli-product.sh
bash scripts/test-cli-installer.sh
```

For Rust media-processing changes, add the focused Cargo test, for example:

```bash
cargo fmt --manifest-path /Users/kartz/Development/Forge/Cargo.toml --all -- --check
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml sprite_sheet_transparent
```

For release work, use the tag-driven GitHub workflow and installer contract. The public README exposes only the command-line installer; do not add Homebrew or manual installation paths.
