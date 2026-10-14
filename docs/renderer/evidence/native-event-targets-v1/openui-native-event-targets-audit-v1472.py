"""Audit completed native Rust event behavior without compiling or capturing."""
import ast
import hashlib
import json
import subprocess
from pathlib import Path
from PIL import Image

ROOT = Path('/home/nero/code/open-ui')
RAW = ROOT / 'out/renderer-evidence/native-viewport-scroll-v1'
SOURCE = '1c8540e9f8c6eeb8fce10a76cb9a32c2f42f01fd'
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
load = lambda p: json.loads(p.read_bytes())
owner_path = RAW/'native-event-targets-pipeline-v1457/receipt.json'
build_path = RAW/'native-event-targets-clean-v1456/build.json'
guards_path = RAW/'native-event-targets-guards-v1456/receipt.json'
consumer_path = RAW/'native-event-targets-consumer-v1456/receipt.json'
owner,build,guards,consumer = [load(p) for p in [owner_path,build_path,guards_path,consumer_path]]
source = build['source']
assert source['clean'] and source['commit']==SOURCE
for row in [build,guards,consumer,owner]:
    assert row['source']==row['source_after']==source
for name in ['guards','native-build','native-application']:
    assert next(s for s in owner['steps'] if s['name']==name)['observed_exit_code']==0
assert build['all_commands_terminal'] and len(build['steps'])==7
assert all(s['observed_exit_code']==0 for s in build['steps'])
assert build['workspace_packages_cleaned']==18
for row in build['steps']:
    assert sha(build_path.parent/(row['name']+'.log'))==row['log_sha256']
    if 'binary' in row:
        assert sha(Path(row['binary']))==row['binary_sha256']
workspace = next(s for s in build['steps'] if s['name']=='workspace')
assert {k:workspace[k] for k in ['passed','failed','ignored']}==dict(passed=8536,failed=0,ignored=13)
assert 'C ABI verified: symbols=113 C_examples=11 C++=5 ran=True' in (build_path.parent/'ffi-consumers.log').read_text()
assert guards['all_commands_terminal'] and guards['baseline_regression_reproduced']
assert [s['observed_exit_code'] for s in guards['steps']]==[0,101,0,0]
for row in guards['steps']:
    assert sha(guards_path.parent/(row['name']+'.log'))==row['log_sha256']
baseline = (guards_path.parent/'baseline-event-phase-guard.log').read_text()
assert 'document::tests::rust_event_dispatch_clears_phase_after_callbacks_return ... FAILED' in baseline
assert '0 passed; 1 failed;' in baseline and 'event phase must clear after dispatch' in baseline
assert len(guards['selected_native_test_names'])==5
assert guards['steps'][-1]['observed_named_tests']==guards['selected_native_test_names']
assert guards['steps'][-1]['test_counts']==[[5,0,0]]
assert consumer['all_commands_terminal'] and consumer['observed_exit_code']==0
assert consumer['pixel_tolerance']==0 and not consumer['unstable_reference_captures']
assert consumer['public_native_rust_api'] and not consumer['javascript_executed_by_openui']
expected = dict(images=10,pixel_exact=10,geometry_exact=10,native_runs=10,deterministic_native_pairs=5,
    independent_chromium_capture_processes=20,consecutive_chromium_captures=40)
