"""Compare real Rust callbacks and explicit raster selection with immutable Chromium captures."""
import functools,hashlib,json,os,shutil,subprocess,sys
from pathlib import Path
from PIL import Image,ImageChops
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
assert os.environ.get('OPENUI_NATIVE_WHOLE_OWNER')=='native-glyph-current-pipeline-v1830'
label=sys.argv[1];assert label in ['baseline','candidate'];ROOT=Path('/dev/shm/openui-native-glyph-'+label+'-16187f4f-v1825')
OUT=RAW/('native-glyph-current-'+label+'-consumer-v1828');STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
for p in ['cargo','pixel_compare']:assert subprocess.run(['pgrep','-x',p],capture_output=True).returncode==1
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
from residuals import analyze_image_difference
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();source=repository_source_identity(ROOT)
buildpath=RAW/('native-glyph-current-'+label+'-build-v1828/build.json');build=json.loads(buildpath.read_bytes());assert build['all_commands_terminal'] and build['observed_exit_code']==0 and source==build['source']==build['source_after'] and source['clean']
binary=buildpath.parent/'native_glyph_coverage';assert sha(binary)==next(r['binary_sha256'] for r in build['steps'] if r['name']=='glyph-build')
priorpath=RAW/'native-text-style-runtime-consumer-v1677/receipt.json';prior=json.loads(priorpath.read_bytes());assert prior['all_commands_terminal'] and len(prior['cases'])==100 and all(p['consecutive_png_bytes_identical'] for p in prior['capture_pairs'])
chrome=MAIN/'chrome/linux-147.0.7727.50/chrome-linux64/chrome';assert sha(chrome)==prior['chromium']['binary_sha256']
harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py';assert sha(harness)==prior['capture_harness_sha256']
fonts=ROOT/'bindings/rust/openui-text/fonts';assert {p.name:sha(p) for p in sorted(fonts.glob('*.ttf'))}==prior['font_assets']
report=dict(schema_version=1,label=label,source=source,source_after=source,all_commands_terminal=False,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,javascript_executed_by_openui=False,public_native_rust_application_apis=True,immutable_raster_selections=['default','chromium-lcd'],pixel_tolerance=0,build_receipt_sha256=sha(buildpath),native_binary_sha256=sha(binary),prior_reference_receipt_sha256=sha(priorpath),chromium_binary_sha256=sha(chrome),capture_harness_sha256=sha(harness),font_assets=prior['font_assets'],new_chromium_processes=0,reference_bytes_reused_unchanged=True,probe_sha256=sha(Path(__file__)),cases=[],inputs={},default_prior_image_checks=[])
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
try:
 for raster in ['default','chromium-lcd']:
  for previous in prior['cases']:
   assert previous['policy']=='fontations';family=previous['family'];size=previous['size'];scale=previous['scale'];key=family.replace(' ','-')+'-'+str(size)
   directory=OUT/raster/key/str(scale);directory.mkdir(parents=True);prior_directory=priorpath.parent/'fontations'/key/str(scale)
   row=dict(raster=raster,family=family,size=size,scale=scale,native_runs=[],images=[]);report['cases'].append(row)
   geometries=[]
   for repeat in [1,2]:
    destination=directory/('native-'+str(repeat));command=[str(binary),str(destination),str(scale),family,str(size),raster];result=subprocess.run(command,cwd=ROOT,capture_output=True);log=directory/('native-'+str(repeat)+'.log');log.write_bytes(result.stdout+result.stderr);row['native_runs'].append(dict(repeat=repeat,observed_exit_code=result.returncode,log_sha256=sha(log)));save();assert result.returncode==0,'native Rust callback, owned configuration, geometry or teardown failed'
    geometries.append(json.loads((destination/'geometry.json').read_bytes()))
   assert geometries[0]==geometries[1] and geometries[0]['callback_count']==1
   for state in ['before','after']:
    original_input=Path(prior['inputs'][key][state]['path']);assert sha(original_input)==prior['inputs'][key][state]['sha256'];inputpath=OUT/'inputs'/(key+'-'+state+'.html');inputpath.parent.mkdir(exist_ok=True)
    if not inputpath.exists():shutil.copy2(original_input,inputpath)
    assert sha(inputpath)==sha(original_input);report['inputs'][key+'-'+state]=dict(path=str(inputpath),sha256=sha(inputpath))
    oldimage=next(i for i in previous['images'] if i['language']=='rust' and i['state']==state);observations=oldimage['independent_reference_runs'];assert len(observations)==2 and observations[0]==observations[1]
    actual=directory/'native-1'/(state+'.png');assert actual.read_bytes()==(directory/'native-2'/(state+'.png')).read_bytes()
    refs=[]
    for repeat in [1,2]:
     oldpath=prior_directory/(state+'-chromium-'+str(repeat)+'.png');assert sha(oldpath)==oldimage['chromium_png_sha256']==observations[repeat-1]['chromium_png_sha256'];assert observations[repeat-1]['repeated_capture_pair_identical'] and observations[repeat-1]['capture_count']==2;ref=directory/oldpath.name;shutil.copy2(oldpath,ref);refs.append(ref)
    assert refs[0].read_bytes()==refs[1].read_bytes();query=observations[0]['query'];assert query['metrics']==dict(width=800,height=600,dpr=scale) and len(query['bounds'])==64
    assert len(observations[0]['platform_fonts'])==1 and observations[0]['platform_fonts'][0]['familyName']==family
    analysis=analyze_image_difference(refs[0],actual)
    with Image.open(actual) as a,Image.open(refs[0]) as b:
     assert a.size==b.size==(round(800*scale),round(600*scale));delta=ImageChops.difference(a.convert('RGBA'),b.convert('RGBA'));mask=functools.reduce(ImageChops.lighter,delta.split());phases=[]
     for phase in range(64):
      x,y=12+phase%8*92,12+phase//8*60;cell=tuple(round(v*scale) for v in (x,y,x+92,y+60));native=geometries[0][state][phase];expected=query['bounds'][phase];phases.append(dict(logical_phase_64ths=phase,mismatched_pixels=sum(mask.crop(cell).histogram()[1:]),geometry_exact=native==expected,native_bounds=native,chromium_bounds=expected))
    row['images'].append(dict(state=state,analysis=analysis,phases=phases,native_png_sha256=sha(actual),chromium_png_sha256=sha(refs[0]),independent_reference_runs=observations,owner='shared text strike and positioning; reviewed root cause pending' if analysis['mismatched_pixels'] else None,reviewed_root_cause=False if analysis['mismatched_pixels'] else None))
    if raster=='default':
     oldnative=prior_directory/'rust-1'/(state+'.png');assert sha(oldnative)==oldimage['native_png_sha256'];report['default_prior_image_checks'].append(dict(family=family,size=size,scale=scale,state=state,prior_png_sha256=sha(oldnative),native_png_sha256=sha(actual),same_png=oldnative.read_bytes()==actual.read_bytes(),same_rgba=analyze_image_difference(oldnative,actual)['mismatched_pixels']==0))
   save()
  images=[i for row in report['cases'] if row['raster']==raster for i in row['images']];phases=[p for i in images for p in i['phases']]
  print(json.dumps(dict(label=label,raster=raster,images=len(images),exact=sum(i['analysis']['mismatched_pixels']==0 for i in images),geometry_exact=sum(p['geometry_exact'] for p in phases))),flush=True)
 images=[i for row in report['cases'] for i in row['images']];phases=[p for i in images for p in i['phases']]
 report['totals']=dict(cases=len(report['cases']),images=len(images),pixel_exact=sum(i['analysis']['mismatched_pixels']==0 for i in images),phase_states=len(phases),phase_exact=sum(p['mismatched_pixels']==0 for p in phases),geometry_exact=sum(p['geometry_exact'] for p in phases),native_processes=400,all_native_pairs_deterministic=True,all_reference_bytes_unchanged=True)
 assert len(report['cases'])==200 and len(images)==400 and len(phases)==25600
 report['observed_exit_code']=int(report['totals']['pixel_exact']!=400 or report['totals']['geometry_exact']!=25600)
except BaseException as error:
 report.update(observed_exit_code=1,failure=str(error));raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT));assert report['source_after']==source and sha(binary)==report['native_binary_sha256'] and sha(priorpath)==report['prior_reference_receipt_sha256'] and sha(chrome)==report['chromium_binary_sha256'];save()
raise SystemExit(report['observed_exit_code'])
