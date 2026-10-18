"""Audit completed small matrices without starting builds or raster work."""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from residuals import canonical_sha256

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
build_path = RAW / 'umbrella-clean-workspace-clean-v1410/build.json'
build = json.loads(build_path.read_bytes())
source = build['source']
assert source['clean'] and source == build['source_after']
assert source['commit'] == '2e443f49d49b81d65674f64bb099fed2922775b5'
assert len(build['steps']) == 6 and all(s['observed_exit_code'] == 0 for s in build['steps'])
binary = next(s for s in build['steps'] if s['name'] == 'pixel-build')
assert sha(Path(binary['binary'])) == binary['binary_sha256']
fields = ['openui_png_sha256', 'openui_rgba_sha256', 'chromium_png_sha256',
    'chromium_rgba_sha256', 'chromium_oracle_identity_sha256',
    'chromium_oracle_rgba_sha256', 'status', 'mismatched_pixels', 'diff_signature']
report = dict(schema_version=1, source=source, source_after=source,
    release_qualification=False, full_original_and_expanded_qualification_pending=True,
    workspace_and_abi_clean_source_audit='passed, scoped to those components',
    all_commands_terminal=True, cargo_commands_run=0, screenshots_generated=0,
    build_receipt_sha256=sha(build_path), probe_sha256=sha(Path(__file__)), suites={})
for suite, count in [('focused', 640), ('primitive', 960)]:
    old_path = RAW / f'cache-integration-clean-{suite}-v1340/{suite}-summary.json'
    current_path = RAW / f'umbrella-clean-workspace-clean-{suite}-v1410/{suite}-summary.json'
    exit_path = RAW / f'umbrella-clean-workspace-{suite}-exit-v1410.json'
    old = json.loads(old_path.read_bytes())
    current = json.loads(current_path.read_bytes())
    observed = json.loads(exit_path.read_bytes())
    assert observed['observed_exit_code'] == 0
    assert sha(RAW / f'umbrella-clean-workspace-clean-{suite}-v1410.log') == observed['log_sha256']
    assert current['source'] == current['source_after'] == source == current['openui']['build_identity']['source']
    assert current['openui']['binary_sha256'] == binary['binary_sha256']
    assert current['suite'] == old['suite'] == suite
    assert current['complete_contract_scope'] and current['evidence']['source_unchanged']
    assert current['evidence']['binary_unchanged'] and current['evidence']['tolerance_pixels'] == 0
    for key in ['chromium', 'font_byte_hashes', 'resource_hashes', 'raster', 'contract_sha256', 'manifest_scope']:
        assert old[key] == current[key], key
    assert current['raster']['backend_selection'] == 'explicit-immutable'
    assert current['openui']['raster_backend_identity']['backend'] == 'cpu-skia'
    for key in ['count', 'sha256']:
        assert old['id_manifest'][key] == current['id_manifest'][key]
    assert sha(Path(current['id_manifest']['path'])) == current['id_manifest']['sha256']
    assert len(current['profiles']) == len(old['profiles']) == 40
    total = 0
    for before, profile in zip(old['profiles'], current['profiles'], strict=True):
        for key in ['profile', 'device_scale', 'logical_size_css_px', 'physical_size_px', 'ordered_id_sha256']:
            assert before[key] == profile[key], key
        assert len(profile['tests']) == len(before['tests']) == current['id_manifest']['count']
        assert canonical_sha256(profile['tests']) == profile['result_sha256']
        for previous, row in zip(before['tests'], profile['tests'], strict=True):
            assert previous['id'] == row['id']
            assert all(previous.get(key) == row.get(key) for key in fields)
            assert row['status'] == 'exact' and row['mismatched_pixels'] == 0
            total += 1
    assert total == count == current['results']['total'] == current['results']['exact']
    assert current['results']['different'] == current['results']['errors'] == 0
    report['suites'][suite] = dict(total=count, exact=count, different=0, errors=0,
        actual_exit=0, all_nine_comparison_invariants_unchanged=total,
        summary_sha256=sha(current_path), profile_result_sha256=current['results']['profile_result_sha256'],
        prior_summary_sha256=sha(old_path), exit_receipt_sha256=sha(exit_path),
        source_attributed_small_matrix_pass=True, original_pixel_gate_still_required=True)
p = RAW / 'umbrella-clean-small-matrices-audit-v1438.json'
assert not p.exists()
p.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(path=str(p), sha256=sha(p), suites=report['suites'])))
