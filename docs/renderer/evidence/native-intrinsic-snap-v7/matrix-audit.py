"""Audit the complete native text/style source against the accepted renderer."""
import collections
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW/'native-intrinsic-snap-matrix-audit-v1744.json'
assert not OUT.exists()
sys.path.insert(0,str(ROOT/'tools/qualification'))
from residuals import canonical_sha256
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
load = lambda p: json.loads(p.read_bytes())
invariants = ['openui_png_sha256','openui_rgba_sha256','chromium_png_sha256','chromium_rgba_sha256',
              'chromium_oracle_identity_sha256','chromium_oracle_rgba_sha256','status','mismatched_pixels','diff_signature']
oracle = invariants[2:6]
index = load(ROOT/'docs/renderer/generated/native-event-targets-v1.json')
owner_path = RAW/'native-intrinsic-snap-pipeline-v1717/receipt.json'
owner = load(owner_path)
assert owner['all_commands_terminal'] and len(owner['steps']) == 7
assert [s['observed_exit_code'] for s in owner['steps']] == [0,0,1,0,0,1,1]
assert owner['source']['commit']=='727da10e580c9439f5db83d36bfc3e2340168207'
report = dict(schema_version=1,source=owner['source'],source_after=owner['source_after'],
    accepted_source=index['source'],all_commands_terminal=True,owner_receipt_sha256=sha(owner_path),
    observed_exit_code=0,pixel_tolerance=0,reference_bytes_changed=False,
    release_qualification=False,promotion_allowed=False,applied_to_umbrella=False,
    accepted_renderer_unchanged=True,new_release_states_admitted=0,
    javascript_executed_by_openui=False,cargo_commands_run=0,screenshots_generated=0,
    suites={},changed_original_comparisons=[],changed_addition_comparisons=[],
    reviewed_root_causes_not_inferred_from_hash_changes=True,probe_sha256=sha(Path(__file__)))
maps = {}
for suite in ['full','expanded','focused','primitive']:
    prior_path = RAW/f'native-event-targets-clean-{suite}-v1456/{suite}-summary.json'
    current_path = RAW/f'native-intrinsic-snap-clean-{suite}-v1716/{suite}-summary.json'
    prior,current = load(prior_path),load(current_path)
    assert sha(prior_path) == index['suites'][suite]['summary_sha256']
    assert current['source'] == current['source_after'] == owner['source'] and current['source']['clean']
    assert prior['source'] == prior['source_after'] == index['source'] and prior['source']['clean']
    for data in [prior,current]:
        assert data['results']['profile_result_sha256'] == canonical_sha256([p['result_sha256'] for p in data['profiles']])
        for profile in data['profiles']:
            assert profile['result_sha256'] == canonical_sha256(profile['tests'])
            assert len(profile['tests']) == profile['total']
        assert sum(p['total'] for p in data['profiles']) == data['results']['total']
    flatten = lambda d: {(p['profile'],r['id']):r for p in d['profiles'] for r in p['tests']}
    a,b = flatten(prior),flatten(current)
    assert a.keys() == b.keys()
    maps[suite] = b
    changes=[];losses=0;gains=0;native_changes=0;same=0
    for key, row in b.items():
        old = a[key]
        assert all(old[name] == row[name] for name in oracle), (suite,key,'immutable Chromium changed')
        unchanged = all(old[name] == row[name] for name in invariants)
        same += unchanged
        if not unchanged:
            loss = old['status']=='exact' and row['status']!='exact'
            gain = old['status']!='exact' and row['status']=='exact'
            losses += loss;gains += gain
            native_changes += old['openui_rgba_sha256'] != row['openui_rgba_sha256']
            changes.append(dict(profile=key[0],id=key[1],prior_status=old['status'],status=row['status'],
                prior_mismatched_pixels=old['mismatched_pixels'],mismatched_pixels=row['mismatched_pixels'],
                exact_loss=loss,exact_gain=gain,prior_openui_rgba_sha256=old['openui_rgba_sha256'],
                openui_rgba_sha256=row['openui_rgba_sha256'],chromium_oracle_identity_sha256=row['chromium_oracle_identity_sha256'],
                chromium_oracle_rgba_sha256=row['chromium_oracle_rgba_sha256'],reviewed_root_cause=False))
    actual_path = RAW/f'native-intrinsic-snap-{suite}-exit-v1716.json'
    actual = load(actual_path)
    assert actual['observed_exit_code'] == (1 if suite in ['full','expanded'] else 0)
    report['suites'][suite] = dict(results=current['results'],prior_results=prior['results'],
        actual_exit=actual['observed_exit_code'],summary_sha256=sha(current_path),prior_summary_sha256=sha(prior_path),
        actual_exit_receipt_sha256=sha(actual_path),unchanged_nine_invariant_rows=same,
        immutable_chromium_rows=len(b),changed_comparisons=len(changes),native_rgba_changes=native_changes,
        exact_losses=losses,exact_gains=gains,residual_ids=len({r['id'] for r in b.values() if r['status']!='exact'}))
    if suite=='full':report['changed_original_comparisons']=changes
    # Record every change before assessing whether the candidate is safe.
assert all(all(row[n]==maps['expanded'][key][n] for n in invariants) for key,row in maps['full'].items())
added = {k:v for k,v in maps['expanded'].items() if k not in maps['full']}
assert len(added) == 804
prior_added_map = {(p['profile'],r['id']):r for p in load(RAW/'native-event-targets-clean-expanded-v1456/expanded-summary.json')['profiles'] for r in p['tests']}
report['changed_addition_comparisons']=[dict(profile=k[0],id=k[1],
    prior_status=prior_added_map[k]['status'],status=v['status'],
    prior_mismatched_pixels=prior_added_map[k]['mismatched_pixels'],mismatched_pixels=v['mismatched_pixels'],
    reviewed_root_cause=False) for k,v in added.items() if any(prior_added_map[k][n]!=v[n] for n in invariants)]
groups=collections.defaultdict(list)
for (_,test_id),row in added.items():groups[test_id].append(row)
assert len(groups)==201 and all(len(rows)==4 for rows in groups.values())
report.update(original_rows_identical_in_expanded=22924,
    additions=804,addition_cases=201,addition_cases_four_profile_exact=sum(all(r['status']=='exact' for r in rows) for rows in groups.values()),
    changed_additions=len(report['changed_addition_comparisons']),
    complete_trial_has_exact_regressions=report['suites']['full']['exact_losses']>0,
    source_pixel_unqualified=True,formal_wpt_residual_ownership_unchanged=True)
assert sha(owner_path)==report['owner_receipt_sha256']
OUT.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(receipt=str(OUT),sha256=sha(OUT),
    suites={k:{n:v[n] for n in ['results','changed_comparisons','exact_losses','exact_gains','immutable_chromium_rows','residual_ids']} for k,v in report['suites'].items()},
    addition_cases_four_profile_exact=report['addition_cases_four_profile_exact'],changed_additions=report['changed_additions'],
    source_pixel_unqualified=True,release_qualification=False)),flush=True)
