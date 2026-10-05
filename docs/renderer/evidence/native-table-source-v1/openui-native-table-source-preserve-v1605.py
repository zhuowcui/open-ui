"""Preserve source-retention evidence, with byte-exact compressed hosted logs."""
import gzip,hashlib,json,shutil,subprocess,sys
from datetime import datetime,timezone
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
NATIVE=Path('/dev/shm/openui-native-table-source-1d846e68')
OUT=ROOT/'docs/renderer/evidence/native-table-source-v1';INDEX=ROOT/'docs/renderer/generated/native-table-source-v1.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir()
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest();artifacts=[]
def preserve(path,name,expected=None):
    digest=sha(path)
    if expected is not None:assert digest==expected,str(path)
    target=OUT/name;assert not target.exists();shutil.copyfile(path,target)
    assert sha(path)==sha(target)==digest
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=digest,bytes=target.stat().st_size))
def compressed_log(path,name,expected):
    content=path.read_bytes();assert hashlib.sha256(content).hexdigest()==expected
    target=OUT/name;assert not target.exists();target.write_bytes(gzip.compress(content,compresslevel=9,mtime=0))
    assert gzip.decompress(target.read_bytes())==content and sha(path)==expected
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=sha(target),bytes=target.stat().st_size,
        encoding='gzip',decompressed_sha256=expected,decompressed_bytes=len(content),source_path=str(path)))
def snapshot(path,name):
    content=path.read_bytes();target=OUT/name;assert not target.exists();target.write_bytes(content)
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=sha(target),bytes=len(content)))
    return json.loads(content)
queue_path=RAW/'native-table-source-queue-prepared-v1600.json';queue=json.loads(queue_path.read_bytes());source=queue['source']
sys.path.insert(0,str(NATIVE/'tools/qualification'))
from renderer_source_identity import repository_source_identity
assert source==repository_source_identity(NATIVE) and source['commit']=='e389b26ab28f259544e43c3052c895501e4c8686'
preserve(queue_path,'queue-prepared.json')
for p,digest in queue['scripts'].items():preserve(Path(p),Path(p).name,digest)
for version,name in [(1598,'checks'),(1600,'prepare'),(1602,'dispatch'),(1603,'hosted')]:
    path=Path('/tmp')/f'openui-native-table-source-{name}-v{version}.py';preserve(path,path.name)
preserve(Path('/tmp/openui-native-table-source-dispatch-v1602.json'),'hosted-dispatch.json')
checks_path=RAW/'native-table-source-checks-v1598/receipt.json';checks=json.loads(checks_path.read_bytes())
assert checks['source']==checks['source_after']==source and checks['all_commands_terminal']
assert len(checks['checks'])==11 and all(r['observed_exit_code']==0 for r in checks['checks'])
preserve(checks_path,'read-only-checks.json',queue['checks_receipt_sha256'])
for row in checks['checks']:preserve(checks_path.parent/(row['name']+'.log'),'check-'+row['name']+'.log',row['log_sha256'])
for name,start,end in [('prior-progress-and-guards.patch','1d846e68b2b31b4091824b07ce3b54377f0409ca',queue['baseline_commit']),('canonical-table-source.patch',queue['baseline_commit'],source['commit'])]:
    path=OUT/name;path.write_bytes(subprocess.check_output(['git','diff','--binary',start,end],cwd=NATIVE))
    artifacts.append(dict(path=path.relative_to(ROOT).as_posix(),sha256=sha(path),bytes=path.stat().st_size))
preserve(NATIVE/'bindings/rust/openui/examples/native_table_progress.rs','native_table_progress.rs')
preserve(NATIVE/'docs/v02/generated/openui-ffi-layout.json','openui-ffi-layout.json')
for version in [1595,1596]:
    path=Path('/tmp')/f'openui-native-table-fragment-debug-v{version}.py';preserve(path,path.name)
    receipt=RAW/f'native-table-fragment-debug-v{version}/receipt.json';data=json.loads(receipt.read_bytes())
    assert data['all_commands_terminal'];preserve(receipt,f'debug-v{version}.json')
    if version==1595:
        assert data['observed_exit_code']==1 and data['runs']==[]
        preserve(receipt.parent/'legacy-1.log','failed-debug-cli.log')
    else:
        assert data['observed_exit_code']==0 and data['independent_debug_runs_identical'] and data['observations']==8
        for run in data['runs']:
            for row in run['rows']:preserve(Path(row['log']),f"debug-{row['profile']}-{run['repeat']}.log",row['log_sha256'])
projection_path=RAW/'native-table-source-data-flow-v1599.json';projection=json.loads(projection_path.read_bytes())
assert projection['fixed_geometry_not_executed'] and projection['debug_fixture_is_different_from_native_engine_guard']
assert len(projection['projections'])==4 and all(p['cropped_first_body_row_css_height']==60 and p['source_body_css_height']==100 for p in projection['projections'])
preserve(projection_path,'data-flow-projection.json')
audit_path=RAW/'native-table-source-data-flow-audit-v1604.json';audit=json.loads(audit_path.read_bytes())
assert audit['all_commands_terminal'] and audit['observed_exit_code']==0 and audit['projection_receipt_sha256']==sha(projection_path)
preserve(audit_path,'data-flow-audit.json')
preserve(Path('/tmp/openui-native-table-source-data-flow-audit-v1604.py'),'openui-native-table-source-data-flow-audit-v1604.py')
owner=snapshot(RAW/'native-table-source-pipeline-v1601/receipt.json','owner-at-observation.json')
assert owner['source']==source and owner['all_commands_terminal'] is False and owner['steps']==[]
hosted=snapshot(RAW/'native-table-source-hosted-v1603/receipt.json','hosted-at-observation.json')
assert hosted['source']==source['commit'] and hosted['run_id']==37344385577
ci_path=RAW/'native-keywords-umbrella-ci-v1593/receipt.json';ci=json.loads(ci_path.read_bytes())
assert ci['commit']=='1d846e68b2b31b4091824b07ce3b54377f0409ca' and ci['all_three_hosted_workflows_complete_success']
assert ci['job_conclusions']==dict(success=6,skipped=5)
preserve(ci_path,'umbrella-hosted.json')
for row in ci['workflows']:
    run=str(row['databaseId']);preserve(ci_path.parent/(run+'.json'),'umbrella-'+run+'.json')
    compressed_log(ci_path.parent/(run+'.log'),'umbrella-'+run+'.log.gz',row['captured_log_sha256'])
