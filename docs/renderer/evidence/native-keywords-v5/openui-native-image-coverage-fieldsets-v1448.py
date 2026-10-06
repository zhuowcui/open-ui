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
OUT = RAW / 'native-image-coverage-fieldsets-v1448'
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
build_path = RAW / 'native-image-coverage-clean-v1448/build.json'
build = json.loads(build_path.read_bytes())
source = repository_source_identity(ROOT)
assert source['clean'] and source == build['source'] == build['source_after']
assert source['commit'] == '2eacae2c8aad222850af30f9cea6f8a79bbddd4a'
assert build['all_commands_terminal'] and len(build['steps']) == 8 and all(s['observed_exit_code'] == 0 for s in build['steps'])
binary = build_path.parent / 'native_fieldset_intrinsics'
assert sha(binary) == next(s['binary_sha256'] for s in build['steps'] if s['name'] == 'fieldset-build')
asset = ROOT / 'bindings/rust/openui/tests/assets/green-200.png'
assert sha(asset) == 'd49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe'
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
chrome = Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome')
env = capture.chrome_environment(str(chrome.parent), True, False)
profiles = [(800, 600, s) for s in (1.0, 1.25, 1.5, 2.0, 3.0)]
profiles += [(375, 667, 2.0), (1280, 720, 1.25), (1920, 1080, 1.5)]
inputs = {}
for tag in ('div', 'fieldset'):
    for padding in (0, 10):
        for sizing in ('content-box', 'border-box'):
            case = f'{tag}-padding-{padding}-{sizing}'
            inputs[case] = {}
            for state, height in (('before', 100), ('after', 150)):
                path = OUT / f'{case}-{state}.html'
                path.write_text(f'''<!doctype html><meta charset="utf-8"><style>
* {{margin:0;padding:0;box-sizing:content-box}}
html {{overflow:hidden}}
body {{padding:20px;background:white;color:black;font:16px/1 Ahem}}
body,body * {{font-family:Ahem,"Droid Sans Fallback","Noto Sans Devanagari","Noto Color Emoji","DejaVu Sans"!important;
font-weight:normal!important;font-style:normal!important;font-synthesis:none!important;
font-kerning:none!important;font-variant-ligatures:none!important}}
#container {{display:block;border:0;margin:0;padding:{padding}px;box-sizing:{sizing};width:fit-content;height:{height}px;background:red}}
img {{display:inline;height:100%}}
</style><{tag} id="container"><img id="image" src="data:image/png;base64,{base64.b64encode(asset.read_bytes()).decode()}"></{tag}>
''')
                inputs[case][state] = dict(path=str(path), sha256=sha(path))
for case, states in inputs.items():
    for state, row in states.items():
        prior = RAW / 'native-fieldset-capture-forensics-v1345' / f'{case}-{state}.html'
        assert Path(row['path']).read_bytes() == prior.read_bytes()
report = dict(schema_version=1, source=source, source_after=source,
    build_receipt_sha256=sha(build_path), native_binary_sha256=sha(binary),
    image_sha256=sha(asset), ahem_sha256=sha(ROOT / 'bindings/rust/openui-text/fonts/Ahem.ttf'),
    chromium_binary_sha256=sha(chrome), capture_harness_sha256=sha(harness),
    fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])), inputs=inputs,
    javascript_executed_by_openui=False, public_native_rust_api=True, pixel_tolerance=0,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    all_commands_terminal=False, cases=[], unstable_reference_captures=[], capture_conditions_unchanged=True, probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
expression = '''(async()=>{
if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});
const container=document.getElementById('container'),image=document.getElementById('image');
if(!image.complete)await new Promise(r=>image.addEventListener('load',r,{once:true}));
return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
bounds:{container:rect(container.getBoundingClientRect()),image:rect(image.getBoundingClientRect())},
natural:{width:image.naturalWidth,height:image.naturalHeight}};
})()'''

