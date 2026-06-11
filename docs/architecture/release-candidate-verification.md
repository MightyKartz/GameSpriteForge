# Release Candidate Verification

Use this checklist to verify the current MVP distribution artifact.

Status as of 2026-06-06: the refreshed DMG and mounted app are Developer ID signed, notarized, stapled, accepted by Gatekeeper, and verified through mounted-DMG launch. The synchronized `/Applications` app passes codesign, validates its stapled app ticket, launches locally, and shows the Local Workbench UX hardening copy. Gate 5 is satisfied for the current public release candidate. Deterministic missing-ffmpeg QA now passes through the resolver override; a true clean-Mac check remains useful external confidence before broad distribution.

## Artifact

```text
target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg
SHA-256: 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
Architecture: Apple Silicon / arm64
Signing: Developer ID Application: Ka Yan (J6P96F432P)
Notarization: Accepted, submission 834c0445-1c45-4201-88d5-a2c99c008714
Stapled ticket: pass
Installed app: /Applications/Game Sprite Forge.app
```

## Current Candidate Check

Use these commands for the current notarized and stapled candidate:

```bash
shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
hdiutil verify "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
xcrun stapler validate -v "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
spctl --assess --type install --verbose=4 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
codesign --verify --deep --strict --verbose=2 "/Applications/Game Sprite Forge.app"
xcrun stapler validate -v "/Applications/Game Sprite Forge.app"
plutil -p "/Applications/Game Sprite Forge.app/Contents/Info.plist" | rg "CFBundleExecutable|CFBundleIdentifier|CFBundleName"
```

Expected current result:

```text
SHA-256 matches 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
hdiutil verify: pass
stapler validate: pass
spctl DMG: accepted, source=Notarized Developer ID
codesign: pass
installed app stapler validate: pass
CFBundleExecutable: Game Sprite Forge
CFBundleIdentifier: dev.gamespriteforge.desktop
```

## Notarization Preflight

Before submitting a future rebuilt DMG to Apple, run the preflight. It does not submit, staple, or claim notarization passed.

```bash
bash scripts/notarization-preflight.sh "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" \
  "$(shasum -a 256 "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg" | awk '{print $1}')"
```

The preflight checks:

```text
xcrun, notarytool, and stapler availability
selected credential method without printing secrets
DMG SHA-256
hdiutil verify
DMG Developer ID signature
DMG secure timestamp
mounted app Developer ID signature
mounted app secure timestamp
mounted app hardened runtime
current stapled-ticket state
current Gatekeeper DMG assessment
exact next notarization, stapling, and release-verifier commands
```

Credential method selection:

```bash
bash scripts/notarization-preflight.sh \
  --keychain-profile "GameSpriteForgeNotary" \
  "target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg"
```

You can also set `NOTARYTOOL_KEYCHAIN_PROFILE` or `FORGE_NOTARY_KEYCHAIN_PROFILE` instead of passing `--keychain-profile`.

After creating a keychain profile, verify the stored item without printing secrets:

```bash
xcrun notarytool history --keychain-profile "GameSpriteForgeNotary"
```

If this reports `No Keychain password item found for profile`, run `xcrun notarytool store-credentials` before submission.

Or set all three App Store Connect API key variables before running the same preflight:

```bash
export APP_STORE_CONNECT_API_KEY_PATH="/path/to/AuthKey_KEYID.p8"
export APP_STORE_CONNECT_KEY_ID="KEYID"
export APP_STORE_CONNECT_ISSUER_ID="ISSUER_UUID"
```

If a rebuilt local candidate has no stapled ticket, the preflight should report `Stapled ticket: missing` and print the submit/staple commands. That is a public-release hold for that rebuilt candidate, not a release pass.

## Public Release Check

Copy the DMG and `scripts/verify-release-package.sh` to the verification Mac, then run:

```bash
chmod +x scripts/verify-release-package.sh
scripts/verify-release-package.sh "/path/to/Game Sprite Forge_0.1.0_aarch64.dmg" \
  0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
```

The script verifies:

```text
SHA-256
hdiutil verify
stapled notarization ticket
Gatekeeper DMG assessment
app bundle codesign
Gatekeeper app assessment
launch from mounted DMG
cleanup
```

The script mounts the DMG at a per-run temporary mountpoint instead of assuming `/Volumes/Game Sprite Forge`, so it can run even when another same-named volume is already mounted.

After the verifier passes on the notarized and stapled DMG, create or refresh the release package:

