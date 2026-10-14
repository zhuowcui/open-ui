import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT=Path('/home/nero/code/open-ui')
OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-font-units-chromium-v3628')
sys.path[:0]=[str(ROOT),'/tmp']
from openui_parallel_source_identity_v3060 import identity
from tools.qualification.residuals import analyze_image_difference
from tools.qualification.generate_renderer_contract import QUALIFICATION_PROFILES

def sha(path):
    h=hashlib.sha256()
    with Path(path).open('rb') as stream:
        for block in iter(lambda:stream.read(2**20),b''):
            h.update(block)
    return h.hexdigest()

load=lambda p:json.loads(Path(p).read_bytes())
r=load(OUT/'receipt.json')
assert r['all_commands_terminal'] and r['source_unchanged']
assert identity(ROOT)==r['source']==r['source_after'] and r['source']['clean']
assert subprocess.run(['ps','-p',str(r['owner_pid'])],capture_output=True).returncode==1
for step in r['steps']:
    assert subprocess.run(['ps','-p',str(step['pid'])],capture_output=True).returncode==1
for name in ['cargo','rustc','rustfmt','pixel_compare','native_font_uni']:
    assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
assert len(r['cases'])==len(QUALIFICATION_PROFILES)==4
members={}
for index,(row,profile) in enumerate(zip(r['cases'],QUALIFICATION_PROFILES)):
    assert (row['profile'],row['width'],row['height'],row['scale'])==profile[:4]
    directory=OUT/f'profile-{index}'
    assert sha(directory/'test.html')==row['input_sha256']
    for run in row['native_runs']:
        destination=Path(run['directory'])
        assert {p.name:sha(p) for p in sorted(destination.iterdir())}==run['files']
    assert row['native_runs'][0]['files']==row['native_runs'][1]['files']
    for stage,data in row['stages'].items():
        native=directory/'native-1'/f'{stage}.png'
        reference=directory/f'chromium-1-{stage}-capture-1.png'
        assert sha(native)==data['native_png_sha256']
        assert sha(reference)==data['chromium_png_sha256']
        assert load(directory/'native-1'/f'{stage}.json')==data['native_bounds']
        assert load(directory/f'chromium-1-{stage}.json')==data['chromium_bounds']
        assert data['bounds_exact']==(data['native_bounds']==data['chromium_bounds'])
        assert data['bounds_count']==len(data['native_bounds'])==4
        assert analyze_image_difference(reference,native)==data['analysis']
        for repeat in [1,2]:
            assert load(directory/f'chromium-{repeat}-{stage}.json')==data['chromium_bounds']
            for number in [1,2]:
                assert sha(directory/f'chromium-{repeat}-{stage}-capture-{number}.png')==data['chromium_png_sha256']
for p in sorted(OUT.rglob('*')):
    if p.is_file():
        members[str(p.relative_to(OUT))]=sha(p)
result=dict(schema_version=1,source=r['source'],actual_owner_exit_code=r['observed_exit_code'],
            actual_unified_session=79325,all_local_commands_terminal=True,owner_absent=True,
            receipt_sha256=sha(OUT/'receipt.json'),audit_driver_sha256=sha(__file__),members_sha256=members,
            profiles=[c['profile'] for c in r['cases']],
            bounds=sum(s['bounds_count'] for c in r['cases'] for s in c['stages'].values()),
            exact_bounds=sum(s['bounds_count'] for c in r['cases'] for s in c['stages'].values() if s['bounds_exact']),
            images=8,exact_images=sum(s['analysis']['mismatched_pixels']==0 for c in r['cases'] for s in c['stages'].values()),
            javascript_executed_by_openui=False,pixel_tolerance=0,full_renderer_contract_qualified=False,
            all_native_apis_qualified=False,release_qualified=False)
p=Path('/tmp/openui-native-font-units-chromium-terminal-v3629.json')
assert not p.exists()
p.write_text(json.dumps(result,sort_keys=True,indent=2)+'\n')
print(json.dumps({k:v for k,v in result.items() if k!='members_sha256'}),flush=True)
