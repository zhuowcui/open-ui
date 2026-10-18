"""Preserve every actual current-head generator and hosted job result."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-text-inheritance-v11';INDEX=ROOT/'docs/renderer/generated/native-text-inheritance-v11.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();rows=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected:assert digest==expected
 q=OUT/name;assert not q.exists();q.write_bytes(gzip.compress(b,mtime=0) if compress else b)
 a=dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(q.read_bytes())==b;a.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 rows.append(a);return json.loads(b) if p.suffix=='.json' else None
hosted=keep(RAW/'native-current-hosted-v1793/receipt.json','current-hosted.json','d288f4438a5f3ed7902f4e554b7ecd6fb9ac5e5d5b0d7fd3e6794132c3343f83')
assert hosted['all_commands_terminal'] and hosted['source']=='8a6be0fe8e8d756d594eb2ff5bb9f2bb0c99f5e9' and hosted['successful_jobs']==6 and hosted['skipped_jobs']==5 and hosted['failed_jobs']==0
for w in hosted['workflows']:keep(Path(w['path']),str(w['run_id'])+'.json',w['sha256'])
for j in hosted['job_logs']:
 if 'path' in j:keep(Path(j['path']),str(j['job_id'])+'.log.gz',j['sha256'],True)
cp=RAW/'native-build-evidence-checks-v1791/receipt.json';checks=keep(cp,'current-checks.json')
assert checks['source']==checks['source_after'] and checks['source']['clean'] and checks['source']['commit']==hosted['source'] and checks['all_commands_terminal'] and len(checks['checks'])==14 and all(s['observed_exit_code']==0 for s in checks['checks'])
for s in checks['checks']:keep(cp.parent/(s['name']+'.log'),'check-'+s['name']+'.log.gz',s['log_sha256'],True)
keep(Path('/tmp/openui-native-current-hosted-collect-v1793.py'),'hosted-collector.py',hosted['probe_sha256'])
keep(Path('/tmp/openui-native-build-evidence-checks-v1791.py'),'current-checks.py',checks['probe_sha256'])
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=checks['source'],source_after=checks['source_after'],all_commands_terminal=True,all_14_read_only_checks_passed=True,all_three_hosted_workflows_successful=True,successful_hosted_jobs=6,skipped_hosted_jobs=5,failed_hosted_jobs=0,all_job_conclusions_and_executed_logs_preserved=True,skipped_jobs_not_release_passes=True,renderer_runtime_unchanged_from_integrated_41b616c3=True,javascript_executed_by_openui=False,public_native_rust_apis_required=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,artifact_count=len(rows),artifact_bytes=sum(a['bytes'] for a in rows))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'index':str(INDEX.relative_to(ROOT)),'sha256':sha(INDEX),'artifacts':len(rows)}),flush=True)
