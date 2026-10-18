"""Reuse actual guards only when every source/input byte matches."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-glyph-raster-3b2e0d1f');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);p=RAW/'native-glyph-guard-v1666/receipt.json';d=json.loads(p.read_bytes())
assert hashlib.sha256(p.read_bytes()).hexdigest()=='68a1d693f17c70cfdebc1dd98177e3bd5386e4ff6bf76c07e85784bc15dc60e3'
assert source==d['source']==d['source_after'] and source['clean']
assert d['all_commands_terminal'] and d['baseline_regression_reproduced'] and d['state']=='complete'
for r in d['steps']:assert hashlib.sha256((p.parent/(r['name']+'.log')).read_bytes()).hexdigest()==r['log_sha256']
print(json.dumps({'terminal_guards_reused':True,'source':source['commit'],'baseline_failure_and_fixed_pass_verified':True,'new_tests_run':0}),flush=True)
