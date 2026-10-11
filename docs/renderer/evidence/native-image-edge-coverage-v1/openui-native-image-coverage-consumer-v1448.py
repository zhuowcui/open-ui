"""Native image mutation and neighboring paint effects, with strict references."""
import ast
import base64
import hashlib
import importlib.util
import io
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path('/dev/shm/openui-native-image-coverage-d913041d')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-image-coverage-consumer-v1448'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '2eacae2c8aad222850af30f9cea6f8a79bbddd4a'
build_path = RAW / 'native-image-coverage-clean-v1448/build.json'
build = json.loads(build_path.read_bytes())
assert build['all_commands_terminal'] and len(build['steps']) == 8
assert all(row['observed_exit_code'] == 0 for row in build['steps'])
assert source == build['source'] == build['source_after']
binary = build_path.parent / 'native_image_background_culling'
assert sha(binary) == next(row['binary_sha256'] for row in build['steps'] if row['name'] == 'image-build')
asset = ROOT / 'bindings/rust/openui/tests/assets/green-200.png'
assert sha(asset) == 'd49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe'
assets = dict(green=asset, white=asset.parent / '1x1-white.png', transparent=asset.parent / 'green-transparent-200x200.png')
assert sha(assets['white']) == 'b31782b0ecaa71394f1bccf3cc4647ba70b7208464244546b48521a71e1f1dd0'
assert sha(assets['transparent']) == '77e8da29ee253660e7a650f43241d96e636b8e2cec5547cb99fd160e95419422'
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
chrome = Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome')
env = capture.chrome_environment(str(chrome.parent), True, False)
report = dict(schema_version=1, source=source, source_after=source,
    probe_sha256=sha(Path(__file__)), build_receipt_sha256=sha(build_path),
    native_binary_sha256=sha(binary), chromium_binary_sha256=sha(chrome),
    capture_harness_sha256=sha(harness), image_sha256=sha(asset), resource_sha256={key: sha(path) for key, path in assets.items()},
    fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),
    public_native_rust_api=True, javascript_executed_by_openui=False,
    pixel_tolerance=0, release_qualification=False, promotion_allowed=False,
    new_release_states_admitted=0, all_commands_terminal=False,
    cases=[], unstable_reference_captures=[], inputs={})
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

# Reuse the reviewed capture function verbatim. It writes both consecutive
# captures before requiring equality, and closes its Chromium process in finally.
capture_source = Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py')
capture_text = capture_source.read_text()
nodes = ast.parse(capture_text).body
reference_node = next(n for n in nodes if isinstance(n, ast.FunctionDef) and n.name == 'reference')
reference_text = ast.get_source_segment(capture_text, reference_node)
exec(compile(reference_text, str(capture_source), 'exec'))
report['reference_function_sha256'] = hashlib.sha256(reference_text.encode()).hexdigest()
expression_node = next(n for n in nodes if isinstance(n, ast.Assign)
    and any(isinstance(t, ast.Name) and t.id == 'expression' for t in n.targets))
expression = ast.literal_eval(expression_node.value)

