"""Move the stopped ABI checker's owned scratch, preserving every file byte."""
import fcntl,hashlib,json,os,shutil,subprocess
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
OWNER=RAW/'native-keywords-current-pipeline-v1773/receipt.json';BUILD=RAW/'native-keywords-current-clean-v1772/build.json'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
owner=json.loads(OWNER.read_bytes());build=json.loads(BUILD.read_bytes())
assert owner['all_commands_terminal'] and build['all_commands_terminal']
assert [(r['name'],r['observed_exit_code']) for r in owner['steps']]==[('guards',0),('native-build',241)]
assert build['steps'][-1]['name']=='ffi-consumers' and build['steps'][-1]['observed_exit_code']==-15 and build['steps'][-1]['disk_guard_triggered']
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
for n in ['cargo','rustc','pixel_compare']:assert subprocess.run(['pgrep','-x',n],capture_output=True).returncode==1
source=Path('/tmp/openui-ffi-_1lng7e8');destination=Path('/mnt/e/openui-v02-qualification-d174ea0b/native-keywords-abi-owned-scratch-v1777')
assert source.is_dir() and not destination.exists()
ffi=next(r for r in build['steps'] if r['name']=='ffi-build')
assert sha(source/'libopenui.so.0')==ffi['binary_sha256']
files={str(p.relative_to(source)):dict(sha256=sha(p),bytes=p.stat().st_size) for p in source.rglob('*') if p.is_file()}
assert files and all(not p.is_symlink() for p in source.rglob('*'))
free_before=shutil.disk_usage(MAIN).free
shutil.move(str(source),str(destination))
assert not source.exists()
for name,row in files.items():assert sha(destination/name)==row['sha256'] and (destination/name).stat().st_size==row['bytes']
p=RAW/'native-keywords-abi-scratch-relocation-v1777.json';assert not p.exists()
report=dict(schema_version=1,all_commands_terminal=True,observed_exit_code=0,owner_receipt_sha256=sha(OWNER),build_receipt_sha256=sha(BUILD),source=str(source),destination=str(destination),files=files,file_count=len(files),all_bytes_preserved=True,owned_scratch_library_matches_source_7d6ffabf=True,free_bytes_before=free_before,free_bytes_after=shutil.disk_usage(MAIN).free,no_qualification_reference_changed=True,release_qualification=False,probe_sha256=sha(Path(__file__)))
p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'receipt':str(p),'sha256':sha(p),'files':len(files),'moved_bytes':sum(r['bytes'] for r in files.values()),'free_gib':round(report['free_bytes_after']/2**30,3)}),flush=True)
