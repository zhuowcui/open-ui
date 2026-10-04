import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/tmp/openui-native-fieldset-intrinsic-c170aa7e')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-fieldset-pipeline-v1265'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '2518bdcd3934d0f67ec5e0232313cf5eda8d7abc'
report = dict(schema_version=1, source=source, source_after=source,
              state='awaiting-separate-lcd-and-cache-pipelines', all_commands_terminal=False,
              release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
              steps=[], probe_sha256=sha(Path(__file__)))
script_paths = [Path('/tmp/' + name) for name in [
    'openui-native-fieldset-guards-v1261.py', 'openui-native-fieldset-build-v1262.py',
    'openui-native-fieldset-sizing-v1266.py', 'openui-native-fieldset-consumer-v1263.py',
    'openui-native-fieldset-matrices-v1267.py']]
report['immutable_probe_hashes'] = {str(path): sha(path) for path in script_paths}
report['selection'] = dict(path=str(RAW / 'native-fieldset-selection-v1267.json'),
    sha256=sha(RAW / 'native-fieldset-selection-v1267.json'))
report['ordered_build_and_image_stages'] = True
report['fieldset_causation_proven'] = False
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
priorpaths = [RAW / name / 'receipt.json' for name in [
    'native-lcd-pipeline-v1219', 'scene-recording-cache-pipeline-v1233']]
while not all(json.loads(path.read_bytes())['all_commands_terminal'] for path in priorpaths):
    time.sleep(5)
report['prior_terminal_pipeline_sha256'] = {str(path): sha(path) for path in priorpaths}
save()
stages = [
    ('regression-guards', [sys.executable, '/tmp/openui-native-fieldset-guards-v1261.py'],
     RAW / 'native-fieldset-guards-v1261/receipt.json'),
    ('native-build', [sys.executable, '/tmp/openui-native-fieldset-build-v1262.py'],
     RAW / 'native-fieldset-clean-v1262/build.json'),
    ('intrinsic-native', [sys.executable, '/tmp/openui-native-fieldset-sizing-v1266.py'],
     RAW / 'native-fieldset-sizing-v1266/receipt.json'),
    ('fieldset-native', [sys.executable, '/tmp/openui-native-fieldset-consumer-v1263.py'],
     RAW / 'native-fieldset-consumer-v1263/receipt.json'),
] + [(name, [sys.executable, '/tmp/openui-native-fieldset-matrices-v1267.py', name],
      RAW / f'native-fieldset-clean-{name}-v1267/{"full" if name == "selection" else name}-summary.json')
     for name in ['selection', 'focused', 'primitive', 'full', 'expanded']]
for name, command, terminalpath in stages:
    assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
    assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
    assert repository_source_identity(ROOT) == source
    assert all(sha(path) == report['immutable_probe_hashes'][str(path)] for path in script_paths)
    assert sha(Path(report['selection']['path'])) == report['selection']['sha256']
    report['state'] = 'running-' + name
    log = OUT / (name + '.log')
    save()
    with log.open('xb') as stream:
        process = subprocess.Popen(command, cwd=ROOT, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'),
                                   stdout=stream, stderr=subprocess.STDOUT)
        report['current_process'] = dict(pid=process.pid, program=Path(command[1]).name)
        save()
        process.wait()
    report.pop('current_process')
    report['steps'].append(dict(name=name, command=command, observed_exit_code=process.returncode,
                                log_sha256=sha(log)))
    report['source_after'] = repository_source_identity(ROOT)
    assert report['source_after'] == source
    terminal = terminalpath.exists() and (name not in ('regression-guards', 'intrinsic-native', 'fieldset-native')
        or json.loads(terminalpath.read_bytes()).get('all_commands_terminal'))
    if not terminal or name in ('regression-guards', 'native-build') and process.returncode:
        report.update(state='stopped-on-harness-build-or-guard-failure-' + name, all_commands_terminal=True)
        save()
        raise SystemExit(process.returncode or 1)
    save()
    print(json.dumps(report['steps'][-1]), flush=True)
report.update(state='complete-requires-results-review', all_commands_terminal=True)
save()
raise SystemExit(int(any(row['observed_exit_code'] for row in report['steps'])))
