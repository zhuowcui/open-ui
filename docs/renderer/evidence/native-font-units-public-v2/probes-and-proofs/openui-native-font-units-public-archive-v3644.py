import gzip
import hashlib
import json
import subprocess
from pathlib import Path

ROOT = Path('/home/nero/code/open-ui')
BASE = Path('/mnt/d/openui-v02-qualification-d174ea0b')
DEST = ROOT/'docs/renderer/evidence/native-font-units-public-v2'
INDEX = ROOT/'docs/renderer/generated/native-font-units-public-v2.json'
load = lambda path: json.loads(Path(path).read_bytes())
sha = lambda data: hashlib.sha256(data).hexdigest()
renderer = load('/tmp/openui-public-native-font-units-renderer-terminal-v3635.json')
workspace = load('/tmp/openui-native-font-units-all-features-terminal-v3641.json')
reaped = load('/tmp/openui-native-font-units-all-features-reaped-v3643.json')
assert renderer['all_local_commands_terminal'] and renderer['actual_audit_exit_code'] == 0
assert workspace['complete'] and workspace['actual_owner_exit_code'] == 0
assert reaped['all_local_commands_terminal'] and reaped['actual_audit_exit_code'] == 0
assert renderer['source'] == workspace['source'] == reaped['source']
assert not DEST.exists() and not INDEX.exists()
DEST.mkdir(parents=True)
members = []


def archive(relative, data):
    compressed = len(data) > 65536 and not relative.endswith('.png')
    target = DEST / (relative + '.gz' if compressed else relative)
    target.parent.mkdir(parents=True, exist_ok=True)
    stored = gzip.compress(data, mtime=0) if compressed else data
    target.write_bytes(stored)
    assert sha(gzip.decompress(stored) if compressed else stored) == sha(data)
    members.append(dict(path=str(target.relative_to(ROOT)), raw_bytes=len(data), raw_sha256=sha(data),
                        bytes=len(stored), sha256=sha(stored), compression='gzip' if compressed else 'none'))


out = BASE/'public-native-font-units-renderer-v3632'
r = load(out/'receipt.json')
assert r['all_commands_terminal'] and r['source_unchanged']
assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode == 1
archive('renderer/receipt.json', (out/'receipt.json').read_bytes())
for step in r['steps']:
    archive('renderer/'+step['name']+'.log', (out/(step['name']+'.log')).read_bytes())
for path in sorted((out/'guarded-build').iterdir()):
    if path.is_file() and path.name not in ('pixel_compare','receipt.tmp'):
        archive('renderer/guarded-build/'+path.name, path.read_bytes())
for suite in ['focused','primitive','full','expanded']:
    path = out/(suite+'-matrix')/(suite+'-summary.json')
    archive('renderer/'+suite+'-summary.json', path.read_bytes())
out = BASE/'native-font-units-all-features-v3640'
assert load(out/'receipt.json')['all_commands_terminal']
for path in sorted(out.iterdir()):
    if path.is_file() and path.name != 'receipt.tmp':
        archive('workspace/'+path.name, path.read_bytes())
for version in ['3639','3642']:
    out = BASE/('native-font-unit-api-oracle-v'+version)
    assert load(out/'receipt.json')['all_commands_terminal']
    for path in sorted(out.iterdir()):
        if path.is_file() and path.name != 'receipt.tmp':
            archive('reference-only-'+version+'/'+path.name, path.read_bytes())
for name in [
    'openui_parallel_source_identity_v3060.py',
    'openui-public-native-font-units-renderer-v3632.py',
    'openui-public-native-font-units-renderer-audit-v3633.py',
    'openui-public-native-font-units-renderer-completed-audit-v3633.json',
    'openui-public-native-font-units-renderer-reaped-v3635.py',
    'openui-public-native-font-units-renderer-terminal-v3635.json',
    'openui-native-font-unit-api-oracle-v3639.py',
    'openui-native-font-unit-api-oracle-terminal-v3639.json',
    'openui-native-font-unit-api-oracle-v3642.py',
    'openui-native-font-unit-api-oracle-terminal-v3642.json',
    'openui-native-font-units-all-features-v3640.py',
    'openui-native-font-units-all-features-audit-v3641.py',
    'openui-native-font-units-all-features-terminal-v3641.json',
    'openui-native-font-units-all-features-reaped-v3643.py',
    'openui-native-font-units-all-features-reaped-v3643.json',
    'openui-native-font-units-public-archive-v3644.py',
    'openui-native-font-units-public-docs-v3647.py',
    'openui-native-font-units-test-count-review-v3638.json',
    'openui-font-units-hardening-v3645-observed-6.json',
    'openui-font-units-hardening-terminal-v3645.json',
    'openui-native-font-api-doc-correction-v3646.patch',
]:
    archive('probes-and-proofs/'+name, Path('/tmp',name).read_bytes())
summary = dict(schema_version=1, source=renderer['source'],
               actual_combined_umbrella_source_measured=True,
               implementation_integrated=True, renderer=renderer['results'],
               candidate_nonregression=renderer['candidate_nonregression'],
               chromium_reference_changed_rows=0, comparison_rows_verified=48252,
               workspace=workspace['results'],
               fresh_local_workspace_artifact_records=workspace['fresh_local_records'],
               compiled_workspace_paths=workspace['compiled_paths'],
               prior_public_app_proof='docs/renderer/generated/native-font-units-v1.json',
               prior_default_feature_test_scope_preserved=True,
               new_chromium_api_measurements=dict(unique_geometry_observations=100,
                   independent_repeats=2, scales=[1,1.25,1.5,2,3], reference_only=True,
                   native_application_executed=False, native_api_qualification=False,
                   native_pixel_qualification=False, initial_browser_start_failure_preserved=True),
               hosted_hardening=load('/tmp/openui-font-units-hardening-terminal-v3645.json'),
               pixel_target='pinned Chromium', pixel_tolerance=0,
               old_openui_images_are_pixel_targets=False, javascript_executed_by_openui=False,
               all_local_commands_terminal=True, all_members_hash_verified=True,
               full_renderer_contract_qualified=False, all_native_apis_qualified=False,
               release_qualified=False,
               remaining=['all residual Chromium pixel differences and reviewed ownership',
                          'mixed-unit animation and retained font-relative line-height declarations',
                          'root, adjusted-font-size, orientation, missing-glyph and metric-override contexts',
                          'complete needed native APIs, compositor, physical lab and release qualification'],
               archive=dict(members=members, files=len(members), bytes=sum(m['bytes'] for m in members)))
INDEX.write_text(json.dumps(summary, indent=2, sort_keys=True)+'\n')
for member in members:
    raw = (ROOT/member['path']).read_bytes()
    assert sha(raw) == member['sha256']
    if member['compression'] == 'gzip':
        raw = gzip.decompress(raw)
    assert sha(raw) == member['raw_sha256']
print(json.dumps(dict(index=str(INDEX), files=len(members), bytes=summary['archive']['bytes'],
                      all_members_hash_verified=True)), flush=True)
