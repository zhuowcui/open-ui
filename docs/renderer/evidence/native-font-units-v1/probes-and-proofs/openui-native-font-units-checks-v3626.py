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
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-checks-v3626')
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
source = identity(ROOT)
assert source['clean'] and source['commit'].startswith('45d21648')
assert not OUT.exists()
OUT.mkdir()
env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), CARGO_PROFILE_DEV_DEBUG='0',
           CARGO_PROFILE_TEST_DEBUG='0', CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='4',
           RUST_MIN_STACK='4194304', PYTHONDONTWRITEBYTECODE='1', TMPDIR='/dev/shm')
env.pop('LD_PRELOAD', None)
env.pop('LD_LIBRARY_PATH', None)
env['OPENUI_CLANG_FORMAT'] = '/mnt/d/openui-v02-qualification-d174ea0b/clang-format-tool-v3625/clang_format/data/bin/clang-format'
cargo = ['cargo', '--config', str(ROOT / 'bindings/rust/.cargo/config.chromium.toml')]
r = dict(schema_version=1, source=source, owner_pid=os.getpid(), steps=[], cases=[],
         driver_sha256=sha(__file__), local_artifacts=[], all_commands_terminal=False,
         javascript_executed_by_openui=False, pixel_target='pinned Chromium', pixel_tolerance=0,
         historical_openui_archive_is_a_pixel_target=False,
         full_renderer_contract_qualified=False, all_native_apis_qualified=False, release_qualified=False)

def save():
    p = OUT / 'receipt.tmp'
    p.write_text(json.dumps(r, indent=2, sort_keys=True) + '\n')
    os.replace(p, OUT / 'receipt.json')

def run(name, command, cwd=ROOT, timeout=None):
    print('Starting ' + name, flush=True)
    log = OUT / (name + '.log')
    child = None
    code = None
    start = time.monotonic()
    try:
        with log.open('xb') as stream:
            child = subprocess.Popen(command, cwd=cwd, env=env, stdout=stream,
                                     stderr=subprocess.STDOUT, start_new_session=True)
            r['current_process'] = dict(name=name, pid=child.pid)
            save()
            while child.poll() is None:
                for path, required in [(ROOT,128*2**20),(OUT,10*2**30),
                                       (TARGET,10*2**30),(Path('/dev/shm'),256*2**20)]:
                    if shutil.disk_usage(path).free < required:
                        raise RuntimeError('disk guard: ' + str(path))
                if timeout and time.monotonic()-start > timeout:
                    raise TimeoutError(name)
                time.sleep(1)
            code = child.wait()
    finally:
        if child is not None and child.poll() is None:
            os.killpg(child.pid, signal.SIGTERM)
            try:
                child.wait(timeout=20)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
        r.pop('current_process', None)
        summary = re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;', log.read_text())
        totals = [sum(int(row[i]) for row in summary) for i in range(3)]
        r['steps'].append(dict(name=name, command=command, cwd=str(cwd),
                              actual_exit_code=None if child is None else child.returncode,
                              log_sha256=sha(log), passed=totals[0], failed=totals[1], ignored=totals[2]))
        save()
    print(json.dumps(dict(stage=name, actual_exit_code=code, passed=totals[0], failed=totals[1])), flush=True)
    return code, log

save()
code = 1
try:
    for name, command in [
        ('ffi-generator',['python3','-B','tools/ffi/generate_ffi.py','--check']),
        ('style-generator',['python3','-B','tools/style/generate_properties.py','--check']),
        ('renderer-generator',['python3','-B','tools/qualification/generate_renderer_contract.py','--check']),
        ('release-generator',['python3','-B','tools/release/generate_v02_contract.py','--check']),
        ('accountability',['python3','-B','tools/accountability/audit.py','--repository-only']),
    ]:
        run(name,command)
    metadata_code, metadata_log = run('metadata',cargo+['metadata','--locked','--offline','--format-version','1'],ROOT/'bindings/rust')
    assert metadata_code == 0
    packages = {p['id'] for p in json.loads(metadata_log.read_bytes())['packages']
                if Path(p['manifest_path']).resolve().is_relative_to(ROOT)}
    assert packages
    clean_code,_ = run('clean-local-packages',cargo+['clean','--locked',*[arg for p in sorted(packages) for arg in ['-p',p]]],ROOT/'bindings/rust')
    assert clean_code == 0
    build_code, build_log = run('native-build',cargo+['build','--locked','--offline','-p','openui','--example','native_font_units','--message-format=json-render-diagnostics'],ROOT/'bindings/rust')
    if build_code == 0:
        for line in build_log.read_text().splitlines():
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if row.get('reason') == 'compiler-artifact' and row['package_id'] in packages:
                assert row['fresh'] is False
                assert Path(row['manifest_path']).resolve().is_relative_to(ROOT)
                assert Path(row['target']['src_path']).resolve().is_relative_to(ROOT)
                row['sha256'] = {p:sha(p) for p in row['filenames']}
                r['local_artifacts'].append(row)
        assert any(a['target']['name']=='native_font_units' for a in r['local_artifacts'])
        binary = OUT/'native_font_units'
        shutil.copy2(TARGET/'debug/examples/native_font_units',binary)
        r['native_binary_sha256'] = sha(binary)
        save()
        for i,(width,height,scale) in enumerate([(800,600,1.0),(1280,720,1.25),(800,600,2.0),(1280,720,1.5)]):
            row = dict(width=width,height=height,scale=scale,runs=[])
            for repeat in [1,2]:
                destination = OUT/f'profile-{i}'/f'native-{repeat}'
                status,_ = run(f'profile-{i}-native-{repeat}',[str(binary),str(destination),str(width),str(height),str(scale)],timeout=120)
                assert status==0
                row['runs'].append(dict(directory=str(destination),files={p.name:sha(p) for p in sorted(destination.iterdir())}))
            assert row['runs'][0]['files']==row['runs'][1]['files']
            r['cases'].append(row)
            save()
    run('workspace-all-targets',cargo+['test','--locked','--offline','--workspace','--all-targets'],ROOT/'bindings/rust')
    ffi_code,_ = run('ffi-build',cargo+['build','--locked','--offline','-p','openui-ffi'],ROOT/'bindings/rust')
    if ffi_code == 0:
        r['ffi_library_sha256'] = sha(TARGET/'debug/libopenui_ffi.so')
        run('abi-consumers',['python3','-B','tools/ffi/verify_abi.py','--library',str(TARGET/'debug/libopenui_ffi.so')],timeout=600)
    code = int(any(step['actual_exit_code']!=0 for step in r['steps']))
except BaseException as error:
    r['failure'] = repr(error)
finally:
    r['source_after'] = identity(ROOT)
    r['source_unchanged'] = r['source_after']==source
    r['all_commands_terminal'] = True
    r['observed_exit_code'] = code
    save()
print(json.dumps(dict(complete=True,actual_exit_code=code,source_unchanged=r['source_unchanged'])),flush=True)
sys.exit(code)
