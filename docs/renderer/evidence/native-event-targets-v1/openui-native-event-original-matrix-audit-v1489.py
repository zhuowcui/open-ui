"""Check the finished event checkpoint matrices without raster work."""
import hashlib,json,sys
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
sys.path.insert(0,str(ROOT/'tools/qualification'))
from residuals import canonical_sha256
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ownerpath=RAW/'native-event-targets-pipeline-v1457/receipt.json';ownerbytes=ownerpath.read_bytes();owner=json.loads(ownerbytes)
buildpath=RAW/'native-event-targets-clean-v1456/build.json';build=json.loads(buildpath.read_bytes());source=build['source']
assert source['clean'] and source['commit']=='1c8540e9f8c6eeb8fce10a76cb9a32c2f42f01fd'
assert source==build['source_after']==owner['source']==owner['source_after']
binary=next(s for s in build['steps'] if s['name']=='pixel-build')
fields=['openui_png_sha256','openui_rgba_sha256','chromium_png_sha256','chromium_rgba_sha256',
    'chromium_oracle_identity_sha256','chromium_oracle_rgba_sha256','status','mismatched_pixels','diff_signature']
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=True,
    whole_pipeline_terminal_at_audit=owner['all_commands_terminal'],whole_pipeline_state_at_audit=owner['state'],
    release_qualification=False,promotion_allowed=False,whole_candidate_qualified=False,new_release_states_admitted=0,
    original_and_expanded_matrices_require_separate_terminal_evidence=True,suites={},
    owner_receipt_sha256_at_observation=hashlib.sha256(ownerbytes).hexdigest(),build_receipt_sha256=sha(buildpath),
    probe_sha256=sha(Path(__file__)),cargo_commands_run=0,screenshots_generated=0)
for suite,total in [('full',22924)]:
    stage=next(s for s in owner['steps'] if s['name']==suite);assert stage['observed_exit_code']==1
    currentpath=RAW/f'native-event-targets-clean-{suite}-v1456/{suite}-summary.json'
    oldpath=RAW/f'umbrella-clean-workspace-clean-{suite}-v1410/{suite}-summary.json'
    current=json.loads(currentpath.read_bytes());old=json.loads(oldpath.read_bytes())
    exitpath=RAW/f'native-event-targets-{suite}-exit-v1456.json';observed=json.loads(exitpath.read_bytes())
    assert observed['observed_exit_code']==1
    assert sha(RAW/f'native-event-targets-clean-{suite}-v1456.log')==observed['log_sha256']
    assert current['source']==current['source_after']==source==current['openui']['build_identity']['source']
    assert current['openui']['binary_sha256']==binary['binary_sha256']
    assert current['suite']==old['suite']==suite
    assert current['complete_contract_scope'] and not current['evidence']['qualified']
    assert current['id_manifest']['count']==5731 and current['id_manifest']['sha256']==old['id_manifest']['sha256']
    assert sha(Path(current['id_manifest']['path']))==current['id_manifest']['sha256']
    assert current['evidence']['source_unchanged'] and current['evidence']['binary_unchanged']
    assert current['evidence']['tolerance_pixels']==0
    assert current['raster']['backend_selection']=='explicit-immutable'
    assert current['openui']['raster_backend_identity']['backend']=='cpu-skia'
    for key in ['chromium','font_byte_hashes','resource_hashes','raster','contract_sha256','manifest_scope']:
        assert current[key]==old[key],key
    assert len(current['profiles'])==len(old['profiles'])==4
    count=0
    for previous,profile in zip(old['profiles'],current['profiles'],strict=True):
        for key in ['profile','device_scale','logical_size_css_px','physical_size_px','ordered_id_sha256']:
            assert previous[key]==profile[key],key
        assert canonical_sha256(profile['tests'])==profile['result_sha256']
        for before,row in zip(previous['tests'],profile['tests'],strict=True):
            assert before['id']==row['id']
            assert all(before.get(field)==row.get(field) for field in fields)
            count+=1
    assert count==total
    assert {key:current['results'][key] for key in ['total','exact','different','errors']}==dict(total=total,exact=21334,different=1590,errors=0)
    report['suites'][suite]=dict(total=total,exact=21334,different=1590,errors=0,actual_exit=1,profiles=4,
        all_nine_comparison_invariants_unchanged=total,summary_sha256=sha(currentpath),
        prior_summary_sha256=sha(oldpath),exit_receipt_sha256=sha(exitpath))
out=RAW/'native-event-original-matrix-audit-v1489.json';assert not out.exists()
out.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'path':str(out),'sha256':sha(out),'suites':report['suites'],'release_qualification':False}),flush=True)
