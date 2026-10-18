"""Preserve the clean text build's loader stop and both fresh verification queues."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui'); RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-review-v5'; INDEX=ROOT/'docs/renderer/generated/native-review-v5.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir()
sha=lambda b:hashlib.sha256(b).hexdigest();artifacts=[]
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=sha(b)
 if expected is not None:assert digest==expected,str(p)
 assert p.read_bytes()==b
 target=OUT/name;assert not target.exists();target.write_bytes(gzip.compress(b,compresslevel=9,mtime=0) if compress else b)
 row=dict(path=str(target.relative_to(ROOT)),sha256=sha(target.read_bytes()),bytes=target.stat().st_size,source_path=str(p))
 if compress:
  assert gzip.decompress(target.read_bytes())==b;row.update(encoding='gzip',decompressed_bytes=len(b),decompressed_sha256=digest)
 else:assert target.read_bytes()==b
 artifacts.append(row);return json.loads(b) if p.suffix=='.json' else None
owner=keep(RAW/'native-text-retry-pipeline-v1650/receipt.json','text-owner-terminal.json')
assert owner['all_commands_terminal'] and [r['observed_exit_code'] for r in owner['steps']]==[0,127]
for r in owner['steps']:keep(RAW/'native-text-retry-pipeline-v1650'/(r['name']+'.log'),'owner-'+r['name']+'.log',r['log_sha256'])
buildpath=RAW/'native-text-retry-clean-v1649/build.json';build=keep(buildpath,'text-build-terminal.json')
assert build['all_commands_terminal'] and build['source']==build['source_after']==owner['source']
assert [r['observed_exit_code'] for r in build['steps']]==[0,0,0,0,0,0,0,0,127]
assert build['steps'][1]['passed']==8538 and build['steps'][1]['failed']==0 and build['steps'][1]['ignored']==13
for r in build['steps']:
 keep(buildpath.parent/(r['name']+'.log'),'build-'+r['name']+'.log.gz',r['log_sha256'],compress=True)
 for k in ['binary','installed_soname']:
  if k in r:assert sha(Path(r[k]).read_bytes())==r['binary_sha256']
assert b'libopenui.so.0: cannot open shared object file' in (buildpath.parent/'run-c.log').read_bytes()
loader=keep(RAW/'native-text-loader-retry-prepared-v1655.json','loader-retry-prepared.json','c8a05e1df5fc399940ab0de36881f05c1be5d43c5a4fc9200ed6d1deb4b445d7')
for p,digest in loader['scripts'].items():keep(Path(p),Path(p).name,digest)
keep(Path('/tmp/openui-native-text-loader-prepare-v1655.py'),'loader-retry-prepare.py',loader['probe_sha256'])
loaderowner=keep(RAW/'native-text-loader-retry-pipeline-v1656/receipt.json','loader-owner-at-observation.json')
glyph=keep(RAW/'native-glyph-guard-prepared-v1657.json','glyph-guard-prepared.json','3c9b11a585fa6ded11f5c7c245087ea05eba42d6e593d9c44195a0d31e7629f5')
for p,digest in glyph['scripts'].items():keep(Path(p),Path(p).name,digest)
keep(Path('/tmp/openui-native-glyph-guard-prepare-v1657.py'),'glyph-guard-prepare.py',glyph['probe_sha256'])
glyphowner=keep(RAW/'native-glyph-guard-pipeline-v1658/receipt.json','glyph-owner-at-observation.json')
checks=keep(RAW/'native-review-umbrella-checks-v1653/receipt.json','preceding-umbrella-read-only.json')
assert checks['all_commands_terminal'] and len(checks['checks'])==10 and all(r['observed_exit_code']==0 for r in checks['checks'])
keep(Path('/tmp/openui-native-review-umbrella-checks-v1653.py'),'preceding-umbrella-read-only.py',checks['probe_sha256'])
keep(Path(__file__),Path(__file__).name)
report=dict(schema_version=1,accepted_original_exact=21334,accepted_expanded_exact=22137,accepted_renderer_unchanged=True,
 text_source=build['source'],text_guard_baseline_named_failure_and_fixed_pass=True,text_engine_10000_update_guard_passed=True,
 text_native_conformance_passed=58,text_clean_workspace=dict(passed=8538,failed=0,ignored=13),text_build_completed_steps=9,
 text_build_actual_exit=127,text_build_failure='C loader cannot find libopenui.so.0: the harness installed only libopenui_ffi.so',
 text_abi_consumers_passed=True,old_source_and_artifacts_unchanged=True,text_cpp_build_and_app_images_not_executed=True,
 loader_retry=dict(source=loader['source'],owner='native-text-loader-retry-pipeline-v1656',
 observed_state=loaderowner['state'],terminal_at_observation=loaderowner['all_commands_terminal'],
 installed_soname='libopenui.so.0',soname_bytes_verified_against_built_library=True,prior_whole_owners=35,
 required_stages=7,required_native_images=600,required_geometry_states=38400,all_pixel_gates_still_required=True),
 glyph_guard=dict(source=glyph['source_commit'],baseline=glyph['baseline_commit'],owner='native-glyph-guard-pipeline-v1658',
 observed_state=glyphowner['state'],terminal_at_observation=glyphowner['all_commands_terminal'],prior_whole_owners=36,
 guard_only=True,parent_has_83_exact_losses=True,native_app_and_pixel_gates_still_required=True),
 preceding_umbrella_read_only=dict(source=checks['source'],passed=10),pixel_tolerance=0,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,release_qualification=False,new_release_states_admitted=0,
 artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)),sha256=sha(INDEX.read_bytes()),artifacts=len(artifacts),bytes=report['artifact_bytes'])),flush=True)
