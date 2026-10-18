"""Freeze corrected fresh captures using a complete clean same-source build."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-keywords-pixels-7d6ffabf-v1785')
COMMIT='7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
prior=RAW/'native-keywords-current-retry-pipeline-v1780/receipt.json';assert sha(prior)=='3d24233f9c05f8d156e837402918d71d8944e3e64a727f243ee6b8f76b987771'
p=json.loads(prior.read_bytes());assert p['all_commands_terminal'] and [s['observed_exit_code'] for s in p['steps']]==[0,0,1]
build_path=RAW/'native-keywords-current-retry-clean-v1779/build.json';build=json.loads(build_path.read_bytes())
assert build['all_commands_terminal'] and len(build['steps'])==13 and all(s['observed_exit_code']==0 and not s['disk_guard_triggered'] for s in build['steps'])
assert build['steps'][1]['passed']==8554 and build['steps'][1]['failed']==0 and build['steps'][1]['ignored']==13
log=RAW/'native-keywords-current-retry-pipeline-v1780/native-application.log';assert b'ModuleNotFoundError' in log.read_bytes()
assert not (RAW/'native-keywords-current-retry-consumer-v1779/receipt.json').exists()
finding=RAW/'native-keywords-capture-path-failure-v1785.json';assert not finding.exists()
missing='/dev/shm/openui-native-keywords-current-retry-current-0733955a';assert not Path(missing).exists()
f=dict(schema_version=1,all_commands_terminal=True,actual_native_application_stage_exit=1,actual_native_raster_commands_run=0,chromium_capture_commands_run=0,screenshots_generated=0,failed_before_source_identity_import=True,reason='A namespace replacement rewrote the already substituted source root in the capture probes.',bad_source_root=missing,previous_owner_sha256=sha(prior),failure_log_sha256=sha(log),build_complete_at_declared_source=True,workspace_passed=8554,actual_c_and_cpp_native_apps_passed=True,source=build['source'],release_qualification=False,new_release_states_admitted=0)
finding.write_text(json.dumps(f,sort_keys=True,indent=2)+'\n')
assert not ROOT.exists();subprocess.run(['git','worktree','add','--detach',str(ROOT),COMMIT],cwd=MAIN,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
sys.path.insert(0,str(MAIN/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==COMMIT and source==build['source']==build['source_after']
files={};parents={}
for kind in ['consumer','table-geometry','matrices']:
 original=Path('/tmp/openui-native-keywords-current-retry-'+kind+'-v1779.py');parents[str(original)]=sha(original)
 t=original.read_text().replace('native-keywords-current-retry-','native-keywords-capture-').replace('v1779','v1785')
 # The completed build stays under its original, immutable output path.
 t=t.replace('native-keywords-capture-clean-v1785','native-keywords-current-retry-clean-v1779')
 # Replace the source assignment itself, after output namespace changes.
 m=ast.parse(t);assign=next(n for n in m.body if isinstance(n,ast.Assign) and any(isinstance(x,ast.Name) and x.id=='ROOT' for x in n.targets))
 old=ast.get_source_segment(t,assign);new='ROOT = Path('+repr(str(ROOT))+')';assert t.count(old)==1;t=t.replace(old,new)
 files[Path('/tmp/openui-native-keywords-capture-'+kind+'-v1785.py')]=t
reuse='''"""Verify actual same-source complete build and guards without rerunning Cargo."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-keywords-pixels-7d6ffabf-v1785')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-keywords-capture-build-and-guards-v1785'
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
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\\n');print(json.dumps({'same_source_build_stages_reused':13,'actual_fixed_named_guards_passed':3}),flush=True)
'''
files[Path('/tmp/openui-native-keywords-capture-build-reuse-v1785.py')]=reuse
original=Path('/tmp/openui-native-keywords-current-retry-pipeline-v1780.py');parents[str(original)]=sha(original);t=original.read_text();config=ast.literal_eval(ast.parse(t).body[0].value)
config.update(root=str(ROOT),name='native-keywords-capture-pipeline-v1786',initial_state='awaiting-terminal-source-build-owner',prior_pipelines=['native-keywords-current-retry-pipeline-v1780'],scripts=[p.name for p in files]+['openui-native-image-coverage-fieldsets-v1448.py'],hard_stop_stages=['build-and-guards'])
config['selections'] += ['native-keywords-current-retry-pipeline-v1780/receipt.json','native-keywords-current-retry-clean-v1779/build.json','native-keywords-capture-path-failure-v1785.json']
config['stages']=[('build-and-guards',['/usr/bin/python3','/tmp/openui-native-keywords-capture-build-reuse-v1785.py'],'native-keywords-capture-build-and-guards-v1785/receipt.json',True),('native-application',['/usr/bin/python3','/tmp/openui-native-keywords-capture-consumer-v1785.py'],'native-keywords-capture-consumer-v1785/receipt.json',True),('native-table-geometry-diagnostic',['/usr/bin/python3','/tmp/openui-native-keywords-capture-table-geometry-v1785.py'],'native-keywords-capture-table-geometry-v1785/receipt.json',True)]+[(s,['/usr/bin/python3','/tmp/openui-native-keywords-capture-matrices-v1785.py',s],f'native-keywords-capture-clean-{s}-v1785/{s}-summary.json',False) for s in ['focused','primitive','full','expanded']]
body=t.split('\n',1)[1].replace('expected_stages=8','expected_stages=7')
body=body.replace('fresh_retry_preserves_original_failure=True','fresh_retry_preserves_original_failure=True, capture_path_correction_only=True, completed_same_source_build_reused=True')
files[Path('/tmp/openui-native-keywords-capture-pipeline-v1786.py')]='CONFIG = '+repr(config)+'\n'+body
for path,text in files.items():
 assert not path.exists();module=ast.parse(text)
 if path.name.endswith('pipeline-v1786.py'):continue
 a=next(n for n in module.body if isinstance(n,ast.Assign) and any(isinstance(x,ast.Name) and x.id=='ROOT' for x in n.targets))
 assert isinstance(a.value,ast.Call) and ast.literal_eval(a.value.args[0])==str(ROOT)
 assert (ROOT/'tools/qualification/renderer_source_identity.py').is_file()
 assert missing not in text and '/dev/shm/openui-native-keywords-capture-current-0733955a' not in text
for p,t in files.items():p.write_text(t)
assert repository_source_identity(ROOT)==source
p=RAW/'native-keywords-capture-prepared-v1785.json';assert not p.exists()
report=dict(schema_version=1,source=source,source_after=source,original_path_failure_sha256=sha(finding),completed_clean_build_receipt_sha256=sha(build_path),same_source_build_reused=True,fresh_capture_root=str(ROOT),all_worker_source_root_assignments_ast_verified=True,all_required_modules_exist_at_declared_root=True,required_stages=7,all_four_pixel_matrices_required=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,javascript_executed_by_openui=False,cargo_commands_run=0,screenshots_generated=0,scripts={str(p):sha(p) for p in files},adapted_from=parents,probe_sha256=sha(Path(__file__)))
p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'receipt':str(p),'sha256':sha(p),'source':COMMIT,'worker_source_roots_verified':True,'stages':7}),flush=True)
