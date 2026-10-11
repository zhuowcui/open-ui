"""Verify the integrated native API on a clean current-head build."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1';HEAD='6def29f8ec6e10d87b5778df9dd1ed0157cf0cb4'
ROOT=Path('/dev/shm/openui-native-text-style-integration-6def29f8');BRANCH='agent/native-text-style-integration-v1709'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),HEAD],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();old_owner=Path('/tmp/openui-native-text-style-runtime-pipeline-v1678.py');text=old_owner.read_text();c=ast.literal_eval(ast.parse(text).body[0].value)
priors=c['prior_pipelines']+[c['name'],'native-intrinsic-snap-guard-pipeline-v1703'];assert len(priors)==len(set(priors))==41
for name in [c['name'],'native-intrinsic-snap-guard-pipeline-v1703']:assert json.loads((RAW/name/'receipt.json').read_bytes())['all_commands_terminal']
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==HEAD
assert not subprocess.check_output(['git','diff','41b616c3be5224b074e9d06ba6aa963731a1b265',HEAD,'--','bindings/rust','tools/style','examples/c_v02','include','.github'],cwd=ROOT)
old=Path('/tmp/openui-native-text-style-runtime-build-v1677.py');s=old.read_text().replace(c['root'],str(ROOT)).replace('native-text-style-runtime-clean-v1677','native-text-style-integration-clean-v1709').replace('41b616c3be5224b074e9d06ba6aa963731a1b265',HEAD)
s=s.replace('for prior_name in '+repr(c['prior_pipelines'])+':','for prior_name in '+repr(priors)+':')
build=Path('/tmp/openui-native-text-style-integration-build-v1709.py');assert not build.exists();ast.parse(s);build.write_text(s)
c.update(root=str(ROOT),commit=HEAD,name='native-text-style-integration-pipeline-v1710',initial_state='awaiting-all-41-prior-whole-pipelines',prior_pipelines=priors,scripts=[build.name],selections=['native-text-style-integration-v1708.json'],hard_stop_stages=['native-build'],
 stages=[('native-build',['/usr/bin/python3',str(build)],'native-text-style-integration-clean-v1709/build.json',True)])
body=text.split('\n',1)[1].replace('import hashlib\n','import hashlib\nimport fcntl\n',1)
body=body.replace("ROOT = Path(CONFIG['root'])","OWNER_LOCK = open('/tmp/openui-native-cargo-raster-owner.lock', 'a')\nfcntl.flock(OWNER_LOCK, fcntl.LOCK_EX)\nROOT = Path(CONFIG['root'])",1)
body=body.replace("expected_stages=7","expected_stages=1").replace("consumer_compile_correction_only=False, renderer_production_unchanged_from_applied_renderer=False","consumer_compile_correction_only=False, renderer_production_unchanged_from_applied_renderer=True")
owner=Path('/tmp/openui-native-text-style-integration-pipeline-v1710.py');assert not owner.exists();s='CONFIG = '+repr(c)+'\n'+body;ast.parse(s);owner.write_text(s)
p=RAW/'native-text-style-integration-prepared-v1709.json';assert not p.exists();p.write_text(json.dumps(dict(schema_version=1,source=source,root=str(ROOT),branch=BRANCH,prior_whole_owners=41,source_code_matches_tested_41=True,required_build_stages=13,required_workspace_packages_cleaned=18,whole_owner_lock_preserves_gaps=True,scripts={str(p):sha(p) for p in [build,owner]},release_qualification=False,probe_sha256=sha(Path(__file__))),sort_keys=True,indent=2)+'\n');print(json.dumps(dict(prepared=True,head=HEAD,receipt_sha256=sha(p))),flush=True)
