"""Run read-only generators and accountability on the exact candidate."""
import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-rust-integrated-current-checks-v1818'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
checks = json.loads((RAW / 'native-scroll-docs-checks-v543.json').read_bytes())['checks']
extra = json.loads((RAW / 'native-intrinsic-snap-checks-v1693/receipt.json').read_bytes())['checks'][10:]
for item in extra:
    item['command'] = [part.replace('/dev/shm/openui-native-intrinsic-snap-41b616c3', str(ROOT)).replace(' bindings/rust/openui/tests/native_intrinsic_snap.rs', '') for part in item['command']]
checks.extend(extra)
assert len(checks) == 13
format_paths = [p for p in subprocess.check_output(['git', 'ls-files', '-z', '--', 'src', 'include', 'examples'], cwd=ROOT, text=True).split('\0') if p and Path(p).suffix in ['.cc', '.h', '.c'] and p not in ['include/openui.h', 'include/openui_style_properties.h']]
assert len(format_paths) == 60
checks.append(dict(name='clang-format', command=['/mnt/e/openui-v02-format-tools-v1724/clang_format/data/bin/clang-format', '--dry-run', '--Werror', *format_paths]))
assert len(checks) == 14
for item in checks:
    item['command'] = [part.replace('/home/nero/code/open-ui/bindings/rust', str(ROOT / 'bindings/rust')) for part in item['command']]
checks.append(dict(name='native-options-rust-format', command=['bash','-c','ulimit -s 262144; rustfmt --edition 2021 --config skip_children=true --check bindings/rust/openui/src/document.rs bindings/rust/openui/src/app.rs bindings/rust/openui/src/prelude.rs bindings/rust/openui/tests/native_raster_options.rs bindings/rust/openui/examples/native_raster_options.rs']))

assert source['commit'] == '16187f4f54e1b62ea317a155f620c42f91a38c72'

def run(check):
    log = OUT / (check['name'] + '.log')
    with log.open('xb') as stream:
        result = subprocess.run(check['command'], cwd=ROOT,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'), stdout=stream, stderr=subprocess.STDOUT)
    return dict(name=check['name'], command=check['command'], observed_exit_code=result.returncode,
        log_sha256=sha(log))
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    rows = list(pool.map(run, checks))
after = repository_source_identity(ROOT)
assert source == after
report = dict(schema_version=1, source=source, source_after=after, checks=rows,
    release_qualification=False, all_commands_terminal=True, cargo_commands_run=0,
    screenshots_generated=0, probe_sha256=sha(Path(__file__)))
(OUT / 'receipt.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({row['name']: row['observed_exit_code'] for row in rows}), flush=True)
raise SystemExit(int(any(row['observed_exit_code'] for row in rows)))