```bash
scripts/package-release-candidate.sh
```

This package script validates the stapled ticket first and refuses to create a `release-candidates/GameSpriteForge-0.1.0-aarch64-<sha12>-notarized` package from an unstapled DMG. The current package is `release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized`. The SHA prefix keeps current packages distinct from older same-version candidates.

## Manual App Check

Record the result of each item:

The ffmpeg environment rows are covered locally by deterministic resolver overrides and the Homebrew PATH/default-directory recovery path. A true clean Mac remains useful as a broad-distribution confidence check, but it is no longer a blocker for this local installed-app MVP QA result.

```text
[x] app opens from the notarized DMG without bypassing Gatekeeper; release verifier observed `Game Sprite Forge` pid after mounted-DMG launch
[x] missing ffmpeg state says: Install ffmpeg or choose an ffmpeg binary in Settings. Verified with `GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=/tmp/forge-empty-tools`, `GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS=1`, and `PATH=/usr/bin:/bin`.
[x] configured ffmpeg and ffprobe paths work
[x] invalid configured ffmpeg or ffprobe path reports the configured-path error without silently falling back to PATH
[x] PATH/default-directory fallback works when ffmpeg and ffprobe are available in the app launch environment
[x] examples/inputs/green-box-character.mp4 processes end to end in the installed local app
[x] a non-Forge real local short video processes end to end in the installed local app
[x] exported .gsfpack validates in the installed local app
[x] app re-imports the exported .gsfpack in the installed local app
[x] re-export preserves the original frame count in the installed local app
[x] empty PNG sequence input shows `choose at least one PNG frame`
[x] non-video import shows `ffmpeg_command_failed: ... Invalid data found when processing input`
[x] mixed-size PNG sequence has accepted normalization behavior and processes 3 frames instead of failing
[x] invalid sprite sheet grid shows `sprite sheet grid exceeds source image bounds`
[x] invalid .gsfpack folder shows `required pack file is missing: previews/preview.gif`
[x] empty default output folder disables export with `Choose an export folder in Settings.`
[x] no AI provider, website, MCP, marketplace, cloud, BYOK, or cost setup is required
```

## Current Verification Evidence

Recorded on 2026-06-06 for the refreshed Local Workbench UX hardening release candidate:

```text
npm run test:scripts: pass
npm --workspace apps/mac run build: pass
npm --workspace apps/mac run smoke:ui:mvp: pass
npm --workspace apps/mac run smoke:ui:responsive: pass
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml: pass
scripts/build-signed-release-dmg.sh: pass; direct Tauri Developer ID signing was bypassed because Tauri CLI did not pass a macOS secure timestamp to codesign
pre-staple DMG SHA-256: 457b6234f9b87bcc35f1e206ca69c046769e4e9ea65927b2e37d081fa98c9551
notarization preflight before submission: pass; missing stapled ticket and pre-submission Gatekeeper rejection were expected
notarization submission: Accepted, submission 834c0445-1c45-4201-88d5-a2c99c008714
xcrun stapler staple DMG: pass
xcrun stapler validate DMG: pass
post-staple DMG SHA-256: 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42
scripts/verify-release-package.sh target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg 0a5d18c3d1c8df79ba73e617ed972241cc2208b553227fda56ec124e90b5cc42: pass; observed mounted-DMG launch pid 34824
scripts/package-release-candidate.sh: created release-candidates/GameSpriteForge-0.1.0-aarch64-0a5d18c3d1c8-notarized and matching zip
/Applications sync from mounted DMG: pass
codesign /Applications/Game Sprite Forge.app: pass
xcrun stapler staple /Applications/Game Sprite Forge.app: pass
xcrun stapler validate /Applications/Game Sprite Forge.app: pass
spctl /Applications/Game Sprite Forge.app: accepted, source=Notarized Developer ID
Computer Use mounted-DMG UI: Run Sample Pipeline processed and validated 24 frames; Exports Refresh/Inspect/Validate/Re-import worked; re-imported pack processed, exported, and Validate Re-import preserved 24 frames
Computer Use /Applications UI: app launched from /Applications and showed the new First Run copy plus Load Sample Path hint
hard public-release gate: satisfied for the refreshed DMG and release candidate package
remaining external release-environment checks: optional true clean-Mac ffmpeg/ffprobe confidence check
```

Historical 2026-06-05 candidate evidence:

