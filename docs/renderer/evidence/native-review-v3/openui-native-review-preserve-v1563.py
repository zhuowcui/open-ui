"""Preserve terminal failures and scoped retry evidence without running Cargo."""
import hashlib
import json
import shutil
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = ROOT / 'docs/renderer/evidence/native-review-v3'
INDEX = ROOT / 'docs/renderer/generated/native-review-v3.json'
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
sha = lambda p: hashlib.file_digest(p.open('rb'), 'sha256').hexdigest()
read = lambda p: json.loads(p.read_bytes())
artifacts = []

def preserve(path, name, expected=None):
    before = sha(path)
    if expected is not None:
        assert before == expected, str(path)
    target = OUT / name
    assert not target.exists()
    shutil.copyfile(path, target)
    assert sha(target) == sha(path) == before
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(), sha256=before,
                          bytes=target.stat().st_size))

def steps(directory, prefix, expected_codes):
    path = RAW / directory / 'receipt.json'
    report = read(path)
    assert report['all_commands_terminal']
    assert [r['observed_exit_code'] for r in report['steps']] == expected_codes
    preserve(path, prefix + '-receipt.json')
    for row in report['steps']:
        preserve(path.parent / (row['name'] + '.log'),
                 prefix + '-' + row['name'] + '.log', row['log_sha256'])
    return report

inline = steps('native-inline-replaced-pipeline-v1529', 'inline-owner', [0, 0, 0, 0, 0, 1, 1])
assert inline['source'] == inline['source_after']
assert inline['source']['commit'] == 'c92e2d08163779f5e55784f14e556ecaa9d16e7f'
full_path = RAW / 'native-inline-replaced-full-audit-v1556.json'
expanded_path = RAW / 'native-inline-replaced-expanded-audit-v1561.json'
full, expanded = read(full_path), read(expanded_path)
assert full['results'] == dict(total=22924, exact=21334, different=1590, errors=0)
assert full['changed_comparisons'] == 0 and full['all_22924_chromium_inputs_unchanged']
assert expanded['results'] == dict(total=23728, exact=22133, different=1595, errors=0)
assert expanded['changed_kinds']['exact-loss'] == 4 and expanded['changed_kinds']['exact-gain'] == 0
assert expanded['all_23728_chromium_inputs_unchanged']
assert expanded['addition_cases_exact_at_all_four_profiles'] == 199
assert {r['id'] for r in expanded['changed']} == {'wpt/css_multicol/multicol-on-broken-image-alt-text'}
for path, name in [(full_path, 'inline-original-audit.json'), (expanded_path, 'inline-expanded-audit.json')]:
    report = read(path)
    for p, digest in report['file_hashes'].items():
        assert sha(Path(p)) == digest
    preserve(path, name)
for suite in ['full', 'expanded']:
    preserve(RAW / ('native-inline-replaced-' + suite + '-exit-v1528.json'),
             'inline-' + suite + '-exit.json')
for version in [1555, 1556, 1561]:
    path = Path('/tmp/openui-native-inline-replaced-' + ('expanded' if version == 1561 else 'full') + '-audit-v' + str(version) + '.py')
    preserve(path, path.name)

table = steps('native-table-progress-pipeline-v1548', 'table-owner', [1])
table_guards = steps('native-table-progress-guards-v1547', 'table-guards', [0, 101, 0, 101])
assert table['source']['commit'] == '4b3cb72c03c58416a736a207b97cdcc8a225cb5a'
assert table_guards['baseline_regression_reproduced']
for row in table_guards['steps']:
    assert not row['disk_guard_triggered']
for name in ['baseline-table-progress-guard', 'fixed-table-progress-guard']:
    log = (RAW / 'native-table-progress-guards-v1547' / (name + '.log')).read_text()
    assert 'left: 4' in log and 'right: 41' in log

geometry_path = RAW / 'native-table-progress-absolute-geometry-v1562/receipt.json'
geometry = read(geometry_path)
assert geometry['all_commands_terminal'] and geometry['observed_exit_code'] == 0
assert geometry['independent_queries_identical'] and sum(len(r['rows']) for r in geometry['runs']) == 96
preserve(geometry_path, 'table-absolute-chromium-geometry.json')
for row in geometry['inputs']:
    preserve(Path(row['path']), 'absolute-' + Path(row['path']).name, row['sha256'])
path = Path('/tmp/openui-native-table-progress-absolute-geometry-v1562.py')
preserve(path, path.name)

hosted_path = RAW / 'native-table-progress-hosted-v1550/receipt.json'
hosted = read(hosted_path)
assert hosted['all_commands_terminal'] and hosted['all_seven_jobs_passed']
assert hosted['successful_jobs'] == 7 and hosted['skipped_jobs'] == 0
preserve(hosted_path, 'table-own-source-hosted.json')
for extension, key in [('json', 'detail_sha256'), ('log', 'log_sha256')]:
    preserve(hosted_path.parent / (str(hosted['run_id']) + '.' + extension),
             'table-hosted.' + extension, hosted[key])
path = Path('/tmp/openui-native-table-progress-hosted-v1550.py')
preserve(path, path.name)

