import base64,hashlib,importlib.util,json,os,shutil,signal,subprocess,sys,tempfile
from pathlib import Path
from PIL import Image
ROOT=Path('/dev/shm/openui-native-intrinsic-test-api-7d723caa');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1');OUT=RAW/'native-intrinsic-constraints-consumer-v1418';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1;assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1;STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True);sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity;from residuals import analyze_image_difference
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();source=repository_source_identity(ROOT);assert source['clean'] and source['commit']=='a6d386e48d864a5769ebdadd8bc8dea46e4eff21';buildpath=RAW/'native-intrinsic-constraints-clean-v1418/build.json';build=json.loads(buildpath.read_bytes());assert len(build['steps'])==11 and all(s['observed_exit_code']==0 for s in build['steps']) and source==build['source']==build['source_after'];binary=buildpath.parent/'native_intrinsic_constraints';expected_binary_sha=next(s['binary_sha256'] for s in build['steps'] if s['name']=='constraints-build');assert sha(binary)==expected_binary_sha
harness=ROOT/'tools/accountability/run_all_pixel_comparisons.py';spec=importlib.util.spec_from_file_location('capture',harness);capture=importlib.util.module_from_spec(spec);spec.loader.exec_module(capture);chrome=Path('/home/nero/code/open-ui/chrome/linux-147.0.7727.50/chrome-linux64/chrome');env=capture.chrome_environment(str(chrome.parent),True,False);font=ROOT/'bindings/rust/openui-text/fonts/Ahem.ttf';width,height=320,240
report=dict(schema_version=1,source=source,source_after=source,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,pixel_tolerance=0,javascript_executed_by_openui=False,public_native_rust_api=True,all_commands_terminal=False,cases=[],captures=[],inputs={},native_binary_sha256=sha(binary),build_receipt_sha256=sha(buildpath),chromium_binary_sha256=sha(chrome),capture_harness_sha256=sha(harness),fontconfig_sha256=sha(Path(env['FONTCONFIG_FILE'])),ahem_sha256=sha(font),probe_sha256=sha(Path(__file__)))
receipt=OUT/'receipt.json';save=lambda:receipt.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
expression="""(async()=>{if(document.readyState!=='complete')await new Promise(r=>addEventListener('load',r,{once:true}));await document.fonts.ready;await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));const e=document.querySelector('#outer'),r=e.getBoundingClientRect();return {metrics:{width:innerWidth,height:innerHeight,dpr:devicePixelRatio},bounds:{x:r.x,y:r.y,width:r.width,height:r.height},fontReady:document.fonts.check('16px Ahem')};})()"""
def reference(html, destination, scale):
    profile = tempfile.mkdtemp(prefix='chrome-', dir=STORE)
    process = client = None
    try:
        command = [str(chrome), '--headless', '--disable-gpu', '--no-sandbox', '--no-first-run',
            '--no-default-browser-check', '--remote-debugging-port=0', '--user-data-dir=' + profile,
            f'--window-size={round(width * scale)},{round(height * scale) + 87}', 'about:blank']
        process = subprocess.Popen(command, env=env, start_new_session=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        port = capture._wait_for_devtools_endpoint(profile, process)
        client = capture._CdpWebSocket(capture._page_websocket_url(port))
        client.command('Page.enable')
        client.command('Emulation.setDeviceMetricsOverride', dict(width=width, height=height,
            deviceScaleFactor=scale, mobile=False, screenWidth=width, screenHeight=height))
        client.command('Page.navigate', dict(url=html.resolve().as_uri()))
        result = client.command('Runtime.evaluate', dict(expression=expression, awaitPromise=True, returnByValue=True))
        assert 'exceptionDetails' not in result, result.get('exceptionDetails')
        query = result['result']['value']
        assert capture._device_metrics_match(query['metrics'], width, height, scale)
        options = dict(format='png', fromSurface=True, captureBeyondViewport=False)
        outputs = []
        for number in (1, 2):
            data = client.command('Page.captureScreenshot', options)
            p = destination.with_name(destination.stem + f'-capture-{number}.png')
            p.write_bytes(base64.b64decode(data['data'], validate=True))
            with Image.open(p) as image:
                assert image.size == (round(width * scale), round(height * scale))
            outputs.append(p)
        observation = dict(query=query, first=str(outputs[0]), second=str(outputs[1]),
            first_sha256=sha(outputs[0]), second_sha256=sha(outputs[1]),
            consecutive_png_bytes_identical=outputs[0].read_bytes() == outputs[1].read_bytes())
        report['captures'].append(observation)
        if not observation['consecutive_png_bytes_identical']:
            observation['decoded_rgba_difference'] = analyze_image_difference(outputs[0], outputs[1])
            save()
            raise AssertionError('unstable Chromium reference; both captures preserved without a retry')
        destination.write_bytes(outputs[0].read_bytes())
        return observation
    finally:
        if client is not None:
            client.close()
        if process is not None:
            try:
                os.killpg(process.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            process.wait(timeout=10)
        shutil.rmtree(profile)

contexts={'absolute':'position:absolute','inline-block':'display:inline-block','float':'float:left','min-content':'width:min-content','max-content':'width:max-content'}
cases=[('constraints',context,sizing,minimum,'owned') for context in contexts for sizing in ['border-box','content-box'] for minimum in [0,60,200]]
cases += [('whitespace',context,'border-box',200,profile) for context in contexts for profile in ['owned','installed']]
assert len(cases)==40
inputdir=OUT/'inputs';inputdir.mkdir()
for mode,context,sizing,minimum,profile in cases:
 name=f'{mode}-{context}-{sizing}-{minimum}-{profile}';report['inputs'][name]={}
 for state in ['before','after']:
  resource='@font-face{font-family:Ahem;src:url(data:font/ttf;base64,'+base64.b64encode(font.read_bytes()).decode()+')}' if profile=='owned' else ''
  outer=contexts[context]+';background:red';body=''
  if mode=='constraints':
   value=minimum if state=='before' else (200 if minimum==0 else 0);maximum='' if state=='before' else ';max-width:100px';body=f'<div style="display:inline-block;width:min-content;min-width:{value}px{maximum};box-sizing:{sizing};padding:10px;background:green"><div style="width:20px;height:20px;background:blue"></div></div>'
  else:
   body='\n XXXXX\n' if state=='before' else ' XXXXX ';outer+=';text-indent:20px' if state=='after' else ''
  html=inputdir/(name+'-'+state+'.html');html.write_text('<!doctype html><style>*{margin:0;padding:0;box-sizing:content-box}body{background:white;font:16px/1 Ahem}'+resource+'</style><div style="position:relative;width:240px;height:200px"><div id="outer" style="'+outer+'">'+body+'</div></div>');report['inputs'][name][state]={'path':str(html),'sha256':sha(html)}
save()
try:
 for scale in [1.0,1.25,1.5,2.0,3.0]:
  for mode,context,sizing,minimum,profile in cases:
   name=f'{mode}-{context}-{sizing}-{minimum}-{profile}';directory=OUT/str(scale)/name;directory.mkdir(parents=True);command=[str(binary),str(directory/'native'),str(scale),mode,context,sizing,str(minimum),profile];log=directory/'native.log'
   with log.open('xb') as stream:process=subprocess.run(command,cwd=ROOT,stdout=stream,stderr=subprocess.STDOUT,timeout=60)
   row={'id':name,'scale':scale,'command':command,'observed_exit_code':process.returncode,'log_sha256':sha(log),'states':[]};report['cases'].append(row);save();assert process.returncode==0
   native=json.loads((directory/'native/geometry.json').read_bytes())
   for state in ['before','after']:
    html=Path(report['inputs'][name][state]['path']);assert sha(html)==report['inputs'][name][state]['sha256'];references=[]
    for repeat in [1,2]:
     destination=directory/(state+f'-chromium-{repeat}.png');observation=reference(html,destination,scale);references.append((observation['query'],destination))
    assert references[0][0]==references[1][0] and references[0][1].read_bytes()==references[1][1].read_bytes()
    nativepath=directory/'native'/(state+'.png');geometry_exact=native[state]==references[0][0]['bounds'];difference=analyze_image_difference(references[0][1],nativepath);item={'state':state,'native_bounds':native[state],'chromium_query':references[0][0],'geometry_exact':geometry_exact,'pixel_exact':difference['mismatched_pixels']==0,'difference':difference,'native_png_sha256':sha(nativepath),'chromium_png_sha256':sha(references[0][1]),'owners':[] if geometry_exact and difference['mismatched_pixels']==0 else ['openui-layout','openui-text','openui-paint']};row['states'].append(item);save()
  print('scale',scale,'40 native apps and 80 image comparisons complete',flush=True)
 report['native_apps_complete']=len(report['cases']);report['geometry_exact']=sum(s['geometry_exact'] for r in report['cases'] for s in r['states']);report['pixel_exact']=sum(s['pixel_exact'] for r in report['cases'] for s in r['states']);report['required_images']=400;assert len(report['cases'])==200 and sum(len(r['states']) for r in report['cases'])==400;report['observed_exit_code']=int(report['geometry_exact']!=400 or report['pixel_exact']!=400)
except BaseException:
 report['observed_exit_code']=1;raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT),binary_unchanged=sha(binary)==expected_binary_sha,input_bytes_unchanged=all(sha(Path(i['path']))==i['sha256'] for states in report['inputs'].values() for i in states.values()));assert report['source_after']==source;save()
raise SystemExit(report['observed_exit_code'])
