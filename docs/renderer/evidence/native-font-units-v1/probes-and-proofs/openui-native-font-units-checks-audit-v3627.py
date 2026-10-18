import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT=Path('/home/nero/code/open-ui')
OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-checks-v3626')
TARGET=Path('/mnt/e/openui-v02-cargo-c73754e2/target')
sys.path.insert(0,'/tmp')
from openui_parallel_source_identity_v3060 import identity

def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda:stream.read(2**20),b''):
            h.update(block)
    return h.hexdigest()

r=json.loads((OUT/'receipt.json').read_bytes())
assert r['all_commands_terminal'] and r['source_unchanged']
assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode==1
for name in ['cargo','rustc','rustfmt','pixel_compare']:
    assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
assert identity(ROOT)==r['source']==r['source_after'] and r['source']['clean']
for step in r['steps']:
    assert sha(OUT/(step['name']+'.log'))==step['log_sha256']
    values=re.findall(r'test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;', (OUT/(step['name']+'.log')).read_text())
    assert [sum(int(v[i]) for v in values) for i in range(3)]==[step['passed'],step['failed'],step['ignored']]
for row in r['local_artifacts']:
    assert row['fresh'] is False
    for path,digest in row['sha256'].items():
        assert sha(path)==digest,path
assert sha(OUT/'native_font_units')==r['native_binary_sha256']
assert sha(TARGET/'debug/libopenui_ffi.so')==r['ffi_library_sha256']
for case in r['cases']:
    assert case['runs'][0]['files']==case['runs'][1]['files']
    for run in case['runs']:
        directory=Path(run['directory'])
        assert {p.name:sha(p) for p in sorted(directory.iterdir())}==run['files']
        life=json.loads((directory/'lifecycle.json').read_bytes())
        assert life['callbacks']==1 and life['weak_teardown_passed'] and not life['javascript_executed_by_openui']
steps={s['name']:s for s in r['steps']}
workspace=steps['workspace-all-targets']
abi=(OUT/'abi-consumers.log').read_text()
counts=re.search(r'C ABI verified: symbols=(\d+) C_examples=(\d+) C\+\+=(\d+) ran=True',abi)
result=dict(schema_version=1,actual_unified_session=55766,actual_owner_exit_code=r['observed_exit_code'],
            all_local_commands_terminal=True,owner_absent=True,source=r['source'],
            receipt_sha256=sha(OUT/'receipt.json'),audit_driver_sha256=sha(__file__),
            native_binary_sha256=r['native_binary_sha256'],ffi_library_sha256=r['ffi_library_sha256'],
            fresh_local_records=len(r['local_artifacts']),compiled_paths=sum(len(a['sha256']) for a in r['local_artifacts']),
            workspace_passed=workspace['passed'],workspace_failed=workspace['failed'],workspace_ignored=workspace['ignored'],
            abi_counts=None if counts is None else [int(v) for v in counts.groups()],
            stages={s['name']:s['actual_exit_code'] for s in r['steps']},
            pixel_comparisons_executed=0,profile_scope='four diagnostic native app profiles; two differ from the required contract dimensions',
            full_renderer_contract_qualified=False,all_native_apis_qualified=False,release_qualified=False)
p=Path('/tmp/openui-native-font-units-checks-terminal-v3627.json')
assert not p.exists()
p.write_text(json.dumps(result,sort_keys=True,indent=2)+'\n')
print(json.dumps(result),flush=True)
