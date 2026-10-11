"""Observe every hosted job on the exact documentation checkpoint."""
import collections,hashlib,json,subprocess,time
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1';HEAD='a6bd4629580887722aacca45cf4d1ce2933a4f3d'
OUT=RAW/'native-text-style-umbrella-ci-v1691';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
runs={'CI':37371335916,'Format Check':37371335936,'v0.2 Hardening':37371335970}
report=dict(schema_version=1,source=HEAD,all_commands_terminal=False,all_hosted_jobs_terminal=False,release_qualification=False,workflows=[],probe_sha256=sha(Path(__file__)))
p=OUT/'receipt.json';save=lambda:p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save();deadline=time.monotonic()+3600;previous=None
while True:
 rows=[]
 for name,run in runs.items():
  row=json.loads(subprocess.check_output(['gh','run','view',str(run),'--json','databaseId,headSha,url,status,conclusion,workflowName,jobs,attempt'],cwd=ROOT))
  assert row['headSha']==HEAD and row['workflowName']==name;rows.append(row)
 report['workflows']=rows;report['all_hosted_jobs_terminal']=all(r['status']=='completed' for r in rows);save()
 observed=[(r['workflowName'],r['status'],r['conclusion']) for r in rows]
 if observed!=previous:print(json.dumps(dict(workflow_states=observed)),flush=True);previous=observed
 if report['all_hosted_jobs_terminal']:break
 if time.monotonic()>=deadline:
  report.update(all_commands_terminal=True,observation_deadline_reached=True);save();raise SystemExit(2)
 time.sleep(20)
for row in rows:
 stem=str(row['databaseId'])+'-attempt'+str(row['attempt']);metadata=OUT/(stem+'.json');metadata.write_text(json.dumps(row,sort_keys=True,indent=2)+'\n');row.update(metadata_path=str(metadata),metadata_sha256=sha(metadata),job_logs=[])
 for job in row['jobs']:
  if job['conclusion']=='skipped':continue
  log=OUT/(str(job['databaseId'])+'.log');result=subprocess.run(['gh','run','view',str(row['databaseId']),'--job',str(job['databaseId']),'--log'],cwd=ROOT,capture_output=True);log.write_bytes(result.stdout+result.stderr)
  if job['conclusion']=='success':assert result.returncode==0 and len(result.stdout)>0
  row['job_logs'].append(dict(job=job['name'],job_id=job['databaseId'],conclusion=job['conclusion'],path=str(log),sha256=sha(log),capture_actual_exit=result.returncode))
report.update(all_commands_terminal=True,all_three_workflows_success=all(r['conclusion']=='success' for r in rows),job_conclusions=dict(collections.Counter(j['conclusion'] for r in rows for j in r['jobs'])),cancelled_or_skipped_jobs_are_not_passes=True)
save();print(json.dumps(dict(receipt_sha256=sha(p),source=HEAD,job_conclusions=report['job_conclusions'],all_three_workflows_success=report['all_three_workflows_success'])),flush=True);raise SystemExit(int(not report['all_three_workflows_success']))
