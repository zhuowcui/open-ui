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

ROOT = Path('/home/nero/code/open-ui')
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-all-features-v3640')
TARGET = Path('/mnt/e/openui-v02-cargo-c73754e2/target')
PREVIOUS_TERMINAL = Path('/tmp/openui-public-native-font-units-renderer-terminal-v3635.json')
sys.path.insert(0, '/tmp')
from openui_parallel_source_identity_v3060 import identity


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(2**20), b''):
            digest.update(block)
    return digest.hexdigest()


lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
terminal = json.loads(PREVIOUS_TERMINAL.read_bytes())
assert terminal['all_local_commands_terminal'] and terminal['actual_audit_exit_code'] == 0
for name in ['cargo', 'rustc', 'rustfmt', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', name], capture_output=True).returncode == 1
source = identity(ROOT)
assert source['clean'] and source == terminal['source']
assert not OUT.exists()
OUT.mkdir()
env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='4',
           RUST_MIN_STACK='4194304', PYTHONDONTWRITEBYTECODE='1', TMPDIR='/dev/shm')
env.pop('LD_PRELOAD', None)
env.pop('LD_LIBRARY_PATH', None)
cargo = ['cargo', '--config', str(ROOT / 'bindings/rust/.cargo/config.chromium.toml')]
receipt = dict(schema_version=1, owner_pid=os.getpid(), source=source,
               driver_sha256=sha(__file__), previous_terminal_sha256=sha(PREVIOUS_TERMINAL),
               steps=[], local_artifacts=[], all_commands_terminal=False,
               javascript_executed_by_openui=False, pixel_target='pinned Chromium',
               full_renderer_contract_qualified=False, all_native_apis_qualified=False,
               release_qualified=False)


def save():
    temporary = OUT / 'receipt.tmp'
    temporary.write_text(json.dumps(receipt, indent=2, sort_keys=True) + '\n')
    os.replace(temporary, OUT / 'receipt.json')


def run(name, command):
    assert identity(ROOT) == source
    print('Starting ' + name, flush=True)
    log = OUT / (name + '.log')
    child = None
    try:
        with log.open('xb') as stream:
            child = subprocess.Popen(command, cwd=ROOT/'bindings/rust', env=env,
                                     stdout=stream, stderr=subprocess.STDOUT,
                                     start_new_session=True)
            receipt['current_process'] = dict(name=name, pid=child.pid)
            save()
            while child.poll() is None:
                for path, required in [(ROOT, 128*2**20), (OUT, 10*2**30),
                                       (TARGET, 10*2**30), (Path('/dev/shm'), 256*2**20)]:
                    if shutil.disk_usage(path).free < required:
                        raise RuntimeError('disk guard: ' + str(path))
                time.sleep(1)
            child.wait()
    finally:
        if child is not None and child.poll() is None:
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=20)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
        receipt.pop('current_process', None)
        summaries = re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;', log.read_text())
        totals = [sum(int(row[i]) for row in summaries) for i in range(3)]
        receipt['steps'].append(dict(name=name, command=command, actual_exit_code=child.returncode,
                                    log_sha256=sha(log), passed=totals[0], failed=totals[1],
                                    ignored=totals[2], test_groups=len(summaries)))
        save()
    assert identity(ROOT) == source
    print(json.dumps(receipt['steps'][-1]), flush=True)
    return child.returncode, log


save()
status = 1
try:
    code, metadata = run('metadata', cargo + ['metadata', '--locked', '--offline', '--format-version', '1'])
    assert code == 0
    packages = {p['id'] for p in json.loads(metadata.read_bytes())['packages']
                if Path(p['manifest_path']).resolve().is_relative_to(ROOT)}
    assert packages
    code, _ = run('clean-local-packages', cargo + ['clean', '--locked',
                    *[arg for p in sorted(packages) for arg in ['-p', p]]])
    assert code == 0
    code, all_targets = run('workspace-all-targets-all-features', cargo +
               ['test', '--locked', '--offline', '--workspace', '--all-targets',
                '--all-features', '--message-format=json-render-diagnostics'])
    for line in all_targets.read_text().splitlines():
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        if row.get('reason') == 'compiler-artifact' and row['package_id'] in packages:
            assert row['fresh'] is False
            assert Path(row['manifest_path']).resolve().is_relative_to(ROOT)
            assert Path(row['target']['src_path']).resolve().is_relative_to(ROOT)
            row['sha256'] = {p: sha(p) for p in row['filenames']}
            receipt['local_artifacts'].append(row)
    save()
    run('workspace-and-docs-all-features', cargo +
        ['test', '--locked', '--offline', '--workspace', '--all-features',
         '--message-format=json-render-diagnostics'])
    status = int(any(s['actual_exit_code'] != 0 for s in receipt['steps']))
except BaseException as error:
    receipt['failure'] = repr(error)
finally:
    receipt['source_after'] = identity(ROOT)
    receipt['source_unchanged'] = receipt['source_after'] == source
    receipt['all_commands_terminal'] = True
    receipt['observed_exit_code'] = status
    save()
print(json.dumps(dict(complete=True, actual_exit_code=status,
                      source_unchanged=receipt['source_unchanged'])), flush=True)
raise SystemExit(status)
