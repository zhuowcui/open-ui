"""Qualify the corrected strike guard after the entire width owner has ended."""
import fcntl
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

MAIN = Path('/home/nero/code/open-ui')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
FIXED_ROOT = Path('/dev/shm/openui-native-glyph-descriptor-corrected-2f53d5de-v1754')
BASELINE_ROOT = Path('/dev/shm/openui-native-glyph-descriptor-baseline-0733955a-v1740')
FIXED = 'fce42e086dee48f6e2d725b2b4f3bb8815e79e5c'
BASELINE = '54bcdeab5b021fc7899ca3e25868ab39182aee62'
TEST = 'shaping::shape_result::tests::fontations_preserves_physical_strike_descriptor_for_real_fonts'
OUT = RAW / 'native-glyph-descriptor-guards-v1764'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
prior = RAW / 'native-intrinsic-snap-pipeline-v1717/receipt.json'
assert hashlib.sha256(prior.read_bytes()).hexdigest() == 'c152f0e0b4804c79e1bc54dd5ca4ee4b1d2662c3d9156e5d563af1c1770ce55a'
previous = json.loads(prior.read_bytes())
assert previous['all_commands_terminal'] and len(previous['steps']) == 7
for program in ['cargo', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', program], capture_output=True).returncode == 1
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(MAIN / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
source = repository_source_identity(FIXED_ROOT)
baseline_source = repository_source_identity(BASELINE_ROOT)
assert source['clean'] and source['commit'] == FIXED
assert baseline_source['clean'] and baseline_source['commit'] == BASELINE
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
env = dict(os.environ, CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target', CARGO_INCREMENTAL='0',
           RUST_MIN_STACK='4194304', CARGO_BUILD_JOBS='4', PYTHONDONTWRITEBYTECODE='1',
           CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
base = ['cargo', '--config', '.cargo/config.chromium.toml']
metadata = json.loads(subprocess.check_output(base + ['metadata', '--offline', '--locked', '--no-deps', '--format-version', '1'],
                                             cwd=FIXED_ROOT / 'bindings/rust', env=env))
members = set(metadata['workspace_members'])
packages = [p['name'] for p in metadata['packages'] if p['id'] in members]
assert len(packages) == 18
clean = base + ['clean'] + [arg for package in packages for arg in ['-p', package]]
test = base + ['test', '--locked', '-p', 'openui-text', '--lib']
report = dict(schema_version=1, source=source, baseline_source=baseline_source, source_after=source,
              steps=[], state='running-clean-baseline', all_commands_terminal=False,
              whole_pipeline_lock_covers_all_stages_and_gaps=True, owner_pid=os.getpid(),
              prior_whole_owner_sha256=sha(prior), workspace_packages_cleaned_at_each_source=18,
              baseline_regression_reproduced=False, probe_sha256=sha(Path(__file__)),
              javascript_executed_by_openui=False, public_native_rust_apis_required=True,
              screenshots_generated=0, native_app_and_pixel_matrices_unexecuted=True,
              release_qualification=False, pixel_tolerance=0, new_release_states_admitted=0)
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
def run(name, root, command, identity):
    assert repository_source_identity(root) == identity
    for program in ['cargo', 'pixel_compare']:
        assert subprocess.run(['pgrep', '-x', program], capture_output=True).returncode == 1
    report['state'] = 'running-' + name
    save()
    log = OUT / (name + '.log')
    disk_guard = False
    with log.open('xb') as stream:
        process = subprocess.Popen(command, cwd=root / 'bindings/rust', env=env,
                                   stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        report['current_process'] = dict(pid=process.pid, program='cargo')
        save()
        while process.poll() is None:
            if shutil.disk_usage(MAIN).free < 512 * 2**20 or shutil.disk_usage(STORE).free < 10 * 2**30:
                disk_guard = True
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=20)
                break
            time.sleep(1)
        process.wait()
    report.pop('current_process')
    assert repository_source_identity(root) == identity
    data = log.read_bytes()
    counts = [tuple(map(int, n)) for n in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', data)]
    row = dict(name=name, actual_exit=process.returncode, disk_guard_triggered=disk_guard,
               source=identity, source_after=identity, log_sha256=sha(log),
               passed=sum(n[0] for n in counts), failed=sum(n[1] for n in counts), ignored=sum(n[2] for n in counts))
    report['steps'].append(row)
    save()
    print(json.dumps({k: row[k] for k in ['name', 'actual_exit', 'passed', 'failed', 'ignored', 'disk_guard_triggered']}), flush=True)
    return row, data
try:
    row, _ = run('clean-baseline', BASELINE_ROOT, clean, baseline_source)
    assert row['actual_exit'] == 0 and not row['disk_guard_triggered']
    row, data = run('baseline-physical-strike-guard', BASELINE_ROOT, test + [TEST, '--', '--exact'], baseline_source)
    reproduced = (row['actual_exit'] == 101 and row['passed'] == 0 and row['failed'] == 1
                  and not row['disk_guard_triggered'] and (TEST + ' ... FAILED').encode() in data
                  and b'Fontations must preserve the physical strike descriptor' in data)
    report['baseline_regression_reproduced'] = reproduced
    save()
    assert reproduced
    row, _ = run('clean-fixed', FIXED_ROOT, clean, source)
    assert row['actual_exit'] == 0 and not row['disk_guard_triggered']
    row, data = run('fixed-physical-strike-guard', FIXED_ROOT, test + [TEST, '--', '--exact'], source)
    assert row['actual_exit'] == 0 and row['passed'] == 1 and row['failed'] == 0 and (TEST + ' ... ok').encode() in data
    row, data = run('all-text-tests', FIXED_ROOT, test, source)
    assert row['actual_exit'] == 0 and row['failed'] == 0 and row['passed'] > 300 and not row['disk_guard_triggered']
    report.update(state='complete-native-pixels-still-unqualified', observed_exit_code=0)
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(FIXED_ROOT))
    if report['state'].startswith('running-'):
        report['state'] = 'stopped-requires-actual-failure-review'
        report['observed_exit_code'] = 1
    save()
