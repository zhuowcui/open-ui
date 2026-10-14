"""Freeze canonical-table verification behind every preceding whole owner."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-table-source-1d846e68')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
FIXED='e389b26ab28f259544e43c3052c895501e4c8686';BASELINE='cd6d80be4be16151a607cddcbad94e2f73a79e62';BRANCH='agent/native-table-source-v1597'
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==FIXED
checks_path=RAW/'native-table-source-checks-v1598/receipt.json';checks=json.loads(checks_path.read_bytes())
assert checks['all_commands_terminal'] and checks['source']==checks['source_after']==source
assert len(checks['checks'])==11 and all(r['observed_exit_code']==0 for r in checks['checks'])
old_path=Path('/tmp/openui-native-table-progress-pipeline-v1548.py');old_text=old_path.read_text();old=ast.literal_eval(ast.parse(old_text).body[0].value)
prior_path=Path('/tmp/openui-native-keywords-pipeline-v1588.py');prior=ast.literal_eval(ast.parse(prior_path.read_text()).body[0].value)
priors=prior['prior_pipelines']+[prior['name']];assert len(priors)==len(set(priors))==31
geometry_path=RAW/'native-table-progress-geometry-v1545/receipt.json';geometry=json.loads(geometry_path.read_bytes())
absolute_path=RAW/'native-table-progress-absolute-geometry-v1562/receipt.json';absolute=json.loads(absolute_path.read_bytes())
for data in [geometry,absolute]:
 assert data['all_commands_terminal'] and data['observed_exit_code']==0 and data['independent_queries_identical']
 assert sum(len(r['rows']) for r in data['runs'])==96
reference=Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py')
files={};original_hashes={str(old_path):sha(old_path),str(prior_path):sha(prior_path)}
for kind in ['guards','build','consumer','matrices']:
 path=Path('/tmp/openui-native-table-progress-'+kind+'-v1547.py');original_hashes[str(path)]=sha(path)
 text=path.read_text()
 for before,after in [('/dev/shm/openui-native-table-progress-b7e28e56',str(ROOT)),('4b3cb72c03c58416a736a207b97cdcc8a225cb5a',FIXED),('4318f606d9ed9c204ea9ee3edacbfee9b0fd5479',BASELINE),('agent/native-table-progress-v1543',BRANCH),('native-table-progress-','native-table-source-'),('v1547','v1600'),(repr(old['prior_pipelines']),repr(priors))]:text=text.replace(before,after)
 # Earlier immutable Chromium queries retain their original provenance paths.
 text=text.replace('native-table-source-geometry-v1545','native-table-progress-geometry-v1545')
 files[Path('/tmp/openui-native-table-source-'+kind+'-v1600.py')]=text
config=dict(old,root=str(ROOT),commit=FIXED,name='native-table-source-pipeline-v1601',initial_state='awaiting-all-31-prior-whole-pipelines',prior_pipelines=priors,selections=['native-table-source-checks-v1598/receipt.json','native-table-progress-geometry-v1545/receipt.json','native-table-progress-absolute-geometry-v1562/receipt.json'])
config['scripts']=[p.name for p in files]+[reference.name]
config['stages']=[('guards',['/usr/bin/python3','/tmp/openui-native-table-source-guards-v1600.py'],'native-table-source-guards-v1600/receipt.json',True),('native-build',['/usr/bin/python3','/tmp/openui-native-table-source-build-v1600.py'],'native-table-source-clean-v1600/build.json',True),('native-application',['/usr/bin/python3','/tmp/openui-native-table-source-consumer-v1600.py'],'native-table-source-consumer-v1600/receipt.json',True)]+[(suite,['/usr/bin/python3','/tmp/openui-native-table-source-matrices-v1600.py',suite],f'native-table-source-clean-{suite}-v1600/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']]
body=old_text.split('\n',1)[1];start=body.index('report.update(owner_pid=');end=body.index("receipt = OUT / 'receipt.json'",start)
body=body[:start]+f'''report.update(owner_pid=os.getpid(),baseline_commit={BASELINE!r},expected_stages=7,
    workspace_cleaned_at_every_source_switch=True,existing_113_exports_and_30_layouts_preserved=True,
    native_event_api_preserved=True,c_abi_unchanged=True,actual_pixel_gain_claimed=False,
    root_cause_owner='openui-layout canonical repeated table source and ancestor continuation geometry',
    strict_named_baseline_failure_required=True,public_rust_callbacks_and_owned_bounds=True,
    strict_chromium_capture_pairs=True,preserves_both_unstable_reference_captures=True,
    immutable_source_shared_across_slices_required=True,source_release_after_teardown_required=True,
    accepted_renderer_unchanged=True,source_retention_hypothesis_not_yet_natively_verified=True)
report.pop('renderer_production_unchanged_from_applied_renderer',None)
'''+body[end:]
owner=Path('/tmp/openui-native-table-source-pipeline-v1601.py');files[owner]='CONFIG = '+repr(config)+'\n'+body
for path,text in files.items():
 assert not path.exists();ast.parse(text)
 assert '4b3cb72c' not in text and '4318f606' not in text and '/dev/shm/openui-native-table-progress-b7e28e56' not in text
assert "BRANCH = 'agent/native-table-source-v1597'" in files[Path('/tmp/openui-native-table-source-guards-v1600.py')]
assert 'Chromium table continuation count' in files[Path('/tmp/openui-native-table-source-guards-v1600.py')]
for path,text in files.items():path.write_text(text)
report=dict(schema_version=1,source=source,source_after=source,baseline_commit=BASELINE,branch=BRANCH,
    read_only_checks_passed=11,prior_whole_owners=31,prior_whole_pipelines=priors,
    strict_named_baseline_failure_required=True,original_chromium_expected_count_preserved=41,
    baseline_already_contains_rejected_progress_trial=True,source_retention_and_teardown_checks_required=True,
    current_exports=113,existing_30_c_layouts_preserved=True,required_native_images=60,
    required_independent_chromium_processes=120,required_consecutive_chromium_captures=240,
    all_four_matrices_required=True,required_stages=7,pixel_tolerance=0,
    javascript_executed_by_openui=False,public_native_rust_api_required=True,
    old_openui_pixels_are_provenance_only=True,applied_to_umbrella=False,release_qualification=False,
    new_release_states_admitted=0,baseline_and_fixed_guards_pending=True,
    cargo_commands_run=0,screenshots_generated=0,checks_receipt_sha256=sha(checks_path),
    geometry_receipt_sha256=sha(geometry_path),absolute_geometry_receipt_sha256=sha(absolute_path),
    scripts={str(p):sha(p) for p in files},original_probe_sha256=original_hashes,
    reference_capture_source_sha256=sha(reference),probe_sha256=sha(Path(__file__)))
path=RAW/'native-table-source-queue-prepared-v1600.json';assert not path.exists();path.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
assert repository_source_identity(ROOT)==source
print(json.dumps(dict(path=str(path),sha256=sha(path),source=FIXED,baseline=BASELINE,prior_whole_owners=31,stages=7)),flush=True)
