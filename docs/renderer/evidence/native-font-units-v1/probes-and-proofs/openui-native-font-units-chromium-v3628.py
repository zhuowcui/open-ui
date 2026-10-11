import base64
import fcntl
import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
BASE = Path('/mnt/d/openui-v02-qualification-d174ea0b')
BUILD = BASE/'native-font-units-checks-v3626'
OUT = BASE/'native-font-units-chromium-v3628'
sys.path[:0] = [str(ROOT), '/tmp']
from openui_parallel_source_identity_v3060 import identity
from tools.accountability import run_all_pixel_comparisons as capture
from tools.qualification.residuals import analyze_image_difference
from tools.qualification.generate_renderer_contract import QUALIFICATION_PROFILES

def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(2**20), b''):
            h.update(block)
    return h.hexdigest()

load = lambda p: json.loads(Path(p).read_bytes())
lock = open('/tmp/openui-native-cargo-raster-owner.lock','a+')
fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
b = load(BUILD/'receipt.json')
a = load('/tmp/openui-native-font-units-checks-terminal-v3627.json')
assert b['all_commands_terminal'] and b['source_unchanged']
assert a['all_local_commands_terminal'] and a['receipt_sha256']==sha(BUILD/'receipt.json')
assert a['source']==b['source']==identity(ROOT) and b['source']['clean']
assert subprocess.run(['ps','-p',str(b['owner_pid'])],capture_output=True).returncode==1
for name in ['cargo','rustc','rustfmt','pixel_compare']:
    assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
binary = BUILD/'native_font_units'
assert sha(binary)==b['native_binary_sha256']
chrome = ROOT/'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
env = capture.chrome_environment(str(chrome.parent),True,False)
env['TMPDIR'] = '/dev/shm'
version = subprocess.check_output([str(chrome),'--version'],env=env,text=True).strip()
assert version.split()[-1]=='147.0.7727.50'
assert not OUT.exists()
OUT.mkdir()
r = dict(schema_version=1,owner_pid=os.getpid(),source=b['source'],cases=[],steps=[],
         driver_sha256=sha(__file__),native_binary_sha256=sha(binary),
         build_receipt_sha256=sha(BUILD/'receipt.json'),build_terminal_sha256=sha('/tmp/openui-native-font-units-checks-terminal-v3627.json'),
         chromium_binary_sha256=sha(chrome),chromium_version=version,
         capture_harness_sha256=sha(ROOT/'tools/accountability/run_all_pixel_comparisons.py'),
         fontconfig_sha256=sha(env['FONTCONFIG_FILE']),ahem_font_sha256=sha(ROOT/'bindings/rust/openui-text/fonts/Ahem.ttf'),
         pixel_target='pinned Chromium',pixel_tolerance=0,javascript_executed_by_openui=False,
         scripts_run_only_in_separate_chromium_reference_process=True,
         historical_openui_archive_is_a_pixel_target=False,all_commands_terminal=False,
         full_renderer_contract_qualified=False,all_native_apis_qualified=False,release_qualified=False)

def save():
    p=OUT/'receipt.tmp'
    p.write_text(json.dumps(r,sort_keys=True,indent=2)+'\n')
    os.replace(p,OUT/'receipt.json')

HTML = '''<!doctype html><meta charset="utf-8"><style>
* { margin:0; padding:0; box-sizing:content-box; }
html { overflow:hidden; }
body { overflow:hidden; background:white; font-family:Ahem; font-size:18.72px; }
div { position:absolute; left:10px; height:10px; background:black; }
#ch { top:10px; width:2.5ch; line-height:normal; }
#ex { top:35px; width:2.5ex; line-height:normal; }
#lh-number { top:60px; width:2.5lh; line-height:1.2; }
#lh-percent { top:85px; width:2.5lh; line-height:127.5%; }
</style><div id="ch"></div><div id="ex"></div><div id="lh-number"></div><div id="lh-percent"></div>'''
READY = "new Promise(resolve=>{const done=()=>document.fonts.ready.then(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));if(document.readyState==='complete')done();else addEventListener('load',done,{once:true});})"
QUERY = "Array.from(document.querySelectorAll('[id]'),e=>{const r=e.getBoundingClientRect();return {name:e.id,bounds:{x:r.x,y:r.y,width:r.width,height:r.height}};})"

