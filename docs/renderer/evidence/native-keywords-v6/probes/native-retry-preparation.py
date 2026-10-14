"""Prepare fresh native-only probes while the full pixel owner continues."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-keywords-native-7d6ffabf-v1787')
SOURCE='7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
SOCKETS=Path('/dev/shm/openui-native-keywords-chromium-temporary-v1787')
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
assert not ROOT.exists() and not SOCKETS.exists()
subprocess.run(['git','worktree','add','--detach',str(ROOT),SOURCE],cwd=MAIN,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);SOCKETS.mkdir()
sys.path.insert(0,str(MAIN/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==SOURCE
build_path=RAW/'native-keywords-current-retry-clean-v1779/build.json';b=json.loads(build_path.read_bytes());assert b['all_commands_terminal'] and b['source']==b['source_after']==source and len(b['steps'])==13 and all(s['observed_exit_code']==0 for s in b['steps'])
files={};parents={}
for kind in ['consumer','table-geometry','build-reuse']:
 p=Path('/tmp/openui-native-keywords-capture-'+kind+'-v1785.py');parents[str(p)]=sha(p)
 t=p.read_text().replace('native-keywords-capture-','native-keywords-native-retry-').replace('v1785','v1787')
 m=ast.parse(t);n=next(n for n in m.body if isinstance(n,ast.Assign) and any(isinstance(x,ast.Name) and x.id=='ROOT' for x in n.targets));s=ast.get_source_segment(t,n);assert t.count(s)==1;t=t.replace(s,'ROOT=Path('+repr(str(ROOT))+')')
 if kind=='consumer':
  anchor="env=capture.chrome_environment(str(chrome.parent),True,False)";assert t.count(anchor)==1;t=t.replace(anchor,anchor+"\nenv['TMPDIR']="+repr(str(SOCKETS)))
  anchor="adapted=original.replace(removed,'');exec(compile(adapted,str(capture_source),'exec'))";assert t.count(anchor)==1
  replacement="adapted=original.replace(removed,'')\nprofile_anchor=\"tempfile.mkdtemp(prefix='chrome-reference-', dir=STORE)\"\nassert adapted.count(profile_anchor)==1\nadapted=adapted.replace(profile_anchor,\"tempfile.mkdtemp(prefix='chrome-reference-', dir=Path(\"+repr("+repr(str(SOCKETS))+")+\"))\")\nexec(compile(adapted,str(capture_source),'exec'))"
  t=t.replace(anchor,replacement)
  t=t.replace("reference_adaptations=['Remove only the image natural-size assertion for a div case']","reference_adaptations=['Remove image natural-size assertion for a div case','Store Chromium profile and socket temporary files on a native Linux filesystem'],capture_visual_conditions_unchanged=True,chrome_profile_storage_correction_only=True")
 if kind=='table-geometry':
  t=t.replace('import hashlib, json, re','import hashlib, json, re, math')
  anchor='round(375*scale),round(667*scale)';assert t.count(anchor)==1;t=t.replace(anchor,'math.floor(375*scale+0.5),math.floor(667*scale+0.5)')
  t=t.replace('geometry_diagnostic_only=True,','geometry_diagnostic_only=True, viewport_rounding_matches_positive_winit_half_away_from_zero=True,')
 files[Path('/tmp/openui-native-keywords-native-retry-'+kind+'-v1787.py')]=t
p=Path('/tmp/openui-native-keywords-capture-pipeline-v1786.py');parents[str(p)]=sha(p);t=p.read_text();config=ast.literal_eval(ast.parse(t).body[0].value)
config.update(root=str(ROOT),name='native-keywords-native-retry-pipeline-v1788',initial_state='awaiting-confirmed-whole-census-owner-terminal',prior_pipelines=['native-keywords-capture-pipeline-v1786'],scripts=[p.name for p in files]+['openui-native-image-coverage-fieldsets-v1448.py'])
config['selections'] += ['native-keywords-capture-consumer-v1785/receipt.json','native-keywords-capture-table-geometry-v1785/receipt.json']
config['stages']=[(n,[part.replace('native-keywords-capture-','native-keywords-native-retry-').replace('v1785','v1787') for part in cmd],receipt.replace('native-keywords-capture-','native-keywords-native-retry-').replace('v1785','v1787'),flag) for n,cmd,receipt,flag in config['stages'][:3]]
body=t.split('\n',1)[1].replace('expected_stages=7','expected_stages=3')
# This owner does not compile; Chrome needs native Linux socket storage.
body=body.replace("TMPDIR='/mnt/e/openui-v02-qualification-d174ea0b/native-keywords-current-temporary-v1779'",'TMPDIR='+repr(str(SOCKETS)))
files[Path('/tmp/openui-native-keywords-native-retry-pipeline-v1788.py')]='CONFIG = '+repr(config)+'\n'+body
for p,t in files.items():
 assert not p.exists();m=ast.parse(t)
 if p.name.endswith('pipeline-v1788.py'):continue
 n=next(n for n in m.body if isinstance(n,ast.Assign) and any(isinstance(x,ast.Name) and x.id=='ROOT' for x in n.targets));assert ast.literal_eval(n.value.args[0])==str(ROOT)
 assert (ROOT/'tools/qualification/renderer_source_identity.py').is_file()
 if 'consumer' in p.name:
  assert "env['TMPDIR']="+repr(str(SOCKETS)) in t and 'chrome_profile_storage_correction_only=True' in t
 if 'table-geometry' in p.name:assert 'math.floor(375*scale+0.5)' in t
for p,t in files.items():p.write_text(t)
q=RAW/'native-keywords-native-retry-prepared-v1787.json';assert not q.exists()
report=dict(schema_version=1,source=source,source_after=source,completed_same_source_build_sha256=sha(build_path),fresh_native_root=str(ROOT),waiting_for_whole_census_owner='native-keywords-capture-pipeline-v1786',pipeline_not_launched=True,chromium_profile_and_tmp_on_native_linux_filesystem=True,c_viewport_rounding_follows_current_native_contract=True,no_reference_png_changed=True,reference_visual_capture_conditions_preserved=True,all_worker_source_roots_ast_verified=True,required_native_images=10,required_independent_chromium_processes=20,required_consecutive_screenshots=40,native_c_geometry_diagnostic_only=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,javascript_executed_by_openui=False,cargo_commands_run=0,screenshots_generated=0,scripts={str(p):sha(p) for p in files},adapted_from=parents,probe_sha256=sha(Path(__file__)))
q.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'receipt':str(q),'sha256':sha(q),'stages':3,'waiting_for_whole_census_owner':True,'not_launched':True}),flush=True)
