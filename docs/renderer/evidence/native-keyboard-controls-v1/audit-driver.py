import fcntl,hashlib,json,subprocess,sys
from pathlib import Path
OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-c-qualification-v3528');SDK=Path('/mnt/e/openui-v02-qualification-d174ea0b/private-native-keyboard-source-v3505');MAIN=Path('/home/nero/code/open-ui')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
r=json.loads((OUT/'receipt.json').read_bytes());assert r['all_commands_terminal'] and r['observed_exit_code']==0 and r['source_unchanged'];assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode==1
for name in ['cargo','rustc','rustfmt','pixel_compare']:assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
sys.path.insert(0,'/tmp');from openui_parallel_source_identity_v3060 import identity
assert identity(SDK)==r['source'] and identity(MAIN)==r['main_source'];assert all(sha(p)==h for p,h in r['reference_inputs'].items())
paths={};records=0
for step in r['steps']:
 stage=step['name'];artifacts=r['local_artifacts'].get(stage,[])
 for row in artifacts:
  records+=1;m=row['record'];assert Path(m['manifest_path']).resolve().is_relative_to(SDK.resolve());assert Path(m['target']['src_path']).resolve().is_relative_to(SDK.resolve())
  for p,h in row['sha256'].items():
   if m['fresh']:assert paths.get(p)==h
   assert sha(p)==h;paths[p]=h
for step in r['steps']:assert step['actual_exit_code']==0;assert sha(OUT/(step['name']+'.log'))==step['log_sha256']
assert sha(OUT/'native_control_keyboard')==r['rust_binary_sha256'] and r['rust_binary_sha256'] in paths.values()
assert sha(OUT/'libopenui.so.0')==r['library_sha256'] and r['library_sha256'] in paths.values()
for name,record in r['consumers'].items():assert sha(OUT/name)==record['binary_sha256'];assert sha(OUT/(name+'.o'))==record['object_sha256']
for language in ['rust','c','cpp']:
 assert (OUT/('observations-'+language+'-1.json')).read_bytes()==(OUT/('observations-'+language+'-2.json')).read_bytes();assert r['behavior_results'][language]==dict(total=99,exact=99,different=0)
assert r['behavior_exact'];symbols=(SDK/'docs/v02/generated/openui-ffi-symbols.txt').read_text().splitlines();assert len(symbols)==131
layout=json.loads((SDK/'docs/v02/generated/openui-ffi-layout.json').read_bytes());assert len(layout['types'])==34
proof=dict(schema_version=1,complete=True,owner_terminal=True,owner_absent=True,actual_whole_exit_code=0,source=r['source'],main_source=r['main_source'],receipt_sha256=sha(OUT/'receipt.json'),artifact_records=records,compiled_paths=len(paths),behavior_results=r['behavior_results'],behavior_exact=True,exports=131,layouts=34,private_implementation=True,cpp_c_behavior_qualification=True,all_native_apis_qualified=False,renderer_qualified=False,release_qualified=False,retired_for_reexecution_after_next_source_or_target_epoch_change=True)
Path('/tmp/openui-native-keyboard-c-completed-audit-v3529.json').write_text(json.dumps(proof,sort_keys=True,indent=2)+'\n')
print(json.dumps(proof))
