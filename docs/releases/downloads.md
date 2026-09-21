# Choose a Forge download / 选择下载文件

Choose one installer from the release page's download table:

| Platform | File | Use |
| --- | --- | --- |
| macOS Apple Silicon | `forge-installer.sh` | Run `FORGE_VERSION=vX.Y.Z sh forge-installer.sh` with the version shown on the release page. It downloads and verifies that release's full package. |
| Windows x64 | `forge-windows-installer.zip` | Extract it, then run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\install-windows.ps1` from the extracted directory. The full package and checksum are already included. |

Keep the extracted Windows installer files together. For fresh installation,
upgrades and selecting an install directory, see the
[Windows guide](windows-portable.md). Godot is installed separately.

## Advanced and supporting files

Releases assembled by the current workflow contain eight uploaded assets:

- The two recommended installers above.
- `forge-aarch64-apple-darwin.zip` and its `.sha256`: the macOS payload, also used
  by the online installer. Its executable is under `forge-dist/bin/forge`.
- `forge-x86_64-pc-windows-msvc.zip` and its `.sha256`: the Windows portable
  payload for existing automation or direct use of its `forge.cmd` launcher.
  It is already inside the recommended Windows installer ZIP.
- `forge-windows-installer.zip.sha256`: optional outer ZIP verification.
- `forge-source-and-notices.zip`: exact FFmpeg/zlib sources, native licenses,
  build recipes, macOS-target Rust SBOM, paired-package verification and an
  internal checksum manifest. This is not required to install or run Forge.

The source bundle deduplicates the FFmpeg/zlib tarballs into `sources/`; native
build/license evidence remains separated by platform. Native package bytes,
internal manifests and legacy archive/checksum names remain unchanged by
assembly. GitHub also supplies source-code ZIP/tar.gz links automatically;
those are repository snapshots, not installers, and are not included in the
eight uploaded assets.

Older releases such as v0.6.3 have separate source, license, SBOM and build-report
attachments. They remain available as originally published. New release notes
put the recommended installers first, include SHA-256 values and link detailed
verification documents to the exact release commit.

## 中文说明

macOS 用户下载 `forge-installer.sh`，按发布页给出的版本运行安装命令；脚本自动
下载并校验完整包。Windows 用户只下载 `forge-windows-installer.zip`，保持解压后的
文件在同一目录，再运行其中的安装脚本。无需把两个 Windows ZIP 都下载一遍。

当前发布流程将上传附件整理为 8 个。安装入口、便携包以及旧自动化依赖的文件名和
校验文件保持兼容；分散的源码、许可证、构建说明、SBOM 和验证报告合并到
`forge-source-and-notices.zip`，安装时无需下载。该包完整保留相关材料；Rust SBOM
目前仅针对 macOS 构建，不能当作 Windows 依赖清单。

v0.6.3 等历史发布仍保持原来的分散附件，不会被重写。GitHub 自动生成的 Source code
ZIP/tar.gz 是仓库源码快照，不是安装包。Godot 需另行安装。
