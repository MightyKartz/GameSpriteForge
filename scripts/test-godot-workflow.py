#!/usr/bin/env python3
"""Native Godot setup, source isolation, failure reporting and export contracts."""
import argparse
import json
import os
from pathlib import Path
import shutil
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
    # Baseline saves live only in this fixture profile, never in real game data.
    baseline = root/'original-profile'
    for directory in ['home', 'data', 'config', 'cache', 'temp']:
        (baseline/directory).mkdir(parents=True)
    baseline_env = dict(HOME=str(baseline/'home'), APPDATA=str(baseline/'data'),
                        XDG_DATA_HOME=str(baseline/'data'), XDG_CONFIG_HOME=str(baseline/'config'),
                        XDG_CACHE_HOME=str(baseline/'cache'), LOCALAPPDATA=str(baseline/'cache'))
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
textures/vram_compression/import_etc2_astc=true
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
    main_script = '''extends Node2D
var score := 0
func hit() -> void:
\tscore += 1
func _ready() -> void:
\tvar count := 0
\tif FileAccess.file_exists("user://launch-count.txt"):
\t\tcount = int(FileAccess.get_file_as_string("user://launch-count.txt"))
\tvar file := FileAccess.open("user://launch-count.txt", FileAccess.WRITE)
\tfile.store_string(str(count + 1))
\tfile.close()
\tprint("FORGE_TEST_USERDIR=" + OS.get_user_data_dir())
\tprint("FORGE_TEST_COUNT=" + str(count + 1))
'''
    (project/'main.gd').write_text(main_script)
    (project/'interaction.gd').write_text('''extends SceneTree
func _initialize() -> void:
	var node = load("res://main.tscn").instantiate()
	print("FORGE_TEST_USERDIR=" + OS.get_user_data_dir())
	if FileAccess.get_file_as_string("user://launch-count.txt") != "1":
		quit(1)
		return
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
    checked = run('godot', 'check', '--project', project)

    def seed_save():
        native = subprocess.run([str(args.godot.absolute()), '--headless', '--path', str(project),
                                 '--quit-after', '2'], env=dict(env, **baseline_env),
                                capture_output=True, text=True, encoding='utf-8', timeout=60)
        assert native.returncode == 0, (native.stdout, native.stderr)
        userdir = next(line.split('=', 1)[1] for line in native.stdout.splitlines()
                       if line.startswith('FORGE_TEST_USERDIR='))
        save = Path(userdir)/'launch-count.txt'
        assert save.read_text() == '1'
        return save

    def check_isolation(report, phase, original_save):
        text = Path(report[phase]['stdout']).read_text(encoding='utf-8')
        userdir = next(line.split('=', 1)[1] for line in text.splitlines()
                       if line.startswith('FORGE_TEST_USERDIR='))
        assert Path(userdir).resolve().is_relative_to(Path(report['userData']['root']).resolve()), text
        assert (Path(userdir)/'launch-count.txt').read_text() == '1'
        assert original_save.read_text() == '1', 'Acceptance mutated the original save'

    original_save = seed_save()
    # Baseline import creates a cache; removing only our fixture cache makes the
    # existing source-cache regression assertion meaningful again.
    fixture_cache = project/'.godot'
    assert fixture_cache.resolve().is_relative_to(root.resolve())
    if fixture_cache.exists():
        shutil.rmtree(fixture_cache)
    source_config = (project/'project.godot').read_bytes()
    verify_args = ['godot', 'verify', '--project', project, '--output', root/'verified',
                   '--acceptance-script', 'interaction.gd']
    if args.screenshot:
        verify_args += ['--screenshot']
    report = run(*verify_args, overrides=baseline_env)
    assert report['status'] == 'passed' and report['visualReview'] == 'not_assessed'
    assert report['interactionTests']['status'] == 'passed'
    check_isolation(report, 'runtime', original_save)
    check_isolation(report, 'interactionTests', original_save)
    assert (project/'project.godot').read_bytes() == source_config
    assert not (project/'.godot').exists(), 'Source cache was mutated'
    run(*verify_args, fail=True)  # Existing evidence must not be overwritten.
    run('godot', 'verify', '--project', project, '--output', project/'inside', fail=True)

    (project/'main.gd').write_text('extends Node2D\nfunc broken(:\n')
    run('godot', 'verify', '--project', project, '--output', root/'broken', fail=True)
    assert json.loads((root/'broken/report.json').read_text())['status'] == 'failed'
    (project/'main.gd').write_text('extends Node2D\nfunc _ready() -> void:\n\tget_tree().quit(0)\n')
    run('godot', 'verify', '--project', project, '--output', root/'early-exit', fail=True)
    assert json.loads((root/'early-exit/report.json').read_text())['runtime']['status'] == 'failed'
    (project/'main.gd').write_text(main_script)
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
        check_isolation(exported, 'exportedRuntime', original_save)

        template = args.template or Path(checked['templates']['directory']) / (
            'windows_release_x86_64.exe' if os.name == 'nt' else 'macos.zip')
        external = root/'templates outside project'/'release template 模板'
        external = external.with_suffix(template.suffix)
        external.parent.mkdir()
        shutil.copyfile(template, external)
        preset = project/'export_presets.cfg'
        custom_preset = preset.read_text().replace('[preset.0.options]',
            '[preset.0.options]\ncustom_template/release=' +
            json.dumps('../' + external.relative_to(root).as_posix(), ensure_ascii=False))
        # Remove the old absolute custom path when --template was supplied.
        if options:
            custom_preset = custom_preset.replace(options, '')
        preset.write_text(custom_preset, encoding='utf-8')
        saved_preset = preset.read_bytes()
        (project/'project.godot').write_text(source_config.decode().replace('[application]',
            '[application]\nconfig/use_custom_user_dir=true\nconfig/custom_user_dir_name="Forge test/custom saves"'))
        custom_save = seed_save()
        custom_config = (project/'project.godot').read_bytes()
        for name in ['custom-save-1', 'custom-save-2']:
            report = run('godot', 'verify', '--project', project, '--output', root/name,
                         '--acceptance-script', 'interaction.gd', overrides=baseline_env)
            check_isolation(report, 'runtime', custom_save)
            check_isolation(report, 'interactionTests', custom_save)
        relative = run('godot', 'export', '--project', project, '--output', root/'relative-template',
                       '--preset', 'Desktop', '--run', overrides=baseline_env)
        check_isolation(relative, 'exportedRuntime', custom_save)
        assert Path(relative['releaseTemplate']['path']).samefile(external)
        assert preset.read_bytes() == saved_preset
        assert (project/'project.godot').read_bytes() == custom_config
    (root/'summary.json').write_text(json.dumps({'passed': True, 'cases': results,
        'screenshotChecked': args.screenshot, 'exportChecked': bool(args.export or args.template),
        'regressions': {'defaultUserDataIsolated': True,
            'customUserDataAndRepeatedRunsIsolated': bool(args.export or args.template),
            'externalRelativeTemplateExportedAndStarted': bool(args.export or args.template),
            'sourceSettingsPreserved': True}}, indent=2))
    print(json.dumps({'passed': True, 'cases': len(results), 'output': str(root)}))


if __name__ == '__main__':
    main()
