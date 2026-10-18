import concurrent.futures, hashlib, json, os, subprocess, sys
from pathlib import Path
root=Path('/dev/shm/openui-native-font-backends-107e2e36');raw=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1');out=raw/'native-font-backends-checks-v1274';out.mkdir();sys.path.insert(0,str(root/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(root);assert source['clean'];env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1');checks=json.loads((raw/'native-scroll-docs-checks-v543.json').read_text())['checks']
def run(t):
 log=out/(t['name']+'.log')
 with log.open('xb') as stream:p=subprocess.run(t['command'],cwd=root,env=env,stdout=stream,stderr=subprocess.STDOUT)
 return dict(name=t['name'],command=t['command'],observed_exit_code=p.returncode,log_sha256=hashlib.sha256(log.read_bytes()).hexdigest())
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(run,checks))
after=repository_source_identity(root);assert source==after
r=dict(schema_version=1,source=source,source_after=after,release_qualification=False,checks=rows);(out/'receipt.json').write_text(json.dumps(r,sort_keys=True,indent=2)+'\n');print(json.dumps({t['name']:t['observed_exit_code'] for t in rows}),flush=True);raise SystemExit(int(any(t['observed_exit_code'] for t in rows)))
