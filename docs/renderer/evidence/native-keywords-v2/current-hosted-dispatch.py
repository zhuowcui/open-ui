"""Dispatch existing hardening for the fresh shared native value constructors."""
import datetime
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-keywords-current-0733955a')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
SOURCE = '7d6ffabfea1f72c90f97475488dba8a8058ac4c7'
BRANCH = 'agent/native-keywords-current-v1746'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == SOURCE
assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT, text=True)
checks = json.loads((RAW / 'native-keywords-current-checks-v1749/receipt.json').read_bytes())
assert checks['source']['commit'] == SOURCE and checks['all_commands_terminal']
assert len(checks['checks']) == 16 and all(c['observed_exit_code'] == 0 for c in checks['checks'])
receipt = RAW / 'native-keywords-current-dispatch-v1753.json'
assert not receipt.exists()
report = dict(schema_version=1, source=SOURCE, branch=BRANCH, steps=[],
              release_qualification=False, native_pixel_qualification=False, probe_sha256=sha(Path(__file__)))
for name, command in [('push', ['git', 'push', '--quiet', 'origin', BRANCH]),
                      ('dispatch', ['gh', 'workflow', 'run', 'hardening.yml', '--ref', BRANCH])]:
    log = RAW / ('native-keywords-current-dispatch-v1753-' + name + '.log')
    with log.open('xb') as stream:
        result = subprocess.run(command, cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
    report['steps'].append(dict(name=name, actual_exit=result.returncode, log_path=str(log), log_sha256=sha(log)))
    report['dispatched_at_utc'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
    receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    assert result.returncode == 0
print(json.dumps(dict(dispatched=True, source=SOURCE, receipt_sha256=sha(receipt))), flush=True)
