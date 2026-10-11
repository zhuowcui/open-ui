"""Audit completed intrinsic candidate against immutable Chromium and the applied renderer."""
import hashlib, json, sys
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui'); RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0,str(ROOT/'tools/qualification'))
from residuals import canonical_sha256
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
load=lambda p:json.loads(p.read_bytes())
owner_path=RAW/'native-intrinsic-cache-guard-pipeline-v1469/receipt.json';owner=load(owner_path)
assert owner['all_commands_terminal'] and owner['state']=='complete-requires-results-review'
assert [s['observed_exit_code'] for s in owner['steps']]==[0,0,0,1,1,0,0,1,1]
build_path=RAW/'native-intrinsic-cache-guard-clean-v1468/build.json';build=load(build_path)
source=build['source'];assert source['clean'] and source['commit']=='a6d386e48d864a5769ebdadd8bc8dea46e4eff21'
assert source==build['source_after']==owner['source']==owner['source_after']
assert len(build['steps'])==11 and all(s['observed_exit_code']==0 for s in build['steps'])
for step in build['steps']:
 assert sha(build_path.parent/(step['name']+'.log'))==step['log_sha256']
 if 'binary' in step:assert sha(Path(step['binary']))==step['binary_sha256']
binary=next(s for s in build['steps'] if s['name']=='pixel-build')
invariants=['openui_png_sha256','openui_rgba_sha256','chromium_png_sha256','chromium_rgba_sha256','chromium_oracle_identity_sha256','chromium_oracle_rgba_sha256','status','mismatched_pixels','diff_signature']
oracles=['chromium_png_sha256','chromium_rgba_sha256','chromium_oracle_identity_sha256','chromium_oracle_rgba_sha256']
suites={};original={};additions={};files={str(owner_path):sha(owner_path),str(build_path):sha(build_path)}
for suite,total,profiles in [('focused',640,40),('primitive',960,40),('full',22924,4),('expanded',23728,4)]:
 path=RAW/f'native-intrinsic-cache-guard-clean-{suite}-v1468/{suite}-summary.json'
 prior_path=RAW/f'native-event-targets-clean-{suite}-v1456/{suite}-summary.json'
 current,prior=load(path),load(prior_path)
 exit_path=RAW/f'native-intrinsic-cache-guard-{suite}-exit-v1468.json'; observed=load(exit_path)
 assert current['source']==current['source_after']==source==current['openui']['build_identity']['source']
 assert current['evidence']['source_unchanged'] and current['evidence']['binary_unchanged'] and current['evidence']['tolerance_pixels']==0
 assert current['openui']['binary_sha256']==binary['binary_sha256'] and current['complete_contract_scope']
 assert current['raster']['backend_selection']=='explicit-immutable' and current['openui']['raster_backend_identity']['backend']=='cpu-skia'
 for key in ['chromium','font_byte_hashes','resource_hashes','raster','contract_sha256','manifest_scope']:assert current[key]==prior[key],key
 assert current['id_manifest']['sha256']==prior['id_manifest']['sha256']==sha(Path(current['id_manifest']['path']))
 assert len(current['profiles'])==len(prior['profiles'])==profiles
 counts=dict(rows=0,all_nine_invariants_unchanged=0,improved_to_exact=0,lost_exactness=0,fewer_differing_pixels=0,more_differing_pixels=0)
 changes=[]
 for before_profile,profile in zip(prior['profiles'],current['profiles'],strict=True):
  for key in ['profile','device_scale','logical_size_css_px','physical_size_px','ordered_id_sha256']:assert profile[key]==before_profile[key],key
  assert canonical_sha256(profile['tests'])==profile['result_sha256']
  for before,row in zip(before_profile['tests'],profile['tests'],strict=True):
   assert before['id']==row['id'];assert all(before.get(k)==row.get(k) for k in oracles)
   counts['rows']+=1
   identical=all(before.get(k)==row.get(k) for k in invariants);counts['all_nine_invariants_unchanged']+=int(identical)
   gained=before['status']!='exact' and row['status']=='exact';lost=before['status']=='exact' and row['status']!='exact'
   counts['improved_to_exact']+=int(gained);counts['lost_exactness']+=int(lost)
   counts['fewer_differing_pixels']+=int(row['mismatched_pixels']<before['mismatched_pixels']);counts['more_differing_pixels']+=int(row['mismatched_pixels']>before['mismatched_pixels'])
   if not identical:changes.append(dict(profile=profile['profile'],id=row['id'],before_status=before['status'],after_status=row['status'],before_mismatched_pixels=before['mismatched_pixels'],after_mismatched_pixels=row['mismatched_pixels'],lost_exactness=lost,gained_exactness=gained,mismatch_bounds=row.get('diff_signature',{}).get('mismatch_bounds'),connected_region_count=row.get('diff_signature',{}).get('connected_region_count'),channel_deltas=row.get('diff_signature',{}).get('channel_deltas')))
   identity=(profile['profile'],row['id']);fingerprint=canonical_sha256({k:row.get(k) for k in invariants})
   if suite=='full':original[identity]=fingerprint
   elif suite=='expanded':
    if identity in original:assert original[identity]==fingerprint
    else:additions.setdefault(row['id'],[]).append(row['status'])
 assert counts['rows']==total==current['results']['total']
 assert current['results']['errors']==0
 actual=int(current['results']['different']>0);assert observed['observed_exit_code']==actual==next(s for s in owner['steps'] if s['name']==suite)['observed_exit_code']
 log=RAW/f'native-intrinsic-cache-guard-clean-{suite}-v1468.log';assert sha(log)==observed['log_sha256']
 files.update({str(path):sha(path),str(prior_path):sha(prior_path),str(exit_path):sha(exit_path),str(log):sha(log)})
 suites[suite]=dict(results={k:current['results'][k] for k in ['total','exact','different','errors']},actual_exit=actual,counts=counts,changes=changes)
 del current,prior
assert len(original)==22924 and len(additions)==201 and all(len(states)==4 for states in additions.values())
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=True,release_qualification=False,api_checkpoint_ready_for_umbrella=False,applied_to_umbrella=False,reason='Exact losses in complete matrices and incomplete consuming-app capture gate',suites=suites,addition_cases=201,addition_cases_four_profile_exact=sum(all(s=='exact' for s in states) for states in additions.values()),chromium_oracle_invariants_unchanged=48252,original_rows_identical_in_expanded=22924,file_hashes=files,cargo_commands_run=0,screenshots_generated=0,pixel_tolerance=0,javascript_executed_by_openui=False,probe_sha256=sha(Path(__file__)))
out=RAW/'native-intrinsic-complete-audit-v1510.json';assert not out.exists();out.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(path=str(out),sha256=sha(out),suites={name:{k:value for k,value in data.items() if k!='changes'} for name,data in suites.items()},addition_cases_four_profile_exact=report['addition_cases_four_profile_exact'],applied_to_umbrella=False)),flush=True)
