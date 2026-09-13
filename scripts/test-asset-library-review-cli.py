#!/usr/bin/env python3
"""Offline preview/review CLI acceptance with public synthetic image/GIF/WAV media."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import struct
import subprocess
import tempfile
import wave
from PIL import Image, ImageDraw


def check(forge, root):
    env = dict(os.environ, FORGE_JOB_STORE=str(root / 'jobs'), FORGE_PLAN_STORE=str(root / 'plans'))
    def run(*args):
        result = subprocess.run([str(forge), *map(str, args), '--json'], env=env, capture_output=True, text=True, encoding='utf-8', timeout=90)
        assert result.returncode == 0, result.stdout + result.stderr
        value = json.loads(result.stdout)
        assert value['ok'], value
        return value['data']
    sources = root / 'sources'
    sources.mkdir(parents=True)
    frames = []
    for color in ['#dfa34c', '#7cbeb3']:
        image = Image.new('RGBA', (96, 96))
        draw = ImageDraw.Draw(image)
        draw.ellipse((16, 16, 80, 80), fill=color, outline='#f4ecd8', width=3)
        frames.append(image)
    frames[0].save(sources / 'coin.png')
    frames[0].save(sources / 'pulse.gif', save_all=True, append_images=frames[1:], duration=[70, 190], loop=0)
    with wave.open(str(sources / 'cue.wav'), 'wb') as audio:
        audio.setparams((1, 2, 8000, 0, 'NONE', 'not compressed'))
        audio.writeframes(b''.join(struct.pack('<h', int(math.sin(i / 8000 * math.tau * 440) * 3000)) for i in range(1600)))
    library = root / 'library'
    run('project', 'init', '--path', library, '--name', 'Synthetic review fixtures', '--local-assets')
    plan = root / 'scan.json'
    scan = run('asset', 'scan', '--project', library, '--root', sources, '--out', plan)
    records = run('asset', 'register', '--project', library, '--input', plan)
    image_id = next(i['assetId'] for i in scan['items'] if i['kind'] == 'image')
    revision = next(i['revision'] for i in records if i['assetId'] == image_id)
    run('asset', 'retain', '--project', library, '--id', image_id, '--revision', revision)
    evidence = root / 'review.txt'
    evidence.write_text('Synthetic fixture review; no production approval or license assessment.', encoding='utf-8')
    review = run('asset', 'review', '--project', library, '--id', image_id, '--revision', revision,
                 '--domain', 'visual', '--verdict', 'needs_review', '--reviewer', 'fixture reviewer',
                 '--statement', 'Compare the copper and jade variants.', '--evidence', evidence)
    assert review['evidenceSha256'] == hashlib.sha256(evidence.read_bytes()).hexdigest()
    assert run('asset', 'reviews', '--project', library, '--id', image_id, '--revision', revision)[0]['statement'] == review['statement']
    run('asset', 'annotate', '--project', library, '--id', image_id, '--name', 'Coin variants', '--tag', 'fixture')
    frames[1].save(sources / 'coin.png')
    second_plan = root / 'second.json'
    second = run('asset', 'scan', '--project', library, '--root', sources, '--out', second_plan)
    for item in second['items']:
        item['newRevision'] = True
    second_plan.write_text(json.dumps(second), encoding='utf-8')
    run('asset', 'register', '--project', library, '--input', second_plan)
    selected = run('asset', 'search', '--project', library, '--review-domain', 'visual', '--review-verdict', 'needs_review')
    assert selected['total'] == 1
    head = (library / '.forge/catalog.json').read_bytes()
    output = root / 'preview'
    preview = run('asset', 'preview', '--project', library, '--out', output)
    assert preview['revisions'] == 4 and preview['mediaFiles'] == 4 and not preview['issues'], preview
    assert preview['selection']['total'] == 4
    assert head == (library / '.forge/catalog.json').read_bytes()
    html = (output / 'index.html').read_text(encoding='utf-8')
    assert '<audio controls' in html and 'visual: unknown' in html and 'visual: needs_review' in html
    assert not (root / 'jobs').exists() and not (root / 'plans').exists()
    build = run('doctor')['build']
    print(json.dumps({'passed': True, 'build': build, 'preview': preview, 'checks': ['image_gif_wav', 'retained_review_evidence', 'new_revision_unknown', 'search_assertions', 'readonly_preview', 'no_provider_or_job']}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--forge', type=Path, required=True)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if args.output:
        args.output.mkdir(parents=True, exist_ok=False)
        check(args.forge.resolve(), args.output.resolve())
    else:
        with tempfile.TemporaryDirectory(prefix='forge-library-review-') as temporary:
            check(args.forge.resolve(), Path(temporary))
