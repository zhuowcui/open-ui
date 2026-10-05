"""Native Rust callbacks versus exact, repeated pinned Chromium captures."""
import ast,base64,hashlib,importlib.util,io,json,os,shutil,signal,subprocess,sys,tempfile
from pathlib import Path
from PIL import Image
ROOT=Path('/dev/shm/openui-native-table-progress-b7e28e56');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-table-progress-consumer-v1547';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1;assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
build_path=RAW/'native-table-progress-clean-v1547/build.json';build=json.loads(build_path.read_bytes());source=repository_source_identity(ROOT)
assert source['clean'] and source['commit']=='4b3cb72c03c58416a736a207b97cdcc8a225cb5a' and source==build['source']==build['source_after'];assert build['all_commands_terminal'] and len(build['steps'])==7 and all(r['observed_exit_code']==0 for r in build['steps'])
binary=build_path.parent/'native_table_progress';assert sha(binary)==next(r['binary_sha256'] for r in build['steps'] if r['name']=='table-build')
harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py';spec=importlib.util.spec_from_file_location('capture',harness);capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture)
chrome=Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome');env=capture.chrome_environment(str(chrome.parent),True,False)
geometry_path=RAW/'native-table-progress-geometry-v1545/receipt.json';geometry=json.loads(geometry_path.read_bytes());assert geometry['all_commands_terminal'] and geometry['independent_queries_identical']
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=False,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,public_native_rust_api=True,javascript_executed_by_openui=False,pixel_tolerance=0,native_binary_sha256=sha(binary),chromium_binary_sha256=sha(chrome),capture_harness_sha256=sha(harness),fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),probe_sha256=sha(Path(__file__)),build_receipt_sha256=sha(build_path),geometry_receipt_sha256=sha(geometry_path),unstable_reference_captures=[],inputs={},native_runs=[],cases=[])
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
reference_source=Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py');original_text=reference_source.read_text();node=next(n for n in ast.parse(original_text).body if isinstance(n,ast.FunctionDef) and n.name=='reference');original=ast.get_source_segment(original_text,node)
removed="        assert query['natural'] == dict(width=200, height=200)\n";assert original.count(removed)==1;adapted=original.replace(removed,'');exec(compile(adapted,str(reference_source),'exec'))
report.update(reference_source_sha256=sha(reference_source),original_reference_function_sha256=hashlib.sha256(original.encode()).hexdigest(),adapted_reference_function_sha256=hashlib.sha256(adapted.encode()).hexdigest(),reference_adaptations=['Remove only the image natural-size assertion for table div boxes'],capture_conditions_unchanged=True,strict_chromium_capture_pairs=True,preserves_both_unstable_reference_captures=True)
expression='''(async()=>{if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},table:[...document.getElementById('table').getClientRects()].map(rect),body:[...document.getElementById('body').getClientRects()].map(rect)};})()'''
for height in [40,30,20,40.5]:
 for state,body_height in [('before',100),('after',50),('restored',100)]:
  p=OUT/f'height-{height:g}-{state}.html'
  p.write_text('<!doctype html><meta charset="utf-8"><style>*{margin:0;padding:0;border:0;box-sizing:content-box}html,body{width:375px;height:667px;background:white;overflow:hidden}'+f'#outer{{position:absolute;left:120px;top:120px;width:135px;height:{height}px;columns:4;column-gap:16px;column-fill:auto;background:yellow}}'+ '#spacer{margin-bottom:-60px}#inner{columns:1;column-fill:auto;background:lime}#table{display:table}.header{display:table-header-group;break-inside:avoid}.footer{display:table-footer-group;break-inside:avoid}.blue,.pink{width:20px;height:20px}.blue{background:blue}.pink{background:hotpink}.row{display:table-row}.cell{display:table-cell}'+f'#body{{height:{body_height}px;background:black}}'+'</style><div id="outer"><div id="spacer"></div><div id="inner"><div id="table"><div class="header"><div class="blue"></div></div><div class="footer"><div class="pink"></div></div><div class="row"><div class="cell"><div id="body"></div></div></div></div></div></div>\n')
  report['inputs'][f'{height:g}-{state}']=dict(path=str(p),sha256=sha(p))
save()
try:
 for repeat in [1,2]:
  command=[str(binary),str(OUT/f'native-{repeat}')];result=subprocess.run(command,cwd=ROOT,capture_output=True);log=OUT/f'native-{repeat}.log';log.write_bytes(result.stdout+result.stderr);report['native_runs'].append(dict(command=command,observed_exit_code=result.returncode,log_sha256=sha(log)));save();assert result.returncode==0,'native Rust callback, geometry or teardown failed';assert result.stdout.count(b'native geometry passed')==60
 for scale in [1.0,1.25,1.5,2.0,3.0]:
  for height in [40,30,20,40.5]:
   directory=OUT/f'scale-{scale:g}'/f'height-{height:g}';directory.mkdir(parents=True)
   for state,body_height in [('before',100),('after',50),('restored',100)]:
    relative=f'scale-{scale:g}/height-{height:g}/{state}.png';actual=OUT/'native-1'/relative;assert actual.read_bytes()==(OUT/'native-2'/relative).read_bytes()
    expected=next(r['query']['elements'] for r in geometry['runs'][0]['rows'] if r['width']==375 and r['outer_height']==height and r['state']==state)
    observations=[];refs=[]
    for repeat in [1,2]:
     p=directory/f'{state}-chromium-{repeat}.png';observations.append(reference(Path(report['inputs'][f'{height:g}-{state}']['path']),p,375,667,scale));refs.append(p)
     assert observations[-1]['query']['table']==expected[3]['clientRects'];assert observations[-1]['query']['body']==expected[8]['clientRects']
    assert refs[0].read_bytes()==refs[1].read_bytes() and observations[0]['query']==observations[1]['query']
    analysis=analyze_image_difference(refs[0],actual);report['cases'].append(dict(scale=scale,outer_height=height,state=state,analysis=analysis,native_png_sha256=sha(actual),chromium_png_sha256=sha(refs[0]),geometry_exact=True,native_geometry_verified_by_application_assertions=True,independent_reference_runs=observations));save()
   print(json.dumps(dict(scale=scale,outer_height=height,states=3,mismatched_pixels=[r['analysis']['mismatched_pixels'] for r in report['cases'][-3:]])),flush=True)
 rows=report['cases'];report['totals']=dict(images=len(rows),pixel_exact=sum(r['analysis']['mismatched_pixels']==0 for r in rows),geometry_exact=sum(r['geometry_exact'] for r in rows),native_application_runs=2,deterministic_native_image_pairs=60,independent_chromium_capture_processes=120,consecutive_chromium_captures=240,native_callback_states=80);report['observed_exit_code']=int(len(rows)!=60 or report['totals']['pixel_exact']!=60)
except BaseException as error:
 report.update(observed_exit_code=1,failure=str(error));raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT));assert source==report['source_after'] and sha(binary)==report['native_binary_sha256'];assert all(sha(Path(r['path']))==r['sha256'] for r in report['inputs'].values());save()
raise SystemExit(report['observed_exit_code'])
