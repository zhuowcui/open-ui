"""Audit complete candidate images against the accepted immutable Chromium evidence."""
import collections,hashlib,json,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1';OUT=RAW/'native-glyph-current-matrix-audit-v1832.json';assert not OUT.exists()
sys.path.insert(0,str(MAIN/'tools/qualification'));from residuals import canonical_sha256
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();load=lambda p:json.loads(p.read_bytes())
ownerpath=RAW/'native-glyph-current-pipeline-v1830/receipt.json';owner=load(ownerpath);assert owner['all_commands_terminal'] and len(owner['steps'])==8 and not owner.get('failure')
invariants=['openui_png_sha256','openui_rgba_sha256','chromium_png_sha256','chromium_rgba_sha256','chromium_oracle_identity_sha256','chromium_oracle_rgba_sha256','status','mismatched_pixels','diff_signature'];oracle=invariants[2:6]
report=dict(schema_version=1,source=owner['source'],source_after=owner['source_after'],all_commands_terminal=True,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,javascript_executed_by_openui=False,owner_receipt_sha256=sha(ownerpath),probe_sha256=sha(Path(__file__)),suites={},changes={},all_chromium_inputs_unchanged=True,formal_wpt_ownership_unchanged=True)
maps={}
for suite in ['focused','primitive','full','expanded']:
 beforepath=RAW/('native-keywords-capture-clean-'+suite+'-v1785')/(suite+'-summary.json');afterpath=RAW/('native-glyph-current-clean-'+suite+'-v1828')/(suite+'-summary.json');before,after=load(beforepath),load(afterpath)
 assert before['source']==before['source_after'] and before['source']['commit']=='7d6ffabfea1f72c90f97475488dba8a8058ac4c7';assert after['source']==after['source_after']==owner['source'] and after['source']['clean']
 for d in [before,after]:
  assert d['results']['profile_result_sha256']==canonical_sha256([p['result_sha256'] for p in d['profiles']])
  for p in d['profiles']:assert len(p['tests'])==p['total'] and p['result_sha256']==canonical_sha256(p['tests'])
 flat=lambda d:{(p['profile'],r['id']):r for p in d['profiles'] for r in p['tests']};a,b=flat(before),flat(after);assert a.keys()==b.keys();maps[suite]=b;changes=[];gains=losses=unchanged=0
 for key,row in b.items():
  old=a[key];assert all(old[k]==row[k] for k in oracle),(suite,key,'immutable Chromium input changed');same=all(old[k]==row[k] for k in invariants);unchanged+=same
  if not same:
   gain=old['status']!='exact' and row['status']=='exact';loss=old['status']=='exact' and row['status']!='exact';gains+=gain;losses+=loss
   changes.append(dict(profile=key[0],id=key[1],prior_status=old['status'],status=row['status'],prior_mismatched_pixels=old['mismatched_pixels'],mismatched_pixels=row['mismatched_pixels'],exact_gain=gain,exact_loss=loss,prior_openui_png_sha256=old['openui_png_sha256'],openui_png_sha256=row['openui_png_sha256'],prior_openui_rgba_sha256=old['openui_rgba_sha256'],openui_rgba_sha256=row['openui_rgba_sha256'],chromium_png_sha256=row['chromium_png_sha256'],chromium_oracle_identity_sha256=row['chromium_oracle_identity_sha256'],reviewed_root_cause=False))
 actualpath=RAW/('native-glyph-current-'+suite+'-exit-v1828.json');actual=load(actualpath)
 report['suites'][suite]=dict(results=after['results'],prior_results=before['results'],actual_exit=actual['observed_exit_code'],summary_sha256=sha(afterpath),prior_summary_sha256=sha(beforepath),actual_exit_receipt_sha256=sha(actualpath),changed_comparisons=len(changes),exact_gains=gains,exact_losses=losses,unchanged_nine_invariant_rows=unchanged,immutable_chromium_rows=len(b),residual_ids=len({r['id'] for r in b.values() if r['status']!='exact'}));report['changes'][suite]=changes
assert all(all(row[k]==maps['expanded'][key][k] for k in invariants) for key,row in maps['full'].items());additions={k:v for k,v in maps['expanded'].items() if k not in maps['full']};assert len(additions)==804;groups=collections.defaultdict(list)
for (_,test),row in additions.items():groups[test].append(row)
assert len(groups)==201 and all(len(rows)==4 for rows in groups.values());report.update(original_rows_agree_in_expanded=22924,addition_cases=201,addition_cases_four_profile_exact=sum(all(r['status']=='exact' for r in rows) for rows in groups.values()),candidate_has_exact_regressions=any(v['exact_losses'] for v in report['suites'].values()))
OUT.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'suites':{k:{n:v[n] for n in ['results','changed_comparisons','exact_gains','exact_losses','residual_ids']} for k,v in report['suites'].items()},'addition_cases_four_profile_exact':report['addition_cases_four_profile_exact'],'sha256':sha(OUT)}))
