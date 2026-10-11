"""Preserve measured native text failures and fresh verification checkpoints."""
import gzip
import hashlib
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
NATIVE = Path('/dev/shm/openui-native-text-content-viewport-queue-90310e15')
OUT = ROOT / 'docs/renderer/evidence/native-text-content-v1'
INDEX = ROOT / 'docs/renderer/generated/native-text-content-v1.json'
assert not OUT.exists() and not INDEX.exists()
OUT.mkdir()
sha = lambda p: hashlib.file_digest(p.open('rb'), 'sha256').hexdigest()
artifacts = []

def preserve(path, name, expected=None, compressed=False):
    content = path.read_bytes()
    digest = hashlib.sha256(content).hexdigest()
    if expected is not None:
        assert digest == expected, str(path)
    target = OUT / name
    assert not target.exists()
    target.write_bytes(gzip.compress(content, compresslevel=9, mtime=0) if compressed else content)
    assert sha(path) == digest
    row = dict(path=target.relative_to(ROOT).as_posix(), sha256=sha(target), bytes=target.stat().st_size,
               source_path=str(path))
    if compressed:
        assert gzip.decompress(target.read_bytes()) == content
        row.update(encoding='gzip', decompressed_sha256=digest, decompressed_bytes=len(content))
    else:
        assert sha(target) == digest
    artifacts.append(row)
    return json.loads(content) if path.suffix == '.json' else None

def generated(name, content):
    target = OUT / name
    assert not target.exists()
    target.write_bytes(content)
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(), sha256=sha(target), bytes=len(content)))

