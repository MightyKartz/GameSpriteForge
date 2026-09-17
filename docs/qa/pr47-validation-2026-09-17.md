# PR #47 validation — 2026-09-17

PR: https://github.com/MightyKartz/GameSpriteForge/pull/47

Reviewed head: `f9b4398e7e506f45afc01589675498de2b3f6f9f`. The remote PR was open, mergeable, and clean when checked. This section records the original review; see the remediation below for subsequent fixes.

## Existing CI

All three workflows passed for the reviewed PR head:

- [Godot agent workflow](https://github.com/MightyKartz/GameSpriteForge/actions/runs/35119095973): Windows and macOS native cases.
- [Windows portable package](https://github.com/MightyKartz/GameSpriteForge/actions/runs/35119095582): package and installation checks.
- [Forge quality matrix](https://github.com/MightyKartz/GameSpriteForge/actions/runs/35119095607): quality gates and cross-platform exchange.

The cases below exercise gaps in that coverage. They do not invalidate the successful covered cases.

## Local build and evidence

- Executable: `C:\AI\Forge\GameSpriteForge\target\debug\forge.exe`, freshly built with `cargo build --locked -p forge-cli --no-default-features`.
- Build: reviewed commit, `x86_64-pc-windows-msvc`, debug, no features, `dirty:true`. Existing untracked QA files account for the dirty checkout; tracked production source was unchanged.
- Forge SHA-256: `223f379d00ccd8ec38295bd192c5d3938c3f6803b5b459d77f2eebbd0c8689b1`.
- Real Godot: `4.6.3.stable.official.7d41c59c4`; console SHA-256 `63b3b2208819714c9677fbfdd8217c5b7dee8ecf5f383502e826bc9e2227ff5a`.
- Local reproduction script: `target/qa/godot-workflow/review-pr47.py`.
- Full ignored evidence directory: `target/qa/pr47-review-20260917-063123/` (including `summary.json`, native logs and reports).
- Tests used a synthetic project, a unique project name, and isolated APPDATA/Forge stores. No consumer game or real save was run or modified. Existing Godot and its official release template were read only.

## P1: Verification and exported startup reuse the game's user data

Locations: `packages/cli/src/godot_workflow.rs:529-532`, `592-600`, and `790-801`.

The snapshot changes the project path but keeps its Godot user-data identity and inherits the environment. The exported runtime launch also has no user-data isolation. A normal project that saves on startup therefore reads or writes the same `user://` directory as the original game during automated acceptance. Old saves can also influence acceptance results.

Reproduction: make a synthetic startup script read `user://pr47-review-launch-count.txt`, increment it, write it back, and print `OS.get_user_data_dir()` and the count. Launch the original project once; then run Forge verification; then run Forge export with `--run` and an absolute custom template path.

Observed results:

1. Original launch: count `1`.
2. `forge godot verify`: count `2`, same user-data path, report `status:passed`.
3. `forge godot export --run`: count `3`, same user-data path, report `status:passed`.

Evidence: `userdata-baseline.stdout.log`, `verify/runtime.stdout.log`, and `userdata-export/exported-runtime.stdout.log`. All print the same unique directory under the test's isolated APPDATA. The `currentCount` field in the raw summary was captured after verification; the later `exportedRuntime` field records count 3.

This finding concerns normal engine-managed saves, not a claim that the documented working-copy isolation is an arbitrary-code sandbox. Use a per-run isolated Godot user-data location across verification and exported startup, and require explicit opt-in to use existing saves. Add a regression asserting that an existing original save remains unchanged.

## P2: External relative custom templates break in the snapshot

Locations: `packages/cli/src/godot_workflow.rs:702-720`, `732-740`, and `751-755`.

The selected preset's custom release template is resolved and hashed relative to the original project, but the copied preset retains its original relative path. Godot is then launched against the relocated copy, where the relative path points elsewhere.

Reproduction: place a valid Windows release template beside the project at `../templates/release.exe` and set `custom_template/release="../templates/release.exe"` in a Windows Desktop preset. Export the original with Godot directly, then export the same preset through Forge.

Observed results:

- Direct Godot export: exit `0`, executable produced.
- Forge export: exit `1`, custom release template not found.
- Changing only the template path to an absolute path allows Forge export and actual exported startup to pass.

Evidence: `direct-relative-export.stdout.log`, `forge-relative-export.stdout.log`, `forge-export/export.stderr.log`, and `userdata-export/report.json`. The synthetic source preset was changed to absolute for the subsequent user-data export check; the script preserves the original relative-path setup.

Bind the resolved template into the copied preset, or copy it into the snapshot and rewrite the path there. Keep the original preset unchanged and ensure the recorded template hash describes the file actually used. Add a regression with a valid template outside the source project referenced through `..`.

## Review scope

No product changes, merge, installation, or GitHub review submission were performed. These findings were reproduced on Windows; existing macOS CI passes, but the two new edge cases have not been reproduced on macOS in this review.

## Remediation and local regression verification

Both findings were fixed after the user requested implementation:

- Each verification/export now creates a fresh child-process profile under its evidence directory. Imports, main-scene runs, interaction scripts and exported startup use it. Godot project settings and exported PCKs keep the original game identity. `userData.root` records the profile location. Windows verbatim filesystem prefixes are normalized before passing profile paths to Godot.
- Release templates are resolved before profile isolation, bound by absolute path in the copied preset, and fingerprinted before/after export. This preserves relative custom templates outside the project and standard installed templates. `releaseTemplate` records the actual template and whether it came from standard installation or the selected custom preset. Other presets and the original configuration are preserved.
- The existing Windows/macOS native CI script now checks default and custom user directories, repeated independent runs, original-save preservation, source-configuration preservation and an external relative template with spaces/Unicode in its filename. Unit coverage includes `res://`, missing options sections, other-preset preservation, and versioned self-contained standard templates.

Local Windows checks passed: 19 CLI unit tests, focused Clippy with warnings denied, formatting/diff checks, embedded guide checks (86 commands), and all 20 native workflow cases including a real GPU screenshot and both standard/custom exported program launches. Default and custom original saves remained at `1`; every acceptance runtime used its own evidence profile. The earlier review's relative-template failure now exports and starts successfully.

The local verification build was based on `f9b4398` with these source changes (`dirty:true`, debug, no features). Its SHA-256 is `468b23b0ea7e97b3dbffe9c305ebd2593173125ffa3cd59d5155fd01de1eae2e`. Native evidence is under `target/qa/pr47-fix-native-final/`; the compact retained record is [artifacts/pr47-fixes-2026-09-17.json](artifacts/pr47-fixes-2026-09-17.json). The installed consumer CLI, actual game projects and personal Godot configuration were not updated. Native macOS and packaged Windows checks are delegated to the PR's existing CI workflows; local checks alone do not establish those results.
