#!/usr/bin/env python3
"""Assemble a small public download set from unchanged, verified native packages."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import posixpath
import re
from urllib.parse import quote
import zipfile

spec = importlib.util.spec_from_file_location('release_assets', Path(__file__).with_name('verify-release-assets.py'))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)

MAC = 'forge-aarch64-apple-darwin.zip'
WINDOWS = 'forge-x86_64-pc-windows-msvc.zip'
INSTALLER = 'forge-windows-installer.zip'
SUPPORT = 'forge-source-and-notices.zip'
SOURCE_HASHES = {
    'ffmpeg-8.1.2.tar.xz': '464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c',
    'zlib-1.3.2.tar.xz': 'd7a0654783a4da529d1bb793b7ad9c3318020af77667bcae35f95d0e42a792f3',
}
REPOSITORY_FILES = ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'third_party/ffmpeg/BUILD.md',
                    'third_party/ffmpeg/PATCHES.md', 'scripts/build-windows-ffmpeg.ps1',
                    'docs/releases/windows-portable.md']
PUBLIC_NAMES = [MAC, MAC + '.sha256', 'forge-installer.sh', WINDOWS, WINDOWS + '.sha256',
                INSTALLER, INSTALLER + '.sha256', SUPPORT]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def payload(archive):
    with zipfile.ZipFile(archive) as package:
        identity, = [n for n in package.namelist() if n.endswith('BUILD_INFO.json')]
        prefix = identity.removesuffix('BUILD_INFO.json')
        return {n.removeprefix(prefix): package.read(n) for n in package.namelist()
                if n.startswith(prefix) and not n.endswith('/')}


def assemble(macos, windows, root, version, commit, output):
    if output.exists():
        raise ValueError('Choose a new output directory; existing downloads are not overwritten')
    results = [release.verify_package(path, target, version, commit) for path, target in [
        (macos / MAC, 'aarch64-apple-darwin'), (windows / WINDOWS, 'x86_64-pc-windows-msvc')]]
    mac_payload, win_payload = payload(macos / MAC), payload(windows / WINDOWS)
    source = (macos / 'ffmpeg-8.1.2-source.tar.xz').read_bytes()
    if digest(source) != SOURCE_HASHES['ffmpeg-8.1.2.tar.xz']:
        raise ValueError('macOS helper source hash mismatch')
    support = {name: (root / name).read_bytes() for name in REPOSITORY_FILES}
    for name, expected in SOURCE_HASHES.items():
        data = win_payload['sources/' + name]
        if digest(data) != expected:
            raise ValueError('Windows helper source hash mismatch: ' + name)
        support['sources/' + name] = data
    if source != support['sources/ffmpeg-8.1.2.tar.xz']:
        raise ValueError('Native helper source archives differ')
    for platform, files in [('macos', mac_payload), ('windows', win_payload)]:
        for name, data in files.items():
            if name.startswith('licenses/') or (name.startswith('sources/') and name.removeprefix('sources/') not in SOURCE_HASHES):
                support[platform + '/' + name] = data
        # Fail closed if a native build omitted its applicable license text.
        support[platform + '/licenses/FFMPEG-LGPL-2.1.txt'] = files['licenses/FFMPEG-LGPL-2.1.txt']
        support[platform + '/BUILD_INFO.json'] = files['BUILD_INFO.json']
    support['windows/licenses/ZLIB-LICENSE.txt'] = win_payload['licenses/ZLIB-LICENSE.txt']
    support['windows/FFMPEG_BUILD.json'] = win_payload['FFMPEG_BUILD.json']
    support['forge-sbom.cdx.json'] = (macos / 'forge-sbom.cdx.json').read_bytes()
    json.loads(support['forge-sbom.cdx.json'])
    support['release-verification.json'] = (json.dumps({'passed': True, 'packages': results}, indent=2) + '\n').encode()
    support['README.md'] = (f'''# Forge {version}: source, notices and verification

This archive is optional for installation. Download the installer for your OS
from the release page to use Forge.

- `sources/`: exact pinned FFmpeg and zlib source archives, shared by the native builds.
- `macos/` and `windows/`: native build identities, license texts and helper evidence.
- `windows/sources/`: retained Windows build scripts. The source tarballs are
  deduplicated into the top-level `sources/` directory; the original portable
  Windows package still contains its complete, unchanged source layout.
- `third_party/ffmpeg/BUILD.md` and `scripts/build-windows-ffmpeg.ps1`: build recipes.
- `forge-sbom.cdx.json`: Rust dependency SBOM generated for the macOS target;
  it is not a Windows-target SBOM.
- `release-verification.json`: archive and binary hashes for both native packages.
- `MANIFEST.sha256`: checksums of every file in this archive except the manifest itself.

安装时无需下载此附件，请在发布页选择对应系统的安装文件。
本附件集中保存对应源码、许可证、构建说明、SBOM 和双平台验证报告。
源码包集中在 sources/，原始 Windows 安装包中的源码布局保持不变。
''').encode()
    support['MANIFEST.sha256'] = ''.join(digest(data) + '  ' + name + '\n' for name, data in sorted(support.items())).encode()
    public = {name: (directory / name).read_bytes() for directory, names in [
        (macos, [MAC, MAC + '.sha256', 'forge-installer.sh']),
        (windows, [WINDOWS, WINDOWS + '.sha256', INSTALLER, INSTALLER + '.sha256'])] for name in names}
    # Preserve public launcher bytes and native archives already accepted by the native jobs.
    output.mkdir(parents=True)
    for name, data in public.items():
        (output / name).write_bytes(data)
    with zipfile.ZipFile(output / SUPPORT, 'x', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in sorted(support.items()):
            archive.writestr(name, data)
    return {name: digest((output / name).read_bytes()) for name in PUBLIC_NAMES}


def render_notes(notes, note_path, repository, version, commit, hashes):
    base = f'https://github.com/{repository}/releases/download/v{version}'
    def link(match):
        target = match.group(1)
        if re.match(r'[a-zA-Z][a-zA-Z0-9+.-]*:', target) or target.startswith(('#', '/')):
            return match.group(0)
        path = posixpath.normpath(posixpath.join(note_path.parent.as_posix(), target))
        if path.startswith('../'):
            raise ValueError('Release note link escapes the repository')
        return '](' + f'https://github.com/{repository}/blob/{commit}/' + quote(path, safe='/#') + ')'
    notes = re.sub(r'\]\(([^)\s]+)\)', link, notes)
    rows = '\n'.join(f'| `{name}` | `{value}` |' for name, value in hashes.items())
    return f'''## Download / 下载

Choose **one installer for your system**. The other files are optional.
请按系统选择一个安装文件，无需逐个下载 Assets 中的所有附件。

| System / 系统 | Recommended download / 推荐下载 | Run after download / 下载后操作 |
| --- | --- | --- |
| macOS Apple Silicon (M1/M2/M3/M4…) | [forge-installer.sh]({base}/forge-installer.sh) | `FORGE_VERSION=v{version} sh forge-installer.sh` (downloads the verified package / 自动下载并校验完整包) |
| Windows x64 | [forge-windows-installer.zip]({base}/{INSTALLER}) | Extract; open PowerShell in that folder and run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\\install-windows.ps1` / 解压后在该目录运行命令 |

Windows users need only the installer ZIP, which already contains the full
portable package and checksum. Godot is installed separately. Packages remain
unsigned; macOS is not notarized; Windows support is experimental.
Windows 安装 ZIP 已包含完整便携包和校验文件。Godot 需另行安装；当前安装包未签名，
macOS 未公证，Windows 支持仍为实验性。

<details>
<summary>Portable archives, source and checksums / 便携包、源码与校验</summary>

- [macOS portable archive]({base}/{MAC}) · [SHA-256]({base}/{MAC}.sha256)
- [Windows portable archive]({base}/{WINDOWS}) · [SHA-256]({base}/{WINDOWS}.sha256)
- [Windows installer checksum]({base}/{INSTALLER}.sha256)
- [Source, licenses, build recipes, SBOM and verification]({base}/{SUPPORT}) — optional for installation / 安装时无需下载。

Existing automation-facing archive names and checksum files are preserved.
GitHub-generated source-code ZIP/tar.gz downloads contain Forge source, not installed executables.
旧版自动安装所用文件名与校验文件保持兼容；GitHub 自动提供的 Source code 压缩包不是安装包。

| File / 文件 | SHA-256 |
| --- | --- |
{rows}

</details>

---

{notes}'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for flag in ['macos', 'windows', 'output', 'notes', 'notes-output']:
        parser.add_argument('--' + flag, type=Path, required=True)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument('--repository', default='MightyKartz/GameSpriteForge')
    parser.add_argument('--version', required=True)
    parser.add_argument('--commit', required=True)
    args = parser.parse_args()
    if not re.fullmatch(r'\d+\.\d+\.\d+(?:-rc\.[1-9]\d*)?', args.version):
        parser.error('version must be a release SemVer without the v prefix')
    if args.notes_output.exists():
        parser.error('notes output must be new')
    notes_path = args.notes.resolve().relative_to(args.root.resolve())
    notes = args.notes.read_text(encoding='utf-8')
    hashes = assemble(args.macos, args.windows, args.root, args.version, args.commit, args.output)
    with args.notes_output.open('x', encoding='utf-8') as stream:
        stream.write(render_notes(notes, notes_path, args.repository, args.version, args.commit, hashes))
    print(json.dumps({'assets': hashes, 'notes': str(args.notes_output)}, indent=2))


if __name__ == '__main__':
    main()