variants = ['opaque', 'partial', 'opacity', 'padding', 'contain', 'offset', 'blur', 'shadow', 'white', 'white-opacity', 'transparent', 'clip']
try:
    for display in ['block', 'inline']:
        for variant in variants:
            for phase in [0.0, 0.25, 0.5]:
                case = f'{display}-{variant}-{phase}'
                asset = assets['white'] if variant in ('white', 'white-opacity') else assets['transparent'] if variant == 'transparent' else assets['green']
                inputs = {}
                for state, height in [('before', 100), ('after', 150)]:
                    path = OUT / f'{case}-{state}.html'
                    parent_effect = 'box-shadow:1px 1px 1px black;' if variant == 'shadow' else ''
                    effects = dict(opacity='opacity:.5;', padding='padding:1px;',
                        contain='object-fit:contain;', offset='object-position:0% 50%;', blur='filter:blur(1px);')
                    effects['white-opacity'] = 'opacity:.5;'
                    effects['clip'] = 'object-fit:none;overflow-x:hidden;overflow-y:hidden;'
                    image_width = 100 if variant == 'partial' else 150
                    path.write_text(f'''<!doctype html><meta charset="utf-8"><style>
* {{margin:0;padding:0;border:0;box-sizing:content-box}}
html {{overflow:hidden}} body {{background:white;color:black;font:16px/1 Ahem}}
body,body * {{font-family:Ahem,"Droid Sans Fallback","Noto Sans Devanagari","Noto Color Emoji","DejaVu Sans"!important;
font-weight:normal!important;font-style:normal!important;font-synthesis:none!important;
font-kerning:none!important;font-variant-ligatures:none!important}}
#container {{position:absolute;display:block;left:{20+phase}px;top:{20+phase}px;width:150px;height:{height}px;background:red;{parent_effect}}}
img {{display:{display};width:{image_width}px;height:{height}px;{effects.get(variant,'')}}}
</style><div id="container"><img id="image" src="data:image/png;base64,{base64.b64encode(asset.read_bytes()).decode()}"></div>
''')
                    inputs[state] = dict(path=str(path), sha256=sha(path))
                if variant in variants[:8]:
                    for state, item in inputs.items():
                        previous = RAW / 'native-image-occlusion-consumer-v1436' / f'{case}-{state}.html'
                        if previous.exists():
                            assert previous.read_bytes() == Path(item['path']).read_bytes()
                report['inputs'][case] = inputs
                for scale in [1.0, 1.25, 1.5, 2.0, 3.0]:
                    directory = OUT / case / str(scale)
                    directory.mkdir(parents=True)
                    row = dict(case=case, display=display, variant=variant, phase=phase,
                        scale=scale, native_runs=[], images=[])
                    report['cases'].append(row)
                    geometries = []
                    for repeat in [1, 2]:
                        command = [str(binary), str(directory / f'native-{repeat}'), str(scale), display, variant, str(phase)]
                        result = subprocess.run(command, cwd=ROOT, capture_output=True)
                        log = directory / f'native-{repeat}.log'
                        log.write_bytes(result.stdout + result.stderr)
                        row['native_runs'].append(dict(command=command, observed_exit_code=result.returncode, log_sha256=sha(log)))
                        if result.returncode == 0:
                            geometries.append(json.loads((directory / f'native-{repeat}/geometry.json').read_bytes()))
                    row['native_application_success'] = all(r['observed_exit_code'] == 0 for r in row['native_runs'])
                    if row['native_application_success']:
                        row['callback_and_owned_bounds_verified'] = all(g['callback_count'] == 1 for g in geometries)
                        row['native_repeats_identical'] = geometries[0] == geometries[1] and all(
                            (directory / f'native-1/{state}.png').read_bytes() == (directory / f'native-2/{state}.png').read_bytes()
                            for state in ['before', 'after'])
                        for state in ['before', 'after']:
                            refs, observations = [], []
                            for repeat in [1, 2]:
                                path = directory / f'{state}-chromium-{repeat}.png'
                                observations.append(reference(Path(inputs[state]['path']), path, 320, 240, scale))
                                refs.append(path)
                            assert refs[0].read_bytes() == refs[1].read_bytes()
                            assert observations[0]['query'] == observations[1]['query']
                            actual = directory / f'native-1/{state}.png'
                            analysis = analyze_image_difference(refs[0], actual)
                            row['images'].append(dict(state=state,
                                geometry_exact=geometries[0][state] == observations[0]['query']['bounds'],
                                native_bounds=geometries[0][state], chromium_bounds=observations[0]['query']['bounds'],
                                native_png_sha256=sha(actual), chromium_png_sha256=sha(refs[0]),
                                analysis=analysis, independent_reference_runs=observations))
                    save()
                    print(json.dumps(dict(case=case, scale=scale,
                        native_exits=[r['observed_exit_code'] for r in row['native_runs']],
                        geometry_exact=sum(i['geometry_exact'] for i in row['images']),
                        pixel_differences=[i['analysis']['mismatched_pixels'] for i in row['images']])), flush=True)
    images = [i for row in report['cases'] for i in row['images']]
    report['totals'] = dict(cases=len(report['cases']), images=len(images),
        geometry_exact=sum(i['geometry_exact'] for i in images),
        pixel_exact=sum(i['analysis']['mismatched_pixels'] == 0 for i in images),
        native_runs_passed=sum(r['observed_exit_code'] == 0 for row in report['cases'] for r in row['native_runs']),
        deterministic_native_repeats=sum(row.get('native_repeats_identical', False) for row in report['cases']),
        callback_contracts=sum(row.get('callback_and_owned_bounds_verified', False) for row in report['cases']))
    report['observed_exit_code'] = int(report['totals'] != dict(cases=360, images=720,
        geometry_exact=720, pixel_exact=720, native_runs_passed=720, deterministic_native_repeats=360, callback_contracts=360))
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source and sha(binary) == report['native_binary_sha256']
    assert all(sha(Path(i['path'])) == i['sha256'] for states in report['inputs'].values() for i in states.values())
    save()
raise SystemExit(report['observed_exit_code'])
