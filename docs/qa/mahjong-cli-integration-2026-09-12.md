# Mahjong-driven Forge CLI improvements

Mahjong already uses Forge for static assets. Its local workflow exposes reusable
CLI gaps: Windows requires a source patch, usage JSON contains native backslashes,
and a successful import can still leave the game's image lock out of sync. This
change moves the generic image checks into Forge and fixes those Windows defects.

## Reviewed inputs

- Consumer checkout: `Mahjong`, commit `e51e5fdbf57dde939d72600d7bdabd36bc294475`,
  clean branch `codex/mahjong-development-skill`.
- Consumer instructions: `AGENTS.md`, personal `mahjong-godot-workflow/SKILL.md`
  and `references/project-workflow.md`.
- Consumer contracts: `tools/toolchain.json`, `tools/asset-lock.json`,
  `tools/validate-assets.ps1`, `tools/forge/invoke-forge.ps1`, retained requests
  and receipts, `docs/toolchain-status.md`, `docs/art-pipeline.md` and
  `docs/reviews/PR-008-live-table-layout.md`.
- Forge base: `4bc766b3f093fc8d042e8886f617059384cd7ea3`; work is on
  `codex/mahjong-cli-integration`.

No consumer source was copied into Forge. Mahjong's project files, personal skill,
image lock, toolchain lock and pinned executable were not modified. Native Godot
installation tests used separate temporary projects, not the Mahjong game.

## Implemented behavior

1. `asset verify-images` verifies a consumer-provided image lock against explicit
   scan directories. It checks exact PNG membership, SHA256, decoded dimensions,
   declared RGBA/opacity/border rules and an optional image count. It reports
   missing and extra files even when their counts cancel out. Invalid containers,
   case collisions, traversal, symbolic links and Windows junctions fail. It does
   not write a lock or accept new files as reviewed artwork.
2. The report is explicitly `image_contract_only`. Other top-level fields and
   image metadata are listed in `notCheckedFields`, including rig/audio rules,
   source manifests, transfer receipts and visual regions. Mismatches return
   exit 1, `ok:false`, `image_contract_failed`, and the complete `data.issues`.
3. Windows FFmpeg executable discovery compiles without a downstream patch and
   checks `.exe` candidates. Newly installed Godot usage paths use `/`; read-only
   audits also accept the equivalent URI from old Windows usage records without
   rewriting their registered bytes.
4. Embedded delivery guidance and the CLI protocol document the new capability.
   CI gains Windows source compilation and offline delivery/image-contract checks,
   plus the CLI smoke test in the existing macOS workflow. Remote CI has not been
   triggered by this local task.

This addresses the PR-008 failure mode: after an import adds an unlocked texture,
the image command detects it immediately rather than waiting for a game export.
The game's existing validator still owns its bespoke rules and final delivery.

## Verification

| Check | Result |
| --- | --- |
| Windows default CLI source build | Passed; Rust/Cargo 1.98.1, MSVC 14.44.35207 |
| `cargo fmt --all -- --check` | Passed |
| `image_contract_tests` | 12 passed, including 16-bit alpha, corrupt IEND, junction cycle and Windows scan-root case alias |
| `test-image-contract-cli.py` | Passed 9 reported checks, including exit codes, one JSON envelope and no file/store writes |
| `test-cli-skill.py --guide-only` | 9 passed / 61 CLI calls / 7 embedded resources / 15 relative links; zero executed Provider requests |
| FFmpeg discovery unit tests | 4 passed |
| `delivery_audit_tests` | 5 passed, including byte-preserving legacy Windows URI acceptance |
| `static_delivery_tests` | 3 structural tests passed |
| Ignored real Godot static delivery test | Passed with Godot 4.6.3; four isolated deliveries and saved scene loading |
| Real Mahjong image contract | 163 locked / 163 scanned / 163 verified; zero issues |

The embedded-guide harness required Windows portability fixes: decode the CLI's
UTF-8 bytes without locale/newline conversion, resolve bundled links as POSIX
paths, and preserve exported example bytes. The first supported guide checks
passed; the full harness then reached the existing product error
`skill_update_failed: safe skill installation is not implemented on this platform`.
Windows guide verification is therefore explicitly scoped with `--guide-only`;
it does not claim skill installation/update or symlink-protection tests passed.
The existing full harness remains the macOS CI check. No Windows security settings
or privileges were changed.

The real image check completed in 34.251 seconds using the debug executable. Its
input image bytes, both lock files, pinned CLI SHA256 and Git status were compared
before and after and remained unchanged. No Job or Plan store was created and no
Provider request occurred. Full per-image diagnostics stay in ignored local
`target/qa/mahjong-image-contract/full-report.json`; the compact provenance result
is retained in [the QA artifact](artifacts/mahjong-cli-integration-2026-09-12.json).

The tested executable is `target/debug/forge.exe`, version `0.3.2`, with the base
commit above, `dirty:true`, target `x86_64-pc-windows-msvc`, profile `debug` and
features `[]`. Its SHA256 is
`caf8932f26f917386c1a4fb2793f49337bb82200e10e4bd7bec3ee59a7f14278`.
The version label alone does not identify this development build. Windows source
verification does not change the official macOS distribution support declaration.

Reproduce using the development executable, without changing Mahjong's pin:

```powershell
& 'C:\AI\Forge\GameSpriteForge\target\debug\forge.exe' asset verify-images `
  --root 'C:\AI\Games\Mahjong' --lock tools/asset-lock.json --scan game --json
```

## Further improvements supported by the consumer evidence

| Priority | Evidence and proposed contract |
| --- | --- |
| Next: static matting | Mahjong compiled a local single-image wrapper because static import has no chroma-key path and animation preparation requires multiple frames. Add a dedicated static operation using Forge core, preserve source hashes and explicit parameters, and validate edge behavior with fixtures before claiming it replaces the wrapper. Do not copy its project-specific thresholds as defaults. |
| Next: native-size static delivery | `table-render-v4/final-v1/prepare-plan.json` rejected a 2048 canvas; backgrounds of 1672×941 and a 2241×702 texture bypass static square normalization. Define an explicit preserve-source canvas policy with unchanged default normalization, then test Pack/Godot rendering contracts. |
| Later: runtime transfer evidence | Mahjong sometimes transfers files from isolated installs into its runtime asset tree. A generic transfer receipt could link reviewed source, Pack and final runtime paths, without pretending a copied PNG proves native installation or EXE/PCK membership. |
| Optional Windows skill installation | Embedded `guide` and `skill show` work, but safe `skill install` remains implemented only for macOS/Linux. Windows support needs its own filesystem/atomic update and link-protection contract; it is independent of using the CLI's built-in guide. |

These are subsequent feature contracts, not capabilities implemented here. The
current image check does not validate rig motion, audio, import caches, native
resource loading in Mahjong, exported EXE/PCK contents, or artwork quality.
