import hashlib, json, os, re, shutil, signal, subprocess, sys, time
from pathlib import Path
root=Path('/dev/shm/openui-native-keywords-retry-7d6ffabf-v1779');raw=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
assert os.environ.get('OPENUI_NATIVE_WHOLE_OWNER') == 'native-keywords-current-retry-pipeline-v1780'
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
out=raw/'native-keywords-current-retry-clean-v1779';storage=Path('/mnt/e/openui-v02-qualification-d174ea0b')/out.name;assert not out.exists() and not storage.exists();storage.mkdir();out.symlink_to(storage,target_is_directory=True)
sys.path.insert(0,str(root/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(root);assert source['commit']==subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip() and source['clean'] and source['commit']==__import__('subprocess').check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
env=dict(os.environ,CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',CARGO_INCREMENTAL='0',RUST_MIN_STACK='4194304',CARGO_BUILD_JOBS='4',PYTHONDONTWRITEBYTECODE='1',TMPDIR='/mnt/e/openui-v02-qualification-d174ea0b/native-keywords-current-temporary-v1779',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0')
base=['cargo','--config','.cargo/config.chromium.toml']
metadata=json.loads(subprocess.check_output(base+['metadata','--offline','--locked','--no-deps','--format-version','1'],cwd=root/'bindings/rust',env=env,text=True))
members=set(metadata['workspace_members']);packages=[p['name'] for p in metadata['packages'] if p['id'] in members];assert len(packages)==18
steps=[
 ('clean-workspace',base+['clean']+[arg for package in packages for arg in ['-p',package]],None),
 ('workspace',base+['test','--workspace','--locked','--features','openui-ffi/linux'],None),
 ('keywords-build',base+['build','--locked','-p','openui','--example','native_fragment_keywords'],'debug/examples/native_fragment_keywords'),
 ('ffi-build',base+['build','--locked','-p','openui-ffi','--features','linux'],'debug/libopenui_ffi.so'),
 ('pixel-build',base+['build','--locked','-p','pixel-compare','--bin','pixel_compare'],'debug/pixel_compare'),
 ('native-rust-geometry',[str(out/'native_fragment_keywords')],None),
 ('ffi-consumers',['python3',str(root/'tools/ffi/verify_abi.py'),'--library',str(out/'libopenui_ffi.so')],None),
]
import importlib.util
spec=importlib.util.spec_from_file_location('ffi_verify',root/'tools/ffi/verify_abi.py')
ffi_verify=importlib.util.module_from_spec(spec);spec.loader.exec_module(ffi_verify)
cc,cxx,c_flags,cxx_flags,link_flags=ffi_verify.compilers();assert cc and cxx
for language,suffix,compiler,standard,flags in [('c','c',cc,'c11',c_flags),('cpp','cc',cxx,'c++17',cxx_flags)]:
 object_file=out/('fragment_keywords-'+language+'.o');binary=out/('fragment_keywords-'+language)
 steps.extend([
  ('compile-'+language,[compiler,'-std='+standard,'-Wall','-Wextra','-Werror',*flags,'-I'+str(root/'include'),str(root/'examples/c_v02'/('fragment_keywords.'+suffix)),'-c','-o',str(object_file)],None),
  ('link-'+language,[cxx,*cxx_flags,*link_flags,'-fuse-ld=lld',str(object_file),str(out/'libopenui_ffi.so'),'-Wl,-rpath,'+str(out),'-o',str(binary)],str(binary)),
  ('run-'+language,[str(binary)],None),
 ])
assert len(steps)==13
assert source['commit'] == '7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
report=dict(all_commands_terminal=False, workspace_packages_cleaned=18, schema_version=1,release_qualification=False,source=source,steps=[],cargo_target_dir=env['CARGO_TARGET_DIR'],probe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
try:
 for name,cmd,artifact in steps:
  log=out/(name+'.log')
  tracked=root/'tests/pixel_text/openui_renders/basic_text_openui.png';backup=tracked.read_bytes() if name=='workspace' and tracked.exists() else None
  if name in ('link-c','run-c','link-cpp','run-cpp'):
   assert (out/'libopenui.so.0').resolve()==(out/'libopenui_ffi.so').resolve()
  with log.open('xb') as stream:
   p=subprocess.Popen(cmd,cwd=root/'bindings/rust',env=env,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True);disk_guard=False
   while p.poll() is None:
    if shutil.disk_usage('/home/nero/code/open-ui').free<512*2**20 or shutil.disk_usage(Path(env['CARGO_TARGET_DIR'])).free<10*2**30:
     disk_guard=True;os.killpg(p.pid,signal.SIGTERM);p.wait(timeout=20);break
    time.sleep(1)
   p.wait()
  if backup is not None and tracked.read_bytes()!=backup:
   (out/'generated-diagnostic-openui.png').write_bytes(tracked.read_bytes());tracked.write_bytes(backup)
  entry=dict(name=name,command=cmd,observed_exit_code=p.returncode,disk_guard_triggered=disk_guard,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest());report['steps'].append(entry);report['source_after']=repository_source_identity(root);assert source==report['source_after']
  if name=='workspace':
   totals=[tuple(map(int,m)) for m in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_bytes())];entry.update(passed=sum(t[0] for t in totals),failed=sum(t[1] for t in totals),ignored=sum(t[2] for t in totals))
  if p.returncode==0 and artifact:
   path=out/Path(artifact).name
   if not Path(artifact).is_absolute():shutil.copy2(Path(env['CARGO_TARGET_DIR'])/artifact,path)
   else:assert Path(artifact)==path and path.is_file()
   entry['binary_sha256']=hashlib.sha256(path.read_bytes()).hexdigest();entry['binary']=str(path)
   if name=='ffi-build':
    soname=out/'libopenui.so.0';assert not soname.exists();soname.symlink_to(path.name)
    assert hashlib.sha256(soname.read_bytes()).hexdigest()==entry['binary_sha256']
    entry['installed_soname']=str(soname);entry['soname_sha256']=entry['binary_sha256']
   if name=='pixel-build':
    ident=json.loads(subprocess.check_output([str(path),'build-source-identity'],cwd=root,env=env,text=True));assert ident['source']==source;report['build_identity']=ident
  (out/'build.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({k:entry[k] for k in ['name','observed_exit_code','disk_guard_triggered','passed','failed','ignored'] if k in entry}),flush=True)
  if p.returncode:raise SystemExit(p.returncode)
finally:
 report['all_commands_terminal']=True
 report['source_after']=repository_source_identity(root)
 (out/'build.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
