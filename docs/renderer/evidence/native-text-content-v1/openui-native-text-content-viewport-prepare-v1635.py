"""Freeze a fresh native text queue with consistent logical viewport inputs."""
import ast
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-text-content-viewport-queue-90310e15')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
FIXED = '90310e15b86bed558a79c8eb085d2a72092eab69'
BASELINE = subprocess.check_output(['git', 'rev-parse', '9fe1665d'], cwd=ROOT, text=True).strip()
BRANCH = 'agent/native-text-content-viewport-queue-v1634'
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
checks_path = RAW / 'native-text-content-viewport-checks-v1626/receipt.json'
checks = json.loads(checks_path.read_bytes())
assert checks['source'] == checks['source_after'] == source and checks['all_commands_terminal']
assert len(checks['checks']) == 13 and all(row['observed_exit_code'] == 0 for row in checks['checks'])
old_queue_path = RAW / 'native-text-content-queue-prepared-v1620.json'
old_queue = json.loads(old_queue_path.read_bytes())
owner_path = Path('/tmp/openui-native-text-content-pipeline-v1621.py')
owner_text = owner_path.read_text()
config = ast.literal_eval(ast.parse(owner_text).body[0].value)
old_priors = config['prior_pipelines']
priors = old_priors + [config['name']]
assert len(priors) == len(set(priors)) == 33
old_fixed = old_queue['source']['commit']
old_baseline = old_queue['baseline_commit']
replacements = [
    (old_queue['source_root'], str(ROOT)),
    (old_fixed, FIXED), (old_baseline, BASELINE),
    (old_queue['branch'], BRANCH),
    ('native-text-content-', 'native-text-content-viewport-'),
    ('v1620', 'v1635'), ('v1621', 'v1636'),
    (repr(old_priors), repr(priors)),
    ('awaiting-all-32-prior-whole-pipelines', 'awaiting-all-33-prior-whole-pipelines'),
]
files = {}
originals = {str(owner_path): sha(owner_path)}
for kind in ['guards', 'build', 'consumer', 'matrices']:
    old = Path('/tmp') / ('openui-native-text-content-' + kind + '-v1620.py')
    assert old_queue['scripts'][str(old)] == sha(old)
    originals[str(old)] = sha(old)
    text = old.read_text()
    for before, after in replacements:
        text = text.replace(before, after)
    # The Chromium references are from a completed earlier consumer, not this queue.
    assert "prior_path = RAW / 'native-raster-fields-retry-consumer-v1559/receipt.json'" in text if kind == 'consumer' else True
    new = Path('/tmp') / ('openui-native-text-content-viewport-' + kind + '-v1635.py')
    files[new] = text

config.update(root=str(ROOT), commit=FIXED, name='native-text-content-viewport-pipeline-v1636',
              prior_pipelines=priors, initial_state='awaiting-all-33-prior-whole-pipelines',
              scripts=[p.name for p in files],
              selections=['native-text-content-viewport-checks-v1626/receipt.json',
                          'native-text-content-consumer-audit-v1615.json',
                          'native-raster-fields-retry-consumer-v1559/receipt.json'])
config['stages'] = [
    ('guards', ['/usr/bin/python3', '/tmp/openui-native-text-content-viewport-guards-v1635.py'],
     'native-text-content-viewport-guards-v1635/receipt.json', True),
    ('native-build', ['/usr/bin/python3', '/tmp/openui-native-text-content-viewport-build-v1635.py'],
     'native-text-content-viewport-clean-v1635/build.json', True),
    ('native-application', ['/usr/bin/python3', '/tmp/openui-native-text-content-viewport-consumer-v1635.py'],
     'native-text-content-viewport-consumer-v1635/receipt.json', True),
] + [(suite, ['/usr/bin/python3', '/tmp/openui-native-text-content-viewport-matrices-v1635.py', suite],
      f'native-text-content-viewport-clean-{suite}-v1635/{suite}-summary.json', False)
     for suite in ['focused', 'primitive', 'full', 'expanded']]
