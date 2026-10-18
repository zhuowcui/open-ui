import collections
import hashlib
import json
import os
import subprocess
import time
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
OUT = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1/native-keywords-hosted-v1590'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
EXPECTED = '06e1f89a4a2e7a53465bceb780675383d9748464'
RUN = 37340082754
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
report = dict(schema_version=1, owner_pid=os.getpid(), source=EXPECTED, run_id=RUN,
    run_url=f'https://github.com/zhuowcui/open-ui/actions/runs/{RUN}',
    all_commands_terminal=False, state='awaiting-own-source-manual-hardening',
    release_qualification=False, native_pixel_qualification=False,
    all_seven_jobs_passed=False, probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
while True:
    command = ['gh', 'run', 'view', str(RUN), '--json',
        'databaseId,headSha,url,conclusion,status,workflowName,jobs,event']
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    row = json.loads(result.stdout)
    assert row['headSha'] == EXPECTED and row['workflowName'] == 'v0.2 Hardening'
    assert row['event'] == 'workflow_dispatch'
    counts = collections.Counter(job['conclusion'] for job in row['jobs'])
    report.update(status=row['status'], conclusion=row['conclusion'], job_conclusions=dict(counts),
        jobs=[dict(name=job['name'], status=job['status'], conclusion=job['conclusion']) for job in row['jobs']])
    save()
    if row['status'] == 'completed':
        break
    time.sleep(10)
detail = OUT / f'{RUN}.json'
detail.write_text(json.dumps(row, sort_keys=True, indent=2) + '\n')
result = subprocess.run(['gh', 'run', 'view', str(RUN), '--log'], cwd=ROOT, capture_output=True)
log = OUT / f'{RUN}.log'
log.write_bytes(result.stdout + result.stderr)
assert result.returncode == 0
passed = row['conclusion'] == 'success' and len(row['jobs']) == 7 and counts['success'] == 7 and counts['skipped'] == 0
report.update(all_commands_terminal=True, state='terminal-results-recorded',
    all_seven_jobs_passed=passed, successful_jobs=counts['success'], skipped_jobs=counts['skipped'],
    actual_exit=int(not passed), detail_sha256=sha(detail), log_sha256=sha(log))
save()
print(json.dumps({k:v for k,v in report.items() if k != 'jobs'}), flush=True)
raise SystemExit(report['actual_exit'])
