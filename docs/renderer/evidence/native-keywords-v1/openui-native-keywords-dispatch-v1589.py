"""Dispatch hardening once for the exact pushed native constructor candidate."""
import datetime,json,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=Path('/tmp/openui-native-keywords-dispatch-v1589.json');assert not OUT.exists()
branch='agent/native-keywords-v1585';source='06e1f89a4a2e7a53465bceb780675383d9748464'
remote=subprocess.check_output(['git','ls-remote','origin','refs/heads/'+branch],cwd=ROOT,text=True).split();assert remote==[source,'refs/heads/'+branch]
runs=json.loads(subprocess.check_output(['gh','run','list','--workflow','hardening.yml','--branch',branch,'--event','workflow_dispatch','--limit','10','--json','databaseId,headSha,status,conclusion'],cwd=ROOT,text=True));assert runs==[]
r=json.loads((RAW/'native-keywords-checks-v1586/receipt.json').read_bytes());assert r['source']['commit']==source and r['source']==r['source_after'] and r['all_commands_terminal'];assert len(r['checks'])==13 and all(x['observed_exit_code']==0 for x in r['checks'])
result=subprocess.run(['gh','workflow','run','.github/workflows/hardening.yml','--ref',branch],cwd=ROOT)
record=dict(schema_version=1,branch=branch,source=source,dispatched_at_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),observed_exit_code=result.returncode,release_qualification=False,native_pixel_qualification=False)
OUT.write_text(json.dumps(record,sort_keys=True,indent=2)+'\n');assert result.returncode==0;print(json.dumps(record),flush=True)
