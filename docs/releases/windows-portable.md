# Windows x64 portable distribution

The Windows package is experimental and unsigned. It contains the Rust MSVC CLI,
separate FFmpeg/FFprobe executables, exact helper source archives, license notices,
the source build recipe, compiled `BUILD_INFO.json`, and a complete SHA-256 file
inventory. Windows 10/11 x64 is the intended desktop environment. This workflow
uploads reviewable CI artifacts. From v0.4.0, the release workflow also calls this
same Windows job and publishes its verified package with the macOS package after
both native jobs pass.

## Use or install a verified archive

Download the single `forge-windows-installer.zip` from the same trusted GitHub
Release or CI artifact. It contains the verified package, its `.sha256`, the
installer and the shared installer support script. Extract it, then run the
included installer; it discovers and checks the bundled archive automatically.
Checksums establish byte identity; these artifacts do not have Authenticode signing.

```powershell
Expand-Archive .\forge-windows-installer.zip .\forge-windows
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\forge-windows\install-windows.ps1
& "$env:LOCALAPPDATA\GameSpriteForge\bin\forge.cmd" doctor --json
```

The installer supports Windows PowerShell 5.1 and PowerShell 7. It needs no admin
rights or symbolic-link privileges and does not change the system or user PATH.
Use `-InstallDirectory 'C:\Tools\Forge with spaces'` for another location. To install
from the audit package directly, pass `-Archive` and `-Sha256` explicitly. The
public entry point is `bin\forge.cmd`; keep it intact and call this path during
acceptance checks. A ZIP can also be extracted and its root `forge.cmd` used
directly, with the complete adjacent `bin` directory retained.

If Windows PowerShell blocks the downloaded script under its default execution
policy, first review the two trusted installer scripts, then invoke the installer
with `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install-windows.ps1`
and the same `-Archive`/`-Sha256` arguments. This changes policy for that process
only; it does not require changing a machine or user policy.

Installing the same archive is idempotent. Upgrading verifies the archive, rejects
unsafe ZIP paths/reparse points and mismatched file inventories, stores the new
payload below `releases`, then atomically replaces only its owned launcher. Old
payloads and launcher backups remain available. Modified launchers are preserved
and refused. The installer never imports game assets or changes another game's
pinned Forge executable. It rejects dirty/debug/feature builds unless the caller
explicitly supplies `-AllowDevelopmentBuild`; such a package is QA evidence, not
a release build.

To restore a retained version, run the installer again with that version's
verified archive and hash. This keeps the same validation and activation process.

## Godot and helper discovery

FFmpeg and FFprobe are bundled and discovered next to the actual Forge payload
executable, including when called through `bin\forge.cmd`. No external FFmpeg
installation is needed. Windows canonical paths reported by `doctor` may start
with `\\?\`; this is a normal Windows absolute-path representation.

Godot is not bundled. Point Forge at a verified Godot 4.6.x or 4.7.x console executable.
The managed downloader defaults to Godot 4.7.2; select 4.6.3 explicitly when an
existing project requires it. Keep new Godot versions in separate directories,
run `doctor` with the new path, and validate a preview before updating a game's
own toolchain pin.

Use the official [Godot 4.6.3 Windows archive](https://github.com/godotengine/godot-builds/releases/download/4.6.3-stable/Godot_v4.6.3-stable_win64.exe.zip)
for a project locked to that version, or run `forge setup godot --download --version 4.6.3 --templates --json`.

```powershell
$env:FORGE_GODOT_PATH = 'C:\Tools\Godot\Godot_v4.6.3-stable_win64_console.exe'
& "$env:LOCALAPPDATA\GameSpriteForge\bin\forge.cmd" doctor --json
```

Check `godotSupported`, `ffmpegPath`, `ffprobePath`, `build`, and `toolChecks`.
`platformSupported` identifies an implemented runtime platform; the separate
`distributionStatus` retains the experimental/unsigned status. Doctor does not
authenticate Providers. Missing Godot blocks native import/preview, but does not
prevent local PNG preparation, Pack validation or reading the embedded `guide`.
The optional `skill install` has its own platform support; successfully reading
the embedded guide does not establish installer support.

## Rebuild and verify

Source builds use Rust/Cargo, the Visual Studio C++ toolchain and Windows SDK.
The helper builder additionally needs PowerShell 7 and a `7z` CLI; it downloads
the pinned portable C compiler into its work directory and persists no environment
changes. Use a helper build directory without spaces (an FFmpeg source-build
constraint); archive and installation paths with spaces are explicitly tested.
Packaging and acceptance scripts require PowerShell 7. Every source compilation
extracts fresh source directories. A verified helper cache avoids recompilation;
pass `-ForceRebuild` to the helper builder to repeat compilation from pinned inputs.

```powershell
cargo build --locked --release -p forge-cli --no-default-features
./scripts/build-windows-ffmpeg.ps1 -WorkDirectory C:/build/forge-ffmpeg -Jobs 8
./scripts/package-windows.ps1 -Forge target/release/forge.exe `
  -Helpers C:/build/forge-ffmpeg/helpers -Output target/windows-dist/forge-windows.zip
./scripts/test-windows-package.ps1 -Archive target/windows-dist/forge-windows.zip `
  -Output target/qa/windows-package -Godot C:/Tools/Godot/Godot_v4.6.3-stable_win64_console.exe
