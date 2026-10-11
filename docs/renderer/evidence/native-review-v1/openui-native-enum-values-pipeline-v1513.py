CONFIG = {'root': '/dev/shm/openui-native-enum-values-619a465c', 'commit': '893ea292cacf49b50240ba3176f4dc9ff55829d7', 'name': 'native-enum-values-pipeline-v1513', 'initial_state': 'awaiting-all-25-prior-whole-pipelines', 'prior_pipelines': ['cache-umbrella-pipeline-v1291', 'native-fieldset-retry-pipeline-v1310', 'native-font-retry-pipeline-v1311', 'native-c-raster-retry-pipeline-v1312', 'native-raster-fields-pipeline-v1337', 'native-default-strike-pipeline-v1338', 'cache-integration-pipeline-v1341', 'native-fieldset-capture-forensics-pipeline-v1346', 'native-style-integration-pipeline-v1354', 'native-intrinsic-constraints-pipeline-v1375', 'native-intrinsic-constraints-pipeline-v1369', 'native-raster-consumer-pipeline-v1398', 'umbrella-clean-workspace-pipeline-v1411', 'native-style-integration-retry-pipeline-v1419', 'native-intrinsic-constraints-retry-pipeline-v1420', 'native-raster-api-pipeline-v1425', 'native-image-occlusion-pipeline-v1437', 'native-image-coverage-pipeline-v1449', 'native-event-targets-pipeline-v1457', 'native-border-contrast-pipeline-v1462', 'native-intrinsic-cache-guard-pipeline-v1469', 'native-raster-fields-public-pipeline-v1470', 'native-image-imports-pipeline-v1471', 'native-rounded-border-pipeline-v1482', 'native-border-guard-pipeline-v1501'], 'selections': ['native-enum-values-checks-v1511/receipt.json'], 'hard_stop_stages': ['guards', 'native-build'], 'scripts': ['openui-native-enum-values-guards-v1512.py', 'openui-native-enum-values-build-v1512.py', 'openui-native-enum-values-consumer-v1512.py', 'openui-native-enum-values-matrices-v1512.py', 'openui-native-image-coverage-fieldsets-v1448.py'], 'stages': [('guards', ['/usr/bin/python3', '/tmp/openui-native-enum-values-guards-v1512.py'], 'native-enum-values-guards-v1512/receipt.json', True), ('native-build', ['/usr/bin/python3', '/tmp/openui-native-enum-values-build-v1512.py'], 'native-enum-values-clean-v1512/build.json', True), ('native-application', ['/usr/bin/python3', '/tmp/openui-native-enum-values-consumer-v1512.py'], 'native-enum-values-consumer-v1512/receipt.json', True), ('focused', ['/usr/bin/python3', '/tmp/openui-native-enum-values-matrices-v1512.py', 'focused'], 'native-enum-values-clean-focused-v1512/focused-summary.json', False), ('primitive', ['/usr/bin/python3', '/tmp/openui-native-enum-values-matrices-v1512.py', 'primitive'], 'native-enum-values-clean-primitive-v1512/primitive-summary.json', False), ('full', ['/usr/bin/python3', '/tmp/openui-native-enum-values-matrices-v1512.py', 'full'], 'native-enum-values-clean-full-v1512/full-summary.json', False), ('expanded', ['/usr/bin/python3', '/tmp/openui-native-enum-values-matrices-v1512.py', 'expanded'], 'native-enum-values-clean-expanded-v1512/expanded-summary.json', False)]}
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path

# CONFIG is inserted as a literal into each independently owned pipeline.
ROOT = Path(CONFIG['root'])
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / CONFIG['name']
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == CONFIG['commit']
scripts = [Path('/tmp') / name for name in CONFIG['scripts']]
priorpaths = [RAW / name / 'receipt.json' for name in CONFIG['prior_pipelines']]
selections = [RAW / name for name in CONFIG.get('selections', [])]
report = dict(schema_version=1, source=source, source_after=source,
    state=CONFIG['initial_state'], all_commands_terminal=False,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    ordered_build_and_image_stages=True, steps=[], probe_sha256=sha(Path(__file__)),
    immutable_probe_hashes={str(p): sha(p) for p in scripts},
    selection_hashes={str(p): sha(p) for p in selections},
    prior_whole_pipelines=[str(p) for p in priorpaths],
    javascript_executed_by_openui=False, old_openui_pixels_are_provenance_only=True,
    consumer_compile_correction_only=False, renderer_production_unchanged_from_applied_renderer=True,
    original_pixel_gate_remains_required=True)

report.update(owner_pid=os.getpid(), baseline_commit='33a6578b4c49bf83fbd40c77e409782d0e8f1e23', expected_stages=7,
    workspace_cleaned_at_every_source_switch=True, existing_113_exports_and_30_layouts_preserved=True,
    native_event_api_preserved=True, c_abi_unchanged=True, actual_pixel_gain_claimed=False,
    root_cause_owner='openui-style shared native value construction', native_constructor_qualification_pending=True,
    strict_named_baseline_failure_required=True, public_rust_callbacks_and_owned_bounds=True,
    strict_chromium_capture_pairs=True, preserves_both_unstable_reference_captures=True)
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

def complete(path):
    try:
        return json.loads(path.read_bytes())['all_commands_terminal'] is True
    except (FileNotFoundError, json.JSONDecodeError):
        return False

while not all(complete(p) for p in priorpaths):
    time.sleep(5)
report['prior_terminal_pipeline_sha256'] = {str(p): sha(p) for p in priorpaths}
save()
for stage in CONFIG['stages']:
    name, command, terminal_name, has_terminal_flag = stage
    terminalpath = RAW / terminal_name
    assert all(complete(p) for p in priorpaths)
    assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
    assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
    assert repository_source_identity(ROOT) == source
    assert all(sha(p) == report['immutable_probe_hashes'][str(p)] for p in scripts)
    assert all(sha(p) == report['selection_hashes'][str(p)] for p in selections)
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
    row = dict(name=name, command=command, observed_exit_code=process.returncode, log_sha256=sha(log))
    report['steps'].append(row)
    report['source_after'] = repository_source_identity(ROOT)
    assert report['source_after'] == source
    terminal = terminalpath.exists() and (not has_terminal_flag or
        json.loads(terminalpath.read_bytes()).get('all_commands_terminal') is True)
    if not terminal or name in CONFIG['hard_stop_stages'] and process.returncode:
        report.update(state='stopped-on-build-guard-or-harness-failure-' + name, all_commands_terminal=True)
        save()
        raise SystemExit(process.returncode or 1)
    save()
    print(json.dumps(row), flush=True)
report.update(state='complete-requires-results-review', all_commands_terminal=True)
save()
raise SystemExit(int(any(row['observed_exit_code'] for row in report['steps'])))
