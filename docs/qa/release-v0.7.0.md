# v0.7.0 release verification

Scope: default-feature macOS Apple Silicon and experimental Windows x64 packages.
Preparation baseline: `7eb9083281cbf35932dd2346e1b6e60a394a6d04`.
No consumer projects, toolchain locks, source media or historical receipts change.

## Gates and evidence

The release preparation PR and its workflow runs retain exact source commits,
package/binary hashes, doctor identities and native logs. Before merging, run
`release-cli.yml` with `workflow_dispatch`, version `v0.7.0`, on the PR branch.
That preflight builds both native packages and assembles downloads without
publishing a release. After merge the tag workflow repeats the gates for the
exact merged commit; only its successful packages are published.

| Gate | Evidence source |
| --- | --- |
| Formatting, Clippy, Rust workspace tests and clean release identity | macOS release job, `verify-cli-build.py` |
| macOS fresh/reinstall and historical upgrade | `test-cli-release-artifact.sh`, public symlink and manifest checks |
| macOS published v0.6.4 → v0.7.0 upgrade/reinstall | `forge-macos-agent-verification-RUN_ID`, old manifest/full inventory, new BUILD_INFO/doctor |
| Windows fresh/reinstall, PowerShell 5.1 and prior-payload preservation | `windows-portable/summary.json`, `powershell51-doctor.json` |
| Windows actual v0.6.4 upgrade plus older pre-library upgrade | `windows-v064-upgrade/summary.json`, `windows-historical-upgrade/summary.json` |
| Installed M2 animation revision and M3 audio/static recovery | macOS `m2/m3` and Windows `windows-agent-m2/m3` artifacts; upgraded public launchers with external helper discovery disabled |
| Bundled helpers and preview/audio behavior | doctor helper paths, native package tests, PNG/GIF/MP4/audio outputs |
| Paired archives, binary/source identities and eight downloads | `release-verification.json`, assembled source/notices manifest and download checksums |

The existing macOS/Windows × Godot 4.6.3/4.7.2 PR matrix covers source workflows;
packaged release gates use Godot 4.6.3. Neither substitutes for the other. M2/M3
fixtures are synthetic technical validation, not M4 real-project benefit evidence.
Visual/listening/license approval remains separate.

## Local preparation checks

Formatting, locked offline Cargo metadata, shell syntax and workflow YAML parsing
passed locally. The 12 release-asset regression tests passed with bundled Python
3.12; the first system Python 3.9 run had four errors because unittest.enterContext
is unavailable there, so that run is not reported as a pass. Diff/link checks are
also required before merge. Native packaging is exercised
on GitHub's macOS/Windows runners; this machine has insufficient free disk for a
safe duplicate release build. No consumer installation is used as a test root.

At initial preparation the new versioned preflight and publication are pending.
Record actual run IDs/results on the PR and final publication report; do not treat
this gate checklist as a passed runtime result. Published notes point to the exact
release commit and the downloadable paired package verification.
