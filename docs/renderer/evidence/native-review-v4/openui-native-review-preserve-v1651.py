"""Preserve terminal raster regressions, harness stops and glyph precision evidence."""
import gzip
import hashlib
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT=Path('/home/nero/code/open-ui'); RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-review-v4'; INDEX=ROOT/'docs/renderer/generated/native-review-v4.json'
NATIVE=Path('/dev/shm/openui-native-author-glyph-precision-e0dc491e')
assert not OUT.exists() and not INDEX.exists(); OUT.mkdir()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest(); artifacts=[]
def preserve(path,name,expected=None,compressed=False):
    content=path.read_bytes(); digest=hashlib.sha256(content).hexdigest()
    if expected is not None:assert digest==expected,str(path)
    target=OUT/name;assert not target.exists()
    target.write_bytes(gzip.compress(content,compresslevel=9,mtime=0) if compressed else content)
    assert sha(path)==digest
    row=dict(path=target.relative_to(ROOT).as_posix(),sha256=sha(target),bytes=target.stat().st_size,source_path=str(path))
    if compressed:
        assert gzip.decompress(target.read_bytes())==content
        row.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(content))
    else:assert sha(target)==digest
    artifacts.append(row)
    return json.loads(content) if path.suffix=='.json' else None
def generated(name,content):
    target=OUT/name;assert not target.exists();target.write_bytes(content)
    artifacts.append(dict(path=target.relative_to(ROOT).as_posix(),sha256=sha(target),bytes=len(content)))
audit=preserve(RAW/'native-raster-fields-retry-full-audit-v1648.json','raster-full-audit.json',
    '81108ad8bf4640c246e77f2973fc7f9417b66fa300da84405970831f9d24ca30')
preserve(Path('/tmp/openui-native-raster-fields-retry-full-audit-v1648.py'),'raster-full-audit.py')
assert audit['source_rejected_for_application'] and audit['suites']['full']['exact_losses']==83
assert audit['suites']['full']['exact_gains']==0 and audit['changed_additions']==4
for suite in ['full','expanded','focused','primitive']:
    row=audit['suites'][suite]
    preserve(RAW/f'native-raster-fields-retry-clean-{suite}-v1559/{suite}-summary.json',
        'raster-'+suite+'-summary.json.gz',row['summary_sha256'],compressed=True)
    preserve(RAW/f'native-raster-fields-retry-{suite}-exit-v1559.json','raster-'+suite+'-exit.json',row['actual_exit_receipt_sha256'])
owner=preserve(RAW/'native-raster-fields-retry-pipeline-v1560/receipt.json','raster-owner-terminal.json')
assert owner['all_commands_terminal'] and len(owner['steps'])==12
for name in ['full','expanded']:
    step=next(s for s in owner['steps'] if s['name']==name)
    preserve(RAW/'native-raster-fields-retry-pipeline-v1560'/(name+'.log'),'raster-'+name+'.log',step['log_sha256'])
stops=[]
for name,version in [('native-inline-fallback',1576),('native-keywords',1588),('native-table-source',1601),
                     ('native-text-content',1621),('native-text-content-viewport',1636)]:
    p=RAW/f'{name}-pipeline-v{version}/receipt.json';data=preserve(p,f'v{version}-owner-terminal.json')
    assert data['all_commands_terminal'] and len(data['steps'])==1
    preserve(p.parent/'guards.log',f'v{version}-guards.log',data['steps'][0]['log_sha256'])
    stops.append(dict(owner=name+f'-pipeline-v{version}',source=data['source']['commit'],
        state=data['state'],actual_guard_process_exit=data['steps'][0]['observed_exit_code']))
for name,version in [('native-keywords',1587),('native-table-source',1600),('native-text-content',1620)]:
    p=RAW/f'{name}-guards-v{version}/receipt.json';data=preserve(p,f'v{version}-guard-terminal.json')
    assert data['all_commands_terminal']
    for row in data['steps']:
        preserve(p.parent/(row['name']+'.log'),f'v{version}-'+row['name']+'.log',row['log_sha256'])