def reference(path, destination, width, height, scale):
    profile_dir = tempfile.mkdtemp(prefix='chrome-reference-', dir=STORE)
    process = client = None
    try:
        command = [str(chrome), '--headless', '--disable-gpu', '--no-sandbox', '--no-first-run',
                   '--no-default-browser-check', '--remote-debugging-port=0',
                   '--user-data-dir=' + profile_dir, 'about:blank']
        process = subprocess.Popen(command, env=env, start_new_session=True,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        port = capture._wait_for_devtools_endpoint(profile_dir, process)
        client = capture._CdpWebSocket(capture._page_websocket_url(port))
        client.command('Page.enable')
        client.command('Emulation.setDeviceMetricsOverride', dict(width=width, height=height,
            deviceScaleFactor=scale, mobile=False, screenWidth=width, screenHeight=height))
        client.command('Page.navigate', dict(url=path.resolve().as_uri()))
        result = client.command('Runtime.evaluate', dict(expression=expression,
            awaitPromise=True, returnByValue=True))
        assert 'exceptionDetails' not in result, result.get('exceptionDetails')
        query = result['result']['value']
        assert capture._device_metrics_match(query['metrics'], width, height, scale)
        assert query['natural'] == dict(width=200, height=200)
        options = dict(format='png', fromSurface=True, captureBeyondViewport=False)
        first = client.command('Page.captureScreenshot', options)
        second = client.command('Page.captureScreenshot', options)
        first_png = base64.b64decode(first['data'], validate=True)
        second_png = base64.b64decode(second['data'], validate=True)
        first_path = destination.with_name(destination.stem + '-first-capture.png')
        second_path = destination.with_name(destination.stem + '-second-capture.png')
        first_path.write_bytes(first_png)
        second_path.write_bytes(second_png)
        observation = dict(destination=str(destination), query=query,
            first_capture=dict(path=str(first_path), sha256=sha(first_path)),
            second_capture=dict(path=str(second_path), sha256=sha(second_path)),
            identical_png_bytes=first_png == second_png)
        if first_png != second_png:
            observation['decoded_rgba_difference'] = analyze_image_difference(first_path, second_path)
            report['unstable_reference_captures'].append(observation)
            save()
        assert first_png == second_png, 'unstable reference surface; both captures preserved'
        png = first_png
        with Image.open(io.BytesIO(png)) as image:
            assert image.size == (round(width * scale), round(height * scale))
        destination.write_bytes(png)
        return dict(query=query, repeated_capture_pair_identical=True, capture_count=2)
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
    for tag in ('div', 'fieldset'):
        for padding in (0, 10):
            for sizing in ('content-box', 'border-box'):
                case = f'{tag}-padding-{padding}-{sizing}'
                for width, height, scale in profiles:
                    directory = OUT / case / f'{width}x{height}@{scale}'
                    directory.mkdir(parents=True)
                    row = dict(case=case, viewport=[width, height], scale=scale, native_runs=[], images=[])
                    for repeat in (1, 2):
                        command = [str(binary), str(directory / f'native-{repeat}'), str(scale), tag,
                                   str(padding), sizing, str(width), str(height)]
                        result = subprocess.run(command, cwd=ROOT, capture_output=True)
                        log = directory / f'native-{repeat}.log'
                        log.write_bytes(result.stdout + result.stderr)
                        row['native_runs'].append(dict(repeat=repeat, command=command,
                            observed_exit_code=result.returncode, log_sha256=sha(log)))
                    if all(r['observed_exit_code'] == 0 for r in row['native_runs']):
                        geometries = [json.loads((directory / f'native-{r}/geometry.json').read_bytes())
                                      for r in (1, 2)]
                        row['native_repeats_identical'] = geometries[0] == geometries[1] and all(
                            (directory / f'native-1/{state}.png').read_bytes()
                            == (directory / f'native-2/{state}.png').read_bytes() for state in ('before', 'after'))
                        for state in ('before', 'after'):
                            refs, observations = [], []
                            for repeat in (1, 2):
                                path = directory / f'{state}-chromium-{repeat}.png'
                                observations.append(reference(Path(inputs[case][state]['path']), path, width, height, scale))
                                refs.append(path)
                            assert refs[0].read_bytes() == refs[1].read_bytes()
                            assert observations[0]['query'] == observations[1]['query']
                            actual = directory / f'native-1/{state}.png'
                            analysis = analyze_image_difference(refs[0], actual)
                            exact_geometry = geometries[0][state] == observations[0]['query']['bounds']
                            row['images'].append(dict(state=state, geometry_exact=exact_geometry,
                                native_bounds=geometries[0][state], chromium_bounds=observations[0]['query']['bounds'],
                                native_png_sha256=sha(actual), chromium_png_sha256=sha(refs[0]),
                                analysis=analysis, independent_reference_runs=observations))
                    report['cases'].append(row)
                    report['source_after'] = repository_source_identity(ROOT)
                    assert report['source_after'] == source
                    save()
                    print(json.dumps(dict(case=case, width=width, height=height, scale=scale,
                        native_exits=[r['observed_exit_code'] for r in row['native_runs']],
                        geometry_exact=sum(i['geometry_exact'] for i in row['images']),
                        pixel_differences=[i['analysis']['mismatched_pixels'] for i in row['images']])), flush=True)
    images = [i for c in report['cases'] for i in c['images']]
    report['totals'] = dict(cases=len(report['cases']), images=len(images),
        geometry_exact=sum(i['geometry_exact'] for i in images),
        pixel_exact=sum(i['analysis']['mismatched_pixels'] == 0 for i in images),
        native_runs_passed=sum(r['observed_exit_code'] == 0 for c in report['cases'] for r in c['native_runs']),
        deterministic_native_repeats=sum(c.get('native_repeats_identical', False) for c in report['cases']))
    report.update(all_commands_terminal=True, observed_exit_code=int(
        report['totals'] != dict(cases=64, images=128, geometry_exact=128, pixel_exact=128,
            native_runs_passed=128, deterministic_native_repeats=64)))
    assert all(sha(Path(i['path'])) == i['sha256'] for states in inputs.values() for i in states.values())
    save()
    print(json.dumps(report['totals']), flush=True)
except BaseException as error:
    report.update(all_commands_terminal=True, observed_exit_code=1, failure=str(error))
    save()
    raise
raise SystemExit(report['observed_exit_code'])
