"""Native Rust event delegation versus unchanged Chromium capture policies."""
import ast
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

ROOT=Path('/dev/shm/openui-native-event-targets-43706e83')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-event-targets-consumer-v1456'
STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists()
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
build_path=RAW/'native-event-targets-clean-v1456/build.json'
build=json.loads(build_path.read_bytes());source=repository_source_identity(ROOT)
assert source['clean'] and source['commit']=='1c8540e9f8c6eeb8fce10a76cb9a32c2f42f01fd'
assert source==build['source']==build['source_after']
assert build['all_commands_terminal'] and len(build['steps'])==7 and all(row['observed_exit_code']==0 for row in build['steps'])
binary=build_path.parent/'native_event_targets'
assert sha(binary)==next(row['binary_sha256'] for row in build['steps'] if row['name']=='event-build')
harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py'
spec=importlib.util.spec_from_file_location('capture',harness)
capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture)
chrome=Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome')
env=capture.chrome_environment(str(chrome.parent),True,False)
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=False,
    release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,
    public_native_rust_api=True,javascript_executed_by_openui=False,pixel_tolerance=0,
    native_binary_sha256=sha(binary),chromium_binary_sha256=sha(chrome),
    capture_harness_sha256=sha(harness),fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),
    probe_sha256=sha(Path(__file__)),build_receipt_sha256=sha(build_path),
    unstable_reference_captures=[],inputs={},cases=[])
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
save()
# Preserve capture, viewport validation, and mismatch retention. Only remove the
# image-specific natural-size check, because this consuming app paints a div.
capture_source=Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py')
capture_text=capture_source.read_text();nodes=ast.parse(capture_text).body
node=next(node for node in nodes if isinstance(node,ast.FunctionDef) and node.name=='reference')
original_reference_text=ast.get_source_segment(capture_text,node)
removed_line="        assert query['natural'] == dict(width=200, height=200)\n"
assert original_reference_text.count(removed_line)==1
reference_text=original_reference_text.replace(removed_line,'')
exec(compile(reference_text,str(capture_source),'exec'))
report.update(reference_source_sha256=sha(capture_source),
    original_reference_function_sha256=hashlib.sha256(original_reference_text.encode()).hexdigest(),
    adapted_reference_function_sha256=hashlib.sha256(reference_text.encode()).hexdigest(),
    reference_adaptations=['Remove only the image natural-size assertion for a div case'],
    capture_conditions_unchanged=True,strict_chromium_capture_pairs=True,
    preserves_both_unstable_reference_captures=True)
expression="""(async()=>{
 if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));
 await document.fonts.ready;
 await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
 const target=document.getElementById('image'); const r=target.getBoundingClientRect();
 return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},
  bounds:{x:r.x,y:r.y,width:r.width,height:r.height},trace:window.trace,
  phase_after_dispatch:window.saved.map(e=>e.eventPhase),
  current_target_after_dispatch:window.saved.map(e=>e.currentTarget===null),
  target_after_dispatch:window.saved.map(e=>e.target===target)};
})()"""
for state in ['before','after']:
    path=OUT/(state+'.html')
    action="target.click();" if state=='after' else ''
    path.write_text('''<!doctype html><meta charset="utf-8"><style>
* {margin:0;padding:0;border:0;box-sizing:content-box}
html,body {width:160px;height:100px;overflow:hidden;background:white}
#container {width:160px;height:100px}
#image {position:absolute;left:20px;top:20px;width:40px;height:40px;background:blue}
</style><div id="container"><div id="image"></div></div><script>
const parent=document.getElementById('container'),target=document.getElementById('image');
window.trace=[];window.saved=[];
const observe=(name,phase)=>event=>{
 if(event.target!==target || event.currentTarget!==(name==='target'?target:parent) || event.eventPhase!==phase) throw new Error('event target/phase mismatch');
 window.trace.push({name,phase:event.eventPhase});window.saved.push(event);
 if(phase===3) event.target.style.backgroundColor='red';
};
parent.addEventListener('click',observe('root',1),true);
target.addEventListener('click',observe('target',2));
parent.addEventListener('click',observe('root',3));
''' + action + '</script>\n')
    report['inputs'][state]=dict(path=str(path),sha256=sha(path))
save()
try:
    for scale in [1.0,1.25,1.5,2.0,3.0]:
        directory=OUT/str(scale);directory.mkdir()
        row=dict(scale=scale,native_runs=[],images=[]);report['cases'].append(row)
        native=[]
        for repeat in [1,2]:
            command=[str(binary),str(directory/f'native-{repeat}'),str(scale)]
            result=subprocess.run(command,cwd=ROOT,capture_output=True)
            log=directory/f'native-{repeat}.log';log.write_bytes(result.stdout+result.stderr)
            row['native_runs'].append(dict(command=command,observed_exit_code=result.returncode,log_sha256=sha(log)))
            assert result.returncode==0,'native callback/geometry/teardown application failed'
            native.append(json.loads((directory/f'native-{repeat}/events.json').read_bytes()))
        assert native[0]==native[1]
        assert native[0]['callbacks']==3 and native[0]['owned_bounds_unchanged'] and native[0]['phase_cleared'] and native[0]['targets_expired_after_teardown']
        row['native_contract']=native[0]
        for state in ['before','after']:
            actual=directory/f'native-1/{state}.png'
            assert actual.read_bytes()==(directory/f'native-2/{state}.png').read_bytes()
            refs=[];observations=[]
            for repeat in [1,2]:
                path=directory/f'{state}-chromium-{repeat}.png'
                observations.append(reference(Path(report['inputs'][state]['path']),path,160,100,scale))
                refs.append(path)
            assert refs[0].read_bytes()==refs[1].read_bytes()
            assert observations[0]['query']==observations[1]['query']
            query=observations[0]['query']
            assert query['bounds']==native[0]['bounds']
            if state=='after':
                assert query['trace']==[dict(name='root',phase=1),dict(name='target',phase=2),dict(name='root',phase=3)]
                assert query['phase_after_dispatch']==[0]*3 and query['current_target_after_dispatch']==[True]*3 and query['target_after_dispatch']==[True]*3
            else:assert query['trace']==[]
            analysis=analyze_image_difference(refs[0],actual)
            row['images'].append(dict(state=state,analysis=analysis,native_png_sha256=sha(actual),
                chromium_png_sha256=sha(refs[0]),geometry_exact=True,independent_reference_runs=observations))
        save()
        print(json.dumps({'scale':scale,'native_contracts':2,'pixel_differences':[i['analysis']['mismatched_pixels'] for i in row['images']]}),flush=True)
    images=[image for row in report['cases'] for image in row['images']]
    report['totals']=dict(images=len(images),pixel_exact=sum(i['analysis']['mismatched_pixels']==0 for i in images),
        geometry_exact=sum(i['geometry_exact'] for i in images),native_runs=10,deterministic_native_pairs=5,
        independent_chromium_capture_processes=20,consecutive_chromium_captures=40)
    report['observed_exit_code']=int(report['totals']['images']!=10 or report['totals']['pixel_exact']!=10)
except BaseException as error:
    report.update(observed_exit_code=1,failure=str(error));raise
finally:
    report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT))
    assert report['source_after']==source and sha(binary)==report['native_binary_sha256']
    assert all(sha(Path(row['path']))==row['sha256'] for row in report['inputs'].values())
    save()
raise SystemExit(report['observed_exit_code'])
