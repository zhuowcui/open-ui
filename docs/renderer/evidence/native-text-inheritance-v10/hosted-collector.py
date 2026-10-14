"""Record every job and available log for the measured umbrella head."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW / 'native-current-hosted-v1776'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir(); OUT.symlink_to(STORE,target_is_directory=True)
HEAD = 'e82493683e02498bb8c5be570a4df22708c88c0c'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
report = dict(schema_version=1, source=HEAD, workflows=[], job_logs=[],
              all_commands_terminal=False, release_qualification=False,
              javascript_executed_by_openui=False, probe_sha256=sha(Path(__file__)))
for run in [37394006700, 37394005463, 37394006724]:
    result = subprocess.run(['gh', 'run', 'view', str(run), '--json',
                             'databaseId,headSha,status,conclusion,attempt,jobs'],
                            cwd=ROOT, capture_output=True)
    assert result.returncode == 0
    metadata = OUT / (str(run) + '.json')
    metadata.write_bytes(result.stdout)
    d = json.loads(result.stdout)
    assert d['headSha'] == HEAD and d['status'] == 'completed'
    report['workflows'].append(dict(run_id=run, path=str(metadata), sha256=sha(metadata),
                                    conclusion=d['conclusion'], attempt=d['attempt']))
    for job in d['jobs']:
        assert job['status'] == 'completed'
        row = dict(run_id=run, job_id=job['databaseId'], name=job['name'],
                   conclusion=job['conclusion'], executed_steps=len(job['steps']))
        if job['conclusion'] != 'skipped':
            log = OUT / (str(job['databaseId']) + '.log')
            with log.open('xb') as stream:
                result = subprocess.run(['gh', 'run', 'view', str(run), '--job',
                                         str(job['databaseId']), '--log'], cwd=ROOT,
                                        stdout=stream, stderr=subprocess.STDOUT)
            row.update(capture_actual_exit=result.returncode, path=str(log), sha256=sha(log))
            assert result.returncode == 0
        report['job_logs'].append(row)
report.update(all_commands_terminal=True,
              successful_jobs=sum(j['conclusion'] == 'success' for j in report['job_logs']),
              skipped_jobs=sum(j['conclusion'] == 'skipped' for j in report['job_logs']),
              failed_jobs=sum(j['conclusion'] not in ['success', 'skipped'] for j in report['job_logs']))
p = OUT / 'receipt.json'
p.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(receipt=str(p), sha256=sha(p), successful_jobs=report['successful_jobs'],
                      skipped_jobs=report['skipped_jobs'], failed_jobs=report['failed_jobs'])), flush=True)
