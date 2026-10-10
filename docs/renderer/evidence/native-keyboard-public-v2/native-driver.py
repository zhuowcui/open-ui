import fcntl,hashlib,json,os,re,resource,shutil,signal,subprocess,sys,time
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');SDK=MAIN;TARGET=Path('/mnt/e/openui-v02-cargo-c73754e2/target');OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-regressions-v3547');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
for name in ['cargo','rustc','rustfmt','pixel_compare']:assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
prior=Path('/tmp/openui-native-keyboard-final-local-status-v3536.json');terminal=json.loads(prior.read_bytes());assert terminal['all_local_commands_terminal'] and terminal['main_clean'] and terminal['main_commit']=='cfa249088c0ab6fcd0dffd3c92f0146fe6805a4f'
sys.path.insert(0,'/tmp');from openui_parallel_source_identity_v3060 import identity
relocation=json.loads(Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-cargo-debug-v3548/receipt.json').read_bytes());assert relocation['all_operations_terminal'] and relocation['observed_exit_code']==0 and relocation['artifact_bytes_preserved'] and relocation['compiler_output_inode_aliases_preserved']
source=identity(MAIN);main_source=source;assert source['clean'] and source['commit']==Path('/tmp/openui-native-keyboard-release-test-commit-v3546.txt').read_text().strip()
assert resource.getrlimit(resource.RLIMIT_STACK)[0]==8388608,'Original deep-50 benchmark main stack must remain 8 MiB'
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest();references={str(prior):sha(prior)}
for name in ['native-form-owner-reference-v3410','native-fieldset-reference-v3192','native-fieldset-neighbors-reference-v3211','native-disabled-state-reference-v3212','native-radio-programmatic-reference-v3242','native-checkable-default-reference-v3249','native-checkable-activation-reference-v3257','native-checkable-dispatch-reference-v3315']:
 directory=RAW/name;r=json.loads((directory/'receipt.json').read_bytes());assert r['all_commands_terminal'] and r['observed_exit_code']==0 and r['source_unchanged'];assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode==1
 assert (directory/'observations-1.json').read_bytes()==(directory/'observations-2.json').read_bytes()
 for file in ['receipt.json','observations-1.json','observations-2.json']:references[str(directory/file)]=sha(directory/file)
assert sha(SDK/'tools/qualification/reproducers/native-form-owner-cases.json')==sha(RAW/'native-form-owner-reference-v3410/cases.json')
regression=RAW/'public-native-selection-v3182';reference_modes=dict(focus='native-fieldset-reference-v3192',neighbors='native-fieldset-neighbors-reference-v3211',state='native-disabled-state-reference-v3212')
for name in ['native_input_metadata','native_selection_phases','native_selection_values','native_range_scalar','native_range_modes','native_disabled_focus','selection_tasks-c','selection_tasks-cpp','disabled_focus-c','disabled_focus-cpp','input_metadata-c','input_metadata-cpp']:
 p=regression/(name+'-1.json');references[str(p)]=sha(p)
assert not OUT.exists();OUT.mkdir();(OUT/'abi-scratch').mkdir()
report=dict(schema_version=1,source=source,main_source=source,owner_pid=os.getpid(),driver_sha256=sha(__file__),retired_prior_terminal_proof_sha256=sha(prior),reference_inputs_sha256=references,steps=[],all_commands_terminal=False,implementation_integrated=True,qualification_checkout='canonical clean PR checkout',javascript_executed_by_openui=False,pixel_target='pinned Chromium',pixel_tolerance=0,original_main_stack_limit=resource.getrlimit(resource.RLIMIT_STACK)[0],all_native_apis_qualified=False,renderer_qualified=False,release_qualified=False,broader_all_targets_gate_qualified=False)
env=dict(os.environ,CARGO_TARGET_DIR=str(TARGET),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='4',RUST_MIN_STACK='4194304',PYTHONDONTWRITEBYTECODE='1',TMPDIR='/dev/shm',OPENUI_CLANG_FORMAT='/tmp/openui-clang18-ci-v2621/unpacked/clang_format/data/bin/clang-format');env.pop('LD_PRELOAD',None);env.pop('LD_LIBRARY_PATH',None);local={};proof={}
def save():
 tmp=OUT/'receipt.tmp';tmp.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');os.replace(tmp,OUT/'receipt.json')
def run(name,command,required=True,cwd=None):
 assert not report.get('disk_guard_triggered'), 'A disk-guard stop requires a saved terminal audit before continuation'
 with (OUT/(name+'.log')).open('xb') as log:
  child=subprocess.Popen(command,cwd=cwd or SDK/'bindings/rust',env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);report['current_process']=dict(pid=child.pid,name=name);save()
  try:
   while child.poll() is None:
    if shutil.disk_usage(MAIN).free<128*2**20 or shutil.disk_usage(OUT).free<10*2**30 or shutil.disk_usage(env['CARGO_TARGET_DIR']).free<10*2**30:
     report['disk_guard_triggered']=True;os.killpg(child.pid,signal.SIGTERM);break
    time.sleep(1)
   code=child.wait()
  finally:
   if child.poll() is None:os.killpg(child.pid,signal.SIGTERM);child.wait(timeout=20)
   report.pop('current_process',None);report['steps'].append(dict(name=name,command=command,actual_exit_code=child.returncode,log_sha256=sha(OUT/(name+'.log'))));save()
 print(json.dumps(dict(name=name,actual_exit_code=code)),flush=True)
 data=(OUT/(name+'.log')).read_bytes()
 if local and command[0]=='cargo':
  artifacts=[]
  for line in data.splitlines():
   if not line.startswith(b'{'):continue
   record=json.loads(line)
   if record.get('reason')!='compiler-artifact' or record['target']['kind']==['custom-build'] or record['package_id'] not in local:continue
   assert Path(record['manifest_path']).resolve()==Path(local[record['package_id']]['manifest_path']).resolve()
   assert Path(record['target']['src_path']).resolve().is_relative_to(SDK.resolve())
   hashes={p:sha(p) for p in record['filenames']}
   if record['fresh']:assert name!='locked-all-targets-tests' and all(proof.get(p)==h or any(digest==h and Path(prior).samefile(p) for prior,digest in proof.items()) for p,h in hashes.items()),record
   proof.update(hashes);artifacts.append(dict(record=record,sha256=hashes))
  if artifacts:report.setdefault('local_artifacts',{})[name]=artifacts;save()
 if required:assert code==0,name
 return data
save()
try:
 cargo=['cargo','--config',str(SDK/'bindings/rust/.cargo/config.chromium.toml')]
 metadata=json.loads(run('locked-metadata',cargo+['metadata','--locked','--offline','--format-version','1']))
 local={p['id']:p for p in metadata['packages'] if p['source'] is None}
 assert all(Path(p['manifest_path']).resolve().is_relative_to(SDK.resolve()) for p in local.values())
 run('clean-audited-local-packages',cargo+['clean','--locked']+[a for p in local for a in ['-p',p]])
 all_targets=run('locked-all-targets-tests',cargo+['test','--locked','--offline','--workspace','--all-targets','--all-features','--message-format=json'],required=False)
 all_target_totals=[tuple(map(int,m)) for m in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',all_targets)]
 report['all_targets_tests']=dict(passed=sum(t[0] for t in all_target_totals),failed=sum(t[1] for t in all_target_totals),ignored=sum(t[2] for t in all_target_totals),actual_exit_code=report['steps'][-1]['actual_exit_code'])
 report['broader_all_targets_gate_qualified']=report['all_targets_tests']['actual_exit_code']==0 and report['all_targets_tests']['failed']==0
 save()
 workspace=run('locked-workspace-tests',cargo+['test','--locked','--offline','--workspace','--all-features','--message-format=json'],required=False)
 totals=[tuple(map(int,m)) for m in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',workspace)]
 report['workspace_tests']=dict(passed=sum(t[0] for t in totals),failed=sum(t[1] for t in totals),ignored=sum(t[2] for t in totals));save()
 assert report['workspace_tests']['failed']==0 and report['steps'][-1]['actual_exit_code']==0
 assert report['workspace_tests']['passed']>=8633 and report['workspace_tests']['ignored']==13
 assert report['all_targets_tests']['passed']>=8631 and report['all_targets_tests']['ignored']==0
 app_names=['native_fieldset','native_input_metadata','native_selection_phases','native_selection_values','native_range_scalar','native_range_modes','native_disabled_focus']
 run('native-app-build',cargo+['build','--locked','--offline','-p','openui','--features','linux,ffi-integration','--examples','--message-format=json'])
 report['binaries']={}
 for name in app_names+['native_form_owner','native_checkable','native_disabled_appearance']:
  binary=OUT/name;shutil.copy2(Path(env['CARGO_TARGET_DIR'])/'debug/examples'/name,binary);assert sha(binary) in proof.values();report['binaries'][name]=dict(sha256=sha(binary));save()
  modes=['focus','neighbors','state'] if name=='native_fieldset' else ['reference','dispatch'] if name=='native_checkable' else ['form-owner'] if name=='native_form_owner' else [] if name=='native_disabled_appearance' else ['regression']
  for mode in modes:
   args=[mode] if name=='native_fieldset' or (name=='native_checkable' and mode=='dispatch') else [str(SDK/'tools/qualification/reproducers/native-form-owner-cases.json')] if name=='native_form_owner' else []
   for repeat in [1,2]:
    label=name+'-'+mode+'-'+str(repeat);data=run(label,[str(binary),*args]);json.loads(data);(OUT/(label+'.json')).write_bytes(data)
   assert (OUT/(name+'-'+mode+'-1.json')).read_bytes()==(OUT/(name+'-'+mode+'-2.json')).read_bytes()
 run('ffi-library-build',cargo+['build','--locked','--offline','-p','openui-ffi','--features','linux','--message-format=json'])
 library=OUT/'libopenui_ffi.so';shutil.copy2(Path(env['CARGO_TARGET_DIR'])/'debug/libopenui_ffi.so',library);(OUT/'libopenui.so.0').symlink_to(library.name);report['ffi_library_sha256']=sha(library);assert sha(library) in proof.values();save()
 env['TMPDIR']=str(OUT/'abi-scratch')
 run('all-c-cpp-consumers',[sys.executable,str(SDK/'tools/ffi/verify_abi.py'),'--library',str(library)],cwd=SDK)
 env['TMPDIR']='/dev/shm'
 sys.path.insert(0,str(SDK/'tools/ffi'));import verify_abi
 cc,cxx,c_flags,cxx_flags,link_flags=verify_abi.compilers()
 for language,compiler,flags,standard in [('c',cc,c_flags,'c11'),('cpp',cxx,cxx_flags,'c++17')]:
  for stem in ['fieldset','fieldset_state','selection_tasks','disabled_focus','input_metadata','checkable','form_owner','control_keyboard']:
   example=SDK/'examples/c_v02'/(stem+('.c' if language=='c' else '.cc'));obj=OUT/(stem+'-'+language+'.o');binary=OUT/(stem+'-'+language)
   run('compile-'+stem+'-'+language,[compiler,'-std='+standard,'-Wall','-Wextra','-Werror','-pthread',*flags,'-I'+str(SDK/'include'),str(example),'-c','-o',str(obj)])
   run('link-'+stem+'-'+language,[cxx,'-pthread',*cxx_flags,*link_flags,'-fuse-ld=lld',str(obj),str(library),'-Wl,-rpath,'+str(OUT),'-o',str(binary)])
   report['binaries'][stem+'-'+language]=dict(sha256=sha(binary));save()
   for mode in (['form-owner'] if stem=='form_owner' else ['accessibility','dispatch'] if stem=='checkable' else ['focus','neighbors'] if stem=='fieldset' else ['state'] if stem=='fieldset_state' else ['regression']):
    for repeat in [1,2]:
     label=stem+'-'+language+'-'+mode+'-'+str(repeat);data=run(label,[str(binary)]+([mode] if stem=='fieldset' or (stem=='checkable' and mode=='dispatch') else []));json.loads(data);(OUT/(label+'.json')).write_bytes(data)
    assert (OUT/(stem+'-'+language+'-'+mode+'-1.json')).read_bytes()==(OUT/(stem+'-'+language+'-'+mode+'-2.json')).read_bytes()
 comparisons=[]
 for mode,name in reference_modes.items():
  expected=json.loads((RAW/name/'observations-1.json').read_bytes())
  for language,label in [('rust','native_fieldset-'+mode),('c',('fieldset_state-c-state' if mode=='state' else 'fieldset-c-'+mode)),('cpp',('fieldset_state-cpp-state' if mode=='state' else 'fieldset-cpp-'+mode))]:
   actual=json.loads((OUT/(label+'-1.json')).read_bytes());assert len(actual)==len(expected)
   for a,e in zip(actual,expected):
    reference=e
    if mode=='state' and language!='rust':
     reference=json.loads(json.dumps(e))
     for phase in ['initial','synchronous','observed']:
      for state in reference[phase].values():state.pop('kind');state.pop('parent')
    comparisons.append(dict(language=language,mode=mode,scenario=a['scenario'],exact=a==reference,native=a,chromium=reference,projection=('own/property/effective/enabled/connected only; kind and parent Rust-only' if mode=='state' and language!='rust' else 'complete reference row')))
 checkable_comparisons=[]
 checkable_references=dict(programmatic='native-radio-programmatic-reference-v3242',default='native-checkable-default-reference-v3249',activation='native-checkable-activation-reference-v3257')
 consumers=[('rust','native_checkable-reference'),('rust','native_checkable-dispatch')]+[(language,'checkable-'+language+'-'+mode) for language in ['c','cpp'] for mode in ['accessibility','dispatch']]
 for language,label in consumers:
  actual=json.loads((OUT/(label+'-1.json')).read_bytes());assert set(actual)==set(checkable_references)
  for suite,name in checkable_references.items():
   raw_dispatch=label.endswith('dispatch')
   if raw_dispatch:
    raw_rows=json.loads((RAW/'native-checkable-dispatch-reference-v3315/observations-1.json').read_bytes())
    expected=[{k:v for k,v in e.items() if k!='suite'} for e in raw_rows if e['suite']==suite]
   else:
    expected=json.loads((RAW/name/'observations-1.json').read_bytes())
   assert len(actual[suite])==len(expected)
   for original,e in zip(actual[suite],expected):
    assert original['api_errors']==[], original
    a={k:v for k,v in original.items() if k!='api_errors'}
    checkable_comparisons.append(dict(language=language,consumer=label,mode=suite,scenario=a['scenario'],exact=a==e,native=a,chromium=e,projection='complete behavior row; raw-reference suite marker moved to grouping key; native API error list must be empty',reference_operation=('DOM raw click dispatchEvent' if raw_dispatch else 'HTMLElement.click / native property writes')))
 comparisons.extend(checkable_comparisons)
 report.update(checkable_comparisons=checkable_comparisons,new_reference_cases=220,new_comparison_rows=len(checkable_comparisons),native_api_errors_verified_empty=True)
 regression_rows=[]
 for name in app_names[1:]:
  a=json.loads((OUT/(name+'-regression-1.json')).read_bytes());e=json.loads((regression/(name+'-1.json')).read_bytes());regression_rows.append(dict(language='rust',name=name,scenarios=len(a),unchanged=a==e,reference_sha256=sha(regression/(name+'-1.json'))))
 for stem in ['selection_tasks','disabled_focus','input_metadata']:
  for language in ['c','cpp']:
   a=json.loads((OUT/(stem+'-'+language+'-regression-1.json')).read_bytes());e=json.loads((regression/(stem+'-'+language+'-1.json')).read_bytes());regression_rows.append(dict(language=language,name=stem,scenarios=len(a),unchanged=a==e,reference_sha256=sha(regression/(stem+'-'+language+'-1.json'))))
 report.update(comparisons=comparisons,scenario_counts={language:dict(total=sum(x['language']==language for x in comparisons),exact=sum(x['language']==language and x['exact'] for x in comparisons)) for language in ['rust','c','cpp']},regression_comparisons=regression_rows,all_regression_rows_unchanged=all(x['unchanged'] for x in regression_rows),fresh_local_library_provenance_verified=True)
 form_comparisons=[]
 expected=json.loads((RAW/'native-form-owner-reference-v3410/observations-1.json').read_bytes())
 for language,label in [('rust','native_form_owner-form-owner'),('c','form_owner-c-form-owner'),('cpp','form_owner-cpp-form-owner')]:
  actual=json.loads((OUT/(label+'-1.json')).read_bytes());assert len(actual)==len(expected)==88
  for original,e in zip(actual,expected):
   if language=='rust':assert not original.pop('api_errors')
   form_comparisons.append(dict(language=language,mode='form-owner',scenario=original['scenario'],exact=original==e,native=original,chromium=e,projection='complete reference row, owned public form and parent queries; Rust API errors must be empty'))
 comparisons.extend(form_comparisons)
 report.update(comparisons=comparisons,form_owner_comparisons=form_comparisons,form_owner_rows=len(form_comparisons),scenario_counts={language:dict(total=sum(x['language']==language for x in comparisons),exact=sum(x['language']==language and x['exact'] for x in comparisons)) for language in ['rust','c','cpp']})
 keyboard_reference=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-reference-v3506/observations-1.json');references[str(keyboard_reference)]=sha(keyboard_reference);expected_keyboard=json.loads(keyboard_reference.read_bytes());report['keyboard_comparisons']=[]
 # The example is compiled from the same clean canonical source above.
 keyboard_binary=OUT/'native_control_keyboard';shutil.copy2(TARGET/'debug/examples/native_control_keyboard',keyboard_binary);assert sha(keyboard_binary) in proof.values();report['binaries']['native_control_keyboard']=dict(sha256=sha(keyboard_binary))
 for repeat in [1,2]:
  data=run('native-keyboard-rust-'+str(repeat),[str(keyboard_binary),str(SDK/'tools/qualification/reproducers/native-control-keyboard-cases.json')]);(OUT/('native-keyboard-rust-'+str(repeat)+'.json')).write_bytes(data);assert json.loads(data)==expected_keyboard
 assert (OUT/'native-keyboard-rust-1.json').read_bytes()==(OUT/'native-keyboard-rust-2.json').read_bytes();report['keyboard_behavior_by_language']={'rust':dict(total=99,exact=99,different=0)}
 for language in ['c','cpp']:
  first=OUT/('control_keyboard-'+language+'-regression-1.json');second=OUT/('control_keyboard-'+language+'-regression-2.json');assert first.read_bytes()==second.read_bytes();assert json.loads(first.read_bytes())==expected_keyboard;report['keyboard_behavior_by_language'][language]=dict(total=99,exact=99,different=0)
 save()
 headless=run('headless-c-abi-tests',cargo+['test','--locked','--offline','-p','openui-ffi','--lib','--message-format=json'])
 report['headless_tests']=[tuple(map(int,m)) for m in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',headless)];save()
 report['observed_exit_code']=int(any(x['actual_exit_code']!=0 for x in report['steps']) or not all(x['exact'] for x in comparisons) or not report['all_regression_rows_unchanged']);report['implementation_integrated']=True;report['scoped_native_behavior_qualified']=report['observed_exit_code']==0;save()
except BaseException as error:
 report.update(observed_exit_code=130 if isinstance(error,KeyboardInterrupt) else 1,failure=repr(error));raise
finally:
 report['source_after']=identity(MAIN);report['main_source_after']=report['source_after'];report['source_unchanged']=report['source_after']==source;report['main_source_unchanged']=report['main_source_after']==main_source
 assert report['source_unchanged'] and report['main_source_unchanged']
 assert all(sha(p)==h for p,h in references.items());report['all_commands_terminal']=True;save()
print(json.dumps({k:report.get(k) for k in ['observed_exit_code','all_targets_tests','broader_all_targets_gate_qualified','workspace_tests','scenario_counts','all_regression_rows_unchanged','source_unchanged','main_source_unchanged','form_owner_rows']}),flush=True)
raise SystemExit(report['observed_exit_code'])
