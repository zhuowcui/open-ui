"""Reuse actual guards only when every source/input byte matches."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-intrinsic-snap-full-727da10e');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);p=RAW/'native-intrinsic-snap-guards-v1702/receipt.json';d=json.loads(p.read_bytes())
assert hashlib.sha256(p.read_bytes()).hexdigest()=='4c761b86cd60dcc8c08fc30aa7ce8314d7af958e03c8411636f6a9363e36baee'
assert source==d['source']==d['source_after'] and source['clean']
assert d['all_commands_terminal'] and d['baseline_regression_reproduced'] and d['state']=='complete'
for r in d['steps']:assert hashlib.sha256((p.parent/(r['name']+'.log')).read_bytes()).hexdigest()==r['log_sha256']
print(json.dumps({'terminal_guards_reused':True,'source':source['commit'],'baseline_failure_and_fixed_pass_verified':True,'new_tests_run':0}),flush=True)

assert [r['observed_exit_code'] for r in d['steps']] == [0,101,0,0,0,0,0,0,0]
