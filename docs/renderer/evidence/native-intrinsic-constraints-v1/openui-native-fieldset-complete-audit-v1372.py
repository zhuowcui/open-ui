"""Audit completed immutable fieldset-trial censuses against Chromium inputs."""
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
out = RAW / ('native-fieldset-' + suite + '-complete-audit-v1372.json')
assert not out.exists()
sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
paths = dict(
    accepted=RAW / f'cache-umbrella-clean-{suite}-v1290/{suite}-summary.json',
    prior_trial=RAW / f'native-intrinsic-clean-{suite}-v1212/{suite}-summary.json',
    candidate=RAW / f'native-fieldset-retry-clean-{suite}-v1310/{suite}-summary.json',
    build=RAW / 'native-fieldset-retry-clean-v1310/build.json',
    pipeline=RAW / 'native-fieldset-retry-pipeline-v1310/receipt.json',
)
if suite == 'expanded':
    paths['original'] = RAW / 'native-fieldset-retry-clean-full-v1310/full-summary.json'
data = {key: json.loads(path.read_bytes()) for key, path in paths.items()}
candidate, build, pipeline = (data[key] for key in ('candidate', 'build', 'pipeline'))
source = build['source']
assert source == build['source_after'] == candidate['source'] == candidate['source_after']
assert source == candidate['openui']['build_identity']['source']
assert source['clean'] and source['commit'] == 'abed078ddb2c8c2442155d14571cbadcb3b94758'
assert len(build['steps']) == 10 and all(row['observed_exit_code'] == 0 for row in build['steps'])
assert pipeline['all_commands_terminal'] and len(pipeline['steps']) == 9
stage = next(row for row in pipeline['steps'] if row['name'] == suite)
assert stage['observed_exit_code'] == 1
binary = next(row for row in build['steps'] if row['name'] == 'pixel-build')
assert candidate['openui']['binary_sha256'] == binary['binary_sha256'] == sha(Path(binary['binary']))
expected_cases = 5731 if suite == 'full' else 5932
assert candidate['id_manifest']['count'] == expected_cases
assert sha(Path(candidate['id_manifest']['path'])) == candidate['id_manifest']['sha256']
assert candidate['evidence']['binary_unchanged'] and candidate['evidence']['source_unchanged']
assert candidate['evidence']['tolerance_pixels'] == 0
fields = ['openui_png_sha256', 'openui_rgba_sha256', 'chromium_png_sha256',
          'chromium_rgba_sha256', 'chromium_oracle_identity_sha256',
          'chromium_oracle_rgba_sha256', 'status', 'mismatched_pixels', 'diff_signature']
oracle_fields = fields[2:6]
comparisons = {}
for baseline_name in ('accepted', 'prior_trial'):
    before = data[baseline_name]
    assert before['suite'] == candidate['suite'] == suite
    assert before['complete_contract_scope'] and candidate['complete_contract_scope']
    for key in ('chromium', 'font_byte_hashes', 'resource_hashes', 'raster',
                'contract_sha256', 'manifest_scope'):
        assert before[key] == candidate[key], (baseline_name, key)
    for key in ('count', 'sha256'):
        assert before['id_manifest'][key] == candidate['id_manifest'][key]
    counts = collections.Counter()
    changes, residuals, additions = [], set(), {}
    for old_profile, profile in zip(before['profiles'], candidate['profiles'], strict=True):
        for key in ('profile', 'device_scale', 'logical_size_css_px', 'physical_size_px', 'ordered_id_sha256'):
            assert old_profile[key] == profile[key], key
        assert len(profile['tests']) == len(old_profile['tests']) == expected_cases
        assert canonical_sha256(profile['tests']) == profile['result_sha256']
        original_rows = {}
        if suite == 'expanded':
            original = data['original']
            assert original['source'] == source == original['source_after']
            original_rows = {row['id']: row for row in next(
                p for p in original['profiles'] if p['profile'] == profile['profile'])['tests']}
            assert len(original_rows) == 5731
        for old, row in zip(old_profile['tests'], profile['tests'], strict=True):
            assert old['id'] == row['id']
            assert all(old.get(key) == row.get(key) for key in oracle_fields), (profile['profile'], row['id'])
            counts['total'] += 1
            counts[row['status']] += 1
            if row['status'] != 'exact':
                residuals.add(row['id'])
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
                changes.append(dict(profile=profile['profile'], id=row['id'], exact_loss=loss,
                    exact_gain=gain, mismatched_pixel_count_worsens=worse,
                    owner='openui-layout intrinsic contribution and inline whitespace; openui-text/paint for glyph residuals',
                    root_cause_review_complete=False,
                    old={key: old.get(key) for key in fields}, new={key: row.get(key) for key in fields}))
            if suite == 'expanded':
                if row['id'] in original_rows:
                    assert all(row.get(key) == original_rows[row['id']].get(key) for key in fields)
                    counts['original_comparisons_agree'] += 1
                else:
                    counts['addition_comparisons'] += 1
                    counts['addition_invariants_unchanged'] += not changed
                    additions.setdefault(row['id'], []).append(dict(profile=profile['profile'], status=row['status']))
    assert counts['total'] == expected_cases * 4 == candidate['results']['total']
    for key in ('exact', 'different', 'errors'):
        assert counts['error' if key == 'errors' else key] == candidate['results'][key]
    entry = dict(prior_source=before['source'], delta_counts=dict(counts), changes=changes,
                 changed_test_ids=sorted({row['id'] for row in changes}), remaining_test_ids=sorted(residuals))
    if suite == 'expanded':
        assert counts['original_comparisons_agree'] == 22924 and counts['addition_comparisons'] == 804
        assert len(additions) == 201 and all(len(rows) == 4 for rows in additions.values())
        failing = {key: rows for key, rows in additions.items() if any(row['status'] != 'exact' for row in rows)}
        entry.update(addition_cases_exact_all_four_profiles=201-len(failing), failing_additions=failing)
    comparisons[baseline_name] = entry
report = dict(schema_version=1, suite=suite, source=source, complete_scope=True,
    complete_results=candidate['results'], observed_exit_code=stage['observed_exit_code'],
    zero_tolerance=True, release_qualification=False, promotion_allowed=False,
    all_chromium_image_and_oracle_invariants_unchanged=True, reference_inputs_changed=False,
    comparisons=comparisons, changed_comparison_root_causes_reviewed=False,
    generated_ownership_ledger_qualifying=candidate['residual_ledger']['qualifying'],
    generated_ownership_ledger_error=candidate['residual_ledger'].get('error'),
    artifacts=[dict(path=str(path.relative_to(ROOT)), sha256=sha(path)) for path in paths.values()],
    probe_sha256=sha(Path(__file__)))
out.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(suite=suite, results=report['complete_results'],
    delta_counts={key: entry['delta_counts'] for key, entry in comparisons.items()}, sha256=sha(out))), flush=True)
