"""Explicit native font engines, 64 logical phases and five device scales.

Scripts run only inside the separate Chromium reference process. The app uses
public Rust methods and callbacks; decoded RGBA equality has zero tolerance.
These fresh diagnostic references do not replace any original oracle inputs.
"""
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

ROOT = Path('/dev/shm/openui-native-font-consumer-0601cd30')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-font-retry-consumer-v1311'
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
build_path = RAW / 'native-font-retry-clean-v1311/build.json'
build = json.loads(build_path.read_bytes())
assert source['clean'] and source == build['source'] == build['source_after']
assert len(build['steps']) == 8 and all(s['observed_exit_code'] == 0 for s in build['steps'])
binary = build_path.parent / 'native_font_raster'
assert sha(binary) == next(s['binary_sha256'] for s in build['steps'] if s['name'] == 'font-build')
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
chrome = Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome')
font_dir = ROOT / 'bindings/rust/openui-text/fonts'
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
families = ('Ahem', 'DejaVu Sans', 'DejaVu Serif', 'DejaVu Sans Mono')
sizes = (10, 12, 16, 20, 24)
scales = (1.0, 1.25, 1.5, 2.0, 3.0)
policies = dict(freetype='Freetype', fontations='Fontations')
inputs = {}
for family in families:
    for size in sizes:
        key = family.replace(' ', '-') + '-' + str(size)
        inputs[key] = {}
        for state, text, color in [('before', 'X', 'black'), ('after', 'XX', 'blue')]:
            nodes = ''.join(
                f'<div id="text-{phase}" style="left:{20 + (phase % 8) * 92 + phase / 64}px;'
                f'top:{20 + (phase // 8) * 60}px">{text}</div>' for phase in range(64))
            path = OUT / f'{key}-{state}.html'
            path.write_text(f'''<!doctype html><meta charset="utf-8"><style>
html,body {{margin:0;padding:0;background:white;overflow:hidden}}
body>div {{position:absolute;font-family:"{family}";font-size:{size}px;
font-weight:normal;font-style:normal;line-height:1;color:{color}}}
</style>{nodes}\n''')
            inputs[key][state] = dict(path=str(path), sha256=sha(path))

report = dict(schema_version=1, source=source, source_after=source,
    build_receipt_sha256=sha(build_path), native_binary_sha256=sha(binary),
    chromium=dict(path=str(chrome), version=version, binary_sha256=sha(chrome)),
    capture_harness_sha256=sha(harness), fontconfig_sha256=sha(fontconfig),
    font_assets={p.name: sha(p) for p in sorted(font_dir.glob('*.ttf'))},
    inputs=inputs, logical_viewport=[800, 600], logical_phases_64ths=list(range(64)),
    scales=list(scales), explicit_font_engines=policies,
    javascript_executed_by_openui=False, public_native_rust_api=True, pixel_tolerance=0,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    all_commands_terminal=False, cases=[], probe_sha256=sha(Path(__file__)))
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

def references(paths, directory, scale, policy, repeat):
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
        client.command('Page.enable')
        client.command('DOM.enable')
        client.command('CSS.enable')
        client.command('Emulation.setDeviceMetricsOverride', dict(width=800, height=600,
            deviceScaleFactor=scale, mobile=False, screenWidth=800, screenHeight=600))
        result = {}
        for state in ('before', 'after'):
            client.command('Page.navigate', dict(url=paths[state].resolve().as_uri()))
            query_result = client.command('Runtime.evaluate', dict(expression=expression,
                awaitPromise=True, returnByValue=True))
            assert 'exceptionDetails' not in query_result, query_result.get('exceptionDetails')
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
            assert first['data'] == second['data'], 'unstable reference surface'
            png = base64.b64decode(first['data'], validate=True)
            with Image.open(io.BytesIO(png)) as image:
                assert image.size == (round(800 * scale), round(600 * scale))
            destination = directory / f'{state}-chromium-{repeat}.png'
            destination.write_bytes(png)
            result[state] = dict(query=query, platform_fonts=fonts, feature_flag=flag,
                repeated_capture_pair_identical=True, capture_count=2,
                chromium_png_sha256=sha(destination))
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

