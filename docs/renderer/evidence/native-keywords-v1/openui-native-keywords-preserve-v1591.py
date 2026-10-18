"""Preserve completed evidence and the unqualified native API retry."""
import collections,hashlib,json,shutil,subprocess,sys
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
NATIVE=Path('/dev/shm/openui-native-keywords-92741843')
OUT=ROOT/'docs/renderer/evidence/native-keywords-v1';INDEX=ROOT/'docs/renderer/generated/native-keywords-v1.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir()
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
artifacts=[]
def preserve(path,name,expected=None):
    digest=sha(path)
    if expected is not None:assert digest==expected,str(path)
    target=OUT/name;assert not target.exists();shutil.copyfile(path,target)
    assert sha(path)==sha(target)==digest
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=digest,bytes=target.stat().st_size))
def snapshot(path,name):
    content=path.read_bytes();target=OUT/name;assert not target.exists();target.write_bytes(content)
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=sha(target),bytes=len(content)))
    return json.loads(content)
queue_path=RAW/'native-keywords-queue-prepared-v1587.json';queue=json.loads(queue_path.read_bytes());source=queue['source']
sys.path.insert(0,str(NATIVE/'tools/qualification'))
from renderer_source_identity import repository_source_identity
assert source==repository_source_identity(NATIVE) and source['commit']=='06e1f89a4a2e7a53465bceb780675383d9748464'
preserve(queue_path,'queue-prepared.json')
for path,digest in queue['scripts'].items():preserve(Path(path),Path(path).name,digest)
for name in ['checks-v1586','prepare-v1587','dispatch-v1589','hosted-v1590']:
    path=Path('/tmp/openui-native-keywords-'+name+'.py');preserve(path,path.name)
preserve(Path('/tmp/openui-native-keywords-dispatch-v1589.json'),'hosted-dispatch.json')
checks_path=RAW/'native-keywords-checks-v1586/receipt.json';checks=json.loads(checks_path.read_bytes())
assert checks['source']==checks['source_after']==source and checks['all_commands_terminal']
assert len(checks['checks'])==13 and all(r['observed_exit_code']==0 for r in checks['checks'])
preserve(checks_path,'read-only-checks.json',queue['checks_receipt_sha256'])
for row in checks['checks']:preserve(checks_path.parent/(row['name']+'.log'),'check-'+row['name']+'.log',row['log_sha256'])
for rel in ['bindings/rust/openui/examples/native_fragment_keywords.rs','examples/c_v02/fragment_keywords.c','examples/c_v02/fragment_keywords.cc','docs/v02/generated/openui-ffi-layout.json']:
    preserve(NATIVE/rel,Path(rel).name)
for name,start,end in [('native-api-guards.patch','92741843283d02d6a1e7f82d84f69acc8dcee5f0',queue['baseline_commit']),('native-keyword-construction.patch',queue['baseline_commit'],source['commit'])]:
    path=OUT/name;path.write_bytes(subprocess.check_output(['git','diff','--binary',start,end],cwd=NATIVE))
    artifacts.append(dict(path=path.relative_to(ROOT).as_posix(),sha256=sha(path),bytes=path.stat().st_size))
keyword_owner=snapshot(RAW/'native-keywords-pipeline-v1588/receipt.json','owner-at-observation.json')
assert keyword_owner['source']==source and keyword_owner['all_commands_terminal'] is False and keyword_owner['steps']==[]
keyword_hosted=snapshot(RAW/'native-keywords-hosted-v1590/receipt.json','hosted-at-observation.json')
assert keyword_hosted['source']==source['commit'] and keyword_hosted['run_id']==37340082754
for version in [1582,1583,1584]:
    path=Path('/tmp/openui-native-table-c-geometry-v'+str(version)+'.py');preserve(path,path.name)
for version in [1583,1584]:
    path=RAW/f'native-table-c-geometry-v{version}/receipt.json';data=json.loads(path.read_bytes())
    assert data['all_commands_terminal'] and data['observed_exit_code']==1 and data['runs']==[]
    preserve(path,f'failed-c-geometry-v{version}.json')
failed_c=json.loads((RAW/'native-table-c-geometry-v1584/receipt.json').read_bytes())
assert failed_c['failure']=="('parse COLUMN_FILL auto', -1)"
ci_path=RAW/'native-inline-fallback-umbrella-ci-v1580/receipt.json';ci=json.loads(ci_path.read_bytes())
assert ci['commit']=='92741843283d02d6a1e7f82d84f69acc8dcee5f0' and ci['all_three_hosted_workflows_complete_success']
assert ci['job_conclusions']==dict(success=6,skipped=5)
preserve(ci_path,'umbrella-hosted.json')
for row in ci['workflows']:
    run=str(row['databaseId']);preserve(ci_path.parent/(run+'.json'),'umbrella-'+run+'.json')
    preserve(ci_path.parent/(run+'.log'),'umbrella-'+run+'.log',row['captured_log_sha256'])
