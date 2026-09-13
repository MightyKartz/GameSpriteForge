#!/usr/bin/env python3
"""Offline resource-library CLI contracts using isolated synthetic metadata."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

from jsonschema import Draft202012Validator
from referencing import Registry, Resource


def inventory(root):
    return {p.relative_to(root).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in root.rglob('*') if p.is_file()}


def intake_validators():
    schemas = Path(__file__).resolve().parents[1] / 'schemas'
    documents = [json.loads((schemas / name).read_text(encoding='utf-8'))
                 for name in ['asset-intake.schema.json', 'asset-library-intake.schema.json']]
    registry = Registry().with_resources((doc['$id'], Resource.from_contents(doc)) for doc in documents)
    for doc in documents:
        Draft202012Validator.check_schema(doc)
    return [Draft202012Validator(doc, registry=registry) for doc in documents]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--forge', type=Path, required=True)
    parser.add_argument('--legacy-forge', type=Path)
    args = parser.parse_args()
    forge = args.forge.absolute()
    validators = intake_validators()
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

        media = root / 'media'
        media.mkdir()
        (media / 'source.bin').write_bytes(b'local fixture')
        scan_file = root / 'scan.json'
        before = inventory(project)
        scan = run('asset', 'scan', '--project', project, '--root', media, '--out', scan_file)
        assert inventory(project) == before and len(scan['items']) == 1
        for validator in validators:
            validator.validate(scan)
        run('asset', 'scan', '--project', project, '--root', media, '--out', scan_file, ok=False)
        registered = run('asset', 'register', '--project', project, '--input', scan_file)
        before = inventory(project)
        assert run('asset', 'register', '--project', project, '--input', scan_file)[0]['outcome'] == 'existing'
        assert inventory(project) == before
        found = run('asset', 'search', '--project', project, '--kind', 'file', '--limit', 1)
        assert found['total'] == 1 and found['items'][0]['status'] == 'available'
        assert run('asset', 'history', '--project', project, '--id', registered[0]['assetId'])[0]['revision'] == registered[0]['revision']
        assert run('asset', 'list', '--project', project) == []
        assert inventory(project) == before
        (media / 'source.bin').write_bytes(b'drift')
        run('asset', 'register', '--project', project, '--input', scan_file, ok=False)
        assert inventory(project) == before
        assert run('asset', 'search', '--project', project, '--status', 'changed')['total'] == 1

        # Both public schema paths accept real scan output and all supported item
        # fields, with or without scan-only observations. Unknown fields fail
        # validation and CLI parsing before any registration becomes visible.
        (media / 'source.bin').write_bytes(b'local fixture')
        full = copy.deepcopy(scan)
        full['items'][0].update(assetId='schema-one', purpose='battle', variant='combat',
                                role='source', parentRevisions=[registered[0]['revision']],
                                origin={'tool': 'External editor', 'model': 'Fixture',
                                        'license': 'User assertion', 'notes': 'Original notes'},
                                tags=['fixture'], newRevision=True)
        second = copy.deepcopy(full['items'][0])
        second['assetId'] = 'schema-two'
        full['items'].append(second)
        for validator in validators:
            validator.validate(full)
        batch_file = root / 'schema-batch.json'
        batch_file.write_text(json.dumps(full), encoding='utf-8')
        assert len(run('asset', 'register', '--project', project, '--input', batch_file)) == 2
        full.pop('issues', None)
        full['items'].pop()
        for validator in validators:
            validator.validate(full)
        batch_file.write_text(json.dumps(full), encoding='utf-8')
        assert run('asset', 'register', '--project', project, '--input', batch_file)[0]['outcome'] == 'existing'
        full['items'][0]['unsupportedField'] = True
        assert all(list(validator.iter_errors(full)) for validator in validators)
        batch_file.write_text(json.dumps(full), encoding='utf-8')
        before = inventory(project)
        run('asset', 'register', '--project', project, '--input', batch_file, ok=False)
        assert inventory(project) == before

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
                                     'legacy_backup', 'unknown_evidence', 'scan_register_search_history', 'intake_schema_contracts', 'no_jobs_or_provider_requests']}))


if __name__ == '__main__':
    main()