try:
    for policy in policies:
        for family in families:
            for size in sizes:
                key = family.replace(' ', '-') + '-' + str(size)
                for scale in scales:
                    directory = OUT / policy / key / str(scale)
                    directory.mkdir(parents=True)
                    row = dict(policy=policy, family=family, size=size, scale=scale,
                        native_runs=[], images=[])
                    for repeat in (1, 2):
                        command = [str(binary), str(directory / f'native-{repeat}'), str(scale),
                                   policy, family, str(size), 'all', '800', '600']
                        result = subprocess.run(command, cwd=ROOT, capture_output=True)
                        log = directory / f'native-{repeat}.log'
                        log.write_bytes(result.stdout + result.stderr)
                        row['native_runs'].append(dict(repeat=repeat, command=command,
                            observed_exit_code=result.returncode, log_sha256=sha(log)))
                    report['cases'].append(row)
                    save()
                    assert all(r['observed_exit_code'] == 0 for r in row['native_runs']), 'native app failed'
                    geometries = [json.loads((directory / f'native-{r}/geometry.json').read_bytes())
                                  for r in (1, 2)]
                    assert all(g['callback_count'] == 1 for g in geometries)
                    row['native_repeats_identical'] = geometries[0] == geometries[1] and all(
                        (directory / f'native-1/{state}.png').read_bytes()
                        == (directory / f'native-2/{state}.png').read_bytes() for state in ('before', 'after'))
                    assert row['native_repeats_identical'], 'native repeats differ'
                    observations = [references({state: Path(inputs[key][state]['path'])
                        for state in ('before', 'after')}, directory, scale, policy, repeat)
                        for repeat in (1, 2)]
                    for state in ('before', 'after'):
                        refs = [directory / f'{state}-chromium-{repeat}.png' for repeat in (1, 2)]
                        assert refs[0].read_bytes() == refs[1].read_bytes()
                        assert observations[0][state] == observations[1][state]
                        fonts = observations[0][state]['platform_fonts']
                        assert len(fonts) == 1 and fonts[0]['familyName'] == family, fonts
                        actual = directory / f'native-1/{state}.png'
                        analysis = analyze_image_difference(refs[0], actual)
                        with Image.open(actual) as a, Image.open(refs[0]) as b:
                            assert a.size == b.size == (round(800 * scale), round(600 * scale))
                            delta = ImageChops.difference(a.convert('RGBA'), b.convert('RGBA'))
                            mask = functools.reduce(ImageChops.lighter, delta.split())
                            phases = []
                            for phase in range(64):
                                x, y = 12 + phase % 8 * 92, 12 + phase // 8 * 60
                                cell = tuple(round(v * scale) for v in (x, y, x + 92, y + 60))
                                mismatches = sum(mask.crop(cell).histogram()[1:])
                                native_bounds = geometries[0][state][phase]
                                chromium_bounds = observations[0][state]['query']['bounds'][phase]
                                phases.append(dict(logical_phase_64ths=phase,
                                    mismatched_pixels=mismatches, geometry_exact=native_bounds == chromium_bounds,
                                    native_bounds=native_bounds, chromium_bounds=chromium_bounds))
                        row['images'].append(dict(state=state, analysis=analysis, phases=phases,
                            native_png_sha256=sha(actual), chromium_png_sha256=sha(refs[0]),
                            independent_reference_runs=[o[state] for o in observations],
                            owner='openui-text font selection, physical strike and origin' if
                            analysis['mismatched_pixels'] else None))
                    save()
                    print(json.dumps(dict(policy=policy, family=family, size=size, scale=scale,
                        images_exact=sum(i['analysis']['mismatched_pixels'] == 0 for i in row['images']),
                        phases_exact=sum(p['mismatched_pixels'] == 0 for i in row['images'] for p in i['phases']),
                        geometry_exact=sum(p['geometry_exact'] for i in row['images'] for p in i['phases']))), flush=True)
    images = [i for c in report['cases'] for i in c['images']]
    phases = [p for i in images for p in i['phases']]
    report['totals'] = dict(cases=len(report['cases']), images=len(images), phase_states=len(phases),
        images_exact=sum(i['analysis']['mismatched_pixels'] == 0 for i in images),
        phases_exact=sum(p['mismatched_pixels'] == 0 for p in phases),
        geometry_exact=sum(p['geometry_exact'] for p in phases))
    report['observed_exit_code'] = int(report['totals'] != dict(cases=200, images=400,
        phase_states=25600, images_exact=400, phases_exact=25600, geometry_exact=25600))
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source
    assert sha(binary) == report['native_binary_sha256']
    assert sha(chrome) == report['chromium']['binary_sha256']
    assert sha(fontconfig) == report['fontconfig_sha256']
    assert all(sha(Path(i['path'])) == i['sha256'] for states in inputs.values() for i in states.values())
    save()
raise SystemExit(report['observed_exit_code'])
