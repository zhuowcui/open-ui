import hashlib,json,os,subprocess,sys
from pathlib import Path
root=Path('/dev/shm/openui-native-raster-fields-ac08ec56');raw=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1');out=raw/'native-raster-fields-static-v1322';store=Path('/mnt/e/openui-v02-qualification-d174ea0b')/out.name
assert not out.exists() and not store.exists();store.mkdir();out.symlink_to(store,target_is_directory=True)
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
sys.path.insert(0,str(root/'tools/qualification'));from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
buildpath=raw/'native-raster-fields-clean-v1322/build.json';build=json.loads(buildpath.read_bytes());source=repository_source_identity(root)
assert source['clean'] and source==build['source']==build['source_after']
assert len(build['steps'])==16 and all(s['observed_exit_code']==0 for s in build['steps'])
binary=buildpath.parent/'native_static_position';assert sha(binary)==next(s['binary_sha256'] for s in build['steps'] if s['name']=='static-build')
refpath=raw/'native-static-oracle-v1129/receipt.json';ref=json.loads(refpath.read_bytes());refroot=refpath.parent
assert ref['all_commands_terminal'] and len(ref['cases'])==30 and ref['pixel_comparisons_total']==60
assert sha(Path(ref['chrome']['path']))==ref['chrome']['binary_sha256']
assert sha(root/'bindings/rust/openui-text/fonts/Ahem.ttf')==ref['font_sha256']
assert sha(root/'tools/accountability/run_all_pixel_comparisons.py')==ref['chrome']['capture_harness_sha256']
for states in ref['inputs'].values():
 for item in states.values():assert sha(Path(item['path']))==item['sha256']
r={'schema_version':1,'source':source,'source_after':source,'all_commands_terminal':False,'release_qualification':False,'promotion_allowed':False,'public_native_rust_api':True,'javascript_executed_by_openui':False,'new_release_states_admitted':0,'pixel_tolerance':0,'new_chromium_screenshots_generated':0,'retained_independent_chromium_runs_per_state':2,'reference_receipt_sha256':sha(refpath),'reference_inputs_changed':False,'build_receipt_sha256':sha(buildpath),'native_binary_sha256':sha(binary),'probe_sha256':sha(Path(__file__)),'runs':[]}
for policy in ['default','chromium-linux-lcd','chromium-linux-fontations-lcd']:
 for original in ref['cases']:
  case,scale=original['case'],original['scale'];directory=out/policy/case/str(scale);directory.mkdir(parents=True)
  row={'policy':policy,'case':case,'scale':scale,'native_runs':[],'images':[],'native_repeats_identical':False}
  for repeat in [1,2]:
   folder=directory/f'native-{repeat}';command=[str(binary),str(folder),str(scale),case,policy]
   p=subprocess.run(command,cwd=root,capture_output=True);log=directory/f'native-{repeat}.log';log.write_bytes(p.stdout+p.stderr)
   row['native_runs'].append({'repeat':repeat,'command':command,'observed_exit_code':p.returncode,'log_sha256':sha(log)})
  if all(s['observed_exit_code']==0 for s in row['native_runs']):
   geometry=[json.loads((directory/f'native-{i}/geometry.json').read_bytes()) for i in [1,2]]
   row['native_repeats_identical']=geometry[0]==geometry[1] and all((directory/f'native-1/{state}.png').read_bytes()==(directory/f'native-2/{state}.png').read_bytes() for state in ['before','after'])
   for old in original['images']:
    state=old['state'];refs=[refroot/case/str(scale)/f'{state}-chromium-{i}.png' for i in [1,2]]
    assert refs[0].read_bytes()==refs[1].read_bytes() and sha(refs[0])==old['chromium_png_sha256']
    queries=[o['query'] for o in old['independent_reference_runs']];assert queries[0]==queries[1]
    actual=directory/f'native-1/{state}.png';analysis=analyze_image_difference(refs[0],actual);exact_geometry=geometry[0][state]==queries[0]['bounds']
    row['images'].append({'state':state,'chromium_png_sha256':sha(refs[0]),'native_png_sha256':sha(actual),'input_sha256':old['input_sha256'],'native_bounds':geometry[0][state],'chromium_bounds':queries[0]['bounds'],'geometry_exact':exact_geometry,'analysis':analysis,'owner':None if exact_geometry and analysis['mismatched_pixels']==0 else 'openui-layout intrinsic sizing' if not exact_geometry else 'openui-paint text raster policy and glyph coverage'})
  r['runs'].append(row);r['source_after']=repository_source_identity(root);assert r['source_after']==source
  (out/'receipt.json').write_text(json.dumps(r,sort_keys=True,indent=2)+'\n');print(json.dumps({'policy':policy,'case':case,'scale':scale,'native_exits':[v['observed_exit_code'] for v in row['native_runs']],'repeats_identical':row['native_repeats_identical'],'geometry_exact':sum(i['geometry_exact'] for i in row['images']),'pixel_differences':[i['analysis']['mismatched_pixels'] for i in row['images']]}),flush=True)
r['totals']={}
for policy in ['default','chromium-linux-lcd','chromium-linux-fontations-lcd']:
 runs=[v for v in r['runs'] if v['policy']==policy];images=[i for v in runs for i in v['images']]
 r['totals'][policy]={'native_runs_passed':sum(v['observed_exit_code']==0 for s in runs for v in s['native_runs']),'deterministic_repeats':sum(v['native_repeats_identical'] for v in runs),'total':len(images),'geometry_exact':sum(i['geometry_exact'] for i in images),'pixel_exact':sum(i['analysis']['mismatched_pixels']==0 for i in images)}
r['all_commands_terminal']=True;r['all_chromium_policy_images_exact']=r['totals']['chromium-linux-fontations-lcd']=={'native_runs_passed':60,'deterministic_repeats':30,'total':60,'geometry_exact':60,'pixel_exact':60}
assert sha(refpath)==r['reference_receipt_sha256']
(out/'receipt.json').write_text(json.dumps(r,sort_keys=True,indent=2)+'\n');print(json.dumps(r['totals']),flush=True);raise SystemExit(int(not r['all_chromium_policy_images_exact']))
