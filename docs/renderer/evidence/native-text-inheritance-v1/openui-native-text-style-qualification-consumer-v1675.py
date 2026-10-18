"""Public native apps and pinned Chromium; no script executes in Open UI."""
import base64
import functools
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

from PIL import Image, ImageChops

ROOT = Path('/dev/shm/openui-native-text-style-complete-41b616c3')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-text-style-qualification-consumer-v1675'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '41b616c3be5224b074e9d06ba6aa963731a1b265'
build_path = RAW / 'native-text-style-qualification-clean-v1675/build.json'
build = json.loads(build_path.read_bytes())
assert source == build['source'] == build['source_after']
assert build['all_commands_terminal'] and len(build['steps']) == 13
assert all(r['observed_exit_code'] == 0 for r in build['steps'])
binaries = {language: build_path.parent / name for language, name in [
    ('rust', 'native_text_content'), ('c', 'text_content-c'),
    ('cpp', 'text_content-cpp')]}
binary_hashes = {language: sha(path) for language, path in binaries.items()}
for language, path in binaries.items():
    assert binary_hashes[language] == next(r['binary_sha256'] for r in build['steps']
        if r.get('binary') == str(path))
library = build_path.parent / 'libopenui_ffi.so'
library_sha = sha(library)
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
chrome = Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome')
font_dir = ROOT / 'bindings/rust/openui-text/fonts'
font_files = {'Ahem': 'Ahem.ttf', 'DejaVu Sans': 'DejaVuSans.ttf',
              'DejaVu Serif': 'DejaVuSerif.ttf', 'DejaVu Sans Mono': 'DejaVuSansMono.ttf'}
assert all((font_dir / name).is_file() for name in font_files.values())
fontconfig = OUT / 'fontconfig.conf'
fontconfig.write_text(f'''<?xml version="1.0"?>
<!DOCTYPE fontconfig SYSTEM "fonts.dtd">
<fontconfig>
<reset-dirs/><dir>{font_dir}</dir>
<cachedir>{STORE / 'fontconfig-cache'}</cachedir>
<match target="font">
<edit name="antialias" mode="assign"><bool>true</bool></edit>
<edit name="hinting" mode="assign"><bool>true</bool></edit>
<edit name="hintstyle" mode="assign"><const>hintslight</const></edit>
<edit name="autohint" mode="assign"><bool>false</bool></edit>
<edit name="rgba" mode="assign"><const>rgb</const></edit>
</match>
</fontconfig>
''')
env = capture.chrome_environment(str(chrome.parent), False, True)
env['FONTCONFIG_FILE'] = str(fontconfig)
version = subprocess.check_output([str(chrome), '--version'], env=env, text=True).strip()
assert version.split()[-1] == '147.0.7727.50'
policies = dict(fontations='Fontations')
presets = dict(freetype=3, fontations=4)
sizes = (10, 12, 16, 20, 24)
scales = (1.0, 1.25, 1.5, 2.0, 3.0)
prior_path = RAW / 'native-raster-fields-retry-consumer-v1559/receipt.json'
prior = json.loads(prior_path.read_bytes()) if prior_path.exists() else None
prior_cases = {}
font_hashes = {p.name: sha(p) for p in sorted(font_dir.glob('*.ttf'))}
if prior is not None:
    assert prior['all_commands_terminal'], 'prior capture is incomplete; review before reusing'
    assert prior['chromium']['binary_sha256'] == sha(chrome)
    assert prior['chromium']['version'] == version
    assert prior['capture_harness_sha256'] == sha(harness)
    assert prior['font_assets'] == font_hashes
    prior_cases = {(r['policy'], r['family'], r['size'], r['scale']): r for r in prior['cases']}
inputs = {}
for family in font_files:
    for size in sizes:
        key = family.replace(' ', '-') + '-' + str(size)
        inputs[key] = {}
        for state, text, color in [('before', 'X', 'black'), ('after', 'XX', 'blue')]:
            nodes = ''.join(f'<div id="text-{phase}" style="left:{20 + (phase % 8) * 92 + phase / 64}px;'
                           f'top:{20 + (phase // 8) * 60}px">{text}</div>' for phase in range(64))
            path = OUT / f'{key}-{state}.html'
            path.write_text(f'''<!doctype html><meta charset="utf-8"><style>
html,body {{margin:0;padding:0;background:white;overflow:hidden}}
body>div {{position:absolute;font-family:"{family}";font-size:{size}px;
font-weight:normal;font-style:normal;line-height:1;color:{color}}}
</style>{nodes}\n''')
            inputs[key][state] = dict(path=str(path), sha256=sha(path))
            if prior is not None:
                original = prior['inputs'][key][state]
                assert sha(Path(original['path'])) == original['sha256'] == sha(path)

