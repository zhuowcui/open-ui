import fcntl
import gzip
import hashlib
import json
import os
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
NATIVE = Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-regressions-v3547')
RENDERER = Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-renderer-v3552')
DEST = ROOT / 'docs/renderer/evidence/native-keyboard-public-v2'
INDEX = ROOT / 'docs/renderer/generated/native-keyboard-public-v2.json'
load = lambda p: json.loads(Path(p).read_bytes())
digest = lambda data: hashlib.sha256(data).hexdigest()
lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
n = load('/tmp/openui-native-keyboard-regressions-completed-audit-v3551.json')
r = load('/tmp/openui-public-native-keyboard-renderer-completed-audit-v3555.json')
t = load('/tmp/openui-public-native-keyboard-renderer-terminal-v3556.json')
assert n['complete'] and n['actual_whole_exit_code'] == 0
assert r['complete'] and r['actual_whole_exit_code'] == 1
assert t['all_local_commands_terminal'] and t['actual_audit_exit_code'] == 0
assert n['source'] == r['source'] == t['source']
assert subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip() == n['source']['commit']
assert not subprocess.check_output(['git', 'status', '--porcelain=v1', '-z'], cwd=ROOT)
assert not DEST.exists() and not INDEX.exists()
prior = ROOT / 'docs/renderer/evidence/native-keyboard-controls-v1'
prior_hashes = {str(p.relative_to(prior)): digest(p.read_bytes()) for p in prior.rglob('*') if p.is_file()}
DEST.mkdir()
files = {}

def archive(origin, name, compress=False):
    origin = Path(origin)
    data = origin.read_bytes()
    target = DEST / name
    assert not target.exists()
    stored = gzip.compress(data, compresslevel=9, mtime=0) if compress else data
    target.write_bytes(stored)
    assert (gzip.decompress(target.read_bytes()) if compress else target.read_bytes()) == data
    files[name] = dict(origin=str(origin), bytes=len(stored), sha256=digest(stored), uncompressed_sha256=digest(data))

for origin, name in [
    ('/tmp/openui-native-keyboard-regressions-completed-audit-v3551.json', 'native-completed-audit.json'),
    ('/tmp/openui-native-keyboard-regressions-terminal-v3553.json', 'native-terminal.json'),
    ('/tmp/openui-native-keyboard-regressions-completed-audit-failed-v3558.json', 'native-prior-failed-audit.json'),
    ('/tmp/openui-public-native-keyboard-renderer-completed-audit-v3555.json', 'renderer-completed-audit.json'),
    ('/tmp/openui-public-native-keyboard-renderer-terminal-v3556.json', 'renderer-terminal.json'),
    ('/tmp/openui-native-keyboard-current-pr-status-v3554.json', 'hosted-ordinary.json'),
    ('/tmp/openui-native-keyboard-current-hardening-v3557.json', 'hosted-hardening.json'),
    ('/tmp/openui-native-keyboard-regressions-v3547.py', 'native-driver.py'),
    ('/tmp/openui-native-keyboard-regressions-completed-audit-v3551.py', 'native-audit-driver.py'),
    ('/tmp/openui-native-keyboard-regressions-completed-audit-failed-v3558.py', 'native-prior-failed-audit-driver.py'),
    ('/tmp/openui-native-keyboard-regressions-terminal-v3553.py', 'native-terminal-driver.py'),
    ('/tmp/openui-public-native-keyboard-renderer-v3552.py', 'renderer-driver.py'),
    ('/tmp/openui-public-native-keyboard-renderer-audit-v3555.py', 'renderer-audit-driver.py'),
    ('/tmp/openui-public-native-keyboard-renderer-terminal-v3556.py', 'renderer-terminal-driver.py'),
    (RENDERER / 'receipt.json', 'renderer-receipt.json'),
    (RENDERER / 'guarded-build/receipt.json', 'renderer-build-receipt.json'),
    (__file__, 'evidence-builder.py'),
]:
    archive(origin, name)
