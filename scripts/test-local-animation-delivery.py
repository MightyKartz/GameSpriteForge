#!/usr/bin/env python3
"""Exercise the public offline animation CLI and load installed Godot resources.

Requires Python Pillow and Godot 4.6.x. All generated files stay under --output.
"""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess

from PIL import Image

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--forge', required=True, type=Path)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--godot', type=Path, default=Path('/Applications/Godot.app/Contents/MacOS/Godot'))
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(parents=True, exist_ok=True)
project = root / 'project'
project.mkdir(exist_ok=True)
(project / 'project.godot').write_text('config_version=5\n[application]\nconfig/name="Local animation QA"\nconfig/features=PackedStringArray("4.6")\n')
env = dict(os.environ, FORGE_JOB_STORE=str(root / 'jobs'), FORGE_PLAN_STORE=str(root / 'plans'),
           FORGE_CACHE_STORE=str(root / 'cache'), FORGE_GODOT_PATH=str(args.godot),
           FORGE_REAL_PROVIDER_MAX_REQUESTS='0')
env.pop('FORGE_REAL_PROVIDER_ACCEPT', None)
commands = []

def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')

def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

def call(label, *command, fail=False):
    process = subprocess.run([str(args.forge), *map(str, command)], env=env, text=True, capture_output=True, timeout=120)
    (root / f'{label}.stdout.json').write_text(process.stdout)
    (root / f'{label}.stderr.log').write_text(process.stderr)
    commands.append({'label': label, 'command': [str(args.forge), *map(str, command)], 'exitCode': process.returncode})
    result = json.loads(process.stdout)
    assert bool(result['ok']) != fail, (label, result)
    assert (process.returncode != 0) == fail, (label, process.returncode)
    return result.get('data', result.get('error'))

def execute(name, kind, request):
    path = root / f'{name}-request.json'
    save(path, request)
    plan = call(f'{name}-plan', 'plan', kind, '--request', path, '--json')
    assert plan['estimate']['providerRequestEstimate'] == 0
    assert plan['estimate']['maximumProviderRequests'] == 0
    job = call(f'{name}-execute', 'plan', 'execute', '--token', plan['token'], '--wait', '--json')
    assert job['lifecycle_state'] == 'succeeded', job
    stored = call(f'{name}-job', 'job', 'get', '--id', job['job_id'], '--json')
    assert stored['lifecycle_state'] == 'succeeded'
    return job

input_dir = root / 'input'
input_dir.mkdir(exist_ok=True)
paths = []
for index in range(3):
    frame = Image.new('RGBA', (64, 64))
    for y in range(20, 52):
        for x in range(22, 38):
            frame.putpixel((x, y), (200, 70, 90, 255))
    for x in range(38, 43 + index * 2):
        frame.putpixel((x, 30), (80, 150, 210, 128))
    path = input_dir / f'{index}.png'
    frame.save(path)
    paths.append(str(path))
base = {'schemaVersion': '1', 'input': {'kind': 'png_sequence', 'paths': paths},
        'metadata': {'name': 'F01 single', 'animation': 'idle', 'fps': 10, 'frameDurationsMs': [70, 150, 230]},
        'rendering': {'textureFilter': 'linear', 'pixelSnap': False},
        'normalize': {'mode': 'preserve_source', 'margin': 0, 'marginBottom': 0, 'alphaThreshold': 0,
                      'manualAnchor': {'x': 32.5, 'y': 52.25, 'lockedByUser': True}},
        'quality': {'requireGameReady': False}}
