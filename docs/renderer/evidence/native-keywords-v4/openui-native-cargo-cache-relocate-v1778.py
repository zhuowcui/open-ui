"""Relocate the workspace build cache without deleting or changing its bytes."""
import fcntl,hashlib,json,shutil,subprocess
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
SOURCE=MAIN/'bindings/rust/target';DEST=Path('/mnt/e/openui-v02-home-cargo-cache-v1778')
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
for n in ['cargo','rustc','pixel_compare']:assert subprocess.run(['pgrep','-x',n],capture_output=True).returncode==1
assert SOURCE.is_dir() and not SOURCE.is_symlink() and not DEST.exists()
files={}
for p in SOURCE.rglob('*'):
 assert not p.is_symlink()
 if p.is_file():files[str(p.relative_to(SOURCE))]=dict(sha256=sha(p),bytes=p.stat().st_size)
assert len(files)==5327 and sum(r['bytes'] for r in files.values())==2747111217
free_before=shutil.disk_usage(MAIN).free
shutil.move(str(SOURCE),str(DEST));SOURCE.symlink_to(DEST,target_is_directory=True)
assert SOURCE.resolve()==DEST
for name,row in files.items():assert sha(SOURCE/name)==row['sha256'] and (SOURCE/name).stat().st_size==row['bytes']
p=RAW/'native-cargo-cache-relocation-v1778.json';assert not p.exists()
report=dict(schema_version=1,all_commands_terminal=True,observed_exit_code=0,source=str(SOURCE),destination=str(DEST),files=files,file_count=len(files),total_bytes=sum(r['bytes'] for r in files.values()),all_original_logical_paths_preserved=True,all_file_sha256_and_lengths_unchanged=True,no_build_cache_files_deleted=True,no_frozen_source_reference_or_qualification_artifact_edited=True,free_bytes_before=free_before,free_bytes_after=shutil.disk_usage(MAIN).free,release_qualification=False,probe_sha256=sha(Path(__file__)))
p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'receipt':str(p),'sha256':sha(p),'files':len(files),'bytes':report['total_bytes'],'free_gib':round(report['free_bytes_after']/2**30,2)}),flush=True)