for p in sorted(NATIVE.iterdir()):
    if p.suffix in ['.log', '.json']:
        archive(p, 'native-' + p.name + '.gz', True)
for p in sorted(RENDERER.glob('*.log')):
    archive(p, 'renderer-' + p.name + '.gz', True)
for p in sorted((RENDERER / 'guarded-build').iterdir()):
    if p.suffix in ['.stdout', '.stderr']:
        archive(p, 'renderer-build-' + p.name + '.gz', True)
for suite in ['focused', 'primitive', 'full', 'expanded']:
    archive(RENDERER / (suite + '-matrix') / (suite + '-summary.json'), suite + '-summary.json.gz', True)
ordinary = load('/tmp/openui-native-keyboard-current-pr-status-v3554.json')
hardening = load('/tmp/openui-native-keyboard-current-hardening-v3557.json')
assert ordinary['head'] == hardening['head'] == n['source']['commit']
summary = dict(
    schema_version=1,
    scope='Canonical umbrella native keyboard/control regressions and complete CPU renderer matrices',
    measured_source=n['source'], measurement_checkout='canonical umbrella checkout',
    implementation_integrated=True, new_public_qualification_commands_executed=True,
    javascript_executed_by_openui=False, offline_chromium_reference_scripts_only=True,
    pixel_target='pinned Chromium', pixel_tolerance=0, historical_openui_archive_is_a_pixel_target=False,
    all_targets_tests=n['all_targets_tests'], workspace_tests=n['workspace_tests'], headless_tests=n['headless_tests'],
    native_behavior_by_language=n['native_behavior_by_language'],
    native_behavior_comparisons=n['native_behavior_comparisons'], regression_groups=n['regression_groups'],
    repeated_processes_identical=True, exports=n['exports'], layouts=n['layouts'],
    c_consumers=n['c_consumers'], cpp_consumers=n['cpp_consumers'],
    original_main_stack_bytes=n['original_main_stack_bytes'],
    c_disabled_focus_comparison_scope='Rust-only tree/root observation fields are projected out; all common measured Chromium state and callback fields are compared.',
    native_actual_exit_code=0, renderer_actual_exit_code=1,
    renderer_matrix_results=r['results'], renderer_matrix_rows_verified=r['matrix_rows_verified'],
    renderer_linked_libraries_fresh=r['linked_local_libraries_fresh'],
    renderer_compiled_paths_verified=r['compiled_paths_verified'],
    original_residual_test_ids=884, original_residual_ownership_qualified=False,
    expanded_candidates=201, expanded_candidates_exact_at_all_four_profiles=200,
    hosted_ordinary=ordinary, hosted_hardening=hardening,
    all_local_commands_terminal=True,
    previous_keyboard_v1_evidence_unchanged=True,
    saved_audits_are_chronological_and_retired_for_reexecution=True,
    keyboard_pixels_qualified=False, rtl_keyboard_qualified=False, keypress_api_qualified=False,
    all_native_apis_qualified=False, renderer_qualified=False, compositor_qualified=False,
    hardware_qualified=False, release_qualified=False,
    remaining=['Broader keyboard, RTL, keypress, composition and remaining native APIs',
               'Keyboard appearance', '1,586 original and 1,587 expanded Chromium pixel differences',
               'Reviewed root causes and ownership for all residuals',
               'Remaining AST-derived native final-state cases', 'Required complete Chromium pixel CI gate',
               'Retained compositor, 3D, physical lab, performance, packaging and release qualification'],
    files=files,
)
INDEX.write_text(json.dumps(summary, sort_keys=True, indent=2) + '\n')
assert prior_hashes == {str(p.relative_to(prior)): digest(p.read_bytes()) for p in prior.rglob('*') if p.is_file()}
print(json.dumps(dict(files=len(files), bytes=sum(v['bytes'] for v in files.values()), summary_sha256=digest(INDEX.read_bytes()))), flush=True)
