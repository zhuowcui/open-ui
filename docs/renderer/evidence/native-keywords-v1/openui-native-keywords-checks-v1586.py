"""Read-only contract, source formatting and C/C++ syntax checks."""
import concurrent.futures, hashlib, importlib.util, json, os, subprocess, sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-keywords-92741843')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-keywords-checks-v1586'; STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']=='06e1f89a4a2e7a53465bceb780675383d9748464'
checks=json.loads((RAW/'native-scroll-docs-checks-v543.json').read_bytes())['checks']
spec=importlib.util.spec_from_file_location('ffi_verification',ROOT/'tools/ffi/verify_abi.py');module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
cc,cxx,cflags,cxxflags,link=module.compilers()
checks=checks+[dict(name='c-syntax',command=[cc,'-std=c11','-Wall','-Wextra','-Werror',*cflags,'-Iinclude','-fsyntax-only','examples/c_v02/fragment_keywords.c']),dict(name='cpp-syntax',command=[cxx,'-std=c++17','-Wall','-Wextra','-Werror',*cxxflags,'-Iinclude','-fsyntax-only','examples/c_v02/fragment_keywords.cc']),dict(name='rust-format',command=['bash','-c','ulimit -s 262144; exec rustfmt --edition 2021 --config skip_children=true --check bindings/rust/openui-style/src/property.rs bindings/rust/openui-ffi/src/lib.rs bindings/rust/openui/examples/native_fragment_keywords.rs'])]
def run(check):
 log=OUT/(check['name']+'.log')
 with log.open('xb') as stream:result=subprocess.run(check['command'],cwd=ROOT,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'),stdout=stream,stderr=subprocess.STDOUT)
 return dict(name=check['name'],command=check['command'],observed_exit_code=result.returncode,log_sha256=sha(log))
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(run,checks))
after=repository_source_identity(ROOT);assert source==after
report=dict(schema_version=1,source=source,source_after=after,checks=rows,all_commands_terminal=True,release_qualification=False,cargo_commands_run=0,screenshots_generated=0,probe_sha256=sha(Path(__file__)))
(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({r['name']:r['observed_exit_code'] for r in rows}),flush=True)
raise SystemExit(int(any(r['observed_exit_code'] for r in rows)))
