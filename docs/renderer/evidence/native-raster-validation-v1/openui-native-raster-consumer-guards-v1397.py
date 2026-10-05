"""Require named raster regressions and clean every changed workspace source."""
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

ROOT = Path('/dev/shm/openui-native-raster-consumer-0e1f12ff')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-raster-consumer-guards-v1397'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
FIXED = '3d4eea1184ac6703ae4b29de12ddb4cb75fe5ed3'
BRANCH = 'agent/native-raster-consumer-v1393'
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
report = dict(schema_version=1, source=source, source_after=source,
    release_qualification=False, promotion_allowed=False, all_commands_terminal=False,
    workspace_cleaned_at_every_source_switch=True, named_baseline_regressions=[],
    steps=[], probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
env = dict(os.environ, CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',
    CARGO_INCREMENTAL='0', RUST_MIN_STACK='4194304', CARGO_BUILD_JOBS='4',
    PYTHONDONTWRITEBYTECODE='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
base = ['cargo', '--config', '.cargo/config.chromium.toml']
members = tomllib.loads((ROOT / 'bindings/rust/Cargo.toml').read_text())['workspace']['members']
packages = [tomllib.loads((ROOT / 'bindings/rust' / member / 'Cargo.toml').read_text())['package']['name'] for member in members]
assert len(packages) == len(set(packages)) == 18
clean = base + ['clean'] + [arg for package in packages for arg in ['-p', package]]
paint = ['-p', 'openui-paint', '--test', 'text_raster_configuration_tests']
text = ['-p', 'openui-text', '--lib']
phase_test = 'lcd_phase_moves_ink_by_physical_pixels_for_each_text_role'
settings_test = 'shaping::shape_result::tests::fontations_preserves_requested_mask_and_position_settings'
default_test = 'native_outlines_fit_at_physical_size_with_default_and_explicit_backends'

def run(name, command, commit, test=None):
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
            if shutil.disk_usage('/home/nero/code/open-ui').free < 512 * 2**20 or shutil.disk_usage(STORE).free < 10 * 2**30:
                guard = True
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=20)
                break
            time.sleep(1)
        process.wait()
    after = repository_source_identity(ROOT)
    assert before == after
    content = log.read_bytes()
    row = dict(name=name, command=command, source=before, source_after=after,
        observed_exit_code=process.returncode, disk_guard_triggered=guard, log_sha256=sha(log))
    if test:
        row.update(test=test, named_test_failed=(test + ' ... FAILED').encode() in content,
            named_test_passed=(test + ' ... ok').encode() in content)
    report['steps'].append(row)
    save()
    print(name, 'exit', process.returncode, flush=True)
    return row, content

try:
    for name, commit, tests in [
        ('phase-and-settings', 'b7284ac9fb733d22f4314561c0ae19166007f444', [(phase_test, paint), (settings_test, text)]),
        ('default-strike', '5390b683c6b7898d79f36450f1034f0d6a89d37d', [(default_test, paint)]),
    ]:
        subprocess.run(['git', 'switch', '--detach', commit], cwd=ROOT, check=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        row, _ = run('clean-baseline-' + name, clean, commit)
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
        for test, options in tests:
            row, content = run('baseline-' + test.split('::')[-1],
                base + ['test', '--locked'] + options + [test, '--', '--exact'], commit, test)
            assert row['observed_exit_code'] == 101 and not row['disk_guard_triggered']
            assert row['named_test_failed'] and b'0 passed; 1 failed;' in content
            report['named_baseline_regressions'].append(test)
            save()
    subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    assert repository_source_identity(ROOT) == source
    row, _ = run('clean-fixed', clean, FIXED)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    for test, options in [(phase_test, paint), (settings_test, text), (default_test, paint),
        ('shaping::shape_result::tests::fontations_lcd_hints_at_physical_size_before_replay', text),
        ('tests::compositor_distinguishes_owned_recordings_with_equal_document_generations', ['-p', 'openui-engine', '--lib'])]:
        row, content = run('fixed-' + test.split('::')[-1],
            base + ['test', '--locked'] + options + [test, '--', '--exact'], FIXED, test)
        assert row['observed_exit_code'] == 0 and row['named_test_passed']
        assert b'1 passed; 0 failed; 0 ignored;' in content
    for name, options in [
        ('text-library', ['-p', 'openui-text', '--lib']),
        ('paint-raster-configuration', paint),
        ('engine-library', ['-p', 'openui-engine', '--lib']),
        ('software-compositor', ['-p', 'openui-compositor', '--lib']),
        ('ganesh-compositor', ['-p', 'openui-compositor', '--features', 'ganesh-gl', '--lib', '--', '--test-threads=1']),
    ]:
        row, _ = run(name, base + ['test', '--locked'] + options, FIXED)
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    report.update(state='complete', observed_exit_code=0)
except BaseException as error:
    report.update(state='failed-requires-review', observed_exit_code=1, failure=str(error))
    raise
finally:
    subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    report.update(source_after=repository_source_identity(ROOT), all_commands_terminal=True)
    assert report['source_after'] == source
    save()
