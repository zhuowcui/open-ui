CONFIG = {'root': '/dev/shm/openui-native-keywords-native-7d6ffabf-v1787', 'commit': '7d6ffabfea1f72c90f97475488dba8a8058ac4c7', 'name': 'native-keywords-native-retry-pipeline-v1788', 'initial_state': 'awaiting-confirmed-whole-census-owner-terminal', 'prior_pipelines': ['native-keywords-capture-pipeline-v1786'], 'selections': ['native-keywords-current-checks-v1749/receipt.json', 'native-keywords-current-hosted-v1762/receipt.json', 'native-keywords-current-pipeline-v1773/receipt.json', 'native-keywords-current-clean-v1772/build.json', 'native-keywords-current-guards-v1772/receipt.json', 'native-keywords-abi-scratch-relocation-v1777.json', 'native-cargo-cache-relocation-v1778.json', 'native-keywords-current-retry-pipeline-v1780/receipt.json', 'native-keywords-current-retry-clean-v1779/build.json', 'native-keywords-capture-path-failure-v1785.json', 'native-keywords-capture-consumer-v1785/receipt.json', 'native-keywords-capture-table-geometry-v1785/receipt.json'], 'hard_stop_stages': ['build-and-guards'], 'scripts': ['openui-native-keywords-native-retry-consumer-v1787.py', 'openui-native-keywords-native-retry-table-geometry-v1787.py', 'openui-native-keywords-native-retry-build-reuse-v1787.py', 'openui-native-image-coverage-fieldsets-v1448.py'], 'stages': [('build-and-guards', ['/usr/bin/python3', '/tmp/openui-native-keywords-native-retry-build-reuse-v1787.py'], 'native-keywords-native-retry-build-and-guards-v1787/receipt.json', True), ('native-application', ['/usr/bin/python3', '/tmp/openui-native-keywords-native-retry-consumer-v1787.py'], 'native-keywords-native-retry-consumer-v1787/receipt.json', True), ('native-table-geometry-diagnostic', ['/usr/bin/python3', '/tmp/openui-native-keywords-native-retry-table-geometry-v1787.py'], 'native-keywords-native-retry-table-geometry-v1787/receipt.json', True)]}
import hashlib
import fcntl
import json
import os
import subprocess
import sys
import time
from pathlib import Path

# CONFIG is inserted as a literal into each independently owned pipeline.
OWNER_LOCK = open('/tmp/openui-native-cargo-raster-owner.lock', 'a')
fcntl.flock(OWNER_LOCK, fcntl.LOCK_EX | fcntl.LOCK_NB)
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
    consumer_compile_correction_only=False, renderer_production_unchanged_from_applied_renderer=False,
    original_pixel_gate_remains_required=True)

report.update(temporary_files_on_data_volume=True, fresh_retry_preserves_original_failure=True, capture_path_correction_only=True, completed_same_source_build_reused=True, original_owner_sha256='189711442797ae28149d346c7caa1b75dc768e81066bdab258f1167b79a12ba7', owner_pid=os.getpid(), baseline_commit='7cb31314f2f871e9d1bb6fc0dc63c594bb1cf20e', expected_stages=3,
    whole_pipeline_lock_covers_all_stages_and_gaps=True,
    workspace_cleaned_at_every_source_switch=True, existing_113_exports_and_30_layouts_preserved=True,
    native_event_api_preserved=True, c_abi_unchanged=True, actual_pixel_gain_claimed=False,
    root_cause_owner='shared native style value construction',
    strict_named_baseline_failure_required=True, public_rust_c_cpp_callbacks_and_owned_bounds=True,
    strict_chromium_capture_pairs=True, preserves_both_unstable_reference_captures=True,
    native_table_geometry_is_diagnostic_only=True, accepted_renderer_unchanged=True,
    native_api_and_chromium_pixels_still_pending=True, old_queue_unlaunched=True)
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

WITNESS=RAW/'owner-interruption-witness-v1664.json'
assert hashlib.sha256(WITNESS.read_bytes()).hexdigest()=='e3b0b081e43fdb29917909d2d8f03a643430bf17d971f1a3219211be612d29e2'
INTERRUPTED={r['receipt']:r for r in json.loads(WITNESS.read_bytes())['owners']}
def verified_done(path):
 try:
  d=json.loads(path.read_bytes())
  if d['all_commands_terminal']:return True
  r=INTERRUPTED.get(str(path))
  if not r or hashlib.sha256(path.read_bytes()).hexdigest()!=r['receipt_sha256']:return False
  for pid in r['pids_verified_missing']:
   try:os.kill(pid,0);return False
   except ProcessLookupError:pass
  return True
 except (FileNotFoundError,json.JSONDecodeError):return False

complete=verified_done

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
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1', OPENUI_NATIVE_WHOLE_OWNER=CONFIG['name'], TMPDIR='/dev/shm/openui-native-keywords-chromium-temporary-v1787'), stdout=stream, stderr=subprocess.STDOUT)
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
    print(json.dumps({'stage':name,'actual_exit':process.returncode}),flush=True)
report.update(state='complete-requires-results-review', all_commands_terminal=True)
save()
raise SystemExit(int(any(row['observed_exit_code'] for row in report['steps'])))
