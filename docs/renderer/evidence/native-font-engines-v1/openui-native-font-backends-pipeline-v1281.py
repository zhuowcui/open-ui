import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-font-backends-107e2e36')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-font-backends-pipeline-v1281'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'].startswith('0601cd30')
report = dict(schema_version=1, source=source, source_after=source,
              state='awaiting-complete-cache-and-fieldset-pipelines', all_commands_terminal=False,
              release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
              steps=[], probe_sha256=sha(Path(__file__)))
script_paths = [Path('/tmp/' + name) for name in [
    'openui-native-font-backends-guards-v1275.py', 'openui-native-font-backends-build-v1276.py',
    'openui-native-font-backends-sizing-v1279.py', 'openui-native-font-backends-consumer-v1277.py',
    'openui-native-font-backends-matrices-v1280.py', 'openui-native-font-backends-static-v1278.py']]
report['immutable_probe_hashes'] = {str(path): sha(path) for path in script_paths}
report['selection'] = dict(path=str(RAW / 'native-font-backends-selection-v1280.json'),
    sha256=sha(RAW / 'native-font-backends-selection-v1280.json'))
report['ordered_build_and_image_stages'] = True
report['physical_fontations_causation_proven'] = False
report['c_raster_configuration_parity'] = 'unfinished required API work'
report['javascript_executed_by_openui'] = False
report['old_openui_pixels_are_provenance_only'] = True
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
priorpaths = [RAW / name / 'receipt.json' for name in [
    'native-lcd-pipeline-v1219', 'scene-recording-cache-pipeline-v1233', 'native-fieldset-pipeline-v1265']]
def prior_complete(path):
    try:
        return json.loads(path.read_bytes())['all_commands_terminal']
    except json.JSONDecodeError:
        return False
while not all(prior_complete(path) for path in priorpaths):
    time.sleep(5)
report['prior_terminal_pipeline_sha256'] = {str(path): sha(path) for path in priorpaths}
save()
stages = [
    ('regression-guards', [sys.executable, '/tmp/openui-native-font-backends-guards-v1275.py'],
     RAW / 'native-font-backends-guards-v1275/receipt.json'),
    ('native-build', [sys.executable, '/tmp/openui-native-font-backends-build-v1276.py'],
     RAW / 'native-font-backends-clean-v1276/build.json'),
    ('intrinsic-native', [sys.executable, '/tmp/openui-native-font-backends-sizing-v1279.py'],
     RAW / 'native-font-backends-sizing-v1279/receipt.json'),
    ('font-native', [sys.executable, '/tmp/openui-native-font-backends-consumer-v1277.py'],
     RAW / 'native-font-backends-consumer-v1277/receipt.json'),
    ('static-native', [sys.executable, '/tmp/openui-native-font-backends-static-v1278.py'],
     RAW / 'native-font-backends-static-v1278/receipt.json'),
] + [(name, [sys.executable, '/tmp/openui-native-font-backends-matrices-v1280.py', name],
      RAW / f'native-font-backends-clean-{name}-v1280/{"full" if name == "selection" else name}-summary.json')
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
    terminal = terminalpath.exists() and (name not in ('regression-guards', 'intrinsic-native', 'font-native', 'static-native')
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
