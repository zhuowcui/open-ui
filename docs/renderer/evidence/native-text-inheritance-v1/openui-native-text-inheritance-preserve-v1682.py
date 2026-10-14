"""Preserve measured native API corrections and all failed/interrupted evidence."""
import gzip,hashlib,json,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-text-inheritance-v1';INDEX=ROOT/'docs/renderer/generated/native-text-inheritance-v1.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();artifacts=[]
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected is not None:assert digest==expected,str(p)
 assert p.read_bytes()==b
 out=OUT/name;assert not out.exists();out.write_bytes(gzip.compress(b,compresslevel=9,mtime=0) if compress else b)
 row=dict(path=str(out.relative_to(ROOT)),sha256=sha(out),bytes=out.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(out.read_bytes())==b;row.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 artifacts.append(row);return json.loads(b) if p.suffix=='.json' else None
def generated(name,b):
 out=OUT/name;assert not out.exists();out.write_bytes(b);artifacts.append(dict(path=str(out.relative_to(ROOT)),sha256=sha(out),bytes=len(b)))
witness=keep(RAW/'owner-interruption-witness-v1664.json','owner-interruption-witness.json','e3b0b081e43fdb29917909d2d8f03a643430bf17d971f1a3219211be612d29e2')
keep(Path('/tmp/openui-owner-interruption-witness-v1664.py'),'owner-interruption-witness.py',witness['probe_sha256'])
for row in witness['owners']:
 keep(Path(row['receipt']),row['name']+'-interrupted.json',row['receipt_sha256'])
consumer=keep(Path(witness['consumer']['receipt']),'old-text-consumer-interrupted.json.gz',witness['consumer']['sha256'],compress=True)
assert not consumer['all_commands_terminal']
keep(RAW/'native-text-loader-retry-pipeline-v1656/native-application.log','old-text-consumer-interrupted.log')
oldbuild=RAW/'native-text-loader-retry-clean-v1655/build.json';build=keep(oldbuild,'old-text-build-terminal.json')
assert build['all_commands_terminal'] and len(build['steps'])==12 and all(r['observed_exit_code']==0 for r in build['steps'])
for row in build['steps']:keep(oldbuild.parent/(row['name']+'.log'),'old-build-'+row['name']+'.log.gz',row['log_sha256'],compress=True)
source=keep(RAW/'native-text-inheritance-source-v1668.json','shared-native-source.json')
keep(Path('/tmp/openui-native-text-inheritance-source-v1668.py'),'shared-native-source.py',source['probe_sha256'])
NATIVE=Path(source['root']);BASE=source['baseline'];FIXED=source['source']
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=NATIVE,text=True).strip()==FIXED
assert not subprocess.check_output(['git','status','--porcelain'],cwd=NATIVE)
generated('public-native-regression.patch',subprocess.check_output(['git','diff','--binary',source['parent'],BASE],cwd=NATIVE))
generated('shared-native-inheritance.patch',subprocess.check_output(['git','diff','--binary',source['parent'],FIXED],cwd=NATIVE))
for name in ['native-glyph-guard-v1666','native-text-style-qualification-guards-v1675','native-text-inheritance-checks-v1673']:
 p=RAW/name/'receipt.json';d=keep(p,name+'.json');assert d['all_commands_terminal']
 rows=d.get('steps',d.get('checks'))
 for r in rows:keep(p.parent/(r['name']+'.log'),name+'-'+r['name']+'.log.gz',r['log_sha256'],compress=True)
for ownername in ['native-glyph-retry-pipeline-v1667','native-text-style-qualification-pipeline-v1676']:
 p=RAW/ownername/'receipt.json';d=keep(p,ownername+'.json');assert d['all_commands_terminal']
 for r in d['steps']:keep(p.parent/(r['name']+'.log'),ownername+'-'+r['name']+'.log',r['log_sha256'])
for filename in ['native-glyph-retry-prepared-v1666.json','native-text-inheritance-prepared-v1669.json','native-text-style-qualification-prepared-v1675.json','native-text-style-runtime-prepared-v1677.json']:
 d=keep(RAW/filename,filename)
 for path,digest in d.get('scripts',d.get('files',{})).items():keep(Path(path),Path(path).name,digest)
for version,name in [(1666,'openui-native-glyph-retry-prepare'),(1669,'openui-native-text-inheritance-prepare'),(1671,'openui-native-text-style-qualification-prepare'),(1675,'openui-native-text-style-complete-prepare'),(1677,'openui-native-text-style-runtime-prepare')]:
 keep(Path('/tmp')/(name+f'-v{version}.py'),name+f'-v{version}.py')
