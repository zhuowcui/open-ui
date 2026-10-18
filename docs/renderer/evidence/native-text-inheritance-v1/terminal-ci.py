"""Record every hosted job, including a cancelled job with unavailable logs."""
import collections,hashlib,json,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1';OUT=RAW/'native-umbrella-terminal-ci-v1674';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
HEAD='95426acf51bcf789108c68671f5d875d051bd7b4';sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def get(args):return json.loads(subprocess.check_output(['gh',*args],cwd=ROOT))
candidates=get(['run','list','--branch','agent/v02-final-closure','--commit',HEAD,'--limit','30','--json','databaseId,workflowName,headSha,status'])
wanted={'CI','Format Check','v0.2 Hardening'};byname={r['workflowName']:r for r in candidates if r['headSha']==HEAD and r['workflowName'] in wanted};assert set(byname)==wanted
rows=[]
for name in sorted(wanted):
 run=byname[name]['databaseId'];row=get(['run','view',str(run),'--json','databaseId,headSha,url,conclusion,status,workflowName,jobs'])
 assert row['headSha']==HEAD and row['status']=='completed'
 meta=get(['api',f'repos/zhuowcui/open-ui/actions/runs/{run}']);row['run_attempt']=meta['run_attempt']
 p=OUT/(str(run)+'.json');p.write_text(json.dumps(row,sort_keys=True,indent=2)+'\n')
 result=subprocess.run(['gh','run','view',str(run),'--log'],cwd=ROOT,capture_output=True);log=OUT/(str(run)+'.log');log.write_bytes(result.stdout+result.stderr)
 row.update(log_capture_actual_exit=result.returncode,log_sha256=sha(log),all_available_logs_preserved=True,cancelled_jobs_with_no_steps=[j['name'] for j in row['jobs'] if j['conclusion']=='cancelled' and not j.get('steps')])
 rows.append(row)
counts=collections.Counter(j['conclusion'] for row in rows for j in row['jobs'])
report=dict(schema_version=1,source=HEAD,all_commands_terminal=True,all_hosted_jobs_terminal=True,workflows=rows,job_conclusions=dict(counts),
 all_three_workflows_success=all(r['conclusion']=='success' for r in rows),unavailable_job_logs_are_not_passes=True,
 release_qualification=False,probe_sha256=sha(Path(__file__)))
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(source=HEAD,workflows=[dict(name=r['workflowName'],conclusion=r['conclusion'],attempt=r['run_attempt']) for r in rows],job_conclusions=dict(counts),receipt_sha256=sha(p))),flush=True)
raise SystemExit(int(not report['all_three_workflows_success']))
