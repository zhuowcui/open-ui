"""Freeze the native keyword retry behind all preceding whole pipelines."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-keywords-92741843')
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
SOURCE='06e1f89a4a2e7a53465bceb780675383d9748464'
BASELINE='027480369dae42ca774ee8e4865ee29e9c634af1'
BRANCH='agent/native-keywords-v1585'
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==SOURCE
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==SOURCE
checks_path=RAW/'native-keywords-checks-v1586/receipt.json';checks=json.loads(checks_path.read_bytes())
assert checks['source']==checks['source_after']==source and checks['all_commands_terminal']
assert len(checks['checks'])==13 and all(r['observed_exit_code']==0 for r in checks['checks'])
previous=Path('/tmp/openui-native-inline-fallback-pipeline-v1576.py')
previous_text=previous.read_text();config=ast.literal_eval(ast.parse(previous_text).body[0].value)
priors=config['prior_pipelines']+[config['name']];assert len(priors)==len(set(priors))==30
old_config=ast.literal_eval(ast.parse(Path('/tmp/openui-native-enum-values-pipeline-v1513.py').read_text()).body[0].value)
original_hashes={};files={}
def adapt(path):
    original_hashes[str(path)]=sha(path)
    text=path.read_text()
    substitutions=[('/dev/shm/openui-native-enum-values-619a465c',str(ROOT)),('893ea292cacf49b50240ba3176f4dc9ff55829d7',SOURCE),('33a6578b4c49bf83fbd40c77e409782d0e8f1e23',BASELINE),('agent/native-enum-values-v1509',BRANCH),('native-enum-values-','native-keywords-'),('v1512','v1587'),(repr(old_config['prior_pipelines']),repr(priors))]
    for before,after in substitutions:text=text.replace(before,after)
    return text
for kind in ['guards','build','consumer','matrices']:
    original=Path('/tmp/openui-native-enum-values-'+kind+'-v1512.py')
    text=adapt(original)
    files[Path('/tmp/openui-native-keywords-'+kind+'-v1587.py')]=text
assert "BRANCH = 'agent/native-keywords-v1585'" in files[Path('/tmp/openui-native-keywords-guards-v1587.py')]
assert 'native keyword constructor must accept ColumnFill auto' in files[Path('/tmp/openui-native-keywords-guards-v1587.py')]
assert "[[1, 0, 0]]" in files[Path('/tmp/openui-native-keywords-guards-v1587.py')]
# The already-corrected C adapter is a geometry diagnostic only. It never rasterizes.
original=Path('/tmp/openui-native-table-c-geometry-v1584.py');original_hashes[str(original)]=sha(original)
geometry=original.read_text().replace('/dev/shm/openui-native-raster-fields-retry-e0dc491e',str(ROOT)).replace('native-table-c-geometry-v1584','native-keywords-table-geometry-v1587').replace('native-raster-fields-retry-clean-v1559','native-keywords-clean-v1587')
needle="assert set(source_layout['types'])-set(main_layout['types'])=={'OuiRasterConfigurationV1','OuiTextRasterConfigurationV1'}"
assert geometry.count(needle)==1
geometry=geometry.replace(needle,"assert source_layout['types']==main_layout['types']\nassert build['source']['commit']=='"+SOURCE+"'")
anchor="report['used_abi_types_verified'] = used_types"
assert geometry.count(anchor)==1
geometry=geometry.replace(anchor,anchor+"\nreport.update(chromium_pixel_qualification=False, geometry_diagnostic_only=True, geometry_equality_with_chromium_not_claimed=True, expected_native_observations=120, missing_column_fill_constructor_previous_actual_exit=1)")
files[Path('/tmp/openui-native-keywords-table-geometry-v1587.py')]=geometry
config.update(root=str(ROOT),commit=SOURCE,name='native-keywords-pipeline-v1588',initial_state='awaiting-all-30-prior-whole-pipelines',prior_pipelines=priors,selections=['native-keywords-checks-v1586/receipt.json'])
config['scripts']=[p.name for p in files]+['openui-native-image-coverage-fieldsets-v1448.py']
config['stages']=[('guards',['/usr/bin/python3','/tmp/openui-native-keywords-guards-v1587.py'],'native-keywords-guards-v1587/receipt.json',True),('native-build',['/usr/bin/python3','/tmp/openui-native-keywords-build-v1587.py'],'native-keywords-clean-v1587/build.json',True),('native-application',['/usr/bin/python3','/tmp/openui-native-keywords-consumer-v1587.py'],'native-keywords-consumer-v1587/receipt.json',True),('native-table-geometry-diagnostic',['/usr/bin/python3','/tmp/openui-native-keywords-table-geometry-v1587.py'],'native-keywords-table-geometry-v1587/receipt.json',True)]+[(suite,['/usr/bin/python3','/tmp/openui-native-keywords-matrices-v1587.py',suite],f'native-keywords-clean-{suite}-v1587/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']]
body=previous_text.split('\n',1)[1];start=body.index('report.update(owner_pid=');end=body.index("receipt = OUT / 'receipt.json'",start)
body=body[:start]+f'''report.update(owner_pid=os.getpid(),baseline_commit={BASELINE!r},expected_stages=8,
    workspace_cleaned_at_every_source_switch=True,existing_113_exports_and_30_layouts_preserved=True,
    native_event_api_preserved=True,c_abi_unchanged=True,actual_pixel_gain_claimed=False,
    root_cause_owner='openui-style shared native value construction',native_constructor_qualification_pending=True,
    strict_named_baseline_failure_required=True,public_rust_callbacks_and_owned_bounds=True,
    strict_chromium_capture_pairs=True,preserves_both_unstable_reference_captures=True,
    native_table_geometry_is_diagnostic_only=True,accepted_renderer_unchanged=True)
report.pop('renderer_production_unchanged_from_applied_renderer',None)
'''+body[end:]
owner=Path('/tmp/openui-native-keywords-pipeline-v1588.py');files[owner]='CONFIG = '+repr(config)+'\n'+body
for path,text in files.items():
    assert not path.exists()
    ast.parse(text)
    assert '/dev/shm/openui-native-enum-values-' not in text
    assert '893ea292' not in text and '33a6578b' not in text
for path,text in files.items():path.write_text(text)
assert repository_source_identity(ROOT)==source
report=dict(schema_version=1,source=source,baseline_commit=BASELINE,branch=BRANCH,
    prior_whole_pipelines_required_terminal=30,prior_whole_pipelines=priors,
    checks_receipt_sha256=sha(checks_path),read_only_checks_passed=13,
    strict_named_baseline_failure_required=True,named_fixed_guards_required=3,
    workspace_packages_cleaned=18,current_exports=113,current_struct_layouts=30,
    required_native_images=10,required_independent_chromium_processes=20,required_consecutive_chromium_captures=40,
    native_table_geometry_diagnostic_only=True,required_native_table_geometry_observations=120,
    all_four_matrices_required=True,pixel_tolerance=0,applied_to_umbrella=False,
    accepted_renderer_unchanged=True,release_qualification=False,new_release_states_admitted=0,
    javascript_executed_by_openui=False,cargo_commands_run=0,screenshots_generated=0,
    scripts={str(p):sha(p) for p in files},original_probe_sha256=original_hashes,
    capture_helper_sha256=sha(Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py')),probe_sha256=sha(Path(__file__)))
out=RAW/'native-keywords-queue-prepared-v1587.json';assert not out.exists();out.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(path=str(out),sha256=sha(out),source=SOURCE,baseline=BASELINE,prior_whole_owners=30,stages=8)),flush=True)
