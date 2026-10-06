"""Save real hosted snapshots and collect logs only after every job is terminal."""
import concurrent.futures,hashlib,json,subprocess,sys,time
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=RAW/'native-rust-integrated-hosted-watch-v1823';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
HEAD='16187f4f54e1b62ea317a155f620c42f91a38c72';RUNS=[37407697770,37407697750,37407697920,37407693299]
def query(run):
 r=subprocess.run(['gh','run','view',str(run),'--json','databaseId,headSha,status,conclusion,attempt,jobs'],cwd=ROOT,capture_output=True);assert r.returncode==0;d=json.loads(r.stdout);assert d['headSha']==HEAD;return run,r.stdout,d
snapshots=[];terminal=False
for attempt in range(10):
 with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:results=list(pool.map(query,RUNS))
 rows=[]
 for run,data,d in results:
  path=OUT/f'{attempt:02d}-{run}.json';path.write_bytes(data);rows.append(dict(run_id=run,path=str(path),sha256=hashlib.sha256(data).hexdigest(),status=d['status'],conclusion=d['conclusion']))
 snapshots.append(rows);terminal=all(d['status']=='completed' for _,_,d in results)
 print(json.dumps({'attempt':attempt,'completed_workflows':sum(d['status']=='completed' for _,_,d in results),'total_workflows':4,'terminal':terminal}),flush=True)
 if terminal:break
 time.sleep(30)
report=dict(schema_version=1,source=HEAD,all_commands_terminal=True,all_hosted_workflows_terminal=terminal,probe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),snapshots=snapshots,release_qualification=False)
if terminal:
 script=Path('/tmp/openui-native-rust-integrated-hosted-collect-v1819.py');report['collector_probe_sha256']=hashlib.sha256(script.read_bytes()).hexdigest()
 with (OUT/'collector.log').open('xb') as stream:r=subprocess.run([sys.executable,str(script)],cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
 report['collector_observed_exit_code']=r.returncode;print(json.dumps({'collector_actual_exit':r.returncode}),flush=True)
(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
raise SystemExit(0 if terminal and report.get('collector_observed_exit_code')==0 else 1)