def oracle(html,directory,repeat,width,height,scale):
    profile=Path('/dev/shm')/f'ou3628-{directory.name}-{repeat}'
    assert not profile.exists()
    profile.mkdir()
    command=[str(chrome),'--headless','--disable-gpu','--no-sandbox','--no-first-run','--no-default-browser-check',
             '--remote-debugging-port=0',f'--user-data-dir={profile}',
             f'--window-size={round(width*scale)},{round(height*scale)+87}','about:blank']
    process=subprocess.Popen(command,env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True)
    client=None
    r['current_process']=dict(name='chromium',pid=process.pid)
    save()
    result={}
    try:
        port=capture._wait_for_devtools_endpoint(profile,process)
        client=capture._CdpWebSocket(capture._page_websocket_url(port))
        client.command('Page.enable')
        client.command('Emulation.setDeviceMetricsOverride',dict(width=width,height=height,deviceScaleFactor=scale,mobile=False,screenWidth=width,screenHeight=height))
        client.command('Page.navigate',dict(url=html.resolve().as_uri()))
        for stage in ['before','after']:
            if stage=='before':
                expression=READY+".then(()=>{document.getElementById('ch').addEventListener('click',()=>{document.body.style.fontSize='20.37px';});return "+QUERY+";})"
            else:
                expression="document.getElementById('ch').click(); "+READY+".then(()=>"+QUERY+")"
            answer=client.command('Runtime.evaluate',dict(expression=expression,awaitPromise=True,returnByValue=True))
            assert 'exceptionDetails' not in answer,answer
            result[stage]=answer['result']['value']
            query_path=directory/f'chromium-{repeat}-{stage}.json'
            query_path.write_text(json.dumps(result[stage],sort_keys=True,indent=2)+'\n')
            pixels=[]
            for number in [1,2]:
                data=base64.b64decode(client.command('Page.captureScreenshot',dict(format='png',fromSurface=True,captureBeyondViewport=False))['data'],validate=True)
                path=directory/f'chromium-{repeat}-{stage}-capture-{number}.png'
                path.write_bytes(data)
                pixels.append(data)
            assert pixels[0]==pixels[1],'Preserved both disagreeing captures'
        metrics=client.command('Runtime.evaluate',dict(expression='({width:innerWidth,height:innerHeight,dpr:devicePixelRatio})',returnByValue=True))['result']['value']
        assert capture._device_metrics_match(metrics,width,height,scale)
        return result
    finally:
        if client:
            client.close()
        capture._stop_chromium_process(process)
        assert process.poll() is not None
        shutil.rmtree(profile)
        r['steps'].append(dict(name='chromium-'+directory.name+'-'+str(repeat),pid=process.pid,actual_exit_code=process.returncode))
        r.pop('current_process',None)
        save()

save()
code=1
try:
    for index,(name,width,height,scale,status) in enumerate(QUALIFICATION_PROFILES):
        width,height=int(width),int(height)
        directory=OUT/f'profile-{index}'
        directory.mkdir()
        html=directory/'test.html'
        html.write_text(HTML)
        row=dict(profile=name,width=width,height=height,scale=scale,input_sha256=sha(html),native_runs=[])
        for repeat in [1,2]:
            destination=directory/f'native-{repeat}'
            command=[str(binary),str(destination),str(width),str(height),str(scale)]
            with (directory/f'native-{repeat}.log').open('xb') as log:
                child=subprocess.Popen(command,cwd=ROOT,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
                r['current_process']=dict(name='native',pid=child.pid)
                save()
                try:
                    child.wait(timeout=120)
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid,signal.SIGTERM)
                        child.wait(timeout=20)
                    r.pop('current_process',None)
                    r['steps'].append(dict(name=f'profile-{index}-native-{repeat}',pid=child.pid,actual_exit_code=child.returncode))
                    save()
            assert child.returncode==0
            row['native_runs'].append(dict(directory=str(destination),files={p.name:sha(p) for p in sorted(destination.iterdir())}))
            assert load(destination/'lifecycle.json')['callbacks']==1
            assert load(destination/'lifecycle.json')['weak_teardown_passed']
        assert row['native_runs'][0]['files']==row['native_runs'][1]['files']
        observations=[oracle(html,directory,repeat,width,height,scale) for repeat in [1,2]]
        assert observations[0]==observations[1]
        row['stages']={}
        for stage in ['before','after']:
            reference=directory/f'chromium-1-{stage}-capture-1.png'
            assert sha(reference)==sha(directory/f'chromium-2-{stage}-capture-1.png')
            native=directory/'native-1'/f'{stage}.png'
            bounds=load(directory/'native-1'/f'{stage}.json')
            analysis=analyze_image_difference(reference,native)
            row['stages'][stage]=dict(bounds_exact=bounds==observations[0][stage],bounds_count=len(bounds),
                                      native_bounds=bounds,chromium_bounds=observations[0][stage],
                                      native_png_sha256=sha(native),chromium_png_sha256=sha(reference),analysis=analysis)
        r['cases'].append(row)
        save()
        print(json.dumps(dict(profile=name,bounds_exact=all(s['bounds_exact'] for s in row['stages'].values()),wrong_pixels={k:v['analysis']['mismatched_pixels'] for k,v in row['stages'].items()})),flush=True)
    r['all_bounds_exact']=all(s['bounds_exact'] for c in r['cases'] for s in c['stages'].values())
    r['all_pixels_exact']=all(s['analysis']['mismatched_pixels']==0 for c in r['cases'] for s in c['stages'].values())
    code=int(not(r['all_bounds_exact'] and r['all_pixels_exact']))
except BaseException as error:
    r['failure']=repr(error)
    raise
finally:
    r['source_after']=identity(ROOT)
    r['source_unchanged']=r['source_after']==r['source']
    r['all_commands_terminal']=True
    r['observed_exit_code']=code
    save()
print(json.dumps(dict(complete=True,actual_exit_code=code,source_unchanged=r['source_unchanged'])),flush=True)
sys.exit(code)
