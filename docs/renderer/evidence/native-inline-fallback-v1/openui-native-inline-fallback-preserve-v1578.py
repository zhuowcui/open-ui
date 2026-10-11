"""Preserve a prepared native correction without claiming pending execution."""
import hashlib, json, shutil, subprocess, sys
from datetime import datetime, timezone
from pathlib import Path

ROOT=Path('/home/nero/code/open-ui')
NATIVE=Path('/dev/shm/openui-native-inline-fallback-retry-7d09f7a1')
RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-inline-fallback-v1'
INDEX=ROOT/'docs/renderer/generated/native-inline-fallback-v1.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir()
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
artifacts=[]
def preserve(path,name,expected=None):
    before=sha(path)
    if expected is not None:assert before==expected,str(path)
    target=OUT/name;assert not target.exists();shutil.copyfile(path,target)
    assert sha(target)==sha(path)==before
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=before,bytes=target.stat().st_size))

queue_path=RAW/'native-inline-fallback-queue-prepared-v1575.json'
queue=json.loads(queue_path.read_bytes());source=queue['source']
sys.path.insert(0,str(NATIVE/'tools/qualification'))
from renderer_source_identity import repository_source_identity
assert repository_source_identity(NATIVE)==source and source['clean']
assert source['commit']=='7d09f7a159b2c3f211662d20dbf7c7092d6024ed'
preserve(queue_path,'queue-prepared.json')
for p,digest in queue['scripts'].items():preserve(Path(p),Path(p).name,digest)
for name in ['checks-v1569','geometry-v1570','retry-prepare-v1575']:
    path=Path('/tmp/openui-native-inline-fallback-'+name+'.py');preserve(path,path.name)
checks_path=RAW/'native-inline-fallback-checks-v1569/receipt.json'
checks=json.loads(checks_path.read_bytes());assert checks['all_commands_terminal']
assert checks['source']==checks['source_after']==source
assert len(checks['checks'])==13 and all(r['observed_exit_code']==0 for r in checks['checks'])
preserve(checks_path,'read-only-checks.json',queue['checks_receipt_sha256'])
for row in checks['checks']:preserve(checks_path.parent/(row['name']+'.log'),'check-'+row['name']+'.log',row['log_sha256'])
geometry_path=RAW/'native-inline-fallback-geometry-v1570/receipt.json'
geometry=json.loads(geometry_path.read_bytes());assert geometry['all_commands_terminal'] and geometry['observed_exit_code']==0
assert geometry['independent_queries_identical'] and geometry['observations']==120
preserve(geometry_path,'chromium-geometry.json',queue['geometry_receipt_sha256'])
for row in geometry['inputs']:preserve(Path(row['path']),Path(row['path']).name,row['sha256'])
for rel in ['bindings/rust/openui/examples/native_inline_fallback.rs','bindings/rust/openui/tests/assets/green-200.png','examples/c_v02/inline_replaced.c','examples/c_v02/inline_replaced.cc']:
    preserve(NATIVE/rel,Path(rel).name)
for name,base in [('native-api-and-guards.patch','59cac2298d4c6a391b7032091a0455f639f2b70a'),('fallback-renderer.patch',queue['baseline_commit'])]:
    path=OUT/name;path.write_bytes(subprocess.check_output(['git','diff','--binary',base,source['commit']],cwd=NATIVE))
    artifacts.append(dict(path=path.relative_to(ROOT).as_posix(),sha256=sha(path),bytes=path.stat().st_size))
owner_path=RAW/'native-inline-fallback-pipeline-v1576/receipt.json'
owner_bytes=owner_path.read_bytes();owner=json.loads(owner_bytes)
assert not owner['all_commands_terminal'] and owner['source']==source
path=OUT/'owner-at-observation.json';path.write_bytes(owner_bytes)
artifacts.append(dict(path=path.relative_to(ROOT).as_posix(),sha256=sha(path),bytes=path.stat().st_size))
assert owner['c_abi_append_only'] and not owner['c_abi_unchanged']
old_path=RAW/'native-inline-fallback-queue-prepared-v1571.json'
preserve(old_path,'superseded-prelaunch-queue.json')
old=json.loads(old_path.read_bytes())
for p,digest in old['scripts'].items():preserve(Path(p),'superseded-'+Path(p).name,digest)
path=Path('/tmp/openui-native-inline-fallback-prepare-v1571.py');preserve(path,'superseded-'+path.name)

