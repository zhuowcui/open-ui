"""Reuse completed exact-source named guards; no Cargo command."""
import hashlib,json,os,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-keywords-retry-7d6ffabf-v1779')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-keywords-current-retry-guards-v1779'
STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
p=RAW/'native-keywords-current-guards-v1772/receipt.json';assert sha(p)=='aac2e9336f9e616ed701e65be88841f7ecce1cffc0760f0461682250ca9798cb'
d=json.loads(p.read_bytes());source=repository_source_identity(ROOT)
assert source['clean'] and source['commit']=='7d6ffabfea1f72c90f97475488dba8a8058ac4c7' and source==d['source']==d['source_after']
assert d['all_commands_terminal'] and d['observed_exit_code']==0 and d['baseline_regression_reproduced']
assert [r['actual_exit'] for r in d['steps']]==[0,101,0,0,0,0] and not any(r['disk_guard_triggered'] for r in d['steps'])
for row in d['steps']:assert sha(p.parent/(row['name']+'.log'))==row['log_sha256']
assert all(r['passed']==1 and r['failed']==0 for r in d['steps'][3:])
report=dict(schema_version=1,source=source,source_after=source,baseline_source=d['baseline_source'],all_commands_terminal=True,observed_exit_code=0,guard_receipt=str(p),guard_receipt_sha256=sha(p),baseline_actual_exit=101,all_three_fixed_named_guards_passed=True,actual_guard_stage_exits=[0,101,0,0,0,0],completed_same_source_guards_reused_without_rerun=True,cargo_commands_run=0,screenshots_generated=0,javascript_executed_by_openui=False,release_qualification=False,new_release_states_admitted=0,probe_sha256=sha(Path(__file__)))
q=OUT/'receipt.json';q.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'baseline_actual_exit':101,'fixed_named_guards_passed':3,'same_source_guards_reused':True}),flush=True)
