import fcntl, hashlib, json, subprocess, sys
from pathlib import Path
ROOT = Path('/home/nero/code/open-ui')
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-regressions-v3547')
AUDIT = Path('/tmp/openui-native-keyboard-regressions-completed-audit-v3551.json')
PROOF = Path('/tmp/openui-native-keyboard-regressions-terminal-v3553.json')
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
r = json.loads((OUT / 'receipt.json').read_bytes()); a = json.loads(AUDIT.read_bytes())
assert a['complete'] and a['actual_whole_exit_code'] == r['observed_exit_code'] == 0
assert a['receipt_sha256'] == sha(OUT / 'receipt.json')
assert r['all_commands_terminal'] and subprocess.run(['ps', '-p', str(r['owner_pid'])], capture_output=True).returncode == 1
for name in ['cargo', 'rustc', 'rustfmt', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', name], capture_output=True).returncode == 1
sys.path.insert(0, '/tmp')
from openui_parallel_source_identity_v3060 import identity
assert identity(ROOT) == a['source'] == r['source']
hosted = json.loads(Path('/tmp/openui-native-keyboard-current-hardening-v3557.json').read_bytes())
pr = json.loads(Path('/tmp/openui-native-keyboard-current-pr-status-v3554.json').read_bytes())
assert hosted['head'] == pr['head'] == a['source']['commit']
assert len(hosted['jobs']) == 7 and all(j['status'] == 'completed' and j['conclusion'] == 'success' for j in hosted['jobs'])
assert sum(j['conclusion'] == 'SUCCESS' for j in pr['jobs']) == 6
proof = dict(schema_version=1, all_local_commands_terminal=True,
             actual_owner_session_exit_code=0, actual_audit_session_exit_code=0,
             owner_pid=r['owner_pid'], owner_absent=True, source=a['source'],
             audit_sha256=sha(AUDIT), receipt_sha256=sha(OUT / 'receipt.json'),
             hosted_pr_snapshot_sha256=sha('/tmp/openui-native-keyboard-current-pr-status-v3554.json'),
             hosted_hardening_snapshot_sha256=sha('/tmp/openui-native-keyboard-current-hardening-v3557.json'),
             hosted_regular_jobs_passed=6, hosted_explicit_hardening_jobs_passed=7,
             prior_audit_retired_after_next_source_or_target_mutation=True,
             native_behavior_comparisons=a['native_behavior_comparisons'],
             all_native_apis_qualified=False, renderer_qualified=False, release_qualified=False)
assert not PROOF.exists()
PROOF.write_text(json.dumps(proof, sort_keys=True, indent=2) + '\n')
print(json.dumps(proof), flush=True)
