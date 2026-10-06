"""Freeze fresh source and probes after the observed ABI scratch-space failure."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-keywords-retry-7d6ffabf-v1779')
SOURCE='7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
TEMP=Path('/mnt/e/openui-v02-qualification-d174ea0b/native-keywords-current-temporary-v1779')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
prior=RAW/'native-keywords-current-pipeline-v1773/receipt.json';assert sha(prior)=='189711442797ae28149d346c7caa1b75dc768e81066bdab258f1167b79a12ba7'
p=json.loads(prior.read_bytes());assert p['all_commands_terminal'] and [r['observed_exit_code'] for r in p['steps']]==[0,241]
cache=RAW/'native-cargo-cache-relocation-v1778.json';c=json.loads(cache.read_bytes());assert c['all_commands_terminal'] and c['observed_exit_code']==0 and c['all_file_sha256_and_lengths_unchanged']
assert not ROOT.exists() and not TEMP.exists()
subprocess.run(['git','worktree','add','--detach',str(ROOT),SOURCE],cwd=MAIN,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);TEMP.mkdir()
sys.path.insert(0,str(MAIN/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==SOURCE
assert source==p['source']==p['source_after']
files={};adapted_from={}
reuse='''"""Reuse completed exact-source named guards; no Cargo command."""
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
q=OUT/'receipt.json';q.write_text(json.dumps(report,sort_keys=True,indent=2)+'\\n')
print(json.dumps({'baseline_actual_exit':101,'fixed_named_guards_passed':3,'same_source_guards_reused':True}),flush=True)
'''
files[Path('/tmp/openui-native-keywords-current-retry-guards-v1779.py')]=reuse
for kind in ['build','consumer','table-geometry','matrices']:
 original=Path('/tmp/openui-native-keywords-current-'+kind+'-v1772.py');adapted_from[str(original)]=sha(original)
 t=original.read_text().replace('/dev/shm/openui-native-keywords-current-0733955a',str(ROOT)).replace('native-keywords-current-','native-keywords-current-retry-').replace('v1772','v1779').replace('pipeline-v1773','pipeline-v1780')
 if kind=='build':
  t=t.replace("PYTHONDONTWRITEBYTECODE='1',CARGO_PROFILE_DEV_DEBUG='0'",f"PYTHONDONTWRITEBYTECODE='1',TMPDIR={str(TEMP)!r},CARGO_PROFILE_DEV_DEBUG='0'")
  assert 'TMPDIR=' in t
 files[Path('/tmp/openui-native-keywords-current-retry-'+kind+'-v1779.py')]=t
original=Path('/tmp/openui-native-keywords-current-pipeline-v1773.py');adapted_from[str(original)]=sha(original)
t=original.read_text();config=ast.literal_eval(ast.parse(t).body[0].value)
config.update(root=str(ROOT),name='native-keywords-current-retry-pipeline-v1780',initial_state='awaiting-terminal-original-owner',prior_pipelines=['native-keywords-current-pipeline-v1773'],scripts=[p.name for p in files]+['openui-native-image-coverage-fieldsets-v1448.py'])
config['selections'] += ['native-keywords-current-pipeline-v1773/receipt.json','native-keywords-current-clean-v1772/build.json','native-keywords-current-guards-v1772/receipt.json','native-keywords-abi-scratch-relocation-v1777.json','native-cargo-cache-relocation-v1778.json']
config['stages']=[(name,[part.replace('native-keywords-current-','native-keywords-current-retry-').replace('v1772','v1779') for part in command],terminal.replace('native-keywords-current-','native-keywords-current-retry-').replace('v1772','v1779'),flag) for name,command,terminal,flag in config['stages']]
body=t.split('\n',1)[1]
body=body.replace("OPENUI_NATIVE_WHOLE_OWNER=CONFIG['name']",f"OPENUI_NATIVE_WHOLE_OWNER=CONFIG['name'], TMPDIR={str(TEMP)!r}")
body=body.replace("report.update(owner_pid=os.getpid(),",f"report.update(temporary_files_on_data_volume=True, fresh_retry_preserves_original_failure=True, original_owner_sha256={sha(prior)!r}, owner_pid=os.getpid(),")
files[Path('/tmp/openui-native-keywords-current-retry-pipeline-v1780.py')]='CONFIG = '+repr(config)+'\n'+body
for path,text in files.items():assert not path.exists();ast.parse(text)
for path,text in files.items():path.write_text(text)
assert repository_source_identity(ROOT)==source
p=RAW/'native-keywords-current-retry-prepared-v1779.json';assert not p.exists()
report=dict(schema_version=1,source=source,original_source_same=True,original_owner_sha256=sha(prior),cache_relocation_sha256=sha(cache),fresh_root=str(ROOT),temporary_directory=str(TEMP),temporary_files_on_data_volume=True,baseline_and_three_fixed_guards_reused_from_actual_complete_same_source_receipt=True,required_stages=8,required_build_stages=13,all_four_pixel_matrices_required=True,native_table_geometry_diagnostic_only=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,javascript_executed_by_openui=False,cargo_commands_run=0,screenshots_generated=0,scripts={str(p):sha(p) for p in files},adapted_from=adapted_from,probe_sha256=sha(Path(__file__)))
p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'receipt':str(p),'sha256':sha(p),'source':SOURCE,'stages':8,'temporary_directory_on_data_volume':True}),flush=True)
