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
OUT = RAW / 'native-intrinsic-regression-queries-v1256'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
audit_path = RAW / 'native-intrinsic-expanded-audit-v1243.json'
audit = json.loads(audit_path.read_bytes())
assert audit['complete_scope'] and audit['delta_counts']['exact_losses'] == 74
input_rows = []
input_dir = OUT / 'inputs'
input_dir.mkdir()
for tid in audit['changed_test_ids']:
    original = RAW / 'native-intrinsic-clean-expanded-v1212/legacy-800x600@1' / tid / 'test.html'
    path = input_dir / (tid.replace('/', '__') + '.html')
    path.write_bytes(original.read_bytes())
    input_rows.append(dict(id=tid, source_path=str(original), source_sha256=sha(original),
                           path=str(path), sha256=sha(path)))
assert len(input_rows) == 34
harness = ROOT / 'tools/accountability/run_all_pixel_comparisons.py'
spec = importlib.util.spec_from_file_location('capture', harness)
capture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(capture)
chrome = ROOT / 'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
env = capture.chrome_environment(str(chrome.parent), True, False)
profiles = [(800, 600, scale) for scale in (1.0, 1.25, 1.5, 2.0, 3.0)]
profiles += [(375, 667, 2.0), (1280, 720, 1.25), (1920, 1080, 1.5)]
blink = Path('/home/nero/chromium/src/third_party/blink/renderer/core/layout/forms/fieldset_layout_algorithm.cc')
report = dict(schema_version=1, source=audit['source'], prior_source=audit['prior_source'],
    queries_only=True, release_qualification=False, promotion_allowed=False,
    javascript_executed_by_openui=False, screenshots_generated=0, canvas_pixels_generated=0,
    native_geometry_comparisons=0, all_commands_terminal=False,
    audit=dict(path=str(audit_path), sha256=sha(audit_path)),
    chromium_binary_sha256=sha(chrome), capture_harness_sha256=sha(harness),
    fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),
    ahem_sha256=sha(ROOT / 'bindings/rust/openui-text/fonts/Ahem.ttf'),
    blink_fieldset_source=dict(path=str(blink), sha256=sha(blink),
        source_commit=subprocess.check_output(['git', '-C', str(blink.parents[6]), 'rev-parse', 'HEAD'], text=True).strip()),
    probe_sha256=sha(Path(__file__)), inputs=input_rows,
    profiles=[dict(width=w, height=h, scale=s) for w, h, s in profiles], runs=[])
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
expression = '''(async()=>{
if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});
const fields=['display','position','float','clear','width','height','minWidth','maxWidth','minHeight','maxHeight',
'writingMode','direction','boxSizing','textIndent','whiteSpace','boxDecorationBreak',
'marginTop','marginRight','marginBottom','marginLeft','paddingTop','paddingRight','paddingBottom','paddingLeft',
'fontFamily','fontSize','lineHeight','overflowX','overflowY'];
const elements=[...document.body.querySelectorAll('*')].filter(e=>!['STYLE','SCRIPT','LINK','META'].includes(e.tagName));
return {url:location.href,metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
nodes:elements.map((e,index)=>{const s=getComputedStyle(e);const style=Object.fromEntries(fields.map(k=>[k,s[k]]));
const range=document.createRange();range.selectNodeContents(e);
return {index,tag:e.tagName,id:e.id,className:typeof e.className==='string'?e.className:'',
text:e.children.length?'':e.textContent,bounds:rect(e.getBoundingClientRect()),
rects:[...e.getClientRects()].map(rect),textBounds:rect(range.getBoundingClientRect()),style,
image:e.tagName==='IMG'?{naturalWidth:e.naturalWidth,naturalHeight:e.naturalHeight,complete:e.complete}:null};})};
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
            for width, height, scale in profiles:
                client.command('Emulation.setDeviceMetricsOverride', dict(width=width, height=height,
                    deviceScaleFactor=scale, mobile=False))
                for item in input_rows:
                    path = Path(item['path'])
                    client.command('Page.navigate', dict(url=path.resolve().as_uri()))
                    value = client.command('Runtime.evaluate', dict(expression=expression,
                        awaitPromise=True, returnByValue=True))
                    assert 'exceptionDetails' not in value, value.get('exceptionDetails')
                    query = value['result']['value']
                    assert query['url'] == path.resolve().as_uri()
                    assert capture._device_metrics_match(query['metrics'], width, height, scale)
                    rows.append(dict(id=item['id'], width=width, height=height, scale=scale, query=query))
                print(json.dumps(dict(repeat=repeat, width=width, height=height, scale=scale,
                                      completed_queries=len(rows))), flush=True)
            report['runs'].append(dict(repeat=repeat, rows=rows))
            save()
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
    report['independent_queries_identical'] = report['runs'][0]['rows'] == report['runs'][1]['rows']
    assert len(report['runs'][0]['rows']) == len(report['runs'][1]['rows']) == 272
    assert report['independent_queries_identical']
    assert all(sha(Path(i['source_path'])) == i['source_sha256'] == sha(Path(i['path']))
               for i in input_rows)
    report.update(all_commands_terminal=True, observed_exit_code=0,
                  all_immutable_input_bytes_unchanged=True)
    save()
    print(json.dumps(dict(all_commands_terminal=True, independent_queries_identical=True,
                          queries=544, screenshots_generated=0, sha256=sha(receipt))), flush=True)
except BaseException as error:
    report.update(all_commands_terminal=True, observed_exit_code=1, failure=str(error))
    save()
    raise
