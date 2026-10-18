"""Audit complete original and expanded reports without raster work."""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from residuals import canonical_sha256

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
owner_path = RAW / 'umbrella-clean-workspace-pipeline-v1411/receipt.json'
owner = json.loads(owner_path.read_bytes())
assert owner['all_commands_terminal']
assert len(owner['steps']) == 6
assert [row['observed_exit_code'] for row in owner['steps']] == [0, 0, 0, 0, 1, 1]
build_path = RAW / 'umbrella-clean-workspace-clean-v1410/build.json'
build = json.loads(build_path.read_bytes())
source = build['source']
assert source == build['source_after'] == owner['source'] == owner['source_after']
assert source['clean'] and source['commit'] == '2e443f49d49b81d65674f64bb099fed2922775b5'
assert len(build['steps']) == 6 and all(row['observed_exit_code'] == 0 for row in build['steps'])
binary = next(row for row in build['steps'] if row['name'] == 'pixel-build')
assert sha(Path(binary['binary'])) == binary['binary_sha256']
fields = ['openui_png_sha256', 'openui_rgba_sha256', 'chromium_png_sha256',
    'chromium_rgba_sha256', 'chromium_oracle_identity_sha256',
    'chromium_oracle_rgba_sha256', 'status', 'mismatched_pixels', 'diff_signature']
report = dict(schema_version=1, source=source, source_after=source, all_commands_terminal=True,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    cargo_commands_run=0, screenshots_generated=0, suites={},
    owner_receipt_sha256=sha(owner_path), build_receipt_sha256=sha(build_path),
    probe_sha256=sha(Path(__file__)))
original_rows = {}
for suite, expected in [('full', dict(total=22924, exact=21334, different=1590, errors=0)),
    ('expanded', dict(total=23728, exact=22137, different=1591, errors=0))]:
    old_path = RAW / f'cache-integration-clean-{suite}-v1340/{suite}-summary.json'
    current_path = RAW / f'umbrella-clean-workspace-clean-{suite}-v1410/{suite}-summary.json'
    exit_path = RAW / f'umbrella-clean-workspace-{suite}-exit-v1410.json'
    old = json.loads(old_path.read_bytes())
    current = json.loads(current_path.read_bytes())
    observed = json.loads(exit_path.read_bytes())
    assert observed['observed_exit_code'] == 1
    assert sha(RAW / f'umbrella-clean-workspace-clean-{suite}-v1410.log') == observed['log_sha256']
    assert current['source'] == current['source_after'] == source == current['openui']['build_identity']['source']
    assert current['openui']['binary_sha256'] == binary['binary_sha256']
    assert current['suite'] == old['suite'] == suite and current['complete_contract_scope']
    assert current['evidence']['source_unchanged'] and current['evidence']['binary_unchanged']
    assert current['evidence']['tolerance_pixels'] == 0 and not current['evidence']['qualified']
    for key in ['chromium', 'font_byte_hashes', 'resource_hashes', 'raster', 'contract_sha256', 'manifest_scope']:
        assert old[key] == current[key], key
    for key in ['count', 'sha256']:
        assert old['id_manifest'][key] == current['id_manifest'][key]
    assert sha(Path(current['id_manifest']['path'])) == current['id_manifest']['sha256']
    assert current['raster']['backend_selection'] == 'explicit-immutable'
    assert current['openui']['raster_backend_identity']['backend'] == 'cpu-skia'
    assert len(current['profiles']) == len(old['profiles']) == 4
    count = 0
    cross_count = 0
    residuals = set()
    for before, profile in zip(old['profiles'], current['profiles'], strict=True):
        for key in ['profile', 'device_scale', 'logical_size_css_px', 'physical_size_px', 'ordered_id_sha256']:
            assert before[key] == profile[key], key
        assert len(before['tests']) == len(profile['tests']) == current['id_manifest']['count']
        assert canonical_sha256(profile['tests']) == profile['result_sha256']
        for previous, row in zip(before['tests'], profile['tests'], strict=True):
            assert previous['id'] == row['id']
            assert all(previous.get(key) == row.get(key) for key in fields)
            key = (profile['profile'], row['id'])
            if suite == 'full':
                original_rows[key] = {field: row.get(field) for field in fields}
            elif key in original_rows:
                assert original_rows[key] == {field: row.get(field) for field in fields}
                cross_count += 1
            if row['status'] != 'exact':
                residuals.add(row['id'])
            count += 1
    assert count == expected['total']
    assert {key: current['results'][key] for key in expected} == expected
    if suite == 'expanded':
        assert cross_count == 22924
    report['suites'][suite] = dict(expected, actual_exit=1,
        all_nine_comparison_invariants_unchanged=count, residual_ids=len(residuals),
        original_comparisons_matching_separate_census=cross_count,
        summary_sha256=sha(current_path), prior_summary_sha256=sha(old_path),
        profile_result_sha256=current['results']['profile_result_sha256'],
        exit_receipt_sha256=sha(exit_path), residual_ledger_qualifying=False,
        source_attributed_complete_pixel_gate_pass=False)
out = RAW / 'umbrella-clean-full-matrices-audit-v1448.json'
assert not out.exists()
out.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({'path':str(out), 'sha256':sha(out), 'suites':report['suites']}), flush=True)
