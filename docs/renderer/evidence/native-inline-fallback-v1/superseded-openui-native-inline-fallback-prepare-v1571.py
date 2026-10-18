"""Prepare one exclusive verification pipeline for the shared fallback fix."""
import ast, hashlib, json, subprocess, sys
from pathlib import Path

ROOT = Path('/dev/shm/openui-native-inline-fallback-59cac229')
RAW = Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
SOURCE = '7d09f7a159b2c3f211662d20dbf7c7092d6024ed'
BASELINE = 'cd99c95278f89fb1590834fe358e93511353b106'
BRANCH = 'agent/native-inline-fallback-v1568'
sys.path.insert(0, str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha = lambda p: hashlib.file_digest(p.open('rb'), 'sha256').hexdigest()
source = repository_source_identity(ROOT)
assert source['clean'] and source['commit'] == SOURCE
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip() == SOURCE
old_path = Path('/tmp/openui-native-inline-replaced-pipeline-v1529.py')
old_text = old_path.read_text()
old = ast.literal_eval(ast.parse(old_text).body[0].value)
raster_text = Path('/tmp/openui-native-raster-fields-retry-pipeline-v1560.py').read_text()
raster = ast.literal_eval(ast.parse(raster_text).body[0].value)
priors = raster['prior_pipelines'] + [raster['name']]
assert len(priors) == len(set(priors)) == 29
checks_path = RAW/'native-inline-fallback-checks-v1569/receipt.json'
checks = json.loads(checks_path.read_bytes())
assert checks['all_commands_terminal'] and checks['source'] == checks['source_after'] == source
assert len(checks['checks']) == 13 and all(r['observed_exit_code'] == 0 for r in checks['checks'])
geometry_path = RAW/'native-inline-fallback-geometry-v1570/receipt.json'
geometry = json.loads(geometry_path.read_bytes())
assert geometry['all_commands_terminal'] and geometry['observed_exit_code'] == 0
assert geometry['independent_queries_identical'] and geometry['observations'] == 120
assert geometry['native_app_sha256'] == sha(ROOT/'bindings/rust/openui/examples/native_inline_fallback.rs')
paths = []
original_hashes = {}
def write(name, text):
    p = Path('/tmp')/name
    assert not p.exists()
    ast.parse(text)
    p.write_text(text)
    paths.append(p)
    return p
def adapted(name):
    p = Path('/tmp')/name
    original_hashes[str(p)] = sha(p)
    s = p.read_text().replace('/dev/shm/openui-native-inline-replaced-80e71181',str(ROOT))
    s = s.replace('native-inline-replaced-', 'native-inline-fallback-').replace('v1528','v1571')
    s = s.replace('c92e2d08163779f5e55784f14e556ecaa9d16e7f',SOURCE)
    s = s.replace(repr(old['prior_pipelines']),repr(priors))
    return s

guard = adapted('openui-native-inline-replaced-guards-v1528.py')
guard = guard.replace('d8742684adf1985ce6db2715778fbbfb0770e6d9',BASELINE).replace('agent/native-inline-replaced-v1524',BRANCH)
old_test = 'tests::native_inline_replaced_boxes_preserve_size_through_retained_mutations'
test = 'tests::native_inline_fallback_children_flow_across_image_resource_changes'
guard = guard.replace(old_test,test).replace('native inline replaced content must keep its authored box','fallback children must use normal inline flow')
needle = "('fixed-ordinary-inline-neighbor', 'tests::ordinary_inline_container_still_measures_its_atomic_child')"
assert guard.count(needle) == 1
guard = guard.replace(needle, needle + ",\n                       ('fixed-resource-image-canvas-svg-neighbor', '" + old_test + "')")
guard = guard.replace("assert source['clean'] and source['commit'] == FIXED", "assert source['clean'] and source['commit'] == FIXED\nassert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip() == FIXED")
write('openui-native-inline-fallback-guards-v1571.py',guard)

build = adapted('openui-native-inline-replaced-build-v1528.py')
needle = "('inline-build',base+['build','--locked','-p','openui','--example','native_inline_replaced'],'debug/examples/native_inline_replaced'),"
assert build.count(needle) == 1
build = build.replace(needle,needle+"\n ('fallback-build',base+['build','--locked','-p','openui','--example','native_inline_fallback'],'debug/examples/native_inline_fallback'),")
needle = "('native-rust-geometry',[str(out/'native_inline_replaced')],None),"
assert build.count(needle) == 1
build = build.replace(needle,needle+"\n ('native-fallback-geometry',[str(out/'native_inline_fallback')],None),")
write('openui-native-inline-fallback-build-v1571.py',build)
write('openui-native-inline-fallback-matrices-v1571.py',adapted('openui-native-inline-replaced-matrices-v1528.py'))
consumer = adapted('openui-native-inline-replaced-consumer-v1528.py').replace("len(build['steps']) == 7","len(build['steps']) == 9")
consumer = consumer.replace("native-inline-fallback-consumer-v1571", "native-inline-fallback-resource-consumer-v1571")
write('openui-native-inline-fallback-resource-consumer-v1571.py',consumer)

transition = consumer.replace('native-inline-fallback-resource-consumer-v1571','native-inline-fallback-transition-consumer-v1571')
transition = transition.replace("binary = build_path.parent / 'native_inline_replaced'", "binary = build_path.parent / 'native_inline_fallback'")
transition = transition.replace("r['name'] == 'inline-build'", "r['name'] == 'fallback-build'")
needle = "exec(compile(original, str(reference_source), 'exec'))"
assert transition.count(needle) == 1
transition = transition.replace(needle, "adapted_reference = original.replace(\"assert query['natural'] == dict(width=200, height=200)\", \"assert query['natural'] == expected_natural\")\nassert adapted_reference != original\nexec(compile(adapted_reference, str(reference_source), 'exec'))")
transition = transition.replace('capture_conditions_unchanged=True', 'capture_conditions_unchanged=True, natural_size_assertion_parameterized_for_verified_loaded_and_fallback_states=True')
start = transition.index('for phase in [0.0, 0.25, 0.5]:')
end = transition.index('save()\ntry:', start)
new_inputs = '''geometry_path = RAW / 'native-inline-fallback-geometry-v1570/receipt.json'
geometry = json.loads(geometry_path.read_bytes())
assert geometry['all_commands_terminal'] and geometry['independent_queries_identical']
assert geometry['observations'] == 120 and geometry['native_app_sha256'] == sha(ROOT/'bindings/rust/openui/examples/native_inline_fallback.rs')
verified_queries = {(r['scale'],r['phase'],r['state']):r['query'] for r in geometry['runs'][0]['rows']}
report['geometry_receipt_sha256'] = sha(geometry_path)
for source in geometry['inputs']:
    assert sha(Path(source['path'])) == source['sha256']
    for state in ['before','loaded','cleared','reattached']:
        path = OUT / f"phase-{source['phase']:g}-{state}.html"
        text = Path(source['path']).read_text()
        if state == 'loaded':
            text = text.replace('src="invalid.jpg"','src="data:image/png;base64,' + base64.b64encode(asset.read_bytes()).decode() + '"')
        path.write_text(text)
        report['inputs'][f"{source['phase']:g}-{state}"] = dict(path=str(path),sha256=sha(path))
native_geometry = []
'''
transition = transition[:start] + new_inputs + transition[end:]
transition = transition.replace("assert result.stdout.count(b'callback=1 owned-bounds/detach/teardown passed') == 15",'''matches = __import__('re').findall(rb'scale=([0-9.]+) phase=([0-9.]+) state=([a-z]+) x=([-0-9.]+) y=([-0-9.]+) width=([-0-9.]+) height=([-0-9.]+)',result.stdout)
        assert len(matches) == 60
        measured = {(float(r[0]),float(r[1]),r[2].decode()):dict(zip(['x','y','width','height'],map(float,r[3:]))) for r in matches}
        assert len(measured) == 60
        native_geometry.append(measured)
        if repeat == 2: assert native_geometry[0] == native_geometry[1]''')
transition = transition.replace("['before', 'after', 'hidden', 'reattached']", "['before', 'loaded', 'cleared', 'reattached']")
needle = "observations.append(reference(Path(report['inputs'][f'{phase:g}-{state}']['path']), path, 320, 240, scale))"
assert transition.count(needle) == 1
transition = transition.replace(needle,"expected_natural = dict(width=200,height=200) if state == 'loaded' else dict(width=0,height=0)\n                    " + needle)
start = transition.index("                expected = dict(x=0, y=0, width=0, height=0)")
end = transition.index("                analysis =", start)
transition = transition[:start] + '''                expected = verified_queries[(scale,phase,state)]['bounds']
                assert observations[0]['query']['bounds'] == expected
                geometry_exact = native_geometry[0][(scale,phase,state)] == expected
''' + transition[end:]
transition = transition.replace('geometry_exact=True, native_geometry_verified_by_application_assertions=True', 'geometry_exact=geometry_exact, native_bounds=native_geometry[0][(scale,phase,state)], chromium_bounds=expected, native_geometry_verified_by_application_assertions=True')
transition = transition.replace("report['totals']['pixel_exact'] != 60", "report['totals']['pixel_exact'] != 60 or report['totals']['geometry_exact'] != 60")
write('openui-native-inline-fallback-transition-consumer-v1571.py',transition)

config = dict(old,root=str(ROOT),commit=SOURCE,name='native-inline-fallback-pipeline-v1572',
    initial_state='awaiting-all-29-prior-whole-pipelines',prior_pipelines=priors,
    selections=['native-inline-fallback-checks-v1569/receipt.json','native-inline-fallback-geometry-v1570/receipt.json'],
    scripts=[p.name for p in paths] + ['openui-native-image-coverage-fieldsets-v1448.py'],
    stages=[
        ('guards',['/usr/bin/python3','/tmp/openui-native-inline-fallback-guards-v1571.py'],'native-inline-fallback-guards-v1571/receipt.json',True),
        ('native-build',['/usr/bin/python3','/tmp/openui-native-inline-fallback-build-v1571.py'],'native-inline-fallback-clean-v1571/build.json',True),
        ('native-resource-app',['/usr/bin/python3','/tmp/openui-native-inline-fallback-resource-consumer-v1571.py'],'native-inline-fallback-resource-consumer-v1571/receipt.json',True),
        ('native-transition-app',['/usr/bin/python3','/tmp/openui-native-inline-fallback-transition-consumer-v1571.py'],'native-inline-fallback-transition-consumer-v1571/receipt.json',True),
    ] + [(suite,['/usr/bin/python3','/tmp/openui-native-inline-fallback-matrices-v1571.py',suite],f'native-inline-fallback-clean-{suite}-v1571/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']])
body = old_text[old_text.index('\nimport hashlib'):]
body = body.replace('d8742684adf1985ce6db2715778fbbfb0770e6d9',BASELINE)
body = body.replace('expected_stages=7','expected_stages=8').replace('existing_113_exports_and_30_layouts_preserved=True','existing_113_exports_and_30_layouts_preserved=True, new_exports=1, current_exports=114')
body = body.replace("'openui-layout shared inline replaced box collection'", "'openui-layout image fallback flow and shared native image resource lifecycle'")
write('openui-native-inline-fallback-pipeline-v1572.py','CONFIG = '+repr(config)+body)
for p,digest in original_hashes.items(): assert sha(Path(p)) == digest
report = dict(schema_version=1,source=source,source_after=source,baseline_commit=BASELINE,branch=BRANCH,
    read_only_checks_passed=13,ordered_chromium_geometry_queries=120,two_query_runs_identical=True,
    native_and_pixel_results_pending=True,cargo_commands_run=0,screenshots_generated=0,
    required_native_images=120,required_independent_chromium_processes=240,required_consecutive_chromium_captures=480,
    prior_whole_owners=29,required_stages=8,pixel_tolerance=0,javascript_executed_by_openui=False,
    old_openui_pixels_are_provenance_only=True,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,
    public_rust_image_clear_method='Element::clear_image_resource',public_c_image_clear_method='oui_element_clear_image',
    existing_exports_preserved=113,current_exports=114,struct_layouts_unchanged=30,
    checks_receipt_sha256=sha(checks_path),geometry_receipt_sha256=sha(geometry_path),
    original_probe_sha256=original_hashes,scripts={str(p):sha(p) for p in paths},probe_sha256=sha(Path(__file__)))
path = RAW/'native-inline-fallback-queue-prepared-v1571.json'
assert not path.exists();path.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(path=str(path),sha256=sha(path),source=SOURCE,stages=8,prior_whole_owners=29,native_and_pixels_pending=True)),flush=True)
