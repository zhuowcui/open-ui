"""Preserve both the cancelled formatting attempt and its successful retry."""
import collections,gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-text-inheritance-v4';INDEX=ROOT/'docs/renderer/generated/native-text-inheritance-v4.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();artifacts=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected:assert digest==expected
 assert p.read_bytes()==b
 q=OUT/name;assert not q.exists();q.write_bytes(gzip.compress(b,compresslevel=9,mtime=0) if compress else b)
 a=dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(q.read_bytes())==b;a.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 artifacts.append(a);return json.loads(b) if p.suffix=='.json' else None
old=keep(RAW/'native-text-style-umbrella-ci-v1691/receipt.json','umbrella-hosted-attempt1.json','734a16f80a8792133f773da33d03a7413802a7607fc300a89bc02754c7867c0b');assert old['all_commands_terminal'] and not old['all_three_workflows_success']
keep(Path('/tmp/openui-native-text-style-umbrella-ci-v1691.py'),'umbrella-hosted-observer.py',old['probe_sha256'])
for w in old['workflows']:
 keep(Path(w['metadata_path']),str(w['databaseId'])+'-attempt1.json',w['metadata_sha256'])
 for j in w['job_logs']:keep(Path(j['path']),str(j['job_id'])+'.log.gz',j['sha256'],compress=True)
retry=keep(RAW/'native-umbrella-format-retry-v1699/receipt.json','format-attempt2-terminal.json','2b1162b8f1add03c41f811855477076042294aae69f4ba64cb88c859d1a9fb44');assert retry['all_commands_terminal'] and retry['format_workflow_passed'] and retry['previous_receipt_sha256']==sha(RAW/'native-text-style-umbrella-ci-v1691/receipt.json')
keep(Path('/tmp/openui-native-umbrella-format-retry-v1699.py'),'format-retry-observer.py',retry['probe_sha256'])
keep(RAW/'native-umbrella-format-retry-v1699/format-attempt2.json','format-attempt2.json',retry['metadata_sha256'])
for j in retry['job_logs']:keep(Path(j['path']),str(j['job_id'])+'-retry.log.gz',j['sha256'],compress=True)
latest=[retry['workflow'] if w['workflowName']=='Format Check' else w for w in old['workflows']]
counts=dict(collections.Counter(j['conclusion'] for w in latest for j in w['jobs']));assert counts==dict(success=6,skipped=5) and all(w['conclusion']=='success' for w in latest)
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=old['source'],all_three_workflows_success=True,all_hosted_jobs_terminal=True,successful_jobs=6,skipped_jobs=5,
 first_attempt_gn_job_cancelled_without_steps=True,first_attempt_not_rewritten=True,format_retry_attempt=2,all_available_hosted_logs_preserved=True,
 skipped_jobs_not_passes=True,full_pixel_gate_still_failed=True,accepted_original_exact=21334,accepted_expanded_exact=22137,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,release_qualification=False,new_release_states_admitted=0,
 artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for a in artifacts:
 assert sha(ROOT/a['path'])==a['sha256']
 if a.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress((ROOT/a['path']).read_bytes())).hexdigest()==a['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)),sha256=sha(INDEX),artifacts=len(artifacts),bytes=report['artifact_bytes'])),flush=True)
