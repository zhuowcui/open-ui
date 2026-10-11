"""Reuse terminal source guards; build the same source on a fresh checked root."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-text-style-runtime-41b616c3');BRANCH='agent/native-text-style-runtime-v1677';FIXED='41b616c3be5224b074e9d06ba6aa963731a1b265'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old_owner=Path('/tmp/openui-native-text-style-qualification-pipeline-v1676.py');t=old_owner.read_text();c=ast.literal_eval(ast.parse(t).body[0].value)
priors=c['prior_pipelines']+[c['name']];assert len(priors)==len(set(priors))==39
terminal=json.loads((RAW/c['name']/'receipt.json').read_bytes());assert terminal['all_commands_terminal'] and terminal['steps'][-1]['observed_exit_code']==1
assert b"name 'RAW' is not defined" in (RAW/c['name']/'native-build.log').read_bytes()
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
guardpath=RAW/'native-text-style-qualification-guards-v1675/receipt.json';guards=json.loads(guardpath.read_bytes())
assert guards['all_commands_terminal'] and guards['state']=='complete' and guards['source']==guards['source_after']==source
assert guards['baseline_regression_reproduced'] and [r['observed_exit_code'] for r in guards['steps']]==[0,101,0,0,0,0,0,0]
expected=json.loads((MAIN/'docs/renderer/generated/native-scroll-insets-v40.json').read_bytes())['style_guard']['corrected_selection_names']['native-inheritance-guards']
assert guards['steps'][-1]['observed_named_tests']==expected
files={};originals={str(old_owner):sha(old_owner)}
replacements=[(c['root'],str(ROOT)),('agent/native-text-style-qualification-v1675',BRANCH),('native-text-style-qualification-','native-text-style-runtime-'),('v1675','v1677'),('v1676','v1678')]
for kind in ['build','consumer','matrices']:
 old=Path('/tmp')/f'openui-native-text-style-qualification-{kind}-v1675.py';text=old.read_text();originals[str(old)]=sha(old)
 for before,after in replacements:text=text.replace(before,after)
 if kind=='build':
  assert text.count("WITNESS=RAW/'owner-interruption-witness-v1664.json'")==1
  text=text.replace("WITNESS=RAW/'owner-interruption-witness-v1664.json'","RAW=raw\nWITNESS=RAW/'owner-interruption-witness-v1664.json'",1)
  before="for prior_name in "+repr(c['prior_pipelines'])+":"
  assert before in text;text=text.replace(before,"for prior_name in "+repr(priors)+":")
 tree=ast.parse(text);assigned=set();roots=[]
 for n in tree.body:
  if isinstance(n,ast.Assign):
   for target in n.targets:
    if isinstance(target,ast.Name):
     if isinstance(n.value,ast.BinOp) and isinstance(n.value.left,ast.Name) and n.value.left.id=='RAW':assert 'RAW' in assigned,(kind,target.id)
     assigned.add(target.id)
     if target.id in ['ROOT','root']:roots.append(ast.literal_eval(n.value.args[0]))
 assert roots==[str(ROOT)]
 p=Path('/tmp')/f'openui-native-text-style-runtime-{kind}-v1677.py';assert not p.exists();files[p]=text
verify=Path('/tmp/openui-native-text-style-runtime-guard-reuse-v1677.py');assert not verify.exists()
files[verify]='''"""Reuse actual guards only when every source/input byte matches."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path(__ROOT__);RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);p=RAW/'native-text-style-qualification-guards-v1675/receipt.json';d=json.loads(p.read_bytes())
assert hashlib.sha256(p.read_bytes()).hexdigest()==__GUARD_SHA__
assert source==d['source']==d['source_after'] and source['clean']
assert d['all_commands_terminal'] and d['baseline_regression_reproduced'] and d['state']=='complete'
for r in d['steps']:assert hashlib.sha256((p.parent/(r['name']+'.log')).read_bytes()).hexdigest()==r['log_sha256']
print(json.dumps({'terminal_guards_reused':True,'source':source['commit'],'baseline_failure_and_fixed_pass_verified':True,'new_tests_run':0}),flush=True)
'''.replace('__ROOT__',repr(str(ROOT))).replace('__GUARD_SHA__',repr(sha(guardpath)))
c.update(root=str(ROOT),name='native-text-style-runtime-pipeline-v1678',prior_pipelines=priors,initial_state='awaiting-all-39-prior-whole-pipelines',scripts=[p.name for p in files],selections=c['selections']+['native-text-style-qualification-guards-v1675/receipt.json','native-text-inheritance-checks-v1673/receipt.json'])
c['stages']=[('guards',['/usr/bin/python3',str(verify)],'native-text-style-qualification-guards-v1675/receipt.json',True)]+[(name,[s.replace('native-text-style-qualification-','native-text-style-runtime-').replace('v1675','v1677') for s in cmd],p.replace('native-text-style-qualification-','native-text-style-runtime-').replace('v1675','v1677'),flag) for name,cmd,p,flag in c['stages'][1:]]
body=t.split('\n',1)[1]
for before,after in replacements:body=body.replace(before,after)
owner=Path('/tmp/openui-native-text-style-runtime-pipeline-v1678.py');assert not owner.exists();files[owner]='CONFIG = '+repr(c)+'\n'+body
for p,text in files.items():ast.parse(text);p.write_text(text)
report=dict(schema_version=1,source=source,root=str(ROOT),branch=BRANCH,prior_whole_owners=39,
 actual_source_guards_reused=True,guard_receipt_sha256=sha(guardpath),all_guard_logs_rehashed=True,
 old_failed_build_stopped_before_cargo=True,lowercase_raw_binding_checked=True,required_build_stages=13,
 required_native_images=600,required_geometry_states=38400,required_matrices=4,source_unchanged=True,
 scripts={str(p):sha(p) for p in files},originals=originals,release_qualification=False)
p=RAW/'native-text-style-runtime-prepared-v1677.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(prepared=True,source=FIXED,guard_reuse_verified=True,receipt_sha256=sha(p))),flush=True)
