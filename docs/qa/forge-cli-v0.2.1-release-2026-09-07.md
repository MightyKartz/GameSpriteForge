# Forge CLI 0.2.1 release verification — 2026-09-07

Scope: default-feature macOS Apple Silicon CLI. Experimental directional reuse remains a source-only helper. Existing reviewed assets and archived WIP are not promoted by this release. No real Provider calls were made by this release verification.

## Local checks

- Installer contract: fresh install, reinstall, upgrade, and failure preservation passed.
- Unsigned release / optional signing contract passed.
- Directional reuse integrity: 7 tests passed.
- Rust 1.98 formatting, 38 relative documentation links, shell examples, workflow YAML and shell syntax passed.
- Product contract using the existing main default binary passed. Godot 4.6.3 loaded 32 fixture frames and verified textures, loop flags, and timing. This is validation of the new test script, not evidence of a newly built release binary.
- The new Godot check also passed against 52 existing main-integration frames (12 right / 24 down / 16 up), covering nonuniform native timing.

The first version of the new Godot check assumed every manifest contained explicit frame durations. Local execution caught legacy FPS-only fixtures; the check now supports both formats and requires an explicit success marker. Superseded preflight 34097612810 and quality run 34097613204 were cancelled, not counted as passes.

## Remote and released-artifact evidence

| Check | Source | Result |
| --- | --- | --- |
| Candidate release preflight | [34097802584](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34097802584), head `57ef8d84a64aca6c436c54ba67448a603c08025b` | Passed; 242 default Rust tests, 0 failed, 0 ignored; installer, artifact and Godot checks passed |
| Candidate full quality matrix | [34097805718](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34097805718), same head | All six gates passed |
| Published RC | [34099232812](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34099232812), tag `v0.2.1-rc.1` | Passed; prerelease, not draft; stable latest remained `v0.2.0-cli.1` |
| Stable full quality matrix | [34099299508](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34099299508), head `b6be6b060919dfa9da6f02dff7c2eb98f1ebabea` | All six gates passed |

The six matrix gates are Rust quality, Character V2 contract, Character V2 full matrix,
static CLI contract, static five-style matrix, and experimental world assets. Optional
feature checks do not make those features part of the default release.

The published RC archive was downloaded from GitHub and passed SHA-256 verification,
fresh installation, reinstallation, upgrade from `v0.2.0-cli.1`, fixture product flows,
and Godot checks on this Mac. Its SBOM attestation verified against
`MightyKartz/GameSpriteForge/.github/workflows/release-cli.yml` with predicate
`https://cyclonedx.org/bom`; its BUILD_INFO commit matches the RC tag.

The RC Forge binary is byte-identical to the preflight binary. That binary also rebuilt
the three-direction engineering candidate from the existing recovery corpus: 52 frames,
13 installed textures, 7 text resources, exact pixel checks, native timing, unchanged
sources, and zero Provider requests. This new engineering candidate remains visually
unreviewed and is not promoted to production.

The [stable release pipeline 34100750035](https://github.com/MightyKartz/GameSpriteForge/actions/runs/34100750035)
passed on tag `v0.2.1`, commit `e6045c12623410a18ede1cba3744a0d6e8554d8c`.
This run passed 242 default Rust tests with 0 failures and 0 ignored tests, plus the
installer, product, reuse integrity, and packaged Godot checks. GitHub now resolves
Latest to `v0.2.1` (not draft, not prerelease).

The downloaded stable archive passed the local artifact contract, including upgrade
from RC; the release CI separately verified upgrade from the previous stable version.
The archive's attestation, BUILD_INFO version/commit, and SBOM component version all
matched. The released installer matches the repository's installer byte-for-byte.
The default installer was also executed without a version override, test mode, or URL
override in an isolated user directory: it downloaded and installed `forge 0.2.1`.

That installed stable binary rebuilt and verified the existing 52-frame directional
corpus in Godot 4.6.3. All 13 textures, 7 text resources, pixel comparisons, source
immutability checks, and runtime checks passed, with zero Provider requests. The
new engineering output is not a new human approval or production promotion.

Archive SHA-256:

- RC: `05ff4f021a29892e57829f21099e6b4d7b2bb11074508f76ec28335bc8d9bb35`
- Stable: `2a3f21dd82a5ff273de15413e5d8d5cb42505a8ac17b860d6d44de863058189e`

Build metadata and compact verification evidence are in
[summary.json](artifacts/forge-cli-v0.2.1-release-2026-09-07/summary.json).
The English and Chinese README blob hashes were checked against GitHub and match
local content.

These checks use isolated installation directories on macOS, not a fresh macOS user account. Automated engine checks do not replace candidate-specific human visual review. Binaries remain unsigned and not notarized.

## Reproduction

```bash
# Download the current and baseline archives from their GitHub Releases first.
bash scripts/test-cli-release-artifact.sh \
  /absolute/stable-artifacts v0.2.1 /absolute/rc-artifacts v0.2.1-rc.1

gh attestation verify /absolute/stable-artifacts/forge-aarch64-apple-darwin.zip \
  --repo MightyKartz/GameSpriteForge \
  --signer-workflow MightyKartz/GameSpriteForge/.github/workflows/release-cli.yml \
  --predicate-type https://cyclonedx.org/bom
```

The artifact test requires macOS Apple Silicon, jq, and Godot 4.6.x on PATH. It verifies
checksums, preserved previous binaries, version metadata, arm64 executable types,
FFmpeg/FFprobe execution, fixture workflows, and installed Godot textures/loop/timing.
Full archives and temporary Job/Plan stores remain outside Git under ignored local
release-candidate directories. See the [release notes](../releases/v0.2.1.md) for
installation and source-only reuse helper requirements.
