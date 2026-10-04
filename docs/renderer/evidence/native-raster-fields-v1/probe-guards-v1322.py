"""Require named failures on the test-only baseline, then test the correction."""
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

ROOT = Path('/dev/shm/openui-native-raster-fields-ac08ec56')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-raster-fields-guards-v1322'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
BASELINE = 'ee3d4134'
FIXED = 'bcb7063f5f51fd32160f3095b546c226f8e89ff9'
BRANCH = 'agent/native-raster-fields-v1320'
TESTS = [
    ('phase', 'lcd_phase_moves_ink_by_physical_pixels_for_each_text_role',
     ['-p', 'openui-paint', '--test', 'text_raster_configuration_tests']),
    ('font-settings', 'shaping::shape_result::tests::fontations_preserves_requested_mask_and_position_settings',
     ['-p', 'openui-text', '--lib']),
]
assert not OUT.exists() and not STORE.exists()
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
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
report = dict(schema_version=1, source=source, source_after=source,
              baseline=BASELINE, release_qualification=False, promotion_allowed=False,
              all_commands_terminal=False, steps=[], probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

def run(name, test, options, commit):
    before = repository_source_identity(ROOT)
    assert before['clean'] and before['commit'].startswith(commit)
    assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
    assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
    command = base + options + [test, '--', '--exact']
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
    content = log.read_bytes()
    row = dict(name=name, test=test, command=command, source=before, source_after=after,
               observed_exit_code=process.returncode, disk_guard_triggered=guard,
               log_sha256=sha(log), named_test_failed=(test + ' ... FAILED').encode() in content,
               named_test_passed=(test + ' ... ok').encode() in content)
    report['steps'].append(row)
    save()
    print(json.dumps(row), flush=True)
    return row, content

try:
    try:
        subprocess.run(['git', 'switch', '--detach', BASELINE], cwd=ROOT, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        proofs = []
        for name, test, options in TESTS:
            row, content = run('baseline-' + name, test, options, BASELINE)
            proofs.append(row['observed_exit_code'] == 101 and not row['disk_guard_triggered']
                          and row['named_test_failed'] and b'0 passed; 1 failed;' in content)
        report['baseline_regressions_reproduced'] = proofs
    finally:
        subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
                       stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        assert repository_source_identity(ROOT) == source
    if not all(proofs):
        report['state'] = 'baseline-proof-failed'
        raise SystemExit(1)
    neighbors = [('physical-strike',
        'shaping::shape_result::tests::fontations_lcd_hints_at_physical_size_before_replay',
        ['-p', 'openui-text', '--lib'])]
    for name, test, options in TESTS + neighbors:
        row, content = run('fixed-' + name, test, options, FIXED)
        if row['observed_exit_code'] or not row['named_test_passed'] or b'1 passed; 0 failed;' not in content:
            report['state'] = 'fixed-proof-failed-' + name
            raise SystemExit(row['observed_exit_code'] or 1)
    report['state'] = 'complete'
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source
    save()
