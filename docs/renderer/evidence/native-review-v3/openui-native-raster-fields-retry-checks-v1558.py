"""Run read-only generators and accountability on the exact candidate."""
import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-raster-fields-retry-e0dc491e')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-raster-fields-retry-checks-v1558'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == 'e0dc491e61e17ce4407ff2dd30289e741690572b'
checks = json.loads((RAW / 'native-scroll-docs-checks-v543.json').read_bytes())['checks']
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