ci_path = RAW / 'native-table-progress-umbrella-ci-v1552/receipt.json'
ci = read(ci_path)
assert ci['commit'] == '4144c85a26f567d70d07ae09932d4b4d05c68792'
assert ci['all_three_hosted_workflows_complete_success'] and ci['job_conclusions'] == dict(success=6, skipped=5)
preserve(ci_path, 'umbrella-hosted.json')
for row in ci['workflows']:
    run = str(row['databaseId'])
    preserve(ci_path.parent / (run + '.json'), 'umbrella-' + run + '.json')
    preserve(ci_path.parent / (run + '.log'), 'umbrella-' + run + '.log', row['captured_log_sha256'])

raster = steps('native-raster-fields-retry-guards-v1559', 'raster-retry-guards',
               [0, 101, 101, 0, 101, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
assert raster['observed_exit_code'] == 0
queue_path = RAW / 'native-raster-fields-retry-queue-prepared-v1559.json'
queue = read(queue_path)
assert queue['restore_branch_corrected'] and queue['old_failure_root_and_probes_unchanged']
preserve(queue_path, 'raster-retry-queue.json')
for p, digest in queue['scripts'].items():
    preserve(Path(p), Path(p).name, digest)
for version in [1558, 1559]:
    path = Path('/tmp/openui-native-raster-fields-retry-' + ('checks' if version == 1558 else 'prepare') + '-v' + str(version) + '.py')
    preserve(path, path.name)
path = RAW / 'native-raster-fields-retry-checks-v1558/receipt.json'
checks = read(path)
assert checks['all_commands_terminal'] and len(checks['checks']) == 10
assert all(r['observed_exit_code'] == 0 for r in checks['checks'])
preserve(path, 'raster-retry-checks.json', queue['checks_receipt_sha256'])
for row in checks['checks']:
    preserve(path.parent / (row['name'] + '.log'), 'raster-check-' + row['name'] + '.log', row['log_sha256'])
owner_path = RAW / 'native-raster-fields-retry-pipeline-v1560/receipt.json'
owner_bytes = owner_path.read_bytes()
owner = json.loads(owner_bytes)
target = OUT / 'raster-owner-at-observation.json'
target.write_bytes(owner_bytes)
artifacts.append(dict(path=target.relative_to(ROOT).as_posix(), sha256=sha(target), bytes=target.stat().st_size))
assert not owner['all_commands_terminal'] and owner['steps'][0]['observed_exit_code'] == 0
path = RAW / 'completed-binary-v1553/manifest.json'
preserve(path, 'evidence-relocation-v1553.json')
preserve(Path(__file__), Path(__file__).name)

report = dict(schema_version=1, observed_at_utc=datetime.now(timezone.utc).isoformat(),
    accepted_renderer_unchanged=True, release_qualification=False, promotion_allowed=False,
    new_release_states_admitted=0, pixel_tolerance=0, javascript_executed_by_openui=False,
    public_native_rust_api_required=True, needed_native_operations_not_waived_by_pixel_exclusion=True,
    old_openui_pixels_are_provenance_only=True,
    inline=dict(source=inline['source'], whole_pipeline_terminal=True,
                stage_exits=[0, 0, 0, 0, 0, 1, 1], native_application_exact=60,
                focused_exact=640, primitive_exact=960, original=full['results'],
                expanded=expanded['results'], exact_gains=0, exact_losses=4,
                addition_cases_four_profile_exact=199, rejected_for_promotion=True,
                regression_id='wpt/css_multicol/multicol-on-broken-image-alt-text',
                chromium_inputs_unchanged=46652,
                reviewed_root_cause='The generic image tag is routed to atomic inline layout even when the retained node has fallback children and no decoded replaced resource.',
                root_cause_owner='openui-layout inline replacement versus fallback flow'),
    table=dict(source=table['source'], whole_pipeline_terminal=True, owner_actual_exit=1,
               baseline_actual_exit=101, fixed_actual_exit=101, baseline_fragments=4,
               fixed_fragments=4, chromium_fragments=41, absolute_geometry_queries=96,
               two_geometry_runs_identical=True, workspace_not_executed=True,
               application_not_executed=True, matrices_not_executed=True,
               own_source_hosted_success=7, own_source_hosted_skips=0,
               root_cause_owner='openui-layout repeated-table continuation and fragment propagation'),
    raster_retry=dict(source=queue['source'], fresh_root=True, restore_branch_corrected=True,
                      old_failure_root_and_probes_unchanged=True, guard_stage_terminal=True,
                      guard_actual_exit=0, three_named_baselines_failed_and_fixed_passed=True,
                      whole_pipeline_terminal=False, observed_state=owner['state'],
                      remaining_native_and_pixel_stages_pending=True),
    umbrella_hosted=dict(source=ci['commit'], workflows_success=3, jobs_success=6,
                        jobs_skipped=5, skips_are_not_passes=True),
    artifacts=artifacts, artifact_count=len(artifacts),
    artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(index=str(INDEX), sha256=sha(INDEX), artifacts=len(artifacts),
                      bytes=report['artifact_bytes'], inline_rejected=True,
                      table_guard_failed=True, raster_whole_pipeline_pending=True)), flush=True)
