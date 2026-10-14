import fcntl
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-unit-api-oracle-v3639')
CHROME = ROOT/'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
sys.path[:0] = [str(ROOT), '/tmp']
from openui_parallel_source_identity_v3060 import identity
from tools.accountability import run_all_pixel_comparisons as capture


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


lock = open('/tmp/openui-native-font-unit-api-oracle-v3639.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
source = identity(ROOT)
assert source['clean']
assert not OUT.exists()
OUT.mkdir()
environment = capture.chrome_environment(str(CHROME.parent), True, False)
environment['TMPDIR'] = str(OUT)
version = subprocess.check_output([str(CHROME), '--version'], env=environment, text=True).strip()
assert version.split()[-1] == '147.0.7727.50'
html = '''<!doctype html><meta charset="utf-8"><style>
* { margin:0; padding:0; }
html { font-size:16px; }
body { font:18.72px/1.25 Ahem; }
div { position:absolute; width:2.5lh; height:1px; font-size:30.41px; }
</style>'''
cases = [('ch', '2ch'), ('ex', '2ex'), ('lh', '2lh'), ('em', '1.5em'),
         ('rem', '1.5rem'), ('percentage', '127.5%'), ('number', '1.2')]
for name, line_height in cases:
    html += f'<div id="{name}" style="line-height:{line_height}"></div>'
for name in ['mixed-ch-ex', 'mixed-ch-px', 'same-ch']:
    html += f'<div id="{name}" style="width:2ch;line-height:normal"></div>'
path = OUT/'test.html'
path.write_text(html)
ready = "new Promise(resolve=>{const done=()=>document.fonts.ready.then(()=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));if(document.readyState==='complete')done();else addEventListener('load',done,{once:true});})"
setup = "window.nativeApiGapAnimations=[];for(const [id,a,b] of [['mixed-ch-ex','2ch','3ex'],['mixed-ch-px','2ch','75px'],['same-ch','2ch','3ch']]){const x=document.getElementById(id).animate([{width:a},{width:b}],{duration:1000,fill:'both'});x.pause();x.currentTime=500;window.nativeApiGapAnimations.push(x);}"
query = "({metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},nodes:Array.from(document.querySelectorAll('[id]'),e=>{const r=e.getBoundingClientRect(),s=getComputedStyle(e);return {id:e.id,font_size:s.fontSize,line_height:s.lineHeight,width:s.width,bounds:{x:r.x,y:r.y,width:r.width,height:r.height}}})})"
receipt = dict(schema_version=1, owner_pid=os.getpid(), source=source,
               driver_sha256=sha(__file__), input_sha256=sha(path),
               chromium_version=version, chromium_binary_sha256=sha(CHROME),
               capture_harness_sha256=sha(ROOT/'tools/accountability/run_all_pixel_comparisons.py'),
               fontconfig_sha256=sha(environment['FONTCONFIG_FILE']),
               ahem_sha256=sha(ROOT/'bindings/rust/openui-text/fonts/Ahem.ttf'),
               samples=[], all_commands_terminal=False, reference_only=True,
               native_application_executed=False, native_api_qualification=False,
               native_pixel_qualification=False, javascript_executed_by_openui=False,
               scripts_run_only_in_separate_chromium_reference_process=True)


def save():
    temporary = OUT/'receipt.tmp'
    temporary.write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    os.replace(temporary, OUT/'receipt.json')


save()
status = 1
try:
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0]:
        independent = []
        for repeat in [1, 2]:
            profile = OUT/f'profile-{scale}-{repeat}'
            profile.mkdir()
            child = subprocess.Popen([str(CHROME), '--headless', '--disable-gpu', '--no-sandbox',
                                      '--no-first-run', '--no-default-browser-check',
                                      '--remote-debugging-port=0', f'--user-data-dir={profile}',
                                      '--window-size=800,687', 'about:blank'], env=environment,
                                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                     start_new_session=True)
            receipt['current_process'] = dict(pid=child.pid, scale=scale, repeat=repeat)
            save()
            client = None
            observed = {}
            try:
                port = capture._wait_for_devtools_endpoint(profile, child)
                client = capture._CdpWebSocket(capture._page_websocket_url(port))
                client.command('Page.enable')
                client.command('Emulation.setDeviceMetricsOverride',
                               dict(width=800, height=600, deviceScaleFactor=scale, mobile=False))
                client.command('Page.navigate', dict(url=path.resolve().as_uri()))
                for stage in ['before', 'after']:
                    action = setup if stage == 'before' else "document.body.style.fontSize='20.37px';"
                    expression = ready + '.then(()=>{' + action + 'return ' + ready + ';}).then(()=>'+query+')'
                    result = client.command('Runtime.evaluate',
                                            dict(expression=expression, awaitPromise=True, returnByValue=True))
                    assert 'exceptionDetails' not in result, result
                    observed[stage] = result['result']['value']
                    assert capture._device_metrics_match(observed[stage]['metrics'],800,600,scale)
                    assert len(observed[stage]['nodes']) == 10
                output = OUT/f'observations-{scale}-{repeat}.json'
                output.write_text(json.dumps(observed, indent=2, sort_keys=True) + '\n')
                independent.append(observed)
            finally:
                if client:
                    client.close()
                capture._stop_chromium_process(child)
                assert child.poll() is not None
                receipt.pop('current_process', None)
                save()
                shutil.rmtree(profile)
            assert len(independent) == repeat
        assert independent[0] == independent[1]
        receipt['samples'].append(dict(scale=scale, nodes_per_stage=10, stages=2,
                                       independent_processes=2, repeated_process_observations_identical=True))
        save()
        print(json.dumps(receipt['samples'][-1]), flush=True)
    status = 0
except BaseException as error:
    receipt['failure'] = repr(error)
finally:
    receipt['source_after'] = identity(ROOT)
    receipt['source_unchanged'] = receipt['source_after'] == source
    receipt['all_commands_terminal'] = True
    receipt['observed_exit_code'] = status
    save()
print(json.dumps(dict(complete=True, actual_exit_code=status, source_unchanged=receipt['source_unchanged'])), flush=True)
raise SystemExit(status)
