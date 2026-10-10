import argparse
import hashlib
import json
import subprocess
from pathlib import Path

parser=argparse.ArgumentParser()
parser.add_argument('--owner-exit-code',type=int,required=True)
parser.add_argument('--audit-session',type=int,required=True)
parser.add_argument('--audit-exit-code',type=int,required=True)
args=parser.parse_args()
OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/public-native-font-units-renderer-v3632')
AUDIT=Path('/tmp/openui-public-native-font-units-renderer-completed-audit-v3633.json')
load=lambda p:json.loads(Path(p).read_bytes())
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
r=load(OUT/'receipt.json')
a=load(AUDIT)
assert args.audit_exit_code==0
assert r['all_commands_terminal'] and r['source_unchanged'] and r['public_source_unchanged']
assert r['observed_exit_code']==args.owner_exit_code==a['actual_whole_exit_code']
assert a['owner_terminal'] and a['owner_absent'] and a['complete']
assert a['receipt_sha256']==sha(OUT/'receipt.json')
assert a['source']==r['source']==r['source_after']
assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode==1
for name in ['cargo','rustc','rustfmt','pixel_compare']:
    assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
p=Path('/tmp/openui-public-native-font-units-renderer-terminal-v3635.json')
assert not p.exists()
p.write_text(json.dumps(dict(schema_version=1,source=r['source'],all_local_commands_terminal=True,
                             actual_owner_session=27794,actual_owner_exit_code=args.owner_exit_code,
                             actual_audit_session=args.audit_session,actual_audit_exit_code=args.audit_exit_code,
                             completed_audit_sha256=sha(AUDIT),receipt_sha256=sha(OUT/'receipt.json'),
                             results=a['results'],candidate_nonregression=a['candidate_nonregression'],
                             frozen_chromium_fields_unchanged=True,
                             renderer_qualified=False,all_native_apis_qualified=False,release_qualified=False,
                             retired_for_reexecution_after_source_or_target_mutation=True),sort_keys=True,indent=2)+'\n')
print(p,flush=True)
