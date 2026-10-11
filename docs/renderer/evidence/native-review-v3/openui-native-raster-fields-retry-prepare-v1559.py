"""Correct the failed restore branch on a fresh root; preserve old evidence."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-raster-fields-retry-e0dc491e');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1');SOURCE='e0dc491e61e17ce4407ff2dd30289e741690572b';BRANCH='agent/native-raster-fields-retry-v1557'
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==SOURCE
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==SOURCE
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd='/dev/shm/openui-native-raster-fields-public-fb284c54',text=True).strip()=='5390b683c6b7898d79f36450f1034f0d6a89d37d';assert not subprocess.check_output(['git','status','--porcelain'],cwd='/dev/shm/openui-native-raster-fields-public-fb284c54')
previous=Path('/tmp/openui-native-raster-fields-public-pipeline-v1470.py').read_text();old_config=ast.literal_eval(ast.parse(previous).body[0].value)
table=ast.literal_eval(ast.parse(Path('/tmp/openui-native-table-progress-pipeline-v1548.py').read_text()).body[0].value);priors=table['prior_pipelines']+[table['name']];assert len(priors)==len(set(priors))==28
checks_path=RAW/'native-raster-fields-retry-checks-v1558/receipt.json';checks=json.loads(checks_path.read_bytes());assert checks['source']==checks['source_after']==source and checks['all_commands_terminal'];assert len(checks['checks'])==10 and all(x['observed_exit_code']==0 for x in checks['checks'])
paths=[];old_probe_hashes={};new_probe_hashes={}
for name in old_config['scripts']:
 p=Path('/tmp',name);s=p.read_text();old_probe_hashes[str(p)]=sha(p)
 s=s.replace('/dev/shm/openui-native-raster-fields-public-fb284c54',str(ROOT)).replace('native-raster-fields-public-','native-raster-fields-retry-').replace('v1468','v1559')
 if name.endswith('guards-v1468.py'):
  assert s.count("BRANCH = 'agent/native-raster-api-v1422'")==1;s=s.replace("BRANCH = 'agent/native-raster-api-v1422'",'BRANCH = '+repr(BRANCH))
  s=s.replace("assert source['clean'] and source['commit'] == FIXED", "assert source['clean'] and source['commit'] == FIXED\nassert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip() == FIXED")
 q=Path('/tmp',name.replace('native-raster-fields-public-','native-raster-fields-retry-').replace('v1468','v1559'));assert not q.exists();q.write_text(s);ast.parse(s);paths.append(q);new_probe_hashes[str(q)]=sha(q)
config=dict(old_config,root=str(ROOT),name='native-raster-fields-retry-pipeline-v1560',initial_state='awaiting-all-28-prior-whole-pipelines',prior_pipelines=priors,selections=old_config['selections']+['native-raster-fields-retry-checks-v1558/receipt.json'],scripts=[p.name for p in paths],stages=[(stage[0],[str(arg).replace('native-raster-fields-public-','native-raster-fields-retry-').replace('v1468','v1559') for arg in stage[1]],stage[2].replace('native-raster-fields-public-','native-raster-fields-retry-').replace('v1468','v1559'),stage[3]) for stage in old_config['stages']])
body=previous[previous.index('\nimport hashlib'):]
body=body.replace("    current_source_runtime_qualification_pending=True)","    current_source_runtime_qualification_pending=True, restore_branch_corrected=True, fresh_source_root=True, old_failure_root_and_probes_unchanged=True, restore_branch='agent/native-raster-fields-retry-v1557')")
p=Path('/tmp/openui-native-raster-fields-retry-pipeline-v1560.py');assert not p.exists();p.write_text('CONFIG = '+repr(config)+body);ast.parse(p.read_text());paths.append(p)
report=dict(schema_version=1,source=source,source_after=source,branch=BRANCH,read_only_checks_passed=10,old_failure_root='/dev/shm/openui-native-raster-fields-public-fb284c54',old_failure_root_commit='5390b683c6b7898d79f36450f1034f0d6a89d37d',old_failure_root_and_probes_unchanged=True,source_bytes_unchanged_from_e0_candidate=True,restore_branch_corrected=True,original_guard_assertions_and_strict_capture_conditions_unchanged=True,cargo_commands_run=0,screenshots_generated=0,native_and_pixel_results_pending=True,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,pixel_tolerance=0,old_openui_pixels_are_provenance_only=True,javascript_executed_by_openui=False,public_native_rust_api_required=True,prior_whole_owners=28,required_stages=len(config['stages']),original_probe_sha256=old_probe_hashes,scripts={str(p):sha(p) for p in paths},checks_receipt_sha256=sha(checks_path),selection_receipt_sha256=sha(RAW/old_config['selections'][0]),probe_sha256=sha(Path(__file__)))
p=RAW/'native-raster-fields-retry-queue-prepared-v1559.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(path=str(p),sha256=sha(p),source=SOURCE,old_root_unchanged=True,prior_whole_owners=28,stages=len(config['stages']))),flush=True)
