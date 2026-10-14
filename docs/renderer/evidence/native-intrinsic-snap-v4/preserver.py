"""Preserve the real formatting failure, its narrow fix, and the successful width retry."""
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

p = RAW / 'native-intrinsic-snap-hosted-retry-v1727/receipt.json'
retry = json.loads(p.read_bytes())
assert sha(p.read_bytes()) == 'c55b6ea844c5e2bf1b0744bcecf25f2819da46f23e05bc7f3f1e28fe4c6661d4'
assert retry['source'] == '727da10e580c9439f5db83d36bfc3e2340168207'
assert retry['all_commands_terminal'] and retry['all_seven_jobs_passed'] and retry['job_conclusions'] == dict(success=7)
inputs = [(p, 'hosted-attempt2-terminal.json', sha(p.read_bytes()), False),
          (p.parent / 'attempt2.json', 'hosted-attempt2.json', retry['metadata_sha256'], False),
          (Path('/tmp/openui-native-intrinsic-snap-hosted-retry-v1727.py'), 'hosted-retry-observer.py', retry['probe_sha256'], False)]
for job in retry['job_logs']:
    assert job['capture_actual_exit'] == 0
    inputs.append((Path(job['path']), str(job['job_id']) + '.log.gz', job['sha256'], True))
preserve('native-intrinsic-snap-v4', inputs,
         dict(source=retry['source'], all_hosted_jobs_terminal=True, all_available_hosted_logs_preserved=True,
              successful_jobs=7, skipped_jobs=0, retry_attempt=2,
              previous_cancelled_jobs_preserved_in_v2=True, candidate_applied_to_umbrella=False,
              full_native_app_and_renderer_matrices_unexecuted=True))

p = RAW / 'native-text-style-integrated-checks-v1720/receipt.json'
checks = json.loads(p.read_bytes())
assert checks['all_commands_terminal'] and checks['source'] == checks['source_after']
assert checks['source']['commit'] == '987c20d92e686dcf59bb2c677d5286c502de893e'
assert len(checks['checks']) == 13 and all(r['observed_exit_code'] == 0 for r in checks['checks'])
inputs = [(p, 'source-checks-terminal.json', 'f96efdefdb60390f7247c2d3c2ef9bff563e5ef2d2dea58a319d04d4c6d97e4d', False),
          (Path('/tmp/openui-native-text-style-integrated-checks-v1720.py'), 'source-checks.py', checks['probe_sha256'], False)]
for row in checks['checks']:
    inputs.append((p.parent / (row['name'] + '.log'), 'source-' + row['name'] + '.log.gz', row['log_sha256'], True))
for name, target, expected, compress in [
    ('native-text-style-umbrella-hosted-at-observation-v1722.json', 'umbrella-hosted-first-observation.json', '3a8f32e8a59ce4b0a65cc7cdd7214a9cf135dff143df4c8adffb70f85f32a71b', False),
    ('native-integrated-hosted-at-observation-v1726.json', 'hosted-later-observation.json', '042f664356bcca59af440719ee491e45f0f4bdb9a44648b93baf5d82671c0e00', False),
    ('native-integrated-format-112001221744-v1723.log', 'clang-format-failed.log.gz', '0eedd19a9f848b2e9f278e68a6ca5ac3520b43c0d30ef0ef6649795ce6191344', True),
    ('native-integrated-format-112001222185-v1726.log', 'gn-format-passed.log.gz', '5d2079944b12ca3ba3ef606aefc9028d841773863242315c67fabd7b0513de23', True),
    ('native-integrated-clang-format-correction-v1725.json', 'format-correction.json', None, False),
    ('native-integrated-clang-format-baseline-v1725.log', 'format-baseline.log.gz', '0bbe08673c2b247294cf0da19c35810211da93da888d5920552b3d17c84d97db', True),
    ('native-integrated-clang-format-fixed-v1725.log', 'format-fixed.log', 'e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855', False),
]:
    inputs.append((RAW / name, target, expected, compress))
correction = json.loads((RAW / 'native-integrated-clang-format-correction-v1725.json').read_bytes())
assert correction['baseline_actual_exit'] == 1 and correction['fixed_all_tracked_actual_exit'] == 0
assert correction['only_whitespace_changed'] and correction['tracked_files_checked'] == 58
assert sha((ROOT / 'examples/c_v02/text_content.c').read_bytes()) == correction['after_sha256']
patch = Path('/tmp/openui-native-c-format-correction-v1728.patch')
assert not patch.exists()
patch.write_bytes(subprocess.check_output(['git', 'diff', '--', 'examples/c_v02/text_content.c'], cwd=ROOT))
inputs.append((patch, 'c-format-correction.patch', sha(patch.read_bytes()), False))
preserve('native-text-inheritance-v7', inputs,
         dict(source_checked=checks['source'], all_13_source_checks_passed=True,
              hosted_formatting_failed_on_c_example=True, hosted_gn_formatting_passed=True,
              original_hosted_failure_not_rewritten=True, c_example_whitespace_correction_only=True,
              local_clang_format_version='18.1.3', all_58_tracked_c_cpp_format_checks_passed=True,
              rust_renderer_abi_oracle_and_workflows_unchanged=True, current_head_hosted_results_still_required=True,
              full_chromium_pixel_gate_still_failed=True))
