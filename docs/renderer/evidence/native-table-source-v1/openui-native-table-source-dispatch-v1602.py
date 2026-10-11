"""Dispatch hardening once for the exact pushed native constructor candidate."""
import datetime,json,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=Path('/tmp/openui-native-table-source-dispatch-v1602.json');assert not OUT.exists()
branch='agent/native-table-source-v1597';source='e389b26ab28f259544e43c3052c895501e4c8686'
remote=subprocess.check_output(['git','ls-remote','origin','refs/heads/'+branch],cwd=ROOT,text=True).split();assert remote==[source,'refs/heads/'+branch]
runs=json.loads(subprocess.check_output(['gh','run','list','--workflow','hardening.yml','--branch',branch,'--event','workflow_dispatch','--limit','10','--json','databaseId,headSha,status,conclusion'],cwd=ROOT,text=True));assert runs==[]
r=json.loads((RAW/'native-table-source-checks-v1598/receipt.json').read_bytes());assert r['source']['commit']==source and r['source']==r['source_after'] and r['all_commands_terminal'];assert len(r['checks'])==11 and all(x['observed_exit_code']==0 for x in r['checks'])
result=subprocess.run(['gh','workflow','run','.github/workflows/hardening.yml','--ref',branch],cwd=ROOT)
record=dict(schema_version=1,branch=branch,source=source,dispatched_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),observed_exit_code=result.returncode,release_qualification=False,native_pixel_qualification=False)
OUT.write_text(json.dumps(record,sort_keys=True,indent=2)+'\n');assert result.returncode==0;print(json.dumps(record),flush=True)
