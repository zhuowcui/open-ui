"""Query ordered image/fallback transitions in Chromium; generate no pixels."""
import base64, hashlib, importlib.util, json, os, shutil, signal, subprocess, tempfile
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-inline-fallback-59cac229')
MAIN = Path('/home/nero/code/open-ui')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW / 'native-inline-fallback-geometry-v1570'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir(); OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda p: hashlib.file_digest(p.open('rb'), 'sha256').hexdigest()
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec); spec.loader.exec_module(capture)
chrome = MAIN / 'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
env = capture.chrome_environment(str(chrome.parent), True, False)
asset = ROOT / 'bindings/rust/openui/tests/assets/green-200.png'
font = ROOT / 'bindings/rust/openui-text/fonts/Ahem.ttf'
assert sha(asset) == 'd49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe'
loaded = 'data:image/png;base64,' + base64.b64encode(asset.read_bytes()).decode()
inputs = []
for phase in [0.0, 0.25, 0.5]:
    path = OUT / f'phase-{phase:g}.html'
    path.write_text('<!doctype html><meta charset="utf-8"><style>'
        '*{margin:0;padding:0;border:0;box-sizing:content-box}'
        'html,body{width:320px;height:240px;background:white;overflow:hidden;font:16px/20px Ahem}'
        f'#parent,#cover{{position:absolute;left:{20+phase}px;top:{20+phase}px;width:100px;height:100px}}'
        '#cover{background:green;z-index:1}'
        '#image{display:inline;width:300px;height:200px;columns:5;line-height:20px;orphans:1;widows:1;color:red;background:red}'
        '</style><div id="cover"></div><div id="parent">'
        '<img id="image" alt="XXXXX XXXXX XXXXX XXXXX XXXXX" src="invalid.jpg" width="300" height="200"></div>\n')
    inputs.append(dict(phase=phase, path=str(path), sha256=sha(path)))
expression = '''(async()=>{
if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
const e=document.getElementById('image'),rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});
return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
 bounds:rect(e.getBoundingClientRect()),clientRects:[...e.getClientRects()].map(rect),
 display:getComputedStyle(e).display,natural:{width:e.naturalWidth,height:e.naturalHeight}};
})()'''
report = dict(schema_version=1, queries_only=True, screenshots_generated=0,
    canvas_pixels_generated=0, cargo_commands_run=0, javascript_executed_by_openui=False,
    release_qualification=False, all_commands_terminal=False, inputs=inputs, runs=[],
    chromium_binary_sha256=sha(chrome), capture_harness_sha256=sha(harness),
    image_sha256=sha(asset), ahem_sha256=sha(font), fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),
    native_app_sha256=sha(ROOT/'bindings/rust/openui/examples/native_inline_fallback.rs'),
    probe_sha256=sha(Path(__file__)))
save = lambda: (OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
save()
try:
    for repeat in [1, 2]:
        directory = tempfile.mkdtemp(prefix='chrome-fallback-query-',dir=STORE)
        process = client = None; rows=[]
        try:
            process = subprocess.Popen([str(chrome),'--headless','--disable-gpu','--no-sandbox',
                '--no-first-run','--no-default-browser-check','--remote-debugging-port=0',
                '--user-data-dir='+directory,'about:blank'],env=env,start_new_session=True,
                stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
            port = capture._wait_for_devtools_endpoint(directory,process)
            client = capture._CdpWebSocket(capture._page_websocket_url(port));client.command('Page.enable')
            for scale in [1.0,1.25,1.5,2.0,3.0]:
                client.command('Emulation.setDeviceMetricsOverride',dict(width=320,height=240,deviceScaleFactor=scale,mobile=False))
                for source in inputs:
                    client.command('Page.navigate',dict(url=Path(source['path']).resolve().as_uri()))
                    for state in ['before','loaded','cleared','reattached']:
                        if state in ['loaded','cleared']:
                            uri = loaded if state=='loaded' else 'invalid.jpg'
                            mutation = '(async()=>{const e=document.getElementById("image");await new Promise(resolve=>{e.onload=resolve;e.onerror=resolve;e.src='+json.dumps(uri)+';});e.onload=e.onerror=null;})()'
                            value = client.command('Runtime.evaluate',dict(expression=mutation,awaitPromise=True,returnByValue=True))
                            assert 'exceptionDetails' not in value
                        elif state=='reattached':
                            value = client.command('Runtime.evaluate',dict(expression='const e=document.getElementById("image"),p=e.parentNode;e.remove();p.appendChild(e);'))
                            assert 'exceptionDetails' not in value
                        value = client.command('Runtime.evaluate',dict(expression=expression,awaitPromise=True,returnByValue=True))
                        assert 'exceptionDetails' not in value
                        query=value['result']['value'];assert capture._device_metrics_match(query['metrics'],320,240,scale)
                        assert query['natural']==dict(width=200,height=200) if state=='loaded' else query['natural']==dict(width=0,height=0)
                        rows.append(dict(scale=scale,phase=source['phase'],state=state,query=query))
            report['runs'].append(dict(repeat=repeat,rows=rows));save()
            print(json.dumps(dict(repeat=repeat,observations=len(rows),screenshots=0)),flush=True)
        finally:
            if client:client.close()
            if process:
                if process.poll() is None:os.killpg(process.pid,signal.SIGTERM)
                process.wait(timeout=10)
            shutil.rmtree(directory)
    assert report['runs'][0]['rows']==report['runs'][1]['rows']
    report.update(observed_exit_code=0,independent_queries_identical=True,observations=120)
except BaseException:
    report['observed_exit_code']=1;raise
finally:
    report.update(all_commands_terminal=True,inputs_unchanged=all(sha(Path(r['path']))==r['sha256'] for r in inputs))
    assert report['inputs_unchanged'];assert sha(chrome)==report['chromium_binary_sha256'];save()
print(json.dumps(dict(path=str(OUT/'receipt.json'),sha256=sha(OUT/'receipt.json'),observations=120,identical=True)),flush=True)
