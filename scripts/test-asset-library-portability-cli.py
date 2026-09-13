#!/usr/bin/env python3
"""Produce/consume the same public media bundle across actual CI operating systems."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import struct
import subprocess
import tempfile
import wave
from PIL import Image, ImageDraw


def save(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n', encoding='utf-8')


def native_pack_hash(root):
    digest = hashlib.sha256(b'forge-directory-hash-v2\0')
    # Rust PathBuf ordering compares components, not a flattened slash string.
    paths = sorted((p.relative_to(root) for p in root.rglob('*') if p.is_file()), key=lambda p: p.parts)
    for path in paths:
        name = str(path).encode('utf-8')
        content = (root / path).read_bytes()
        digest.update(b'file\0' + struct.pack('<Q', len(name)) + name + struct.pack('<Q', len(content)) + content)
    return digest.hexdigest()


def runner(forge, root, godot):
    env = dict(os.environ, FORGE_JOB_STORE=str(root / 'jobs'), FORGE_PLAN_STORE=str(root / 'plans'))
    if godot:
        env['FORGE_GODOT_PATH'] = str(godot)
    def run(*args, ok=True):
        result = subprocess.run([str(forge), *map(str, args), '--json'], env=env, capture_output=True,
                                text=True, encoding='utf-8', timeout=180)
        if not ok:
            assert result.returncode != 0, result.stdout
            return None
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        data = json.loads(result.stdout)
        assert data['ok'], data
        return data['data']
    return run


def install(run, root, library, lock):
    game = root / 'game'
    game.mkdir()
    (game / 'project.godot').write_text('config_version=5\n[application]\nconfig/name="Portable fixture"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n', encoding='utf-8')
    plan = run('godot', 'plan-install', '--library', library, '--asset-id', 'fixture.props', '--asset-lock', lock,
               '--project', game, '--asset-key', 'fixture-props')
    assert 'whole_pack' in json.dumps(plan) and 'coin' in json.dumps(plan)
    job = run('plan', 'execute', '--token', plan['token'], '--wait')
    assert job['lifecycle_state'] == 'succeeded', job
    usage = run('job', 'report', '--id', job['job_id'])
    assert usage['providerRequestCount'] == 0 and not usage['providerRequestOccurred']
    audit = run('project', 'verify-assets', '--project', library)['audit']
    assert any(i['status'] == 'verified_current_installation' for i in audit['installations']), audit
    return job['job_id']


def produce(forge, out, godot):
    run = runner(forge, out, godot)
    sources = out / 'sources'
    sources.mkdir()
    image = Image.new('RGBA', (64, 64))
    ImageDraw.Draw(image).ellipse((16, 8, 48, 56), fill='#eab34c')
    image.save(sources / 'coin.png')
    image.save(sources / '\u52a8\u753b preview.gif', save_all=True, append_images=[Image.new('RGBA', (64, 64), '#385f4b')], duration=[70, 190], loop=0)
    with wave.open(str(sources / 'cue.wav'), 'wb') as audio:
        audio.setparams((1, 2, 8000, 0, 'NONE', 'not compressed'))
        audio.writeframes(b'\0\0' * 1600)
    request = out / 'static.json'
    save(request, {'schemaVersion': '1', 'kind': 'prop_set', 'id': 'portable-props', 'name': 'Portable props',
                   'license': 'CC0-1.0', 'sampling': 'nearest', 'canvasSize': 64,
                   'items': [{'id': 'coin', 'name': 'Coin', 'path': str(sources / 'coin.png')}]})
    plan = run('plan', 'prepare-static', '--request', request)
    job = run('plan', 'execute', '--token', plan['token'], '--wait')
    assert job['lifecycle_state'] == 'succeeded'
    assert run('job', 'report', '--id', job['job_id'])['providerRequestCount'] == 0
    pack = Path(next(a['path'] for a in job['artifacts'] if a['kind'] == 'gsfpack'))
    legacy_hash = native_pack_hash(pack)
    library = out / 'library'
    (library / '.forge').mkdir(parents=True)
    save(library / '.forge/catalog.json', {'schemaVersion': '2', 'updatedAt': '2026-01-01T00:00:00Z', 'assets': {
        'fixture.props': {'assetId': 'fixture.props', 'name': 'Portable props', 'kind': 'prop_set',
                          'packPath': str(pack), 'packSha256': legacy_hash, 'sourceJobId': job['job_id'],
                          'workflow': 'static-set@1.0.0', 'createdAt': '2026-01-01T00:00:00Z'}}})
    preview = run('project', 'migrate-assets', '--project', library, '--dry-run')
    run('project', 'migrate-assets', '--project', library, '--apply', '--expected-sha256', preview['expectedSha256'])
    scan = out / 'scan.json'
    run('asset', 'scan', '--project', library, '--root', sources, '--out', scan)
    run('asset', 'register', '--project', library, '--input', scan)
    revisions = run('asset', 'search', '--project', library, '--limit', 100)['items']
    assert len(revisions) == 4
    lock = out / 'resources.lock.json'
    for item in revisions:
        run('asset', 'retain', '--project', library, '--id', item['assetId'], '--revision', item['revision'])
        run('asset', 'lock', '--project', library, '--id', item['assetId'], '--revision', item['revision'], '--out', lock)
    evidence = out / 'notes.txt'
    evidence.write_text('Synthetic evidence only; no human approval asserted.', encoding='utf-8')
    selected = revisions[0]
    run('asset', 'review', '--project', library, '--id', selected['assetId'], '--revision', selected['revision'],
        '--domain', 'technical', '--verdict', 'needs_review', '--reviewer', 'fixture', '--statement', 'Original notes', '--evidence', evidence)
    if godot:
        install(run, out, library, lock)
    shutil.rmtree(out / 'jobs')
    shutil.rmtree(sources)
    assert run('project', 'verify-assets', '--project', library, '--rebuild-index')['audit']['completeMedia']
    exported = run('project', 'export-assets', '--project', library, '--asset-lock', lock, '--out', out / 'bundle')
    manifest = json.loads((out / 'bundle/bundle.json').read_text(encoding='utf-8'))
    assert next(s for s in manifest['selections'] if s['assetId'] == 'fixture.props')['legacyPackSha256'] == legacy_hash
    summary = {'producerOs': platform.system(), 'producerBuild': run('doctor')['build'],
               'manifestSha256': exported['manifestSha256'], 'legacyPackSha256': legacy_hash,
               'revisions': {r['assetId']: r['revision'] for r in revisions}, 'providerRequests': 0}
    save(out / 'exchange.json', summary)
    return summary


def consume(forge, bundle_root, out, godot, require_foreign):
    baseline = json.loads((bundle_root / 'exchange.json').read_text(encoding='utf-8'))
    if require_foreign:
        assert baseline['producerOs'] != platform.system(), baseline
    run = runner(forge, out, godot)
    bundle = bundle_root / 'bundle'
    digest = baseline['manifestSha256']
    assert run('project', 'verify-asset-bundle', '--input', bundle)['trustedBaselineVerified'] is False
    run('project', 'verify-asset-bundle', '--input', bundle, '--expected-sha256', '0' * 64, ok=False)
    verified = run('project', 'verify-asset-bundle', '--input', bundle, '--expected-sha256', digest)
    assert verified['trustedBaselineVerified']
    library = out / '\u8d44\u6e90 restored library'
    run('project', 'import-assets', '--input', bundle, '--path', library, '--expected-sha256', digest)
    report = run('project', 'verify-assets', '--project', library, '--rebuild-index')['audit']
    assert report['completeMedia'] and len(report['revisions']) == 4, report
    assert {r['assetId']: r['revision'] for r in report['revisions']} == baseline['revisions']
    legacy_entries = run('asset', 'list', '--project', library)
    assert len(legacy_entries) == 1 and legacy_entries[0]['packSha256'] == baseline['legacyPackSha256'], legacy_entries
    members = run('asset', 'search', '--project', library, '--query', 'coin')
    assert any(i['assetId'] == 'fixture.props' for i in members['items']), members
    preview = run('asset', 'preview', '--project', library, '--out', out / 'preview')
    assert not preview['issues'] and preview['mediaFiles'] >= 4
    lock = library / '.forge/library/consumer-resources.lock.json'
    assert lock.read_bytes() == (bundle / 'payload/.forge/library/consumer-resources.lock.json').read_bytes()
    if godot:
        install(run, out, library, lock)
    else:
        assert not (out / 'jobs').exists() and not (out / 'plans').exists()
    summary = {'passed': True, 'producerOs': baseline['producerOs'], 'consumerOs': platform.system(),
               'consumerBuild': run('doctor')['build'], 'manifestSha256': digest,
               'revisions': baseline['revisions'], 'legacyPackSha256': baseline['legacyPackSha256'],
               'providerRequests': 0, 'nativeInstallation': bool(godot), 'browserPlayback': 'separate_manual_acceptance'}
    save(out / 'summary.json', summary)
    return summary


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--forge', type=Path, required=True)
    parser.add_argument('--mode', choices=['produce', 'consume', 'roundtrip'], default='roundtrip')
    parser.add_argument('--input', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--godot', type=Path)
    parser.add_argument('--require-foreign', action='store_true')
    args = parser.parse_args()
    forge = args.forge.absolute()  # Preserve an installed launcher's symlink.
    def check(root):
        if args.mode == 'produce':
            return produce(forge, root, args.godot)
        if args.mode == 'consume':
            assert args.input, '--input is required for consume'
            return consume(forge, args.input.absolute(), root, args.godot, args.require_foreign)
        first, second = root / 'producer', root / 'consumer'
        first.mkdir(); second.mkdir()
        produce(forge, first, args.godot)
        return consume(forge, first, second, args.godot, False)
    if args.output:
        args.output.mkdir(parents=True, exist_ok=False)
        result = check(args.output.absolute())
    else:
        with tempfile.TemporaryDirectory(prefix='forge-portability-') as root:
            result = check(Path(root))
    print(json.dumps(result))


if __name__ == '__main__':
    main()
