import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
import time
import tomllib
from pathlib import Path

ROOT = Path('/dev/shm/openui-umbrella-clean-workspace-2e443f49')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'umbrella-clean-workspace-guards-v1410'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
FIXED = '2e443f49d49b81d65674f64bb099fed2922775b5'
BASELINE = '639e9c80347e674c208c43d39fd4101f9161b0ba'
BRANCH = 'agent/umbrella-clean-workspace-v1408'
TEST = 'tests::compositor_distinguishes_owned_recordings_with_equal_document_generations'
for pipeline in ['scene-recording-cache-pipeline-v1233', 'native-fieldset-pipeline-v1265', 'native-font-backends-pipeline-v1281']:
    assert json.loads((RAW / pipeline / 'receipt.json').read_bytes())['all_commands_terminal']
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
env = dict(os.environ, CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',
           CARGO_INCREMENTAL='0', RUST_MIN_STACK='4194304', CARGO_BUILD_JOBS='4',
           PYTHONDONTWRITEBYTECODE='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
base = ['cargo', '--config', '.cargo/config.chromium.toml', 'test', '--locked']
members = tomllib.loads((ROOT / 'bindings/rust/Cargo.toml').read_text())['workspace']['members']
packages = [tomllib.loads((ROOT / 'bindings/rust' / name / 'Cargo.toml').read_text())['package']['name'] for name in members]
assert len(packages) == len(set(packages)) == 18
clean = ['cargo', '--config', '.cargo/config.chromium.toml', 'clean'] + [arg for package in packages for arg in ['-p', package]]
report = dict(workspace_cleaned_at_every_source_switch=True, schema_version=1, source=source, source_after=source, release_qualification=False,
              promotion_allowed=False, all_commands_terminal=False, baseline_regression_reproduced=False,
              steps=[], probe_sha256=sha(Path(__file__)))
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
    clean_row, _ = run('clean-baseline-workspace', clean, BASELINE)
    assert clean_row['observed_exit_code'] == 0 and not clean_row['disk_guard_triggered']
    row, content = run('baseline-owned-recording-guard',
                       base + ['-p', 'openui-engine', '--lib', TEST, '--', '--exact'], BASELINE)
    report['baseline_regression_reproduced'] = (
        row['observed_exit_code'] == 101 and not row['disk_guard_triggered']
        and (TEST + ' ... FAILED').encode() in content
        and b'0 passed; 1 failed;' in content)
    save()
finally:
    subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    assert repository_source_identity(ROOT) == source

if not report['baseline_regression_reproduced']:
    report.update(all_commands_terminal=True, state='baseline-proof-failed')
    save()
    raise SystemExit(1)

clean_row, _ = run('clean-fixed-workspace', clean, FIXED)
assert clean_row['observed_exit_code'] == 0 and not clean_row['disk_guard_triggered']
steps = [
    ('fixed-owned-recording-guard', base + ['-p', 'openui-engine', '--lib', TEST, '--', '--exact']),
    ('software-cache-and-resource-guards', base + ['-p', 'openui-compositor', '--lib']),
    ('ganesh-cache-and-backend-guards', base + ['-p', 'openui-compositor', '--features', 'ganesh-gl',
                                            '--lib', '--', '--test-threads=1']),
]
for name, command in steps:
    row, content = run(name, command, FIXED)
    if row['observed_exit_code']:
        report.update(all_commands_terminal=True, state='fixed-guard-failed-' + name)
        save()
        raise SystemExit(row['observed_exit_code'])
report.update(all_commands_terminal=True, state='complete', source_after=repository_source_identity(ROOT))
assert report['source_after'] == source
save()
