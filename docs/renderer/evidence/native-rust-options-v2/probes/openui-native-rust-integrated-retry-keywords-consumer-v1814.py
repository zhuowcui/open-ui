"""Native typed Rust mutation versus strict repeated pinned Chromium captures."""
import ast, base64, hashlib, importlib.util, io, json, os, shutil, signal, subprocess, sys, tempfile
from pathlib import Path
from PIL import Image
ROOT=Path('/dev/shm/openui-native-rust-integrated-retry-2d338d6c-v1814');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-rust-integrated-retry-keywords-consumer-v1814';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
build_path=RAW/'native-rust-integrated-clean-v1810/build.json';build=json.loads(build_path.read_bytes());source=repository_source_identity(ROOT)
assert source['clean'] and source['commit']=='2d338d6cf8334bae0d7893315d85633d2d586aad' and source==build['source']==build['source_after']
assert build['all_commands_terminal'] and len(build['steps'])==15 and all(r['observed_exit_code']==0 for r in build['steps'])
binary=build_path.parent/'native_fragment_keywords';assert sha(binary)==next(r['binary_sha256'] for r in build['steps'] if r['name']=='keywords-build')
harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py';spec=importlib.util.spec_from_file_location('capture',harness);capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture)
chrome=Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome');env=capture.chrome_environment(str(chrome.parent),True,False)
env['TMPDIR']='/dev/shm/oui1814'
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=False,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,public_native_rust_api=True,javascript_executed_by_openui=False,pixel_tolerance=0,native_binary_sha256=sha(binary),chromium_binary_sha256=sha(chrome),capture_harness_sha256=sha(harness),fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),probe_sha256=sha(Path(__file__)),build_receipt_sha256=sha(build_path),unstable_reference_captures=[],inputs={},native_runs=[],cases=[])
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
capture_source=Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py');capture_text=capture_source.read_text();node=next(n for n in ast.parse(capture_text).body if isinstance(n,ast.FunctionDef) and n.name=='reference');original=ast.get_source_segment(capture_text,node)
removed="        assert query['natural'] == dict(width=200, height=200)\n";assert original.count(removed)==1
adapted=original.replace(removed,'')
profile_anchor="tempfile.mkdtemp(prefix='chrome-reference-', dir=STORE)"
assert adapted.count(profile_anchor)==1
adapted=adapted.replace(profile_anchor,"tempfile.mkdtemp(prefix='chrome-reference-', dir=Path("+repr('/dev/shm/oui1814')+"))")
exec(compile(adapted,str(capture_source),'exec'))
report.update(reference_source_sha256=sha(capture_source),original_reference_function_sha256=hashlib.sha256(original.encode()).hexdigest(),adapted_reference_function_sha256=hashlib.sha256(adapted.encode()).hexdigest(),reference_adaptations=['Remove image natural-size assertion for a div case','Store Chromium profile and socket temporary files on a native Linux filesystem'],capture_visual_conditions_unchanged=True,chrome_profile_storage_correction_only=True,capture_conditions_unchanged=True,strict_chromium_capture_pairs=True,preserves_both_unstable_reference_captures=True)
expression="""(async()=>{if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const r=document.getElementById('image').getBoundingClientRect();return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},bounds:{x:r.x,y:r.y,width:r.width,height:r.height}};})()"""
for state in ['before','after']:
 path=OUT/(state+'.html');override='border-top-style:none;background:red;column-fill:balance;break-inside:auto' if state=='after' else ''
 path.write_text('<!doctype html><meta charset="utf-8"><style>*{margin:0;padding:0;border:0;box-sizing:content-box}html,body{width:64px;height:48px;overflow:hidden;background:white}#image{width:20px;height:20px;border:4px solid black;background:blue;column-fill:auto;column-wrap:nowrap;column-span:none;break-inside:avoid;break-before:avoid-column;break-after:avoid-page;box-decoration-break:slice;'+override+'}</style><div id="image"></div>\n')
 report['inputs'][state]=dict(path=str(path),sha256=sha(path))
save()
try:
 for repeat in [1,2]:
  directory=OUT/('native-'+str(repeat));command=[str(binary),str(directory)];result=subprocess.run(command,cwd=ROOT,capture_output=True)
  log=OUT/('native-'+str(repeat)+'.log');log.write_bytes(result.stdout+result.stderr);report['native_runs'].append(dict(command=command,observed_exit_code=result.returncode,log_sha256=sha(log)))
  assert result.returncode==0,'native callback/owned style/geometry/teardown application failed'
  assert result.stdout.count(b'callback=1 owned-styles/bounds passed')==5
  save()
 for scale in [1.0,1.25,1.5,2.0,3.0]:
  directory=OUT/str(scale);directory.mkdir();row=dict(scale=scale,images=[]);report['cases'].append(row)
  for state in ['before','after']:
   actual=OUT/f'native-1/scale-{scale:g}/{state}.png';assert actual.read_bytes()==(OUT/f'native-2/scale-{scale:g}/{state}.png').read_bytes()
   refs=[];observations=[]
   for repeat in [1,2]:
    path=directory/f'{state}-chromium-{repeat}.png';observations.append(reference(Path(report['inputs'][state]['path']),path,64,48,scale));refs.append(path)
   assert refs[0].read_bytes()==refs[1].read_bytes() and observations[0]['query']==observations[1]['query']
   expected=dict(x=0,y=0,width=28,height=28 if state=='before' else 24)
   assert observations[0]['query']['bounds']==expected
   analysis=analyze_image_difference(refs[0],actual)
   row['images'].append(dict(state=state,analysis=analysis,native_png_sha256=sha(actual),chromium_png_sha256=sha(refs[0]),geometry_exact=True,native_geometry_verified_by_application_assertions=True,independent_reference_runs=observations))
  save();print(json.dumps(dict(scale=scale,pixel_differences=[i['analysis']['mismatched_pixels'] for i in row['images']])),flush=True)
 images=[i for row in report['cases'] for i in row['images']]
 report['totals']=dict(images=len(images),pixel_exact=sum(i['analysis']['mismatched_pixels']==0 for i in images),geometry_exact=sum(i['geometry_exact'] for i in images),native_application_runs=2,deterministic_native_image_pairs=10,independent_chromium_capture_processes=20,consecutive_chromium_captures=40,native_callback_states=10)
 report['observed_exit_code']=int(len(images)!=10 or report['totals']['pixel_exact']!=10)
except BaseException as error:
 report.update(observed_exit_code=1,failure=str(error));raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT));assert source==report['source_after'] and sha(binary)==report['native_binary_sha256'];assert all(sha(Path(r['path']))==r['sha256'] for r in report['inputs'].values());save()
raise SystemExit(report['observed_exit_code'])
