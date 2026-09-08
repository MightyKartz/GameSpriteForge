# Bundled Forge skill delivery — 2026-09-08

Goal: distribute the self-contained `forge-use` workflow with the CLI, install it
into a selected Codex scope, preserve user content, and verify the downloaded
release. v0.3.1 is published; asset processing contracts stay unchanged.

| Step | Result |
| --- | --- |
| Self-contained skill | Six files with thirteen valid internal Markdown links; entrypoint, three references and two request examples |
| CLI integration | Default `skill show/install/check`, compiled identity and content hashes, explicit project/user scopes |
| Content preservation | Idempotent installs, full backups for unmodified updates, modified/unmanaged/symlink protection, staging outside skill discovery |
| Local verification | Passed; results below |
| GitHub quality and release preflight | Passed; PR #26 merged at `1516bfa` |
| Published archive and installer | Passed; v0.3.1 downloaded, audited and installed through the normal HTTPS installer |

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

## Independent use of the installed skill

A separate agent read only the installed entrypoint, static reference and example
to deliver two existing synthetic PNG props into an isolated Godot project. Both
Jobs succeeded, the Pack validated, and the source/installed hashes matched their
respective evidence. Godot 4.6.3 loaded two textures, instantiated two scenes and
verified their anchors and linear filters. Plans and completed reports confirmed
zero Provider requests. No missing Forge command or field required guessing.
This establishes structural usability, not human art approval. An initial type
annotation error in the newly authored Godot validation script was corrected;
the successful rerun is recorded separately from that harness error. See
[the forward-use result](artifacts/forge-cli-skill-20260908/forward.json).

The Provider reference's inline Style spec and bundled icon spec were also used
unchanged in a separate `fixture` project. Style and icon Jobs succeeded and the
resulting Pack validated. Their three simulated fixture requests are distinct
from real external Provider requests, which remained zero. See
[the example result](artifacts/forge-cli-skill-20260908/provider-example.json).

## Release gates

The [PR quality workflow](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34191672913)
passed all eight matrix gates and the 26 skill contracts. The
[release preflight](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34191684942)
passed the source, clean release identity, product, actual archive, SBOM and upload
gates. Its packaged public launcher executed all 26 skill cases and 66 commands.
Both tested the same six-file content hash. The preflight's clean binary was built
from `80fa6dc`; it is distinct from the final tag's merge commit. See
[preflight results](artifacts/forge-cli-skill-20260908/preflight.json).

[PR #26](https://github.com/MightyKartz/GameSpriteForge/pull/26) merged at
`1516bfa4def9fb425d9463b877adb5c7f7493733`, which is the v0.3.1 tag target.
The final tag workflow and independently downloaded release both passed.

Skill-only edits now trigger the quality workflow. Both source and actual archive
checks exercise the bundled skill. Release packaging verifies fresh installation,
same-version reinstall and upgrade from v0.3.0, including local static and
animation delivery with Godot. Skill backups and staging are outside
`.agents/skills`, so Codex does not discover old copies as duplicate skills.

## Published release verification

The [v0.3.1 release](https://github.com/MightyKartz/GameSpriteForge/releases/tag/v0.3.1)
was published after the
[tag workflow](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34192797493)
passed all checks. All nine public assets were downloaded independently. Archive
checksum, ZIP CRC, payload manifest, local/remote annotated tag, exact tagged
installer bytes and compiled identity matched. The CycloneDX attestation was
verified for this repository, workflow, tag and commit; the downloaded SBOM's
complete JSON matched the verified predicate.

| Identity | SHA-256 |
| --- | --- |
| Release archive | `9cd68880717db49c15a489f113329cc12eeec72f8ab04c44e0bd42ba2ad03845` |
| Forge binary | `eca2580903704ef08c8dea0c0d5808913dac37d44831d4de143cabd742e6d0ca` |
| Embedded skill content | `e5d93005c28e3d3f3b7463c2b89ed2e90ec30ec705eaac1db66abd0ce0935791` |

On the local machine, the downloaded archive passed fresh installation,
same-version reinstall and upgrade from v0.3.0, the 26 skill contracts, the
fixture product/Godot suite, three local static cases and five local animation
cases. External FFmpeg/FFprobe discovery was disabled for the entire artifact
check; calls used the public launcher without resolving it first.

The downloaded HTTPS installer then upgraded the normal `forge` command to
v0.3.1. Its binary hash, clean release identity, default features, embedded bundle
and same-payload helpers matched the audited release. Codex discovered and enabled
the skill in both independently isolated scopes using this normal launcher.
The previous v0.3.0 binary, shell profile and existing consumer toolchain locks
retained their original hashes. See [release results](artifacts/forge-cli-skill-20260908/release.json).

Only this QA documentation and its aggregate evidence are updated after the release
tag; the shipped implementation and embedded skill contents remain those verified
at `1516bfa`. Full local logs and synthetic fixtures are retained outside Git.

No private game art, game toolchain locks or existing import receipts are changed
by this work. Public documentation keeps character animation experimental and
requires separate Codex image-generation capability and Godot installation.
