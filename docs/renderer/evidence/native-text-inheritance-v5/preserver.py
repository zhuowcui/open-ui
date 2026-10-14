"""Preserve complete matrices without promoting a failing release contract."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-text-inheritance-v5';INDEX=ROOT/'docs/renderer/generated/native-text-inheritance-v5.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();artifacts=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected:assert digest==expected
 assert p.read_bytes()==b
 q=OUT/name;assert not q.exists();q.write_bytes(gzip.compress(b,compresslevel=9,mtime=0) if compress else b)
 a=dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(q.read_bytes())==b;a.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 artifacts.append(a);return json.loads(b) if p.suffix=='.json' else None
owner=keep(RAW/'native-text-style-runtime-pipeline-v1678/receipt.json','owner-terminal.json','8a4885aeff98023bda333073c37030a8b4e4caa17d18ae88291d13ec2fc8bfcc');assert owner['all_commands_terminal'] and owner['source']==owner['source_after']
assert [r['observed_exit_code'] for r in owner['steps']]==[0,0,1,0,0,1,1]
for r in owner['steps']:keep(RAW/'native-text-style-runtime-pipeline-v1678'/(r['name']+'.log'),'owner-'+r['name']+'.log.gz',r['log_sha256'],compress=True)
audit=keep(RAW/'native-text-inheritance-matrix-audit-v1688.json','matrix-audit.json');assert audit['owner_receipt_sha256']==sha(RAW/'native-text-style-runtime-pipeline-v1678/receipt.json')
keep(Path('/tmp/openui-native-text-inheritance-matrix-audit-v1688.py'),'matrix-audit.py',audit['probe_sha256'])
wait=keep(RAW/'native-text-inheritance-audit-wait-v1705/receipt.json','audit-run-terminal.json');assert wait['observed_exit_code']==0 and wait['all_commands_terminal']
keep(Path('/tmp/openui-native-text-inheritance-audit-wait-v1705.py'),'audit-owner-wait.py',wait['probe_sha256'])
keep(RAW/'native-text-inheritance-audit-wait-v1705/audit.log','audit.log.gz',wait['log_sha256'],compress=True)
for suite in ['full','expanded','focused','primitive']:
 row=audit['suites'][suite];assert row['changed_comparisons']==row['exact_losses']==row['exact_gains']==0
 assert row['unchanged_nine_invariant_rows']==row['immutable_chromium_rows']==row['results']['total']
 keep(RAW/f'native-text-style-runtime-clean-{suite}-v1677/{suite}-summary.json',suite+'-summary.json.gz',row['summary_sha256'],compress=True)
 keep(RAW/f'native-text-style-runtime-{suite}-exit-v1677.json',suite+'-actual-exit.json',row['actual_exit_receipt_sha256'])
 exitdata=json.loads((RAW/f'native-text-style-runtime-{suite}-exit-v1677.json').read_bytes())
 keep(RAW/f'native-text-style-runtime-clean-{suite}-v1677.log',suite+'.log.gz',exitdata['log_sha256'],compress=True)
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=owner['source'],source_after=owner['source_after'],all_seven_stages_terminal=True,actual_stage_exits=[0,0,1,0,0,1,1],
 full_original_exact=21334,full_original_total=22924,full_original_different=1590,expanded_exact=22137,expanded_total=23728,expanded_different=1591,
 focused_exact=640,focused_total=640,primitive_exact=960,primitive_total=960,render_errors=0,
 all_48252_comparisons_nine_invariants_unchanged=True,all_chromium_reference_inputs_unchanged=True,original_rows_agree_in_expanded=True,
 addition_cases_four_profile_exact=200,addition_cases=201,original_residual_ids=882,expanded_residual_ids=883,formal_wpt_residual_ownership_unchanged=True,
 native_app_exact_images=0,native_app_images=600,native_geometry_exact=34560,native_geometry_total=38400,actual_c_cpp_images_matching_rust=400,
 native_api_source_integrated_into_umbrella=True,integration_code_identical_to_tested_source=True,clean_integration_build_still_required=True,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,new_release_states_admitted=0,release_qualification=False,
 artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for a in artifacts:
 assert sha(ROOT/a['path'])==a['sha256']
 if a.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress((ROOT/a['path']).read_bytes())).hexdigest()==a['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)),sha256=sha(INDEX),artifacts=len(artifacts),bytes=report['artifact_bytes'])),flush=True)
