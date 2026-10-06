"""Audit the completed original suite while preserving the other live stage."""
import collections, hashlib, json, sys
from pathlib import Path
MAIN = Path('/home/nero/code/open-ui')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW / 'native-glyph-original-audit-v1843.json'
assert not OUT.exists()
sys.path.insert(0, str(MAIN / 'tools/qualification'))
from residuals import canonical_sha256
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
a_path = RAW / 'native-keywords-capture-clean-full-v1785/full-summary.json'
b_path = RAW / 'native-glyph-current-clean-full-v1828/full-summary.json'
before, after = [json.loads(p.read_bytes()) for p in [a_path, b_path]]
assert after['source'] == after['source_after'] and after['source']['clean']
assert after['source']['commit'] == 'c68d946c18ecd1bb6d2f3f84accecaa84cba3651'
for d in [before, after]:
    assert d['results']['profile_result_sha256'] == canonical_sha256([p['result_sha256'] for p in d['profiles']])
    for p in d['profiles']:
        assert len(p['tests']) == p['total'] and p['result_sha256'] == canonical_sha256(p['tests'])
flatten = lambda d:{(p['profile'], r['id']):r for p in d['profiles'] for r in p['tests']}
a, b = flatten(before), flatten(after)
assert a.keys() == b.keys() and len(b) == 22924
invariants = ['openui_png_sha256','openui_rgba_sha256','chromium_png_sha256','chromium_rgba_sha256','chromium_oracle_identity_sha256','chromium_oracle_rgba_sha256','status','mismatched_pixels','diff_signature']
changes = []
for key, row in b.items():
    old = a[key]
    assert all(row[k] == old[k] for k in invariants[2:6])
    if any(row[k] != old[k] for k in invariants):
        changes.append(dict(profile=key[0], id=key[1], prior_status=old['status'], status=row['status'],
                            exact_gain=old['status'] != 'exact' and row['status'] == 'exact',
                            exact_loss=old['status'] == 'exact' and row['status'] != 'exact',
                            prior_mismatched_pixels=old['mismatched_pixels'], mismatched_pixels=row['mismatched_pixels'],
                            prior_record=old, record=row, reviewed_root_cause=False))
exit_path = RAW / 'native-glyph-current-full-exit-v1828.json'
actual = json.loads(exit_path.read_bytes())
report = dict(schema_version=1, source=after['source'], source_after=after['source_after'],
              original_suite_complete=True, entire_pipeline_complete=False, actual_exit=actual['observed_exit_code'],
              results=after['results'], prior_results=before['results'], exact_gains=sum(r['exact_gain'] for r in changes),
              exact_losses=sum(r['exact_loss'] for r in changes), changed_comparisons=len(changes),
              changed_test_ids=len({r['id'] for r in changes}), unchanged_nine_invariant_rows=22924-len(changes),
              all_22924_chromium_inputs_unchanged=True, new_formal_wpt_owners_assigned=0,
              formal_ownership_review_still_required=True, changes=changes, source_summary_sha256=sha(b_path),
              baseline_summary_sha256=sha(a_path), actual_exit_receipt_sha256=sha(exit_path), probe_sha256=sha(Path(__file__)),
              release_qualification=False, new_release_states_admitted=0, pixel_tolerance=0, javascript_executed_by_openui=False)
OUT.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({k:report[k] for k in ['results','exact_gains','exact_losses','changed_comparisons','changed_test_ids','all_22924_chromium_inputs_unchanged']}))
