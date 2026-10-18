import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-intrinsic-constraints-abed078d')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-intrinsic-constraints-guards-v1374'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
FIXED = '7d723caa7541882da6f403152b1432f0d45c50a8'
BRANCH = 'agent/native-intrinsic-constraints-v1361'
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
report = dict(schema_version=1, source=source, source_after=source,
    all_commands_terminal=False, release_qualification=False, promotion_allowed=False,
    named_baseline_failures_reproduced=[], steps=[], probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
env = dict(os.environ, CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',
    CARGO_INCREMENTAL='0', RUST_MIN_STACK='4194304', CARGO_BUILD_JOBS='4',
    PYTHONDONTWRITEBYTECODE='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
base = ['cargo', '--config', '.cargo/config.chromium.toml', 'test', '--locked']
build_base = ['cargo', '--config', '.cargo/config.chromium.toml']
metadata = json.loads(subprocess.check_output(build_base + ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'], cwd=ROOT / 'bindings/rust', env=env, text=True))
workspace_members = set(metadata['workspace_members'])
workspace_packages = [package['name'] for package in metadata['packages'] if package['id'] in workspace_members]
assert len(workspace_packages) == 18
clean_command = build_base + ['clean'] + [arg for package in workspace_packages for arg in ['-p', package]]
report['workspace_cleaned_at_every_source_switch'] = True


def run(name, command, commit):
    before = repository_source_identity(ROOT)
    assert before['clean'] and before['commit'] == commit
    assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
    assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
    log = OUT / (name + '.log')
    guard = False
    with log.open('xb') as stream:
        process = subprocess.Popen(command, cwd=ROOT / 'bindings/rust', env=env,
            stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        while process.poll() is None:
            if shutil.disk_usage(ROOT).free < 512 * 2**20 or shutil.disk_usage(STORE).free < 10 * 2**30:
                guard = True
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=20)
                break
            time.sleep(1)
        process.wait()
    after = repository_source_identity(ROOT)
    assert before == after
    row = dict(name=name, command=command, source=before, source_after=after,
        observed_exit_code=process.returncode, disk_guard_triggered=guard, log_sha256=sha(log))
    report['steps'].append(row)
    save()
    print(name, 'exit', process.returncode, flush=True)
    return row, log.read_bytes()

tests = [
    ('atomic', 'e0ab6c32167806ce81cc26720505f060e1570c10',
        'tests::native_atomic_constraints_contribute_to_enclosing_intrinsic_size'),
    ('leading', '3096820371973e6e17c8a94be188f1e8160e953b',
        'tests::native_intrinsic_collapsible_leading_spaces_preserve_width_and_indentation'),
    ('cache', 'ef10f8ef',
        'tests::compositor_distinguishes_owned_recordings_with_equal_document_generations'),
]
try:
    for name, commit, test in tests:
        subprocess.run(['git', 'switch', '--detach', commit], cwd=ROOT, check=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        expected = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()
        clean_row, _ = run('clean-baseline-' + name, clean_command, expected)
        assert clean_row['observed_exit_code'] == 0 and not clean_row['disk_guard_triggered']
        row, content = run('baseline-' + name,
            base + ['-p', 'openui-engine', '--lib', test, '--', '--exact'], expected)
        assert row['observed_exit_code'] == 101 and not row['disk_guard_triggered']
        assert (test + ' ... FAILED').encode() in content and b'0 passed; 1 failed;' in content
        report['named_baseline_failures_reproduced'].append(test)
        save()
    subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    assert repository_source_identity(ROOT) == source
    clean_row, _ = run('clean-fixed', clean_command, FIXED)
    assert clean_row['observed_exit_code'] == 0 and not clean_row['disk_guard_triggered']
    for name, _, test in tests:
        row, content = run('fixed-' + name,
            base + ['-p', 'openui-engine', '--lib', test, '--', '--exact'], FIXED)
        assert row['observed_exit_code'] == 0 and b'1 passed; 0 failed; 0 ignored;' in content
    for name, command in [
        ('engine-library', base + ['-p', 'openui-engine', '--lib']),
        ('layout-library', base + ['-p', 'openui-layout', '--lib']),
        ('software-compositor-library', base + ['-p', 'openui-compositor', '--lib']),
        ('ganesh-compositor-library', base + ['-p', 'openui-compositor', '--features',
            'ganesh-gl', '--lib', '--', '--test-threads=1']),
    ]:
        row, content = run(name, command, FIXED)
        assert row['observed_exit_code'] == 0
    report['observed_exit_code'] = 0
except BaseException:
    report['observed_exit_code'] = 1
    raise
finally:
    subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    report.update(source_after=repository_source_identity(ROOT), all_commands_terminal=True)
    save()
