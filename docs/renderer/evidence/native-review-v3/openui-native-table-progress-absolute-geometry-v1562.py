"""Ordered Chromium layout queries only; Open UI executes no JavaScript."""
import hashlib,importlib.util,json,os,shutil,signal,subprocess,tempfile
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=RAW/'native-table-progress-absolute-geometry-v1562';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py'
spec=importlib.util.spec_from_file_location('capture',harness);capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture)
chrome=ROOT/'chrome/linux-147.0.7727.50/chrome-linux64/chrome';env=capture.chrome_environment(str(chrome.parent),True,False)
source=RAW/'umbrella-clean-workspace-clean-full-v1410/mobile-375x667@2/wpt/css_break/table_repeated-section_variable-fragmentainer-size-003-crash/test.html'
inputs=[]
for height in [40,30,20,40.5]:
 p=OUT/f'height-{height}.html';p.write_bytes(source.read_bytes().replace(b'margin:100px; columns:4;', b'position:absolute;left:120px;top:120px;width:135px;columns:4;', 1).replace(b'height:40px;',f'height:{height}px;'.encode(),1));inputs.append(dict(height=height,path=str(p),sha256=sha(p)))
report=dict(schema_version=1,queries_only=True,screenshots_generated=0,canvas_pixels_generated=0,cargo_commands_run=0,javascript_executed_by_openui=False,public_native_rust_obligation=True,release_qualification=False,all_commands_terminal=False,inputs=inputs,source_path=str(source),source_sha256=sha(source),chromium_binary_sha256=sha(chrome),capture_harness_sha256=sha(harness),fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),probe_sha256=sha(Path(__file__)),runs=[])
receipt=OUT/'receipt.json';save=lambda:receipt.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
expression='''(async()=>{if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const es=[...document.querySelectorAll('body div')];const rect=r=>({x:r.x,y:r.y,width:r.width,height:r.height});return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},elements:es.map((e,index)=>({index,display:getComputedStyle(e).display,clientRects:[...e.getClientRects()].map(rect)}))};})()'''
try:
 for repeat in [1,2]:
  directory=tempfile.mkdtemp(prefix='chrome-layout-query-',dir=STORE);process=client=None;rows=[]
  try:
   process=subprocess.Popen([str(chrome),'--headless','--disable-gpu','--no-sandbox','--no-first-run','--no-default-browser-check','--remote-debugging-port=0','--user-data-dir='+directory,'about:blank'],env=env,start_new_session=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
   port=capture._wait_for_devtools_endpoint(directory,process);client=capture._CdpWebSocket(capture._page_websocket_url(port));client.command('Page.enable')
   for width,height,scale in [(800,600,1),(375,667,2),(1280,720,1.25),(1920,1080,1.5)]:
    client.command('Emulation.setDeviceMetricsOverride',dict(width=width,height=height,deviceScaleFactor=scale,mobile=False))
    for source_input in inputs:
     client.command('Page.navigate',dict(url=Path(source_input['path']).resolve().as_uri()))
     for state,body_height in [('before',100),('after',50),('restored',100)]:
      # Scripts execute in the separate Chromium oracle process only.
      mutation='document.querySelectorAll("body div")[8].style.height="'+str(body_height)+'px";document.querySelectorAll("body div")[8].getBoundingClientRect();'
      result=client.command('Runtime.evaluate',dict(expression=mutation));assert 'exceptionDetails' not in result
      result=client.command('Runtime.evaluate',dict(expression=expression,awaitPromise=True,returnByValue=True));assert 'exceptionDetails' not in result
      query=result['result']['value'];assert capture._device_metrics_match(query['metrics'],width,height,scale)
      rows.append(dict(outer_height=source_input['height'],body_height=body_height,state=state,width=width,height=height,scale=scale,query=query))
   report['runs'].append(dict(repeat=repeat,rows=rows));save();print(json.dumps(dict(repeat=repeat,observations=len(rows),screenshots=0)),flush=True)
  finally:
   if client:client.close()
   if process:
    if process.poll() is None:os.killpg(process.pid,signal.SIGTERM)
    process.wait(timeout=10)
   shutil.rmtree(directory)
 assert report['runs'][0]['rows']==report['runs'][1]['rows'];report['independent_queries_identical']=True;report['observed_exit_code']=0
except BaseException:
 report['observed_exit_code']=1;raise
finally:
 report['all_commands_terminal']=True;report['inputs_unchanged']=sha(source)==report['source_sha256'] and all(sha(Path(p['path']))==p['sha256'] for p in inputs);assert report['inputs_unchanged'];save()
print(json.dumps(dict(path=str(receipt),sha256=sha(receipt),observations=sum(len(r['rows']) for r in report['runs']),identical=True,screenshots=0)),flush=True)
