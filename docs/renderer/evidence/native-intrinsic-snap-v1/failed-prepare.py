"""Prepare exact native width guards and a source-attributed complete queue."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
FIXED='727da10e580c9439f5db83d36bfc3e2340168207';BASE='e995e52f8650ff9286d6beb0618cdd4b75df789b'
ROOT=Path('/dev/shm/openui-native-intrinsic-snap-runtime-727da10e');BRANCH='agent/native-intrinsic-snap-runtime-v1694'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old_owner=Path('/tmp/openui-native-text-style-runtime-pipeline-v1678.py');text=old_owner.read_text();c=ast.literal_eval(ast.parse(text).body[0].value)
priors=c['prior_pipelines']+[c['name']];assert len(priors)==len(set(priors))==40
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==FIXED
for commit in [BASE,FIXED]:assert subprocess.check_output(['git','show',commit+':bindings/rust/openui/tests/native_intrinsic_snap.rs'],cwd=ROOT)
assert subprocess.check_output(['git','show',BASE+':bindings/rust/openui/tests/native_intrinsic_snap.rs'],cwd=ROOT)==(ROOT/'bindings/rust/openui/tests/native_intrinsic_snap.rs').read_bytes()
files={};originals={str(old_owner):sha(old_owner)}
old=Path('/tmp/openui-native-text-style-qualification-guards-v1675.py');s=old.read_text();originals[str(old)]=sha(old)
s=s.replace('/dev/shm/openui-native-text-style-complete-41b616c3',str(ROOT)).replace('native-text-style-qualification-guards-v1675','native-intrinsic-snap-guards-v1694')
s=s.replace('41b616c3be5224b074e9d06ba6aa963731a1b265',FIXED).replace('25322be849f1f0523a6e66291a567d0758931160',BASE).replace('agent/native-text-style-qualification-v1675',BRANCH)
t=ast.parse(s);old_priors=next(ast.literal_eval(n.iter) for n in t.body if isinstance(n,ast.For) and isinstance(n.target,ast.Name) and n.target.id=='prior_name')
s=s.replace('for prior_name in '+repr(old_priors)+':','for prior_name in '+repr(priors)+':')
old_cmd="row, content = run('baseline-public-native-text-inheritance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'native_text_style_inheritance', 'native_text_replacement_inherits_authored_fonts_through_rust_callbacks', '--', '--exact'], BASELINE)"
new_cmd="row, content = run('baseline-public-native-intrinsic-snap', build_base + ['test', '--locked', '-p', 'openui', '--test', 'native_intrinsic_snap', 'native_intrinsic_text_width_retains_fraction_through_rust_callback', '--', '--exact'], BASELINE)"
assert s.count(old_cmd)==1;s=s.replace(old_cmd,new_cmd)
s=s.replace("b'native_text_replacement_inherits_authored_fonts_through_rust_callbacks ... FAILED'", "b'native_intrinsic_text_width_retains_fraction_through_rust_callback ... FAILED'")
s=s.replace("b'authored text must inherit its native container font'", "b\"native intrinsic width must preserve Chromium's shaped fraction\"")
needle="        ('fixed-public-native-conformance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'v02_conformance'], None),"
assert needle in s
s=s.replace(needle,"        ('fixed-public-native-intrinsic-snap', build_base + ['test', '--locked', '-p', 'openui', '--test', 'native_intrinsic_snap', 'native_intrinsic_text_width_retains_fraction_through_rust_callback', '--', '--exact'], None),\n"+needle)
s=s.replace("public_native_rust_api=True, javascript_executed_by_openui=False)","public_native_rust_api=True, public_native_width_test='native_intrinsic_text_width_retains_fraction_through_rust_callback', javascript_executed_by_openui=False)")
guards=Path('/tmp/openui-native-intrinsic-snap-guards-v1694.py');assert not guards.exists();files[guards]=s
for kind in ['build','consumer','matrices']:
 old=Path('/tmp')/f'openui-native-text-style-runtime-{kind}-v1677.py';s=old.read_text();originals[str(old)]=sha(old)
 s=s.replace(c['root'],str(ROOT)).replace('native-text-style-runtime-','native-intrinsic-snap-').replace('v1677','v1694').replace('41b616c3be5224b074e9d06ba6aa963731a1b265',FIXED)
 if kind=='build':
  s=s.replace('for prior_name in '+repr(c['prior_pipelines'])+':','for prior_name in '+repr(priors)+':')
 p=Path('/tmp')/f'openui-native-intrinsic-snap-{kind}-v1694.py';assert not p.exists();files[p]=s
c.update(root=str(ROOT),commit=FIXED,name='native-intrinsic-snap-pipeline-v1695',initial_state='awaiting-all-40-prior-whole-pipelines',prior_pipelines=priors,
 scripts=[p.name for p in files],selections=['native-intrinsic-snap-source-v1692.json','native-intrinsic-snap-checks-v1693/receipt.json','owner-interruption-witness-v1664.json','native-raster-fields-retry-consumer-v1559/receipt.json'],
 stages=[('guards',['/usr/bin/python3',str(guards)],'native-intrinsic-snap-guards-v1694/receipt.json',True)]+[(name,[s.replace('native-text-style-runtime-','native-intrinsic-snap-').replace('v1677','v1694') for s in cmd],p.replace('native-text-style-runtime-','native-intrinsic-snap-').replace('v1677','v1694'),flag) for name,cmd,p,flag in c['stages'][1:]])
body=text.split('\n',1)[1].replace("baseline_commit='25322be849f1f0523a6e66291a567d0758931160'",'baseline_commit='+repr(BASE))
body=body.replace("root_cause_owner='shared Engine native text replacement and C facade'","root_cause_owner='shared intrinsic sizing discards shaped-width fractions before grid ceiling'")
# This lock covers the whole owner, including gaps between commands. The
# predecessor list still gates older owners that predate the lock.
body=body.replace('import hashlib\n','import hashlib\nimport fcntl\n',1)
body=body.replace("ROOT = Path(CONFIG['root'])", "OWNER_LOCK = open('/tmp/openui-native-cargo-raster-owner.lock', 'a')\nfcntl.flock(OWNER_LOCK, fcntl.LOCK_EX)\nROOT = Path(CONFIG['root'])",1)
owner=Path('/tmp/openui-native-intrinsic-snap-pipeline-v1695.py');assert not owner.exists();files[owner]='CONFIG = '+repr(c)+'\n'+body
for p,s in files.items():
 tree=ast.parse(s);assert str(ROOT) in s
 roots=[];assigned=set()
 for n in tree.body:
  if isinstance(n,ast.Assign):
   for target in n.targets:
    if isinstance(target,ast.Name):
     if isinstance(n.value,ast.BinOp) and isinstance(n.value.left,ast.Name) and n.value.left.id=='RAW':assert 'RAW' in assigned,(p.name,target.id)
     assigned.add(target.id)
     if target.id in ['ROOT','root'] and isinstance(n.value,ast.Call) and isinstance(n.value.args[0],ast.Constant):roots.append(ast.literal_eval(n.value.args[0]))
 if p!=owner:assert roots==[str(ROOT)],(p.name,roots)
 assert '41b616c3be5224b074e9d06ba6aa963731a1b265' not in s
 p.write_text(s)
report=dict(schema_version=1,source=source,root=str(ROOT),branch=BRANCH,prior_whole_owners=40,source_support_not_runtime_qualification=True,
 selected_next_owner=owner.name,prepared_glyph_owner_v1690_not_started=True,glyph_queue_requires_new_preparation_after_this_owner,
 whole_owner_lock_preserves_gaps=True,baseline_restore_branch_preflighted=True,baseline_and_fixed_public_test_bytes_identical=True,
 required_build_stages=13,required_native_images=600,required_geometry_states=38400,required_matrices=4,native_owner_not_started=True,
 source_unchanged=True,scripts={str(p):sha(p) for p in files},originals=originals,release_qualification=False,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,probe_sha256=sha(Path(__file__)))
p=RAW/'native-intrinsic-snap-prepared-v1694.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(prepared=True,source=FIXED,owner_not_started=True,scripts=len(files),receipt_sha256=sha(p))),flush=True)
