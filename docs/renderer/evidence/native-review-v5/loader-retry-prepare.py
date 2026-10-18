"""Prepare a fresh text retry with the verified ABI SONAME installed."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-text-loader-retry-90310e15');BRANCH='agent/native-text-loader-retry-v1655'
FIXED='90310e15b86bed558a79c8eb085d2a72092eab69'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old_owner=Path('/tmp/openui-native-text-retry-pipeline-v1650.py');old_text=old_owner.read_text()
config=ast.literal_eval(ast.parse(old_text).body[0].value)
priors=config['prior_pipelines']+[config['name']];assert len(priors)==len(set(priors))==35
assert all(json.loads((RAW/name/'receipt.json').read_bytes())['all_commands_terminal'] for name in priors)
for program in ['cargo','pixel_compare']:assert subprocess.run(['pgrep','-x',program],capture_output=True).returncode==1
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==FIXED
terminal=json.loads((RAW/'native-text-retry-pipeline-v1650/receipt.json').read_bytes())
build=json.loads((RAW/'native-text-retry-clean-v1649/build.json').read_bytes())
assert terminal['all_commands_terminal'] and build['all_commands_terminal']
assert terminal['source']==build['source']==build['source_after']==source
assert [r['observed_exit_code'] for r in build['steps']]==[0,0,0,0,0,0,0,0,127]
assert b'libopenui.so.0: cannot open shared object file' in (RAW/'native-text-retry-clean-v1649/run-c.log').read_bytes()
oldpriors=config['prior_pipelines'];files={}; originals={str(old_owner):sha(old_owner)}
replacements=[(config['root'],str(ROOT)),('agent/native-text-retry-v1649',BRANCH),
 ('native-text-retry-','native-text-loader-retry-'),('v1649','v1655'),('v1650','v1656'),
 (repr(oldpriors),repr(priors)),('awaiting-all-34-prior-whole-pipelines','awaiting-all-35-prior-whole-pipelines')]
preflight=[]
for kind in ['guards','build','consumer','matrices']:
 old=Path('/tmp')/f'openui-native-text-retry-{kind}-v1649.py';originals[str(old)]=sha(old);text=old.read_text()
 for before,after in replacements:text=text.replace(before,after)
 if kind=='build':
  before="   entry['binary_sha256']=hashlib.sha256(path.read_bytes()).hexdigest();entry['binary']=str(path)"
  after=before+"\n   if name=='ffi-build':\n    soname=out/'libopenui.so.0';assert not soname.exists();soname.symlink_to(path.name)\n    assert hashlib.sha256(soname.read_bytes()).hexdigest()==entry['binary_sha256']\n    entry['installed_soname']=str(soname);entry['soname_sha256']=entry['binary_sha256']"
  assert text.count(before)==1;text=text.replace(before,after)
  before="  with log.open('xb') as stream:"
  after="  if name in ('link-c','run-c','link-cpp','run-cpp'):\n   assert (out/'libopenui.so.0').resolve()==(out/'libopenui_ffi.so').resolve()\n  with log.open('xb') as stream:"
  assert text.count(before)==1;text=text.replace(before,after)
 tree=ast.parse(text);roots=[];branches=[]
 for n in tree.body:
  if isinstance(n,ast.Assign):
   for target in n.targets:
    if isinstance(target,ast.Name) and target.id in ['ROOT','root']:
     assert isinstance(n.value,ast.Call) and n.value.func.id=='Path';roots.append(ast.literal_eval(n.value.args[0]))
    if isinstance(target,ast.Name) and target.id=='BRANCH':branches.append(ast.literal_eval(n.value))
 assert roots==[str(ROOT)],(kind,roots)
 if kind=='guards':assert branches==[BRANCH]
 assert (ROOT/'tools/qualification/renderer_source_identity.py').is_file()
 new=Path('/tmp')/f'openui-native-text-loader-retry-{kind}-v1655.py';assert not new.exists();files[new]=text
 preflight.append(dict(kind=kind,root=str(ROOT),guard_branch=BRANCH if kind=='guards' else None))
config.update(root=str(ROOT),name='native-text-loader-retry-pipeline-v1656',prior_pipelines=priors,
 initial_state='awaiting-all-35-prior-whole-pipelines',scripts=[p.name for p in files],
 selections=['native-text-content-viewport-checks-v1626/receipt.json','native-text-retry-guards-v1649/receipt.json',
 'native-text-retry-clean-v1649/build.json','native-raster-fields-retry-consumer-v1559/receipt.json'])
config['stages']=[('guards',['/usr/bin/python3','/tmp/openui-native-text-loader-retry-guards-v1655.py'],'native-text-loader-retry-guards-v1655/receipt.json',True),
 ('native-build',['/usr/bin/python3','/tmp/openui-native-text-loader-retry-build-v1655.py'],'native-text-loader-retry-clean-v1655/build.json',True),
 ('native-application',['/usr/bin/python3','/tmp/openui-native-text-loader-retry-consumer-v1655.py'],'native-text-loader-retry-consumer-v1655/receipt.json',True)]+[
 (suite,['/usr/bin/python3','/tmp/openui-native-text-loader-retry-matrices-v1655.py',suite],f'native-text-loader-retry-clean-{suite}-v1655/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']]
body=old_text.split('\n',1)[1]
for before,after in replacements:body=body.replace(before,after)
owner=Path('/tmp/openui-native-text-loader-retry-pipeline-v1656.py');assert not owner.exists();files[owner]='CONFIG = '+repr(config)+'\n'+body
for p,text in files.items():ast.parse(text);p.write_text(text)
assert repository_source_identity(ROOT)==source
report=dict(schema_version=1,source=source,source_after=source,root=str(ROOT),branch=BRANCH,
 production_source_unchanged=True,prior_whole_owners=35,prior_whole_pipelines=priors,all_prior_owners_terminal=True,
 preflight=preflight,installed_abi_soname='libopenui.so.0',preserves_old_failed_source_probes_and_receipts=True,
 repeated_clean_baseline_fixed_guards_required=True,required_native_images=600,required_geometry_states=38400,
 required_stages=7,pixel_tolerance=0,javascript_executed_by_openui=False,release_qualification=False,
 scripts={str(p):sha(p) for p in files},original_probe_sha256=originals,probe_sha256=sha(Path(__file__)))
out=RAW/'native-text-loader-retry-prepared-v1655.json';assert not out.exists();out.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(prepared=True,source=FIXED,root=str(ROOT),owner=str(owner),prior_owners=35,receipt_sha256=sha(out))),flush=True)
