import argparse
import fcntl
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--owner-session', type=int, required=True)
parser.add_argument('--owner-exit-code', type=int, required=True)
args = parser.parse_args()
ROOT = Path('/home/nero/code/open-ui')
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-all-features-v3640')
PROOF = Path('/tmp/openui-native-font-units-all-features-terminal-v3641.json')
sys.path.insert(0, '/tmp')
from openui_parallel_source_identity_v3060 import identity


def sha(path):
    digest = hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda: stream.read(2**20), b''):
            digest.update(block)
    return digest.hexdigest()


lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
r = json.loads((OUT/'receipt.json').read_bytes())
assert r['all_commands_terminal'] and r['source_unchanged']
assert r['observed_exit_code'] == args.owner_exit_code == 0
assert 'failure' not in r
assert identity(ROOT) == r['source'] == r['source_after']
assert subprocess.run(['ps', '-p', str(r['owner_pid'])], capture_output=True).returncode == 1
for name in ['cargo', 'rustc', 'rustfmt', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', name], capture_output=True).returncode == 1
assert sha('/tmp/openui-native-font-units-all-features-v3640.py') == r['driver_sha256']
assert sha('/tmp/openui-public-native-font-units-renderer-terminal-v3635.json') == r['previous_terminal_sha256']
results = {}
for step in r['steps']:
    path = OUT / (step['name'] + '.log')
    assert sha(path) == step['log_sha256']
    assert step['actual_exit_code'] == 0
    summaries = re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;', path.read_text())
    totals = [sum(int(row[i]) for row in summaries) for i in range(3)]
    assert totals == [step['passed'], step['failed'], step['ignored']]
    assert len(summaries) == step['test_groups']
    if step['name'].startswith('workspace-'):
        assert '--locked' in step['command'] and '--all-features' in step['command']
        assert '--workspace' in step['command'] and totals[0] > 8625 and totals[1] == 0
        results[step['name']] = dict(passed=totals[0], failed=totals[1], ignored=totals[2],
                                     groups=len(summaries), command=step['command'])
assert set(results) == {'workspace-all-targets-all-features', 'workspace-and-docs-all-features'}
assert results['workspace-all-targets-all-features']['ignored'] == 0
assert results['workspace-and-docs-all-features']['ignored'] == 13
assert r['local_artifacts']
paths = set()
for row in r['local_artifacts']:
    assert row['fresh'] is False
    assert Path(row['manifest_path']).resolve().is_relative_to(ROOT)
    assert Path(row['target']['src_path']).resolve().is_relative_to(ROOT)
    for path, digest in row['sha256'].items():
        assert sha(path) == digest
        paths.add(path)
proof = dict(schema_version=1, complete=True, all_local_commands_terminal=True,
             owner_absent=True, actual_owner_session=args.owner_session,
             actual_owner_exit_code=args.owner_exit_code, source=r['source'], results=results,
             fresh_local_records=len(r['local_artifacts']), compiled_paths=len(paths),
             receipt_sha256=sha(OUT/'receipt.json'), audit_driver_sha256=sha(__file__),
             previous_terminal_sha256=r['previous_terminal_sha256'],
             javascript_executed_by_openui=False, full_renderer_contract_qualified=False,
             all_native_apis_qualified=False, release_qualified=False,
             retired_for_reexecution_after_source_or_target_mutation=True)
assert not PROOF.exists()
PROOF.write_text(json.dumps(proof, indent=2, sort_keys=True) + '\n')
print(json.dumps(proof), flush=True)
