"""Prepare complete native API verification with unchanged Chromium inputs."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-text-inheritance-queue-41b616c3');BRANCH='agent/native-text-inheritance-queue-v1669'
FIXED='41b616c3be5224b074e9d06ba6aa963731a1b265';BASELINE='25322be849f1f0523a6e66291a567d0758931160'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old_owner=Path('/tmp/openui-native-text-loader-retry-pipeline-v1656.py');t=old_owner.read_text();c=ast.literal_eval(ast.parse(t).body[0].value)
prior_owner=ast.literal_eval(ast.parse(Path('/tmp/openui-native-glyph-retry-pipeline-v1667.py').read_text()).body[0].value)
priors=prior_owner['prior_pipelines']+[prior_owner['name']];assert len(priors)==len(set(priors))==38
assert json.loads((RAW/'native-glyph-retry-pipeline-v1667/receipt.json').read_bytes())['all_commands_terminal']
helper=Path('/tmp/openui-native-glyph-guard-v1666.py').read_text();a=helper.index('WITNESS=');b=helper.index('\nfor name in PRIORS:',a);helper=helper[a:b]
replacements=[(c['root'],str(ROOT)),('agent/native-text-loader-retry-v1655',BRANCH),
 ('native-text-loader-retry-','native-text-inheritance-'),('v1655','v1669'),('v1656','v1670'),
 ('90310e15b86bed558a79c8eb085d2a72092eab69',FIXED),('9fe1665dcf27df7541ec2983f353f642d4136d0e',BASELINE)]
files={};originals={str(old_owner):sha(old_owner)}
for kind in ['guards','build','consumer','matrices']:
 old=Path('/tmp')/f'openui-native-text-loader-retry-{kind}-v1655.py';text=old.read_text();originals[str(old)]=sha(old)
 for before,after in replacements:text=text.replace(before,after)
 tree=ast.parse(text)
 loops=[n for n in tree.body if isinstance(n,ast.For) and isinstance(n.iter,ast.List) and len(n.iter.elts)>30]
 if loops:
  assert len(loops)==1
  lines=text.splitlines(True);n=loops[0]
  text=''.join(lines[:n.lineno-1])+helper+'\nfor prior_name in '+repr(priors)+":\n assert verified_done(RAW/prior_name/'receipt.json')\n"+''.join(lines[n.end_lineno:])
 if kind=='guards':
  before="        ('fixed-public-native-conformance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'v02_conformance'], None),"
  addition="        ('fixed-public-native-text-inheritance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'native_text_style_inheritance', 'native_text_replacement_inherits_authored_fonts_through_rust_callbacks', '--', '--exact'], None),\n"
  assert before in text;text=text.replace(before,addition+before)
  start=text.index("        row, content = run('baseline-native-text-loader") if "        row, content = run('baseline-native-text-loader" in text else text.index("        row, content = run('baseline-native-text-inheritance-guard'")
  end=text.index('\n        save()',start)
  text=text[:start]+"        row, content = run('baseline-public-native-text-inheritance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'native_text_style_inheritance', 'native_text_replacement_inherits_authored_fonts_through_rust_callbacks', '--', '--exact'], BASELINE)\n        report['baseline_regression_reproduced'] = (row['observed_exit_code'] == 101 and not row['disk_guard_triggered'] and b'native_text_replacement_inherits_authored_fonts_through_rust_callbacks ... FAILED' in content and b'0 passed; 1 failed;' in content and b'authored text must inherit its native container font' in content)"+text[end:]
  marker="    report['observed_exit_code'] = 0"
  extra="""    row, content = run('fixed-native-inheritance-inventory', build_base + ['test', '--locked', '-p', 'openui-engine', '--lib', 'native_inheritance'], FIXED)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    row['observed_named_tests'] = sorted(value.decode() for value in re.findall(rb'^test ([^ ]+) \\.\\.\\. ok$', content, re.MULTILINE))
    expected_path = Path('/home/nero/code/open-ui/docs/renderer/generated/native-scroll-insets-v40.json')
    expected = json.loads(expected_path.read_bytes())['native_style_integration'] if 'native_style_integration' in json.loads(expected_path.read_bytes()) else None
    assert len(row['observed_named_tests']) > 0
