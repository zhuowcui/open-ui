import fcntl
import hashlib
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-diagnostic-v3622')
TARGET = Path('/mnt/e/openui-v02-cargo-c73754e2/target')
sys.path.insert(0, '/tmp')
from openui_parallel_source_identity_v3060 import identity

def sha(path):
    h = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(2**20), b''):
            h.update(block)
    return h.hexdigest()

lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
for name in ['cargo', 'rustc', 'rustfmt', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', name], capture_output=True).returncode == 1
assert not OUT.exists()
OUT.mkdir()
source = identity(ROOT)
env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='4',
           RUST_MIN_STACK='4194304', PYTHONDONTWRITEBYTECODE='1', TMPDIR='/dev/shm')
env.pop('LD_PRELOAD', None)
env.pop('LD_LIBRARY_PATH', None)
command = ['cargo', '--config', str(ROOT / 'bindings/rust/.cargo/config.chromium.toml'),
           'test', '--locked', '--offline', '-p', 'openui-engine', '--lib', 'native_font_unit']
r = dict(schema_version=1, owner_pid=os.getpid(), source=source, command=command,
         driver_sha256=sha(__file__), dirty_diagnostic=True, all_commands_terminal=False,
         javascript_executed_by_openui=False, full_renderer_contract_qualified=False,
         all_native_apis_qualified=False, release_qualified=False)

def save():
    p = OUT / 'receipt.tmp'
    p.write_text(json.dumps(r, indent=2, sort_keys=True) + '\n')
    os.replace(p, OUT / 'receipt.json')

save()
child = None
code = 1
try:
    with (OUT / 'tests.log').open('xb') as log:
        child = subprocess.Popen(command, cwd=ROOT / 'bindings/rust', env=env,
                                 stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        r['child_pid'] = child.pid
        save()
        print(json.dumps(dict(owner_pid=os.getpid(), child_pid=child.pid, started=True)), flush=True)
        while child.poll() is None:
            for path, required in [(ROOT, 128*2**20), (OUT, 10*2**30),
                                   (TARGET, 10*2**30), (Path('/dev/shm'), 256*2**20)]:
                if shutil.disk_usage(path).free < required:
                    raise RuntimeError('disk guard: ' + str(path))
            time.sleep(1)
        code = child.wait()
    r['engine_guard_exit_code'] = code
    r['engine_log_sha256'] = sha(OUT / 'tests.log')
    save()
    if code == 0:
        public_command = command[:3] + ['test', '--locked', '--offline', '-p', 'openui', '--test', 'native_font_units']
        r['public_command'] = public_command
        with (OUT / 'public-tests.log').open('xb') as log:
            child = subprocess.Popen(public_command, cwd=ROOT / 'bindings/rust', env=env,
                                     stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
            r['public_child_pid'] = child.pid
            save()
            print('Public native Rust consumer test started', flush=True)
            while child.poll() is None:
                for path, required in [(ROOT, 128*2**20), (OUT, 10*2**30),
                                       (TARGET, 10*2**30), (Path('/dev/shm'), 256*2**20)]:
                    if shutil.disk_usage(path).free < required:
                        raise RuntimeError('disk guard: ' + str(path))
                time.sleep(1)
            code = child.wait()
        r['public_guard_exit_code'] = code
        r['public_log_sha256'] = sha(OUT / 'public-tests.log')
except BaseException as error:
    r['failure'] = repr(error)
finally:
    if child is not None and child.poll() is None:
        os.killpg(child.pid, signal.SIGTERM)
        try:
            child.wait(timeout=20)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
    r['actual_child_exit_code'] = None if child is None else child.returncode
    r['tests_log_sha256'] = sha(OUT / 'tests.log')
    r['source_after'] = identity(ROOT)
    r['source_unchanged'] = r['source_after'] == source
    r['all_commands_terminal'] = True
    r['observed_exit_code'] = code
    save()
print(json.dumps(dict(complete=True, actual_exit_code=code, source_unchanged=r['source_unchanged'])), flush=True)
sys.exit(code)
