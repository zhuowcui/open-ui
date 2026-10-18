import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-keywords-pixels-7d6ffabf-v1785')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
suite = sys.argv[1]
assert suite in ('focused', 'primitive', 'full', 'expanded')
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
result = RAW / ('native-keywords-capture-clean-' + suite + '-v1785')
log = RAW / ('native-keywords-capture-clean-' + suite + '-v1785.log')
storage = Path('/mnt/e/openui-v02-qualification-d174ea0b') / result.name
assert not result.exists() and not log.exists() and not storage.exists()
storage.mkdir()
result.symlink_to(storage, target_is_directory=True)
command = [sys.executable, str(ROOT / 'tools/qualification/run_renderer_matrix.py'),
           '--suite', suite, '--results-dir', str(result),
           '--cache-dir', '/mnt/e/openui-v02-qualification-d174ea0b/renderer-cache',
           '--oracle-cache-dir', '/home/nero/code/open-ui/out/renderer-qualification-cache/chromium-oracle',
           '--pixel-compare', str(RAW / 'native-keywords-current-retry-clean-v1779/pixel_compare'),
           '--chrome', '/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome',
           '--raster-backend', 'cpu-skia', '--jobs', '8' if suite in ('full', 'expanded') else '2']
with log.open('xb') as stream:
    process = subprocess.run(command, cwd=ROOT, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'),
                             stdout=stream, stderr=subprocess.STDOUT)
entry = {'schema_version': 1, 'suite': suite, 'observed_exit_code': process.returncode, 'command': command,
         'log_sha256': hashlib.sha256(log.read_bytes()).hexdigest(), 'release_qualification': False}
(RAW / ('native-keywords-capture-' + suite + '-exit-v1785.json')).write_text(json.dumps(entry, sort_keys=True, indent=2) + '\n')
print(json.dumps({'suite':suite,'observed_exit_code':process.returncode}),flush=True)
raise SystemExit(process.returncode)
