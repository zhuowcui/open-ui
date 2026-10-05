import hashlib, json, os, re, shutil, signal, subprocess, sys, time
from pathlib import Path
root=Path('/dev/shm/openui-native-text-content-queue-d5bd17d7');raw=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
for prior in ['cache-umbrella-pipeline-v1291', 'native-fieldset-retry-pipeline-v1310', 'native-font-retry-pipeline-v1311', 'native-c-raster-retry-pipeline-v1312', 'native-raster-fields-pipeline-v1337', 'native-default-strike-pipeline-v1338', 'cache-integration-pipeline-v1341', 'native-fieldset-capture-forensics-pipeline-v1346', 'native-style-integration-pipeline-v1354', 'native-intrinsic-constraints-pipeline-v1375', 'native-intrinsic-constraints-pipeline-v1369', 'native-raster-consumer-pipeline-v1398', 'umbrella-clean-workspace-pipeline-v1411', 'native-style-integration-retry-pipeline-v1419', 'native-intrinsic-constraints-retry-pipeline-v1420', 'native-raster-api-pipeline-v1425', 'native-image-occlusion-pipeline-v1437', 'native-image-coverage-pipeline-v1449', 'native-event-targets-pipeline-v1457', 'native-border-contrast-pipeline-v1462', 'native-intrinsic-cache-guard-pipeline-v1469', 'native-raster-fields-public-pipeline-v1470', 'native-image-imports-pipeline-v1471', 'native-rounded-border-pipeline-v1482', 'native-border-guard-pipeline-v1501', 'native-enum-values-pipeline-v1513', 'native-inline-replaced-pipeline-v1529', 'native-table-progress-pipeline-v1548', 'native-raster-fields-retry-pipeline-v1560', 'native-inline-fallback-pipeline-v1576', 'native-keywords-pipeline-v1588', 'native-table-source-pipeline-v1601']:
 assert json.loads((raw/prior/'receipt.json').read_bytes())['all_commands_terminal']
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
out=raw/'native-text-content-clean-v1620';storage=Path('/mnt/e/openui-v02-qualification-d174ea0b')/out.name;assert not out.exists();storage.mkdir();out.symlink_to(storage,target_is_directory=True)
sys.path.insert(0,str(root/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(root);assert source['commit']==subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip() and source['clean'] and source['commit']==__import__('subprocess').check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip()
env=dict(os.environ,CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',CARGO_INCREMENTAL='0',RUST_MIN_STACK='4194304',CARGO_BUILD_JOBS='4',PYTHONDONTWRITEBYTECODE='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0')
base=['cargo','--config','.cargo/config.chromium.toml']
metadata=json.loads(subprocess.check_output(base+['metadata','--offline','--locked','--no-deps','--format-version','1'],cwd=root/'bindings/rust',env=env,text=True))
members=set(metadata['workspace_members']);packages=[p['name'] for p in metadata['packages'] if p['id'] in members];assert len(packages)==18
steps=[
 ('clean-workspace',base+['clean']+[arg for package in packages for arg in ['-p',package]],None),
 ('workspace',base+['test','--workspace','--locked','--features','openui-ffi/linux'],None),
 ('text-build',base+['build','--locked','-p','openui','--example','native_text_content'],'debug/examples/native_text_content'),
 ('ffi-build',base+['build','--locked','-p','openui-ffi','--features','linux'],'debug/libopenui_ffi.so'),
 ('pixel-build',base+['build','--locked','-p','pixel-compare','--bin','pixel_compare'],'debug/pixel_compare'),
 ('ffi-consumers',['python3',str(root/'tools/ffi/verify_abi.py'),'--library',str(out/'libopenui_ffi.so')],None),
]
import importlib.util
spec=importlib.util.spec_from_file_location('ffi_verify',root/'tools/ffi/verify_abi.py')
ffi_verify=importlib.util.module_from_spec(spec);spec.loader.exec_module(ffi_verify)
cc,cxx,c_flags,cxx_flags,link_flags=ffi_verify.compilers();assert cc and cxx
for language,suffix,compiler,standard,flags in [('c','c',cc,'c11',c_flags),('cpp','cc',cxx,'c++17',cxx_flags)]:
 object_file=out/('text_content-'+language+'.o');binary=out/('text_content-'+language)
 steps.extend([
  ('compile-'+language,[compiler,'-std='+standard,'-Wall','-Wextra','-Werror',*flags,'-I'+str(root/'include'),str(root/'examples/c_v02'/('text_content.'+suffix)),'-c','-o',str(object_file)],None),
  ('link-'+language,[cxx,*cxx_flags,*link_flags,'-fuse-ld=lld',str(object_file),str(out/'libopenui_ffi.so'),'-Wl,-rpath,'+str(out),'-o',str(binary)],str(binary)),
  ('run-'+language,[str(binary)],None),
 ])
assert len(steps)==12
assert source['commit'] == 'd5bd17d77a2262f10b97e445d380723dff093d51'
report=dict(all_commands_terminal=False, workspace_packages_cleaned=18, schema_version=1,release_qualification=False,source=source,steps=[],cargo_target_dir=env['CARGO_TARGET_DIR'],probe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
try:
 for name,cmd,artifact in steps:
  log=out/(name+'.log')
  tracked=root/'tests/pixel_text/openui_renders/basic_text_openui.png';backup=tracked.read_bytes() if name=='workspace' and tracked.exists() else None
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
  if name=='workspace' and p.returncode==0:
   content=log.read_bytes()
   assert b'test shaping::shape_result::tests::fontations_lcd_hints_at_physical_size_before_replay ...' not in content, 'private text test absent from declared source was executed'
   entry['known_unattributed_private_text_test_executed']=False
  if p.returncode==0 and artifact:
   path=out/Path(artifact).name
   if not Path(artifact).is_absolute():shutil.copy2(Path(env['CARGO_TARGET_DIR'])/artifact,path)
   else:assert Path(artifact)==path and path.is_file()
   entry['binary_sha256']=hashlib.sha256(path.read_bytes()).hexdigest();entry['binary']=str(path)
   if name=='pixel-build':
    ident=json.loads(subprocess.check_output([str(path),'build-source-identity'],cwd=root,env=env,text=True));assert ident['source']==source;report['build_identity']=ident
  (out/'build.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(entry),flush=True)
  if p.returncode:raise SystemExit(p.returncode)
finally:
 report['all_commands_terminal']=True
 report['source_after']=repository_source_identity(root)
 (out/'build.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