report = dict(schema_version=1, native_raster_policy='immutable default EngineOptions', chromium_reference_policy='pinned Linux default Fontations', cross_language_self_rows_are_not_cross_language_passes=True, source=source, source_after=source,
    build_receipt_sha256=sha(build_path), native_binary_sha256=binary_hashes,
    ffi_library_sha256=library_sha,
    chromium=dict(path=str(chrome), version=version, binary_sha256=sha(chrome)),
    capture_harness_sha256=sha(harness), fontconfig_sha256=sha(fontconfig), font_assets=font_hashes,
    prior_reference_receipt_sha256=sha(prior_path) if prior is not None else None,
    inputs=inputs, logical_viewport=[800, 600], logical_phases_64ths=list(range(64)),
    scales=list(scales), explicit_font_engines=policies, native_languages=list(binaries),
    javascript_executed_by_openui=False, public_native_application_apis=True, pixel_tolerance=0,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    all_commands_terminal=False, cases=[], capture_pairs=[], preserves_both_unequal_reference_captures=True, probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
expression = '''(async()=>{
if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
await document.fonts.ready;
await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
const nodes=[...document.querySelectorAll('body>div')];
return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
bounds:nodes.map(e=>{const r=e.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height}}),
text:nodes.map(e=>e.textContent),font:getComputedStyle(nodes[0]).fontFamily};
})()'''

def fresh_references(paths, directory, scale, policy, repeat):
    profile_dir = tempfile.mkdtemp(prefix='chrome-reference-', dir=STORE)
    process = client = None
    try:
        flag = '--enable-features=FontDataServiceLinux:typeface/' + policies[policy]
        command = [str(chrome), '--headless', '--disable-gpu', '--no-sandbox', '--no-first-run',
            '--no-default-browser-check', '--remote-debugging-port=0', flag,
            '--user-data-dir=' + profile_dir, 'about:blank']
        process = subprocess.Popen(command, env=env, start_new_session=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        port = capture._wait_for_devtools_endpoint(profile_dir, process)
        client = capture._CdpWebSocket(capture._page_websocket_url(port))
        for domain in ('Page', 'DOM', 'CSS'):
            client.command(domain + '.enable')
        client.command('Emulation.setDeviceMetricsOverride', dict(width=800, height=600,
            deviceScaleFactor=scale, mobile=False, screenWidth=800, screenHeight=600))
        result = {}
        for state in ('before', 'after'):
            client.command('Page.navigate', dict(url=paths[state].resolve().as_uri()))
            query_result = client.command('Runtime.evaluate', dict(expression=expression,
                awaitPromise=True, returnByValue=True))
            assert 'exceptionDetails' not in query_result
            query = query_result['result']['value']
            assert capture._device_metrics_match(query['metrics'], 800, 600, scale)
            assert len(query['bounds']) == 64
            assert query['text'] == ['X' if state == 'before' else 'XX'] * 64
            root = client.command('DOM.getDocument', {})['root']['nodeId']
            node = client.command('DOM.querySelector', dict(nodeId=root, selector='#text-0'))['nodeId']
            fonts = client.command('CSS.getPlatformFontsForNode', dict(nodeId=node))['fonts']
            options = dict(format='png', fromSurface=True, captureBeyondViewport=False)
            first = client.command('Page.captureScreenshot', options)
            second = client.command('Page.captureScreenshot', options)
            captures = []
            for number, payload in enumerate([first, second], start=1):
                path = directory / f'{state}-chromium-{repeat}-capture-{number}.png'
                path.write_bytes(base64.b64decode(payload['data'], validate=True))
                with Image.open(path) as image:
                    assert image.size == (round(800 * scale), round(600 * scale))
                captures.append(path)
            pair = dict(state=state, repeat=repeat, scale=scale, policy=policy,
                query=query, platform_fonts=fonts, feature_flag=flag,
                first_path=str(captures[0]), second_path=str(captures[1]),
                first_png_sha256=sha(captures[0]), second_png_sha256=sha(captures[1]),
                consecutive_png_bytes_identical=captures[0].read_bytes() == captures[1].read_bytes())
            report['capture_pairs'].append(pair)
            if not pair['consecutive_png_bytes_identical']:
                pair['decoded_rgba_difference'] = analyze_image_difference(captures[0], captures[1])
                save()
                raise AssertionError('unstable reference; both captures preserved without retry')
            destination = directory / f'{state}-chromium-{repeat}.png'
            destination.write_bytes(captures[0].read_bytes())
            result[state] = dict(query=query, platform_fonts=fonts, feature_flag=flag,
                repeated_capture_pair_identical=True, capture_count=2, chromium_png_sha256=sha(destination))
        return result
    finally:
        if client is not None:
            client.close()
        if process is not None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=10)
        shutil.rmtree(profile_dir)

def references(directory, family, size, scale, policy):
    key = family.replace(' ', '-') + '-' + str(size)
    prior_row = prior_cases.get((policy, family, size, scale))
    if prior is not None and any(not pair['consecutive_png_bytes_identical'] for pair in prior.get('capture_pairs', [])):
        raise AssertionError('earlier reference is unstable; preserve its evidence without recapture')
    images = {i['state']: i for i in prior_row['images']} if prior_row is not None else {}
    if set(images) == {'before', 'after'}:
        observations = [{}, {}]
        previous_directory = prior_path.parent / policy / key / str(scale)
        for state, item in images.items():
            for repeat in (1, 2):
                original = previous_directory / f'{state}-chromium-{repeat}.png'
                observation = item['independent_reference_runs'][repeat - 1]
                assert sha(original) == item['chromium_png_sha256'] == observation['chromium_png_sha256']
                assert observation['repeated_capture_pair_identical'] and observation['capture_count'] == 2
                assert observation['feature_flag'] == '--enable-features=FontDataServiceLinux:typeface/' + policies[policy]
                shutil.copy2(original, directory / original.name)
                observations[repeat - 1][state] = observation
        return observations, True
    observations = [fresh_references({state: Path(inputs[key][state]['path'])
        for state in ('before', 'after')}, directory, scale, policy, repeat) for repeat in (1, 2)]
    # A partially recorded previous capture remains evidence. Do not silently
    # replace a contradictory reference with a new capture.
    for state, item in images.items():
        assert item['chromium_png_sha256'] == sha(directory / f'{state}-chromium-1.png'), \
            'fresh reference contradicts a recorded prior capture'
    return observations, False

try:
    for policy in policies:
        for family in font_files:
            for size in sizes:
                key = family.replace(' ', '-') + '-' + str(size)
                for scale in scales:
                    directory = OUT / policy / key / str(scale)
                    directory.mkdir(parents=True)
                    row = dict(policy=policy, family=family, size=size, scale=scale, native_runs=[], images=[])
                    report['cases'].append(row)
                    geometries = {}
                    for language, binary in binaries.items():
                        for repeat in (1, 2):
                            destination = directory / f'{language}-{repeat}'
                            destination.mkdir()
                            args = [str(scale), family, str(size)] if language == 'rust' \
                                else [str(scale), family, str(size), str(font_dir / font_files[family])]
                            command = [str(binary), str(destination), *args]
                            result = subprocess.run(command, cwd=ROOT, capture_output=True)
                            log = directory / f'{language}-{repeat}.log'
                            log.write_bytes(result.stdout + result.stderr)
                            row['native_runs'].append(dict(language=language, repeat=repeat, command=command,
                                observed_exit_code=result.returncode, log_sha256=sha(log)))
                            save()
                            assert result.returncode == 0, 'native consuming app failed'
                        runs = [json.loads((directory / f'{language}-{repeat}/geometry.json').read_bytes()) for repeat in (1, 2)]
                        assert all(r['callback_count'] == 1 for r in runs)
                        assert runs[0] == runs[1]
                        assert all((directory / f'{language}-1/{state}.png').read_bytes() ==
                            (directory / f'{language}-2/{state}.png').read_bytes() for state in ('before', 'after'))
                        geometries[language] = runs[0]
                    row['native_repeats_identical'] = True
                    observations, reused = references(directory, family, size, scale, policy)
                    row['uses_unchanged_prior_chromium_images'] = reused
                    for state in ('before', 'after'):
                        refs = [directory / f'{state}-chromium-{repeat}.png' for repeat in (1, 2)]
                        assert refs[0].read_bytes() == refs[1].read_bytes()
                        assert observations[0][state] == observations[1][state]
                        fonts = observations[0][state]['platform_fonts']
                        assert len(fonts) == 1 and fonts[0]['familyName'] == family, fonts
                        assert capture._device_metrics_match(observations[0][state]['query']['metrics'], 800, 600, scale)
                        for language in binaries:
                            actual = directory / f'{language}-1/{state}.png'
                            analysis = analyze_image_difference(refs[0], actual)
                            with Image.open(actual) as a, Image.open(refs[0]) as b:
                                assert a.size == b.size == (round(800 * scale), round(600 * scale))
                                delta = ImageChops.difference(a.convert('RGBA'), b.convert('RGBA'))
                                mask = functools.reduce(ImageChops.lighter, delta.split())
                                phases = []
                                for phase in range(64):
                                    x, y = 12 + phase % 8 * 92, 12 + phase // 8 * 60
                                    cell = tuple(round(v * scale) for v in (x, y, x + 92, y + 60))
                                    native_bounds = geometries[language][state][phase]
                                    chromium_bounds = observations[0][state]['query']['bounds'][phase]
                                    phases.append(dict(logical_phase_64ths=phase,
                                        mismatched_pixels=sum(mask.crop(cell).histogram()[1:]),
                                        geometry_exact=native_bounds == chromium_bounds,
                                        native_bounds=native_bounds, chromium_bounds=chromium_bounds))
                            rust_path = directory / f'rust-1/{state}.png'
                            row['images'].append(dict(language=language, state=state, analysis=analysis, phases=phases,
                                native_png_sha256=sha(actual), chromium_png_sha256=sha(refs[0]),
                                rust_pixels_equal=analyze_image_difference(rust_path, actual)['mismatched_pixels'] == 0,
                                rust_geometry_equal=geometries['rust'][state] == geometries[language][state],
                                independent_reference_runs=[o[state] for o in observations],
                                owner='native text layout and default glyph raster; root cause review required' if analysis['mismatched_pixels'] else None,
                                reviewed_root_cause=False if analysis['mismatched_pixels'] else None))
                    save()
                    print(json.dumps(dict(policy=policy, family=family, size=size, scale=scale,
                        prior_references_reused=reused, images_exact=sum(i['analysis']['mismatched_pixels'] == 0 for i in row['images']),
                        images=len(row['images']))), flush=True)
    images = [i for case in report['cases'] for i in case['images']]
    phases = [p for image in images for p in image['phases']]
    report['totals'] = dict(cases=len(report['cases']), images=len(images), phase_states=len(phases),
        images_exact=sum(i['analysis']['mismatched_pixels'] == 0 for i in images),
        phases_exact=sum(p['mismatched_pixels'] == 0 for p in phases),
        geometry_exact=sum(p['geometry_exact'] for p in phases),
        rust_pixels_equal=sum(i['rust_pixels_equal'] for i in images),
        rust_geometry_equal=sum(i['rust_geometry_equal'] for i in images))
    report['observed_exit_code'] = int(report['totals'] != dict(cases=100, images=600, phase_states=38400,
        images_exact=600, phases_exact=38400, geometry_exact=38400, rust_pixels_equal=600, rust_geometry_equal=600))
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source
    assert all(sha(path) == binary_hashes[language] for language, path in binaries.items())
    assert sha(library) == library_sha
    assert sha(chrome) == report['chromium']['binary_sha256']
    assert sha(fontconfig) == report['fontconfig_sha256']
    assert all(sha(Path(i['path'])) == i['sha256'] for values in inputs.values() for i in values.values())
    if prior is not None:
        assert sha(prior_path) == report['prior_reference_receipt_sha256']
    save()
raise SystemExit(report['observed_exit_code'])
