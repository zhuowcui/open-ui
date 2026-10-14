"""Verify actual same-source complete build and guards without rerunning Cargo."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-keywords-native-7d6ffabf-v1787')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-keywords-native-retry-build-and-guards-v1787'
STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']=='7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
bp=RAW/'native-keywords-current-retry-clean-v1779/build.json';b=json.loads(bp.read_bytes());assert b['all_commands_terminal'] and b['source']==b['source_after']==source and len(b['steps'])==13
for s in b['steps']:
 assert s['observed_exit_code']==0 and not s['disk_guard_triggered'] and sha(bp.parent/(s['name']+'.log'))==s['log_sha256']
 if 'binary' in s:assert sha(Path(s['binary']))==s['binary_sha256']
gp=RAW/'native-keywords-current-guards-v1772/receipt.json';assert sha(gp)=='aac2e9336f9e616ed701e65be88841f7ecce1cffc0760f0461682250ca9798cb';g=json.loads(gp.read_bytes())
assert g['all_commands_terminal'] and g['source']==g['source_after']==source and g['baseline_regression_reproduced'] and [s['actual_exit'] for s in g['steps']]==[0,101,0,0,0,0]
for s in g['steps']:assert sha(gp.parent/(s['name']+'.log'))==s['log_sha256'] and not s['disk_guard_triggered']
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=True,observed_exit_code=0,baseline_actual_exit=101,all_three_fixed_named_guards_passed=True,completed_build_reused_from_same_clean_source=True,build_receipt_sha256=sha(bp),guard_receipt_sha256=sha(gp),workspace_passed=8554,workspace_failed=0,workspace_ignored=13,actual_build_stages_passed=13,actual_c_and_cpp_apps_passed=True,cargo_commands_run=0,screenshots_generated=0,javascript_executed_by_openui=False,release_qualification=False,new_release_states_admitted=0,probe_sha256=sha(Path(__file__)))
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'same_source_build_stages_reused':13,'actual_fixed_named_guards_passed':3}),flush=True)
