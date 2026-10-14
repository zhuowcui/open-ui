import argparse
import hashlib
import json
import subprocess
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--owner-session', type=int, required=True)
parser.add_argument('--audit-session', type=int, required=True)
parser.add_argument('--audit-exit-code', type=int, required=True)
args = parser.parse_args()
OUT = Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-all-features-v3640')
AUDIT = Path('/tmp/openui-native-font-units-all-features-terminal-v3641.json')
PROOF = Path('/tmp/openui-native-font-units-all-features-reaped-v3643.json')
r = json.loads((OUT/'receipt.json').read_bytes())
a = json.loads(AUDIT.read_bytes())
assert args.audit_exit_code == 0
assert r['all_commands_terminal'] and r['source_unchanged'] and r['observed_exit_code'] == 0
assert a['complete'] and a['owner_absent'] and a['actual_owner_session'] == args.owner_session
assert a['source'] == r['source'] == r['source_after']
assert a['receipt_sha256'] == hashlib.sha256((OUT/'receipt.json').read_bytes()).hexdigest()
assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode == 1
for name in ['cargo','rustc','rustfmt','pixel_compare']:
    assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode == 1
assert not PROOF.exists()
PROOF.write_text(json.dumps(dict(schema_version=1, source=r['source'],
    all_local_commands_terminal=True, actual_owner_session=args.owner_session,
    actual_owner_exit_code=0, actual_audit_session=args.audit_session,
    actual_audit_exit_code=args.audit_exit_code,
    completed_audit_sha256=hashlib.sha256(AUDIT.read_bytes()).hexdigest(),
    receipt_sha256=a['receipt_sha256'], results=a['results'],
    full_renderer_contract_qualified=False, all_native_apis_qualified=False,
    release_qualified=False, retired_for_reexecution_after_source_or_target_mutation=True),
    indent=2,sort_keys=True)+'\n')
print(PROOF, flush=True)
