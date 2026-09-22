#!/usr/bin/env python3
"""Verify PNG/GIF/MP4 preview contracts through a source binary or installed launcher."""
import argparse
import base64
import hashlib
import html
import json
import os
from pathlib import Path
import re
import subprocess

from PIL import Image, ImageDraw

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--forge', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--bundled-tools', action='store_true')
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(parents=True, exist_ok=False)
# Preserve the public launcher path, including a macOS installation symlink.
forge = args.forge.absolute()
env = dict(os.environ, FORGE_JOB_STORE=str(root / 'jobs'), FORGE_PLAN_STORE=str(root / 'plans'),
           FORGE_CONFIG_DIR=str(root / 'config'), FORGE_REAL_PROVIDER_MAX_REQUESTS='0')
if args.bundled_tools:
    env.update(GAME_SPRITE_FORGE_FFMPEG_SEARCH_DIRS=str(root / 'empty-tools'),
               GAME_SPRITE_FORGE_DISABLE_MACOS_DEFAULT_TOOL_DIRS='1')


def run(label, *command, fail=False):
    result = subprocess.run([str(forge), *map(str, command), '--json'], env=env,
                            capture_output=True, text=True, encoding='utf-8', timeout=180)
    (root / (label + '.json')).write_text(result.stdout, encoding='utf-8')
    (root / (label + '.log')).write_text(result.stderr, encoding='utf-8')
    value = json.loads(result.stdout)
    assert bool(value['ok']) != fail and (result.returncode != 0) == fail, value
    return value.get('data', value.get('error'))


def inventory(path):
    return {p.relative_to(path).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in path.rglob('*') if p.is_file()}


def tool(path, *command):
    result = subprocess.run([str(path), *map(str, command)], env=env, capture_output=True, timeout=60)
    assert result.returncode == 0, result.stderr.decode(errors='replace')
    return result.stdout


doctor = run('doctor', 'doctor')
assert 'pack_mp4_preview' in doctor['capabilities']
frames = []
for index in range(3):
    frame = Image.new('RGBA', (65, 65))
    ImageDraw.Draw(frame).rectangle((10 + index * 10, 20, 19 + index * 10, 44), fill=(0, 180, 240, 180))
    frame.putpixel((0, 0), (0, 0, 0, 1))
    path = root / f'{index}.png'
    frame.save(path)
    frames.append(str(path))
request = {'schemaVersion': '1', 'input': {'kind': 'png_sequence', 'paths': frames},
           'metadata': {'name': 'Preview fixture', 'animation': 'strike', 'fps': 8,
                        'frameDurationsMs': [80, 240, 110], 'loop': False},
           'normalize': {'mode': 'preserve_source', 'margin': 0, 'marginBottom': 0,
                         'alphaThreshold': 0, 'manualAnchor': {'x': 32, 'y': 45, 'lockedByUser': True}},
           'quality': {'requireGameReady': False}}
request_path = root / 'request.json'
request_path.write_text(json.dumps(request), encoding='utf-8')
plan = run('plan', 'plan', 'prepare-asset', '--request', request_path)
assert plan['estimate']['maximumProviderRequests'] == 0
job = run('job', 'plan', 'execute', '--token', plan['token'], '--wait')
assert job['lifecycle_state'] == 'succeeded', job
pack = Path(next(a['path'] for a in job['artifacts'] if a['kind'] == 'gsfpack'))
before = inventory(pack)
video = run('video', 'pack', 'preview', '--path', pack, '--out', root / 'preview.mp4', '--cache-dir', root / 'cache')
probe = json.loads(tool(doctor['ffprobePath'], '-v', 'error', '-show_streams', '-of', 'json', root / 'preview.mp4'))['streams'][0]
assert probe['codec_name'] == 'h264' and (probe['width'], probe['height']) == (66, 66), probe
assert abs(float(probe['duration']) * 1000 - video['encodedDurationMs']) < 2
assert abs(video['encodedDurationMs'] - 430) < 1000 / 120
for time, bright_x, dark_x in [('0', 15, 35), ('0.1', 25, 15)]:
    pixels = tool(doctor['ffmpegPath'], '-v', 'error', '-ss', time, '-i', root / 'preview.mp4',
                  '-frames:v', '1', '-f', 'rawvideo', '-pix_fmt', 'rgb24', 'pipe:1')
    assert len(pixels) == 66 * 66 * 3
    assert pixels[(30 * 66 + bright_x) * 3 + 2] > 130
    assert pixels[(30 * 66 + dark_x) * 3 + 2] < 65
second = run('cached', 'pack', 'preview', '--path', pack, '--out', root / 'cached.mp4', '--cache-dir', root / 'cache')
assert second['cacheHit'] and video['videoSha256'] == second['videoSha256']
run('collision', 'pack', 'preview', '--path', pack, '--out', root / 'preview.mp4', fail=True)
assert hashlib.sha256((root / 'preview.mp4').read_bytes()).hexdigest() == video['videoSha256']
run('inside-pack', 'pack', 'preview', '--path', pack, '--out', pack / 'new.mp4', fail=True)
assert inventory(pack) == before
with Image.open(pack / 'previews/preview.gif') as gif:
    for i, duration in enumerate([80, 240, 110]):
        gif.seek(i)
        assert gif.info['duration'] == duration
        assert gif.convert('RGBA').getpixel((0, 0))[3] == 0
        # Pillow composes frames, unlike a raw GIF frame decoder: detect accumulation.
        for x in [15, 25, 35]:
            assert (gif.convert('RGBA').getpixel((x, 30))[3] > 0) == (x == 15 + i * 10)
library = root / 'library'
run('init', 'project', 'init', '--path', library, '--name', 'Preview test', '--local-assets')
run('scan-result', 'asset', 'scan', '--project', library, '--root', pack, '--out', root / 'scan.json')
run('register', 'asset', 'register', '--project', library, '--input', root / 'scan.json')
head = inventory(library)
review = run('review', 'asset', 'preview', '--project', library, '--out', root / 'review')
assert not review['issues'] and review['mediaFiles'] == 3
assert len(review['media']) == 1 and len(review['media'][0]['files']) == 3
for i, file in enumerate(review['media'][0]['files']):
    assert file['mediaType'] == 'image' and file['label'].startswith('assets/frames/')
    assert (root / 'review' / file['file']).read_bytes() == (pack / f'assets/frames/frame_{i + 1:03}.png').read_bytes()
page = (root / 'review/index.html').read_text(encoding='utf-8')
script = re.search(r'<script type="text/javascript">(.*?)</script>', page, re.S)[1]
assert base64.b64encode(hashlib.sha256(script.encode()).digest()).decode() in page
player = json.loads(html.unescape(re.search(r'data-forge-animation="([^"]+)"', page)[1]))
assert player['animations'][0]['durationsMs'] == [80, 240, 110]
for i, url in enumerate(player['urls']):
    assert (root / 'review' / url).read_bytes() == (pack / f'assets/frames/frame_{i + 1:03}.png').read_bytes()
assert head == inventory(library) and before == inventory(pack)
summary = {'ok': True, 'build': doctor['build'], 'video': video, 'review': review,
           'checks': ['h264_decode', 'gif_composed_frames', 'timing', 'png_bytes', 'csp',
                      'cache_reuse', 'no_clobber', 'source_and_catalog_unchanged']}
(root / 'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps(summary))
