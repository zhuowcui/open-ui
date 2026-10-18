"""Retain actual native style guard/build results and every latest CI job."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def bucket(name):
 out=ROOT/'docs/renderer/evidence'/name;index=ROOT/'docs/renderer/generated'/(name+'.json')
 assert not out.exists() and not index.exists();out.mkdir();return out,index,[]
def keep(out,rows,p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected:assert digest==expected
 q=out/name;assert not q.exists();q.write_bytes(gzip.compress(b,mtime=0) if compress else b)
 a=dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(q.read_bytes())==b;a.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 rows.append(a);return json.loads(b) if p.suffix=='.json' else None
out,index,rows=bucket('native-keywords-v4')
owner=keep(out,rows,RAW/'native-keywords-current-pipeline-v1773/receipt.json','original-owner-terminal.json','189711442797ae28149d346c7caa1b75dc768e81066bdab258f1167b79a12ba7')
assert owner['all_commands_terminal'] and [r['observed_exit_code'] for r in owner['steps']]==[0,241]
for s in owner['steps']:keep(out,rows,RAW/'native-keywords-current-pipeline-v1773'/(s['name']+'.log'),'owner-'+s['name']+'.log.gz',s['log_sha256'],True)
guard=keep(out,rows,RAW/'native-keywords-current-guards-v1772/receipt.json','guards-terminal.json','aac2e9336f9e616ed701e65be88841f7ecce1cffc0760f0461682250ca9798cb')
assert guard['all_commands_terminal'] and guard['baseline_regression_reproduced'] and [s['actual_exit'] for s in guard['steps']]==[0,101,0,0,0,0]
for s in guard['steps']:keep(out,rows,RAW/'native-keywords-current-guards-v1772'/(s['name']+'.log'),s['name']+'.log.gz',s['log_sha256'],True)
build=keep(out,rows,RAW/'native-keywords-current-clean-v1772/build.json','build-stopped.json','4795069932c02a93a2a487c33fce4e11f67ff7a9c314c17eefac4b35f63afc45')
assert build['all_commands_terminal'] and len(build['steps'])==7 and [s['observed_exit_code'] for s in build['steps']]==[0,0,0,0,0,0,-15]
assert build['steps'][1]['passed']==8554 and build['steps'][1]['failed']==0 and build['steps'][1]['ignored']==13 and build['steps'][-1]['disk_guard_triggered']
for s in build['steps']:keep(out,rows,RAW/'native-keywords-current-clean-v1772'/(s['name']+'.log'),'build-'+s['name']+'.log.gz',s['log_sha256'],True)
for probe,digest in owner['immutable_probe_hashes'].items():keep(out,rows,Path(probe),Path(probe).name,digest)
keep(out,rows,Path('/tmp/openui-native-keywords-current-pipeline-v1773.py'),'original-owner.py',owner['probe_sha256'])
for name,label in [('native-keywords-current-prepared-v1772.json','original-preparation.json'),('native-keywords-abi-scratch-relocation-v1777.json','abi-scratch-relocation.json'),('native-cargo-cache-relocation-v1778.json','cargo-cache-relocation.json.gz'),('native-keywords-current-retry-prepared-v1779.json','fresh-retry-preparation.json')]:keep(out,rows,RAW/name,label,compress=label.endswith('.gz'))
for name in ['openui-native-keywords-current-prepare-v1772.py','openui-native-keywords-abi-scratch-relocate-v1777.py','openui-native-cargo-cache-relocate-v1778.py','openui-native-keywords-retry-prepare-v1779.py']:
 keep(out,rows,Path('/tmp')/name,name)
keep(out,rows,Path(__file__),'preserver.py')
report=dict(schema_version=1,source=owner['source'],source_after=owner['source_after'],baseline_source=guard['baseline_source'],all_original_stages_terminal=True,baseline_actual_exit=101,all_three_fixed_named_guards_passed=True,actual_guard_stage_exits=[0,101,0,0,0,0],original_owner_actual_exit=241,original_build_actual_exits=[0,0,0,0,0,0,-15],workspace_passed=8554,workspace_failed=0,workspace_ignored=13,actual_native_rust_callback_and_geometry_app_passed_at_five_scales=True,abi_checker_stopped_by_storage_guard=True,actual_c_cpp_native_app_and_pixel_gates_unexecuted_in_original_owner=True,temporary_storage_correction_preserves_all_bytes_and_original_paths=True,fresh_retry_required_and_separately_prepared=True,candidate_applied_to_umbrella=False,javascript_executed_by_openui=False,public_native_rust_apis_required=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,artifact_count=len(rows),artifact_bytes=sum(r['bytes'] for r in rows))
index.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'index':str(index.relative_to(ROOT)),'sha256':sha(index),'artifacts':len(rows)}),flush=True)
out,index,rows=bucket('native-text-inheritance-v10')
p=RAW/'native-current-hosted-v1776/receipt.json';hosted=keep(out,rows,p,'current-hosted.json','6121b3455f6a0d646b615764d5742835a3c2e8654ace5bd5f4be7242cb9aaefc')
assert hosted['all_commands_terminal'] and hosted['source']=='e82493683e02498bb8c5be570a4df22708c88c0c' and hosted['successful_jobs']==6 and hosted['skipped_jobs']==5 and hosted['failed_jobs']==0
for workflow in hosted['workflows']:keep(out,rows,Path(workflow['path']),str(workflow['run_id'])+'.json',workflow['sha256'])
for job in hosted['job_logs']:
 if 'path' in job:keep(out,rows,Path(job['path']),str(job['job_id'])+'.log.gz',job['sha256'],True)
checks_path=RAW/'native-width-rejection-checks-v1767/receipt.json';checks=keep(out,rows,checks_path,'current-checks.json')
assert checks['source']==checks['source_after'] and checks['source']['commit']==hosted['source'] and checks['all_commands_terminal'] and len(checks['checks'])==14 and all(r['observed_exit_code']==0 for r in checks['checks'])
for item in checks['checks']:keep(out,rows,checks_path.parent/(item['name']+'.log'),'check-'+item['name']+'.log.gz',item['log_sha256'],True)
keep(out,rows,Path('/tmp/openui-native-current-hosted-collect-v1776.py'),'hosted-collector.py',hosted['probe_sha256'])
keep(out,rows,Path('/tmp/openui-native-width-rejection-checks-v1767.py'),'current-checks.py',checks['probe_sha256'])
keep(out,rows,Path(__file__),'preserver.py')
report=dict(schema_version=1,source=checks['source'],source_after=checks['source_after'],all_commands_terminal=True,all_14_read_only_checks_passed=True,all_three_workflows_successful=True,successful_hosted_jobs=6,skipped_hosted_jobs=5,failed_hosted_jobs=0,all_job_conclusions_and_available_logs_preserved=True,skipped_jobs_not_release_passes=True,renderer_runtime_unchanged_from_integrated_41b616c3=True,javascript_executed_by_openui=False,public_native_rust_apis_required=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,artifact_count=len(rows),artifact_bytes=sum(r['bytes'] for r in rows))
index.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'index':str(index.relative_to(ROOT)),'sha256':sha(index),'artifacts':len(rows)}),flush=True)