body = owner_text.split('\n', 1)[1]
for before, after in replacements:
    body = body.replace(before, after)
files[Path('/tmp/openui-native-text-content-viewport-pipeline-v1636.py')] = 'CONFIG = ' + repr(config) + '\n' + body
for path, text in files.items():
    assert not path.exists()
    ast.parse(text)
    assert '/dev/shm/openui-native-text-content-queue-' not in text
    assert old_fixed not in text and old_baseline not in text
    path.write_text(text)
assert repository_source_identity(ROOT) == source
production = subprocess.check_output(['git', 'diff', '--binary', BASELINE, FIXED], cwd=ROOT)
old_production = subprocess.check_output(['git', 'diff', '--binary', old_baseline, old_fixed], cwd=ROOT)
# The FFI blob also contains the corrected test, so diff headers differ.
viewport_setup = (b"            // Logical dimensions are authoritative; derive physical dimensions\n"
                  b"            // at this scale instead of retaining the helper's scale-1 values.\n"
                  b"            config.viewport.physical_width = 0;\n"
                  b"            config.viewport.physical_height = 0;\n")
paths = subprocess.check_output(['git', 'diff', '--name-only', old_baseline, old_fixed], cwd=ROOT, text=True).splitlines()
assert len(paths) == 6
for name in paths:
    original = subprocess.check_output(['git', 'show', old_fixed + ':' + name], cwd=ROOT)
    revised = subprocess.check_output(['git', 'show', FIXED + ':' + name], cwd=ROOT)
    if name == 'bindings/rust/openui-ffi/src/lib.rs':
        assert revised.count(viewport_setup) == 1
        assert revised.replace(viewport_setup, b'', 1) == original
    else:
        assert revised == original
assert subprocess.check_output(['git', 'diff', '--name-only', old_fixed, FIXED], cwd=ROOT, text=True).splitlines() == ['bindings/rust/openui-ffi/src/lib.rs']
assert subprocess.check_output(['git', 'diff', '--name-only', old_baseline, BASELINE], cwd=ROOT, text=True).splitlines() == ['bindings/rust/openui-ffi/src/lib.rs']

report = dict(schema_version=1, source=source, source_after=source, source_root=str(ROOT), branch=BRANCH,
              baseline_commit=BASELINE, read_only_checks_passed=13, prior_whole_owners=33,
              prior_whole_pipelines=priors, whole_owner_version=1636, required_stages=7,
              required_build_steps=12, required_native_cases=100, required_native_image_comparisons=600,
              required_native_phase_states=38400, repeated_native_updates=10000,
              strict_named_c_baseline_failure_required=True, all_four_matrices_required=True,
              immutable_chromium_references_reused=True,
              chromium_reference_policy='pinned Linux default Fontations',
              native_raster_policy='immutable default EngineOptions', current_exports=113, current_c_layouts=30,
              old_source_and_probes_preserved=True, old_source=old_fixed,
              old_prepared_queue_sha256=sha(old_queue_path), old_owner_probe_sha256=sha(owner_path),
              production_code_identical_after_test_setup_correction=True, production_patch_sha256=hashlib.sha256(production).hexdigest(),
              corrected_logical_viewport_physical_dimensions_derived=True,
              failure_is_in_new_test_setup_not_viewport_validation=True,
              javascript_executed_by_openui=False, public_native_rust_c_cpp_api_required=True,
              applied_to_umbrella=False, native_or_pixel_qualification=False,
              release_qualification=False, new_release_states_admitted=0, cargo_commands_run=0,
              screenshots_generated=0, scripts={str(p): sha(p) for p in files},
              original_probe_sha256=originals, checks_receipt_sha256=sha(checks_path), probe_sha256=sha(Path(__file__)))
path = RAW / 'native-text-content-viewport-queue-prepared-v1635.json'
assert not path.exists()
path.write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(receipt=str(path), sha256=sha(path), source=FIXED, baseline=BASELINE,
                     prior_whole_owners=33, stages=7)), flush=True)
