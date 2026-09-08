# Bundled Forge skill delivery — 2026-09-08

Goal: distribute the self-contained `forge-use` workflow with the CLI, install it
into a selected Codex scope, preserve user content, and verify the downloaded
release. The intended release is v0.3.1; asset processing contracts stay unchanged.

| Step | Result |
| --- | --- |
| Self-contained skill | Six files with thirteen valid internal Markdown links; entrypoint, three references and two request examples |
| CLI integration | Default `skill show/install/check`, compiled identity and content hashes, explicit project/user scopes |
| Content preservation | Idempotent installs, full backups for unmodified updates, modified/unmanaged/symlink protection, staging outside skill discovery |
| Local verification | Passed; results below |
| GitHub quality and release preflight | Pending |
| Published archive, installer and local/GitHub audit | Pending |

## Local source verification

The source build is based on `ab16fa5` with the implementation uncommitted at the
time of testing. Its identity correctly reports a dirty debug build with default
features; these results are not release-binary evidence.

- 274 workspace Rust tests passed, zero failed; the one normally ignored native
  Godot compatibility test was also run explicitly and passed.
- Formatting, warning-free workspace Clippy and skill metadata validation passed.
- The offline CLI contract script passed 26 cases across 66 command invocations.
  It checked project/user installs, read-only status, idempotence, modified and
  unknown contents, symlinks, invalid manifests, path traversal, an actual content
  update and its full backup, invalid arguments and standalone-binary installation.
- The installed, unedited static request example produced a valid local plan
  using synthetic PNGs with zero estimated Provider requests.
- Codex Desktop 0.153.4's actual `skills/list` endpoint discovered exactly one
  enabled `forge-use` in each independently isolated project and user scope.
  This was a local discovery check with no model request or existing user config.
- Independent source review found no blocking issues in install/update recovery,
  discovery paths, identity, CLI routing or build-change monitoring.

See [local results](artifacts/forge-cli-skill-20260908/local.json). The reproducible
CLI test is `python3 scripts/test-cli-skill.py --forge /absolute/path/to/forge`.
It preserves the public launcher symlink when invoking installed binaries.

## Release gates

Skill-only edits now trigger the quality workflow. Both source and actual archive
checks exercise the bundled skill. Release packaging verifies fresh installation,
same-version reinstall and upgrade from v0.3.0, including local static and
animation delivery with Godot. Skill backups and staging are outside
`.agents/skills`, so Codex does not discover old copies as duplicate skills.

No private game art, game toolchain locks or existing import receipts are changed
by this work. Public documentation keeps character animation experimental and
requires separate Codex image-generation capability and Godot installation.
