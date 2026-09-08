#!/usr/bin/env python3
"""Validate lossless whole-sheet preprocessing through the CLI and Godot.

The three required production inputs are read-only; all derived files are written
by Forge under --output. Python/Pillow only reads and compares their pixels.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
from PIL import Image

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--forge', required=True, type=Path)
p.add_argument('--output', required=True, type=Path)
p.add_argument('--ghost', required=True, type=Path)
p.add_argument('--boss-idle', required=True, type=Path)
p.add_argument('--boss-slam', required=True, type=Path)
p.add_argument('--godot', type=Path, default=Path('/Applications/Godot.app/Contents/MacOS/Godot'))
a = p.parse_args()
root = a.output.resolve()
root.mkdir(parents=True, exist_ok=True)
project = root / 'project'
project.mkdir(exist_ok=True)
(project / 'project.godot').write_text('config_version=5\n[application]\nconfig/name="Source transform QA"\nconfig/features=PackedStringArray("4.6")\n')
env = dict(os.environ, FORGE_JOB_STORE=str(root / 'jobs'), FORGE_PLAN_STORE=str(root / 'plans'),
           FORGE_GODOT_PATH=str(a.godot), FORGE_REAL_PROVIDER_MAX_REQUESTS='0')
env.pop('FORGE_REAL_PROVIDER_ACCEPT', None)
commands = []
def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')
def call(label, *args, reject=False):
    command = [str(a.forge), *map(str, args)]
    proc = subprocess.run(command, env=env, text=True, capture_output=True, timeout=120)
    (root / (label + '.stdout.json')).write_text(proc.stdout)
    (root / (label + '.stderr.log')).write_text(proc.stderr)
    commands.append({'command': command, 'exitCode': proc.returncode})
    result = json.loads(proc.stdout)
    assert bool(result['ok']) != reject and bool(proc.returncode) == reject, (label, result)
    return result.get('data', result.get('error'))
def execute(label, command, request):
    request_path = root / (label + '-request.json')
    save(request_path, request)
    plan = call(label + '-plan', 'plan', command, '--request', request_path, '--json')
    assert plan['estimate']['providerRequestEstimate'] == plan['estimate']['maximumProviderRequests'] == 0
    job = call(label + '-execute', 'plan', 'execute', '--token', plan['token'], '--wait', '--json')
    assert job['lifecycle_state'] == 'succeeded', job
    return job

cases = [
    ('ghost', a.ghost, 603, 653, 2, 2, 0, 1),
    ('boss_idle', a.boss_idle, 659, 627, 2, 2, 64, 0),
    ('boss_slam', a.boss_slam, 528, 512, 3, 2, 48, 0),
]
results = {}
for name, source, fw, fh, cols, rows, right, bottom in cases:
    source = source.resolve()
    source_hash = sha(source)
    request = {'input': {'kind': 'sprite_sheet', 'path': str(source), 'split': {
        'mode': 'fixed_grid', 'frameWidth': fw, 'frameHeight': fh, 'columns': cols, 'rows': rows,
        'sourcePaddingRightPx': right, 'sourcePaddingBottomPx': bottom}},
        'metadata': {'name': name, 'animation': 'idle', 'fps': 8, 'loop': name != 'boss_slam'},
        'normalize': {'mode': 'preserve_source', 'margin': 0, 'marginBottom': 0, 'alphaThreshold': 0},
        'rendering': {'textureFilter': 'linear', 'pixelSnap': False},
        'quality': {'requireGameReady': False}}
    job = execute(name, 'prepare-asset', request)
    pack = Path(next(item['path'] for item in job['artifacts'] if item['kind'] == 'gsfpack'))
    sidecar = Path(next(item['path'] for item in job['artifacts'] if item['kind'] == 'source_transform'))
    record = json.loads(sidecar.read_text())
    assert record['sourceSha256'] == source_hash == sha(source) == sha(record['originalPath'])
    assert record['derivedSha256'] == sha(record['derivedPath'])
    assert record['discardedNontransparentPixels'] == 0
    original = Image.open(source).convert('RGBA')
    derived = Image.open(record['derivedPath']).convert('RGBA')
    assert derived.size == (fw * cols, fh * rows)
    assert derived.crop((0, 0, original.width, original.height)).tobytes() == original.tobytes()
    if right:
        assert derived.crop((original.width, 0, derived.width, derived.height)).getchannel('A').getextrema() == (0, 0)
    if bottom:
        assert derived.crop((0, original.height, derived.width, derived.height)).getchannel('A').getextrema() == (0, 0)
    for index, frame_path in enumerate(sorted((pack / 'assets/frames').glob('*.png'))):
        x, y = index % cols * fw, index // cols * fh
        frame = Image.open(frame_path).convert('RGBA')
        assert frame.size == (fw, fh)
        assert frame.tobytes() == derived.crop((x, y, x + fw, y + fh)).tobytes()
    normalized = json.loads((Path(job['job_dir']) / 'normalized-frames.json').read_text())
    assert all(frame['offsetX'] == frame['offsetY'] == 0 for frame in normalized)
    assert all(frame['anchor'] == normalized[0]['anchor'] for frame in normalized)
    call(name + '-pack-validate', 'pack', 'validate', '--path', pack, '--json')
    install = execute(name + '-install', 'install-godot', {
        'packPath': str(pack), 'projectPath': str(project), 'target': 'addons/forge_assets/' + name,
        'assetKey': name, 'providerRefs': []})
    usage_path = project / 'addons/forge_assets' / name / 'forge_usage.json'
    usage = json.loads(usage_path.read_text())
    assert usage['frameWidth'] == fw and usage['frameHeight'] == fh
    assert usage['rendering']['textureFilter'] == 'linear' and not usage['rendering']['pixelSnap']
    assert usage['providerProvenance'] == []
    results[name] = {'sourceTransform': record, 'jobId': job['job_id'], 'packPath': str(pack),
                     'usagePath': str(usage_path), 'installJobId': install['job_id'],
                     'qualityVerdict': json.loads((pack / 'quality-report.json').read_text())['verdict']}

rejected = {'input': {'kind': 'sprite_sheet', 'path': str(a.boss_idle.resolve()), 'split': {
    'mode': 'fixed_grid', 'frameWidth': 627, 'frameHeight': 627, 'columns': 2, 'rows': 2, 'sourceOffsetX': -32}},
    'metadata': {'name': 'boss-loss-must-fail'},
    'normalize': {'mode': 'preserve_source', 'margin': 0, 'marginBottom': 0, 'alphaThreshold': 0},
    'rendering': {'textureFilter': 'linear', 'pixelSnap': False}}
save(root / 'boss-loss-request.json', rejected)
error = call('boss-loss', 'plan', 'prepare-asset', '--request', root / 'boss-loss-request.json', '--json', reject=True)
assert 'discard a nontransparent pixel' in error['message'], error
for name, source, *_ in cases:
    assert sha(source) == results[name]['sourceTransform']['sourceSha256']
(project / 'verify.gd').write_text('''extends SceneTree
func _initialize() -> void:
	var specs = {"ghost": [603,653,4], "boss_idle": [659,627,4], "boss_slam": [528,512,6]}
	for name in specs:
		var dims = specs[name]
		var scene = load("res://addons/forge_assets/%s/forge_animated_sprite.tscn" % name)
		assert(scene is PackedScene)
		var root = scene.instantiate()
		var sprite = root.get_node("AnimatedSprite2D")
		assert(sprite.texture_filter == CanvasItem.TEXTURE_FILTER_LINEAR and sprite.centered)
		assert(sprite.position == Vector2(0, -float(dims[1])/2.0))
		assert(sprite.sprite_frames.get_frame_count("idle") == dims[2])
		assert(sprite.sprite_frames.get_animation_loop("idle") == (name != "boss_slam"))
		for i in range(dims[2]):
			assert(sprite.sprite_frames.get_frame_texture("idle", i).get_size() == Vector2(dims[0],dims[1]))
		root.free()
	print("PASS original-source transforms: ghost, boss_idle, boss_slam, shared anchors, linear")
	quit(0)
''')
proc = subprocess.run([str(a.godot), '--headless', '--path', str(project), '--script', 'res://verify.gd', '--quit-after', '120'],
                      text=True, capture_output=True, timeout=60)
(root / 'godot-verify.stdout.log').write_text(proc.stdout)
(root / 'godot-verify.stderr.log').write_text(proc.stderr)
assert proc.returncode == 0 and 'PASS original-source transforms' in proc.stdout, (proc.stdout, proc.stderr)
save(root / 'report.json', {'cli': str(a.forge.resolve()), 'cliSha256': sha(a.forge),
     'results': results, 'rejectedBossOffset': error, 'commands': commands})
print(json.dumps({'ok': True, 'report': str(root / 'report.json'), 'cases': list(results)}))
