"""Measure intrinsic whitespace in pinned Chromium without generating pixels."""
import hashlib
import importlib.util
import json
import os
import shutil
import signal
import subprocess
import tempfile
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW / 'native-empty-whitespace-query-v1400'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
font = ROOT / 'bindings/rust/openui-text/fonts/Ahem.ttf'
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
chrome = ROOT / 'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
env = capture.chrome_environment(str(chrome.parent), True, False)
variants = [
    ('empty', ''), ('space', ' '), ('tab', '\t'), ('newline', '\n'),
    ('mixed', ' \t\n '), ('nbsp', '\u00a0'), ('emspace', '\u2003'),
    ('narrow-nbsp', '\u202f'), ('break', '<br>'),
    ('edge-span', ' <span style="border:2px solid;padding:0 6px"> </span> '),
]
contexts = [
    ('absolute', 'position:absolute'), ('inline-block', 'display:inline-block'),
    ('float', 'float:left'), ('min-content', 'width:min-content'),
    ('max-content', 'width:max-content'),
]
parts = ['<!doctype html><meta charset="utf-8"><style>'
    '*{margin:0;padding:0;box-sizing:content-box}'
    '.parent{position:relative;width:240px;height:80px}'
    '.target{font-family:Ahem;font-size:16px;line-height:16px;'
    'font-kerning:none;font-variant-ligatures:none;font-synthesis:none}'
    '</style>']
cases = []
for context, style in contexts:
    for whitespace in ['normal', 'nowrap', 'pre-line']:
        for indent in [0, 20, -20]:
            for variant, content in variants:
                key = f'{context}-{whitespace}-{indent}-{variant}'
                cases.append(dict(id=key, context=context, whitespace=whitespace,
                    indent=indent, variant=variant))
                parts.append(f'<div class="parent"><div class="target" id="{key}" '
                    f'style="{style};white-space:{whitespace};text-indent:{indent}px">'
                    f'{content}</div></div>')
assert len(cases) == 450
html = OUT / 'input.html'
html.write_text(''.join(parts), encoding='utf-8')
expression = """(async()=>{
  if(document.readyState!=='complete')
    await new Promise(r=>addEventListener('load',r,{once:true}));
  await document.fonts.ready;
  await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
  return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
    rows:[...document.querySelectorAll('.target')].map(e=>{
      const s=getComputedStyle(e),r=e.getBoundingClientRect();
      return {id:e.id,width:r.width,height:r.height,fontFamily:s.fontFamily,
        whiteSpace:s.whiteSpace,textIndent:s.textIndent};
    })};
})()"""
report = dict(schema_version=1, queries_only=True, screenshots_generated=0,
    canvas_pixels_generated=0, cargo_commands_run=0,
    javascript_executed_by_openui=False, release_qualification=False,
    all_commands_terminal=False, runs=[], cases=cases,
    chromium_binary_sha256=sha(chrome), capture_harness_sha256=sha(harness),
    fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])), ahem_sha256=sha(font),
    input_sha256=sha(html), probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
try:
    for repeat in [1, 2]:
        profile = tempfile.mkdtemp(prefix='chrome-query-', dir=STORE)
        process = client = None
        rows = []
        try:
            command = [str(chrome), '--headless', '--disable-gpu', '--no-sandbox',
                '--no-first-run', '--no-default-browser-check',
                '--remote-debugging-port=0', '--user-data-dir=' + profile, 'about:blank']
            process = subprocess.Popen(command, env=env, start_new_session=True,
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            port = capture._wait_for_devtools_endpoint(profile, process)
            client = capture._CdpWebSocket(capture._page_websocket_url(port))
            client.command('Page.enable')
            for scale in [1.0, 1.25, 1.5, 2.0, 3.0]:
                client.command('Emulation.setDeviceMetricsOverride',
                    dict(width=800, height=600, deviceScaleFactor=scale, mobile=False))
                client.command('Page.navigate', dict(url=html.resolve().as_uri()))
                value = client.command('Runtime.evaluate', dict(expression=expression,
                    awaitPromise=True, returnByValue=True))
                assert 'exceptionDetails' not in value
                query = value['result']['value']
                assert capture._device_metrics_match(query['metrics'], 800, 600, scale)
                assert len(query['rows']) == len(cases)
                rows.append(dict(scale=scale, query=query))
                print(f'repeat {repeat} scale {scale}: 450 geometry queries; no images', flush=True)
            report['runs'].append(dict(repeat=repeat, rows=rows))
            save()
        finally:
            if client:
                client.close()
            if process:
                if process.poll() is None:
                    os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=10)
            shutil.rmtree(profile)
    reference_rows = report['runs'][0]['rows'][0]['query']['rows']
    report['logical_geometry_identical_across_repeats_and_scales'] = all(
        item['query']['rows'] == reference_rows
        for run in report['runs'] for item in run['rows'])
    assert report['logical_geometry_identical_across_repeats_and_scales']
    report.update(observed_exit_code=0, total_geometry_observations=4500)
except BaseException as error:
    report.update(observed_exit_code=1, failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, input_unchanged=sha(html) == report['input_sha256'])
    assert sha(chrome) == report['chromium_binary_sha256']
    assert sha(harness) == report['capture_harness_sha256']
    assert sha(font) == report['ahem_sha256']
    save()
