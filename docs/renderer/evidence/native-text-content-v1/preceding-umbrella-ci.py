import collections
import hashlib
import json
import subprocess
import time
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
OUT = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1/native-table-source-umbrella-ci-v1607'
assert not OUT.exists()
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
def capture(command):
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
    assert result.returncode == 0, result.stderr
    return result.stdout
expected = 'f25cd72205a7fc6ce73496f0119749f4ddfeb59d'
assert capture(['git', 'rev-parse', 'HEAD']).strip() == expected
assert capture(['git', 'status', '--porcelain']) == ''
wanted = {'v0.2 Hardening', 'CI', 'Format Check'}
while True:
    candidates = json.loads(capture(['gh', 'run', 'list', '--branch', 'agent/v02-final-closure', '--commit', expected, '--limit', '30', '--json', 'databaseId,workflowName,headSha,status']))
    by_name = {row['workflowName']: row for row in candidates if row['headSha'] == expected and row['workflowName'] in wanted}
    if set(by_name) == wanted:
        break
    time.sleep(10)
rows = []
for name in sorted(wanted):
    run = by_name[name]['databaseId']
    command = ['gh', 'run', 'view', str(run), '--json', 'databaseId,headSha,url,conclusion,status,workflowName,jobs']
    while True:
        row = json.loads(capture(command))
        assert row['headSha'] == expected and row['workflowName'] == name
        if row['status'] == 'completed':
            break
        time.sleep(10)
    (OUT / (str(run) + '.json')).write_text(json.dumps(row, sort_keys=True, indent=2) + '\n')
    result = subprocess.run(['gh', 'run', 'view', str(run), '--log'], cwd=ROOT, capture_output=True)
    log = OUT / (str(run) + '.log')
    log.write_bytes(result.stdout + result.stderr)
    assert result.returncode == 0
    row['captured_log_sha256'] = sha(log)
    rows.append(row)
    print(json.dumps({'workflow': name, 'run': run, 'conclusion': row['conclusion']}), flush=True)
counts = collections.Counter(job['conclusion'] for row in rows for job in row['jobs'])
report = {'schema_version': 1, 'commit': expected, 'release_qualification': False, 'private_runtime_qualification': False,
          'all_three_hosted_workflows_complete_success': all(row['conclusion'] == 'success' for row in rows),
          'successful_jobs': counts['success'], 'skipped_jobs': counts['skipped'], 'skipped_jobs_are_not_passes': True,
          'job_conclusions': dict(counts), 'workflows': rows, 'probe_sha256': sha(Path(__file__))}
(OUT / 'receipt.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({key: value for key, value in report.items() if key != 'workflows'}), flush=True)
raise SystemExit(int(not report['all_three_hosted_workflows_complete_success']))
