import concurrent.futures
import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-c-raster-0601cd30')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT = RAW / 'native-c-raster-checks-v1294'
assert not OUT.exists()
OUT.mkdir()
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity

sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == '3395cefad116fa93a2db8f839ff707fb84230a3a'
env = dict(os.environ, PYTHONDONTWRITEBYTECODE='1')
checks = json.loads((RAW / 'native-scroll-docs-checks-v543.json').read_bytes())['checks']

def run(check):
    log = OUT / (check['name'] + '.log')
    with log.open('xb') as stream:
        result = subprocess.run(check['command'], cwd=ROOT, env=env, stdout=stream,
                                stderr=subprocess.STDOUT)
    return dict(name=check['name'], command=check['command'],
                observed_exit_code=result.returncode, log_sha256=sha(log))

with concurrent.futures.ThreadPoolExecutor(max_workers=4) as pool:
    rows = list(pool.map(run, checks))

parent = '0601cd30b9ba1d51d8642b164b74dea90d3e0809'
git_bytes = lambda name: subprocess.check_output(['git', 'show', parent + ':' + name], cwd=ROOT)
symbols_path = 'docs/v02/generated/openui-ffi-symbols.txt'
old_symbols = set(git_bytes(symbols_path).decode().splitlines())
new_symbols = set((ROOT / symbols_path).read_text().splitlines())
assert len(old_symbols) == 113 and len(new_symbols) == 117
assert old_symbols <= new_symbols
assert new_symbols - old_symbols == {
    'oui_raster_configuration_init_v1',
    'oui_document_create_with_raster_configuration_v1',
    'oui_app_create_with_raster_configuration_v1',
    'oui_document_get_raster_configuration_v1',
}
layout_path = 'docs/v02/generated/openui-ffi-layout.json'
old_layout = json.loads(git_bytes(layout_path))
new_layout = json.loads((ROOT / layout_path).read_bytes())
assert old_layout['abi_version'] == new_layout['abi_version'] == '0x00020000'
assert old_layout['pointer_width'] == new_layout['pointer_width'] == 64
assert len(old_layout['types']) == 30
assert all(new_layout['types'][name] == layout for name, layout in old_layout['types'].items())
assert new_layout['types']['OuiRasterConfigurationV1'] == dict(size=72, align=4)
assert new_layout['types']['OuiTextRasterConfigurationV1'] == dict(size=16, align=4)
assert len(list((ROOT / 'examples/c_v02').glob('*.c'))) == 12
assert len(list((ROOT / 'examples/c_v02').glob('*.cc'))) == 6
after = repository_source_identity(ROOT)
assert source == after
report = dict(schema_version=1, source=source, source_after=after,
              release_qualification=False, javascript_executed_by_openui=False,
              prior_exports_preserved=113, current_exports=117,
              prior_layouts_preserved=30, new_layouts=2, C_consumers=12, Cpp_consumers=6,
              binary_abi_verification_run=False, rust_compilation_run=False,
              pixel_comparison_run=False, checks=rows, probe_sha256=sha(Path(__file__)))
(OUT / 'receipt.json').write_text(json.dumps(report, sort_keys=True, indent=2) + '\n')
print(json.dumps({r['name']: r['observed_exit_code'] for r in rows}), flush=True)
print('ABI metadata: 113 old exports and 30 old layouts preserved; 117 exports prepared', flush=True)
raise SystemExit(int(any(r['observed_exit_code'] for r in rows)))
