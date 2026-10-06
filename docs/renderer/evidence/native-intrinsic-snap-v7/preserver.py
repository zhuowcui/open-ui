"""Preserve complete matrices without promoting a failing release contract."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-intrinsic-snap-v7';INDEX=ROOT/'docs/renderer/generated/native-intrinsic-snap-v7.json'
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
owner=keep(RAW/'native-intrinsic-snap-pipeline-v1717/receipt.json','owner-terminal.json','c152f0e0b4804c79e1bc54dd5ca4ee4b1d2662c3d9156e5d563af1c1770ce55a');assert owner['all_commands_terminal'] and owner['source']==owner['source_after']
assert [r['observed_exit_code'] for r in owner['steps']]==[0,0,1,0,0,1,1]
for r in owner['steps']:keep(RAW/'native-intrinsic-snap-pipeline-v1717'/(r['name']+'.log'),'owner-'+r['name']+'.log.gz',r['log_sha256'],compress=True)
audit=keep(RAW/'native-intrinsic-snap-matrix-audit-v1744.json','matrix-audit.json');assert audit['owner_receipt_sha256']==sha(RAW/'native-intrinsic-snap-pipeline-v1717/receipt.json')
keep(Path('/tmp/openui-native-intrinsic-snap-matrix-audit-v1744.py'),'matrix-audit.py',audit['probe_sha256'])
for suite in ['full','expanded','focused','primitive']:
 row=audit['suites'][suite];assert row['exact_gains']==0 and row['exact_losses']==(21 if suite in ['full','expanded'] else 0)
 assert row['immutable_chromium_rows']==row['results']['total'] and row['unchanged_nine_invariant_rows']==row['results']['total']-row['changed_comparisons']
 keep(RAW/f'native-intrinsic-snap-clean-{suite}-v1716/{suite}-summary.json',suite+'-summary.json.gz',row['summary_sha256'],compress=True)
 keep(RAW/f'native-intrinsic-snap-{suite}-exit-v1716.json',suite+'-actual-exit.json',row['actual_exit_receipt_sha256'])
 exitdata=json.loads((RAW/f'native-intrinsic-snap-{suite}-exit-v1716.json').read_bytes())
 keep(RAW/f'native-intrinsic-snap-clean-{suite}-v1716.log',suite+'.log.gz',exitdata['log_sha256'],compress=True)
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=owner['source'],source_after=owner['source_after'],all_seven_stages_terminal=True,actual_stage_exits=[0,0,1,0,0,1,1],
 full_original_exact=21313,full_original_total=22924,full_original_different=1611,expanded_exact=22116,expanded_total=23728,expanded_different=1612,
 focused_exact=640,focused_total=640,primitive_exact=960,primitive_total=960,render_errors=0,
 all_48252_chromium_inputs_unchanged=True, changed_original_rows=27, changed_expanded_rows=27, exact_original_losses=21, exact_expanded_losses=21, exact_gains=0, candidate_rejected_for_exact_regressions=True,all_chromium_reference_inputs_unchanged=True,original_rows_agree_in_expanded=True,
 addition_cases_four_profile_exact=200,addition_cases=201,original_residual_ids=897,expanded_residual_ids=898,formal_wpt_residual_ownership_unchanged=True,
 native_app_exact_images=0,native_app_images=600,native_geometry_exact=38400,native_geometry_total=38400,actual_c_cpp_images_matching_rust=400,
 candidate_applied_to_umbrella=False, formal_new_loss_root_causes_still_require_review=True, geometry_gain_does_not_waive_pixel_losses=True,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,new_release_states_admitted=0,release_qualification=False,
 artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for a in artifacts:
 assert sha(ROOT/a['path'])==a['sha256']
 if a.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress((ROOT/a['path']).read_bytes())).hexdigest()==a['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)),sha256=sha(INDEX),artifacts=len(artifacts),bytes=report['artifact_bytes'])),flush=True)