sys.path.insert(0, str(NATIVE / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
source = repository_source_identity(NATIVE)
assert source['clean'] and source['commit'] == '90310e15b86bed558a79c8eb085d2a72092eab69'
queue_path = RAW / 'native-text-content-viewport-queue-prepared-v1635.json'
queue = preserve(queue_path, 'queue-prepared.json', 'c708868239deb967f13c1b88bcee5d65f249ac58046936caf91cab726c7303c2')
assert queue['source'] == queue['source_after'] == source and queue['production_code_identical_after_test_setup_correction']
for path, digest in queue['scripts'].items():
    preserve(Path(path), Path(path).name, digest)
checks_path = RAW / 'native-text-content-viewport-checks-v1626/receipt.json'
checks = preserve(checks_path, 'read-only-checks.json', queue['checks_receipt_sha256'])
assert checks['source'] == checks['source_after'] == source and checks['all_commands_terminal']
assert len(checks['checks']) == 13 and all(row['observed_exit_code'] == 0 for row in checks['checks'])
for row in checks['checks']:
    preserve(checks_path.parent / (row['name'] + '.log'), 'check-' + row['name'] + '.log', row['log_sha256'])
for version, prefix in [(1617, 'initial-unlaunched'), (1620, 'original-queued')]:
    old = preserve(RAW / f'native-text-content-queue-prepared-v{version}.json', prefix + '-queue.json')
    for path, digest in old['scripts'].items():
        preserve(Path(path), prefix + '-' + Path(path).name, digest)
preserve(Path('/tmp/openui-native-text-content-prepare-v1617.py'), 'initial-prepare.py')
for name in ['openui-native-text-content-checks-v1616.py', 'openui-native-text-content-viewport-checks-v1626.py',
             'openui-native-text-content-dispatch-v1619.py', 'openui-native-text-content-hosted-v1622.py',
             'openui-native-text-content-viewport-dispatch-v1637.py', 'openui-native-text-content-viewport-hosted-v1638.py',
             'openui-native-text-content-viewport-prepare-v1635.py']:
    preserve(Path('/tmp') / name, name)
old_checks = preserve(RAW / 'native-text-content-checks-v1616/receipt.json', 'original-read-only-checks.json')
assert old_checks['all_commands_terminal'] and len(old_checks['checks']) == 13
assert all(row['observed_exit_code'] == 0 for row in old_checks['checks'])
for row in old_checks['checks']:
    preserve(RAW / 'native-text-content-checks-v1616' / (row['name'] + '.log'),
             'original-check-' + row['name'] + '.log', row['log_sha256'])
for name, start, end in [
    ('native-text-regression.patch', 'f25cd72205a7fc6ce73496f0119749f4ddfeb59d', '1c1b1bffb9cc5b50513e1b060ce25a9c7295ece1'),
    ('viewport-test-setup.patch', '1c1b1bffb9cc5b50513e1b060ce25a9c7295ece1', queue['baseline_commit']),
    ('shared-native-text-content.patch', queue['baseline_commit'], source['commit']),
]:
    generated(name, subprocess.check_output(['git', 'diff', '--binary', start, end], cwd=NATIVE))
for path in ['bindings/rust/openui/examples/native_text_content.rs', 'examples/c_v02/text_content.c',
             'examples/c_v02/text_content.cc', 'docs/v02/generated/openui-ffi-layout.json']:
    preserve(NATIVE / path, Path(path).name)
assert (NATIVE / 'docs/v02/generated/openui-ffi-layout.json').read_bytes() == (ROOT / 'docs/v02/generated/openui-ffi-layout.json').read_bytes()

# Preserve the failed preparation, whose diff-header assertion ran after writing probes.
for kind in ['prepare', 'guards', 'build', 'consumer', 'matrices']:
    p = Path('/tmp') / f'openui-native-text-content-viewport-{kind}-v1627.py'
    preserve(p, 'unlaunched-preparation-' + p.name)
preserve(Path('/tmp/openui-native-text-content-viewport-pipeline-v1628.py'), 'unlaunched-preparation-owner-v1628.py')
generated('unlaunched-preparation-note.json', (json.dumps(dict(
    schema_version=1, preparation_actual_exit=1, failed_assertion='production == old_production',
    reason='the FFI diff index hashes also change when the test viewport setup changes',
    partial_probes_written=True, owner_launched=False, cargo_commands_run=0, screenshots_generated=0,
    fresh_source_root_and_probes_used_for_retry=True, retry_queue=queue_path.name,
    all_original_source_and_probes_preserved=True), sort_keys=True, indent=2) + '\n').encode())

geometry = []
for version in [1610, 1611, 1612]:
    receipt = RAW / f'native-font-c-geometry-v{version}/receipt.json'
    data = preserve(receipt, f'c-geometry-v{version}.json')
    preserve(Path('/tmp') / f'openui-native-font-c-geometry-v{version}.py', f'c-geometry-v{version}.py')
    assert data['all_commands_terminal'] and data['cargo_commands'] == data['raster_commands'] == data['screenshots_generated'] == 0
    if version == 1610:
        assert data['observed_exit_code'] == 1 and data['runs'] == []
    else:
        assert data['observed_exit_code'] == 0 and data['repeats_identical'] and data['source'] == data['source_after']
    geometry.append(dict(version=version, actual_exit=data['observed_exit_code'], repeatable=data.get('repeats_identical', False)))
audit = preserve(RAW / 'native-text-content-consumer-audit-v1615.json', 'reviewed-c-consumer-audit.json.gz',
                 'd999df009f8459729398903a786a41e0f2a38598d7109f1c6c542e6b0551ba59', compressed=True)
assert audit['native_image_comparisons'] == 1200 and audit['pixel_exact'] == 73
assert audit['geometry_exact'] == 25600 and audit['geometry_expected'] == 76800
assert len(audit['reviewed_c_rows']) == 800 and all(row['reviewed'] for row in audit['reviewed_c_rows'])
assert audit['remaining_rust_pixel_root_causes_unreviewed'] == 327 and audit['all_original_receipts_and_images_unchanged']
consumer = preserve(RAW / 'native-raster-fields-retry-consumer-v1559/receipt.json', 'original-consumer.json.gz',
                    audit['consumer_receipt_sha256'], compressed=True)
assert consumer['all_commands_terminal'] and consumer['observed_exit_code'] == 1
assert consumer['source'] == consumer['source_after'] == audit['source']
preserve(Path('/tmp/openui-native-raster-fields-retry-consumer-v1559.py'), 'original-consumer.py', consumer['probe_sha256'])
preserve(RAW / 'native-raster-fields-retry-clean-v1559/build.json', 'original-clean-build.json')
static = preserve(RAW / 'native-raster-fields-retry-static-v1559/receipt.json', 'static-consumer.json.gz', compressed=True)
assert static['all_commands_terminal'] and static['source'] == static['source_after'] == consumer['source']
assert static['totals']['chromium-linux-fontations-lcd']['pixel_exact'] == 60
assert static['totals']['default']['pixel_exact'] == static['totals']['chromium-linux-lcd']['pixel_exact'] == 0
preserve(Path('/tmp/openui-native-raster-fields-retry-static-v1559.py'), 'static-consumer.py', static['probe_sha256'])
matrices = []
for suite, filename, exact, total, exit_code in [('selection', 'full-summary.json', 648, 880, 1),
                                              ('focused', 'focused-summary.json', 640, 640, 0),
                                              ('primitive', 'primitive-summary.json', 960, 960, 0)]:
    data = preserve(RAW / f'native-raster-fields-retry-clean-{suite}-v1559' / filename,
                    suite + '-summary.json.gz', compressed=True)
    actual = preserve(RAW / f'native-raster-fields-retry-{suite}-exit-v1559.json', suite + '-exit.json')
    assert data['source'] == data['source_after'] == consumer['source']
    assert data['results']['exact'] == exact and data['results']['total'] == total and data['results']['errors'] == 0
    assert actual['observed_exit_code'] == exit_code
    matrices.append(dict(suite=suite, results=data['results'], actual_exit=exit_code))
preserve(Path('/tmp/openui-native-raster-fields-retry-matrices-v1559.py'), 'raster-matrices.py')
raster_owner = preserve(RAW / 'native-raster-fields-retry-pipeline-v1560/receipt.json', 'raster-owner-at-observation.json')
original_owner = preserve(RAW / 'native-text-content-pipeline-v1621/receipt.json', 'original-owner-at-observation.json')
owner = preserve(RAW / 'native-text-content-viewport-pipeline-v1636/receipt.json', 'owner-at-observation.json')
assert owner['source'] == source and not owner['all_commands_terminal'] and owner['steps'] == []

for version, prefix in [(1619, 'original'), (1637, 'viewport')]:
    filename = ('openui-native-text-content-' if version == 1619 else 'openui-native-text-content-viewport-') + f'dispatch-v{version}.json'
    preserve(Path('/tmp') / filename, prefix + '-hosted-dispatch.json')
hosted_observations = []
for version, prefix in [(1622, 'original'), (1638, 'viewport')]:
    dirname = ('native-text-content-' if version == 1622 else 'native-text-content-viewport-') + f'hosted-v{version}'
    data = preserve(RAW / dirname / 'receipt.json', prefix + '-hosted-at-observation.json')
    if data['all_commands_terminal']:
        run = str(data['run_id'])
        preserve(RAW / dirname / (run + '.json'), prefix + '-hosted-detail.json', data['detail_sha256'])
        preserve(RAW / dirname / (run + '.log'), prefix + '-hosted.log.gz', data['log_sha256'], compressed=True)
    hosted_observations.append(dict(source=data['source'], run_id=data['run_id'], terminal=data['all_commands_terminal'],
                                    state=data['state'], job_conclusions=data.get('job_conclusions', {}),
                                    actual_exit=data.get('actual_exit'), all_seven_jobs_passed=data['all_seven_jobs_passed']))
for job in [111908423690, 111908423786]:
    preserve(Path('/tmp') / f'openui-text-content-failed-job-{job}-v1623.log', f'original-job-{job}-early-unavailable.log')
    p = Path('/tmp') / f'openui-text-content-failed-job-{job}-v1624.log'
    assert b'assertion `left == right` failed' in p.read_bytes() and b'left: InvalidArgument' in p.read_bytes()
    assert b'31 passed; 1 failed;' in p.read_bytes() and b'Process completed with exit code 101' in p.read_bytes()
    preserve(p, f'original-job-{job}.log.gz', compressed=True)

table_path = RAW / 'native-table-source-hosted-v1603/receipt.json'
table = preserve(table_path, 'table-source-hosted.json')
assert table['all_commands_terminal'] and table['all_seven_jobs_passed'] and table['actual_exit'] == 0
assert table['job_conclusions'] == dict(success=7)
run = str(table['run_id'])
preserve(table_path.parent / (run + '.json'), 'table-source-hosted-detail.json', table['detail_sha256'])
preserve(table_path.parent / (run + '.log'), 'table-source-hosted.log.gz', table['log_sha256'], compressed=True)
ci_path = RAW / 'native-table-source-umbrella-ci-v1607/receipt.json'
ci = preserve(ci_path, 'preceding-umbrella-hosted.json')
assert ci['commit'] == 'f25cd72205a7fc6ce73496f0119749f4ddfeb59d' and ci['all_three_hosted_workflows_complete_success']
assert ci['job_conclusions'] == dict(success=6, skipped=5)
for row in ci['workflows']:
    run = str(row['databaseId'])
    preserve(ci_path.parent / (run + '.json'), 'umbrella-' + run + '.json')
    preserve(ci_path.parent / (run + '.log'), 'umbrella-' + run + '.log.gz', row['captured_log_sha256'], compressed=True)
preserve(Path('/tmp/openui-native-table-source-umbrella-ci-v1607.py'), 'preceding-umbrella-ci.py')
preserve(RAW / 'retired-artifact-relocation-v1614.json', 'retired-artifact-relocation.json',
         'e40cbec85756052c5efc639318b4b7d16730d013bbd680270282b058bfb92c7e')
preserve(Path('/tmp/openui-retired-artifact-relocation-v1614.py'), 'retired-artifact-relocation.py')
assert repository_source_identity(NATIVE) == source
for path, digest in queue['scripts'].items():
    assert sha(Path(path)) == digest
preserve(Path(__file__), Path(__file__).name)
report = dict(schema_version=1, observed_at_utc=datetime.now(timezone.utc).isoformat(), source=source,
    baseline_commit=queue['baseline_commit'], branch=queue['branch'], applied_to_umbrella=False,
    accepted_renderer_unchanged=True, release_qualification=False, promotion_allowed=False,
    new_release_states_admitted=0, pixel_tolerance=0, javascript_executed_by_openui=False,
    public_native_rust_api_required=True, needed_native_operations_not_waived_by_pixel_exclusion=True,
    old_openui_pixels_are_provenance_only=True,
    reviewed_native_text_failure=dict(owner='openui-ffi native element text replacement',
        root_cause='C setter stores container text data; layout consumes authored Text children',
        images=1200, pixel_exact=73, geometry_exact=25600, geometry_expected=76800,
        rust_images=400, rust_pixel_exact=73, rust_geometry_exact=25600,
        c_and_cpp_images=800, c_and_cpp_pixel_exact=0, c_and_cpp_geometry_exact=0,
        c_and_cpp_images_reviewed=800, c_and_cpp_blank_white_images=800,
        real_c_rust_equal_images=0, rust_self_rows_are_not_cross_language_passes=True,
        remaining_rust_pixel_root_causes_unreviewed=327, original_consumer_actual_exit=1,
        text_child_geometry_css_px=dict(Ahem=[40,20], DejaVuSans=[27.40625,20]),
        minimized_geometry_proofs=geometry, source_library_only=True, new_screenshots_generated_by_geometry_probes=0),
    prepared_native_operation=dict(shared_engine_set_text_content=True,
        public_rust_element_and_c_setters_use_same_operation=True,
        existing_raw_engine_set_text_behavior_preserved=True, c_exports=113, c_layouts=30,
        read_only_checks_passed=13, old_source_hosted_test_setup_failure_preserved=True,
        production_code_identical_after_test_setup_correction=True,
        corrected_viewport_test_baseline=queue['baseline_commit'], storage_update_guard_required=10000,
        whole_owner='native-text-content-viewport-pipeline-v1636', prior_whole_owners=33,
        observed_state=owner['state'], required_stages=7, required_native_images=600,
        required_native_phase_and_geometry_states=38400, local_native_and_pixel_execution_pending=True),
    raster_scope=dict(source=consumer['source'], static_policy_totals=static['totals'],
        matrices=matrices, whole_owner_state=raster_owner['state'],
        full_and_expanded_still_required=True, scope_results_are_not_full_renderer_qualification=True),
    original_text_owner_at_observation=dict(state=original_owner['state'], terminal=original_owner['all_commands_terminal']),
    own_source_hosted_at_observation=hosted_observations,
    table_source_hosted=dict(source=table['source'], successful_jobs=7, skipped_jobs=0, actual_exit=0),
    preceding_umbrella_hosted=dict(source=ci['commit'], successful_workflows=3, successful_jobs=6,
                                   skipped_jobs=5, skips_are_not_passes=True),
    evidence_storage=dict(lossless_gzip=True, original_receipts_images_and_oracles_unmodified=True),
    artifacts=artifacts, artifact_count=len(artifacts), artifact_bytes=sum(a['bytes'] for a in artifacts),
    decompressed_artifact_bytes=sum(a.get('decompressed_bytes', a['bytes']) for a in artifacts))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
for row in artifacts:
    p = ROOT / row['path']
    assert sha(p) == row['sha256']
    if row.get('encoding') == 'gzip':
        assert hashlib.sha256(gzip.decompress(p.read_bytes())).hexdigest() == row['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX), sha256=sha(INDEX), artifacts=len(artifacts),
                     stored_bytes=report['artifact_bytes'], decompressed_bytes=report['decompressed_artifact_bytes'],
                     release_qualification=False)), flush=True)
