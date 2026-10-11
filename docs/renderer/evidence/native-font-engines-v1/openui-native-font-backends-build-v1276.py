import hashlib, json, os, re, shutil, signal, subprocess, sys, time
from pathlib import Path
root=Path('/dev/shm/openui-native-font-backends-107e2e36');raw=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
out=raw/'native-font-backends-clean-v1276';storage=Path('/mnt/e/openui-v02-qualification-d174ea0b')/out.name;assert not out.exists();storage.mkdir();out.symlink_to(storage,target_is_directory=True)
sys.path.insert(0,str(root/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(root);assert source['clean'] and source['commit'].startswith('0601cd30')
env=dict(os.environ,CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',CARGO_INCREMENTAL='0',RUST_MIN_STACK='4194304',CARGO_BUILD_JOBS='4',PYTHONDONTWRITEBYTECODE='1',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0')
base=['cargo','--config','.cargo/config.chromium.toml']
metadata=json.loads(subprocess.check_output(base+['metadata','--offline','--locked','--no-deps','--format-version','1'],cwd=root/'bindings/rust',env=env,text=True))
members=set(metadata['workspace_members']);packages=[p['name'] for p in metadata['packages'] if p['id'] in members];assert len(packages)==18
steps=[
 ('clean-runner',base+['clean','-p','pixel-compare'],None),
 ('workspace',base+['test','--workspace','--locked','--features','openui-ffi/linux'],None),
 ('intrinsic-build',base+['build','--locked','-p','openui','--example','native_intrinsic_sizes'],'debug/examples/native_intrinsic_sizes'),
 ('font-build',base+['build','--locked','-p','openui','--example','native_font_raster'],'debug/examples/native_font_raster'),
 ('static-build',base+['build','--locked','-p','openui','--example','native_static_position'],'debug/examples/native_static_position'),
 ('ffi-build',base+['build','--locked','-p','openui-ffi'],'debug/libopenui_ffi.so'),
 ('pixel-build',base+['build','--locked','-p','pixel-compare','--bin','pixel_compare'],'debug/pixel_compare'),
 ('ffi-consumers',['python3',str(root/'tools/ffi/verify_abi.py'),'--library',str(out/'libopenui_ffi.so')],None),
]
report=dict(schema_version=1,release_qualification=False,source=source,steps=[],cargo_target_dir=env['CARGO_TARGET_DIR'],probe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
for name,cmd,artifact in steps:
 log=out/(name+'.log')
 tracked=root/'tests/pixel_text/openui_renders/basic_text_openui.png';backup=tracked.read_bytes() if name=='workspace' and tracked.exists() else None
 with log.open('xb') as stream:
  p=subprocess.Popen(cmd,cwd=root/'bindings/rust',env=env,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True);disk_guard=False
  while p.poll() is None:
   if shutil.disk_usage(root).free<512*2**20 or shutil.disk_usage(Path(env['CARGO_TARGET_DIR'])).free<10*2**30:
    disk_guard=True;os.killpg(p.pid,signal.SIGTERM);p.wait(timeout=20);break
   time.sleep(1)
  p.wait()
 if backup is not None and tracked.read_bytes()!=backup:
  (out/'generated-diagnostic-openui.png').write_bytes(tracked.read_bytes());tracked.write_bytes(backup)
 entry=dict(name=name,command=cmd,observed_exit_code=p.returncode,disk_guard_triggered=disk_guard,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest());report['steps'].append(entry);report['source_after']=repository_source_identity(root);assert source==report['source_after']
 if name=='workspace':
  totals=[tuple(map(int,m)) for m in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',log.read_bytes())];entry.update(passed=sum(t[0] for t in totals),failed=sum(t[1] for t in totals),ignored=sum(t[2] for t in totals))
 if p.returncode==0 and artifact:
  path=out/Path(artifact).name;shutil.copy2(Path(env['CARGO_TARGET_DIR'])/artifact,path);entry['binary_sha256']=hashlib.sha256(path.read_bytes()).hexdigest();entry['binary']=str(path)
  if name=='pixel-build':
   ident=json.loads(subprocess.check_output([str(path),'build-source-identity'],cwd=root,env=env,text=True));assert ident['source']==source;report['build_identity']=ident
 (out/'build.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(entry),flush=True)
 if p.returncode:raise SystemExit(p.returncode)
