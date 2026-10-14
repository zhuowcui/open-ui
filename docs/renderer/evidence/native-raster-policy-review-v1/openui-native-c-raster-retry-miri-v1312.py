import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-c-raster-consumer-3395cefa')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-c-raster-retry-miri-v1312'
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
           CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0', PYTHONDONTWRITEBYTECODE='1',
           RUSTUP_HOME='/dev/shm/openui-miri-nightly-v1312',
           XDG_CACHE_HOME='/dev/shm/openui-miri-cache-v1312', MIRIFLAGS='-Zmiri-strict-provenance')
base = ['cargo', '+nightly-2026-09-01', '--config', '.cargo/config.chromium.toml', 'miri']
steps = [
    ('toolchain', ['rustup', 'toolchain', 'install', 'nightly-2026-09-01', '--profile', 'minimal',
        '--component', 'miri,rust-src']),
    ('sysroot', base + ['setup']),
    ('prefix-test', base + ['test', '--locked', '-p', 'openui-ffi', '--lib',
        'raster_configuration::tests::c_raster_options_reject_short_headers_bad_fields_and_preserve_outputs',
        '--', '--exact']),
]
report = dict(schema_version=1, source=source, source_after=source,
              release_qualification=False, all_commands_terminal=False, steps=[],
              strict_provenance=True, test_executed=False,
              isolated_toolchain_path=env['RUSTUP_HOME'], cache_path=env['XDG_CACHE_HOME'],
              probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
try:
    for name, command in steps:
        log = OUT / (name + '.log')
        with log.open('xb') as stream:
            process = subprocess.run(command, cwd=ROOT / 'bindings/rust', env=env,
                stdout=stream, stderr=subprocess.STDOUT)
        row = dict(name=name, command=command, observed_exit_code=process.returncode,
                   log_sha256=sha(log))
        report['steps'].append(row)
        if name == 'prefix-test':
            report['test_executed'] = True
            if process.returncode == 0:
                assert b'1 passed; 0 failed;' in log.read_bytes()
        save()
        print(json.dumps(row), flush=True)
        if process.returncode:
            raise SystemExit(process.returncode)
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source
    save()
