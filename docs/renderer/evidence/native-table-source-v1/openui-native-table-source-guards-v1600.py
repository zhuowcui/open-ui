import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-table-source-1d846e68')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-table-source-guards-v1600'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
FIXED = 'e389b26ab28f259544e43c3052c895501e4c8686'
BASELINE = 'cd6d80be4be16151a607cddcbad94e2f73a79e62'
BRANCH = 'agent/native-table-source-v1597'
TEST = 'tests::native_repeated_table_body_progress_survives_retained_mutations'
SELECTED = ['tests::native_repeated_table_body_progress_survives_retained_mutations']
for pipeline in ['cache-umbrella-pipeline-v1291', 'native-fieldset-retry-pipeline-v1310', 'native-font-retry-pipeline-v1311', 'native-c-raster-retry-pipeline-v1312', 'native-raster-fields-pipeline-v1337', 'native-default-strike-pipeline-v1338', 'cache-integration-pipeline-v1341', 'native-fieldset-capture-forensics-pipeline-v1346', 'native-style-integration-pipeline-v1354', 'native-intrinsic-constraints-pipeline-v1375', 'native-intrinsic-constraints-pipeline-v1369', 'native-raster-consumer-pipeline-v1398', 'umbrella-clean-workspace-pipeline-v1411', 'native-style-integration-retry-pipeline-v1419', 'native-intrinsic-constraints-retry-pipeline-v1420', 'native-raster-api-pipeline-v1425', 'native-image-occlusion-pipeline-v1437', 'native-image-coverage-pipeline-v1449', 'native-event-targets-pipeline-v1457', 'native-border-contrast-pipeline-v1462', 'native-intrinsic-cache-guard-pipeline-v1469', 'native-raster-fields-public-pipeline-v1470', 'native-image-imports-pipeline-v1471', 'native-rounded-border-pipeline-v1482', 'native-border-guard-pipeline-v1501', 'native-enum-values-pipeline-v1513', 'native-inline-replaced-pipeline-v1529', 'native-table-progress-pipeline-v1548', 'native-raster-fields-retry-pipeline-v1560', 'native-inline-fallback-pipeline-v1576', 'native-keywords-pipeline-v1588']:
    assert json.loads((RAW / pipeline / 'receipt.json').read_bytes())['all_commands_terminal']
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
assert not OUT.exists() and not STORE.exists()
STORE.mkdir(); OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
env = dict(os.environ, CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',
    CARGO_INCREMENTAL='0', RUST_MIN_STACK='4194304', CARGO_BUILD_JOBS='4',
    PYTHONDONTWRITEBYTECODE='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
build_base = ['cargo', '--config', '.cargo/config.chromium.toml']
base = build_base + ['test', '--locked', '-p', 'openui-engine', '--lib']
report = dict(schema_version=1, source=source, source_after=source,
    release_qualification=False, promotion_allowed=False, all_commands_terminal=False,
    baseline_regression_reproduced=False, workspace_cleaned_at_every_source_switch=True,
    selected_native_test_names=SELECTED, steps=[], probe_sha256=sha(Path(__file__)),
    public_native_rust_api=True, javascript_executed_by_openui=False)
metadata = json.loads(subprocess.check_output(build_base + ['metadata', '--offline',
    '--locked', '--no-deps', '--format-version', '1'], cwd=ROOT / 'bindings/rust', env=env, text=True))
members = set(metadata['workspace_members'])
packages = [p['name'] for p in metadata['packages'] if p['id'] in members]
assert len(packages) == 18
clean_command = build_base + ['clean'] + [arg for package in packages for arg in ['-p', package]]
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

def run(name, command, expected_commit):
    before = repository_source_identity(ROOT)
    assert before['clean'] and before['commit'] == expected_commit
    assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
    assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
    log = OUT / (name + '.log')
    guard = False
    with log.open('xb') as stream:
        process = subprocess.Popen(command, cwd=ROOT / 'bindings/rust', env=env,
                                   stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        while process.poll() is None:
            if shutil.disk_usage('/home/nero/code/open-ui').free < 512 * 2**20 or shutil.disk_usage(STORE).free < 10 * 2**30:
                guard = True
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=20)
                break
            time.sleep(1)
        process.wait()
    after = repository_source_identity(ROOT)
    assert before == after
    row = dict(name=name, command=command, observed_exit_code=process.returncode,
               disk_guard_triggered=guard, source=before, source_after=after,
               log_sha256=sha(log))
    report['steps'].append(row)
    save()
    print(json.dumps(row), flush=True)
    return row, log.read_bytes()

try:
    try:
        subprocess.run(['git', 'switch', '--detach', BASELINE], cwd=ROOT, check=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        row, _ = run('clean-baseline', clean_command, BASELINE)
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
        row, content = run('baseline-table-progress-guard', base + [TEST, '--', '--exact'], BASELINE)
        report['baseline_regression_reproduced'] = (
            row['observed_exit_code'] == 101 and not row['disk_guard_triggered']
            and (TEST + ' ... FAILED').encode() in content
            and b'0 passed; 1 failed;' in content
            and b'Chromium table continuation count' in content)
        save()
    finally:
        subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        assert repository_source_identity(ROOT) == source
    assert report['baseline_regression_reproduced'], 'named baseline assertion was not reproduced'
    row, _ = run('clean-fixed', clean_command, FIXED)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    for name, test in [('fixed-table-progress-guard', TEST)]:
        row, content = run(name, base + [test, '--', '--exact'], FIXED)
        row['test_counts'] = [list(map(int, values)) for values in
            re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)]
        row['observed_named_tests'] = sorted(value.decode() for value in
            re.findall(rb'^test ([^ ]+) \.\.\. ok$', content, re.MULTILINE))
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
        assert row['test_counts'] == [[1, 0, 0]] and row['observed_named_tests'] == [test]
    row, content = run('fixed-fragmentation-neighbors', build_base + ['test', '--locked', '-p', 'openui-layout', '--test', 'sp13_f_fragmentation_tests'], FIXED)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    counts = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)
    assert len(counts) == 1 and int(counts[0][0]) > 0 and counts[0][1:] == (b'0', b'0')
    row['test_counts'] = [list(map(int, counts[0]))]
    report['observed_exit_code'] = 0
    report['state'] = 'complete'
except BaseException as error:
    report.update(observed_exit_code=1, state='failed', failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source
    save()
