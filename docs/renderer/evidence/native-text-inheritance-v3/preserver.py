"""Preserve all measured hosted results and available per-job logs."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-text-inheritance-v3';INDEX=ROOT/'docs/renderer/generated/native-text-inheritance-v3.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();artifacts=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected:assert digest==expected
 assert p.read_bytes()==b
 out=OUT/name;assert not out.exists();out.write_bytes(gzip.compress(b,compresslevel=9,mtime=0) if compress else b)
 a=dict(path=str(out.relative_to(ROOT)),sha256=sha(out),bytes=out.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(out.read_bytes())==b;a.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 artifacts.append(a);return json.loads(b) if p.suffix=='.json' else None
report=keep(RAW/'native-text-style-hosted-v1684/receipt.json','hosted-terminal.json','811154761e4e8c475edd66419d129cc9d1e380ec81db12f64098381e58d65c88')
assert report['all_commands_terminal'] and report['native_all_seven_pass'] and not report['main_all_workflows_success']
keep(Path('/tmp/openui-native-text-style-hosted-v1684.py'),'hosted-collector.py',report['probe_sha256'])
for r in report['workflows']:
 stem=str(r['databaseId'])+'-attempt'+str(r['attempt'])
 keep(Path(r['metadata_path']),stem+'.json',r['metadata_sha256'])
 keep(Path(r['log_path']),stem+'.log.gz',r['log_sha256'],compress=True)
logs=keep(RAW/'native-text-style-ci-job-logs-v1685/receipt.json','ci-job-log-terminal.json','ae626401aa033f168ac9cab54a77807632075be42f2d4111ea904dfbaaa15047')
assert logs['all_commands_terminal'] and logs['parent_receipt_sha256']==sha(RAW/'native-text-style-hosted-v1684/receipt.json')
keep(Path('/tmp/openui-native-text-style-ci-job-logs-v1685.py'),'ci-job-collector.py',logs['probe_sha256'])
for r in logs['jobs']:keep(Path(r['log_path']),'ci-job-'+str(r['job_id'])+'.log.gz',r['log_sha256'],compress=True)
keep(Path(__file__),'preserver.py')
index=dict(schema_version=1,source=report['native_source'],own_source_hosted_run=37366409715,own_source_all_seven_jobs_passed=True,own_source_skipped_jobs=0,
 umbrella_source=report['source'],umbrella_workflows=3,umbrella_successful_jobs=5,umbrella_skipped_jobs=5,umbrella_cancelled_jobs=1,
 umbrella_ci_run=37362608916,umbrella_ci_attempt=2,historical_audit_cancelled_without_steps=True,all_available_hosted_logs_preserved=True,
 previous_attempt_index='docs/renderer/generated/native-text-inheritance-v1.json',previous_attempt_index_sha256='f4d74b04230b461b98c5cc889febbc9b2a844b771521f50d22d10aa58b5f4e5e',
 cancelled_or_skipped_jobs_not_passes=True,strict_native_app_gate_failed=True,full_pixel_qualification_pending=True,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,new_release_states_admitted=0,release_qualification=False,
 artifact_count=len(artifacts),artifact_bytes=sum(r['bytes'] for r in artifacts),artifacts=artifacts)
INDEX.write_text(json.dumps(index,sort_keys=True,indent=2)+'\n')
for a in artifacts:
 assert sha(ROOT/a['path'])==a['sha256']
 if a.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress((ROOT/a['path']).read_bytes())).hexdigest()==a['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)),sha256=sha(INDEX),artifacts=len(artifacts),bytes=index['artifact_bytes'])),flush=True)
