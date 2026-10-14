"""Prepare the same tested text source with exact paths and branch preflight."""
import ast
import hashlib
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-text-retry-90310e15')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
BRANCH = 'agent/native-text-retry-v1649'
FIXED = '90310e15b86bed558a79c8eb085d2a72092eab69'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
sys.path.insert(0, str(ROOT / 'tools/qualification'))
from renderer_source_identity import repository_source_identity
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == FIXED
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip() == FIXED
checks_path = RAW / 'native-text-content-viewport-checks-v1626/receipt.json'
checks = json.loads(checks_path.read_bytes())
assert checks['source'] == checks['source_after'] == source and checks['all_commands_terminal']
assert len(checks['checks']) == 13 and all(row['observed_exit_code'] == 0 for row in checks['checks'])
old_owner_path = Path('/tmp/openui-native-text-content-viewport-pipeline-v1636.py')
old_text = old_owner_path.read_text()
config = ast.literal_eval(ast.parse(old_text).body[0].value)
old_priors = config['prior_pipelines']
priors = old_priors + [config['name']]
assert len(priors) == len(set(priors)) == 34
assert all(json.loads((RAW/name/'receipt.json').read_bytes())['all_commands_terminal'] for name in priors)
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode == 1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode == 1
replacements = [
    ('/dev/shm/openui-native-text-content-viewport-viewport-queue-90310e15',str(ROOT)),
    ('/dev/shm/openui-native-text-content-viewport-queue-90310e15',str(ROOT)),
    ('agent/native-text-content-viewport-viewport-queue-v1634',BRANCH),
    ('agent/native-text-content-viewport-queue-v1634',BRANCH),
    ('native-text-content-viewport-','native-text-retry-'),
    ('v1635','v1649'), ('v1636','v1650'),
    (repr(old_priors),repr(priors)),
    ('awaiting-all-33-prior-whole-pipelines','awaiting-all-34-prior-whole-pipelines'),
]
files = {}; original_hashes = {str(old_owner_path):sha(old_owner_path)}
preflight = []
for kind in ['guards','build','consumer','matrices']:
    old = Path('/tmp') / ('openui-native-text-content-viewport-' + kind + '-v1635.py')
    original_hashes[str(old)] = sha(old)
    text = old.read_text()
    for before,after in replacements:
        text = text.replace(before,after)
    tree = ast.parse(text)
    root_values = []
    branch_values = []
    for node in tree.body:
        if isinstance(node,ast.Assign):
            for target in node.targets:
                if isinstance(target,ast.Name) and target.id in ['ROOT','root']:
                    assert isinstance(node.value,ast.Call) and node.value.func.id == 'Path'
                    root_values.append(ast.literal_eval(node.value.args[0]))
                if isinstance(target,ast.Name) and target.id == 'BRANCH':
                    branch_values.append(ast.literal_eval(node.value))
    assert root_values == [str(ROOT)], (kind,root_values)
    assert (ROOT/'tools/qualification/renderer_source_identity.py').is_file()
    if kind == 'guards':
        assert branch_values == [BRANCH]
        text = text.replace("assert source['clean'] and source['commit'] == FIXED",
            "assert source['clean'] and source['commit'] == FIXED\nassert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip() == FIXED",1)
    ast.parse(text)
    assert 'viewport-viewport' not in text
    new = Path('/tmp') / ('openui-native-text-retry-' + kind + '-v1649.py')
    files[new] = text
    preflight.append(dict(kind=kind,root=str(ROOT),root_exists=True,source_module_exists=True,
                          guard_restore_branch=BRANCH if kind=='guards' else None))
config.update(root=str(ROOT),commit=FIXED,name='native-text-retry-pipeline-v1650',
              initial_state='awaiting-all-34-prior-whole-pipelines',prior_pipelines=priors,
              scripts=[p.name for p in files],
              selections=['native-text-content-viewport-checks-v1626/receipt.json',
                          'native-text-content-consumer-audit-v1615.json',
                          'native-raster-fields-retry-consumer-v1559/receipt.json'])
config['stages'] = [
    ('guards',['/usr/bin/python3','/tmp/openui-native-text-retry-guards-v1649.py'],
     'native-text-retry-guards-v1649/receipt.json',True),
    ('native-build',['/usr/bin/python3','/tmp/openui-native-text-retry-build-v1649.py'],
     'native-text-retry-clean-v1649/build.json',True),
    ('native-application',['/usr/bin/python3','/tmp/openui-native-text-retry-consumer-v1649.py'],
     'native-text-retry-consumer-v1649/receipt.json',True),
] + [(suite,['/usr/bin/python3','/tmp/openui-native-text-retry-matrices-v1649.py',suite],
      f'native-text-retry-clean-{suite}-v1649/{suite}-summary.json',False)
     for suite in ['focused','primitive','full','expanded']]
body = old_text.split('\n',1)[1]
for before,after in replacements:
    body = body.replace(before,after)
files[Path('/tmp/openui-native-text-retry-pipeline-v1650.py')] = 'CONFIG = ' + repr(config) + '\n' + body
for path,text in files.items():
    assert not path.exists()
    ast.parse(text)
    path.write_text(text)
assert repository_source_identity(ROOT) == source
report = dict(schema_version=1,source=source,source_after=source,source_root=str(ROOT),branch=BRANCH,
    baseline_commit='9fe1665dcf27df7541ec2983f353f642d4136d0e',production_code_unchanged=True,
    prior_whole_owners=34,prior_whole_pipelines=priors,all_predecessors_terminal=True,
    read_only_checks_passed=13,own_source_hosted_jobs_passed=7,own_source_hosted_skips=0,
    fresh_paths_and_restore_branch_preflight=preflight,required_stages=7,required_build_steps=12,
    required_native_cases=100,required_native_images=600,required_geometry_states=38400,
    all_four_pixel_matrices_required=True,c_exports=113,c_layouts=30,
    scripts={str(p):sha(p) for p in files},old_probe_sha256=original_hashes,
    checks_receipt_sha256=sha(checks_path),probe_sha256=sha(Path(__file__)),
    applied_to_umbrella=False,release_qualification=False,new_release_states_admitted=0,
    cargo_commands_run=0,screenshots_generated=0,javascript_executed_by_openui=False)
path = RAW/'native-text-retry-queue-prepared-v1649.json';assert not path.exists()
path.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(receipt=str(path),sha256=sha(path),source=FIXED,priors=34,
                     all_roots_and_restore_branch_verified=True,stages=7)),flush=True)