```text
npm --workspace apps/mac run build: pass
cargo test --manifest-path /Users/kartz/Development/Forge/Cargo.toml: pass
npm --workspace apps/mac run smoke:ui:mvp: pass
npm run qa:fixtures: pass
npm run qa:pipeline: pass
npm run test:scripts: pass
bash -n scripts/verify-release-package.sh: pass
bash -n scripts/notarization-preflight.sh: pass
bash -n scripts/package-release-candidate.sh: pass
bash -n scripts/build-signed-release-dmg.sh: pass
bash scripts/test-notarization-preflight.sh: pass
impeccable detector: []
scripts/build-signed-release-dmg.sh: pass; direct Tauri Developer ID signing was bypassed because Tauri CLI did not pass a macOS secure timestamp to codesign
historical DMG SHA-256: 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739
hdiutil verify DMG: pass
mounted DMG Info.plist: CFBundleExecutable=Game Sprite Forge, CFBundleIdentifier=dev.gamespriteforge.desktop
mounted DMG app codesign verification: pass
codesign installed app: pass
installed app stapler validate: pass; `/Applications` executable SHA-256 matches the target app bundle
keychain profile check: xcrun notarytool history --keychain-profile GameSpriteForgeNotary reports Successfully received submission history
notarization submission: Accepted, submission 7792d837-5da7-46da-8da4-33b559dda6cc; earlier submission be42fdff-cdff-41f2-8961-c13392ff8800 stayed In Progress after a network timeout and was superseded
notarytool log: status Accepted, statusSummary Ready for distribution, issues null
xcrun stapler staple: pass
xcrun stapler validate: pass
spctl DMG install/open assessment: accepted, source=Notarized Developer ID
spctl mounted app assessment: accepted, source=Notarized Developer ID
scripts/notarization-preflight.sh --keychain-profile GameSpriteForgeNotary target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739: pass
scripts/verify-release-package.sh target/release/bundle/dmg/Game Sprite Forge_0.1.0_aarch64.dmg 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739: pass; observed mounted-DMG launch pid 96126
release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized/scripts/verify-release-package.sh ./Game Sprite Forge_0.1.0_aarch64.dmg 218a5f9adf1fb47e5f744efbda357470ac8c6aa2c065451c926842c35cd0c739: pass; observed mounted-DMG launch pid 78481
scripts/package-release-candidate.sh: created release-candidates/GameSpriteForge-0.1.0-aarch64-218a5f9adf1f-notarized and matching zip
installed app manual QA: sample video 24 frames exported and validated; real non-Forge Wacom Gesture3FingerTap.mp4 video 8 frames exported and validated; high-resolution /Users/kartz/Development/social-auto-upload/videos/demo.mp4 exported as 30 sprite-sheet pages and re-imported with 30 frames; PNG sequence 6 frames exported and validated; sprite sheet 8 frames exported and validated; exported .gsfpack imported, processed, re-exported, and validated with 6 frames
installed app failure-state QA: empty PNG sequence shows choose at least one PNG frame; invalid configured ffmpeg shows ffmpeg: configured tool path must point to ffmpeg; invalid configured ffprobe shows ffprobe: configured tool path must point to ffprobe; deterministic missing ffmpeg shows Install ffmpeg or choose an ffmpeg binary in Settings.; non-video import shows ffmpeg_command_failed ... Invalid data found when processing input; mixed-size PNG sequence is accepted by normalization and processes 3 frames; invalid sprite sheet grid shows sprite sheet grid exceeds source image bounds; invalid .gsfpack shows required pack file is missing: previews/preview.gif; empty output folder disables export with Choose an export folder in Settings.; dependency recovery restored with ffmpeg and ffprobe are available; canceling a video Choose File dialog keeps the sample path and shows Selection cancelled.; single-frame PNG import produces Quality Report Blocked and disables Export Pack / Validate Re-import; Recent Exports keeps prior successful packs after failed or blocked attempts.
source follow-up fix: resolved in the rebuilt installed app; invalid-video recovery routing now sends Invalid data found when processing input to valid-video source selection before generic ffmpeg/toolchain recovery
hard public-release gate: satisfied for the current DMG and release candidate package
remaining external release-environment checks: optional true clean-Mac ffmpeg/ffprobe confidence check; deterministic missing ffmpeg, PATH/default-directory recovery, cancel-dialog no-op, blocked-quality export disablement, failed-job Recent Exports integrity, and non-video recovery routing now have installed-app UI evidence
```
