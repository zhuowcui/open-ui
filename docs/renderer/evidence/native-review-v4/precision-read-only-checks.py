"""Read-only contract, source formatting and C/C++ syntax checks."""
import concurrent.futures, hashlib, importlib.util, json, os, subprocess, sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-author-glyph-precision-e0dc491e')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-author-glyph-precision-checks-v1647'; STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']=='3b2e0d1f60b90813859c4c24325c7b0061dea29c'
checks=json.loads((RAW/'native-scroll-docs-checks-v543.json').read_bytes())['checks']
checks=checks+[dict(name='rust-format',command=['bash','-c','ulimit -s 262144; exec rustfmt --edition 2021 --config skip_children=true --check bindings/rust/openui-text/src/shaping/shape_result.rs'])]
def run(check):
 log=OUT/(check['name']+'.log')
 with log.open('xb') as stream:result=subprocess.run(check['command'],cwd=ROOT,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'),stdout=stream,stderr=subprocess.STDOUT)
 return dict(name=check['name'],command=check['command'],observed_exit_code=result.returncode,log_sha256=sha(log))
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(run,checks))
after=repository_source_identity(ROOT);assert source==after
report=dict(schema_version=1,source=source,source_after=after,checks=rows,all_commands_terminal=True,release_qualification=False,cargo_commands_run=0,screenshots_generated=0,probe_sha256=sha(Path(__file__)))
(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({r['name']:r['observed_exit_code'] for r in rows}),flush=True)
raise SystemExit(int(any(r['observed_exit_code'] for r in rows)))
