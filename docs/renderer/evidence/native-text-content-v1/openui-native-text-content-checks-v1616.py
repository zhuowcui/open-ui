"""Read-only contract, source formatting and C/C++ syntax checks."""
import concurrent.futures, hashlib, importlib.util, json, os, subprocess, sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-text-content-f25cd722')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-text-content-checks-v1616'; STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit'].startswith('d5bd17d7')
checks=json.loads((RAW/'native-scroll-docs-checks-v543.json').read_bytes())['checks']
checks=checks+[dict(name='rust-format',command=['bash','-c','ulimit -s 262144; exec rustfmt --edition 2021 --config skip_children=true --check bindings/rust/openui-engine/src/lib.rs bindings/rust/openui/src/element.rs bindings/rust/openui-ffi/src/lib.rs bindings/rust/openui/examples/native_text_content.rs'])]
spec=importlib.util.spec_from_file_location('abi_verify',ROOT/'tools/ffi/verify_abi.py')
abi_verify=importlib.util.module_from_spec(spec);spec.loader.exec_module(abi_verify)
cc,cxx,c_flags,cxx_flags,_=abi_verify.compilers();assert cc and cxx
for language,compiler,standard,flags,suffix in [('c',cc,'c11',c_flags,'c'),('cpp',cxx,'c++17',cxx_flags,'cc')]:
 checks.append(dict(name='syntax-'+language,command=[compiler,'-std='+standard,'-Wall','-Wextra','-Werror',*flags,'-I'+str(ROOT/'include'),'-fsyntax-only',str(ROOT/'examples/c_v02'/('text_content.'+suffix))]))
def run(check):
 log=OUT/(check['name']+'.log')
 with log.open('xb') as stream:result=subprocess.run(check['command'],cwd=ROOT,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'),stdout=stream,stderr=subprocess.STDOUT)
 return dict(name=check['name'],command=check['command'],observed_exit_code=result.returncode,log_sha256=sha(log))
with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:rows=list(pool.map(run,checks))
after=repository_source_identity(ROOT);assert source==after
report=dict(schema_version=1,source=source,source_after=after,checks=rows,all_commands_terminal=True,release_qualification=False,cargo_commands_run=0,screenshots_generated=0,probe_sha256=sha(Path(__file__)))
(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({r['name']:r['observed_exit_code'] for r in rows}),flush=True)
raise SystemExit(int(any(r['observed_exit_code'] for r in rows)))
