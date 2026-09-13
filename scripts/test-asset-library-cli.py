#!/usr/bin/env python3
"""Offline resource-library CLI contracts using isolated synthetic metadata."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile


def inventory(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file()}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--forge', type=Path, required=True)
    parser.add_argument('--legacy-forge', type=Path)
    args = parser.parse_args()
    forge = args.forge.absolute()
    with tempfile.TemporaryDirectory(prefix='forge-library-contract-') as directory:
        root = Path(directory)
        env = dict(os.environ, FORGE_JOB_STORE=str(root / 'jobs'), FORGE_PLAN_STORE=str(root / 'plans'))

        def run(*command, executable=forge, ok=True):
            result = subprocess.run([str(executable), *map(str, command), '--json'], env=env,
                                    capture_output=True, text=True, encoding='utf-8')
            if ok:
                assert result.returncode == 0, result.stdout + result.stderr
                value = json.loads(result.stdout)
                assert value['ok'], value
                return value['data']
            assert result.returncode != 0, result.stdout
            return result

        project = root / '素材 library'
        header = run('project', 'init', '--path', project, '--name', 'Local assets', '--local-assets')
        assert header['schemaVersion'] == '3'
        assert not (project / 'forge-project.json').exists()
        before = inventory(project)
        assert run('project', 'inspect', '--project', project)['projectId'] == header['projectId']
        assert run('asset', 'list', '--project', project) == []
        run('project', 'init', '--path', project, '--name', 'Other', '--local-assets', ok=False)
        assert inventory(project) == before
        if args.legacy_forge:
            failure = run('asset', 'list', '--project', project,
                          executable=args.legacy_forge.absolute(), ok=False)
            assert 'unsupported catalog schemaVersion 3' in failure.stdout

        legacy = root / 'legacy'
        (legacy / '.forge').mkdir(parents=True)
        catalog = legacy / '.forge/catalog.json'
        original = json.dumps({'schemaVersion': '1', 'updatedAt': '2026-01-01T00:00:00Z',
                               'assets': {'old': {'assetId': 'old', 'name': 'Old asset', 'kind': 'prop_set',
                               'packPath': 'missing.gsfpack', 'packSha256': '1' * 64, 'sourceJobId': 'legacy-job',
                               'workflow': 'static-set@1.0.0', 'createdAt': '2026-01-01T00:00:00Z'}}}).encode()
        catalog.write_bytes(original)
        before = inventory(legacy)
        preview = run('project', 'migrate-assets', '--project', legacy, '--dry-run')
        assert inventory(legacy) == before
        assert preview['assets'][0]['issues']
        assert preview['catalogSha256'] == hashlib.sha256(original).hexdigest()
        run('project', 'migrate-assets', '--project', legacy, '--apply', '--expected-sha256', '0' * 64, ok=False)
        assert catalog.read_bytes() == original
        run('project', 'migrate-assets', '--project', legacy, '--apply', '--expected-sha256', preview['expectedSha256'])
        assert (legacy / '.forge/library/backups' / (preview['catalogSha256'] + '.json')).read_bytes() == original
        before = inventory(legacy)
        found = run('asset', 'inspect', '--project', legacy, '--id', 'old')
        assert found['sourceJobId'] == 'legacy-job'
        assert 'reviewedAt' not in found
        assert inventory(legacy) == before
        assert not (root / 'jobs').exists() and not (root / 'plans').exists()
        build = run('doctor')['build']
        print(json.dumps({'passed': True, 'build': build, 'legacyBinaryChecked': bool(args.legacy_forge),
                          'checks': ['local_init', 'readonly_query', 'migration_preview', 'stale_preview',
                                     'legacy_backup', 'unknown_evidence', 'no_jobs_or_provider_requests']}))


if __name__ == '__main__':
    main()
