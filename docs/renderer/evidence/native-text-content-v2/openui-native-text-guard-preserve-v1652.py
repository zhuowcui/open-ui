"""Preserve the text retry's actual baseline and fixed native guard results."""
import hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui'); RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-text-content-v2'; INDEX=ROOT/'docs/renderer/generated/native-text-content-v2.json'
assert not OUT.exists() and not INDEX.exists(); OUT.mkdir()
sha=lambda content:hashlib.sha256(content).hexdigest()
artifacts=[]
def keep(path,name,expected=None):
 content=path.read_bytes(); digest=sha(content)
 if expected is not None:assert digest==expected
 target=OUT/name;assert not target.exists();target.write_bytes(content)
 assert sha(path.read_bytes())==digest==sha(target.read_bytes())
 artifacts.append(dict(path=str(target.relative_to(ROOT)),sha256=digest,bytes=len(content),source_path=str(path)))
 return json.loads(content) if path.suffix=='.json' else None
p=RAW/'native-text-retry-guards-v1649/receipt.json'; d=keep(p,'native-guard-terminal.json')
assert d['all_commands_terminal'] and d['baseline_regression_reproduced'] and d['state']=='complete'
assert [r['observed_exit_code'] for r in d['steps']]==[0,101,0,0,0,0]
assert all(not r['disk_guard_triggered'] for r in d['steps'])
for r in d['steps']:keep(p.parent/(r['name']+'.log'),r['name']+'.log',r['log_sha256'])
assert d['steps'][-1]['test_counts']==[[58,0,0]]
keep(Path('/tmp/openui-native-text-retry-guards-v1649.py'),'guard-probe.py',d['probe_sha256'])
owner=keep(RAW/'native-text-retry-pipeline-v1650/receipt.json','whole-owner-at-observation.json')
keep(Path(__file__),Path(__file__).name)
previous=ROOT/'docs/renderer/generated/native-review-v4.json';previous_sha=sha(previous.read_bytes())
assert previous_sha=='e9c8776024883689bdebc3ccfd85a5b37b6f99289f6bc576e32d2e39f9dce83d'
report=dict(schema_version=1,source=d['source'],source_after=d['source_after'],
 baseline_commit='9fe1665dcf27df7541ec2983f353f642d4136d0e',baseline_named_failure_reproduced=True,
 fixed_native_c_text_guard_passed=True,fixed_engine_10000_update_storage_guard_passed=True,
 public_native_conformance_passed=58,source_root='/dev/shm/openui-native-text-retry-90310e15',
 owner='native-text-retry-pipeline-v1650',owner_state_at_observation=owner['state'],
 whole_owner_terminal_at_observation=owner['all_commands_terminal'],
 preceding_evidence=dict(path=str(previous.relative_to(ROOT)),sha256=previous_sha),
 hosted_seven_jobs_passed=True,hosted_source=d['source']['commit'],hosted_run=37355859674,
 required_native_images=600,required_geometry_states=38400,required_renderer_matrices=4,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,
 application_and_pixel_qualification_still_required=True,release_qualification=False,
 new_release_states_admitted=0,artifacts=artifacts,artifact_count=len(artifacts),
 artifact_bytes=sum(r['bytes'] for r in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)),sha256=sha(INDEX.read_bytes()),artifacts=len(artifacts))),flush=True)