```

The package test calls the installed public launcher with external helper search
disabled. It checks fresh installation, unchanged reinstall, bad-checksum refusal,
upgrade activation with retained backups, compiled identity and bundled helper
paths. It also runs real H.264 MP4 encoding with Media Foundation, PNG extraction
and GIF creation/probing. The installed-launcher preview test additionally checks
decoded frames, timing, cache reuse, source integrity and PNG resource review.
Pass `-PreviousArchive` for an actual old-binary upgrade. In the absence of a prior
Windows package, the test labels its older packaging-revision fixture explicitly:
the compiled version/commit are unchanged, and no older CLI compatibility is claimed.

## FFmpeg inputs and licenses

Forge builds unmodified [FFmpeg 8.1.2 source](https://ffmpeg.org/download.html)
with `--disable-gpl --disable-nonfree --disable-autodetect`, and explicitly links
static [zlib 1.3.2](https://zlib.net/). The build uses the pinned portable
[w64devkit 2.9.1 compiler](https://github.com/skeeto/w64devkit/releases/tag/v2.9.1).
These exact download bytes are checked before extraction:

| Input | SHA-256 |
| --- | --- |
| FFmpeg 8.1.2 `.tar.xz` | `464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c` |
| zlib 1.3.2 `.tar.xz` | `d7a0654783a4da529d1bb793b7ad9c3318020af77667bcae35f95d0e42a792f3` |
| w64devkit x64 2.9.1 `.7z.exe` | `9208c19755cd4964b7915b9afcf02c66d493a4c870c4b3e83f6c538d9c1237a5` |

The ZIP contains the exact FFmpeg and zlib source archives, LGPL-2.1 and zlib
license texts, MinGW runtime notices, build script, configure command and helper
binary hashes. Forge runs the helpers as separate programs. GPL-only codec
libraries and nonfree components are not enabled. Networking is disabled in the
helpers; Forge supplies local downloaded media. External x86 assembly is disabled
to avoid an additional assembler dependency; this can reduce video performance.
The normal built-in codecs and filters remain enabled, including native H.264
decoding, PNG and GIF processing. `--enable-mediafoundation` explicitly enables
Windows H.264 encoding despite disabled autodetection. x264/x265 encoding is not
included. Earlier helper builds lack this encoder. The native CI gate must use
the newly built helpers, then repeat export through the installed public launcher.
Windows must provide a working Media Foundation encoder; missing OS components
or unusable encoders produce an error, without automatic installation.

The recipe uses relative include/library paths, disables PE linker timestamps,
and fixes `SOURCE_DATE_EPOCH=0` so GNU strip does not reinsert the current time.
The [GNU binutils documentation](https://www.sourceware.org/binutils/docs/binutils/objcopy.html)
describes this timestamp behavior.
Pinned inputs make the build inspectable and repeatable; bit-for-bit reproduction
across different hosts has not been asserted. Toolchain provenance is in
`FFMPEG_BUILD.json`. Keep the full source/license inventory when redistributing;
the repository's existing commercial distribution review gate still applies.
