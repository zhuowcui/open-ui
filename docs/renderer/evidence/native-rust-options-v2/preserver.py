"""Freeze combined native API evidence without relabeling earlier source results."""
import gzip,hashlib,json,re,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-rust-options-v2';INDEX=ROOT/'docs/renderer/generated/native-rust-options-v2.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();rows=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 data=p.read_bytes();digest=hashlib.sha256(data).hexdigest()
 if expected:assert digest==expected
 target=OUT/name;target.parent.mkdir(parents=True,exist_ok=True);assert not target.exists();target.write_bytes(gzip.compress(data,mtime=0) if compress else data)
 row=dict(path=str(target.relative_to(ROOT)),sha256=sha(target),bytes=target.stat().st_size,source_path=str(p))
 if compress:
  assert gzip.decompress(target.read_bytes())==data;row.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(data))
 rows.append(row);return json.loads(data) if p.suffix=='.json' else None
buildpath=RAW/'native-rust-integrated-clean-v1810/build.json';build=keep(buildpath,'build.json')
assert build['all_commands_terminal'] and len(build['steps'])==15 and all(s['observed_exit_code']==0 and not s['disk_guard_triggered'] for s in build['steps'])
source=build['source'];assert source==build['source_after'] and source['clean'] and source['commit']=='2d338d6cf8334bae0d7893315d85633d2d586aad'
workspace=next(s for s in build['steps'] if s['name']=='workspace');assert (workspace['passed'],workspace['failed'],workspace['ignored'])==(8558,0,13)
guards=['native_raster_selection_survives_callbacks_cloning_and_resize','default_native_constructors_preserve_rendering_and_configuration','window_and_headless_apps_keep_explicit_native_raster_selection','selecting_ganesh_does_not_fall_back_to_cpu_raster','tests::native_fragment_keywords_reach_engine_and_keep_owned_property_identity','property::tests::native_fragment_keyword_values_cover_declared_enum_variants']
log=(buildpath.parent/'workspace.log').read_text()
for name in guards:assert re.search(r'^test '+re.escape(name)+r' \.\.\. ok$',log,re.M)
for step in build['steps']:
 keep(buildpath.parent/(step['name']+'.log'),'build-'+step['name']+'.log.gz',step['log_sha256'],True)
 if 'binary_sha256' in step:assert sha(Path(step['binary']))==step['binary_sha256']
assert 'C ABI verified: symbols=113 C_examples=13 C++=7 ran=True' in (buildpath.parent/'ffi-consumers.log').read_text()
checks_path=RAW/'native-rust-integrated-checks-v1809/receipt.json';checks=keep(checks_path,'checks.json')
assert checks['source']==checks['source_after']==source and checks['all_commands_terminal'] and len(checks['checks'])==15 and all(s['observed_exit_code']==0 for s in checks['checks'])
for c in checks['checks']:keep(checks_path.parent/(c['name']+'.log'),'check-'+c['name']+'.log.gz',c['log_sha256'],True)
probes={}
for name in ['native-rust-integrated-pipeline-v1811','native-rust-integrated-retry-pipeline-v1815','native-rust-integrated-retry-pipeline-v1816']:
 path=RAW/name/'receipt.json';owner=keep(path,name+'/receipt.json');assert owner['all_commands_terminal']
 if name.endswith('1815'):
  assert owner['observed_exit_code']==1 and owner['worker_processes_started']==0
 else:
  assert owner['source']==owner['source_after']==source
  assert [s['observed_exit_code'] for s in owner['steps']]==([0,1,1] if name.endswith('1811') else [0,0])
  for s in owner['steps']:keep(path.parent/(s['name']+'.log'),name+'/'+s['name']+'.log.gz',s['log_sha256'],True)
  probes.update(owner['immutable_probe_hashes'])
 probes['/tmp/openui-'+name+'.py']=owner['probe_sha256']