assert not (RAW/'native-inline-fallback-guards-v1575/receipt.json').exists()
assert not (RAW/'native-text-content-viewport-guards-v1635/receipt.json').exists()
for p in ['/tmp/openui-native-inline-fallback-guards-v1575.py','/tmp/openui-native-text-content-viewport-guards-v1635.py']:
    preserve(Path(p),Path(p).name)
position=preserve(RAW/'native-font-position-audit-v1643.json','font-position-audit.json',
    '576ed803a62c3acd9a2cf408846349727749c934f6662d2b3151531b7b135374')
preserve(Path('/tmp/openui-native-font-position-audit-v1643.py'),'font-position-audit.py')
for number,(path,digest) in enumerate(position['input_file_sha256'].items()):
    preserve(Path(path),f'position-source-{number:02d}-'+Path(path).name,digest)
sys.path.insert(0,str(NATIVE/'tools/qualification'))
from renderer_source_identity import repository_source_identity
source=repository_source_identity(NATIVE)
assert source['clean'] and source['commit']=='3b2e0d1f60b90813859c4c24325c7b0061dea29c'
checks_path=RAW/'native-author-glyph-precision-checks-v1647/receipt.json'
checks=preserve(checks_path,'precision-read-only-checks.json')
assert checks['source']==checks['source_after']==source and checks['all_commands_terminal']
assert len(checks['checks'])==11 and all(row['observed_exit_code']==0 for row in checks['checks'])
for row in checks['checks']:preserve(checks_path.parent/(row['name']+'.log'),'precision-check-'+row['name']+'.log',row['log_sha256'])
preserve(Path('/tmp/openui-native-author-glyph-precision-checks-v1647.py'),'precision-read-only-checks.py')
for name,start,end in [('precision-regression.patch',owner['source']['commit'],'347d901c8de7b5a7890031ac28e58b14cd061275'),
                       ('authored-glyph-precision.patch','347d901c8de7b5a7890031ac28e58b14cd061275',source['commit'])]:
    generated(name,subprocess.check_output(['git','diff','--binary',start,end],cwd=NATIVE))
hosted_path=RAW/'native-text-content-viewport-hosted-v1638/receipt.json';hosted=preserve(hosted_path,'text-hosted-terminal.json')
assert hosted['all_commands_terminal'] and hosted['all_seven_jobs_passed'] and hosted['actual_exit']==0
assert hosted['job_conclusions']==dict(success=7)
run=str(hosted['run_id']);preserve(hosted_path.parent/(run+'.json'),'text-hosted-detail.json',hosted['detail_sha256'])
preserve(hosted_path.parent/(run+'.log'),'text-hosted.log.gz',hosted['log_sha256'],compressed=True)
ci_path=RAW/'native-text-content-umbrella-ci-v1641/receipt.json';ci=preserve(ci_path,'preceding-umbrella-hosted.json')
assert ci['all_three_hosted_workflows_complete_success'] and ci['job_conclusions']==dict(success=6,skipped=5)
for row in ci['workflows']:
    run=str(row['databaseId']);preserve(ci_path.parent/(run+'.json'),'umbrella-'+run+'.json')
    preserve(ci_path.parent/(run+'.log'),'umbrella-'+run+'.log.gz',row['captured_log_sha256'],compressed=True)
queue_path=RAW/'native-text-retry-queue-prepared-v1649.json';queue=preserve(queue_path,'text-retry-prepared.json',
    'cc17a8779c2114c8c126f6695604907b66796ebf856d40409f7594d2d4803613')
