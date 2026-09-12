# PR #32 / #33 integration corrections

PR #32 fixes layered project registration and detects kind drift during read-only
verification. PR #33 incorporates that corrected branch without merging main,
retains both Godot install routes, and dispatches shared-v4 Packs by asset type.
Audio resource URIs use forward slashes on Windows as well as macOS. Installed
Windows package CI now exercises the full synthetic audio delivery/receipt test.
PCM sample iteration passes Clippy 1.98.0 with warnings denied.

Validation: 373 workspace tests passed, 4 ignored; workspace and optional-feature
Clippy 1.98.0 passed. Native Godot 4.6.3 audio and layered delivery, registry drift
rejection, five sequence animation cases, 137 embedded guide/skill commands,
world-assets CLI/native contracts and signing contract passed. Both Pack types
are exercised using the same combined binary. No Provider generation was run.

An initial local test link ran out of disk space. Removed only the repository's
regenerable `target/debug/incremental` cache and reran the complete workspace
successfully with incremental compilation disabled.

See [machine-readable evidence](artifacts/pr-integration-fixes-2026-09-12.json)
for source/build identities and hashes. The final clean binary differs from the
pre-commit tested binary in compiled Git identity. GitHub checks validate each
pushed head; their latest status remains authoritative. Merge #32 before #33.


The first Windows packaged audio gate exposed a source-path check rejecting native
backslashes. Intake now accepts native Windows separators, including canonical
verbatim paths, while retaining protocol and WAV-content validation. A synthetic
absolute-path-with-spaces regression covers this path before FFprobe execution;
Windows CLI CI now also runs the audio plan/processing Rust tests.
