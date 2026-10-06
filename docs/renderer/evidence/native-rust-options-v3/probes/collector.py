"""Record every job and available log for the measured umbrella head."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = RAW / 'native-rust-integrated-hosted-v1819'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir(); OUT.symlink_to(STORE,target_is_directory=True)
HEAD = '16187f4f54e1b62ea317a155f620c42f91a38c72'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
report = dict(schema_version=1, source=HEAD, workflows=[], job_logs=[],
              all_commands_terminal=False, release_qualification=False,
              javascript_executed_by_openui=False, probe_sha256=sha(Path(__file__)))
for run in [37407697770, 37407697750, 37407697920, 37407693299]:
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
guard_names = ['native_raster_selection_survives_callbacks_cloning_and_resize',
               'default_native_constructors_preserve_rendering_and_configuration',
               'window_and_headless_apps_keep_explicit_native_raster_selection',
               'selecting_ganesh_does_not_fall_back_to_cpu_raster',
               'tests::native_fragment_keywords_reach_engine_and_keep_owned_property_identity',
               'property::tests::native_fragment_keyword_values_cover_declared_enum_variants']
parity = next(j for j in report['job_logs'] if j['name'] == 'rust-parity')
parity_text = Path(parity['path']).read_text()
report['native_guard_log_observations'] = {name: 'test ' + name + ' ... ok' in parity_text for name in guard_names}
report['all_six_native_guards_executed_and_passed_in_hosted_parity'] = all(report['native_guard_log_observations'].values())
report['skipped_jobs_are_not_release_passes'] = True
p = OUT / 'receipt.json'
p.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(receipt=str(p), sha256=sha(p), successful_jobs=report['successful_jobs'],
                      skipped_jobs=report['skipped_jobs'], failed_jobs=report['failed_jobs'])), flush=True)
