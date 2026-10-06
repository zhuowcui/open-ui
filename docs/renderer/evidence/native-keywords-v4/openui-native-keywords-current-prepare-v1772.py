"""Freeze a fresh current-source native keyword qualification pipeline."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui')
RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-keywords-current-0733955a')
BASE_ROOT=Path('/dev/shm/openui-native-keywords-baseline-7cb31314-v1772')
FIXED='7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
BASELINE='7cb31314f2f871e9d1bb6fc0dc63c594bb1cf20e'
sys.path.insert(0,str(MAIN/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=repository_source_identity(ROOT)
assert source['clean'] and source['commit']==FIXED
checks=RAW/'native-keywords-current-checks-v1749/receipt.json'
d=json.loads(checks.read_bytes());assert d['source']==d['source_after']==source and d['all_commands_terminal'] and len(d['checks'])==16 and all(r['observed_exit_code']==0 for r in d['checks'])
assert not BASE_ROOT.exists()
subprocess.run(['git','worktree','add','--detach',str(BASE_ROOT),BASELINE],cwd=MAIN,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
baseline_source=repository_source_identity(BASE_ROOT)
assert baseline_source['clean'] and baseline_source['commit']==BASELINE
files={};parents={}
def original(name):
 p=Path('/tmp')/name;parents[str(p)]=sha(p);return p.read_text()
guards=original('openui-native-glyph-descriptor-guards-v1764.py')
guards=guards.replace("FIXED_ROOT = Path('/dev/shm/openui-native-glyph-descriptor-corrected-2f53d5de-v1754')",f'FIXED_ROOT = Path({str(ROOT)!r})').replace("BASELINE_ROOT = Path('/dev/shm/openui-native-glyph-descriptor-baseline-0733955a-v1740')",f'BASELINE_ROOT = Path({str(BASE_ROOT)!r})').replace('fce42e086dee48f6e2d725b2b4f3bb8815e79e5c',FIXED).replace('54bcdeab5b021fc7899ca3e25868ab39182aee62',BASELINE).replace('native-glyph-descriptor-guards-v1764','native-keywords-current-guards-v1772')
guards=guards.replace("TEST = 'shaping::shape_result::tests::fontations_preserves_physical_strike_descriptor_for_real_fonts'","TEST = 'tests::native_fragment_keywords_reach_engine_and_keep_owned_property_identity'")
start=guards.index("lock = open(");end=guards.index('STORE.mkdir()',start)
guards=guards[:start]+"assert os.environ.get('OPENUI_NATIVE_WHOLE_OWNER') == 'native-keywords-current-pipeline-v1773'\nfor program in ['cargo', 'pixel_compare']:\n    assert subprocess.run(['pgrep', '-x', program], capture_output=True).returncode == 1\n"+guards[end:]
guards=guards.replace("['test', '--locked', '-p', 'openui-text', '--lib']","['test', '--locked', '-p', 'openui-ffi', '--lib']")
guards=guards.replace('prior_whole_owner_sha256=sha(prior),','')
guards=guards.replace('physical-strike','native-keyword').replace('Fontations must preserve the physical strike descriptor','native keyword constructor must accept ColumnFill auto')
start=guards.index("    row, data = run('all-text-tests'");end=guards.index("    report.update(state=",start)
guards=guards[:start]+'''    for name, package, named in [
        ('fixed-table-display-guard', 'openui-ffi', 'tests::native_table_display_literals_keep_existing_scalar_encoding'),
        ('fixed-enum-coverage-guard', 'openui-style', 'property::tests::native_fragment_keyword_values_cover_declared_enum_variants')]:
        command = base + ['test', '--locked', '-p', package, '--lib', named, '--', '--exact']
        row, data = run(name, FIXED_ROOT, command, source)
        assert row['actual_exit'] == 0 and row['passed'] == 1 and row['failed'] == 0 and not row['disk_guard_triggered'] and (named + ' ... ok').encode() in data
'''+guards[end:]
files[Path('/tmp/openui-native-keywords-current-guards-v1772.py')]=guards
build=original('openui-native-intrinsic-snap-build-v1716.py')
start=build.index('RAW=raw');end=build.index('sys.path.insert',start)
build=build[:start]+"assert os.environ.get('OPENUI_NATIVE_WHOLE_OWNER') == 'native-keywords-current-pipeline-v1773'\nassert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1\nassert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1\nout=raw/'native-keywords-current-clean-v1772';storage=Path('/mnt/e/openui-v02-qualification-d174ea0b')/out.name;assert not out.exists() and not storage.exists();storage.mkdir();out.symlink_to(storage,target_is_directory=True)\n"+build[end:]
build=build.replace('/dev/shm/openui-native-intrinsic-snap-full-727da10e',str(ROOT)).replace('727da10e580c9439f5db83d36bfc3e2340168207',FIXED)
start=build.index('steps=[');end=build.index('import importlib.util',start)
build=build[:start]+'''steps=[
 ('clean-workspace',base+['clean']+[arg for package in packages for arg in ['-p',package]],None),
 ('workspace',base+['test','--workspace','--locked','--features','openui-ffi/linux'],None),
 ('keywords-build',base+['build','--locked','-p','openui','--example','native_fragment_keywords'],'debug/examples/native_fragment_keywords'),
 ('ffi-build',base+['build','--locked','-p','openui-ffi','--features','linux'],'debug/libopenui_ffi.so'),
 ('pixel-build',base+['build','--locked','-p','pixel-compare','--bin','pixel_compare'],'debug/pixel_compare'),
 ('native-rust-geometry',[str(out/'native_fragment_keywords')],None),
 ('ffi-consumers',['python3',str(root/'tools/ffi/verify_abi.py'),'--library',str(out/'libopenui_ffi.so')],None),
]
'''+build[end:]
build=build.replace('text_content','fragment_keywords')
start=build.index("  if name=='workspace' and p.returncode==0:");end=build.index('  if p.returncode==0 and artifact:',start)
build=build[:start]+build[end:]
# Captured output is slim; raw command and identity remain in the receipt.
build=build.replace('print(json.dumps(entry),flush=True)',"print(json.dumps({k:entry[k] for k in ['name','observed_exit_code','disk_guard_triggered','passed','failed','ignored'] if k in entry}),flush=True)")
files[Path('/tmp/openui-native-keywords-current-build-v1772.py')]=build
for kind in ['consumer','table-geometry','matrices']:
 t=original('openui-native-keywords-'+kind+'-v1587.py')
 t=t.replace('/dev/shm/openui-native-keywords-92741843',str(ROOT)).replace('06e1f89a4a2e7a53465bceb780675383d9748464',FIXED).replace('native-keywords-','native-keywords-current-').replace('v1587','v1772')
 if kind=='consumer':t=t.replace("len(build['steps'])==7","len(build['steps'])==13")
 if kind=='matrices':t=t.replace('print(json.dumps(entry), flush=True)',"print(json.dumps({'suite':suite,'observed_exit_code':process.returncode}),flush=True)")
 files[Path('/tmp/openui-native-keywords-current-'+kind+'-v1772.py')]=t
prior=original('openui-native-intrinsic-snap-pipeline-v1717.py')
config=ast.literal_eval(ast.parse(prior).body[0].value)
config.update(root=str(ROOT),commit=FIXED,name='native-keywords-current-pipeline-v1773',initial_state='awaiting-terminal-previous-owners',prior_pipelines=['native-intrinsic-snap-pipeline-v1717','native-glyph-descriptor-guards-v1764'],selections=['native-keywords-current-checks-v1749/receipt.json','native-keywords-current-hosted-v1762/receipt.json'],scripts=[p.name for p in files]+['openui-native-image-coverage-fieldsets-v1448.py'])
config['stages']=[('guards',['/usr/bin/python3','/tmp/openui-native-keywords-current-guards-v1772.py'],'native-keywords-current-guards-v1772/receipt.json',True),('native-build',['/usr/bin/python3','/tmp/openui-native-keywords-current-build-v1772.py'],'native-keywords-current-clean-v1772/build.json',True),('native-application',['/usr/bin/python3','/tmp/openui-native-keywords-current-consumer-v1772.py'],'native-keywords-current-consumer-v1772/receipt.json',True),('native-table-geometry-diagnostic',['/usr/bin/python3','/tmp/openui-native-keywords-current-table-geometry-v1772.py'],'native-keywords-current-table-geometry-v1772/receipt.json',True)]+[(s,['/usr/bin/python3','/tmp/openui-native-keywords-current-matrices-v1772.py',s],f'native-keywords-current-clean-{s}-v1772/{s}-summary.json',False) for s in ['focused','primitive','full','expanded']]
body=prior.split('\n',1)[1]
body=body.replace('fcntl.flock(OWNER_LOCK, fcntl.LOCK_EX)','fcntl.flock(OWNER_LOCK, fcntl.LOCK_EX | fcntl.LOCK_NB)')
start=body.index('report.update(owner_pid=');end=body.index("receipt = OUT / 'receipt.json'",start)
body=body[:start]+f'''report.update(owner_pid=os.getpid(), baseline_commit={BASELINE!r}, expected_stages=8,
    whole_pipeline_lock_covers_all_stages_and_gaps=True,
    workspace_cleaned_at_every_source_switch=True, existing_113_exports_and_30_layouts_preserved=True,
    native_event_api_preserved=True, c_abi_unchanged=True, actual_pixel_gain_claimed=False,
    root_cause_owner='shared native style value construction',
    strict_named_baseline_failure_required=True, public_rust_c_cpp_callbacks_and_owned_bounds=True,
    strict_chromium_capture_pairs=True, preserves_both_unstable_reference_captures=True,
    native_table_geometry_is_diagnostic_only=True, accepted_renderer_unchanged=True,
    native_api_and_chromium_pixels_still_pending=True, old_queue_unlaunched=True)
'''+body[end:]
body=body.replace("env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1')","env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1', OPENUI_NATIVE_WHOLE_OWNER=CONFIG['name'])")
body=body.replace('print(json.dumps(row), flush=True)',"print(json.dumps({'stage':name,'actual_exit':process.returncode}),flush=True)")
files[Path('/tmp/openui-native-keywords-current-pipeline-v1773.py')]='CONFIG = '+repr(config)+'\n'+body
for p,t in files.items():
 assert not p.exists();ast.parse(t)
 assert 'native-keywords-92741843' not in t and '06e1f89a' not in t
 assert 'native-intrinsic-snap-full-' not in t
for p,t in files.items():p.write_text(t)
assert repository_source_identity(ROOT)==source and repository_source_identity(BASE_ROOT)==baseline_source
receipt=RAW/'native-keywords-current-prepared-v1772.json';assert not receipt.exists()
report=dict(schema_version=1,source=source,baseline_source=baseline_source,baseline_root=str(BASE_ROOT),public_native_rust_apis_required=True,javascript_executed_by_openui=False,release_qualification=False,new_release_states_admitted=0,screenshots_generated=0,cargo_commands_run=0,source_checks_passed=16,required_fixed_named_guards=3,stages=8,workspace_build_stages=13,pixel_tolerance=0,scripts={str(p):sha(p) for p in files},adapted_from=parents,probe_sha256=sha(Path(__file__)))
receipt.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'receipt':str(receipt),'sha256':sha(receipt),'source':FIXED,'baseline':BASELINE,'stages':8}),flush=True)
