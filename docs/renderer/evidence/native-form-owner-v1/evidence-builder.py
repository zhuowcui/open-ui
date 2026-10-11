import hashlib
import json
from pathlib import Path
import shutil

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
DEST = ROOT / 'docs/renderer/evidence/native-form-owner-v1'
SUMMARY = ROOT / 'docs/renderer/generated/native-form-owner-v1.json'
assert not DEST.exists() and not SUMMARY.exists()
assert shutil.disk_usage(ROOT).free > 350 * 2**20
DEST.mkdir()

def load(path):
    return json.loads(Path(path).read_bytes())

def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

native = load(RAW / 'private-native-form-owner-layout-continuation-v3461/receipt.json')
appearance = load(RAW / 'private-native-form-owner-appearance-v3467/receipt.json')
renderer = load(RAW / 'private-native-form-owner-renderer-v3478/receipt.json')
native_audit = load('/tmp/openui-native-form-owner-layout-continuation-completed-audit-v3462.json')
appearance_audit = load('/tmp/openui-native-form-owner-appearance-completed-audit-v3468.json')
renderer_audit = load('/tmp/openui-private-native-form-owner-renderer-completed-audit-v3479.json')
integration = load('/mnt/d/openui-v02-qualification-d174ea0b/public-native-form-owner-integration-v3489/receipt.json')
assert native['all_commands_terminal'] and native['observed_exit_code'] == 0
assert native_audit['complete'] and native_audit['native_behavior_exact'] == 1188
assert appearance['all_commands_terminal'] and appearance['observed_exit_code'] == 0
assert appearance_audit['complete'] and appearance_audit['native_appearance_qualified']
assert renderer['all_commands_terminal'] and renderer['observed_exit_code'] == 1
assert renderer_audit['complete'] and renderer_audit['private_candidate_nonregression']
assert integration['all_commands_terminal'] and integration['actual_exit_code'] == 0
source = native['source']
assert source == appearance['source'] == renderer['source'] == integration['public_source']
assert source['clean'] and source['commit'] == 'a401a8ac8c33cc5e24ed9bb19934061716cd84dd'

files = {}

def copy(origin, name):
    origin = Path(origin)
    target = DEST / name
    assert not target.exists()
    target.write_bytes(origin.read_bytes())
    assert sha(target) == sha(origin)
    files[name] = dict(origin=str(origin), sha256=sha(target), bytes=target.stat().st_size)

for origin, name in [
    (RAW / 'private-native-form-owner-layout-continuation-v3461/receipt.json', 'native-receipt.json'),
    ('/tmp/openui-native-form-owner-layout-continuation-completed-audit-v3462.json', 'native-completed-audit.json'),
    ('/tmp/openui-native-form-owner-layout-continuation-terminal-status-v3466.json', 'native-terminal-status.json'),
    (RAW / 'private-native-form-owner-appearance-v3467/receipt.json', 'appearance-receipt.json'),
    ('/tmp/openui-native-form-owner-appearance-completed-audit-v3468.json', 'appearance-completed-audit.json'),
    ('/tmp/openui-native-form-owner-appearance-terminal-status-v3473.json', 'appearance-terminal-status.json'),
    (RAW / 'private-native-form-owner-renderer-v3478/receipt.json', 'renderer-receipt.json'),
    ('/tmp/openui-private-native-form-owner-renderer-completed-audit-v3479.json', 'renderer-completed-audit.json'),
    ('/tmp/openui-private-native-form-owner-renderer-continuation-terminal-status-v3488.json', 'renderer-terminal-status.json'),
    ('/mnt/d/openui-v02-qualification-d174ea0b/native-form-owner-renderer-supervisor-v3482/receipt.json', 'renderer-supervisor-receipt.json'),
    ('/mnt/d/openui-v02-qualification-d174ea0b/public-native-form-owner-integration-v3489/receipt.json', 'integration-receipt.json'),
    ('/tmp/openui-native-form-owner-public-integration-terminal-status-v3490.json', 'integration-terminal-status.json'),
    ('/tmp/openui-private-native-form-owner-renderer-interrupted-completed-audit-v3477.json', 'interrupted-renderer-build-audit.json'),
    ('/tmp/openui-native-form-owner-appearance-failed-terminal-status-v3469.json', 'failed-appearance-terminal-status.json'),
    ('/mnt/d/openui-v02-storage-relocation-v3470/receipt.json', 'storage-recovery-receipt.json'),
    (__file__, 'evidence-builder.py'),
]:
    copy(origin, name)

for suite in ['focused', 'primitive', 'full', 'expanded']:
    copy(RAW / 'private-native-form-owner-renderer-v3478/trial' / (suite + '-matrix') / (suite + '-summary.json'), suite + '-summary.json')
for name in ['cases.json', 'observations-1.json', 'observations-2.json', 'probe.py', 'receipt.json']:
    copy(RAW / 'native-form-owner-reference-v3410' / name, 'chromium-' + name)

form_rows = native['form_owner_comparisons']
assert len(form_rows) == native['form_owner_rows'] == 264 and all(row['exact'] for row in form_rows)
assert all(count['total'] == count['exact'] == 396 for count in native['scenario_counts'].values())
reference = load(DEST / 'chromium-receipt.json')
assert reference['repeated_observations_identical'] and reference['runs'][0]['scenarios'] == 88
summary = dict(
    schema_version=1,
    scope='Native form association, radio membership mutations, and owned C parent/form queries',
    measured_source=source,
    measurement_checkout='private qualification clone',
    implementation_integrated=True,
    exact_measured_commit_adopted=True,
    public_source_bytes_equal_measured_source=True,
    new_public_qualification_commands_executed=False,
    current_public_hosted_qualification_complete=False,
    javascript_executed_by_openui=False,
    offline_chromium_reference_scripts_only=True,
    pixel_target='pinned Chromium', pixel_tolerance=0,
    historical_openui_archive_is_a_pixel_target=False,
    new_form_owner_scenarios=88, new_form_owner_comparisons=len(form_rows),
    total_native_behavior_comparisons=len(native['comparisons']),
    native_behavior_by_language=native['scenario_counts'],
    repeated_native_process_pairs=native_audit['repeated_process_pairs_verified'],
    original_deep_nesting_benchmark_passed=native_audit['deep_nesting_50_benchmark_passed'],
    original_stack_limit_and_command_preserved=native_audit['original_stack_limit_and_command_verified'],
    all_targets_tests=native['all_targets_tests'], workspace_tests=native['workspace_tests'],
    headless_tests=native['headless_tests'],
    control_appearance=dict(total=appearance['total'], exact=appearance['exact'],
                            png_comparisons=appearance_audit['images_verified'],
                            scope='Default light 13px controls on white, five scales, eight subpixel phases, two process repeats'),
    renderer_results=renderer_audit['results'],
    comparison_rows_audited=renderer_audit['matrix_rows_verified'],
    no_renderer_regression=renderer_audit['private_candidate_nonregression'],
    whole_renderer_actual_exit_code=renderer_audit['actual_whole_exit_code'],
    renderer_qualified=False, all_native_apis_qualified=False,
    compositor_qualified=False, hardware_qualified=False, release_qualified=False,
    files=files,
)
SUMMARY.write_text(json.dumps(summary, sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(summary=str(SUMMARY), evidence_files=len(files),
                      evidence_bytes=sum(row['bytes'] for row in files.values()),
                      summary_sha256=sha(SUMMARY)), sort_keys=True))
