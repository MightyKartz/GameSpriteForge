# Product

## Register

product

## Users

Forge V1 is for solo game developers, 2D asset makers, technical artists, and small game teams on macOS. They already have local videos, PNG frame sequences, sprite sheets, or `.gsfpack` folders, and they need a deterministic local workflow that turns those sources into game-ready 2D animation assets.

Users are usually in a focused production context: checking frames, fixing background removal, validating anchors and loops, exporting packs, and confirming that the exported asset can be re-imported and used in an engine.

## Product Purpose

Game Sprite Forge is a local macOS workbench for importing existing media and packaging it into validated 2D sprite animation assets. V1 focuses on an import-only pipeline: import source media, inspect and process frames, compute quality, export frames/sprite sheet/manifest/atlas/preview GIF/`.gsfpack`, and validate the exported pack through re-import.

Success means a user can process a short local character animation end to end without cloud services, AI provider setup, online registry, marketplace flows, or account-dependent features.

## Current MVP Status

As of 2026-06-06, the project has a working Tauri + React macOS MVP installed locally at `/Applications/Game Sprite Forge.app`.

Implemented:

- Local source intake for video, PNG sequences, sprite sheets, and `.gsfpack` folders.
- ffmpeg/ffprobe dependency check with user-configurable paths and deterministic clean-environment QA controls.
- Video probe, frame extraction, chroma preview, batch chroma processing, square-bottom normalization, manual foot anchor support, loop range selection, quality report generation, and export.
- `.gsfpack` create/import/validate/re-import flow backed by local schemas and Rust tests.
- UI truth pass: demo data is labeled, unavailable workflow steps are locked, export readiness lists blockers, live workspaces show a compact Run Summary, and failed pipeline states provide recovery actions.
- Developer ID signed, notarized, stapled DMG build and local `/Applications` install.
- Local Pack Library refresh, inspect, validate, open, and re-import for exported `.gsfpack` folders.

Current release status:

- The current release package is `release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized`.
- Current DMG SHA-256: `0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42`.
- Current release zip SHA-256: `41831d5fa1155ad5620cf0f3e1351b8c6953d8d45d52276c4db97750c5dcb7ce`.
- Notarization submission `834c0445-1c45-4201-88d5-a2c99c008714` is accepted, the DMG is stapled, Gatekeeper accepts the DMG and mounted app as `source=Notarized Developer ID`, mounted-DMG launch verification passed, and the synchronized `/Applications` app is stapled and Gatekeeper-accepted.

## Brand Personality

Technical, trustworthy, and workbench-like.

The product should feel like a focused local desktop tool for game asset production. It should be dense, calm, and operational rather than playful, decorative, marketing-heavy, or cloud-generator-like.

## Anti-references

- Generic SaaS dashboards with oversized hero sections, marketing cards, or vague productivity copy.
- AI generation surfaces that imply one-click final game-ready assets before quality checks.
- Marketplace, creator publishing, cloud upload, account, BYOK, hosted credits, or online registry affordances in the MVP.
- Game-engine chrome that looks powerful but exposes controls that do not perform real local actions.
- Decorative complexity that competes with frame inspection, quality metrics, and export readiness.

## Design Principles

- Make real pipeline state obvious: every visible control should either perform a local action now or clearly say why it is unavailable.
- Keep the primary loop prominent: Import -> Extract -> Process/Quality -> Export -> Validate/Re-import.
- Prefer local-first confidence: no AI, account, cloud, marketplace, or publishing affordances in the MVP.
- Show sample data only when it is unmistakably labeled as sample data.
- Let visual polish support scanning, comparison, and repeated action rather than decoration.

## Accessibility & Inclusion

Target WCAG AA for text contrast, focus visibility, keyboard access, and non-color-only state signaling. The app should remain readable in dense desktop layouts, provide clear disabled and error states, support reduced motion, and avoid relying on hover-only explanations for critical workflow state.
