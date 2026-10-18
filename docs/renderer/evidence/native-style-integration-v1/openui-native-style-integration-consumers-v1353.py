"""Compare consuming Rust applications with fresh, strictly repeated Chromium captures."""
import base64
import hashlib
import importlib.util
import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image

ROOT = Path('/dev/shm/openui-native-style-integration-287e176a')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
MODE = sys.argv[1]
assert MODE in ('inherited', 'relative', 'static')
OUT = RAW / f'native-style-integration-{MODE}-v1353'
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
buildpath = RAW / 'native-style-integration-clean-v1353/build.json'
build = json.loads(buildpath.read_bytes())
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '0ccc37da4a0def755d5b7f25bb93e92b789787ce'
assert source == build['source'] == build['source_after']
assert len(build['steps']) == 9 and all(s['observed_exit_code'] == 0 for s in build['steps'])
CONFIG = {
    'inherited': ('native-inherited-oracle-v1072', 'native_inherited_styles', 128, 128, 10),
    'relative': ('native-resolved-relative-oracle-v1133', 'native_relative_styles', 320, 240, 50),
    'static': ('native-static-oracle-v1129', 'native_static_position', 320, 160, 60),
}
priorname, binaryname, width, height, expected_images = CONFIG[MODE]
priorpath = RAW / priorname / 'receipt.json'
prior = json.loads(priorpath.read_bytes())
inputs = prior['inputs']
cases = sorted(inputs) if MODE == 'static' else ['default']
flat_inputs = [i for states in inputs.values() for i in states.values()] if MODE == 'static' else list(inputs.values())
assert all(sha(Path(i['path'])) == i['sha256'] for i in flat_inputs)
binary = buildpath.parent / binaryname
assert sha(binary) == next(s['binary_sha256'] for s in build['steps'] if s['name'] == MODE + '-build')
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
chrome = Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome')
env = capture.chrome_environment(str(chrome.parent), True, False)
report = dict(schema_version=1, source=source, source_after=source, mode=MODE,
    prior_input_receipt_sha256=sha(priorpath), inputs=inputs, original_inputs_changed=False,
    native_binary_sha256=sha(binary), build_receipt_sha256=sha(buildpath),
    chromium_binary_sha256=sha(chrome), capture_harness_sha256=sha(harness),
    fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),
    public_native_rust_api=True, javascript_executed_by_openui=False, pixel_tolerance=0,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    all_commands_terminal=False, cases=[], captures=[], probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

expression = '''(async()=>{
if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
await document.fonts.ready;
await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
const element=document.querySelector('#absolute')||document.querySelector('#child');
const r=element.getBoundingClientRect(),s=getComputedStyle(element);
return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
bounds:{x:r.x,y:r.y,width:r.width,height:r.height},
style:{fontSize:s.fontSize,color:s.color,textIndent:s.textIndent}};
})()'''

def reference(html, destination, scale):
    profile = tempfile.mkdtemp(prefix='chrome-', dir=STORE)
    process = client = None
    try:
        command = [str(chrome), '--headless', '--disable-gpu', '--no-sandbox', '--no-first-run',
            '--no-default-browser-check', '--remote-debugging-port=0', '--user-data-dir=' + profile,
            f'--window-size={round(width * scale)},{round(height * scale) + 87}', 'about:blank']
        process = subprocess.Popen(command, env=env, start_new_session=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        port = capture._wait_for_devtools_endpoint(profile, process)
        client = capture._CdpWebSocket(capture._page_websocket_url(port))
        client.command('Page.enable')
        client.command('Emulation.setDeviceMetricsOverride', dict(width=width, height=height,
            deviceScaleFactor=scale, mobile=False, screenWidth=width, screenHeight=height))
        client.command('Page.navigate', dict(url=html.resolve().as_uri()))
        result = client.command('Runtime.evaluate', dict(expression=expression, awaitPromise=True, returnByValue=True))
        assert 'exceptionDetails' not in result, result.get('exceptionDetails')
        query = result['result']['value']
        assert capture._device_metrics_match(query['metrics'], width, height, scale)
        options = dict(format='png', fromSurface=True, captureBeyondViewport=False)
        outputs = []
        for number in (1, 2):
            data = client.command('Page.captureScreenshot', options)
            p = destination.with_name(destination.stem + f'-capture-{number}.png')
            p.write_bytes(base64.b64decode(data['data'], validate=True))
            with Image.open(p) as image:
                assert image.size == (round(width * scale), round(height * scale))
            outputs.append(p)
        observation = dict(query=query, first=str(outputs[0]), second=str(outputs[1]),
            first_sha256=sha(outputs[0]), second_sha256=sha(outputs[1]),
            consecutive_png_bytes_identical=outputs[0].read_bytes() == outputs[1].read_bytes())
        report['captures'].append(observation)
        if not observation['consecutive_png_bytes_identical']:
            observation['decoded_rgba_difference'] = analyze_image_difference(outputs[0], outputs[1])
            save()
            raise AssertionError('unstable Chromium reference; both captures preserved without a retry')
        destination.write_bytes(outputs[0].read_bytes())
        return observation
    finally:
        if client is not None:
            client.close()
        if process is not None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=10)
        shutil.rmtree(profile)

