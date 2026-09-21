# Survival feedback implementation verification

Implementation: `a92fe2360f83c689842c0238deeb1a361b1df363`.
PR: [#56](https://github.com/MightyKartz/GameSpriteForge/pull/56).
Plan: [scope and acceptance](../architecture/survival-feedback-implementation.md).
Evidence: [compact JSON](artifacts/survival-delivery-implementation-2026-09-21.json).

## Tested executable

Built from the clean implementation commit with default features disabled:

```sh
CARGO_TARGET_DIR=/Volumes/Untitled/Dev/forge-survival-target CARGO_INCREMENTAL=0 \
  cargo build --locked -p forge-cli --no-default-features -j 2
```

Executable: `/Volumes/Untitled/Dev/forge-survival-target/debug/forge`.
`doctor` identity: version 0.6.3, commit above, `dirty:false`, `features:[]`,
`aarch64-apple-darwin`, debug. SHA-256:
`4e4289ff1eca2875c09bf43705083059a26ed0e2cefb8e6d5e5284a61f97d86d`.
Godot: `/Applications/Godot.app/Contents/MacOS/Godot`,
`4.7.2.stable.official.ed1daf0bf`.

The build target was moved to an isolated external directory after the initial
local build encountered insufficient disk space. No pre-existing caches or
consumer files were removed. Test projects remained isolated on local storage;
only the explicit storage probes used ExFAT.

## Results

- Rust formatting and workspace/all-target Clippy with `-D warnings` passed.
- 72 focused Rust tests passed: Godot/schema/transactions library (16), storage
  (2), delivery audit (6), install transaction integration (5), static preparation
  (9), source matte (5), CLI/receipt/guide unit tests (29). Five existing native
  transaction tests remained ignored in that Cargo invocation; the separate
  native example test below exercised actual Godot installation and failure.
- Complete embedded-guide installation/update/protection verification passed:
  12 resources and 155 CLI calls. Both affected skill validators passed.
- The shipped Python example passed 11 checks using synthetic PNGs: destination
  probe cleanup, wrong-binary rejection, required source locks, embedded example,
  preservation of a concurrently-created output directory, rejection of an ExFAT
  project before production, successful native delivery, zero Provider requests,
  rejection of existing output, retained receipt verification without JobStore,
  and preparation retention/install rollback after a project script parse error.
- On native ExFAT, exclusive publication and hard links reported unsupported
  (OS error 45); replacement, existing-file protection and file locking probes
  succeeded. The temporary probe directory was removed and the sentinel retained.

The native runner deliberately removed only its own synthetic successful
JobStore before verifying the retained Pack, standard receipt and final project.
The injected failing project reported `godot_project_import_failed` with log
paths and recovery actions. No accepted visual review was fabricated.

Reproduction (use new output directories):

```sh
python3 scripts/test-survival-feedback-cli.py \
  --forge /absolute/built/forge --godot /absolute/Godot \
  --storage-root /absolute/existing-exfat-directory --expect-storage-unsupported \
  --output /absolute/new-local-test-output
python3 scripts/test-cli-skill.py --forge /absolute/built/forge \
  --output /absolute/new-guide-test-output
```

Omit the storage arguments when no unsupported external volume is available.
The new tests are included in the existing macOS/Windows, Godot 4.6.3/4.7.2 CI
matrix; see the PR checks for remote outcomes rather than inferring them from
the local run. This change does not provide full transactional ExFAT delivery,
release-package acceptance, device playback or artistic approval. The consumer
Survival checkout, pinned executable and existing receipts were not modified.
