import fcntl, hashlib, json, subprocess, sys
from pathlib import Path
ROOT = Path('/home/nero/code/open-ui')
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-keyboard-renderer-v3552')
AUDIT = Path('/tmp/openui-public-native-keyboard-renderer-completed-audit-v3555.json')
PROOF = Path('/tmp/openui-public-native-keyboard-renderer-terminal-v3556.json')
sha = lambda p: hashlib.sha256(Path(p).read_bytes()).hexdigest()
lock = open('/tmp/openui-native-cargo-raster-owner.lock', 'a+')
fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
r = json.loads((OUT / 'receipt.json').read_bytes()); a = json.loads(AUDIT.read_bytes())
assert a['complete'] and a['owner_terminal'] and a['owner_absent']
assert r['all_commands_terminal'] and r['source_unchanged']
assert a['receipt_sha256'] == sha(OUT / 'receipt.json')
assert a['actual_whole_exit_code'] == r['observed_exit_code']
assert subprocess.run(['ps', '-p', str(r['owner_pid'])], capture_output=True).returncode == 1
for name in ['cargo', 'rustc', 'rustfmt', 'pixel_compare']:
    assert subprocess.run(['pgrep', '-x', name], capture_output=True).returncode == 1
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
assert repository_source_identity(ROOT) == r['source'] == a['source']
proof = dict(schema_version=1, all_local_commands_terminal=True, source=r['source'],
             owner_pid=r['owner_pid'], owner_absent=True, actual_owner_exit_code=r['observed_exit_code'],
             actual_audit_exit_code=0, audit_sha256=sha(AUDIT), receipt_sha256=sha(OUT / 'receipt.json'),
             native_proof_sha256=r['native_completed_audit_sha256'], results=a['results'],
             preserved_chronological_native_and_renderer_proofs=True,
             retired_for_reexecution_after_source_or_target_mutation=True,
             renderer_qualified=False, all_native_apis_qualified=False, release_qualified=False)
assert not PROOF.exists()
PROOF.write_text(json.dumps(proof, sort_keys=True, indent=2) + '\n')
print(json.dumps(proof), flush=True)
