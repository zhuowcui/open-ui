"""Review the terminal native event API checkpoint without new rendering."""
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from residuals import canonical_sha256

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
load = lambda p: json.loads(p.read_bytes())
owner_path = RAW / 'native-event-targets-pipeline-v1457/receipt.json'
owner = load(owner_path)
assert owner['all_commands_terminal']
assert owner['state'] == 'complete-requires-results-review'
assert [s['observed_exit_code'] for s in owner['steps']] == [0, 0, 0, 0, 0, 1, 1]
build_path = RAW / 'native-event-targets-clean-v1456/build.json'
build = load(build_path)
source = build['source']
assert source['clean'] and source['commit'] == '1c8540e9f8c6eeb8fce10a76cb9a32c2f42f01fd'
assert source == build['source_after'] == owner['source'] == owner['source_after']
assert build['all_commands_terminal'] and build['workspace_packages_cleaned'] == 18
assert all(s['observed_exit_code'] == 0 for s in build['steps'])
binary = next(s for s in build['steps'] if s['name'] == 'pixel-build')
for step in build['steps']:
    assert sha(build_path.parent / (step['name'] + '.log')) == step['log_sha256']
    if 'binary' in step:
        assert sha(Path(step['binary'])) == step['binary_sha256']

scoped_path = RAW / 'native-event-targets-audit-v1472.json'
scoped = load(scoped_path)
assert scoped['source'] == scoped['source_after'] == source
assert scoped['scoped_native_event_behavior_verified']
assert scoped['workspace'] == dict(passed=8536, failed=0, ignored=13)
assert scoped['named_fixed_guards_passed'] == 5
for path, expected in scoped['file_hashes'].items():
    assert sha(Path(path)) == expected
assert sha(build_path) == scoped['build_receipt_sha256']
assert sha(RAW / 'native-event-targets-guards-v1456/receipt.json') == scoped['guard_receipt_sha256']
assert sha(RAW / 'native-event-targets-consumer-v1456/receipt.json') == scoped['consumer_receipt_sha256']

hosted_path = RAW / 'native-event-targets-hosted-v1463/receipt.json'
hosted = load(hosted_path)
assert hosted['source'] == source['commit']
assert hosted['all_commands_terminal'] and hosted['actual_exit'] == 0
assert hosted['all_seven_jobs_passed'] and hosted['successful_jobs'] == 7 and hosted['skipped_jobs'] == 0

fields = ['openui_png_sha256', 'openui_rgba_sha256', 'chromium_png_sha256',
          'chromium_rgba_sha256', 'chromium_oracle_identity_sha256',
          'chromium_oracle_rgba_sha256', 'status', 'mismatched_pixels', 'diff_signature']
