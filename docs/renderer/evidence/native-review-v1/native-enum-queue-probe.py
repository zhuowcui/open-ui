"""Prepare immutable native value qualification behind every prior whole owner."""
import ast, hashlib, json, subprocess
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-enum-values-619a465c'); MAIN=Path('/home/nero/code/open-ui')
RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
SOURCE='893ea292cacf49b50240ba3176f4dc9ff55829d7';BASELINE=subprocess.check_output(['git','rev-parse','agent/native-enum-values-baseline-v1510'],cwd=ROOT,text=True).strip()
assert BASELINE.startswith('33a6578b')
checks_path=RAW/'native-enum-values-checks-v1511/receipt.json';checks=json.loads(checks_path.read_bytes())
assert checks['source']==checks['source_after'] and checks['source']['commit']==SOURCE and checks['source']['clean']
assert checks['all_commands_terminal'] and len(checks['checks'])==13 and all(r['observed_exit_code']==0 for r in checks['checks'])
last=Path('/tmp/openui-native-border-guard-pipeline-v1501.py').read_text();config=ast.literal_eval(ast.parse(last).body[0].value)
priors=config['prior_pipelines']+[config['name']];assert len(priors)==len(set(priors))==25
assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()==SOURCE
assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT)

def adapt(text):
 return text.replace('/dev/shm/openui-native-border-guard-619a465c',str(ROOT)).replace('cbcbc9fb776aba1c40e01d84ed88d6afdbd272a7',SOURCE).replace('39aff4434d7a8a9b1be32d3911bc50701fa06e9f',BASELINE).replace('agent/native-border-guard-v1498','agent/native-enum-values-v1509').replace('native-border-guard-','native-enum-values-').replace('v1500','v1512')

guard=adapt(Path('/tmp/openui-native-border-guard-guards-v1500.py').read_text())
guard=guard.replace("TEST = 'tests::native_bevel_border_raises_low_contrast_colors_without_changing_author_style'", "TEST = 'tests::native_fragment_keywords_reach_engine_and_keep_owned_property_identity'").replace("SELECTED = ['tests::native_bevel_border_raises_low_contrast_colors_without_changing_author_style']", "SELECTED = [TEST]").replace("'-p', 'openui-engine'", "'-p', 'openui-ffi'").replace("b\"low-contrast bevel must expose Chromium's lighter shade\"", "b'native keyword constructor must accept ColumnFill auto'")
lines=guard.splitlines()
for i,line in enumerate(lines):
 if line.startswith('for pipeline in '):lines[i]='for pipeline in '+repr(priors)+':'
guard='\n'.join(lines)+'\n'
anchor="report.update(all_commands_terminal=True, source_after=repository_source_identity(ROOT),"
assert guard.count(anchor)==1
extra='''
extra_tests = [
    ('fixed-table-display-guard', base, 'tests::native_table_display_literals_keep_existing_scalar_encoding'),
    ('fixed-enum-coverage-guard', build_base + ['test', '--locked', '-p', 'openui-style', '--lib'], 'property::tests::native_fragment_keyword_values_cover_declared_enum_variants'),
]
extra_exact = True
for name, command, test in extra_tests:
    extra_row, extra_content = run(name, command + [test, '--', '--exact'], FIXED)
    extra_counts = [list(map(int, values)) for values in re.findall(rb'test result: ok\\. (\\d+) passed; (\\d+) failed; (\\d+) ignored;', extra_content)]
    extra_names = sorted(value.decode() for value in re.findall(rb'^test ([^ ]+) \\.\\.\\. ok$', extra_content, re.MULTILINE))
    extra_row.update(test_counts=extra_counts, observed_named_tests=extra_names)
    extra_exact &= extra_row['observed_exit_code'] == 0 and extra_counts == [[1, 0, 0]] and extra_names == [test]
selected_exact &= extra_exact
'''
guard=guard.replace(anchor,extra+anchor)

build=adapt(Path('/tmp/openui-native-border-guard-build-v1500.py').read_text())
prefix,rest=build.split('for prior in ',1);_,rest=rest.split(':\n',1);build=prefix+'for prior in '+repr(priors)+':\n'+rest
build=build.replace("('border-build',base+['build','--locked','-p','openui','--example','native_border_contrast'],'debug/examples/native_border_contrast'),", "('keywords-build',base+['build','--locked','-p','openui','--example','native_fragment_keywords'],'debug/examples/native_fragment_keywords'),")
build=build.replace("('float-build',base+['build','--locked','-p','openui','--example','native_float_colors'],'debug/examples/native_float_colors'),\n",'')
build=build.replace(" ('ffi-consumers',", " ('native-rust-geometry',[str(out/'native_fragment_keywords')],None),\n ('ffi-consumers',")
matrices=adapt(Path('/tmp/openui-native-border-guard-matrices-v1500.py').read_text())