assert consumer['totals']==expected
assert consumer['strict_chromium_capture_pairs'] and consumer['capture_conditions_unchanged']
assert consumer['preserves_both_unstable_reference_captures']
assert sha(build_path)==consumer['build_receipt_sha256']
assert sha(build_path.parent/'native_event_targets')==consumer['native_binary_sha256']
assert sha(ROOT/'chrome/linux-147.0.7727.50/chrome-linux64/chrome')==consumer['chromium_binary_sha256']
assert sha(ROOT/'tools/accountability/run_all_pixel_comparisons.py')==consumer['capture_harness_sha256']
reference_path=Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py')
text=reference_path.read_text(); node=next(n for n in ast.parse(text).body if isinstance(n,ast.FunctionDef) and n.name=='reference')
original=ast.get_source_segment(text,node)
removed="        assert query['natural'] == dict(width=200, height=200)\n"
assert original.count(removed)==1
assert sha(reference_path)==consumer['reference_source_sha256']
assert hashlib.sha256(original.encode()).hexdigest()==consumer['original_reference_function_sha256']
assert hashlib.sha256(original.replace(removed,'').encode()).hexdigest()==consumer['adapted_reference_function_sha256']
assert consumer['reference_adaptations']==['Remove only the image natural-size assertion for a div case']
for value in consumer['inputs'].values():assert sha(Path(value['path']))==value['sha256']
assert [r['scale'] for r in consumer['cases']]==[1,1.25,1.5,2,3]
images=[]; file_hashes={}; retained_capture_count=0
for row in consumer['cases']:
    scale=row['scale']; directory=consumer_path.parent/str(float(scale))
    assert len(row['native_runs'])==2 and len(row['images'])==2
    contracts=[]
    for repeat,run in enumerate(row['native_runs'],1):
        assert run['observed_exit_code']==0 and sha(directory/f'native-{repeat}.log')==run['log_sha256']
        contract_path=directory/f'native-{repeat}/events.json'; contract=load(contract_path);contracts.append(contract)
        file_hashes[str(contract_path)]=sha(contract_path)
    assert contracts[0]==contracts[1]==row['native_contract']
    contract=contracts[0]
    assert contract['callbacks']==3 and contract['scale']==scale
    assert contract['bounds']==dict(x=20,y=20,width=40,height=40)
    assert all(contract[k] for k in ['owned_bounds_unchanged','phase_cleared','targets_expired_after_teardown'])
    for item in row['images']:
        state=item['state']; native=directory/f'native-1/{state}.png'; native_second=directory/f'native-2/{state}.png'
        assert native.read_bytes()==native_second.read_bytes() and sha(native)==item['native_png_sha256']
        file_hashes[str(native)]=sha(native); file_hashes[str(native_second)]=sha(native_second)
        assert item['geometry_exact'] and item['analysis']['mismatched_pixels']==0
        assert len(item['independent_reference_runs'])==2
        refs=[]
        for repeat,observation in enumerate(item['independent_reference_runs'],1):
            assert observation['repeated_capture_pair_identical'] and observation['capture_count']==2
            query=observation['query']
            assert query['bounds']==contract['bounds'] and query['metrics']==dict(width=160,height=100,dpr=scale)
            if state=='after':
                assert query['trace']==[dict(name='root',phase=1),dict(name='target',phase=2),dict(name='root',phase=3)]
                assert query['phase_after_dispatch']==[0]*3
                assert query['current_target_after_dispatch']==[True]*3
                assert query['target_after_dispatch']==[True]*3
            else:assert query['trace']==[]
            ref=directory/f'{state}-chromium-{repeat}.png'; refs.append(ref)
            first=ref.with_name(ref.stem+'-first-capture.png');second=ref.with_name(ref.stem+'-second-capture.png')
            assert ref.read_bytes()==first.read_bytes()==second.read_bytes()
            retained_capture_count+=2
            for path in [ref,first,second]:file_hashes[str(path)]=sha(path)
        assert refs[0].read_bytes()==refs[1].read_bytes() and sha(refs[0])==item['chromium_png_sha256']
        with Image.open(native) as n,Image.open(refs[0]) as r:
            assert n.size==r.size==(round(160*scale),round(100*scale))
            assert n.convert('RGBA').tobytes()==r.convert('RGBA').tobytes()
            assert n.convert('RGBA').getpixel((round(30*scale),round(30*scale)))==((0,0,255,255) if state=='before' else (255,0,0,255))
        images.append(dict(scale=scale,state=state,geometry_exact=True,pixel_exact=True,
            native_png_sha256=sha(native),chromium_png_sha256=sha(refs[0])))
assert len(images)==10 and retained_capture_count==40
subprocess.run(['git','diff','--exit-code','43706e8353defcd6d1d414b45bd125501f900ba9',SOURCE,'--',
    'bindings/rust/openui-engine','bindings/rust/openui-layout','bindings/rust/openui-paint',
    'bindings/rust/openui-compositor','bindings/rust/openui-ffi','include','tools','.github'],cwd=ROOT,check=True)
example=subprocess.check_output(['git','show',SOURCE+':bindings/rust/openui/examples/native_event_targets.rs'],cwd=ROOT).decode()
assert 'use openui::prelude::*;' in example
assert all(name not in example for name in ['openui_engine::','openui_layout::','openui_paint::','openui_compositor::'])
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=True,
    whole_pipeline_terminal_at_audit=owner['all_commands_terminal'],whole_pipeline_state_at_audit=owner['state'],
    scoped_native_event_behavior_verified=True,whole_candidate_qualified=False,
    original_and_expanded_matrices_still_require_terminal_evidence=True,release_qualification=False,
    promotion_allowed=False,new_release_states_admitted=0,applied_to_umbrella=False,
    public_rust_api=['Event::target()','Event::current_target()'],renderer_and_c_abi_source_unchanged=True,
    cpu_backend_qualification_only=True,javascript_executed_by_openui=False,pixel_tolerance=0,
    named_baseline_failure_reproduced=True,named_fixed_guards_passed=5,
    workspace=dict(passed=8536,failed=0,ignored=13),c_abi=dict(exports=113,c_examples=11,cpp_examples=5,ran=True),
    native_app=expected,all_forty_consecutive_chromium_captures_rehashed=True,
    all_native_callback_state_bounds_and_teardown_contracts_rechecked=True,
    only_reference_adaptation='Remove the natural image-size assertion for a div; retain strict capture conditions and both repeated captures',
    images=images,file_hashes=file_hashes,owner_receipt_sha256_at_observation=sha(owner_path),
    build_receipt_sha256=sha(build_path),guard_receipt_sha256=sha(guards_path),consumer_receipt_sha256=sha(consumer_path),
    probe_sha256=sha(Path(__file__)),cargo_commands_run=0,screenshots_generated=0)
out=RAW/'native-event-targets-audit-v1472.json';assert not out.exists()
out.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'path':str(out),'sha256':sha(out),'native_app':expected,'workspace':report['workspace'],
    'named_guards':5,'c_abi':report['c_abi'],'whole_pipeline_terminal':owner['all_commands_terminal'],
    'release_qualification':False}),flush=True)