original = {}
additions = {}
suites = {}
for suite, total, exact, different, profiles, manifest_count in [
    ('focused', 640, 640, 0, 40, 16),
    ('primitive', 960, 960, 0, 40, 24),
    ('full', 22924, 21334, 1590, 4, 5731),
    ('expanded', 23728, 22137, 1591, 4, 5932),
]:
    current_path = RAW / f'native-event-targets-clean-{suite}-v1456/{suite}-summary.json'
    prior_path = RAW / f'umbrella-clean-workspace-clean-{suite}-v1410/{suite}-summary.json'
    current, prior = load(current_path), load(prior_path)
    exit_path = RAW / f'native-event-targets-{suite}-exit-v1456.json'
    observed = load(exit_path)
    actual_exit = int(different > 0)
    assert observed['observed_exit_code'] == actual_exit
    assert next(s for s in owner['steps'] if s['name'] == suite)['observed_exit_code'] == actual_exit
    assert sha(RAW / f'native-event-targets-clean-{suite}-v1456.log') == observed['log_sha256']
    assert current['source'] == current['source_after'] == source == current['openui']['build_identity']['source']
    assert current['openui']['binary_sha256'] == binary['binary_sha256']
    assert current['suite'] == prior['suite'] == suite
    assert current['complete_contract_scope']
    assert current['evidence']['source_unchanged'] and current['evidence']['binary_unchanged']
    assert current['evidence']['tolerance_pixels'] == 0
    assert current['raster']['backend_selection'] == 'explicit-immutable'
    assert current['openui']['raster_backend_identity']['backend'] == 'cpu-skia'
    assert current['id_manifest']['count'] == manifest_count
    assert current['id_manifest']['sha256'] == prior['id_manifest']['sha256']
    assert sha(Path(current['id_manifest']['path'])) == current['id_manifest']['sha256']
    for key in ['chromium', 'font_byte_hashes', 'resource_hashes', 'raster', 'contract_sha256', 'manifest_scope']:
        assert current[key] == prior[key], key
    assert len(current['profiles']) == len(prior['profiles']) == profiles
    count = 0
    for before_profile, profile in zip(prior['profiles'], current['profiles'], strict=True):
        for key in ['profile', 'device_scale', 'logical_size_css_px', 'physical_size_px', 'ordered_id_sha256']:
            assert before_profile[key] == profile[key], key
        assert canonical_sha256(profile['tests']) == profile['result_sha256']
        for before, row in zip(before_profile['tests'], profile['tests'], strict=True):
            assert before['id'] == row['id']
            assert all(before.get(field) == row.get(field) for field in fields)
            identity = (profile['profile'], row['id'])
            invariant_hash = canonical_sha256({field: row.get(field) for field in fields})
            if suite == 'full':
                assert identity not in original
                original[identity] = invariant_hash
            elif suite == 'expanded':
                if identity in original:
                    assert original[identity] == invariant_hash
                else:
                    additions.setdefault(row['id'], []).append(row['status'])
            count += 1
    assert count == total
    assert {k: current['results'][k] for k in ['total', 'exact', 'different', 'errors']} == dict(total=total, exact=exact, different=different, errors=0)
    suites[suite] = dict(total=total, exact=exact, different=different, errors=0,
                        actual_exit=actual_exit, profiles=profiles,
                        all_nine_comparison_invariants_unchanged=total,
                        summary_sha256=sha(current_path), prior_summary_sha256=sha(prior_path),
                        exit_receipt_sha256=sha(exit_path))
    del current, prior
assert len(original) == 22924 and len(additions) == 201
assert all(len(states) == 4 for states in additions.values())
addition_exact = sum(all(state == 'exact' for state in states) for states in additions.values())
assert addition_exact == 200
report = dict(schema_version=1, source=source, source_after=source,
              all_commands_terminal=True, scoped_native_event_behavior_verified=True,
              api_checkpoint_ready_for_umbrella=True, release_qualification=False,
              full_renderer_gate_passed=False, new_release_states_admitted=0,
              public_rust_api=['Event::target()', 'Event::current_target()'],
              javascript_executed_by_openui=False, pixel_tolerance=0,
              workspace=scoped['workspace'], c_abi=scoped['c_abi'], native_app=scoped['native_app'],
              suites=suites, original_rows_identical_in_expanded=22924,
              addition_cases=201, addition_cases_four_profile_exact=addition_exact,
              all_comparison_invariants_unchanged=48252,
              hosted=dict(run_url=hosted['run_url'], successful_jobs=7, skipped_jobs=0),
              owner_receipt_sha256=sha(owner_path), build_receipt_sha256=sha(build_path),
              scoped_behavior_audit_sha256=sha(scoped_path), hosted_receipt_sha256=sha(hosted_path),
              probe_sha256=sha(Path(__file__)), cargo_commands_run=0, screenshots_generated=0)
out = RAW / 'native-event-complete-audit-v1492.json'
assert not out.exists()
out.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(path=str(out), sha256=sha(out), suites=suites,
                     api_checkpoint_ready_for_umbrella=True, release_qualification=False)), flush=True)
