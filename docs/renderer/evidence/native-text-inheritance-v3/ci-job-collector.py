"""Capture individual CI job logs when the cancelled job prevents a whole-run dump."""
import hashlib,json,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=RAW/'native-text-style-ci-job-logs-v1685';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
parent=RAW/'native-text-style-hosted-v1684/receipt.json';d=json.loads(parent.read_bytes());r=next(r for r in d['workflows'] if r['workflowName']=='CI');assert r['attempt']==2
rows=[]
for job in r['jobs']:
 p=OUT/(str(job['databaseId'])+'.log')
 result=subprocess.run(['gh','run','view',str(r['databaseId']),'--job',str(job['databaseId']),'--log'],cwd=ROOT,capture_output=True)
 p.write_bytes(result.stdout+result.stderr)
 if job['conclusion']=='success':assert result.returncode==0 and len(result.stdout)>0
 else:assert job['conclusion']=='cancelled' and not job['steps']
 rows.append(dict(job=job['name'],job_id=job['databaseId'],job_conclusion=job['conclusion'],log_path=str(p),log_sha256=sha(p),capture_actual_exit=result.returncode,bytes=p.stat().st_size))
report=dict(schema_version=1,source=r['headSha'],run=r['databaseId'],attempt=2,parent_receipt_sha256=sha(parent),jobs=rows,
 all_commands_terminal=True,all_available_ci_logs_preserved=True,cancelled_job_not_passed=True,release_qualification=False,probe_sha256=sha(Path(__file__)))
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(receipt_sha256=sha(p),jobs=[{k:v for k,v in x.items() if k not in ('log_path','log_sha256')} for x in rows])),flush=True)
