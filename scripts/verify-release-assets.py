#!/usr/bin/env python3
"""Check both native release packages before publishing the shared version."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import zipfile


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def verify_package(archive, target, version, commit):
    checksum = Path(str(archive) + '.sha256').read_text().split()
    if len(checksum) != 2 or checksum[1] != archive.name or checksum[0] != sha256(archive.read_bytes()):
        raise ValueError(f'{archive.name}: archive checksum mismatch')
    with zipfile.ZipFile(archive) as package:
        names = [entry.filename for entry in package.infolist() if not entry.is_dir()]
        if len(names) != len(set(names)):
            raise ValueError('duplicate ZIP entries')
        for name in names:
            path = PurePosixPath(name)
            if path.is_absolute() or '..' in path.parts or '\\' in name:
                raise ValueError('unsafe ZIP path')
        identities = [name for name in names if PurePosixPath(name).name == 'BUILD_INFO.json']
        if len(identities) != 1:
            raise ValueError('expected one package build identity')
        identity_path = identities[0]
        prefix = identity_path.removesuffix('BUILD_INFO.json')
        info = json.loads(package.read(identity_path))
        if (info['version'], info['commit'], info['target']) != (version, commit, target):
            raise ValueError('package version, commit or target mismatch')
        inventory = {}
        for line in package.read(prefix + 'MANIFEST.sha256').decode().splitlines():
            digest, name = line.split('  ', 1)
            if not re.fullmatch('[a-f0-9]{64}', digest) or name in inventory:
                raise ValueError('invalid payload manifest')
            if prefix + name not in names or sha256(package.read(prefix + name)) != digest:
                raise ValueError('payload checksum mismatch')
            inventory[name] = digest
        files = {name[len(prefix):] for name in names if name.startswith(prefix)}
        if files != set(inventory) | {'MANIFEST.sha256'}:
            raise ValueError('payload inventory is incomplete')
        binary = 'bin/forge.exe' if target == 'x86_64-pc-windows-msvc' else 'bin/forge'
        if binary not in inventory:
            raise ValueError('missing native CLI')
        if target == 'x86_64-pc-windows-msvc':
            build = info['build']
            if (build['gitCommit'], build['target'], build['dirty'], build['profile'], build['features']) != (
                    commit, target, False, 'release', []):
                raise ValueError('Windows package is not a clean default release')
            if info['binarySha256'] != inventory[binary]:
                raise ValueError('Windows binary identity mismatch')
            for script in ['install-windows.ps1', 'windows-package-common.ps1']:
                if (archive.parent / script).read_bytes() != package.read(prefix + script):
                    raise ValueError('published installer differs from tested package')
        return {'target': target, 'version': version, 'commit': commit,
                'archive': archive.name, 'archiveSha256': checksum[0],
                'binarySha256': inventory[binary], 'payloadFiles': len(inventory)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--macos', type=Path, required=True)
    parser.add_argument('--windows', type=Path, required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--commit', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    results = [verify_package(path, target, args.version, args.commit) for path, target in [
        (args.macos, 'aarch64-apple-darwin'), (args.windows, 'x86_64-pc-windows-msvc')]]
    args.output.write_text(json.dumps({'passed': True, 'packages': results}, indent=2) + '\n')
    print(args.output.read_text())


if __name__ == '__main__':
    main()
