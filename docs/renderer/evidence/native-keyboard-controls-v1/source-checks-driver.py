import concurrent.futures,fcntl,gzip,hashlib,json,os,subprocess,sys
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-source-checks-v3533');DEST=ROOT/'docs/renderer/evidence/native-keyboard-controls-v1'
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
summary=json.loads((ROOT/'docs/renderer/generated/native-keyboard-controls-v1.json').read_bytes());assert summary['implementation_integrated'] and not summary['renderer_commands_executed']
for name,row in summary['files'].items():
 p=DEST/name;assert sha(p)==row['sha256'];data=gzip.decompress(p.read_bytes()) if name.endswith('.gz') else p.read_bytes();assert hashlib.sha256(data).hexdigest()==row['uncompressed_sha256'];assert data==Path(row['origin']).read_bytes()
for language in ['rust','c','cpp']:
 first=gzip.decompress((DEST/('observations-'+language+'-1.json.gz')).read_bytes());assert first==gzip.decompress((DEST/('observations-'+language+'-2.json.gz')).read_bytes());rows=json.loads((DEST/('comparisons-'+language+'.json')).read_bytes());assert len(rows)==99 and all(row['exact'] and not row['differences'] for row in rows)
assert not subprocess.check_output(['git','diff','635619da2000ef5db258f97623689cd474084fdb','--','tools/qualification/manifests/complete-5731.json'],cwd=ROOT)
assert not subprocess.check_output(['git','diff','635619da2000ef5db258f97623689cd474084fdb','--','bindings/rust/Cargo.lock'],cwd=ROOT)
old=set(subprocess.check_output(['git','show','635619da2000ef5db258f97623689cd474084fdb:docs/v02/generated/openui-ffi-symbols.txt'],cwd=ROOT,text=True).splitlines());new=set((ROOT/'docs/v02/generated/openui-ffi-symbols.txt').read_text().splitlines());assert len(old)==130 and len(new)==131 and new-old=={'oui_element_is_active_v1'} and old<=new
assert subprocess.check_output(['git','show','635619da2000ef5db258f97623689cd474084fdb:docs/v02/generated/openui-ffi-layout.json'],cwd=ROOT)==(ROOT/'docs/v02/generated/openui-ffi-layout.json').read_bytes()
assert not OUT.exists();OUT.mkdir()
formatter='/tmp/openui-clang18-ci-v2621/unpacked/clang_format/data/bin/clang-format';env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1',OPENUI_CLANG_FORMAT=formatter,CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target')
commands=[('generator-'+Path(p).stem,[sys.executable,p,'--check'],ROOT) for p in ['tools/qualification/generate_renderer_contract.py','tools/wpt/generate_sp13r_multicol_closure.py','tools/wpt/generate_sp17_closure.py','tools/wpt/generate_sp18_closure.py','tools/wpt/generate_sp20_closure.py','tools/release/generate_v02_contract.py','tools/style/generate_properties.py','tools/ffi/generate_ffi.py','tools/ffi/generate_form_owner_cases.py','tools/ffi/generate_control_keyboard_cases.py']]
commands += [('historical-archive-integrity',[sys.executable,'tools/accountability/restore_frozen_openui_archive.py','--check'],ROOT),('release-source',[sys.executable,'tools/release/build_v02_linux.py','--verify-source'],ROOT),('conformance',[sys.executable,'tools/conformance/verify_v02.py'],ROOT),('performance-contract',[sys.executable,'tools/performance/verify_v02.py'],ROOT),('accountability',[sys.executable,'tools/accountability/audit.py','--repository-only'],ROOT),('rust-format',['bash','-c','ulimit -s 262144; cargo fmt --all -- --check'],ROOT/'bindings/rust'),('c-cpp-format',[formatter,'--dry-run','--Werror',*[str(ROOT/'examples/c_v02'/p) for p in ['control_keyboard.c','control_keyboard.cc','control_keyboard_guards.c','control_keyboard_guards.cc','control_keyboard_cases.h']]],ROOT),('diff-check',['git','diff','--check'],ROOT)]
report=dict(schema_version=1,owner_pid=os.getpid(),all_commands_terminal=False,steps=[],archived_files_verified=len(summary['files']),complete_manifest_unchanged=True,lockfile_unchanged=True,existing_exports_preserved=130,exports=131,layouts_unchanged=34,read_only_source_checks=True,renderer_qualified=False,all_native_apis_qualified=False,release_qualified=False)
def save():(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
def run(item):
 name,command,cwd=item
 with (OUT/(name+'.log')).open('xb') as log:r=subprocess.run(command,cwd=cwd,env=env,stdout=log,stderr=subprocess.STDOUT)
 return dict(name=name,command=command,actual_exit_code=r.returncode,log_sha256=sha(OUT/(name+'.log')))
save()
with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
 for step in pool.map(run,commands):report['steps'].append(step);save();print(json.dumps(step),flush=True)
report['all_commands_terminal']=True;report['actual_exit_code']=int(any(step['actual_exit_code']!=0 for step in report['steps']));save();print(json.dumps(dict(actual_exit_code=report['actual_exit_code'],checks=len(commands),evidence_files=report['archived_files_verified'])),flush=True)
raise SystemExit(report['actual_exit_code'])
