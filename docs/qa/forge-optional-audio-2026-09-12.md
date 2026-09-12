# Optional audio workflow verification

Status: passed locally; source-build addition, not a released CLI archive.

Implementation commit: `f898309384a9c19c5f90245fb10e746b845ec27a`.
The final CLI/Godot and embedded-guide runs used a clean default-feature build of
that commit (`features: []`, debug, aarch64-apple-darwin, version 0.3.2).
Binary SHA-256: `cd44b09298a09f3f95ad01c6213400602b2bf94fd29536f8d93f4c4b5d90ef11`.
Godot: 4.6.3; local FFmpeg/FFprobe: Homebrew 8.0.1.

The [machine-readable summary](artifacts/forge-optional-audio-2026-09-12/summary.json)
retains build identity, checks, end-to-end command outcomes and native stream facts.
Full local logs and synthetic fixtures remain under `target/qa/` and are not shipped.

## Delivered behavior

- `audio tools list` and `audio tools doctor` describe ACE-Step and Stable Audio 3
  as optional external tools. Discovery checks only fixed file metadata in an
  explicitly supplied directory. It neither installs dependencies nor executes
  tools, reads credentials, downloads weights or claims model readiness/licensing.
- `audio inspect`, `audio import` and `plan prepare-audio` accept local WAVs through
  fingerprinted, single-use Plans and durable Jobs with zero Provider requests.
  Relative source paths and complete optional reviewed source hashes are supported.
- Trim, gain, fades, resampling and optional loop crossfade produce PCM16 WAVs.
  Float intermediates preserve headroom until final quantization. Original source
  copies, processing parameters, user-asserted origin and SHA-256 hashes remain in
  v4 audio Packs. Technical reports keep listening and license verification false.
- Godot installation reuses the existing transaction and registry. It saves native
  `AudioStreamWAV` `.res` files, verifies samples/timing/loop state in a separate
  engine process, and tracks owned WAV `.sample` caches. Usage exposes `audioPaths`.
  Portable receipts and read-only audits cover audio assets and detect drift.
- The embedded guide and optional skill share the maintained audio reference and
  request example. Existing releases and Sword's assets/toolchain pins are unchanged.

## Verification

Workspace regression passed (340 tests; 4 existing opt-in tests skipped), alongside
default and optional-feature Clippy, World CLI/resource checks, signing contract,
the CLI product contract, and 137 embedded guide/skill commands. Final affected
coverage passed: 3 audio Plan tests, 10 processing tests, all 40 Pack tests, the
native Godot fixture suite, and the 28-command CLI audio/receipt suite.

The final committed-build run proves both 48 kHz and 96 kHz audio import and native
delivery. Godot 4.6.3 rejects the extensible WAV header emitted by FFmpeg at high
rates. Forge therefore wraps the final PCM bytes in a classic PCM16 RIFF header;
tests verify byte preservation and native high-rate playback metadata. Canonical
Pack output rejects extensible headers while retained source WAVs may use them.

Negative checks cover changed/reused Plans, incomplete or wrong source locks,
invalid controls, cancellation cleanup, source/output corruption, inconsistent
crossfade/trim metadata, malformed WAV chunks, symlinks and extra Pack files.
Native tests reject missing/extra resources, changed PCM and loop metadata. CLI
tests detect native-resource/cache tampering and verify a relocated Pack/receipt
after removing the original Job store from its recorded path.

All audio was synthetic. No third-party model was run, no commercial eligibility
was asserted, and no listening acceptance or device-performance claim was made.
Published-package verification and GitHub CI are separate from this local evidence.
