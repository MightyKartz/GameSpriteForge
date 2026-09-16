#!/usr/bin/env python3
"""Native Godot setup, source isolation, failure reporting and export contracts."""
import argparse
import json
import os
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--forge', type=Path, required=True)
    parser.add_argument('--godot', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--screenshot', action='store_true')
    parser.add_argument('--template', type=Path)
    parser.add_argument('--export', action='store_true')
    args = parser.parse_args()
    root = args.output.absolute()
    root.mkdir(parents=True, exist_ok=False)
    env = dict(os.environ, FORGE_CONFIG_DIR=str(root / 'config'),
               FORGE_JOB_STORE=str(root / 'jobs'), FORGE_PLAN_STORE=str(root / 'plans'))
    env.pop('FORGE_GODOT_PATH', None)
    results = []

    def run(*commands, fail=False, overrides=None):
        result = subprocess.run([str(args.forge.absolute()), *map(str, commands), '--json'],
                                env=dict(env, **(overrides or {})), text=True,
                                encoding='utf-8', capture_output=True, timeout=180)
        try:
            value = json.loads(result.stdout)
        except Exception:
            raise AssertionError((commands, result.returncode, result.stdout, result.stderr))
        assert (result.returncode != 0) == fail, (commands, value, result.stderr)
        assert value['ok'] != fail, value
        results.append({'command': list(map(str, commands)), 'expectedFailure': fail,
                        'result': value})
        return value.get('data', value.get('error'))

    run('setup', 'godot', '--path', args.godot.absolute())
    doctor = run('doctor')
    assert doctor['godotSupported'] and doctor['godotVersion'].startswith('4.6.')
    # Explicit broken selection must never fall back to the valid saved engine.
    assert not run('doctor', overrides={'FORGE_GODOT_PATH': str(root/'missing.exe')})['godotSupported']
    config = root/'config/godot.json'
    saved = config.read_text()
    tampered = json.loads(saved)
    tampered['sha256'] = '0'*64
    config.write_text(json.dumps(tampered))
    assert not run('doctor')['godotSupported']
    config.write_text(saved)

    project = root/'game'
    project.mkdir()
    (project/'project.godot').write_text('''config_version=5
[application]
config/name="Forge workflow acceptance"
run/main_scene="res://main.tscn"
[display]
window/size/viewport_width=160
window/size/viewport_height=120
[rendering]
renderer/rendering_method="gl_compatibility"
environment/defaults/default_clear_color=Color(0.04, 0.06, 0.12, 1)
''')
    (project/'main.tscn').write_text('''[gd_scene load_steps=2 format=3]
[ext_resource type="Script" path="res://main.gd" id="1"]
[node name="Main" type="Node2D"]
script = ExtResource("1")
[node name="Square" type="Polygon2D" parent="."]
polygon = PackedVector2Array(25, 25, 100, 25, 100, 85, 25, 85)
color = Color(0.1, 0.8, 0.6, 1)
''')
    (project/'main.gd').write_text('extends Node2D\nvar score := 0\nfunc hit() -> void:\n\tscore += 1\n')
    (project/'interaction.gd').write_text('''extends SceneTree
func _initialize() -> void:
	var node = load("res://main.tscn").instantiate()
	node.hit()
	if node.score != 1:
		quit(1)
		return
	node.free()
	print("FORGE_ACCEPTANCE_OK")
	quit(0)
''')
    run('godot', 'lock', '--project', project)
    run('godot', 'lock', '--project', project, fail=True)
    lock = project/'.forge/toolchain.lock.json'
    original_lock = lock.read_text()
    changed_lock = json.loads(original_lock)
    assert not any('path' in k.lower() for k in changed_lock)
    changed_lock['godotVersion'] = '4.6.0.stable.official.fake'
    lock.write_text(json.dumps(changed_lock))
    run('godot', 'check', '--project', project, fail=True)
    lock.write_text(original_lock)
    run('godot', 'check', '--project', project)
    verify_args = ['godot', 'verify', '--project', project, '--output', root/'verified',
                   '--acceptance-script', 'interaction.gd']
    if args.screenshot:
        verify_args += ['--screenshot']
    report = run(*verify_args)
    assert report['status'] == 'passed' and report['visualReview'] == 'not_assessed'
    assert report['interactionTests']['status'] == 'passed'
    assert not (project/'.godot').exists(), 'Source cache was mutated'
    run(*verify_args, fail=True)  # Existing evidence must not be overwritten.
    run('godot', 'verify', '--project', project, '--output', project/'inside', fail=True)

    (project/'main.gd').write_text('extends Node2D\nfunc broken(:\n')
    run('godot', 'verify', '--project', project, '--output', root/'broken', fail=True)
    assert json.loads((root/'broken/report.json').read_text())['status'] == 'failed'
    (project/'main.gd').write_text('extends Node2D\nvar score := 0\nfunc hit() -> void:\n\tscore += 1\n')
    (project/'timeout.gd').write_text('extends SceneTree\nfunc _initialize() -> void:\n\tpass\n')
    run('godot', 'verify', '--project', project, '--output', root/'timeout',
        '--acceptance-script', 'timeout.gd', '--timeout', '2', fail=True)
    cancel = root/'cancel'
    cancel.write_text('cancel')
    run('godot', 'verify', '--project', project, '--output', root/'cancelled',
        '--cancel-file', cancel, fail=True)

    platform = 'Windows Desktop' if os.name == 'nt' else 'macOS'
    options = ''
    if args.template:
        options = 'custom_template/release=' + json.dumps(args.template.absolute().as_posix()) + '\n'
    (project/'export_presets.cfg').write_text('''[preset.0]
name="Desktop"
platform="%s"
runnable=true
export_filter="all_resources"
include_filter=""
exclude_filter=""
export_path=""
[preset.0.options]
%stexture_format/s3tc_bptc=true
binary_format/embed_pck=false
codesign/enable=false
application/bundle_identifier="dev.forge.acceptance"
''' % (platform, options))
    run('godot', 'export', '--project', project, '--output', root/'bad-export',
        '--preset', 'Missing preset', fail=True)
    if args.export or args.template:
        exported = run('godot', 'export', '--project', project, '--output', root/'exported',
                       '--preset', 'Desktop', '--run')
        assert exported['exportedRuntime']['status'] == 'passed'
    (root/'summary.json').write_text(json.dumps({'passed': True, 'cases': results,
        'screenshotChecked': args.screenshot, 'exportChecked': bool(args.export or args.template)}, indent=2))
    print(json.dumps({'passed': True, 'cases': len(results), 'output': str(root)}))


if __name__ == '__main__':
    main()