try:
    for case in cases:
        state_inputs = inputs[case] if MODE == 'static' else inputs
        for scale in (1.0, 1.25, 1.5, 2.0, 3.0):
            directory = OUT / case / str(scale)
            directory.mkdir(parents=True)
            command = [str(binary), str(directory / 'native'), str(scale)] + ([case] if MODE == 'static' else [])
            result = subprocess.run(command, cwd=ROOT, capture_output=True)
            log = directory / 'native.log'
            log.write_bytes(result.stdout + result.stderr)
            row = dict(case=case, scale=scale, native_observed_exit_code=result.returncode,
                native_log_sha256=sha(log), images=[])
            report['cases'].append(row)
            if result.returncode == 0:
                geometry = json.loads((directory / 'native/geometry.json').read_bytes()) if MODE == 'static' else None
                for state, item in state_inputs.items():
                    paths, observations = [], []
                    for repeat in (1, 2):
                        path = directory / f'{state}-chromium-{repeat}.png'
                        observations.append(reference(Path(item['path']), path, scale))
                        paths.append(path)
                    assert paths[0].read_bytes() == paths[1].read_bytes(), 'independent Chromium runs disagree; captures preserved'
                    assert observations[0]['query'] == observations[1]['query'], 'independent Chromium geometry differs'
                    actual = directory / 'native' / (state + '.png')
                    analysis = analyze_image_difference(paths[0], actual)
                    image = dict(state=state, input_sha256=item['sha256'], native_png_sha256=sha(actual),
                        chromium_png_sha256=sha(paths[0]), analysis=analysis,
                        independent_chromium_runs_identical=True,
                        owner=None if analysis['mismatched_pixels'] == 0 else
                            ('openui-layout intrinsic sizing and openui-text/openui-paint glyph coverage' if MODE == 'static'
                             else 'openui-engine native declarations and openui-paint border coverage'))
                    if geometry is not None:
                        image.update(native_bounds=geometry[state], chromium_bounds=observations[0]['query']['bounds'],
                            geometry_exact=geometry[state] == observations[0]['query']['bounds'])
                    row['images'].append(image)
            save()
    images = [i for row in report['cases'] for i in row['images']]
    report['totals'] = dict(images=len(images), expected_images=expected_images,
        pixel_exact=sum(i['analysis']['mismatched_pixels'] == 0 for i in images),
        native_consumers_passed=sum(row['native_observed_exit_code'] == 0 for row in report['cases']),
        expected_native_consumers=len(cases) * 5)
    report['observed_exit_code'] = int(report['totals']['images'] != expected_images
        or report['totals']['pixel_exact'] != expected_images
        or report['totals']['native_consumers_passed'] != len(cases) * 5
        or MODE == 'static' and not all(i['geometry_exact'] for i in images))
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source and sha(binary) == report['native_binary_sha256']
    assert all(sha(Path(i['path'])) == i['sha256'] for i in flat_inputs)
    save()
print(json.dumps(report['totals']), flush=True)
raise SystemExit(report['observed_exit_code'])
