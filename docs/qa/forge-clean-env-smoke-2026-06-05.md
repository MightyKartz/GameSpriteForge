# Forge Clean Environment Smoke - 2026-06-05

Note: this evidence is historical. The current 2026-06-06 release candidate is `release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized`; see `PRODUCT.md`, `docs/architecture/distribution-mvp.md`, and `docs/qa/local-workbench-ux-evidence-2026-06-06.md`.

Artifact under test:

```text
release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized.zip
DMG SHA-256: 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
Zip SHA-256: fa3b9331eaafe513ebd08d0ae5dc07cb8324669e65d3a2e7a827286666e3ad00
Notarization submission: 7792d837-5da7-46da-8da4-33b559dda6cc
```

Environment:

```text
Machine: current verification Mac, shell package verification plus Computer Use UI smoke
macOS version: macOS 26.5.1 (25F80)
Apple Silicon or Intel: Apple Silicon (arm64)
Fresh user account or scrubbed PATH: scrubbed launch used HOME=/var/folders/wf/dm2xwhyx7ll8zr6khcm9vdnc0000gn/T//forge-clean-home.JCwUlW and PATH=/usr/bin:/bin
ffmpeg source: /opt/homebrew/bin/ffmpeg was still discovered by the app's default macOS tool directories
ffprobe source: /opt/homebrew/bin/ffprobe was still discovered by the app's default macOS tool directories
Deterministic missing-tool launch: GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools and GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1
```

Release package verification:

```bash
cd /Users/kartz/Development/Forge/release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized
shasum -a 256 -c SHA256SUMS
scripts/verify-release-package.sh "./Game Sprite Forge_0.1.0_aarch64.dmg" \
  218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
```

Expected:

```text
SHA256SUMS: all OK
verify-release-package.sh: PASS release package verification script completed.
Gatekeeper DMG assessment: accepted, source=Notarized Developer ID
Gatekeeper app assessment: accepted, source=Notarized Developer ID
mounted-DMG launch: pass
```

Observed shell verification on 2026-06-05:

```text
shasum -a 256 -c SHA256SUMS:
Game Sprite Forge_0.1.0_aarch64.dmg: OK
scripts/verify-release-package.sh: OK
docs/release-candidate-verification.md: OK
README.md: OK

scripts/verify-release-package.sh "./Game Sprite Forge_0.1.0_aarch64.dmg" 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739:
Expected SHA-256: 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
Actual SHA-256:   218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
hdiutil verify: VALID
notarytool ticket validation: worked
Gatekeeper DMG assessment: accepted, source=Notarized Developer ID, origin=Developer ID Application: Ka Yan (J6P96F432P)
Gatekeeper app assessment: accepted, source=Notarized Developer ID, origin=Developer ID Application: Ka Yan (J6P96F432P)
Launch App From DMG: observed Game Sprite Forge pid 503
PASS release package verification script completed.
```

Manual app rows:

| ID | Check | Expected | Result | Evidence |
| --- | --- | --- | --- | --- |
| CE-001 | Open DMG without bypassing Gatekeeper | App launches from mounted DMG | Pass | `verify-release-package.sh` mounted the refreshed DMG, Gatekeeper accepted it, and observed Game Sprite Forge pid 503. |
| CE-002 | Install to `/Applications` | Installed app opens normally | Pass | The refreshed app was synchronized to `/Applications/Game Sprite Forge.app`; executable SHA-256 is `5c81ce11bafd317b3e10b0b8367e2b99ba339505bfb46ea4b3709c25b30862a5`, CDHash is `6a69aa5fe751c37dbfde3a11e145180bb2b3862d`, Developer ID team is `J6P96F432P`, stapler validation worked, and launch observed pid 55298. |
| CE-003 | Missing ffmpeg/ffprobe | `Install ffmpeg or choose an ffmpeg binary in Settings.` | Pass | Computer Use launched the rebuilt signed app with `GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools`, `GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1`, `PATH=/usr/bin:/bin`, and cleared saved tool paths; Check FFmpeg showed the exact missing-tool message. |
| CE-004 | PATH ffmpeg/ffprobe fallback | Settings check reports `ffmpeg and ffprobe are available.` | Pass | Computer Use Check toolchain showed `ffmpeg and ffprobe are available.` in the installed app. |
| CE-005 | Invalid configured ffmpeg | `ffmpeg: configured tool path must point to ffmpeg` | Pass | Settings path `/tmp/not-ffmpeg` showed `ffmpeg: configured tool path must point to ffmpeg`. |
| CE-006 | Invalid configured ffprobe | `ffprobe: configured tool path must point to ffprobe` | Pass | Settings path `/tmp/not-ffprobe` showed `ffprobe: configured tool path must point to ffprobe`. |
| CE-007 | Bundled sample video | Import, process, export, validate re-import succeeds | Pass | Computer Use Run sample completed with `Full pipeline passed: 24 frames exported and re-imported.` The run summary showed `Export validated for this session`, `Green-Box-Character-Pack.gsfpack`, and `Validated Green Box Character Pack with 24 frames`. |

Decision:

```text
Clean-environment confidence: Pass for current local QA - package verification, mounted-DMG launch, installed-app launch, configured/default toolchain detection, deterministic missing-tool detection, invalid configured tool recovery, and bundled sample pipeline passed.
Shell package verification: Pass
Blocking issues: none for the current local import-only MVP release candidate.
Follow-ups: A true clean Mac without `/opt/homebrew/bin/ffmpeg`, `/opt/homebrew/bin/ffprobe`, `/usr/local/bin/ffmpeg`, and `/usr/local/bin/ffprobe` is still a useful external confidence check before broad distribution, but deterministic CE-003 is now reproducible through the QA resolver override.
```

Handoff notes:

```text
No commit was made because /Users/kartz/Development/Forge is not a git repository.
```
