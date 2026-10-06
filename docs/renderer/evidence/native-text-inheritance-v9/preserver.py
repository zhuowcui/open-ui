"""Preserve completed CI and native glyph analysis without claiming qualification."""
import gzip
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
sha = lambda b: hashlib.sha256(b).hexdigest()

def preserve(name, inputs, report):
    out = ROOT / 'docs/renderer/evidence' / name
    index = ROOT / 'docs/renderer/generated' / (name + '.json')
    assert not out.exists() and not index.exists()
    out.mkdir()
    rows = []
    for source, target, expected, compress in inputs + [(Path(__file__), 'preserver.py', None, False)]:
        source = Path(source)
        original = source.read_bytes()
        if expected:
            assert sha(original) == expected, str(source)
        destination = out / target
        assert not destination.exists()
        destination.write_bytes(gzip.compress(original, compresslevel=9, mtime=0) if compress else original)
        row = dict(path=str(destination.relative_to(ROOT)), sha256=sha(destination.read_bytes()),
                   bytes=destination.stat().st_size, source_path=str(source))
        if compress:
            row.update(encoding='gzip', decompressed_sha256=sha(original), decompressed_bytes=len(original))
        rows.append(row)
    report.update(schema_version=1, artifacts=rows, artifact_count=len(rows), artifact_bytes=sum(r['bytes'] for r in rows),
                  javascript_executed_by_openui=False, public_native_rust_apis_required=True,
                  release_qualification=False, pixel_tolerance=0, new_release_states_admitted=0)
    index.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
    for row in rows:
        data = (ROOT / row['path']).read_bytes()
        assert sha(data) == row['sha256']
        if row.get('encoding') == 'gzip':
            assert sha(gzip.decompress(data)) == row['decompressed_sha256']
    print(json.dumps(dict(index=str(index.relative_to(ROOT)), sha256=sha(index.read_bytes()),
                          artifacts=len(rows), bytes=report['artifact_bytes'])), flush=True)

checks_path = RAW / 'native-current-docs-checks-v1751/receipt.json'
checks = json.loads(checks_path.read_bytes())
assert checks['all_commands_terminal'] and checks['source'] == checks['source_after']
assert checks['source']['clean'] and checks['source']['commit'] == '2d78b2db1af129459d27454a40cb9eeabd914f7a'
assert len(checks['checks']) == 14 and all(r['observed_exit_code'] == 0 for r in checks['checks'])
inputs = [(checks_path, 'source-checks-terminal.json', sha(checks_path.read_bytes()), False),
          (Path('/tmp/openui-native-current-docs-checks-v1751.py'), 'source-checks.py', checks['probe_sha256'], False)]
for row in checks['checks']:
    inputs.append((checks_path.parent / (row['name'] + '.log'), 'source-' + row['name'] + '.log.gz', row['log_sha256'], True))
p = RAW / 'native-current-hosted-v1758/receipt.json'
hosted = json.loads(p.read_bytes())
assert sha(p.read_bytes()) == '754b5d964efb235e8a9b4293c54ac3697a412c5f709d362b44e198be01068835'
assert hosted['all_commands_terminal'] and hosted['successful_jobs'] == 6 and hosted['skipped_jobs'] == 5 and hosted['failed_jobs'] == 0
inputs += [(p, 'hosted-terminal.json', sha(p.read_bytes()), False),
           (Path('/tmp/openui-native-current-hosted-collect-v1758.py'), 'hosted-collector.py', hosted['probe_sha256'], False)]
for run in hosted['workflows']:
    inputs.append((Path(run['path']), str(run['run_id']) + '.json', run['sha256'], False))
for job in hosted['job_logs']:
    if job['conclusion'] != 'skipped':
        assert job['capture_actual_exit'] == 0
        inputs.append((Path(job['path']), str(job['job_id']) + '.log.gz', job['sha256'], True))
preserve('native-text-inheritance-v9', inputs,
         dict(source_checked=checks['source'], all_14_source_checks_passed=True,
              all_three_hosted_workflows_terminal=True, successful_jobs=6, skipped_jobs=5, failed_jobs=0,
              skipped_jobs_do_not_satisfy_release_gates=True, all_available_job_logs_preserved=True,
              full_chromium_pixel_gate_still_failed=True))