character = copy.deepcopy(base)
character['schemaVersion'] = '2'
character['metadata'] = {'name': 'F01 character', 'defaultAnimation': 'attack'}
character_input = character.pop('input')
character['animations'] = [
    {'name': 'idle', 'input': character_input, 'fps': 10, 'frameDurationsMs': [70, 150, 230]},
    {'name': 'attack', 'input': character_input, 'fps': 12, 'loop': False, 'frameDurationsMs': [60, 240, 100]},
]
nearest = copy.deepcopy(base)
nearest['metadata']['name'] = 'F01 nearest'
nearest['rendering'] = {'textureFilter': 'nearest', 'pixelSnap': True}
nearest['normalize']['manualAnchor'] = {'x': 32, 'y': 52, 'lockedByUser': True}
legacy = copy.deepcopy(base)
legacy['metadata']['name'] = 'F01 legacy'
legacy.pop('rendering')
legacy.pop('normalize')
legacy['metadata'].pop('frameDurationsMs')

# Both dimensions need padding before the entire sheet moves right/down. Keep
# distinct cells and very low alpha pixels so reordering, rematting or alignment
# cannot pass the decoded-pixel comparison accidentally.
source_sheet = Image.new('RGBA', (126, 127))
for index in range(4):
    x, y = index % 2 * 64, index // 2 * 64
    source_sheet.paste(Image.open(paths[index % 3]), (x, y))
    source_sheet.putpixel((x + 30, y + 30), (40 + index * 30, 90, 160, 255))
    source_sheet.putpixel((x + 21, y + 31), (100, 150, 200, 1))
    source_sheet.putpixel((x + 20, y + 31), (17, 99, 203, 0))
source_path = input_dir / 'source-sheet.png'
source_sheet.save(source_path)
source_bytes = source_path.read_bytes()
source_hash = sha256(source_path)
expected_derived = Image.new('RGBA', (128, 128))
expected_derived.paste(source_sheet, (2, 1))
transformed = copy.deepcopy(base)
transformed['metadata']['name'] = 'F01 synthetic source transform'
transformed['metadata']['frameDurationsMs'] = [70, 110, 150, 230]
transformed['input'] = {'kind': 'sprite_sheet', 'path': str(source_path), 'split': {
    'mode': 'fixed_grid', 'frameWidth': 64, 'frameHeight': 64, 'columns': 2, 'rows': 2,
    'sourcePaddingRightPx': 2, 'sourcePaddingBottomPx': 1, 'sourceOffsetX': 2, 'sourceOffsetY': 1,
}}
results = {}
for name, kind, request in [('single', 'prepare-asset', base), ('character', 'prepare-character', character),
                            ('nearest', 'prepare-asset', nearest), ('legacy', 'prepare-asset', legacy),
                            ('source_transform', 'prepare-asset', transformed)]:
    job = execute(name, kind, request)
    pack = Path(next(a['path'] for a in job['artifacts'] if a['kind'] == 'gsfpack'))
    call(f'{name}-pack-validate', 'pack', 'validate', '--path', pack, '--json')
    normalized = json.loads((Path(job['job_dir']) / 'normalized-frames.json').read_text())
    if name != 'legacy':
        assert all(f['offsetX'] == f['offsetY'] == 0 for f in normalized)
        assert all(f['size'] == {'width': 64, 'height': 64} for f in normalized)
        # Compare decoded RGBA, including semitransparent edge pixels.
        outputs = sorted((pack / 'assets/frames').glob('*.png'))
        assert len(outputs) == (6 if name == 'character' else 4 if name == 'source_transform' else 3)
        for index, output in enumerate(outputs):
            if name == 'source_transform':
                x, y = index % 2 * 64, index // 2 * 64
                expected_frame = expected_derived.crop((x, y, x + 64, y + 64))
            else:
                expected_frame = Image.open(paths[index % 3]).convert('RGBA')
            assert Image.open(output).convert('RGBA').tobytes() == expected_frame.tobytes(), (name, index)
    transform_record = None
    if name == 'source_transform':
        artifact = next(a for a in job['artifacts'] if a['kind'] == 'source_transform')
        sidecar = Path(artifact['path'])
        assert artifact['sha256'] == sha256(sidecar)
        transform_record = json.loads(sidecar.read_text())
        original_path = Path(transform_record['originalPath'])
        derived_path = Path(transform_record['derivedPath'])
        assert source_path.read_bytes() == original_path.read_bytes() == source_bytes
        assert transform_record['sourceSha256'] == source_hash == sha256(original_path) == sha256(source_path)
        assert transform_record['derivedSha256'] == sha256(derived_path)
        assert (transform_record['sourceWidth'], transform_record['sourceHeight']) == (126, 127)
        assert (transform_record['derivedWidth'], transform_record['derivedHeight']) == (128, 128)
        for key in ['sourcePaddingRightPx', 'sourcePaddingBottomPx', 'sourceOffsetX', 'sourceOffsetY']:
            assert transform_record[key] == request['input']['split'][key]
        assert transform_record['discardedNontransparentPixels'] == 0
        derived = Image.open(derived_path).convert('RGBA')
        assert derived.size == expected_derived.size and derived.tobytes() == expected_derived.tobytes()
        assert derived.crop((2, 1, 128, 128)).tobytes() == source_sheet.tobytes()
        assert all(f['anchor'] == normalized[0]['anchor'] for f in normalized)
    install = execute(f'{name}-install', 'install-godot', {
        'schemaVersion': '1', 'packPath': str(pack), 'projectPath': str(project),
        'target': f'addons/forge_assets/{name}', 'assetKey': name, 'providerRefs': [],
    })
    usage_path = project / f'addons/forge_assets/{name}/forge_usage.json'
    usage = json.loads(usage_path.read_text())
    if name != 'legacy':
        assert usage['frameWidth'] == usage['frameHeight'] == 64
        assert usage['rendering']['textureFilter'] == ('nearest' if name == 'nearest' else 'linear')
        assert usage['anchor']['x'] == (32 if name == 'nearest' else 32.5)
        expected_durations = ([60, 240, 100] if name == 'character' else
                              [70, 110, 150, 230] if name == 'source_transform' else [70, 150, 230])
        assert usage['animations'][0]['frameDurationsMs'] == expected_durations
    else:
        assert 'rendering' not in usage
        assert 'frameDurationsMs' not in usage['animations'][0]
    assert usage['providerProvenance'] == []
    results[name] = {'jobId': job['job_id'], 'packPath': str(pack), 'usagePath': str(usage_path),
                     'installJobId': install['job_id'], 'packSha256': usage['packSha256']}
    if transform_record is not None:
        results[name]['sourceTransform'] = transform_record

