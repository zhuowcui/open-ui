"""Audit the complete expanded census and original-row consistency."""
import collections
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from residuals import canonical_sha256

sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
paths = dict(
    baseline=RAW / 'cache-umbrella-clean-expanded-v1290/expanded-summary.json',
    current=RAW / 'cache-integration-clean-expanded-v1340/expanded-summary.json',
    build=RAW / 'cache-integration-clean-v1340/build.json',
    full_exit=RAW / 'cache-integration-expanded-exit-v1340.json',
)
output = RAW / 'cache-integration-expanded-audit-v1407.json'
assert not output.exists()
data = {key: json.loads(path.read_bytes()) for key, path in paths.items()}
old, current, build = data['baseline'], data['current'], data['build']
source = current['source']
assert source['clean'] and source['commit'] == '289d5516dcbacd17f1a5b60046771bdfe28061a6'
assert source == current['source_after'] == build['source'] == build['source_after']
assert source == current['openui']['build_identity']['source']
assert all(step['observed_exit_code'] == 0 for step in build['steps'])
binary = next(step for step in build['steps'] if step['name'] == 'pixel-build')
assert current['openui']['binary_sha256'] == binary['binary_sha256'] == sha(Path(binary['binary']))
assert data['full_exit']['observed_exit_code'] == 1
for key in ['chromium', 'font_byte_hashes', 'resource_hashes', 'raster', 'contract_sha256', 'manifest_scope']:
    assert old[key] == current[key], key
assert old['suite'] == current['suite'] == 'expanded'
assert old['complete_contract_scope'] and current['complete_contract_scope']
assert current['evidence']['source_unchanged'] and current['evidence']['binary_unchanged']
assert current['evidence']['tolerance_pixels'] == 0
for key in ['count', 'sha256']:
    assert old['id_manifest'][key] == current['id_manifest'][key]
assert current['id_manifest']['count'] == 5932
assert sha(Path(current['id_manifest']['path'])) == current['id_manifest']['sha256']
fields = ['openui_png_sha256', 'openui_rgba_sha256', 'chromium_png_sha256',
    'chromium_rgba_sha256', 'chromium_oracle_identity_sha256',
    'chromium_oracle_rgba_sha256', 'status', 'mismatched_pixels', 'diff_signature']
pipeline = json.loads((RAW / 'cache-integration-pipeline-v1341/receipt.json').read_bytes())
assert pipeline['all_commands_terminal'] and len(pipeline['steps']) == 6
assert [step['observed_exit_code'] for step in pipeline['steps']] == [0, 0, 0, 0, 1, 1]
original = json.loads((RAW / 'cache-integration-clean-full-v1340/full-summary.json').read_bytes())
assert original['source'] == source == original['source_after']
counts = collections.Counter()
residuals = set()
addition_rows = {}
for old_profile, profile in zip(old['profiles'], current['profiles'], strict=True):
    for key in ['profile', 'device_scale', 'logical_size_css_px', 'physical_size_px', 'ordered_id_sha256']:
        assert old_profile[key] == profile[key], key
    assert len(old_profile['tests']) == len(profile['tests']) == 5932
    assert canonical_sha256(profile['tests']) == profile['result_sha256']
    original_rows = {row['id']: row for row in next(p for p in original['profiles'] if p['profile'] == profile['profile'])['tests']}
    assert len(original_rows) == 5731
    for before, row in zip(old_profile['tests'], profile['tests'], strict=True):
        assert before['id'] == row['id']
        assert all(before.get(key) == row.get(key) for key in fields), (profile['profile'], row['id'])
        counts['total'] += 1
        counts['all_nine_comparison_invariants_unchanged'] += 1
        counts[row['status']] += 1
        if row['id'] in original_rows:
            assert all(row.get(key) == original_rows[row['id']].get(key) for key in fields)
            counts['original_comparisons_agree'] += 1
        else:
            addition_rows.setdefault(row['id'], []).append(row['status'])
            counts['addition_comparisons'] += 1
        if row['status'] != 'exact':
            residuals.add(row['id'])
assert counts['total'] == current['results']['total'] == 23728
for key in ['exact', 'different', 'errors']:
    assert counts['error' if key == 'errors' else key] == current['results'][key]
assert counts['original_comparisons_agree'] == 22924
assert counts['addition_comparisons'] == 804 and len(addition_rows) == 201
quad_exact = sum(len(rows) == 4 and all(status == 'exact' for status in rows) for rows in addition_rows.values())
assert quad_exact == 200
report = dict(original_comparisons_agree=22924, addition_comparisons=804, addition_cases=201, addition_cases_quad_exact=quad_exact, schema_version=1, source=source, source_after=current['source_after'],
    complete_expanded_scope=True, results=current['results'], observed_exit_code=1,
    expanded_stage_terminal=True, whole_pipeline_terminal=True,
    whole_pipeline_state_at_observation='complete-requires-results-review',
    all_nine_invariants_unchanged=counts['all_nine_comparison_invariants_unchanged'],
    exact_gains=0, exact_losses=0, changed_comparisons=0,
    residual_expanded_ids=len(residuals), zero_tolerance=True,
    chromium_inputs_images_and_oracle_identities_unchanged=True,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    original_pixel_gate='FAIL', expanded_pixel_gate='FAIL',
    generated_ownership_ledger_qualifying=current['residual_ledger']['qualifying'],
    artifacts={key: dict(path=str(path), sha256=sha(path)) for key, path in paths.items()},
    probe_sha256=sha(Path(__file__)))
output.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(results=report['results'], all_nine_invariants_unchanged=report['all_nine_invariants_unchanged'],
    exact_gains=0, exact_losses=0, residual_ids=len(residuals), sha256=sha(output))), flush=True)
