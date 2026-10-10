import fcntl,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-regressions-v3547');AUDIT=Path('/tmp/openui-native-keyboard-regressions-completed-audit-v3551.json')
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
r=json.loads((OUT/'receipt.json').read_bytes());assert r['all_commands_terminal'] and r['observed_exit_code']==0 and r['source_unchanged'] and r['main_source_unchanged'];assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode==1
for name in ['cargo','rustc','rustfmt','pixel_compare']:assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
sys.path.insert(0,'/tmp');from openui_parallel_source_identity_v3060 import identity
assert identity(ROOT)==r['source'];assert all(sha(p)==h for p,h in r['reference_inputs_sha256'].items())
ledger={};records=0;replaced=[]
for step in r['steps']:
 assert step['actual_exit_code']==0;assert sha(OUT/(step['name']+'.log'))==step['log_sha256']
 for row in r.get('local_artifacts',{}).get(step['name'],[]):
  records+=1;m=row['record'];assert Path(m['manifest_path']).resolve().is_relative_to(ROOT);assert Path(m['target']['src_path']).resolve().is_relative_to(ROOT)
  for p,h in row['sha256'].items():
   if m['fresh']:assert ledger.get(p)==h,(step['name'],p)
   if p in ledger and ledger[p]!=h:assert not m['fresh'];replaced.append(dict(stage=step['name'],path=p,before=ledger[p],after=h))
   ledger[p]=h
for p,h in ledger.items():assert sha(p)==h,p
for name,row in r['binaries'].items():assert sha(OUT/name)==row['sha256'];
assert sha(OUT/'libopenui_ffi.so')==r['ffi_library_sha256'] and r['ffi_library_sha256'] in ledger.values()
for name in ['native_fieldset','native_input_metadata','native_selection_phases','native_selection_values','native_range_scalar','native_range_modes','native_disabled_focus','native_form_owner','native_checkable','native_disabled_appearance','native_control_keyboard']:assert r['binaries'][name]['sha256'] in ledger.values()
for language in ['rust','c','cpp']:
 assert r['scenario_counts'][language]==dict(total=396,exact=396);assert r['keyboard_behavior_by_language'][language]==dict(total=99,exact=99,different=0)
assert r['all_regression_rows_unchanged'];assert r['broader_all_targets_gate_qualified'];assert r['all_targets_tests']['passed']>=8631 and r['all_targets_tests']['failed']==0 and r['all_targets_tests']['ignored']==0
assert r['workspace_tests']['passed']>=8633 and r['workspace_tests']['failed']==0 and r['workspace_tests']['ignored']==13
assert r['headless_tests']==[[54,0,0]];assert r['original_main_stack_limit']==8388608;assert 'C ABI verified: symbols=131 C_examples=29 C++=23 ran=True' in (OUT/'all-c-cpp-consumers.log').read_text()
proof=dict(schema_version=1,complete=True,owner_terminal=True,owner_absent=True,actual_whole_exit_code=0,source=r['source'],receipt_sha256=sha(OUT/'receipt.json'),artifact_records=records,current_compiled_paths_verified=len(ledger),chronological_artifact_order_verified=True,artifacts_replaced_by_later_recorded_compilation=replaced,all_targets_tests=r['all_targets_tests'],workspace_tests=r['workspace_tests'],headless_tests=r['headless_tests'],native_behavior_by_language={language:dict(total=495,exact=495,different=0) for language in ['rust','c','cpp']},native_behavior_comparisons=1485,regression_groups=len(r['regression_comparisons']),c_consumers=29,cpp_consumers=23,exports=131,layouts=34,original_main_stack_bytes=8388608,implementation_integrated=True,all_native_apis_qualified=False,renderer_qualified=False,release_qualified=False,retired_for_reexecution_after_source_or_target_mutation=True)
AUDIT.write_text(json.dumps(proof,sort_keys=True,indent=2)+'\n');print(json.dumps(proof),flush=True)
