# Godot 4.7 compatibility

This change extends the source CLI after v0.5.0. It does not change the
published v0.5.0 packages or any consumer's installed executable or lock.

## Contracts

- Engine setup, doctor, preview and asset delivery share the 4.6.x/4.7.x
  version policy. Other minor versions remain unsupported.
- Managed downloads accept exactly 4.6.3 and 4.7.2, defaulting to 4.7.2.
  `--version` requires `--download`; selecting a local path is a separate route.
- The official standard engine and export-template archives use their pinned
  SHA-512 values from the corresponding `godotengine/godot-builds` release's
  `SHA512-SUMS.txt`. New downloads must report the selected engine version.
- Engine directories coexist. Reusing a managed engine checks its version and
  binary fingerprints, including the Windows console launcher's companion.
- Template installation and status follow the selected version. Initial minor
  releases such as `4.7.stable` and .NET template directory names are handled;
  managed .NET downloads are not added.
- Project locks still compare the complete Godot version and Forge package
  version. Cross-minor selection fails without rewriting the lock. Updating
  a lock remains an explicit migration after verification.

## Focused local verification

Development checks on macOS passed: formatting, workspace Clippy with warnings
denied, 14 Godot-related core unit tests, 29 CLI unit tests, 9 non-native delivery
tests and 41 Pack tests. The embedded-guide check passed 149 CLI calls with zero
Provider requests, and the updated forge-use skill passed its validator.

The official 4.7.2 engine was downloaded through the new CLI to an isolated
`FORGE_CONFIG_DIR`, passed archive verification and reported
`4.7.2.stable.official.ed1daf0bf`. The existing `/Applications/Godot.app`
installation was retained as 4.6.3. These are setup and structural results;
they do not claim visual, listening or device acceptance.

## Native CI gate

The Godot workflow runs on macOS and Windows, each with 4.6.3 and 4.7.2.
Its setup checks cover pinned engine/template downloads, repeated setup,
the default version and simultaneous retention of both engine versions.
Each job runs the following checks serially:

- Exact project-lock rejection when selecting the other real engine minor.
- Import, bounded runtime, interaction assertions and rendered screenshots.
- Source and save isolation, timeout, cancellation and expected failures.
- Standard-template export and actual exported startup, plus external relative
  custom templates and custom save directories.
- Native static, animation and audio delivery, install/update rollback,
  layered previews and the shared playback controller.

The synthetic screenshot fixture selects Godot's Dummy audio driver because
Windows hosted runners have no audio output device. Engine errors still fail
verification; this fixture does not assess listening or change user projects.

CI retains the selected engine identity, Forge build identity and binary
SHA-256 in its workflow summary, together with reports, logs and screenshots.
Consult the PR's completed checks and native artifacts for the tested commit;
a local macOS pass is not Windows evidence. Screenshots are render evidence,
not automatic artistic approval. The new source behavior must pass this matrix
before it becomes the recommendation in a future public release.
