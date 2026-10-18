"""Audit a complete owner only after every rendering stage has terminated."""
import hashlib,json,subprocess,time
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
owner=RAW/'native-text-style-runtime-pipeline-v1678/receipt.json';OUT=RAW/'native-text-inheritance-audit-wait-v1705';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
while not json.loads(owner.read_bytes())['all_commands_terminal']:time.sleep(5)
log=OUT/'audit.log'
with log.open('xb') as stream:result=subprocess.run(['python3','/tmp/openui-native-text-inheritance-matrix-audit-v1688.py'],cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT)
report=dict(schema_version=1,all_commands_terminal=True,observed_exit_code=result.returncode,owner_receipt_sha256=sha(owner),log_sha256=sha(log),probe_sha256=sha(Path(__file__)),release_qualification=False)
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(report),flush=True);raise SystemExit(result.returncode)