preserve(Path('/tmp/openui-native-inline-fallback-umbrella-ci-v1580.py'),'openui-native-inline-fallback-umbrella-ci-v1580.py')
fallback_hosted_path=RAW/'native-inline-fallback-hosted-v1577/receipt.json';fallback_hosted=json.loads(fallback_hosted_path.read_bytes())
assert fallback_hosted['all_commands_terminal'] and fallback_hosted['actual_exit']==0 and fallback_hosted['all_seven_jobs_passed']
assert fallback_hosted['source']=='7d09f7a159b2c3f211662d20dbf7c7092d6024ed' and fallback_hosted['job_conclusions']==dict(success=7)
preserve(fallback_hosted_path,'fallback-hosted.json')
run=str(fallback_hosted['run_id'])
preserve(fallback_hosted_path.parent/(run+'.json'),'fallback-'+run+'.json',fallback_hosted['detail_sha256'])
preserve(fallback_hosted_path.parent/(run+'.log'),'fallback-'+run+'.log',fallback_hosted['log_sha256'])
preserve(Path('/tmp/openui-native-inline-fallback-hosted-v1577.py'),'openui-native-inline-fallback-hosted-v1577.py')
application_path=RAW/'native-raster-fields-retry-application-v1559/receipt.json';app=json.loads(application_path.read_bytes())
assert app['all_commands_terminal'] and app['observed_exit_code']==1 and app['chromium_pixel_qualification'] is False
assert app['source']==app['source_after'] and app['source']['commit']=='e0dc491e61e17ce4407ff2dd30289e741690572b'
assert app['totals']==dict(cases=840,contract_exact=828,geometry_and_callback_states=107520,integer_phase_image_checks=800)
preserve(application_path,'raster-application.json')
raster_owner=snapshot(RAW/'native-raster-fields-retry-pipeline-v1560/receipt.json','raster-owner-at-observation.json')
app_step=next(row for row in raster_owner['steps'] if row['name']=='application');assert app_step['observed_exit_code']==1
preserve(RAW/'native-raster-fields-retry-pipeline-v1560/application.log','raster-application-owner.log',app_step['log_sha256'])
failures=[row for row in app['cases'] if row['contract_exact'] is False]
assert len(failures)==12 and all(row['native_application_success'] and row['deterministic'] and row['geometry_unchanged_by_phase'] for row in failures)
failure_groups={key:dict(collections.Counter(str(row.get(key)) for row in failures)) for key in ['family','policy','scale','hinting','edging']}
for path,digest in queue['scripts'].items():assert sha(Path(path))==digest
assert repository_source_identity(NATIVE)==source
preserve(Path(__file__),Path(__file__).name)
report=dict(schema_version=1,observed_at_utc=datetime.now(timezone.utc).isoformat(),source=source,
    baseline_commit=queue['baseline_commit'],branch=queue['branch'],applied_to_umbrella=False,
    accepted_renderer_unchanged=True,release_qualification=False,promotion_allowed=False,
    new_release_states_admitted=0,pixel_tolerance=0,javascript_executed_by_openui=False,
    public_native_rust_api_required=True,needed_native_operations_not_waived_by_pixel_exclusion=True,
    old_openui_pixels_are_provenance_only=True,root_cause_owner='openui-style native keyword construction and property-bound C compound ownership',
    c_api_gap=dict(source=failed_c['source'],operation='oui_style_value_parse(ColumnFill, auto)',observed_status=-1,
        diagnostic_actual_exit=1,completed_geometry_queries=0,preflight_and_adapter_failures_preserved=True),
    native_keywords=dict(enum_types=7,author_properties=13,table_display_roles_added=9,
        generated_from_shared_style_schema=True,current_exports=113,all_30_struct_layouts_unchanged=True,
        public_rust_typed_setters_already_available=True,public_rust_c_and_cpp_consumers_prepared=True,
        read_only_checks_passed=13,whole_owner='native-keywords-pipeline-v1588',observed_state=keyword_owner['state'],
        prior_whole_owners=30,required_stages=8,required_named_fixed_guards=3,
        required_native_images=10,required_independent_chromium_processes=20,required_consecutive_chromium_captures=40,
        native_table_geometry_diagnostic_only=True,native_and_pixel_stages_not_started=True),
    hosted_keyword_observation=dict(source=source['commit'],run_id=37340082754,state=keyword_hosted['state'],
        terminal_at_observation=keyword_hosted['all_commands_terminal'],native_and_pixel_qualification=False),
    umbrella_hosted=dict(source=ci['commit'],successful_workflows=3,successful_jobs=6,skipped_jobs=5,skips_are_not_passes=True),
    fallback_hosted=dict(source=fallback_hosted['source'],successful_jobs=7,skipped_jobs=0,actual_exit=0,
        local_native_and_pixel_stages_still_required=True),
    raster_application=dict(source=app['source'],terminal=True,actual_exit=1,totals=app['totals'],
        failed_cases=12,failure_groups=failure_groups,all_failed_cases_deterministic=True,
        all_failed_cases_geometry_and_callback_checks_passed=True,chromium_pixel_qualification=False,
        source_unapplied=True,whole_owner_terminal_at_observation=raster_owner['all_commands_terminal'],
        whole_owner_state_at_observation=raster_owner['state']),
    artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for a in artifacts:assert sha(ROOT/a['path'])==a['sha256']
print(json.dumps(dict(index=str(INDEX),sha256=sha(INDEX),artifacts=len(artifacts),bytes=report['artifact_bytes'],release_qualification=False)),flush=True)