negative = copy.deepcopy(base)
negative['metadata']['frameDurationsMs'] = [100, 200]
save(root / 'invalid-duration-request.json', negative)
error = call('invalid-duration', 'plan', 'prepare-asset', '--request', root / 'invalid-duration-request.json', '--json', fail=True)
assert 'one positive integer per frame' in error['message']
negative = copy.deepcopy(character)
other = input_dir / 'different-canvas.png'
Image.new('RGBA', (65, 64)).save(other)
negative['animations'][1]['input'] = {'kind': 'png_sequence', 'paths': [str(other)] * 3}
save(root / 'invalid-canvas-request.json', negative)
error = call('invalid-canvas', 'plan', 'prepare-character', '--request', root / 'invalid-canvas-request.json', '--json', fail=True)
assert 'identical source canvas' in error['message']

lossy_source = source_sheet.copy()
lossy_source.putpixel((0, 3), (0, 0, 0, 1))
lossy_path = input_dir / 'alpha-one-edge-sheet.png'
lossy_source.save(lossy_path)
lossy_hash = sha256(lossy_path)
negative = copy.deepcopy(transformed)
negative['input']['path'] = str(lossy_path)
negative['input']['split']['sourceOffsetX'] = -1
save(root / 'invalid-source-offset-request.json', negative)
rejected_source_offset = call('invalid-source-offset', 'plan', 'prepare-asset', '--request',
                              root / 'invalid-source-offset-request.json', '--json', fail=True)