preserve(Path('/tmp/openui-native-keywords-umbrella-ci-v1593.py'),'openui-native-keywords-umbrella-ci-v1593.py')
keyword_path=RAW/'native-keywords-hosted-v1590/receipt.json';keyword=json.loads(keyword_path.read_bytes())
assert keyword['all_commands_terminal'] and keyword['actual_exit']==0 and keyword['all_seven_jobs_passed']
assert keyword['source']=='06e1f89a4a2e7a53465bceb780675383d9748464' and keyword['job_conclusions']==dict(success=7)
preserve(keyword_path,'native-keywords-hosted.json');run=str(keyword['run_id'])
preserve(keyword_path.parent/(run+'.json'),'native-keywords-'+run+'.json',keyword['detail_sha256'])
compressed_log(keyword_path.parent/(run+'.log'),'native-keywords-'+run+'.log.gz',keyword['log_sha256'])
preserve(Path('/tmp/openui-native-keywords-hosted-v1590.py'),'openui-native-keywords-hosted-v1590.py')
main_checks_path=RAW/'native-keywords-umbrella-checks-v1592/receipt.json';main_checks=json.loads(main_checks_path.read_bytes())
assert main_checks['source']==main_checks['source_after'] and main_checks['source']['clean'] and main_checks['source']['commit']==ci['commit']
assert main_checks['all_commands_terminal'] and len(main_checks['checks'])==10 and all(r['observed_exit_code']==0 for r in main_checks['checks'])
preserve(main_checks_path,'prior-umbrella-read-only-checks.json')
for row in main_checks['checks']:preserve(main_checks_path.parent/(row['name']+'.log'),'prior-umbrella-check-'+row['name']+'.log',row['log_sha256'])
preserve(Path('/tmp/openui-native-keywords-umbrella-checks-v1592.py'),'openui-native-keywords-umbrella-checks-v1592.py')
for p,digest in queue['scripts'].items():assert sha(Path(p))==digest
assert repository_source_identity(NATIVE)==source
preserve(Path(__file__),Path(__file__).name)
report=dict(schema_version=1,observed_at_utc=datetime.now(timezone.utc).isoformat(),source=source,
    baseline_commit=queue['baseline_commit'],branch=queue['branch'],applied_to_umbrella=False,
    accepted_renderer_unchanged=True,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,
    pixel_tolerance=0,javascript_executed_by_openui=False,public_native_rust_api_required=True,
    needed_native_operations_not_waived_by_pixel_exclusion=True,old_openui_pixels_are_provenance_only=True,
    root_cause_owner='openui-layout canonical repeated table source and ancestor continuation geometry',
    measured_data_flow=dict(debug_source=projection['source'],independent_debug_runs=2,profiles=4,observations=8,
        source_table_height_css_px=140,header_and_footer_height_css_px=40,cropped_first_body_row_height_css_px=60,
        retained_body_height_css_px=100,unrepresented_remaining_body_css_px=40,
        debug_fixture_table_fragments=5,separate_failed_native_guard_table_fragments=4,
        original_required_chromium_count=41,different_sources_and_fixtures_not_substituted=True,
        source_projection_recomputed_identically=True,screenshots_generated=0),
    prepared_source=dict(shared_immutable_full_table_subtree=True,one_source_per_table_required=True,
        source_release_after_teardown_required=True,existing_expected_geometry_unchanged=True,
        public_rust_callback_application_preserved=True,current_exports=113,all_30_c_layouts_unchanged=True,
        read_only_checks_passed=11,whole_owner='native-table-source-pipeline-v1601',observed_state=owner['state'],
        prior_whole_owners=31,required_stages=7,required_native_images=60,
        required_independent_chromium_processes=120,required_consecutive_chromium_captures=240,
        native_geometry_and_pixel_stages_not_started=True),
    private_hosted_at_observation=dict(source=source['commit'],run_id=37344385577,
        terminal=hosted['all_commands_terminal'],state=hosted['state'],native_pixel_qualification=False),
    keyword_hosted=dict(source=keyword['source'],successful_jobs=7,skipped_jobs=0,actual_exit=0,
        local_native_and_pixel_stages_still_required=True),
    umbrella_hosted=dict(source=ci['commit'],successful_workflows=3,successful_jobs=6,skipped_jobs=5,skips_are_not_passes=True),
    hosted_log_storage=dict(encoding='gzip',all_captured_bytes_preserved=True,all_decompressed_sha256_verified=True,
        source_logs_unmodified=True,compressed_logs=sum(a.get('encoding')=='gzip' for a in artifacts)),
    artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts),
    decompressed_hosted_log_bytes=sum(a.get('decompressed_bytes',0) for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for a in artifacts:
    p=ROOT/a['path'];assert sha(p)==a['sha256']
    if a.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress(p.read_bytes())).hexdigest()==a['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX),sha256=sha(INDEX),artifacts=len(artifacts),stored_bytes=report['artifact_bytes'],decompressed_hosted_log_bytes=report['decompressed_hosted_log_bytes'],release_qualification=False)),flush=True)