"""
  # Validate the named inventory from the source rather than an obsolete count.
  extra=extra[:extra.index('    expected_path')]+"    source_text=(ROOT/'bindings/rust/openui-engine/src/lib.rs').read_text()\n    expected=sorted('tests::'+name for name in re.findall(r'fn (native_inheritance[^ (]+)\\(',source_text))\n    assert row['observed_named_tests']==expected and len(expected)>0\n"
  assert marker in text;text=text.replace(marker,extra+marker)
 if kind=='build':
  before=" ('ffi-build',base+['build','--locked','-p','openui-ffi','--features','linux'],'debug/libopenui_ffi.so'),"
  assert before in text;text=text.replace(before," ('inherited-build',base+['build','--locked','-p','openui','--example','native_inherited_styles'],'debug/examples/native_inherited_styles'),\n"+before)
  text=text.replace('assert len(steps)==12','assert len(steps)==13')
 if kind=='consumer':
  text=text.replace("len(build['steps']) == 12","len(build['steps']) == 13")
 tree=ast.parse(text);roots=[];branches=[]
 for n in tree.body:
  if isinstance(n,ast.Assign):
   for target in n.targets:
    if isinstance(target,ast.Name) and target.id in ['ROOT','root']:roots.append(ast.literal_eval(n.value.args[0]))
    if isinstance(target,ast.Name) and target.id=='BRANCH':branches.append(ast.literal_eval(n.value))
 assert roots==[str(ROOT)],(kind,roots)
 if kind=='guards':assert branches==[BRANCH]
 p=Path('/tmp')/f'openui-native-text-inheritance-{kind}-v1669.py';assert not p.exists();files[p]=text
c.update(root=str(ROOT),commit=FIXED,name='native-text-inheritance-pipeline-v1670',initial_state='awaiting-all-38-prior-whole-pipelines',prior_pipelines=priors,selections=['native-text-inheritance-source-v1668.json','native-glyph-guard-v1666/receipt.json','owner-interruption-witness-v1664.json','native-raster-fields-retry-consumer-v1559/receipt.json'],scripts=[p.name for p in files],stages=[('guards',['/usr/bin/python3','/tmp/openui-native-text-inheritance-guards-v1669.py'],'native-text-inheritance-guards-v1669/receipt.json',True),('native-build',['/usr/bin/python3','/tmp/openui-native-text-inheritance-build-v1669.py'],'native-text-inheritance-clean-v1669/build.json',True),('native-application',['/usr/bin/python3','/tmp/openui-native-text-inheritance-consumer-v1669.py'],'native-text-inheritance-consumer-v1669/receipt.json',True)]+[(suite,['/usr/bin/python3','/tmp/openui-native-text-inheritance-matrices-v1669.py',suite],f'native-text-inheritance-clean-{suite}-v1669/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']])
body=t.split('\n',1)[1]
for before,after in replacements:body=body.replace(before,after)
a=body.index('def complete(path):');b=body.index('while not all(complete(p)',a)
body=body[:a]+helper+'\ncomplete=verified_done\n\n'+body[b:]
owner=Path('/tmp/openui-native-text-inheritance-pipeline-v1670.py');assert not owner.exists();files[owner]='CONFIG = '+repr(c)+'\n'+body
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
for p,text in files.items():ast.parse(text);p.write_text(text)
report=dict(schema_version=1,source=source,baseline=BASELINE,root=str(ROOT),branch=BRANCH,prior_whole_owners=38,required_app_images=600,required_geometry_states=38400,required_matrices=4,required_build_stages=13,files={str(p):sha(p) for p in files},originals=originals,interrupted_owners_verified_absent=True,release_qualification=False)
p=RAW/'native-text-inheritance-prepared-v1669.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(prepared=True,source=FIXED,baseline=BASELINE,receipt_sha256=sha(p))),flush=True)
