"""Retain every actual job result on the narrow native width source."""
import collections,hashlib,json,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1';OUT=RAW/'native-glyph-descriptor-hosted-v1760';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
row=json.loads(subprocess.check_output(['gh','run','view','37387792882','--json','databaseId,headSha,url,status,conclusion,workflowName,jobs,attempt'],cwd=ROOT));assert row['headSha']=='2f53d5de2e6fce9b4c9dc2fbc94fcfc763544b23' and row['status']=='completed' and row['attempt']==1 and len(row['jobs'])==7
meta=OUT/'attempt1.json';meta.write_text(json.dumps(row,sort_keys=True,indent=2)+'\n');logs=[]
for j in row['jobs']:
 p=OUT/(str(j['databaseId'])+'.log');result=subprocess.run(['gh','run','view','37387792882','--job',str(j['databaseId']),'--log'],cwd=ROOT,capture_output=True);p.write_bytes(result.stdout+result.stderr)
 if j['conclusion']=='success':assert result.returncode==0 and len(result.stdout)>0
 logs.append(dict(job=j['name'],job_id=j['databaseId'],conclusion=j['conclusion'],path=str(p),sha256=sha(p),capture_actual_exit=result.returncode,steps=j['steps']))
report=dict(schema_version=1,source=row['headSha'],workflow=row,metadata_sha256=sha(meta),job_logs=logs,all_commands_terminal=True,all_hosted_jobs_terminal=True,
 job_conclusions=dict(collections.Counter(j['conclusion'] for j in row['jobs'])),all_seven_jobs_passed=all(j['conclusion']=='success' for j in row['jobs']),
 cancelled_jobs_not_passes=True,release_qualification=False,probe_sha256=sha(Path(__file__)))
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(receipt_sha256=sha(p),job_conclusions=report['job_conclusions'],cancelled_jobs_with_no_steps=[j['name'] for j in row['jobs'] if j['conclusion']=='cancelled' and not j['steps']])),flush=True);raise SystemExit(int(not report['all_seven_jobs_passed']))