consumer='''"""Native typed Rust mutation versus strict repeated pinned Chromium captures."""
import ast, base64, hashlib, importlib.util, io, json, os, shutil, signal, subprocess, sys, tempfile
from pathlib import Path
from PIL import Image
ROOT=Path(ROOT_LITERAL);RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-enum-values-consumer-v1512';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
build_path=RAW/'native-enum-values-clean-v1512/build.json';build=json.loads(build_path.read_bytes());source=repository_source_identity(ROOT)
assert source['clean'] and source['commit']==SOURCE_LITERAL and source==build['source']==build['source_after']
assert build['all_commands_terminal'] and len(build['steps'])==7 and all(r['observed_exit_code']==0 for r in build['steps'])
binary=build_path.parent/'native_fragment_keywords';assert sha(binary)==next(r['binary_sha256'] for r in build['steps'] if r['name']=='keywords-build')
harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py';spec=importlib.util.spec_from_file_location('capture',harness);capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture)
chrome=Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome');env=capture.chrome_environment(str(chrome.parent),True,False)
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=False,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,public_native_rust_api=True,javascript_executed_by_openui=False,pixel_tolerance=0,native_binary_sha256=sha(binary),chromium_binary_sha256=sha(chrome),capture_harness_sha256=sha(harness),fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),probe_sha256=sha(Path(__file__)),build_receipt_sha256=sha(build_path),unstable_reference_captures=[],inputs={},native_runs=[],cases=[])
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\\n');save()
capture_source=Path('/tmp/openui-native-image-coverage-fieldsets-v1448.py');capture_text=capture_source.read_text();node=next(n for n in ast.parse(capture_text).body if isinstance(n,ast.FunctionDef) and n.name=='reference');original=ast.get_source_segment(capture_text,node)
removed="        assert query['natural'] == dict(width=200, height=200)\\n";assert original.count(removed)==1
adapted=original.replace(removed,'');exec(compile(adapted,str(capture_source),'exec'))
report.update(reference_source_sha256=sha(capture_source),original_reference_function_sha256=hashlib.sha256(original.encode()).hexdigest(),adapted_reference_function_sha256=hashlib.sha256(adapted.encode()).hexdigest(),reference_adaptations=['Remove only the image natural-size assertion for a div case'],capture_conditions_unchanged=True,strict_chromium_capture_pairs=True,preserves_both_unstable_reference_captures=True)
expression="""(async()=>{if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const r=document.getElementById('image').getBoundingClientRect();return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},bounds:{x:r.x,y:r.y,width:r.width,height:r.height}};})()"""
for state in ['before','after']:
 path=OUT/(state+'.html');override='border-top-style:none;background:red;column-fill:balance;break-inside:auto' if state=='after' else ''
 path.write_text('<!doctype html><meta charset="utf-8"><style>*{margin:0;padding:0;border:0;box-sizing:content-box}html,body{width:64px;height:48px;overflow:hidden;background:white}#image{width:20px;height:20px;border:4px solid black;background:blue;column-fill:auto;column-wrap:nowrap;column-span:none;break-inside:avoid;break-before:avoid-column;break-after:avoid-page;box-decoration-break:slice;'+override+'}</style><div id="image"></div>\\n')
 report['inputs'][state]=dict(path=str(path),sha256=sha(path))
save()
try:
 for repeat in [1,2]:
  directory=OUT/('native-'+str(repeat));command=[str(binary),str(directory)];result=subprocess.run(command,cwd=ROOT,capture_output=True)
  log=OUT/('native-'+str(repeat)+'.log');log.write_bytes(result.stdout+result.stderr);report['native_runs'].append(dict(command=command,observed_exit_code=result.returncode,log_sha256=sha(log)))
  assert result.returncode==0,'native callback/owned style/geometry/teardown application failed'
  assert result.stdout.count(b'callback=1 owned-styles/bounds passed')==5
  save()
 for scale in [1.0,1.25,1.5,2.0,3.0]:
  directory=OUT/str(scale);directory.mkdir();row=dict(scale=scale,images=[]);report['cases'].append(row)
  for state in ['before','after']:
   actual=OUT/f'native-1/scale-{scale:g}/{state}.png';assert actual.read_bytes()==(OUT/f'native-2/scale-{scale:g}/{state}.png').read_bytes()
   refs=[];observations=[]
   for repeat in [1,2]:
    path=directory/f'{state}-chromium-{repeat}.png';observations.append(reference(Path(report['inputs'][state]['path']),path,64,48,scale));refs.append(path)
   assert refs[0].read_bytes()==refs[1].read_bytes() and observations[0]['query']==observations[1]['query']
   expected=dict(x=0,y=0,width=28,height=28 if state=='before' else 24)
   assert observations[0]['query']['bounds']==expected
   analysis=analyze_image_difference(refs[0],actual)
   row['images'].append(dict(state=state,analysis=analysis,native_png_sha256=sha(actual),chromium_png_sha256=sha(refs[0]),geometry_exact=True,native_geometry_verified_by_application_assertions=True,independent_reference_runs=observations))
  save();print(json.dumps(dict(scale=scale,pixel_differences=[i['analysis']['mismatched_pixels'] for i in row['images']])),flush=True)
 images=[i for row in report['cases'] for i in row['images']]
 report['totals']=dict(images=len(images),pixel_exact=sum(i['analysis']['mismatched_pixels']==0 for i in images),geometry_exact=sum(i['geometry_exact'] for i in images),native_application_runs=2,deterministic_native_image_pairs=10,independent_chromium_capture_processes=20,consecutive_chromium_captures=40,native_callback_states=10)
 report['observed_exit_code']=int(len(images)!=10 or report['totals']['pixel_exact']!=10)
except BaseException as error:
 report.update(observed_exit_code=1,failure=str(error));raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT));assert source==report['source_after'] and sha(binary)==report['native_binary_sha256'];assert all(sha(Path(r['path']))==r['sha256'] for r in report['inputs'].values());save()
raise SystemExit(report['observed_exit_code'])
'''.replace('ROOT_LITERAL',repr(str(ROOT))).replace('SOURCE_LITERAL',repr(SOURCE))
config.update(root=str(ROOT),commit=SOURCE,name='native-enum-values-pipeline-v1513',initial_state='awaiting-all-25-prior-whole-pipelines',prior_pipelines=priors,selections=['native-enum-values-checks-v1511/receipt.json'])
config['scripts']=['openui-native-enum-values-guards-v1512.py','openui-native-enum-values-build-v1512.py','openui-native-enum-values-consumer-v1512.py','openui-native-enum-values-matrices-v1512.py','openui-native-image-coverage-fieldsets-v1448.py']
config['stages']=[('guards',['/usr/bin/python3','/tmp/openui-native-enum-values-guards-v1512.py'],'native-enum-values-guards-v1512/receipt.json',True),('native-build',['/usr/bin/python3','/tmp/openui-native-enum-values-build-v1512.py'],'native-enum-values-clean-v1512/build.json',True),('native-application',['/usr/bin/python3','/tmp/openui-native-enum-values-consumer-v1512.py'],'native-enum-values-consumer-v1512/receipt.json',True)]+[(suite,['/usr/bin/python3','/tmp/openui-native-enum-values-matrices-v1512.py',suite],f'native-enum-values-clean-{suite}-v1512/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']]
body=last.split('\n',1)[1];start=body.index('report.update(owner_pid=');end=body.index("receipt = OUT / 'receipt.json'",start)
body=body[:start]+f'''report.update(owner_pid=os.getpid(), baseline_commit={BASELINE!r}, expected_stages=7,
    workspace_cleaned_at_every_source_switch=True, existing_113_exports_and_30_layouts_preserved=True,
    native_event_api_preserved=True, c_abi_unchanged=True, actual_pixel_gain_claimed=False,
    root_cause_owner='openui-style shared native value construction', native_constructor_qualification_pending=True,
    strict_named_baseline_failure_required=True, public_rust_callbacks_and_owned_bounds=True,
    strict_chromium_capture_pairs=True, preserves_both_unstable_reference_captures=True)
'''+body[end:]
owner='CONFIG = '+repr(config)+'\n'+body
files={Path('/tmp/openui-native-enum-values-guards-v1512.py'):guard,Path('/tmp/openui-native-enum-values-build-v1512.py'):build,Path('/tmp/openui-native-enum-values-consumer-v1512.py'):consumer,Path('/tmp/openui-native-enum-values-matrices-v1512.py'):matrices,Path('/tmp/openui-native-enum-values-pipeline-v1513.py'):owner}
for path,text in files.items():assert not path.exists();ast.parse(text)
assert "'openui-engine'" not in guard and 'low-contrast bevel' not in guard
assert "[[1, 0, 0]]" in guard and 'native keyword constructor must accept ColumnFill auto' in guard
assert build.count("('keywords-build',")==1 and 'native_border_contrast' not in build
assert 'native-rust-geometry' in build and config['prior_pipelines']==priors
for path,text in files.items():path.write_text(text)
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
report=dict(schema_version=1,root=str(ROOT),source=SOURCE,baseline=BASELINE,branch='agent/native-enum-values-v1509',baseline_branch='agent/native-enum-values-baseline-v1510',prior_whole_pipelines_required_terminal=25,prior_whole_pipelines=priors,checks_receipt_sha256=sha(checks_path),all_thirteen_read_only_checks_passed=True,strict_named_baseline_failure_required=True,named_fixed_guards_required=3,workspace_packages_cleaned=18,c_examples_required=12,cpp_consumers_required=6,required_native_images=10,required_repeated_chromium_captures=40,all_four_matrices_required=True,pixel_tolerance=0,applied_to_umbrella=False,release_qualification=False,new_release_states_admitted=0,javascript_executed_by_openui=False,cargo_commands_run=0,screenshots_generated=0,scripts={str(p):sha(p) for p in files},probe_sha256=sha(Path(__file__)))
out=RAW/'native-enum-values-queue-prepared-v1512.json';assert not out.exists();out.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(path=str(out),sha256=sha(out),source=SOURCE,baseline=BASELINE,prior_whole_owners=25)),flush=True)
