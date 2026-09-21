"""Release gate regressions with synthetic ZIPs; no platform executable is run."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location('release_assets', Path(__file__).with_name('verify-release-assets.py'))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)
spec = importlib.util.spec_from_file_location('release_downloads', Path(__file__).with_name('assemble-release-downloads.py'))
downloads = importlib.util.module_from_spec(spec)
spec.loader.exec_module(downloads)


class ReleaseAssets(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def package(self, target='aarch64-apple-darwin', info_patch=None, corrupt=False, extra=False,
                filename='forge.zip', payload_extra=None):
        windows = target.startswith('x86_64')
        binary = 'bin/forge.exe' if windows else 'bin/forge'
        files = {binary: b'synthetic CLI', **(payload_extra or {})}
        info = {'version': '0.4.0', 'commit': 'abc', 'target': target}
        if windows:
            info.update(build={'gitCommit': 'abc', 'target': target, 'dirty': False,
                               'profile': 'release', 'features': []},
                        binarySha256=hashlib.sha256(files[binary]).hexdigest())
            for name in ['install-windows.ps1', 'windows-package-common.ps1']:
                files[name] = b'tested installer'
                (self.root / name).write_bytes(files[name])
        info.update(info_patch or {})
        files['BUILD_INFO.json'] = json.dumps(info).encode()
        files['MANIFEST.sha256'] = ''.join(hashlib.sha256(data).hexdigest() + '  ' + name + '\n'
                                           for name, data in files.items()).encode()
        if corrupt:
            files[binary] = b'changed bytes'
        if extra:
            files['unexpected'] = b'not inventoried'
        prefix = '' if windows else 'forge-dist/'
        path = self.root / filename
        with zipfile.ZipFile(path, 'w') as archive:
            for name, data in files.items():
                archive.writestr(prefix + name, data)
        Path(str(path) + '.sha256').write_text(hashlib.sha256(path.read_bytes()).hexdigest() + '  ' + filename + '\n')
        if windows:
            bundle = self.root / 'forge-windows-installer.zip'
            with zipfile.ZipFile(bundle, 'w') as archive:
                archive.writestr('forge-x86_64-pc-windows-msvc.zip', path.read_bytes())
                archive.writestr('forge-x86_64-pc-windows-msvc.zip.sha256', Path(str(path) + '.sha256').read_bytes())
                for name in ['install-windows.ps1', 'windows-package-common.ps1']:
                    archive.writestr(name, files[name])
            Path(str(bundle) + '.sha256').write_text(hashlib.sha256(bundle.read_bytes()).hexdigest() + '  forge-windows-installer.zip\n')
        return path

    def download_inputs(self):
        sources = {name: ('synthetic ' + name).encode() for name in downloads.SOURCE_HASHES}
        self.enterContext(patch.dict(downloads.SOURCE_HASHES, {name: downloads.digest(data) for name, data in sources.items()}))
        licenses = {'licenses/FFMPEG-LGPL-2.1.txt': b'synthetic LGPL'}
        self.package(filename=downloads.MAC, payload_extra=licenses)
        self.package('x86_64-pc-windows-msvc', filename=downloads.WINDOWS, payload_extra={
            **licenses, 'licenses/ZLIB-LICENSE.txt': b'synthetic zlib license',
            'FFMPEG_BUILD.json': b'{"synthetic": true}',
            'sources/build-helpers.sh': b'helper build recipe',
            **{'sources/' + name: data for name, data in sources.items()}})
        (self.root / 'ffmpeg-8.1.2-source.tar.xz').write_bytes(sources['ffmpeg-8.1.2.tar.xz'])
        (self.root / 'forge-installer.sh').write_bytes(b'unchanged installer')
        (self.root / 'forge-sbom.cdx.json').write_bytes(b'{"bomFormat":"CycloneDX"}')
        for name in downloads.REPOSITORY_FILES:
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(('synthetic ' + name).encode())

    def assemble(self):
        return downloads.assemble(self.root, self.root, self.root, '0.4.0', 'abc', self.root / 'public')

    def test_consolidated_downloads_preserve_native_bytes_and_sources(self):
        self.download_inputs()
        hashes = self.assemble()
        output = self.root / 'public'
        self.assertEqual(set(hashes), {p.name for p in output.iterdir()})
        self.assertEqual(len(hashes), 8)
        for name in downloads.PUBLIC_NAMES:
            data = (output / name).read_bytes()
            self.assertEqual(downloads.digest(data), hashes[name])
            if name != downloads.SUPPORT:
                self.assertEqual(data, (self.root / name).read_bytes())
        with zipfile.ZipFile(output / downloads.SUPPORT) as archive:
            inventory = dict(line.split('  ', 1)[::-1] for line in archive.read('MANIFEST.sha256').decode().splitlines())
            self.assertEqual(set(inventory) | {'MANIFEST.sha256'}, set(archive.namelist()))
            for name, expected in inventory.items():
                self.assertEqual(downloads.digest(archive.read(name)), expected)
            for name, expected in downloads.SOURCE_HASHES.items():
                self.assertEqual(downloads.digest(archive.read('sources/' + name)), expected)
            report = json.loads(archive.read('release-verification.json'))
            self.assertTrue(report['passed'])
            self.assertEqual(len(report['packages']), 2)
            self.assertIn('macos/licenses/FFMPEG-LGPL-2.1.txt', inventory)
            self.assertIn('windows/licenses/ZLIB-LICENSE.txt', inventory)
            self.assertIn('windows/sources/build-helpers.sh', inventory)
        with self.assertRaises(ValueError):
            self.assemble()

    def test_source_corruption_blocks_assembly_before_writing(self):
        self.download_inputs()
        (self.root / 'ffmpeg-8.1.2-source.tar.xz').write_bytes(b'different source')
        with self.assertRaisesRegex(ValueError, 'source hash mismatch'):
            self.assemble()
        self.assertFalse((self.root / 'public').exists())

    def test_missing_support_file_blocks_assembly(self):
        self.download_inputs()
        (self.root / 'forge-sbom.cdx.json').unlink()
        with self.assertRaises(FileNotFoundError):
            self.assemble()
        self.assertFalse((self.root / 'public').exists())

    def test_tampered_native_archive_blocks_assembly(self):
        self.download_inputs()
        (self.root / downloads.MAC).write_bytes(b'corrupted native archive')
        with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
            self.assemble()
        self.assertFalse((self.root / 'public').exists())

    def test_release_notes_choose_installers_and_pin_links(self):
        notes = downloads.render_notes('[QA](../qa/check.md) [external](https://example.com) [section](#test)',
                                       Path('docs/releases/v0.4.0.md'), 'owner/repo', '0.4.0-rc.1', 'abc', {'file.zip': 'hash'})
        self.assertIn('FORGE_VERSION=v0.4.0-rc.1 sh forge-installer.sh', notes)
        self.assertIn('/releases/download/v0.4.0-rc.1/forge-windows-installer.zip', notes)
        self.assertIn('https://github.com/owner/repo/blob/abc/docs/qa/check.md', notes)
        self.assertIn('[external](https://example.com)', notes)
        self.assertIn('[section](#test)', notes)
        self.assertIn('forge-source-and-notices.zip', notes)

    def test_both_native_layouts(self):
        for target in ['aarch64-apple-darwin', 'x86_64-pc-windows-msvc']:
            with self.subTest(target=target):
                result = release.verify_package(self.package(target), target, '0.4.0', 'abc')
                self.assertEqual(result['target'], target)

    def test_wrong_identity(self):
        for patch in [{'version': '0.3.2'}, {'commit': 'other'}, {'target': 'other'}]:
            with self.subTest(patch=patch), self.assertRaises(ValueError):
                release.verify_package(self.package(info_patch=patch), 'aarch64-apple-darwin', '0.4.0', 'abc')

    def test_corrupt_or_unlisted_payload(self):
        for options in [{'corrupt': True}, {'extra': True}]:
            with self.subTest(options=options), self.assertRaises(ValueError):
                release.verify_package(self.package(**options), 'aarch64-apple-darwin', '0.4.0', 'abc')

    def test_wrong_archive_checksum(self):
        path = self.package()
        Path(str(path) + '.sha256').write_text('0' * 64 + '  forge.zip\n')
        with self.assertRaises(ValueError):
            release.verify_package(path, 'aarch64-apple-darwin', '0.4.0', 'abc')

    def test_windows_development_build(self):
        target = 'x86_64-pc-windows-msvc'
        path = self.package(target, {'build': {'gitCommit': 'abc', 'target': target,
                            'dirty': True, 'profile': 'release', 'features': []}})
        with self.assertRaises(ValueError):
            release.verify_package(path, target, '0.4.0', 'abc')

    def test_windows_installer_drift(self):
        target = 'x86_64-pc-windows-msvc'
        path = self.package(target)
        (self.root / 'install-windows.ps1').write_bytes(b'different installer')
        with self.assertRaises(ValueError):
            release.verify_package(path, target, '0.4.0', 'abc')

    def test_windows_single_download_bundle(self):
        target = 'x86_64-pc-windows-msvc'
        result = release.verify_package(self.package(target), target, '0.4.0', 'abc')
        self.assertEqual(result['target'], target)

        bundle = self.root / 'forge-windows-installer.zip'
        bundle.unlink()
        Path(str(bundle) + '.sha256').unlink()
        with self.assertRaises(FileNotFoundError):
            release.verify_package(self.root / 'forge.zip', target, '0.4.0', 'abc')

        self.package(target)
        with zipfile.ZipFile(bundle) as archive:
            self.assertEqual(
                {entry.filename for entry in archive.infolist() if not entry.is_dir()},
                {'forge-x86_64-pc-windows-msvc.zip', 'forge-x86_64-pc-windows-msvc.zip.sha256',
                 'install-windows.ps1', 'windows-package-common.ps1'})


if __name__ == '__main__':
    unittest.main()
