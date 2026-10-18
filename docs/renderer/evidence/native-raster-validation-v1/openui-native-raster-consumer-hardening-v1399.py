import collections
import hashlib
import json
import subprocess
import time
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
OUT = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1/native-raster-consumer-hardening-v1399'
COMMIT = '3d4eea1184ac6703ae4b29de12ddb4cb75fe5ed3'
BRANCH = 'agent/native-raster-consumer-v1393'
assert not OUT.exists()
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
report = dict(schema_version=1, commit=COMMIT, branch=BRANCH, release_qualification=False,
              direct_cache_regression_qualification=False, all_commands_terminal=False,
              state='awaiting-own-source-manual-hardening', probe_sha256=sha(Path(__file__)))
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
def capture(command):
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return result.stdout
while True:
    candidates = json.loads(capture(['gh', 'run', 'list', '--branch', BRANCH, '--commit', COMMIT,
        '--workflow', 'hardening.yml', '--event', 'workflow_dispatch', '--limit', '10',
        '--json', 'databaseId,headSha,status,workflowName']))
    matches = [row for row in candidates if row['headSha'] == COMMIT and row['workflowName'] == 'v0.2 Hardening']
    assert len(matches) <= 1
    if matches:
        run_id = matches[0]['databaseId']
        break
    time.sleep(10)
report.update(run_id=run_id, state='running-own-source-manual-hardening')
save()
command = ['gh', 'run', 'view', str(run_id), '--json',
           'databaseId,headSha,url,conclusion,status,workflowName,jobs']
while True:
    row = json.loads(capture(command))
    assert row['headSha'] == COMMIT and row['workflowName'] == 'v0.2 Hardening'
    report['last_observed_jobs'] = [dict(name=job['name'], status=job['status'],
        conclusion=job['conclusion']) for job in row['jobs']]
    save()
    if row['status'] == 'completed':
        break
    time.sleep(10)
run_path = OUT / (str(run_id) + '.json')
run_path.write_text(json.dumps(row, sort_keys=True, indent=2) + '\n')
result = subprocess.run(['gh', 'run', 'view', str(run_id), '--log'], cwd=ROOT, capture_output=True)
log_path = OUT / (str(run_id) + '.log')
log_path.write_bytes(result.stdout + result.stderr)
assert result.returncode == 0
counts = collections.Counter(job['conclusion'] for job in row['jobs'])
report.update(all_commands_terminal=True, state='complete-requires-results-review',
    workflow=row, successful_jobs=counts['success'], skipped_jobs=counts['skipped'],
    job_conclusions=dict(counts), skipped_jobs_are_not_passes=True,
    all_observed_jobs_passed=row['conclusion'] == 'success' and counts['success'] == len(row['jobs']),
    workflow_capture_sha256=sha(run_path), workflow_log_sha256=sha(log_path))
save()
print(json.dumps({key: value for key, value in report.items() if key not in ['workflow', 'last_observed_jobs']}), flush=True)
raise SystemExit(int(not report['all_observed_jobs_passed']))
