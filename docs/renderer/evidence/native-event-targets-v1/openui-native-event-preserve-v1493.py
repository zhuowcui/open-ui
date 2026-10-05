"""Preserve reviewed native event evidence without changing reference bytes."""
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
OUT = ROOT / 'docs/renderer/evidence/native-event-targets-v1'
INDEX = ROOT / 'docs/renderer/generated/native-event-targets-v1.json'
assert not OUT.exists() and not INDEX.exists()
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == '1c8540e9f8c6eeb8fce10a76cb9a32c2f42f01fd'
assert not subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)
audit_path = RAW / 'native-event-complete-audit-v1492.json'
audit = json.loads(audit_path.read_bytes())
assert audit['all_commands_terminal'] and audit['api_checkpoint_ready_for_umbrella']
assert not audit['release_qualification'] and not audit['full_renderer_gate_passed']
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
OUT.mkdir()
artifacts = {}

def retain(source, relative):
    target = OUT / relative
    assert not target.exists()
    target.parent.mkdir(parents=True, exist_ok=True)
    data = source.read_bytes()
    target.write_bytes(data)
    assert target.read_bytes() == data
    artifacts[str(target.relative_to(ROOT))] = dict(bytes=len(data), sha256=sha(target), original=str(source))

for name in ['native-event-complete-audit-v1492.json', 'native-event-targets-audit-v1472.json',
             'native-event-small-matrices-audit-v1474.json', 'native-event-original-matrix-audit-v1489.json']:
    retain(RAW / name, name)
for directory in ['native-event-targets-guards-v1456', 'native-event-targets-clean-v1456',
                  'native-event-targets-pipeline-v1457', 'native-event-targets-hosted-v1463']:
    for path in sorted((RAW / directory).iterdir()):
        if path.is_file() and path.suffix in ['.json', '.log']:
            retain(path, str(Path(directory) / path.name))
consumer = RAW / 'native-event-targets-consumer-v1456'
for path in sorted(consumer.rglob('*')):
    if path.is_file():
        retain(path, str(Path(consumer.name) / path.relative_to(consumer)))
for name in ['openui-native-event-complete-audit-v1492.py', 'openui-native-event-targets-audit-v1472.py',
             'openui-native-event-small-matrices-audit-v1474.py', 'openui-native-event-original-matrix-audit-v1489.py',
             'openui-native-event-targets-guards-v1456.py', 'openui-native-event-targets-build-v1456.py',
             'openui-native-event-targets-consumer-v1456.py', 'openui-native-event-targets-matrices-v1456.py',
             'openui-native-event-targets-pipeline-v1457.py', 'openui-native-event-preserve-v1493.py']:
    path = Path('/tmp') / name
    assert path.exists(), name
    retain(path, name)
for suite in ['focused', 'primitive', 'full', 'expanded']:
    path = RAW / f'native-event-targets-clean-{suite}-v1456/{suite}-summary.json'
    summary = json.loads(path.read_bytes())
    metadata = {k: v for k, v in summary.items() if k != 'profiles'}
    metadata['profile_metadata'] = [{k: v for k, v in profile.items() if k != 'tests'} for profile in summary['profiles']]
    metadata['complete_summary_sha256'] = sha(path)
    metadata['complete_summary_bytes'] = path.stat().st_size
    target = OUT / f'{suite}-summary-metadata.json'
    target.write_text(json.dumps(metadata, sort_keys=True, indent=2) + '\n')
    artifacts[str(target.relative_to(ROOT))] = dict(bytes=target.stat().st_size, sha256=sha(target), original=str(path), kind='metadata-only')
    retain(RAW / f'native-event-targets-{suite}-exit-v1456.json', f'{suite}-exit.json')
    retain(RAW / f'native-event-targets-clean-{suite}-v1456.log', f'{suite}-exit.log')
report = dict(schema_version=1, source=audit['source'], source_after=audit['source_after'],
              applied_to_umbrella=True, scoped_native_event_behavior_verified=True,
              release_qualification=False, full_renderer_gate_passed=False,
              new_release_states_admitted=0, javascript_executed_by_openui=False,
              pixel_target='pinned-chromium-only', pixel_tolerance=0,
              public_rust_api=audit['public_rust_api'], workspace=audit['workspace'], c_abi=audit['c_abi'],
              native_app=audit['native_app'], hosted=audit['hosted'], suites=audit['suites'],
              original_rows_identical_in_expanded=audit['original_rows_identical_in_expanded'],
              addition_cases=audit['addition_cases'], addition_cases_four_profile_exact=audit['addition_cases_four_profile_exact'],
              all_comparison_invariants_unchanged=audit['all_comparison_invariants_unchanged'],
              artifacts=artifacts, artifact_count=len(artifacts), artifact_bytes=sum(a['bytes'] for a in artifacts.values()),
              cargo_commands_run=0, screenshots_generated=0, probe_sha256=sha(Path(__file__)))
INDEX.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(path=str(INDEX), sha256=sha(INDEX), artifacts=len(artifacts), artifact_bytes=report['artifact_bytes'],
                     api_verified=True, release_qualification=False)), flush=True)
