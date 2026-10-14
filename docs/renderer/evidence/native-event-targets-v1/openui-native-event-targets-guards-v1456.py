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

ROOT = Path('/dev/shm/openui-native-event-targets-43706e83')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-event-targets-guards-v1456'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
FIXED = '1c8540e9f8c6eeb8fce10a76cb9a32c2f42f01fd'
BASELINE = '1f1870990f6068e9aaff2d0025d8c6bae0183970'
BRANCH = 'agent/native-event-targets-v1454'
TEST = 'document::tests::rust_event_dispatch_clears_phase_after_callbacks_return'
SELECTED = ['document::tests::rust_event_dispatch_clears_phase_after_callbacks_return', 'document::tests::rust_event_targets_clear_listener_state_on_callback_panic', 'document::tests::rust_event_targets_clear_listener_state_on_foreign_callback_error', 'document::tests::rust_event_targets_follow_delegated_listener_scope_and_generations', 'document::tests::rust_event_targets_follow_pointer_capture_and_boundary_dispatch']
for pipeline in ['cache-umbrella-pipeline-v1291', 'native-fieldset-retry-pipeline-v1310', 'native-font-retry-pipeline-v1311', 'native-c-raster-retry-pipeline-v1312', 'native-raster-fields-pipeline-v1337', 'native-default-strike-pipeline-v1338', 'cache-integration-pipeline-v1341', 'native-fieldset-capture-forensics-pipeline-v1346', 'native-style-integration-pipeline-v1354', 'native-intrinsic-constraints-pipeline-v1375', 'native-intrinsic-constraints-pipeline-v1369', 'native-raster-consumer-pipeline-v1398', 'umbrella-clean-workspace-pipeline-v1411', 'native-style-integration-retry-pipeline-v1419', 'native-intrinsic-constraints-retry-pipeline-v1420', 'native-raster-api-pipeline-v1425', 'native-image-occlusion-pipeline-v1437', 'native-image-coverage-pipeline-v1449']:
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
base = build_base + ['test', '--locked', '-p', 'openui', '--features', 'ffi-integration', '--lib']
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
    subprocess.run(['git', 'switch', '--detach', BASELINE], cwd=ROOT, check=True,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    row, _ = run('clean-baseline', clean_command, BASELINE)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    row, content = run('baseline-event-phase-guard', base + [TEST, '--', '--exact'], BASELINE)
    report['baseline_regression_reproduced'] = (
        row['observed_exit_code'] == 101 and not row['disk_guard_triggered']
        and (TEST + ' ... FAILED').encode() in content
        and b'0 passed; 1 failed;' in content
        and b'event phase must clear after dispatch' in content)
    save()
finally:
    subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    assert repository_source_identity(ROOT) == source
if not report['baseline_regression_reproduced']:
    report.update(all_commands_terminal=True, state='baseline-proof-failed')
    save(); raise SystemExit(1)
row, _ = run('clean-fixed', clean_command, FIXED)
assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
row, content = run('fixed-event-target-and-lifetime-guards',
    base + ['document::tests::rust_event_'], FIXED)
row['test_counts'] = [list(map(int, values)) for values in
    re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)]
row['observed_named_tests'] = sorted(value.decode() for value in
    re.findall(rb'^test ([^ ]+) \.\.\. ok$', content, re.MULTILINE))
selected_exact = row['test_counts'] == [[5, 0, 0]] and row['observed_named_tests'] == SELECTED
report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT),
    state='complete' if row['observed_exit_code'] == 0 and selected_exact else 'fixed-guard-failed')
assert report['source_after'] == source
save()
raise SystemExit(row['observed_exit_code'] or int(not selected_exact))
