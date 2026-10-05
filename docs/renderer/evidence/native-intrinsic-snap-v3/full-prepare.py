"""Prepare a fresh complete width queue after the terminal native and integration owners."""
import ast
import hashlib
import json
import subprocess
import sys
from pathlib import Path

MAIN = Path('/home/nero/code/open-ui')
RAW = MAIN / 'out/renderer-evidence/native-viewport-scroll-v1'
ROOT = Path('/dev/shm/openui-native-intrinsic-snap-full-727da10e')
BRANCH = 'agent/native-intrinsic-snap-full-v1716'
FIXED = '727da10e580c9439f5db83d36bfc3e2340168207'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
old_owner = Path('/tmp/openui-native-intrinsic-snap-pipeline-v1697.py')
text = old_owner.read_text()
config = ast.literal_eval(ast.parse(text).body[0].value)
old_priors = config['prior_pipelines']
priors = old_priors + ['native-intrinsic-snap-guard-pipeline-v1703', 'native-text-style-integration-pipeline-v1710']
assert len(priors) == len(set(priors)) == 42
for name in priors[-2:]:
    assert json.loads((RAW / name / 'receipt.json').read_bytes())['all_commands_terminal']
assert not (RAW / config['name']).exists(), 'old owner must remain unlaunched'
assert not (RAW / 'native-glyph-raster-pipeline-v1690').exists()
assert subprocess.run(['pgrep', '-x', 'cargo'], capture_output=True).returncode == 1
assert subprocess.run(['pgrep', '-x', 'pixel_compare'], capture_output=True).returncode == 1
assert not ROOT.exists()
subprocess.run(['git', 'worktree', 'add', '--quiet', '-b', BRANCH, str(ROOT), FIXED], cwd=MAIN, check=True)
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
guard_receipt = RAW / 'native-intrinsic-snap-guards-v1702/receipt.json'
assert sha(guard_receipt) == '4c761b86cd60dcc8c08fc30aa7ce8314d7af958e03c8411636f6a9363e36baee'
guards = json.loads(guard_receipt.read_bytes())
assert source == guards['source'] == guards['source_after']
assert guards['all_commands_terminal'] and guards['baseline_regression_reproduced'] and guards['state'] == 'complete'
assert [r['observed_exit_code'] for r in guards['steps']] == [0, 101, 0, 0, 0, 0, 0, 0, 0]
for r in guards['steps']:
    assert sha(guard_receipt.parent / (r['name'] + '.log')) == r['log_sha256']

files = {}
originals = {str(old_owner): sha(old_owner)}
old = Path('/tmp/openui-native-text-style-runtime-guard-reuse-v1677.py')
originals[str(old)] = sha(old)
reuse = old.read_text().replace('/dev/shm/openui-native-text-style-runtime-41b616c3', str(ROOT))
reuse = reuse.replace('native-text-style-qualification-guards-v1675/receipt.json', 'native-intrinsic-snap-guards-v1702/receipt.json')
reuse = reuse.replace('f32050fea37b3a4dc8d0e6628cc25bcfd114c8d0b23b4f104c2bc27139f25fb8', sha(guard_receipt))
reuse += "\nassert [r['observed_exit_code'] for r in d['steps']] == [0,101,0,0,0,0,0,0,0]\n"
guard_probe = Path('/tmp/openui-native-intrinsic-snap-guard-reuse-v1716.py')
files[guard_probe] = reuse
for kind in ['build', 'consumer', 'matrices']:
    old = Path('/tmp') / f'openui-native-intrinsic-snap-{kind}-v1696.py'
    originals[str(old)] = sha(old)
    body = old.read_text().replace(config['root'], str(ROOT)).replace('v1696', 'v1716')
    if kind == 'build':
        needle = 'for prior_name in ' + repr(old_priors) + ':'
        assert body.count(needle) == 1
        body = body.replace(needle, 'for prior_name in ' + repr(priors) + ':')
    files[Path('/tmp') / f'openui-native-intrinsic-snap-{kind}-v1716.py'] = body
stages = [(name, [arg.replace('v1696', 'v1716') for arg in command], path.replace('v1696', 'v1716'), flag)
          for name, command, path, flag in config['stages'][1:]]
config.update(root=str(ROOT), name='native-intrinsic-snap-pipeline-v1717',
              initial_state='awaiting-all-42-prior-whole-pipelines', prior_pipelines=priors,
              scripts=[p.name for p in files],
              selections=config['selections'] + ['native-intrinsic-snap-guard-prepared-v1702.json',
                                                  'native-text-style-integration-v1708.json'],
              stages=[('guards', ['/usr/bin/python3', str(guard_probe)],
                       'native-intrinsic-snap-guards-v1702/receipt.json', True)] + stages)
owner = Path('/tmp/openui-native-intrinsic-snap-pipeline-v1717.py')
files[owner] = 'CONFIG = ' + repr(config) + '\n' + text.split('\n', 1)[1]
assert len(config['stages']) == 7
for p, body in files.items():
    assert not p.exists()
    ast.parse(body)
    assert str(ROOT) in body or p == owner
    assert 'v1696' not in body and 'pipeline-v1697' not in body
    p.write_text(body)
assert 'fcntl.flock(OWNER_LOCK, fcntl.LOCK_EX)' in files[owner]
assert 'fontations_lcd_hints_at_physical_size_before_replay ...' in files[Path('/tmp/openui-native-intrinsic-snap-build-v1716.py')]
receipt = RAW / 'native-intrinsic-snap-full-prepared-v1716.json'
assert not receipt.exists()
receipt.write_text(json.dumps(dict(schema_version=1, source=source, root=str(ROOT), branch=BRANCH,
                                  prior_whole_owners=42, guard_receipt_sha256=sha(guard_receipt),
                                  native_owner_not_started=True, old_full_and_glyph_owners_not_started=True,
                                  immutable_actual_source_identical_guards_reused=True,
                                  required_build_stages=13, required_native_images=600,
                                  required_geometry_states=38400, required_renderer_matrices=4,
                                  whole_owner_lock_preserves_gaps=True,
                                  scripts={str(p): sha(p) for p in files}, originals=originals,
                                  javascript_executed_by_openui=False, public_native_rust_apis_required=True,
                                  pixel_tolerance=0, release_qualification=False, probe_sha256=sha(Path(__file__))),
                             sort_keys=True, indent=2) + '\n')
print(json.dumps(dict(prepared=True, owner_not_started=True, prior_whole_owners=42,
                      receipt_sha256=sha(receipt), script_count=len(files))), flush=True)