assert 'discard a nontransparent pixel at (0, 3); alpha=1' in rejected_source_offset['message']
assert sha256(lossy_path) == lossy_hash
assert source_path.read_bytes() == source_bytes and sha256(source_path) == source_hash

(project / 'verify.gd').write_text('''extends SceneTree
func _initialize() -> void:
	for name in ["single", "character", "nearest", "legacy", "source_transform"]:
		var scene = load("res://addons/forge_assets/%s/forge_animated_sprite.tscn" % name)
		assert(scene is PackedScene)
		var root = scene.instantiate()
		var sprite = root.get_node("AnimatedSprite2D")
		var native = sprite.sprite_frames
		var action = "attack" if name == "character" else "idle"
		assert(native.get_frame_count(action) == (4 if name == "source_transform" else 3))
		assert(sprite.animation == action)
		if name == "single" or name == "character" or name == "source_transform":
			assert(sprite.texture_filter == CanvasItem.TEXTURE_FILTER_LINEAR)
			assert(sprite.centered and sprite.position == Vector2(-0.5, -20.25))
		else:
			assert(sprite.texture_filter == CanvasItem.TEXTURE_FILTER_NEAREST)
			assert(not sprite.centered)
		if name == "nearest":
			assert(sprite.position == Vector2(-32, -52))
		if name != "legacy":
			var times = [60.0, 240.0, 100.0] if name == "character" else [70.0, 150.0, 230.0]
			if name == "source_transform":
				times = [70.0, 110.0, 150.0, 230.0]
			for i in range(native.get_frame_count(action)):
				var milliseconds = native.get_frame_duration(action, i) / native.get_animation_speed(action) * 1000.0
				assert(abs(milliseconds - times[i]) < 0.001)
				assert(native.get_frame_texture(action, i).get_size() == Vector2(64, 64))
		else:
			assert(native.get_frame_duration(action, 0) == 1.0)
		if name == "character":
			assert(not native.get_animation_loop("attack"))
			assert(native.get_animation_loop("idle"))
			assert(native.get_animation_names().size() == 2)
		if name == "source_transform":
			assert(native.get_animation_loop("idle"))
			assert(native.get_animation_names().size() == 1)
		root.free()
	print("PASS local animation delivery: single, character, nearest, legacy, source_transform, timings, anchors")
	quit(0)
''')
process = subprocess.run([str(args.godot), '--headless', '--path', str(project), '--script', 'res://verify.gd', '--quit-after', '120'],
                         text=True, capture_output=True, timeout=60)
(root / 'godot-verify.stdout.log').write_text(process.stdout)
(root / 'godot-verify.stderr.log').write_text(process.stderr)
assert process.returncode == 0 and 'PASS local animation delivery' in process.stdout, (process.stdout, process.stderr)
report = {'cli': str(args.forge.resolve()), 'cliSha256': hashlib.sha256(args.forge.read_bytes()).hexdigest(),
          'godot': str(args.godot), 'output': str(root), 'providerRequests': 0,
          'assertions': ['five local plans and installs estimate zero Provider requests', 'decoded RGBA pixels preserved',
                         'shared canvas and anchor across actions', 'Pack validation passes',
                         'Godot native durations, sampling, anchor and loop verified', 'legacy defaults preserved',
                         'invalid duration and cross-action canvas rejected',
                         'synthetic whole-sheet padding and offset preserve original bytes, hashes and derived RGBA',
                         'every transformed grid cell matches its exported frame', 'source offset losing alpha=1 rejected'],
          'sources': {str(p): sha256(Path(p)) for p in [*paths, source_path, lossy_path]},
          'rejectedSourceOffset': rejected_source_offset,
          'results': results, 'commands': commands}
save(root / 'report.json', report)
print(json.dumps({'ok': True, 'report': str(root / 'report.json'), 'cases': list(results)}))
