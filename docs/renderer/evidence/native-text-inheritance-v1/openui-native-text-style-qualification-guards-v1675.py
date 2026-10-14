import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-text-style-complete-41b616c3')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-text-style-qualification-guards-v1675'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
FIXED = '41b616c3be5224b074e9d06ba6aa963731a1b265'
BASELINE = '25322be849f1f0523a6e66291a567d0758931160'
BRANCH = 'agent/native-text-style-qualification-v1675'
TEST = 'tests::c_element_text_content_renders_and_matches_native_rust'
SELECTED = ['tests::c_element_text_content_renders_and_matches_native_rust']
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

for prior_name in ['cache-umbrella-pipeline-v1291', 'native-fieldset-retry-pipeline-v1310', 'native-font-retry-pipeline-v1311', 'native-c-raster-retry-pipeline-v1312', 'native-raster-fields-pipeline-v1337', 'native-default-strike-pipeline-v1338', 'cache-integration-pipeline-v1341', 'native-fieldset-capture-forensics-pipeline-v1346', 'native-style-integration-pipeline-v1354', 'native-intrinsic-constraints-pipeline-v1375', 'native-intrinsic-constraints-pipeline-v1369', 'native-raster-consumer-pipeline-v1398', 'umbrella-clean-workspace-pipeline-v1411', 'native-style-integration-retry-pipeline-v1419', 'native-intrinsic-constraints-retry-pipeline-v1420', 'native-raster-api-pipeline-v1425', 'native-image-occlusion-pipeline-v1437', 'native-image-coverage-pipeline-v1449', 'native-event-targets-pipeline-v1457', 'native-border-contrast-pipeline-v1462', 'native-intrinsic-cache-guard-pipeline-v1469', 'native-raster-fields-public-pipeline-v1470', 'native-image-imports-pipeline-v1471', 'native-rounded-border-pipeline-v1482', 'native-border-guard-pipeline-v1501', 'native-enum-values-pipeline-v1513', 'native-inline-replaced-pipeline-v1529', 'native-table-progress-pipeline-v1548', 'native-raster-fields-retry-pipeline-v1560', 'native-inline-fallback-pipeline-v1576', 'native-keywords-pipeline-v1588', 'native-table-source-pipeline-v1601', 'native-text-content-pipeline-v1621', 'native-text-content-viewport-pipeline-v1636', 'native-text-retry-pipeline-v1650', 'native-text-loader-retry-pipeline-v1656', 'native-glyph-guard-pipeline-v1658', 'native-glyph-retry-pipeline-v1667']:
 assert verified_done(RAW/prior_name/'receipt.json')
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
assert not OUT.exists() and not STORE.exists()
STORE.mkdir(); OUT.symlink_to(STORE, target_is_directory=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip() == FIXED
env = dict(os.environ, CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',
    CARGO_INCREMENTAL='0', RUST_MIN_STACK='4194304', CARGO_BUILD_JOBS='4',
    PYTHONDONTWRITEBYTECODE='1', CARGO_PROFILE_DEV_DEBUG='0', CARGO_PROFILE_TEST_DEBUG='0')
build_base = ['cargo', '--config', '.cargo/config.chromium.toml']
base = build_base + ['test', '--locked', '-p', 'openui-ffi', '--lib', '--features', 'linux']
report = dict(schema_version=1, source=source, source_after=source,
    release_qualification=False, promotion_allowed=False, all_commands_terminal=False,
    baseline_regression_reproduced=False, workspace_cleaned_at_every_source_switch=True,
    selected_native_test_names=SELECTED, public_native_text_style_test='native_text_replacement_inherits_authored_fonts_through_rust_callbacks', inherited_style_inventory_includes_suffix_matches=True, steps=[], probe_sha256=sha(Path(__file__)),
    public_native_rust_api=True, javascript_executed_by_openui=False)
metadata = json.loads(subprocess.check_output(build_base + ['metadata', '--offline',
    '--locked', '--no-deps', '--format-version', '1'], cwd=ROOT / 'bindings/rust', env=env, text=True))
members = set(metadata['workspace_members'])
packages = [p['name'] for p in metadata['packages'] if p['id'] in members]
assert len(packages) == 18
clean_command = build_base + ['clean'] + [arg for package in packages for arg in ['-p', package]]
receipt = OUT / 'receipt.json'
save = lambda: receipt.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
save()

def run(name, command, expected_commit):
    before = repository_source_identity(ROOT)
    assert before['clean'] and before['commit'] == expected_commit
    assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
    assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
    log = OUT / (name + '.log')
    guard = False
    with log.open('xb') as stream:
        process = subprocess.Popen(command, cwd=ROOT / 'bindings/rust', env=env,
                                   stdout=stream, stderr=subprocess.STDOUT, start_new_session=True)
        while process.poll() is None:
            if shutil.disk_usage('/home/nero/code/open-ui').free < 512 * 2**20 or shutil.disk_usage(STORE).free < 10 * 2**30:
                guard = True
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=20)
                break
            time.sleep(1)
        process.wait()
    after = repository_source_identity(ROOT)
    assert before == after
    row = dict(name=name, command=command, observed_exit_code=process.returncode,
               disk_guard_triggered=guard, source=before, source_after=after,
               log_sha256=sha(log))
    report['steps'].append(row)
    save()
    print(json.dumps(row), flush=True)
    return row, log.read_bytes()

