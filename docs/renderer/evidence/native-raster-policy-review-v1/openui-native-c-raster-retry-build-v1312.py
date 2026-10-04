import hashlib
import importlib.util
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-c-raster-consumer-3395cefa')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-c-raster-retry-clean-v1312'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == 'ac08ec56e90a6465b22c05c3e76fe8dacefcdcd6'
env = dict(os.environ, CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',
           CARGO_INCREMENTAL='0', RUST_MIN_STACK='4194304', CARGO_BUILD_JOBS='4',
           PYTHONDONTWRITEBYTECODE='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
base = ['cargo', '--config', '.cargo/config.chromium.toml']
steps = [
    ('clean-runner', base + ['clean', '-p', 'pixel-compare'], None),
    ('raster-boundaries', base + ['test', '--locked', '-p', 'openui-ffi', '--lib',
        '--features', 'linux', 'raster_configuration::tests::'], None),
    ('workspace', base + ['test', '--workspace', '--locked', '--features', 'openui-ffi/linux'], None),
    ('font-build', base + ['build', '--locked', '-p', 'openui', '--example', 'native_font_raster'],
        'debug/examples/native_font_raster'),
    ('ffi-build', base + ['build', '--locked', '-p', 'openui-ffi', '--features', 'linux'],
        'debug/libopenui_ffi.so'),
    ('pixel-build', base + ['build', '--locked', '-p', 'pixel-compare', '--bin', 'pixel_compare'],
        'debug/pixel_compare'),
    ('ffi-consumers', [sys.executable, str(ROOT / 'tools/ffi/verify_abi.py'),
        '--library', str(OUT / 'libopenui_ffi.so')], None),
]
spec = importlib.util.spec_from_file_location('ffi_verify', ROOT / 'tools/ffi/verify_abi.py')
ffi_verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ffi_verify)
cc, cxx, c_flags, cxx_flags, link_flags = ffi_verify.compilers()
assert cc and cxx
for language, suffix, compiler, standard, flags in [
    ('c', 'c', cc, 'c11', c_flags), ('cpp', 'cc', cxx, 'c++17', cxx_flags)]:
    object_file = OUT / ('raster_configuration-' + language + '.o')
    binary = OUT / ('raster_configuration-' + language)
    steps.extend([
        ('compile-' + language, [compiler, '-std=' + standard, '-Wall', '-Wextra', '-Werror',
            *flags, '-I' + str(ROOT / 'include'),
            str(ROOT / 'examples/c_v02' / ('raster_configuration.' + suffix)),
            '-c', '-o', str(object_file)], None),
        ('link-' + language, [cxx, *cxx_flags, *link_flags, '-fuse-ld=lld', str(object_file),
            str(OUT / 'libopenui_ffi.so'), '-Wl,-rpath,' + str(OUT), '-o', str(binary)], binary),
        ('run-' + language, [str(binary)], None),
    ])
report = dict(schema_version=1, source=source, source_after=source, release_qualification=False,
              all_commands_terminal=False, steps=[], expected_stages=13,
              cargo_target_dir=env['CARGO_TARGET_DIR'], probe_sha256=sha(Path(__file__)))
assert len(steps) == report['expected_stages']
receipt = OUT / 'build.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
try:
    for name, command, artifact in steps:
        log = OUT / (name + '.log')
        diagnostic = ROOT / 'tests/pixel_text/openui_renders/basic_text_openui.png'
        backup = diagnostic.read_bytes() if name == 'workspace' and diagnostic.exists() else None
        with log.open('xb') as stream:
            process = subprocess.Popen(command, cwd=ROOT / 'bindings/rust', env=env,
                stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
            disk_guard = False
            while process.poll() is None:
                if shutil.disk_usage(ROOT).free < 512 * 2**20 or shutil.disk_usage(
                        Path(env['CARGO_TARGET_DIR'])).free < 10 * 2**30:
                    disk_guard = True
                    os.killpg(process.pid, signal.SIGTERM)
                    process.wait(timeout=20)
                    break
                time.sleep(1)
            process.wait()
        if backup is not None and diagnostic.read_bytes() != backup:
            (OUT / 'generated-diagnostic-openui.png').write_bytes(diagnostic.read_bytes())
            diagnostic.write_bytes(backup)
        entry = dict(name=name, command=command, observed_exit_code=process.returncode,
                     disk_guard_triggered=disk_guard, log_sha256=sha(log))
        report['steps'].append(entry)
        report['source_after'] = repository_source_identity(ROOT)
        assert report['source_after'] == source
        if name in ('workspace', 'raster-boundaries'):
            totals = [tuple(map(int, match)) for match in re.findall(
                rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;', log.read_bytes())]
            entry.update(passed=sum(t[0] for t in totals), failed=sum(t[1] for t in totals),
                         ignored=sum(t[2] for t in totals))
            if name == 'raster-boundaries' and process.returncode == 0:
                assert entry['passed'] == 5 and entry['failed'] == entry['ignored'] == 0
        if process.returncode == 0 and artifact is not None:
            if isinstance(artifact, str):
                path = OUT / Path(artifact).name
                shutil.copy2(Path(env['CARGO_TARGET_DIR']) / artifact, path)
            else:
                path = artifact
            entry.update(binary=str(path), binary_sha256=sha(path))
            if name == 'ffi-build':
                (OUT / 'libopenui.so.0').symlink_to('libopenui_ffi.so')
            if name == 'pixel-build':
                identity = json.loads(subprocess.check_output([str(path), 'build-source-identity'],
                    cwd=ROOT, env=env, text=True))
                assert identity['source'] == source
                report['build_identity'] = identity
        save()
        print(json.dumps(entry), flush=True)
        if process.returncode:
            raise SystemExit(process.returncode)
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source
    save()
