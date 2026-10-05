CONFIG = {'root': '/dev/shm/openui-native-image-coverage-d913041d', 'commit': '2eacae2c8aad222850af30f9cea6f8a79bbddd4a', 'name': 'native-image-coverage-pipeline-v1449', 'initial_state': 'awaiting-every-prior-whole-pipeline-including-image-background-culling', 'prior_pipelines': ['cache-umbrella-pipeline-v1291', 'native-fieldset-retry-pipeline-v1310', 'native-font-retry-pipeline-v1311', 'native-c-raster-retry-pipeline-v1312', 'native-raster-fields-pipeline-v1337', 'native-default-strike-pipeline-v1338', 'cache-integration-pipeline-v1341', 'native-fieldset-capture-forensics-pipeline-v1346', 'native-style-integration-pipeline-v1354', 'native-intrinsic-constraints-pipeline-v1375', 'native-intrinsic-constraints-pipeline-v1369', 'native-raster-consumer-pipeline-v1398', 'umbrella-clean-workspace-pipeline-v1411', 'native-style-integration-retry-pipeline-v1419', 'native-intrinsic-constraints-retry-pipeline-v1420', 'native-raster-api-pipeline-v1425', 'native-image-occlusion-pipeline-v1437'], 'selections': [], 'scripts': ['openui-native-image-coverage-guards-v1448.py', 'openui-native-image-coverage-build-v1448.py', 'openui-native-image-coverage-fieldsets-v1448.py', 'openui-native-image-coverage-consumer-v1448.py', 'openui-native-image-coverage-matrices-v1448.py'], 'hard_stop_stages': ['guards', 'native-build'], 'stages': [('guards', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-guards-v1448.py'], 'native-image-coverage-guards-v1448/receipt.json', True), ('native-build', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-build-v1448.py'], 'native-image-coverage-clean-v1448/build.json', True), ('fieldsets', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-fieldsets-v1448.py'], 'native-image-coverage-fieldsets-v1448/receipt.json', True), ('native-application', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-consumer-v1448.py'], 'native-image-coverage-consumer-v1448/receipt.json', True), ('focused', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-matrices-v1448.py', 'focused'], 'native-image-coverage-clean-focused-v1448/focused-summary.json', False), ('primitive', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-matrices-v1448.py', 'primitive'], 'native-image-coverage-clean-primitive-v1448/primitive-summary.json', False), ('full', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-matrices-v1448.py', 'full'], 'native-image-coverage-clean-full-v1448/full-summary.json', False), ('expanded', ['/usr/bin/python3', '/tmp/openui-native-image-coverage-matrices-v1448.py', 'expanded'], 'native-image-coverage-clean-expanded-v1448/expanded-summary.json', False)]}
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
    consumer_compile_correction_only=False, renderer_production_unchanged_from_prior_candidate=False,
    original_pixel_gate_remains_required=True)

report.update(owner_pid=os.getpid(), baseline_commit='6570b65258677fff7369b336db6e55bb7645daec',
    expected_stages=8, workspace_cleaned_at_every_source_switch=True,
    strict_chromium_capture_pairs=True, preserves_both_unstable_reference_captures=True,
    existing_113_exports_and_30_layouts_preserved=True, public_rust_callbacks_and_owned_bounds=True,
    reviewed_root_cause_owner='openui-paint image sampling and geometric coverage blending',
    coverage_blending_candidate_requires_native_confirmation=True, strict_baseline_assertion_marker_required=True, original_fieldset_inputs_unchanged=True)

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