ci_path=RAW/'native-review-umbrella-ci-v1565/receipt.json'
ci=json.loads(ci_path.read_bytes());assert ci['all_three_hosted_workflows_complete_success']
assert ci['commit']=='59cac2298d4c6a391b7032091a0455f639f2b70a' and ci['job_conclusions']==dict(success=6,skipped=5)
preserve(ci_path,'umbrella-hosted.json')
for row in ci['workflows']:
    run=str(row['databaseId']);preserve(ci_path.parent/(run+'.json'),'umbrella-'+run+'.json')
    preserve(ci_path.parent/(run+'.log'),'umbrella-'+run+'.log',row['captured_log_sha256'])

build_path=RAW/'native-raster-fields-retry-clean-v1559/build.json'
build=json.loads(build_path.read_bytes())
assert build['all_commands_terminal'] and build['source']==build['source_after'] and build['source']['clean']
assert len(build['steps'])==17 and all(r['observed_exit_code']==0 for r in build['steps'])
preserve(build_path,'raster-retry-build.json')
for row in build['steps']:preserve(build_path.parent/(row['name']+'.log'),'raster-build-'+row['name']+'.log',row['log_sha256'])
controls_path=RAW/'native-raster-fields-retry-controls-v1559/receipt.json'
controls=json.loads(controls_path.read_bytes());assert controls['all_commands_terminal'] and controls['observed_exit_code']==0
assert controls['source']==controls['source_after']==build['source'] and controls['chromium_pixel_qualification'] is False
preserve(controls_path,'raster-native-controls.json')
path=RAW/'native-raster-fields-retry-pipeline-v1560/controls.log';preserve(path,'raster-controls-owner.log','ff63cb5651e86c72d10da071e935192f05311fe32f7d74d49d2a5ad5e5b30002')
preserve(Path(__file__),Path(__file__).name)
report=dict(schema_version=1,observed_at_utc=datetime.now(timezone.utc).isoformat(),
    source=source,baseline_commit=queue['baseline_commit'],branch='agent/native-inline-fallback-retry-v1573',
    applied_to_umbrella=False,accepted_renderer_unchanged=True,release_qualification=False,
    promotion_allowed=False,new_release_states_admitted=0,pixel_tolerance=0,
    javascript_executed_by_openui=False,public_native_rust_api_required=True,
    needed_native_operations_not_waived_by_pixel_exclusion=True,old_openui_pixels_are_provenance_only=True,
    root_cause_owner='openui-layout inline replaced versus fallback flow and native image resource lifecycle',
    prepared_rust_api='Element::clear_image_resource',prepared_c_api='oui_element_clear_image',
    existing_113_exports_preserved=True,current_candidate_exports=114,all_30_struct_layouts_unchanged=True,
    frozen_fixtures_and_manifest_unchanged=True,read_only_checks_passed=13,
    chromium_geometry=dict(independent_runs=2,ordered_observations=120,all_repeated_queries_identical=True,
        scales=[1,1.25,1.5,2,3],origins=[0,0.25,0.5],ordered_states=['before','loaded','cleared','reattached'],
        initial_and_cleared_bounds_css_px=[20,20,100,100],loaded_bounds_css_px=[20,20,300,200],screenshots_generated=0),
    native_and_pixel_qualification=dict(all_commands_pending=True,whole_owner='native-inline-fallback-pipeline-v1576',
        observed_state=owner['state'],prior_whole_owners=29,required_stages=8,required_native_images=120,
        required_independent_chromium_processes=240,required_consecutive_chromium_captures=480),
    superseded_prelaunch_queue=dict(path='superseded-prelaunch-queue.json',owner_never_launched=True,
        reason='Original copied owner metadata said C ABI unchanged despite the append-only export. Fresh source root and probes record the additive ABI and adapted reference function hash accurately.'),
    raster_retry=dict(source=build['source'],build_stage_terminal=True,build_steps_passed=17,
        controls_stage_terminal=True,native_controls=controls['totals'],chromium_pixel_qualification=False,
        whole_pipeline_still_pending=True),
    umbrella_hosted=dict(source=ci['commit'],successful_workflows=3,successful_jobs=6,skipped_jobs=5,skips_are_not_passes=True),
    artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(index=str(INDEX),sha256=sha(INDEX),artifacts=len(artifacts),bytes=report['artifact_bytes'],native_and_pixels_pending=True)),flush=True)