generated('unlaunched-preflight-note.json',(json.dumps(dict(schema_version=1,v1671_preflight_exit=1,reason='prefix-only inheritance filter omitted the inherited-styles guard',native_owner_v1672_never_started=True,all_written_probes_preserved=True),sort_keys=True,indent=2)+'\n').encode())
newbuild=RAW/'native-text-style-runtime-clean-v1677/build.json';b=keep(newbuild,'new-text-build-terminal.json')
assert b['all_commands_terminal'] and len(b['steps'])==13 and all(r['observed_exit_code']==0 for r in b['steps'])
assert b['source']==b['source_after'] and b['source']['commit']==FIXED
for r in b['steps']:keep(newbuild.parent/(r['name']+'.log'),'new-build-'+r['name']+'.log.gz',r['log_sha256'],compress=True)
for name in ['native-text-style-runtime-pipeline-v1678','native-umbrella-terminal-ci-v1674']:
 p=RAW/name/'receipt.json';d=keep(p,name+'-at-observation.json')
 if name=='native-umbrella-terminal-ci-v1674':
  for row in d['workflows']:
   run=str(row['databaseId']);keep(p.parent/(run+'.json'),'ci-'+run+'.json');keep(p.parent/(run+'.log'),'ci-'+run+'.log.gz',row['log_sha256'],compress=True)
  keep(Path('/tmp/openui-native-umbrella-terminal-ci-v1674.py'),'terminal-ci.py',d['probe_sha256'])
relocation=keep(RAW/'retired-artifact-relocation-v1681.json','inactive-executable-relocation.json','6384f2dda9edaaf14ec4e332966cb9a373aa27e2c0d437d3d79bc4b266adf5fc')
keep(Path('/tmp/openui-retired-artifact-relocation-v1681.py'),'inactive-executable-relocation.py',relocation['probe_sha256'])
keep(Path(__file__),Path(__file__).name)
report=dict(schema_version=1,accepted_original_exact=21334,accepted_expanded_exact=22137,accepted_renderer_unchanged=True,
 source=b['source'],baseline=BASE,shared_text_and_native_inheritance_combined=True,style_properties_regenerated=True,
 native_public_font_regression_baseline_failed=True,native_public_font_regression_fixed_passed=True,native_callback_scales=5,
 all_nine_named_style_guards_passed=True,c_text_guard_and_10000_update_storage_passed=True,native_conformance_passed=58,
 source_read_only_checks_passed=13,clean_workspace=dict(passed=8551,failed=0,ignored=13),new_clean_build_all_13_stages_passed=True,
 all_113_c_exports_and_30_layouts_preserved=True,shared_native_source_unapplied=True,
 glyph_source='3b2e0d1f60b90813859c4c24325c7b0061dea29c',glyph_guard_baseline_failed_and_fixed_passed=True,glyph_text_tests_passed=342,glyph_parent_has_83_exact_losses=True,glyph_app_and_pixels_not_yet_verified=True,
 old_native_consumer_partial_images=306,old_native_consumer_exact_images=0,old_native_consumer_exact_geometry=0,
 old_actual_c_cpp_images_matching_rust=204,old_actual_c_cpp_images=204,rust_self_rows_not_parity_passes=True,
 interrupted_owners_not_relabelled_as_passes=True,old_consumer_root_cause_owner='openui-engine authored style inheritance',
 fresh_runtime_owner='native-text-style-runtime-pipeline-v1678',required_native_images=600,required_geometry_states=38400,required_matrices=4,
 private_own_source_hosted_run=37366409715,private_own_source_hosted_qualification_pending=True,
 umbrella_954_ci_attempt1=dict(successful_jobs=5,skipped_jobs=5,cancelled_jobs=1,workflow_failures=1,historical_audit_has_no_steps=True),
 umbrella_ci_failed_jobs_rerun_requested=True,inactive_executable_bytes_preserved=912641544,
 pixel_tolerance=0,javascript_executed_by_openui=False,public_native_rust_apis_required=True,
 release_qualification=False,new_release_states_admitted=0,artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for a in artifacts:
 assert sha(ROOT/a['path'])==a['sha256']
 if a.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress((ROOT/a['path']).read_bytes())).hexdigest()==a['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX),sha256=sha(INDEX),artifacts=len(artifacts),stored_bytes=report['artifact_bytes'])),flush=True)