try:
    try:
        subprocess.run(['git', 'switch', '--detach', BASELINE], cwd=ROOT, check=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        row, _ = run('clean-baseline', clean_command, BASELINE)
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
        row, content = run('baseline-public-native-text-inheritance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'native_text_style_inheritance', 'native_text_replacement_inherits_authored_fonts_through_rust_callbacks', '--', '--exact'], BASELINE)
        report['baseline_regression_reproduced'] = (row['observed_exit_code'] == 101 and not row['disk_guard_triggered'] and b'native_text_replacement_inherits_authored_fonts_through_rust_callbacks ... FAILED' in content and b'0 passed; 1 failed;' in content and b'authored text must inherit its native container font' in content)
        save()
    finally:
        subprocess.run(['git', 'switch', BRANCH], cwd=ROOT, check=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        assert repository_source_identity(ROOT) == source
    assert report['baseline_regression_reproduced'], 'named baseline assertion was not reproduced'
    row, _ = run('clean-fixed', clean_command, FIXED)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    for name, test in [('fixed-native-text-style-qualification-guard', TEST)]:
        row, content = run(name, base + [test, '--', '--exact'], FIXED)
        row['test_counts'] = [list(map(int, values)) for values in
            re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)]
        row['observed_named_tests'] = sorted(value.decode() for value in
            re.findall(rb'^test ([^ ]+) \.\.\. ok$', content, re.MULTILINE))
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
        assert row['test_counts'] == [[1, 0, 0]] and row['observed_named_tests'] == [test]
    for name, command, exact_test in [
        ('fixed-text-content-storage', build_base + ['test', '--locked', '-p', 'openui-engine', '--lib'],
         'tests::native_text_content_replaces_children_and_reuses_owned_storage'),
        ('fixed-public-native-text-inheritance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'native_text_style_inheritance', 'native_text_replacement_inherits_authored_fonts_through_rust_callbacks', '--', '--exact'], None),
        ('fixed-public-native-conformance', build_base + ['test', '--locked', '-p', 'openui', '--test', 'v02_conformance'], None),
    ]:
        if exact_test:
            command = command + [exact_test, '--', '--exact']
        row, content = run(name, command, FIXED)
        assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
        counts = re.findall(rb'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;', content)
        assert len(counts) == 1 and int(counts[0][0]) > 0 and counts[0][1:] == (b'0', b'0')
        row['test_counts'] = [list(map(int, counts[0]))]
        if exact_test:
            assert counts == [(b'1', b'0', b'0')] and (exact_test + ' ... ok').encode() in content
    row, content = run('fixed-native-inheritance-inventory', build_base + ['test', '--locked', '-p', 'openui-engine', '--lib', 'native_inher'], FIXED)
    assert row['observed_exit_code'] == 0 and not row['disk_guard_triggered']
    row['observed_named_tests'] = sorted(value.decode() for value in re.findall(rb'^test ([^ ]+) \.\.\. ok$', content, re.MULTILINE))
    source_text=(ROOT/'bindings/rust/openui-engine/src/lib.rs').read_text()
    expected=sorted('tests::'+name for name in re.findall(r'#\[test\]\s*fn ([a-zA-Z0-9_]+)\(',source_text) if 'native_inher' in name)
    assert row['observed_named_tests']==expected and len(expected)>0
    report['observed_exit_code'] = 0
    report['state'] = 'complete'
except BaseException as error:
    report.update(observed_exit_code=1, state='failed', failure=str(error))
    raise
finally:
    report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT))
    assert report['source_after'] == source
    save()
