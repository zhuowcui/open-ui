import base64
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
OUT = RAW / 'native-fieldset-queries-v1258'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
image = ROOT / 'tools/accountability/data/wpt_assets/sp20/d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe.png'
assert sha(image) == 'd49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe'
inputs = {}
for tag in ('div', 'fieldset'):
    path = OUT / (tag + '.html')
    path.write_text(f'''<!doctype html><meta charset="utf-8"><style>
* {{margin:0;padding:0;box-sizing:content-box}}
html {{overflow:hidden}}
body {{font:16px/1 Ahem;background:white}}
#container {{display:block;border:0;margin:0;padding:0;width:fit-content;height:100px;background:red}}
img {{display:inline;height:100%}}
</style><{tag} id="container"><img id="image" src="data:image/png;base64,{base64.b64encode(image.read_bytes()).decode()}"></{tag}>
''')
    inputs[tag] = dict(path=str(path), sha256=sha(path))
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
chrome = ROOT / 'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
env = capture.chrome_environment(str(chrome.parent), True, False)
report = dict(schema_version=1, queries_only=True, release_qualification=False,
    javascript_executed_by_openui=False, screenshots_generated=0, canvas_pixels_generated=0,
    all_commands_terminal=False, native_geometry_comparisons=0, inputs=inputs,
    image_sha256=sha(image), chromium_binary_sha256=sha(chrome),
    capture_harness_sha256=sha(harness), fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),
    ahem_sha256=sha(ROOT / 'bindings/rust/openui-text/fonts/Ahem.ttf'),
    probe_sha256=sha(Path(__file__)), runs=[])
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
expression = '''(async()=>{
if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});
const container=document.getElementById('container'),image=document.getElementById('image');
if(!image.complete)await new Promise(r=>image.addEventListener('load',r,{once:true}));
const measure=()=>({container:rect(container.getBoundingClientRect()),image:rect(image.getBoundingClientRect())});
const before=measure();container.style.height='150px';const after=measure();
return {url:location.href,metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},before,after,
natural:{width:image.naturalWidth,height:image.naturalHeight},minWidth:getComputedStyle(container).minWidth};
})()'''
try:
    for repeat in (1, 2):
        profile_dir = tempfile.mkdtemp(prefix='chrome-query-', dir=STORE)
        process = client = None
        rows = []
        try:
            command = [str(chrome), '--headless', '--disable-gpu', '--no-sandbox', '--no-first-run',
                       '--no-default-browser-check', '--remote-debugging-port=0',
                       '--user-data-dir=' + profile_dir, 'about:blank']
            process = subprocess.Popen(command, env=env, start_new_session=True,
                                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            port = capture._wait_for_devtools_endpoint(profile_dir, process)
            client = capture._CdpWebSocket(capture._page_websocket_url(port))
            client.command('Page.enable')
            for scale in (1.0, 1.25, 1.5, 2.0, 3.0):
                client.command('Emulation.setDeviceMetricsOverride', dict(width=800, height=600,
                    deviceScaleFactor=scale, mobile=False))
                for tag, item in inputs.items():
                    path = Path(item['path'])
                    client.command('Page.navigate', dict(url=path.resolve().as_uri()))
                    value = client.command('Runtime.evaluate', dict(expression=expression,
                        awaitPromise=True, returnByValue=True))
                    assert 'exceptionDetails' not in value, value.get('exceptionDetails')
                    query = value['result']['value']
                    assert query['url'] == path.resolve().as_uri()
                    assert capture._device_metrics_match(query['metrics'], 800, 600, scale)
                    assert query['natural'] == dict(width=200, height=200)
                    for state, height in (('before', 100), ('after', 150)):
                        assert query[state]['container']['width'] == (200 if tag == 'fieldset' else height)
                        assert query[state]['container']['height'] == height
                        assert query[state]['image']['width'] == query[state]['image']['height'] == height
                    rows.append(dict(tag=tag, scale=scale, query=query))
            report['runs'].append(dict(repeat=repeat, rows=rows))
            save()
            print(json.dumps(dict(repeat=repeat, queries=len(rows))), flush=True)
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
    assert report['runs'][0]['rows'] == report['runs'][1]['rows']
    assert all(sha(Path(i['path'])) == i['sha256'] for i in inputs.values())
    report.update(all_commands_terminal=True, observed_exit_code=0, independent_queries_identical=True,
        container_and_image_measurements=80, native_geometry_comparisons=0,
        expected_native_guard_before_after_verified=True)
    save()
    print(json.dumps(dict(all_commands_terminal=True, observations=80, screenshots_generated=0,
                          sha256=sha(receipt))), flush=True)
except BaseException as error:
    report.update(all_commands_terminal=True, observed_exit_code=1, failure=str(error))
    save()
    raise
