import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-c-raster-0601cd30')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-c-raster-pipeline-v1299'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '3395cefad116fa93a2db8f839ff707fb84230a3a'
script_paths = [Path('/tmp') / name for name in [
    'openui-native-c-raster-build-v1295.py', 'openui-native-c-raster-miri-v1296.py',
    'openui-native-c-raster-consumer-v1297.py', 'openui-native-c-raster-matrices-v1298.py']]
priorpaths = [RAW / name / 'receipt.json' for name in [
    'scene-recording-cache-pipeline-v1233', 'native-fieldset-pipeline-v1265',
    'native-font-backends-pipeline-v1281', 'cache-umbrella-pipeline-v1291']]
report = dict(schema_version=1, source=source, source_after=source,
    state='awaiting-entire-cache-fieldset-font-and-clean-cache-pipelines', all_commands_terminal=False,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    steps=[], probe_sha256=sha(Path(__file__)), ordered_build_and_image_stages=True,
    immutable_probe_hashes={str(p): sha(p) for p in script_paths},
    javascript_executed_by_openui=False, old_openui_pixels_are_provenance_only=True,
    c_raster_configuration_parity='prepared, uncompiled and unqualified',
    original_pixel_gate_remains_required=True, prior_whole_pipelines=[str(p) for p in priorpaths],
    existing_exports_preserved=113, proposed_exports=117,
    inherits_unapplied_intrinsic_and_font_trials=True)
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

def prior_complete(path):
    try:
        return json.loads(path.read_bytes())['all_commands_terminal'] is True
    except json.JSONDecodeError:
        return False

while not all(prior_complete(p) for p in priorpaths):
    time.sleep(5)
report['prior_terminal_pipeline_sha256'] = {str(p): sha(p) for p in priorpaths}
save()
stages = [
    ('native-build', [sys.executable, '/tmp/openui-native-c-raster-build-v1295.py'],
        RAW / 'native-c-raster-clean-v1295/build.json'),
    ('miri-prefix', [sys.executable, '/tmp/openui-native-c-raster-miri-v1296.py'],
        RAW / 'native-c-raster-miri-v1296/receipt.json'),
    ('font-native', [sys.executable, '/tmp/openui-native-c-raster-consumer-v1297.py'],
        RAW / 'native-c-raster-consumer-v1297/receipt.json'),
] + [(name, [sys.executable, '/tmp/openui-native-c-raster-matrices-v1298.py', name],
       RAW / f'native-c-raster-clean-{name}-v1298/{name}-summary.json')
      for name in ('focused', 'primitive', 'full', 'expanded')]
for name, command, terminalpath in stages:
    assert all(prior_complete(p) for p in priorpaths)
    assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
    assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
    assert repository_source_identity(ROOT) == source
    assert all(sha(p) == report['immutable_probe_hashes'][str(p)] for p in script_paths)
    report['state'] = 'running-' + name
    save()
    log = OUT / (name + '.log')
    with log.open('xb') as stream:
        process = subprocess.Popen(command, cwd=ROOT,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'), stdout=stream, stderr=subprocess.STDOUT)
        report['current_process'] = dict(pid=process.pid, program=Path(command[1]).name)
        save()
        process.wait()
    report.pop('current_process')
    report['steps'].append(dict(name=name, command=command, observed_exit_code=process.returncode,
        log_sha256=sha(log)))
    report['source_after'] = repository_source_identity(ROOT)
    assert report['source_after'] == source
    terminal = terminalpath.exists() and (name in ('focused', 'primitive', 'full', 'expanded')
        or json.loads(terminalpath.read_bytes()).get('all_commands_terminal') is True)
    if not terminal or name == 'native-build' and process.returncode:
        report.update(state='stopped-on-build-or-harness-failure-' + name, all_commands_terminal=True)
        save()
        raise SystemExit(process.returncode or 1)
    save()
    print(json.dumps(report['steps'][-1]), flush=True)
report.update(state='complete-requires-results-review', all_commands_terminal=True)
save()
raise SystemExit(int(any(r['observed_exit_code'] for r in report['steps'])))
