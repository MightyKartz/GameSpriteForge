# Forge model-neutral Provider V1 QA — 2026-08-02

## Scope

This pass validates the first model-independent generation path: schema V3 top-down Character Packs, direct xAI OAuth/API integration, offline fixture generation, exact video sampling, quality gating, `.gsfpack` export, MCP exposure, and Godot installation. The desktop UI remains an optional review/recovery surface and was not expanded for Provider login.

## Implemented boundary

- Core owns neutral Provider requests/results/errors and all deterministic asset processing.
- `forge_providers` implements `xai` and `fixture` without Grok Build CLI.
- Credential kind is explicit: `api_key`, `oauth_device_code`, or `none`.
- xAI OAuth uses trusted OIDC Discovery, Device Code polling, Refresh Token rotation, and OS credential storage.
- CLI owns interactive login/logout; MCP exposes only Provider list/check and generation planning.
- Provider files are constrained to the JobStore, format checked, size bounded, and SHA-256 hashed before processing.
- V3 locks one Provider and creates `idle`, `walk_up`, `walk_right`, and `walk_down`; left playback is the horizontally flipped right clip.
- A failed action receives at most one regeneration. Export remains blocked unless every action is `game_ready`.

## Automated evidence

### Full Rust workspace

Command: `cargo test --workspace`

Result: passed. This includes Core, pack schema, Tauri backend, sample video pipeline, ten Provider unit tests, the Provider Character Pack contract, and doc tests.

Provider/OAuth cases include:

- untrusted OAuth endpoint rejection;
- `authorization_pending`, `slow_down`, denial, expiry, cancellation;
- Refresh Token rotation and no secret text in returned errors;
- owner-only fallback credential storage;
- OAuth Bearer origin pinning;
- one HTTP 401 refresh retry and immediate local image materialization;
- 403 entitlement classification;
- bounded 429 retry;
- asynchronous video submit/pending/done polling;
- malformed image rejection.

### Offline generation and engine closure

Test: `cargo test -p providers --test character_generation_contract`

Result: passed against local `ffmpeg`, `ffprobe`, and Godot 4.

The test starts with a schema V3 plan and the deterministic `fixture` Provider, generates four animated sources, extracts 32 exact frames, passes every per-animation quality gate, validates the exported `.gsfpack`, verifies schema V3 Provider provenance, and scans the Provider manifest for token fields. It then installs the pack through a separate single-use Godot plan, checks `directionalPlayback.left.flipH: true`, checks Provider provenance in `forge_usage.json`, and loads the project with Godot headlessly.

### CLI/MCP/plugin

- `forge-cli provider list --json`: passed; `xai` and `fixture` capabilities are reported without credentials.
- `forge-cli provider doctor --provider fixture --json`: passed.
- `npm --workspace packages/mcp test`: passed, including the three Provider-generation MCP mappings.
- release `forge-cli` and `plugins/forge-assets/mcp/server.bundle.mjs`: rebuilt.
- Codex plugin validator: passed.
- Forge Assets Skill validator: passed.

### Existing application regression

- `npm run build`: passed.
- `npm run test:scripts`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

### Real xAI OAuth and Imagine run

The user completed Forge's direct Device Code login. `forge-cli provider doctor --provider xai` then reported `oauth_device_code` and authenticated; no Grok Build CLI was invoked.

The first clean-directory request reached xAI successfully and exposed a real response-compatibility issue: the image endpoint returned JPEG bytes through its temporary URL while Forge had reserved a `.png` output path. Core correctly stopped before processing. The xAI Provider was fixed to decode PNG/JPEG/GIF/WebP bytes and atomically canonicalize every image output to local PNG. The URL-fallback contract test now serves JPEG and verifies a PNG result.

The second clean-directory run succeeded:

- Job: `bd7b3dc0-5c8c-4552-94c2-d0fb40b8b5ef`
- Provider/Profile: `xai/default`
- Workflow: `topdown@1.0.0`
- Media: 5 images and 4 videos, all actions on attempt 1
- Requests recorded by Provider: 13
- Usage recorded by xAI: 25,700,000,000 USD ticks (approximately USD 2.57)
- Output: 32 frames; `idle`, `walk_up`, `walk_right`, and `walk_down` all `game_ready`
- Pack SHA-256: `20db8fdf3751f781eed28b388a133a2aca823421905121de7aa3ce0e0f7ced50`

The green RGB visible in a raw PNG viewer was verified to belong to fully transparent pixels: a sampled background pixel had Alpha 0 and a character pixel had Alpha 255. The frame's average Alpha matched the reported foreground coverage.

The real pack was then installed through a separate single-use plan into a new Godot project. Godot imported all eight atlas pages and loaded the project headlessly. `forge_usage.json` contains `walk_up`, `walk_right`, `walk_down`, horizontally flipped left playback, and the xAI generation job provenance.

Two further clean-directory runs completed consecutively after the JPEG fix:

| Run | Job | Attempts | Verdict | Recorded cost | Pack SHA-256 |
| --- | --- | --- | --- | --- | --- |
| 1 | `bd7b3dc0-5c8c-4552-94c2-d0fb40b8b5ef` | all actions 1 | `game_ready` | USD 2.57 | `20db8fdf3751f781eed28b388a133a2aca823421905121de7aa3ce0e0f7ced50` |
| 2 | `ee96c5ee-87ab-4227-b625-145f9d44f67c` | all actions 1 | `game_ready` | USD 2.57 | `578b28a82205f6bfbc226afb4601d7b28b5a8078b7cf9d95be7d0f01972b1360` |
| 3 | `a4efb9c1-38ce-4d80-905c-5b88887ad25d` | `walk_right` 2; others 1 | `game_ready` | USD 3.20 | `007220cbb96efc2740cb344a20d4eb52a5abc0b20e57a087c9fcfd29533f53d1` |

Run 3 is direct evidence for bounded quality recovery: the first `walk_right` failed the local quality gate, only that action was regenerated, its second result passed, and the other three Provider outputs remained unchanged. No Provider or model switch occurred. The three successful runs recorded USD 8.34 total. The first pre-fix reference-image request may also have incurred image-generation cost even though Forge correctly rejected its non-canonical local representation.

All three successful JobStores passed the credential, device-code, authorization-header, and signed-URL leak scan.

Preserved evidence:

- `docs/qa/artifacts/forge-xai-provider-reference-2026-08-02.png`
- `docs/qa/artifacts/forge-xai-character-preview-2026-08-02.gif`
- `docs/qa/artifacts/forge-xai-character-quality-2026-08-02.json`
- `docs/qa/artifacts/forge-xai-provider-manifest-2026-08-02.json`
- `docs/qa/artifacts/forge-xai-godot-usage-2026-08-02.json`

## Security and dependency audit

- A completed fixture JobStore was scanned for authorization headers, Bearer values, access/refresh tokens, device codes, and the displayed device short code; no matches were found.
- The Provider manifest stores local paths, asset IDs, model selection, usage, and SHA-256 only. Temporary xAI media URLs are consumed immediately and are not persisted.
- New direct dependencies (`base64`, `dirs-next`, `gif`, `keyring`, `reqwest`, `sha2`, `tempfile`, and `url`) report MIT or Apache-2.0 dual-compatible licenses.

## Remaining release gates

1. Receive written xAI confirmation for a Forge-specific or approved shared OAuth client before commercial distribution. Until then OAuth remains Preview and `XAI_API_KEY` is the stable commercial mode.
2. Record generated-asset commercial-use and disclosure terms after xAI responds.
