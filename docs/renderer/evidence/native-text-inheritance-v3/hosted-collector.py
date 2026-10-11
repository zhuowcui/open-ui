"""Record actual terminal hosted results without hiding cancellations."""
import collections, hashlib, json, subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=RAW/'native-text-style-hosted-v1684';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def get(args):return json.loads(subprocess.check_output(['gh',*args],cwd=ROOT))
main='95426acf51bcf789108c68671f5d875d051bd7b4';native='41b616c3be5224b074e9d06ba6aa963731a1b265'
old=json.loads((RAW/'native-umbrella-terminal-ci-v1674/receipt.json').read_bytes())
assert old['source']==main
rows=[]
for oldrow in old['workflows']:
 run=oldrow['databaseId'];row=get(['run','view',str(run),'--json','databaseId,headSha,url,conclusion,status,workflowName,jobs,attempt'])
 assert row['headSha']==main and row['status']=='completed'
 if row['workflowName']=='CI':assert row['attempt']==2
 rows.append(row)
private=get(['run','view','37366409715','--json','databaseId,headSha,url,conclusion,status,workflowName,jobs,attempt'])
assert private['headSha']==native and private['status']=='completed'
assert len(private['jobs'])==7 and all(j['conclusion']=='success' for j in private['jobs'])
rows.append(private)
for row in rows:
 stem=str(row['databaseId'])+'-attempt'+str(row['attempt'])
 p=OUT/(stem+'.json');p.write_text(json.dumps(row,sort_keys=True,indent=2)+'\n')
 result=subprocess.run(['gh','run','view',str(row['databaseId']),'--log'],cwd=ROOT,capture_output=True)
 log=OUT/(stem+'.log');log.write_bytes(result.stdout+result.stderr)
 row.update(metadata_path=str(p),metadata_sha256=sha(p),log_path=str(log),log_sha256=sha(log),log_capture_actual_exit=result.returncode,
 unavailable_logs_are_not_passes=True,cancelled_jobs_with_no_steps=[j['name'] for j in row['jobs'] if j['conclusion']=='cancelled' and not j.get('steps')])
mainrows=[r for r in rows if r['headSha']==main]
report=dict(schema_version=1,all_commands_terminal=True,all_hosted_jobs_terminal=True,source=main,native_source=native,workflows=rows,
 main_job_conclusions=dict(collections.Counter(j['conclusion'] for r in mainrows for j in r['jobs'])),
 native_job_conclusions=dict(collections.Counter(j['conclusion'] for j in private['jobs'])),
 native_all_seven_pass=True,native_pixel_qualification=False,main_all_workflows_success=all(r['conclusion']=='success' for r in mainrows),
 release_qualification=False,probe_sha256=sha(Path(__file__)))
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(receipt_sha256=sha(p),main_job_conclusions=report['main_job_conclusions'],native_job_conclusions=report['native_job_conclusions'])),flush=True)
raise SystemExit(int(not report['main_all_workflows_success']))
