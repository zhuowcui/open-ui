"""Preflight the complete named style inventory before any new native execution."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-text-style-qualification-41b616c3');BRANCH='agent/native-text-style-qualification-v1671';FIXED='41b616c3be5224b074e9d06ba6aa963731a1b265'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
p=Path('/tmp/openui-native-text-inheritance-pipeline-v1670.py');t=p.read_text();c=ast.literal_eval(ast.parse(t).body[0].value)
assert not (RAW/c['name']).exists(), 'old preparation was already launched'
originals={str(p):sha(p)};files={}
replacements=[(c['root'],str(ROOT)),('agent/native-text-inheritance-queue-v1669',BRANCH),('native-text-inheritance-','native-text-style-qualification-'),('v1669','v1671'),('v1670','v1672')]
for kind in ['guards','build','consumer','matrices']:
 old=Path('/tmp')/f'openui-native-text-inheritance-{kind}-v1669.py';originals[str(old)]=sha(old);text=old.read_text()
 for before,after in replacements:text=text.replace(before,after)
 if kind=='guards':
  before="expected=sorted('tests::'+name for name in re.findall(r'fn (native_inheritance[^ (]+)\\(',source_text))"
  after="expected=sorted('tests::'+name for name in re.findall(r'#\\[test\\]\\s*fn ([a-zA-Z0-9_]+)\\(',source_text) if 'native_inheritance' in name)"
  assert text.count(before)==1;text=text.replace(before,after)
  text=text.replace("selected_native_test_names=SELECTED,", "selected_native_test_names=SELECTED, public_native_text_style_test='native_text_replacement_inherits_authored_fonts_through_rust_callbacks', inherited_style_inventory_includes_suffix_matches=True,")
 tree=ast.parse(text);roots=[];branches=[]
 for n in tree.body:
  if isinstance(n,ast.Assign):
   for target in n.targets:
    if isinstance(target,ast.Name) and target.id in ['root','ROOT']:roots.append(ast.literal_eval(n.value.args[0]))
    if isinstance(target,ast.Name) and target.id=='BRANCH':branches.append(ast.literal_eval(n.value))
 assert roots==[str(ROOT)]
 if kind=='guards':assert branches==[BRANCH]
 new=Path('/tmp')/f'openui-native-text-style-qualification-{kind}-v1671.py';assert not new.exists();files[new]=text
c.update(root=str(ROOT),name='native-text-style-qualification-pipeline-v1672',scripts=[p.name for p in files])
c['stages']=[(name,[value.replace('native-text-inheritance-','native-text-style-qualification-').replace('v1669','v1671') for value in cmd],receipt.replace('native-text-inheritance-','native-text-style-qualification-').replace('v1669','v1671'),terminal) for name,cmd,receipt,terminal in c['stages']]
body=t.split('\n',1)[1]
for before,after in replacements:body=body.replace(before,after)
owner=Path('/tmp/openui-native-text-style-qualification-pipeline-v1672.py');assert not owner.exists();files[owner]='CONFIG = '+repr(c)+'\n'+body
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
for p,text in files.items():ast.parse(text);p.write_text(text)
import re
source_text=(ROOT/'bindings/rust/openui-engine/src/lib.rs').read_text();inventory=sorted('tests::'+name for name in re.findall(r'#\[test\]\s*fn ([a-zA-Z0-9_]+)\(',source_text) if 'native_inheritance' in name);assert len(inventory)==9
report=dict(schema_version=1,source=source,root=str(ROOT),branch=BRANCH,baseline='25322be849f1f0523a6e66291a567d0758931160',
 complete_named_style_inventory=inventory,old_preparation_unlaunched=True,old_probes_preserved=True,old_probes_sha256=originals,
 scripts={str(p):sha(p) for p in files},prior_whole_owners=38,required_build_stages=13,required_native_images=600,required_geometry_states=38400,required_matrices=4,
 release_qualification=False,probe_sha256=sha(Path(__file__)))
p=RAW/'native-text-style-qualification-prepared-v1671.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(prepared=True,source=FIXED,style_guard_count=len(inventory),receipt_sha256=sha(p))),flush=True)
