import collections
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from residuals import canonical_sha256

suite = sys.argv[1]
assert suite in ('full', 'expanded')
OUT = RAW / ('cache-umbrella-' + suite + '-audit-v1335.json')
assert not OUT.exists()
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
paths = dict(
    baseline=RAW / f'native-scroll-umbrella-clean-{suite}-v566/{suite}-summary.json',
    candidate=RAW / f'cache-umbrella-clean-{suite}-v1290/{suite}-summary.json',
    build=RAW / 'cache-umbrella-clean-v1289/build.json',
    observed_exit=RAW / f'cache-umbrella-{suite}-exit-v1290.json',
)
if suite == 'expanded':
    paths['original'] = RAW / 'cache-umbrella-clean-full-v1290/full-summary.json'
data = {name: json.loads(path.read_bytes()) for name, path in paths.items()}
before, after, build = [data[name] for name in ['baseline', 'candidate', 'build']]
source = build['source']
assert source == build['source_after'] == after['source'] == after['source_after'] == after['openui']['build_identity']['source']
assert source['clean'] and source['commit'] == '3ff5f8af8ad5442f636fd15a0299e23b5e8694da'
assert len(build['steps']) == 6 and all(row['observed_exit_code'] == 0 for row in build['steps'])
binary = next(row for row in build['steps'] if row['name'] == 'pixel-build')
assert after['openui']['binary_sha256'] == binary['binary_sha256'] == sha(Path(binary['binary']))
assert before['suite'] == after['suite'] == suite
assert before['complete_contract_scope'] and after['complete_contract_scope']
assert len(before['profiles']) == len(after['profiles']) == 4
expected_cases = 5731 if suite == 'full' else 5932
for key in ['chromium', 'font_byte_hashes', 'resource_hashes', 'raster', 'contract_sha256', 'manifest_scope']:
    assert before[key] == after[key], key
for key in ['count', 'sha256']:
    assert before['id_manifest'][key] == after['id_manifest'][key]
assert after['id_manifest']['count'] == expected_cases
assert sha(Path(after['id_manifest']['path'])) == after['id_manifest']['sha256']
assert after['evidence']['source_unchanged'] and after['evidence']['binary_unchanged']
assert after['evidence']['tolerance_pixels'] == 0
fields = ['openui_png_sha256', 'openui_rgba_sha256', 'chromium_png_sha256', 'chromium_rgba_sha256',
          'chromium_oracle_identity_sha256', 'chromium_oracle_rgba_sha256', 'status',
          'mismatched_pixels', 'diff_signature']
oracle_fields = fields[2:6]
counts = collections.Counter()
changes = []
remaining = set()
additions = {}
for old_profile, profile in zip(before['profiles'], after['profiles'], strict=True):
    for key in ['profile', 'device_scale', 'logical_size_css_px', 'physical_size_px', 'ordered_id_sha256']:
        assert old_profile[key] == profile[key], key
    assert len(old_profile['tests']) == len(profile['tests']) == expected_cases
    assert canonical_sha256(profile['tests']) == profile['result_sha256']
    original_rows = {}
    if suite == 'expanded':
        original = data['original']
        assert original['source'] == source == original['source_after']
        original_rows = {row['id']: row for row in next(p for p in original['profiles'] if p['profile'] == profile['profile'])['tests']}
        assert len(original_rows) == 5731
    for old, row in zip(old_profile['tests'], profile['tests'], strict=True):
        assert old['id'] == row['id']
        assert all(old.get(key) == row.get(key) for key in oracle_fields), (profile['profile'], row['id'], 'oracle changed')
        counts['total'] += 1
        counts[row['status']] += 1
        if row['status'] != 'exact':
            remaining.add(row['id'])
        changed = any(old.get(key) != row.get(key) for key in fields)
        counts['all_nine_invariants_unchanged'] += not changed
        if changed:
            loss = old['status'] == 'exact' and row['status'] != 'exact'
            gain = old['status'] != 'exact' and row['status'] == 'exact'
            worse = row.get('mismatched_pixels', 0) > old.get('mismatched_pixels', 0)
            counts['changed_comparisons'] += 1
            counts['exact_losses'] += loss
            counts['exact_gains'] += gain
            counts['mismatch_count_worsens'] += worse
            changes.append(dict(profile=profile['profile'], id=row['id'], exact_loss=loss, exact_gain=gain,
                mismatched_pixel_count_worsens=worse, owner_review='openui-engine/openui-compositor recording ownership; minimized renderer cause pending',
                old={key: old.get(key) for key in fields}, new={key: row.get(key) for key in fields}))
        if suite == 'expanded':
            if row['id'] in original_rows:
                assert all(row.get(key) == original_rows[row['id']].get(key) for key in fields), (profile['profile'], row['id'], 'separate original disagrees')
                counts['original_comparisons_agree'] += 1
            else:
                counts['addition_comparisons'] += 1
                counts['addition_invariants_unchanged'] += not changed
                additions.setdefault(row['id'], []).append(dict(profile=profile['profile'], status=row['status'],
                    mismatched_pixels=row.get('mismatched_pixels'), diff_signature=row.get('diff_signature')))
assert not changes, 'recording cache unexpectedly changes standalone renderer pixels'
assert counts['total'] == expected_cases * 4 == after['results']['total']
for key in ['exact', 'different', 'errors']:
    assert counts['error' if key == 'errors' else key] == after['results'][key]
observed_exit = data['observed_exit']['observed_exit_code']
assert observed_exit == int(after['results']['different'] != 0 or after['results']['errors'] != 0)
report = dict(schema_version=1, suite=suite, release_qualification=False, promotion_allowed=False,
    prior_source=before['source'], source=source, complete_scope=True, complete_results=after['results'],
    observed_exit_code=observed_exit, zero_tolerance=True, reference_inputs_changed=False,
    all_chromium_image_and_oracle_invariants_unchanged=True,
    all_comparison_invariants_unchanged=not changes, delta_counts=dict(counts), changes=changes,
    changed_test_ids=sorted({row['id'] for row in changes}), remaining_test_ids=sorted(remaining),
    changed_comparison_root_causes_reviewed=False if changes else None,
    artifacts=[dict(path=str(path.relative_to(ROOT)), sha256=sha(path)) for path in paths.values()],
    probe_sha256=sha(Path(__file__)))
if suite == 'expanded':
    assert counts['original_comparisons_agree'] == 22924 and counts['addition_comparisons'] == 804
    assert len(additions) == 201 and all(len(rows) == 4 for rows in additions.values())
    failing = {key: rows for key, rows in additions.items() if any(row['status'] != 'exact' for row in rows)}
    report.update(separate_original_census_agrees_all_22924=True, addition_cases_total=201,
        addition_cases_exact_all_four_profiles=201-len(failing), failing_additions=failing,
        all_804_addition_invariants_unchanged=counts['addition_invariants_unchanged'] == 804)
OUT.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(suite=suite, results=report['complete_results'], delta_counts=dict(counts),
                     sha256=sha(OUT))), flush=True)
