"""Freeze the next whole pipeline without executing Cargo or image capture."""
import ast,hashlib,json,subprocess
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-table-progress-b7e28e56');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
FIXED='4b3cb72c03c58416a736a207b97cdcc8a225cb5a';BASELINE='4318f606d9ed9c204ea9ee3edacbfee9b0fd5479';BRANCH='agent/native-table-progress-v1543'
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
old_owner=Path('/tmp/openui-native-inline-replaced-pipeline-v1529.py').read_text();old_config=ast.literal_eval(ast.parse(old_owner).body[0].value);priors=old_config['prior_pipelines']+[old_config['name']];assert len(priors)==27
checks_path=RAW/'native-table-progress-checks-v1546/receipt.json';checks=json.loads(checks_path.read_bytes());assert checks['all_commands_terminal'] and checks['source']==checks['source_after'] and checks['source']['commit']==FIXED and checks['source']['clean'];assert len(checks['checks'])==11 and all(r['observed_exit_code']==0 for r in checks['checks'])
geometry_path=RAW/'native-table-progress-geometry-v1545/receipt.json';geometry=json.loads(geometry_path.read_bytes());assert geometry['all_commands_terminal'] and geometry['observed_exit_code']==0 and geometry['independent_queries_identical'];assert sum(len(r['rows']) for r in geometry['runs'])==96
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==FIXED;assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT)
def adapt(s):
 return s.replace('/dev/shm/openui-native-inline-replaced-80e71181',str(ROOT)).replace('c92e2d08163779f5e55784f14e556ecaa9d16e7f',FIXED).replace('d8742684adf1985ce6db2715778fbbfb0770e6d9',BASELINE).replace('agent/native-inline-replaced-v1524',BRANCH).replace('native-inline-replaced-','native-table-progress-').replace('v1528','v1547').replace('native_inline_replaced','native_table_progress')
paths=[]
guard=adapt(Path('/tmp/openui-native-inline-replaced-guards-v1528.py').read_text()).replace('native_table_progress_boxes_preserve_size_through_retained_mutations','native_repeated_table_body_progress_survives_retained_mutations').replace('native inline replaced content must keep its authored box','Chromium table continuation count').replace('baseline-inline-replaced-guard','baseline-table-progress-guard').replace('fixed-inline-replaced-guard','fixed-table-progress-guard')
a=guard.index('for pipeline in ');b=guard.index(':\n',a);guard=guard[:a]+'for pipeline in '+repr(priors)+guard[b:]
a=guard.index("    for name, test in [('fixed-table-progress-guard', TEST),");b=guard.index('        row, content = run',a);guard=guard[:a]+"    for name, test in [('fixed-table-progress-guard', TEST)]:\n"+guard[b:]
a=guard.index("    report['observed_exit_code'] = 0");guard=guard[:a]+'''    row, content = run('fixed-fragmentation-neighbors', build_base + ['test', '--locked', '-p', 'openui-layout', '--test', 'sp13_f_fragmentation_tests'], FIXED)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    counts = re.findall(rb'test result: ok\\. (\\d+) passed; (\\d+) failed; (\\d+) ignored;', content)
    assert len(counts) == 1 and int(counts[0][0]) > 0 and counts[0][1:] == (b'0', b'0')
    row['test_counts'] = [list(map(int, counts[0]))]
'''+guard[a:]
path=Path('/tmp/openui-native-table-progress-guards-v1547.py');path.write_text(guard);paths.append(path)
build=adapt(Path('/tmp/openui-native-inline-replaced-build-v1528.py').read_text()).replace("('inline-build'","('table-build'")
a=build.index('for prior in ');b=build.index(':\n',a);build=build[:a]+'for prior in '+repr(priors)+build[b:]
path=Path('/tmp/openui-native-table-progress-build-v1547.py');path.write_text(build);paths.append(path)
matrix=adapt(Path('/tmp/openui-native-inline-replaced-matrices-v1528.py').read_text());path=Path('/tmp/openui-native-table-progress-matrices-v1547.py');path.write_text(matrix);paths.append(path)
# The consumer is written separately and is included in the immutable owner hash set.
consumer=Path('/tmp/openui-native-table-progress-consumer-v1547.py');assert consumer.exists();paths.append(consumer)
config=dict(old_config,root=str(ROOT),commit=FIXED,name='native-table-progress-pipeline-v1548',initial_state='awaiting-all-27-prior-whole-pipelines',prior_pipelines=priors,selections=['native-table-progress-checks-v1546/receipt.json','native-table-progress-geometry-v1545/receipt.json'],scripts=[p.name for p in paths]+['openui-native-image-coverage-fieldsets-v1448.py'],stages=[('guards',['/usr/bin/python3',str(paths[0])],'native-table-progress-guards-v1547/receipt.json',True),('native-build',['/usr/bin/python3',str(paths[1])],'native-table-progress-clean-v1547/build.json',True),('native-application',['/usr/bin/python3',str(consumer)],'native-table-progress-consumer-v1547/receipt.json',True)]+[(suite,['/usr/bin/python3',str(paths[2]),suite],f'native-table-progress-clean-{suite}-v1547/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']])
owner=old_owner[old_owner.index('\nimport hashlib'):];owner=owner.replace("'d8742684adf1985ce6db2715778fbbfb0770e6d9'",repr(BASELINE)).replace("root_cause_owner='openui-layout shared inline replaced box collection'","root_cause_owner='openui-layout repeated table body progress and native fragment geometry'")
owner=owner.replace('    renderer_production_unchanged_from_applied_renderer=True,','    renderer_production_unchanged_from_applied_renderer=False,')
owner=owner.replace("consumer_compile_correction_only=False, renderer_production_unchanged_from_applied_renderer=True,","consumer_compile_correction_only=False, renderer_production_unchanged_from_applied_renderer=False,")
owner=owner.replace("isolated_from_rejected_intrinsic_changes=True)","isolated_from_rejected_intrinsic_changes=True, native_build_and_pixel_results_pending=True)")
path=Path('/tmp/openui-native-table-progress-pipeline-v1548.py');path.write_text('CONFIG = '+repr(config)+owner);paths.append(path)
for p in paths:ast.parse(p.read_text())
report=dict(schema_version=1,source=checks['source'],source_after=checks['source'],baseline_commit=BASELINE,branch=BRANCH,readonly_checks=11,chromium_geometry_observations=96,independent_geometry_runs=2,cargo_commands_run=0,screenshots_generated=0,baseline_and_fixed_guards_pending=True,native_application_and_four_matrices_pending=True,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,public_native_rust_api=True,javascript_executed_by_openui=False,old_openui_pixels_are_provenance_only=True,required_native_images=60,required_independent_chromium_processes=120,required_consecutive_chromium_captures=240,prior_whole_owners=27,checks_receipt_sha256=sha(checks_path),geometry_receipt_sha256=sha(geometry_path),scripts={str(p):sha(p) for p in paths},reference_capture_source_sha256=sha(Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py')),probe_sha256=sha(Path(__file__)))
p=RAW/'native-table-progress-queue-prepared-v1547.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(path=str(p),sha256=sha(p),source=FIXED,prior_whole_owners=27,baseline_tests_pending=True)),flush=True)
