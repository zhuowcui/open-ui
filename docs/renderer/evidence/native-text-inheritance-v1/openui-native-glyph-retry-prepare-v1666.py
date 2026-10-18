"""Prepare a fresh glyph guard after verified interruption of previous owners."""
import ast,hashlib,json,subprocess
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-glyph-guard-retry-3b2e0d1f');BRANCH='agent/native-glyph-guard-retry-v1666'
FIXED='3b2e0d1f60b90813859c4c24325c7b0061dea29c'
witness=RAW/'owner-interruption-witness-v1664.json';assert hashlib.sha256(witness.read_bytes()).hexdigest()=='e3b0b081e43fdb29917909d2d8f03a643430bf17d971f1a3219211be612d29e2'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
old_owner=Path('/tmp/openui-native-glyph-guard-pipeline-v1658.py');old_text=old_owner.read_text();c=ast.literal_eval(ast.parse(old_text).body[0].value)
oldpriors=c['prior_pipelines'];priors=oldpriors+[c['name']];assert len(priors)==len(set(priors))==37
helper='''
WITNESS=RAW/'owner-interruption-witness-v1664.json'
assert hashlib.sha256(WITNESS.read_bytes()).hexdigest()=='e3b0b081e43fdb29917909d2d8f03a643430bf17d971f1a3219211be612d29e2'
INTERRUPTED={r['receipt']:r for r in json.loads(WITNESS.read_bytes())['owners']}
def verified_done(path):
 try:
  d=json.loads(path.read_bytes())
  if d['all_commands_terminal']:return True
  r=INTERRUPTED.get(str(path))
  if not r or hashlib.sha256(path.read_bytes()).hexdigest()!=r['receipt_sha256']:return False
  for pid in r['pids_verified_missing']:
   try:os.kill(pid,0);return False
   except ProcessLookupError:pass
  return True
 except (FileNotFoundError,json.JSONDecodeError):return False
'''
old=Path('/tmp/openui-native-glyph-guard-v1657.py');t=old.read_text().replace(c['root'],str(ROOT)).replace('agent/native-glyph-guard-v1657',BRANCH).replace('native-glyph-guard-v1657','native-glyph-guard-v1666').replace(repr(oldpriors),repr(priors))
oldloop="for name in PRIORS:assert json.loads((RAW/name/'receipt.json').read_bytes())['all_commands_terminal']"
assert t.count(oldloop)==1;t=t.replace(oldloop,helper+"\nfor name in PRIORS:assert verified_done(RAW/name/'receipt.json')")
probe=Path('/tmp/openui-native-glyph-guard-v1666.py');assert not probe.exists();ast.parse(t);probe.write_text(t)
c.update(root=str(ROOT),name='native-glyph-retry-pipeline-v1667',initial_state='awaiting-all-37-prior-whole-pipelines',prior_pipelines=priors,scripts=[probe.name],selections=c['selections']+['owner-interruption-witness-v1664.json'],stages=[('guards',['/usr/bin/python3',str(probe)],'native-glyph-guard-v1666/receipt.json',True)])
body=old_text.split('\n',1)[1];start=body.index('def complete(path):');end=body.index('while not all(complete(p)',start)
body=body[:start]+helper+'\ncomplete=verified_done\n\n'+body[end:]
owner=Path('/tmp/openui-native-glyph-retry-pipeline-v1667.py');assert not owner.exists();ast.parse('CONFIG = '+repr(c)+'\n'+body);owner.write_text('CONFIG = '+repr(c)+'\n'+body)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
report=dict(schema_version=1,root=str(ROOT),branch=BRANCH,source=FIXED,prior_whole_owners=37,interruption_witness_sha256=sha(witness),original_owner_receipts_unchanged=True,guard_only=True,parent_has_83_exact_losses=True,release_qualification=False,scripts={str(p):sha(p) for p in [probe,owner]})
p=RAW/'native-glyph-retry-prepared-v1666.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(prepared=True,owner=str(owner),receipt_sha256=sha(p))),flush=True)
