import hashlib, importlib.util, json, subprocess, sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-style-integration-287e176a')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-style-integration-relative-c-v1418'
STORE = Path('/mnt/e/openui-v02-qualification-d174ea0b') / OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir()
OUT.symlink_to(STORE, target_is_directory=True)
spec = importlib.util.spec_from_file_location('ffi_check', ROOT/'tools/ffi/verify_abi.py')
ffi_check = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ffi_check)
cc,cxx,c_flags,cxx_flags,link_flags = ffi_check.compilers()
sys.path.insert(0, str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
buildpath = RAW/'native-style-integration-clean-v1418/build.json'
build = json.loads(buildpath.read_text())
source = repository_source_identity(ROOT)
assert source == build['source'] == build['source_after'] and source['clean']
assert len(build['steps']) == 9 and all(s['observed_exit_code'] == 0 for s in build['steps'])
lib = buildpath.parent/'libopenui_ffi.so'
assert sha(lib) == next(s['binary_sha256'] for s in build['steps'] if s['name'] == 'ffi-build')
inputpath = RAW/'native-relative-query-v1087/openui-native-relative-query-v1087.c'
old = json.loads((RAW/'native-relative-query-v1087/receipt.json').read_text())
assert sha(inputpath) == old['input_sha256']
inputcopy = OUT/'query.c'
inputcopy.write_bytes(inputpath.read_bytes())
(OUT/'libopenui.so.0').symlink_to(lib.resolve())
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode == 1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode == 1
commands = [
    [cc,'-std=c11','-Wall','-Wextra','-Werror',*c_flags,'-I'+str(ROOT/'include'),str(inputcopy),'-c','-o',str(OUT/'query.o')],
    [cxx,*cxx_flags,*link_flags,'-fuse-ld=lld',str(OUT/'query.o'),str(lib),'-Wl,-rpath,'+str(OUT),'-o',str(OUT/'query')],
    [str(OUT/'query')],
]
report = dict(schema_version=1, release_qualification=False, public_native_c_api=True,
              native_queries_only=True, source=source, source_after=source,
              library_sha256=sha(lib), build_receipt_sha256=sha(buildpath),
              input_sha256=sha(inputcopy), probe_sha256=sha(Path(__file__)), steps=[])
for i,command in enumerate(commands):
    result = subprocess.run(command, capture_output=True, cwd=ROOT)
    log = OUT/f'{i}.log'
    log.write_bytes(result.stdout+result.stderr)
    report['steps'].append(dict(command=command,observed_exit_code=result.returncode,log_sha256=sha(log)))
    report['source_after'] = repository_source_identity(ROOT)
    assert source == report['source_after']
    (OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
    if result.returncode:
        report.update(all_commands_terminal=True, observed_exit_code=result.returncode)
        (OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
        raise SystemExit(result.returncode)
observations = {line.split()[0]:float(line.split()[1]) for line in (OUT/'2.log').read_text().splitlines()}
oraclepath = RAW/'native-relative-oracle-v1088/receipt.json'
oracle = json.loads(oraclepath.read_text())
assert oracle['independent_reference_queries_identical'] and oracle['captures'][0] == oracle['captures'][1]
assert set(observations) == {row['state'] for row in oracle['native_c_comparisons']}
report['reference_receipt_sha256'] = sha(oraclepath)
report['comparisons'] = [dict(state=state,native_width=width,chromium_width=oracle['captures'][0]['rows'][state]['width'],exact=width==oracle['captures'][0]['rows'][state]['width'],owner=None if width==oracle['captures'][0]['rows'][state]['width'] else 'openui-engine') for state,width in observations.items()]
report['total'] = len(report['comparisons'])
report['exact'] = sum(row['exact'] for row in report['comparisons'])
report['all_commands_terminal'] = True
(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(exact=report['exact'],total=report['total'])),flush=True)
raise SystemExit(int(report['exact'] != report['total']))
