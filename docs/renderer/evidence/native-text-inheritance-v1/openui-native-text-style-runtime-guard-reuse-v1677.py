"""Reuse actual guards only when every source/input byte matches."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-text-style-runtime-41b616c3');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);p=RAW/'native-text-style-qualification-guards-v1675/receipt.json';d=json.loads(p.read_bytes())
assert hashlib.sha256(p.read_bytes()).hexdigest()=='f32050fea37b3a4dc8d0e6628cc25bcfd114c8d0b23b4f104c2bc27139f25fb8'
assert source==d['source']==d['source_after'] and source['clean']
assert d['all_commands_terminal'] and d['baseline_regression_reproduced'] and d['state']=='complete'
for r in d['steps']:assert hashlib.sha256((p.parent/(r['name']+'.log')).read_bytes()).hexdigest()==r['log_sha256']
print(json.dumps({'terminal_guards_reused':True,'source':source['commit'],'baseline_failure_and_fixed_pass_verified':True,'new_tests_run':0}),flush=True)
