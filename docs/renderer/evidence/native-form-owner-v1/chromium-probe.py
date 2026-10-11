import fcntl,hashlib,json,os,subprocess,sys,tempfile
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');NAME='native-form-owner-reference-v3410';RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1';OUT=Path('/mnt/e/openui-v02-qualification-d174ea0b')/NAME
lock=open('/tmp/'+NAME+'.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
assert not OUT.exists() and not (RAW/NAME).exists();OUT.mkdir();(RAW/NAME).symlink_to(OUT,target_is_directory=True)
sys.path.insert(0,str(MAIN/'tools/accountability'));import run_all_pixel_comparisons as capture
sys.path.insert(0,'/tmp');from openui_parallel_source_identity_v3060 import identity
source=identity(MAIN);assert source['clean'] and source['commit']=='783e11f44b0e1cd0023018d24e667d27f0b91bb8'
chrome=MAIN/'chrome/linux-147.0.7727.50/chrome-linux64/chrome';scratch=Path('/dev/shm/ou3410');scratch.mkdir(exist_ok=False)
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
fixture_path=Path('/tmp/openui-native-form-owner-cases-v3410.json');fixture=json.loads(fixture_path.read_bytes());cases=fixture['cases']
setup="document.body.replaceChildren();window.nodes={body:document.body};\nfunction make(label,tag,parent){let n=document.createElement(tag);nodes[label]=n;if(parent)nodes[parent].appendChild(n);return n;}\nfor(let [label,tag] of [['wrap','div'],['fa','form'],['fb','form'],['parking','div'],['blocker','div']]){make(label,tag,'body').id=label;}\nmake('newform','form',null);\nfor(let [label,parent,owner] of [['a','wrap','fa'],['b','wrap','fb'],['c','fa',null],['d','fb',null],['e','body',null]]){let n=make(label,'input',null);n.id=label;n.type=TYPE;n.name='g';if(owner)n.setAttribute('form',owner);nodes[parent].appendChild(n);}\nwindow.fields=['a','b','c','d','e'];window.rows=[];\nwindow.nameOf=n=>Object.entries(nodes).find(([label,node])=>node===n)?.[0]||null;\nwindow.snapshot=()=>({controls:Object.fromEntries(fields.map(label=>{let n=nodes[label];return [label,{checked:n.checked,checkedAttribute:n.hasAttribute('checked'),formAttribute:n.getAttribute('form'),formOwner:nameOf(n.form),connected:n.isConnected,parent:nameOf(n.parentElement)}]})),active:nameOf(document.activeElement)||'body'});\nwindow.apply=op=>{let n=nodes[op.target];switch(op.op){case 'attribute':n.setAttribute(op.name,op.value);break;case 'remove_attribute':n.removeAttribute(op.name);break;case 'detach':n.remove();break;case 'append':nodes[op.parent].appendChild(n);break;case 'insert':nodes[op.parent].insertBefore(n,nodes[op.before]);break;case 'checked':n.checked=op.value;break;default:throw Error(op.op);}};\nfor(let label of fields)for(let type of ['click','input','change','focus','focusin','blur','focusout'])nodes[label].addEventListener(type,event=>rows.push({type,target:nameOf(event.target),bubbles:event.bubbles,cancelable:event.cancelable,...snapshot()}));\nfor(let label of ['a','b','e'])nodes[label].checked=true;true"
report=dict(schema_version=1,source=source,owner_pid=os.getpid(),all_commands_terminal=False,script_runs_only_in_offline_chromium=True,javascript_executed_by_openui=False,native_api_behavior_measured=False,release_qualified=False,diagnostic_only=True,probe_sha256=sha(__file__),chrome_sha256=sha(chrome),harness_sha256=sha(MAIN/'tools/accountability/run_all_pixel_comparisons.py'),runs=[],fixture_sha256=sha(fixture_path),scope='ordered native form association, tree mutations and radio checked-state transitions')
def save():(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
def evaluate(client,text,promise=False):
 r=client.command('Runtime.evaluate',dict(expression=text,returnByValue=True,awaitPromise=promise));assert 'exceptionDetails' not in r,r;return r['result'].get('value')
def barrier(client):return evaluate(client,'new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve(true))))',True)
save()
try:
 for repeat in [1,2]:
  observations=[]
  process=None;client=None
  with tempfile.TemporaryDirectory(prefix='chrome-',dir=scratch) as profile:
   try:
    command=[str(chrome),'--headless','--disable-gpu','--no-sandbox','--no-first-run','--no-default-browser-check','--remote-debugging-port=0','--user-data-dir='+profile,'--window-size=800,687','about:blank'];process=subprocess.Popen(command,env=capture.chrome_environment(str(chrome.parent)),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True);report['current_process']=dict(pid=process.pid,repeat=repeat);save();port=capture._wait_for_devtools_endpoint(profile,process);client=capture._CdpWebSocket(capture._page_websocket_url(port));client.command('Page.enable');client.command('Page.bringToFront');client.command('Emulation.setFocusEmulationEnabled',dict(enabled=True))
    for tag in fixture['controls']:
     for case in cases:
      evaluate(client,setup.replace('TYPE',json.dumps(tag)));barrier(client)
      for op in case['pre']:evaluate(client,'apply('+json.dumps(op)+');true')
      evaluate(client,'rows.length=0;true');initial=evaluate(client,'snapshot()');steps=[]
      for op in case['steps']:
       synchronous=evaluate(client,'apply('+json.dumps(op)+');({rows:rows.slice(),state:snapshot()})');barrier(client);observed=evaluate(client,'({rows:rows.slice(),state:snapshot()})');steps.append(dict(operation=op,synchronous=synchronous,observed=observed))
      observations.append(dict(scenario=tag+'-'+case['name'],control=tag,variant=case['name'],initial=initial,steps=steps));report['current_scenario']=tag+'-'+case['name'];save()
   finally:capture._close_chromium_client(client);capture._stop_chromium_process(process);report.pop('current_process',None);save()
  report.pop('current_scenario',None);path=OUT/('observations-'+str(repeat)+'.json');path.write_text(json.dumps(observations,sort_keys=True,indent=2)+'\n');report['runs'].append(dict(repeat=repeat,scenarios=len(observations),callback_rows=sum(len(x['steps'][-1]['observed']['rows']) for x in observations),observations_sha256=sha(path)));save();print(json.dumps(report['runs'][-1]),flush=True)
 assert (OUT/'observations-1.json').read_bytes()==(OUT/'observations-2.json').read_bytes();report['repeated_observations_identical']=True;report['observed_exit_code']=0
except BaseException as error:report.update(observed_exit_code=1,failure=repr(error));raise
finally:
 report.update(source_after=identity(MAIN),all_commands_terminal=True);report['source_unchanged']=report['source_after']==source;assert report['source_unchanged'];assert sha(fixture_path)==report['fixture_sha256'];assert sha(chrome)==report['chrome_sha256'] and sha(MAIN/'tools/accountability/run_all_pixel_comparisons.py')==report['harness_sha256'];save()
print(json.dumps(dict(all_commands_terminal=True,observed_exit_code=report['observed_exit_code'],scenarios=report['runs'])),flush=True)
raise SystemExit(report['observed_exit_code'])