for suffix in ['keywords','options']:
 for retry in [False,True]:
  name='native-rust-integrated-'+('retry-' if retry else '')+suffix+'-consumer-'+('v1814' if retry else 'v1810')
  p=RAW/name/'receipt.json';d=keep(p,name+'/receipt.json')
  assert d['all_commands_terminal'] and d['source']==d['source_after']==source
  assert d['observed_exit_code']==(0 if retry else 1)
  if retry:
   assert d['totals']['pixel_exact']==d['totals']['geometry_exact']==d['totals']['images']==10 and not d['unstable_reference_captures']
   assert d['totals']['independent_chromium_capture_processes']==20 and d['totals']['consecutive_chromium_captures']==40
  else:assert d['failure']=='Chromium exited before exposing its DevTools endpoint'
  for f in sorted(p.parent.rglob('*')):
   if f.is_file() and f!=p and f.suffix in ['.png','.html','.log']:keep(f,name+'/'+str(f.relative_to(p.parent))+('.gz' if f.suffix=='.log' else ''),compress=f.suffix=='.log')
startup=RAW/'native-chromium-startup-diagnostic-v1813/receipt.json';d=keep(startup,'chromium-startup/receipt.json');assert d['all_commands_terminal'] and [r['endpoint_exposed'] for r in d['rows']]==[False,True]
for r in d['rows']:
 for kind in ['stdout','stderr']:keep(startup.parent/(r['name']+'.'+kind),'chromium-startup/'+r['name']+'.'+kind+'.gz',r[kind+'_sha256'],True)
probes['/tmp/openui-native-rust-integrated-checks-v1809.py']=checks['probe_sha256']
for p,digest in probes.items():keep(Path(p),'probes/'+Path(p).name,digest)
hostedpath=RAW/'native-current-hosted-v1803/receipt.json';hosted=keep(hostedpath,'preceding-c094-hosted/receipt.json')
assert hosted['all_commands_terminal'] and hosted['source']=='c094d645a1e184d287a4698f07dde5c86fa51255' and (hosted['successful_jobs'],hosted['skipped_jobs'],hosted['failed_jobs'])==(6,5,0)
for w in hosted['workflows']:keep(Path(w['path']),'preceding-c094-hosted/'+str(w['run_id'])+'.json',w['sha256'])
for j in hosted['job_logs']:
 if 'path' in j:keep(Path(j['path']),'preceding-c094-hosted/'+str(j['job_id'])+'.log.gz',j['sha256'],True)
keep(Path('/tmp/openui-native-current-hosted-collect-v1803.py'),'probes/preceding-hosted.py',hosted['probe_sha256'])
patch=subprocess.check_output(['git','diff','c094d645a1e184d287a4698f07dde5c86fa51255',source['commit']],cwd=ROOT);target=OUT/'source.patch';target.write_bytes(patch);rows.append(dict(path=str(target.relative_to(ROOT)),sha256=sha(target),bytes=len(patch)))
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=True,all_15_read_only_checks_passed=True,workspace_passed=8558,workspace_failed=0,workspace_ignored=13,all_15_build_stages_passed=True,executed_named_guard_names=guards,all_named_guards_executed_and_passed=True,native_rust_c_and_cpp_callback_consumers_passed=True,c_abi_exports=113,c_abi_layouts=30,c_abi_c_consumers_executed=13,c_abi_cpp_consumers_executed=7,native_pixel_exact=20,native_geometry_exact=20,independent_native_runs=4,independent_chromium_processes=40,consecutive_chromium_captures=80,original_chromium_startup_failures_preserved=True,retry_owner_preflight_failure_preserved=True,shorter_chromium_temporary_storage_starts_successfully=True,startup_root_cause_not_proven_by_stderr=True,combined_source_hosted_results_unexecuted=True,all_configuration_field_rendering_effects_unqualified=True,own_complete_renderer_matrices_unexecuted=True,release_qualification=False,javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,artifact_count=len(rows),artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'index':str(INDEX.relative_to(ROOT)),'sha256':sha(INDEX),'artifacts':len(rows),'native_exact':20}),flush=True)
