"""Release gate regressions with synthetic ZIPs; no platform executable is run."""
import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('release_assets', Path(__file__).with_name('verify-release-assets.py'))
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseAssets(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def package(self, target='aarch64-apple-darwin', info_patch=None, corrupt=False, extra=False):
        windows = target.startswith('x86_64')
        binary = 'bin/forge.exe' if windows else 'bin/forge'
        files = {binary: b'synthetic CLI'}
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
        path = self.root / 'forge.zip'
        with zipfile.ZipFile(path, 'w') as archive:
            for name, data in files.items():
                archive.writestr(prefix + name, data)
        Path(str(path) + '.sha256').write_text(hashlib.sha256(path.read_bytes()).hexdigest() + '  forge.zip\n')
        return path

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


if __name__ == '__main__':
    unittest.main()
