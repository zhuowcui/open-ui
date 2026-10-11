"""Keep the complete native build and pre-render probe failure without admission."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-keywords-v5';INDEX=ROOT/'docs/renderer/generated/native-keywords-v5.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();rows=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected:assert digest==expected
 q=OUT/name;assert not q.exists();q.write_bytes(gzip.compress(b,mtime=0) if compress else b)
 a=dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(q.read_bytes())==b;a.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 rows.append(a);return json.loads(b) if p.suffix=='.json' else None
owner=keep(RAW/'native-keywords-current-retry-pipeline-v1780/receipt.json','owner-terminal.json','3d24233f9c05f8d156e837402918d71d8944e3e64a727f243ee6b8f76b987771');assert owner['all_commands_terminal'] and [s['observed_exit_code'] for s in owner['steps']]==[0,0,1]
for s in owner['steps']:keep(RAW/'native-keywords-current-retry-pipeline-v1780'/(s['name']+'.log'),'owner-'+s['name']+'.log.gz',s['log_sha256'],True)
build=keep(RAW/'native-keywords-current-retry-clean-v1779/build.json','complete-build.json','abcddbbfad58d6a136a482375c8e044dde1ac765d6d5ef77a69f7aa442d8aee8')
assert build['all_commands_terminal'] and len(build['steps'])==13 and all(s['observed_exit_code']==0 and not s['disk_guard_triggered'] for s in build['steps'])
assert build['source']==build['source_after']==owner['source']==owner['source_after'] and build['source']['commit']=='7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
for s in build['steps']:keep(RAW/'native-keywords-current-retry-clean-v1779'/(s['name']+'.log'),'build-'+s['name']+'.log.gz',s['log_sha256'],True)
for p,digest in owner['immutable_probe_hashes'].items():keep(Path(p),Path(p).name,digest)
keep(Path('/tmp/openui-native-keywords-current-retry-pipeline-v1780.py'),'owner.py',owner['probe_sha256'])
keep(RAW/'native-keywords-current-retry-guards-v1779/receipt.json','source-identical-guard-reuse.json')
keep(RAW/'native-keywords-current-retry-prepared-v1779.json','retry-preparation.json')
keep(RAW/'native-keywords-capture-path-failure-v1785.json','capture-source-path-failure.json')
keep(RAW/'native-keywords-capture-prepared-v1785.json','fresh-capture-preparation.json')
keep(Path('/tmp/openui-native-keywords-capture-prepare-v1785.py'),'fresh-capture-preparation.py')
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=owner['source'],source_after=owner['source_after'],all_original_owner_stages_terminal=True,actual_owner_stage_exits=[0,0,1],all_13_clean_build_stages_passed=True,workspace_passed=8554,workspace_failed=0,workspace_ignored=13,existing_exports_verified=113,existing_abi_layouts=30,actual_c_examples_run=13,actual_cpp_examples_run=7,public_rust_c_cpp_keyword_callback_apps_passed_at_five_scales=True,completed_same_source_guard_proof_reused=True,baseline_actual_exit=101,three_fixed_named_guards_passed=True,chromium_capture_probe_stopped_before_render_due_to_missing_source_path=True,original_owner_native_pixel_gate_not_executed=True,fresh_capture_root_assignments_ast_verified=True,candidate_applied_to_umbrella=False,javascript_executed_by_openui=False,public_native_rust_apis_required=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,artifact_count=len(rows),artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'index':str(INDEX.relative_to(ROOT)),'sha256':sha(INDEX),'artifacts':len(rows)}),flush=True)
