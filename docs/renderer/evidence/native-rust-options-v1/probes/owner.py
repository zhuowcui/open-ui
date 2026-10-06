"""Own every build, application, capture stage and intervening gap."""
import fcntl
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

NAME = 'native-rust-options-pipeline-v1801'
ROOT = Path('/dev/shm/openui-native-rust-raster-options-c094d645-v1797')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / NAME
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / NAME
LOCK = open('/tmp/openui-native-cargo-raster-owner.lock', 'a')
fcntl.flock(LOCK, fcntl.LOCK_EX | fcntl.LOCK_NB)
priorpaths = [RAW / name / 'receipt.json' for name in (
    'native-keywords-capture-pipeline-v1786',
    'native-keywords-native-retry-pipeline-v1788',
)]
for path in priorpaths:
    prior = json.loads(path.read_bytes())
    assert prior['all_commands_terminal']
    try:
        os.kill(prior['owner_pid'], 0)
    except ProcessLookupError:
        pass
    else:
        raise AssertionError('prior whole owner remains live')
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '27a47ba25185e20e7ae20801d0a45a386ce9e78c'
scripts = [Path('/tmp') / name for name in (
    'openui-native-rust-options-build-v1800.py',
    'openui-native-rust-options-consumer-v1800.py',
    'openui-native-image-coverage-fieldsets-v1448.py',
)]
checks_path = RAW / 'native-rust-options-checks-v1798/receipt.json'
checks = json.loads(checks_path.read_bytes())
assert checks['source'] == checks['source_after'] == source
assert checks['all_commands_terminal'] and len(checks['checks']) == 15
assert all(row['observed_exit_code'] == 0 for row in checks['checks'])
report = dict(schema_version=1, source=source, source_after=source,
    owner_pid=os.getpid(), state='prepared-source-verified', all_commands_terminal=False,
    release_qualification=False, promotion_allowed=False, new_release_states_admitted=0,
    javascript_executed_by_openui=False, public_native_rust_apis_required=True,
    public_rust_document_app_headless_options=True, backend_selection_immutable=True,
    old_openui_pixels_are_provenance_only=True, pixel_tolerance=0,
    whole_pipeline_lock_covers_all_stages_and_gaps=True, steps=[],
    probe_sha256=sha(Path(__file__)), checks_receipt_sha256=sha(checks_path),
    immutable_probe_hashes={str(p): sha(p) for p in scripts},
    prior_terminal_pipeline_sha256={str(p): sha(p) for p in priorpaths},
    full_pixel_censuses_and_configuration_field_behavior_unqualified=True)
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()
stages = [
    ('clean-build-and-native-app', scripts[0], RAW / 'native-rust-options-clean-v1800/build.json'),
    ('native-chromium-images', scripts[1], RAW / 'native-rust-options-consumer-v1800/receipt.json'),
]
for name, script, terminalpath in stages:
    assert repository_source_identity(ROOT) == source
    assert all(sha(p) == report['immutable_probe_hashes'][str(p)] for p in scripts)
    report['state'] = 'running-' + name
    save()
    log = OUT / (name + '.log')
    with log.open('xb') as stream:
        process = subprocess.Popen([sys.executable, str(script)], cwd=ROOT,
            env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1', OPENUI_NATIVE_WHOLE_OWNER=NAME,
                     TMPDIR='/dev/shm/openui-native-rust-options-chromium-temporary-v1800'),
            stdout=stream, stderr=subprocess.STDOUT)
        report['current_process'] = dict(pid=process.pid, program=script.name)
        save()
        process.wait()
    report.pop('current_process')
    row = dict(name=name, observed_exit_code=process.returncode, log_sha256=sha(log))
    report['steps'].append(row)
    report['source_after'] = repository_source_identity(ROOT)
    assert report['source_after'] == source
    terminal = terminalpath.exists() and json.loads(terminalpath.read_bytes()).get('all_commands_terminal') is True
    if not terminal or name == 'clean-build-and-native-app' and process.returncode:
        report.update(state='stopped-on-build-or-harness-failure-' + name, all_commands_terminal=True)
        save()
        raise SystemExit(process.returncode or 1)
    save()
    print(json.dumps({'stage': name, 'actual_exit': process.returncode}), flush=True)
report.update(state='complete-requires-results-review', all_commands_terminal=True)
save()
raise SystemExit(int(any(row['observed_exit_code'] for row in report['steps'])))
