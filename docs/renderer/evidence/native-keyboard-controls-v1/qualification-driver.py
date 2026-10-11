import fcntl,hashlib,importlib.util,json,os,shutil,signal,subprocess,sys,time
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');SDK=Path('/mnt/e/openui-v02-qualification-d174ea0b/private-native-keyboard-source-v3505');TARGET=Path('/mnt/e/openui-v02-cargo-c73754e2/target');OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-c-qualification-v3528');REF=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-reference-v3506')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
for name in ['cargo','rustc','rustfmt','pixel_compare']:assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
sys.path.insert(0,'/tmp');from openui_parallel_source_identity_v3060 import identity
source=identity(SDK);main_source=identity(MAIN);assert source['clean'] and main_source['clean'];assert source['commit']==Path('/tmp/openui-native-keyboard-c-header-commit-v3527.txt').read_text().strip();assert main_source['commit']=='635619da2000ef5db258f97623689cd474084fdb'
prior=Path('/tmp/openui-native-keyboard-c-failed-terminal-v3526.json');p=json.loads(prior.read_bytes());assert p['all_commands_terminal'] and p['owner_absent'] and p['actual_owner_exit_code']==1 and p['actual_audit_exit_code']==0
ref=json.loads((REF/'receipt.json').read_bytes());assert ref['all_commands_terminal'] and ref['observed_exit_code']==0 and ref['source_unchanged'];assert subprocess.run(['ps','-p',str(ref['owner_pid'])],capture_output=True).returncode==1;assert (REF/'observations-1.json').read_bytes()==(REF/'observations-2.json').read_bytes();assert sha(SDK/'tools/qualification/reproducers/native-control-keyboard-cases.json')==sha(REF/'cases.json')
assert not OUT.exists();OUT.mkdir();(OUT/'abi-scratch').mkdir()
refs={str(REF/name):sha(REF/name) for name in ['receipt.json','cases.json','observations-1.json','observations-2.json']}
report=dict(schema_version=1,owner_pid=os.getpid(),source=source,main_source=main_source,driver_sha256=sha(__file__),reference_inputs=refs,retired_failed_c_compile_terminal_proof_sha256=sha(prior),steps=[],all_commands_terminal=False,javascript_executed_by_openui=False,all_native_apis_qualified=False,renderer_qualified=False,release_qualified=False,implementation_integrated=False,pixel_tolerance=0,native_keyboard_behavior_only=True)
env=dict(os.environ,CARGO_TARGET_DIR=str(TARGET),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='4',RUST_MIN_STACK='4194304',PYTHONDONTWRITEBYTECODE='1',TMPDIR='/dev/shm',OPENUI_CLANG_FORMAT='/tmp/openui-clang18-ci-v2621/unpacked/clang_format/data/bin/clang-format');env.pop('LD_PRELOAD',None);env.pop('LD_LIBRARY_PATH',None)
local={};proof={}
def save():(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
def run(name,command,cwd=None,extra_env=None):
 with (OUT/(name+'.log')).open('xb') as log:
  child=subprocess.Popen(command,cwd=cwd or SDK/'bindings/rust',env=dict(env,**(extra_env or {})),stdout=log,stderr=subprocess.STDOUT,start_new_session=True);report['current_process']=dict(pid=child.pid,name=name);save()
  try:
   while child.poll() is None:
    if shutil.disk_usage(MAIN).free<128*2**20 or shutil.disk_usage(OUT).free<10*2**30:
     report['disk_guard_triggered']=True;os.killpg(child.pid,signal.SIGTERM);break
    time.sleep(1)
   child.wait()
  finally:
   if child.poll() is None:os.killpg(child.pid,signal.SIGTERM);child.wait(timeout=20)
   report.pop('current_process',None);report['steps'].append(dict(name=name,command=command,actual_exit_code=child.returncode,log_sha256=sha(OUT/(name+'.log'))));save()
 print(json.dumps(dict(stage=name,actual_exit_code=child.returncode)),flush=True);assert child.returncode==0,name
 data=(OUT/(name+'.log')).read_bytes()
 if local and command[0]=='cargo':
  artifacts=[]
  for line in data.splitlines():
   if not line.startswith(b'{'):continue
   m=json.loads(line)
   if m.get('reason')!='compiler-artifact' or m['target']['kind']==['custom-build'] or m['package_id'] not in local:continue
   assert Path(m['manifest_path']).resolve()==Path(local[m['package_id']]['manifest_path']).resolve();assert Path(m['target']['src_path']).resolve().is_relative_to(SDK.resolve())
   hashes={p:sha(p) for p in m['filenames']}
   if m['fresh']:assert all(proof.get(p)==h for p,h in hashes.items()),m
   proof.update(hashes);artifacts.append(dict(record=m,sha256=hashes))
  report.setdefault('local_artifacts',{})[name]=artifacts;save()
 return data
def differences(a,b,path=''):
 if type(a)!=type(b):return [dict(path=path,native=a,chromium=b)]
 if isinstance(a,dict):
  out=[]
  for k in sorted(set(a)|set(b)):
   if k not in a or k not in b:out.append(dict(path=path+'/'+k,native=a.get(k),chromium=b.get(k)))
   else:out+=differences(a[k],b[k],path+'/'+k)
  return out
 if isinstance(a,list):
  if len(a)!=len(b):return [dict(path=path,native=a,chromium=b)]
  return [d for i,(x,y) in enumerate(zip(a,b)) for d in differences(x,y,path+'/'+str(i))]
 return [] if a==b else [dict(path=path,native=a,chromium=b)]
save()
try:
 cargo=['cargo','--config',str(SDK/'bindings/rust/.cargo/config.chromium.toml')]
 metadata=json.loads(run('locked-metadata',cargo+['metadata','--locked','--offline','--format-version','1']));local={p['id']:p for p in metadata['packages'] if p['source'] is None};assert all(Path(p['manifest_path']).resolve().is_relative_to(SDK.resolve()) for p in local.values())
 run('retire-audited-local-cargo-cache',cargo+['clean',*[a for p in local.values() for a in ['-p',p['name']]]])
 run('native-rust-build',cargo+['build','--locked','--offline','-p','openui','--features','linux,ffi-integration','--example','native_control_keyboard','--message-format=json'])
 binary=OUT/'native_control_keyboard';shutil.copy2(TARGET/'debug/examples/native_control_keyboard',binary);assert sha(binary) in proof.values();report['rust_binary_sha256']=sha(binary);save()
 run('native-keyboard-release-guards',cargo+['test','--locked','--offline','-p','openui','--features','linux,ffi-integration','--lib','keyboard_activation_guards','--message-format=json'])
 run('ffi-library-build',cargo+['build','--locked','--offline','-p','openui-ffi','--features','linux','--message-format=json'])
 library=OUT/'libopenui.so.0';shutil.copy2(TARGET/'debug/libopenui_ffi.so',library);assert sha(library) in proof.values();report['library_sha256']=sha(library);save()
 run('ffi-active-borrow-unit',cargo+['test','--locked','--offline','-p','openui-ffi','--all-features','--lib','c_active_query_rejects_reentrant_borrow_without_writing_output','--message-format=json'])
 run('ffi-generator-check',[sys.executable,str(SDK/'tools/ffi/generate_ffi.py'),'--check'],SDK)
 run('native-keyboard-generator-check',[sys.executable,str(SDK/'tools/ffi/generate_control_keyboard_cases.py'),'--check'],SDK)
 spec=importlib.util.spec_from_file_location('abi',SDK/'tools/ffi/verify_abi.py');abi=importlib.util.module_from_spec(spec);spec.loader.exec_module(abi);cc,cxx,cflags,cxxflags,linkflags=abi.compilers();assert cc and cxx
 report['consumers']={};save()
 for language,compiler,standard,flags,suffix in [('c',cc,'c11',cflags,'c'),('cpp',cxx,'c++17',cxxflags,'cc')]:
  for stem in ['control_keyboard','control_keyboard_guards']:
   obj=OUT/(stem+'-'+language+'.o');binary_c=OUT/(stem+'-'+language)
   run('compile-'+stem+'-'+language,[compiler,'-std='+standard,'-Wall','-Wextra','-Werror',*flags,'-I'+str(SDK/'include'),'-c',str(SDK/'examples/c_v02'/(stem+'.'+suffix)),'-o',str(obj)],SDK)
   run('link-'+stem+'-'+language,[cxx,*cxxflags,*linkflags,'-fuse-ld=lld',str(obj),str(library),'-Wl,-rpath,'+str(OUT),'-o',str(binary_c)],SDK)
   report['consumers'][stem+'-'+language]=dict(binary_sha256=sha(binary_c),object_sha256=sha(obj));save()
   if stem.endswith('guards'):run('run-'+stem+'-'+language,[str(binary_c)],SDK)
 browser=json.loads((REF/'observations-1.json').read_bytes());report['behavior_results']={};save()
 for language,command in [('rust',[str(binary),str(SDK/'tools/qualification/reproducers/native-control-keyboard-cases.json')]),('c',[str(OUT/'control_keyboard-c')]),('cpp',[str(OUT/'control_keyboard-cpp')])]:
  for repeat in [1,2]:
   data=run('observations-'+language+'-'+str(repeat),command,SDK);json.loads(data);(OUT/('observations-'+language+'-'+str(repeat)+'.json')).write_bytes(data)
  assert (OUT/('observations-'+language+'-1.json')).read_bytes()==(OUT/('observations-'+language+'-2.json')).read_bytes()
  native=json.loads((OUT/('observations-'+language+'-1.json')).read_bytes());assert len(native)==len(browser)==99;comparisons=[]
  for x,y in zip(native,browser):
   assert x['scenario']==y['scenario'];ds=differences(x,y);comparisons.append(dict(scenario=x['scenario'],exact=not ds,differences=ds))
  (OUT/('comparisons-'+language+'.json')).write_text(json.dumps(comparisons,sort_keys=True,indent=2)+'\n');report['behavior_results'][language]=dict(total=99,exact=sum(c['exact'] for c in comparisons),different=sum(not c['exact'] for c in comparisons));save();print(json.dumps(dict(language=language,results=report['behavior_results'][language])),flush=True)
 run('all-c-cpp-abi-consumers',[sys.executable,str(SDK/'tools/ffi/verify_abi.py'),'--library',str(library)],SDK,dict(TMPDIR=str(OUT/'abi-scratch')))
 report['behavior_exact']=all(x['exact']==99 for x in report['behavior_results'].values());report['observed_exit_code']=0 if report['behavior_exact'] else 1
except BaseException as e:report.update(observed_exit_code=1,failure=repr(e));raise
finally:
 report['source_after']=identity(SDK);report['main_source_after']=identity(MAIN);report['source_unchanged']=report['source_after']==source and report['main_source_after']==main_source;report['all_commands_terminal']=True;assert report['source_unchanged'];assert all(sha(p)==h for p,h in refs.items());save()
print(json.dumps({k:report.get(k) for k in ['all_commands_terminal','observed_exit_code','behavior_results','failure']}),flush=True)
raise SystemExit(report['observed_exit_code'])
