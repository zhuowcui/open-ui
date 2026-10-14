"""Review the terminal original census while the whole owner runs expanded."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');NATIVE=Path('/dev/shm/openui-native-inline-replaced-80e71181');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0,str(NATIVE/'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import canonical_sha256
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest();load=lambda p:json.loads(p.read_bytes())
build_path=RAW/'native-inline-replaced-clean-v1528/build.json';build=load(build_path);source=build['source'];assert build['all_commands_terminal'] and source==build['source_after'] and source['clean'] and source['commit']=='c92e2d08163779f5e55784f14e556ecaa9d16e7f';assert repository_source_identity(NATIVE)==source
binary=next(r for r in build['steps'] if r['name']=='pixel-build');assert binary['observed_exit_code']==0 and sha(Path(binary['binary']))==binary['binary_sha256']
p=RAW/'native-inline-replaced-clean-expanded-v1528/expanded-summary.json';oldp=RAW/'native-event-targets-clean-expanded-v1456/expanded-summary.json';exitp=RAW/'native-inline-replaced-expanded-exit-v1528.json';logp=RAW/'native-inline-replaced-clean-expanded-v1528.log'
current,prior,observed=load(p),load(oldp),load(exitp);assert observed['observed_exit_code']==1 and sha(logp)==observed['log_sha256'];assert current['source']==current['source_after']==source==current['openui']['build_identity']['source'];assert current['complete_contract_scope'] and current['evidence']['source_unchanged'] and current['evidence']['binary_unchanged'] and current['evidence']['tolerance_pixels']==0;assert current['openui']['binary_sha256']==binary['binary_sha256'];assert current['openui']['raster_backend_identity']['backend']=='cpu-skia' and current['raster']['backend_selection']=='explicit-immutable'
for k in ['chromium','font_byte_hashes','resource_hashes','raster','contract_sha256','manifest_scope']:assert current[k]==prior[k],k
assert current['id_manifest']['sha256']==prior['id_manifest']['sha256']==sha(Path(current['id_manifest']['path']))
assert len(current['profiles'])==len(prior['profiles'])==4
chromium_keys=['chromium_png_sha256','chromium_rgba_sha256','chromium_oracle_identity_sha256','chromium_oracle_rgba_sha256'];native_keys=['openui_png_sha256','openui_rgba_sha256','status','mismatched_pixels','diff_signature'];changed=[];counts={};unchanged=0;all_rows=[];profile_results=[]
for before_profile,profile in zip(prior['profiles'],current['profiles'],strict=True):
 for k in ['profile','device_scale','logical_size_css_px','physical_size_px','ordered_id_sha256']:assert profile[k]==before_profile[k],k
 assert canonical_sha256(profile['tests'])==profile['result_sha256']
 counter=dict(total=0,exact=0,different=0,errors=0)
 for before,row in zip(before_profile['tests'],profile['tests'],strict=True):
  assert before['id']==row['id'];assert all(before.get(k)==row.get(k) for k in chromium_keys)
  counter['total']+=1;counter[row['status']]+=1
  if all(before.get(k)==row.get(k) for k in native_keys):unchanged+=1
  else:
   entry=dict(profile=profile['profile'],id=row['id'],before_status=before['status'],after_status=row['status'],before_pixels=before.get('mismatched_pixels'),after_pixels=row.get('mismatched_pixels'),before_native_rgba_sha256=before.get('openui_rgba_sha256'),after_native_rgba_sha256=row.get('openui_rgba_sha256'))
   if before['status']=='exact' and row['status']!='exact':entry['kind']='exact-loss'
   elif before['status']!='exact' and row['status']=='exact':entry['kind']='exact-gain'
   elif row.get('mismatched_pixels',0)>before.get('mismatched_pixels',0):entry['kind']='worse-existing-difference'
   else:entry['kind']='changed-existing-difference'
   changed.append(entry)
  all_rows.append(row)
 profile_results.append(dict(profile=profile['profile'],**counter))
 for k,v in counter.items():counts[k]=counts.get(k,0)+v
assert all(counts[k]==current['results'][k] for k in counts);assert current['results']['profile_result_sha256']==canonical_sha256([p['result_sha256'] for p in current['profiles']]);assert counts['total']==23728 and counts['errors']==0;assert repository_source_identity(NATIVE)==source
kinds={k:sum(r['kind']==k for r in changed) for k in ['exact-gain','exact-loss','worse-existing-difference','changed-existing-difference']}
owner_path=RAW/'native-inline-replaced-pipeline-v1529/receipt.json';owner=load(owner_path);assert owner['all_commands_terminal'] and len(owner['steps'])==7 and [r['observed_exit_code'] for r in owner['steps']]==[0,0,0,0,0,1,1]
original_path=RAW/'native-inline-replaced-clean-full-v1528/full-summary.json';original=load(original_path);assert original['source']==original['source_after']==source
original_unchanged=0;addition_rows=[]
for ep,op in zip(current['profiles'],original['profiles'],strict=True):
 by_id={r['id']:r for r in ep['tests']}
 for row in op['tests']:
  compare=by_id.pop(row['id']);assert all(compare.get(k)==row.get(k) for k in chromium_keys+native_keys);original_unchanged+=1
 addition_rows.extend((ep['profile'],r) for r in by_id.values())
assert original_unchanged==22924 and len(addition_rows)==804
addition_ids={r['id'] for _,r in addition_rows};assert len(addition_ids)==201
addition_exact=sum(all(r['status']=='exact' for _,r in addition_rows if r['id']==id) for id in addition_ids)
report=dict(schema_version=1,source=source,source_after=source,applied_to_umbrella=False,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,expanded_gate_passed=False,pixel_tolerance=0,javascript_executed_by_openui=False,public_native_rust_api_required=True,all_scoped_census_commands_terminal=True,whole_pipeline_terminal=True,expanded_pending=False,results=counts,prior_results=prior['results'],observed_exit_code=1,profiles=profile_results,all_23728_chromium_inputs_unchanged=True,unchanged_nine_invariant_comparisons=unchanged,changed_comparisons=len(changed),changed_kinds=kinds,changed=changed,all_22924_original_rows_identical_to_separate_census=True,addition_comparisons=804,addition_cases=201,addition_cases_exact_at_all_four_profiles=addition_exact,exact_regressions_forbid_promotion=kinds['exact-loss']>0,whole_owner_receipt_sha256=sha(owner_path),prior_audit_failure=dict(probe='/tmp/openui-native-inline-replaced-full-audit-v1555.py',probe_sha256=sha(Path('/tmp/openui-native-inline-replaced-full-audit-v1555.py')),actual_exit=1,reason='Results contain the canonical profile hash in addition to the five aggregate counts; the first audit compared unlike schemas.'),expanded_residual_ids=len({r['id'] for r in all_rows if r['status']!='exact'}),file_hashes={str(x):sha(x) for x in [build_path,p,oldp,exitp,logp,owner_path,original_path]},cargo_commands_run=0,screenshots_generated=0,probe_sha256=sha(Path(__file__)))
p=RAW/'native-inline-replaced-expanded-audit-v1561.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(path=str(p),sha256=sha(p),results=counts,changed=kinds,expanded_residual_ids=report['expanded_residual_ids'],profiles=profile_results)),flush=True)
