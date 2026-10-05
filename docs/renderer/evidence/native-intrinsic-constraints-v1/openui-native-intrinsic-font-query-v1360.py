import base64,hashlib,importlib.util,json,os,shutil,signal,subprocess,tempfile
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1';OUT=RAW/'native-intrinsic-font-query-v1360';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();font=ROOT/'bindings/rust/openui-text/fonts/Ahem.ttf';harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py';chrome=ROOT/'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
spec=importlib.util.spec_from_file_location('capture',harness);capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture);env=capture.chrome_environment(str(chrome.parent),True,False)
variants=[('plain','XXXXX'),('leading',' XXXXX'),('trailing','XXXXX '),('both',' XXXXX '),('newline','\n XXXXX\n'),('bare-span',' <span>XXXXX</span> '),('edge-span',' <span style="border:2px solid;padding:0 6px"> XXXXX </span> '),('empty-edge','<span style="border:2px solid;padding:0 6px"></span> XXXXX'),('break','X<br> XXXXX'),('two-words','X XXXXX')]
contexts=[('absolute','position:absolute'),('inline-block','display:inline-block'),('float','float:left'),('min-content','width:min-content'),('max-content','width:max-content')]
parts=['<!doctype html><style>*{margin:0;padding:0;box-sizing:content-box} @font-face{font-family:OwnedAhem;src:url(data:font/ttf;base64,'+base64.b64encode(font.read_bytes()).decode()+')} .parent{position:relative;width:240px;height:80px} .target{font-size:16px;line-height:16px;font-kerning:none;font-variant-ligatures:none;font-synthesis:none}</style>']
for role,family in [('installed','Ahem'),('owned','OwnedAhem')]:
 for context,style in contexts:
  for variant,text in variants:
   parts.append(f'<div class="parent"><div class="target" id="{role}-{context}-{variant}" style="font-family:{family};{style}">{text}</div></div>')
html=OUT/'input.html';html.write_text(''.join(parts));expression="""(async()=>{if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const canvas=document.createElement('canvas'),context=canvas.getContext('2d');return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},ownedLoaded:document.fonts.check('16px OwnedAhem'),rows:[...document.querySelectorAll('.target')].map(e=>{const s=getComputedStyle(e),r=e.getBoundingClientRect();context.font='16px '+s.fontFamily;context.fontKerning='none';return {id:e.id,width:r.width,height:r.height,fontFamily:s.fontFamily,canvasTextAdvance:context.measureText('XXXXX').width};})};})()"""
r={'schema_version':1,'queries_only':True,'screenshots_generated':0,'canvas_pixels_generated':0,'javascript_executed_by_openui':False,'release_qualification':False,'all_commands_terminal':False,'runs':[],'chromium_binary_sha256':sha(chrome),'capture_harness_sha256':sha(harness),'fontconfig_sha256':sha(Path(env['FONTCONFIG_FILE'])),'ahem_sha256':sha(font),'input_sha256':sha(html),'probe_sha256':sha(Path(__file__))};receipt=OUT/'receipt.json';save=lambda:receipt.write_text(json.dumps(r,sort_keys=True,indent=2)+'\n');save()
try:
 for repeat in (1,2):
  profile=tempfile.mkdtemp(prefix='chrome-query-',dir=STORE);process=client=None;rows=[]
  try:
   command=[str(chrome),'--headless','--disable-gpu','--no-sandbox','--no-first-run','--no-default-browser-check','--remote-debugging-port=0','--user-data-dir='+profile,'about:blank'];process=subprocess.Popen(command,env=env,start_new_session=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);port=capture._wait_for_devtools_endpoint(profile,process);client=capture._CdpWebSocket(capture._page_websocket_url(port));client.command('Page.enable')
   for scale in [1.0,1.25,1.5,2.0,3.0]:
    client.command('Emulation.setDeviceMetricsOverride',dict(width=800,height=600,deviceScaleFactor=scale,mobile=False));client.command('Page.navigate',dict(url=html.resolve().as_uri()));value=client.command('Runtime.evaluate',dict(expression=expression,awaitPromise=True,returnByValue=True));assert 'exceptionDetails' not in value;query=value['result']['value'];assert query['ownedLoaded'] and capture._device_metrics_match(query['metrics'],800,600,scale) and len(query['rows'])==100;rows.append({'scale':scale,'query':query});print('repeat',repeat,'scale',scale,'100 geometry/font observations; no screenshots',flush=True)
   r['runs'].append({'repeat':repeat,'rows':rows});save()
  finally:
   if client:client.close()
   if process:
    if process.poll() is None:os.killpg(process.pid,signal.SIGTERM)
    process.wait(timeout=10)
   shutil.rmtree(profile)
 r['independent_queries_identical']=r['runs'][0]['rows']==r['runs'][1]['rows'];assert r['independent_queries_identical'];r['observed_exit_code']=0
except BaseException:
 r['observed_exit_code']=1;raise
finally:
 r['all_commands_terminal']=True;r['input_unchanged']=sha(html)==r['input_sha256'];save()