for path,digest in queue['scripts'].items():preserve(Path(path),Path(path).name,digest)
preserve(Path('/tmp/openui-native-text-retry-prepare-v1649.py'),'text-retry-prepare.py')
retry=preserve(RAW/'native-text-retry-pipeline-v1650/receipt.json','text-retry-owner-at-observation.json')
assert retry['source']['commit']==hosted['source']
relocation=preserve(RAW/'retired-artifact-relocation-v1646.json','retired-artifact-relocation.json',
    'e03698c094468ecdd937a4663cb6b9a4685fbe50889d9aef4cde2e144d865d14')
for version in [1645,1646]:preserve(Path('/tmp')/f'openui-retired-artifact-relocation-v{version}.py',f'relocation-v{version}.py')
generated('relocation-preflight-note.json',(json.dumps(dict(schema_version=1,failed_preflight_version=1645,
    actual_exit=1,reason='old artifacts have 3, 5, 5 and 4 declared build stages; the old template expected six',
    files_moved_by_failed_preflight=0,fresh_probe_version=1646,original_bytes_preserved=True),sort_keys=True,indent=2)+'\n').encode())
assert repository_source_identity(NATIVE)==source
preserve(Path(__file__),Path(__file__).name)
report=dict(schema_version=1,observed_at_utc=datetime.now(timezone.utc).isoformat(),
    accepted_original_exact=21334,accepted_expanded_exact=22137,accepted_renderer_unchanged=True,
    trial_source=owner['source'],trial_suites=audit['suites'],trial_source_rejected_for_application=True,
    trial_addition_cases_four_profile_exact=199,accepted_addition_cases_four_profile_exact=200,
    all_original_and_expanded_chromium_inputs_unchanged=True,terminal_followups=stops,
    harness_failure_causes=dict(fallback='nonexistent restore branch; no baseline or fixed tests execute',
        keywords='named baseline fails as expected; three fixed commands stop at disk guard',
        table_source='baseline clean stops at disk guard before tests',
        original_text='baseline clean stops at disk guard before tests',
        corrected_text='duplicated viewport prefix points at missing source root; no Cargo stage executes'),
    source_supported_glyph_precision=dict(source=source,baseline='347d901c8de7b5a7890031ac28e58b14cd061275',
        parent_raster_trial_has_83_exact_losses=True,applied_to_umbrella=False,read_only_checks_passed=11,
        source_supported_images=3,modeled_images=10,geometry_states=640,edge_checks_exact=12,
        all_modeled_phase_predictions_match=True,actual_runtime_advance_still_needs_verification=True,
        new_guard_and_raster_fix_not_executed=True,native_control_positioning_unchanged=True,
        all_other_rust_font_differences_still_require_review=True,formal_wpt_residual_ownership_unchanged=True),
    text_hosted=dict(source=hosted['source'],run=hosted['run_id'],successful_jobs=7,skips=0,actual_exit=0),
    text_retry=dict(source=retry['source'],owner='native-text-retry-pipeline-v1650',observed_state=retry['state'],
        terminal_at_observation=retry['all_commands_terminal'],fresh_paths_and_branch_preflight_verified=True,
        required_native_images=600,required_geometry_states=38400,required_stages=7,prior_owners=34,
        local_native_and_pixel_gates_still_required=True),
    preceding_umbrella_hosted=dict(source=ci['commit'],successful_workflows=3,successful_jobs=6,skips=5,skips_are_not_passes=True),
    retired_artifact_bytes_preserved=sum(row['bytes'] for row in relocation['artifacts']),
    pixel_tolerance=0,javascript_executed_by_openui=False,public_native_rust_apis_required=True,
    release_qualification=False,new_release_states_admitted=0,large_logs_and_reports_losslessly_compressed=True,
    artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for row in artifacts:
    p=ROOT/row['path'];assert sha(p)==row['sha256']
    if row.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress(p.read_bytes())).hexdigest()==row['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX),sha256=sha(INDEX),artifacts=len(artifacts),stored_bytes=report['artifact_bytes'],
                     release_qualification=False)),flush=True)
