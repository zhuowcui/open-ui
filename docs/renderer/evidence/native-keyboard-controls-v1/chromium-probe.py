import fcntl,hashlib,json,os,subprocess,sys,tempfile,time
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');OUT=Path('/mnt/d/openui-v02-qualification-d174ea0b/native-keyboard-reference-v3506')
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a+');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
assert not OUT.exists();OUT.mkdir()
sys.path.insert(0,str(ROOT/'tools/accountability'));import run_all_pixel_comparisons as capture
sys.path.insert(0,'/tmp');from openui_parallel_source_identity_v3060 import identity
source=identity(ROOT);assert source['clean'] and source['commit']=='635619da2000ef5db258f97623689cd474084fdb'
chrome=ROOT/'chrome/linux-147.0.7727.50/chrome-linux64/chrome'
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def key(name,code,phase='down',modifiers=0):return dict(op='key',key=name,code=code,phase=phase,modifiers=modifiers)
def attr(target,name,value):return dict(op='attribute',target=target,name=name,value=value)
def focus(target):return dict(op='focus',target=target)
def style(target,name,value):return dict(op='style',target=target,name=name,value=value)
cases=[]
def add(name,kind='radio',pre=None,steps=None,handler=None):cases.append(dict(name=name,kind=kind,pre=pre or [],steps=steps or [],handler=handler))
variants={
 'plain':[], 'other-checked':[dict(op='checked',target='b',value=True)],
 'disabled-peer':[attr('b','disabled','')], 'display-none-peer':[style('b','display','none')],
 'visibility-hidden-peer':[style('b','visibility','hidden')], 'negative-tabindex-peer':[attr('b','tabindex','-1')],
 'other-form-peer':[attr('b','form','fb')], 'empty-name':[attr(x,'name','') for x in ['a','b','c']],
 'sole-group':[attr('b','name','other'),attr('c','name','other')],
}
for variant,pre in variants.items():
 for name,code in [('ArrowLeft',37),('ArrowUp',38),('ArrowRight',39),('ArrowDown',40)]:
  add(variant+'-'+name,pre=pre,steps=[key(name,code),key(name,code,'up')])
for mod in [1,2,4,8,3,5,10,15]:
 for name,code in [('ArrowLeft',37),('ArrowRight',39)]:add('modifier-'+str(mod)+'-'+name,steps=[key(name,code,modifiers=mod),key(name,code,'up',mod)])
for name,code in [('Home',36),('End',35),('PageUp',33),('PageDown',34),('Enter',13),(' ',32)]:
 for checked in [True,False]:add('special-'+str(checked)+'-'+str(code),pre=[dict(op='checked',target='a',value=checked)],steps=[key(name,code),key(name,code,'up')])
for kind in ['checkbox','button']:
 for name,code in [(' ',32),('Enter',13),('ArrowRight',39)]:add(kind+'-'+str(code),kind=kind,steps=[key(name,code),key(name,code,'up')])
for kind in ['radio','checkbox','button']:
 for phase in ['keydown','keyup','click']:
  add(kind+'-cancel-'+phase,kind=kind,pre=[dict(op='checked',target='a',value=False)] if kind!='button' else [],steps=[key(' ',32),key(' ',32,'up')],handler=dict(event=phase,target='a',prevent=True,operations=[]))
 for action in [focus('b'),dict(op='detach',target='a'),attr('a','disabled','')]:
  add(kind+'-between-space-'+action['op'],kind=kind,pre=[dict(op='checked',target='a',value=False)] if kind!='button' else [],steps=[key(' ',32),action,key(' ',32,'up')])
 add(kind+'-repeated-space-down',kind=kind,pre=[dict(op='checked',target='a',value=False)] if kind!='button' else [],steps=[key(' ',32),key(' ',32),key(' ',32,'up')])
for event,target,ops,prevent in [
 ('keydown','a',[focus('c')],False),('keydown','a',[dict(op='detach',target='a')],False),
 ('keydown','a',[],True),('click','b',[],True),('focus','b',[focus('c')],False),
 ('blur','a',[focus('c')],False),('click','b',[attr('b','disabled','')],False),
 ('focus','b',[dict(op='detach',target='b')],False),
]:add('arrow-handler-'+event+'-'+target+'-'+str(len(cases)),steps=[key('ArrowRight',39),key('ArrowRight',39,'up')],handler=dict(event=event,target=target,prevent=prevent,operations=ops))
fixture=dict(schema_version=1,scenarios=cases,scope='native control keyboard navigation and activation; operation data only, no expected states')
(OUT/'cases.json').write_text(json.dumps(fixture,sort_keys=True,indent=2)+'\n')
setup=r'''
document.body.replaceWith(document.createElement('body'));window.nodes={body:document.body};window.rows=[];window.errors=[];
for(let name of ['fa','fb']){let e=document.createElement('form');e.id=name;document.body.appendChild(e);nodes[name]=e;}
for(let name of ['a','b','c']){let e=document.createElement(KIND==='button'?'button':'input');e.id=name;if(KIND!=='button'){e.type=KIND;e.name='g';}else e.type='button';document.body.appendChild(e);nodes[name]=e;}
window.nameOf=n=>Object.entries(nodes).find(([label,node])=>node===n)?.[0]||null;
window.snapshot=()=>({active:nameOf(document.activeElement)||'body',controls:Object.fromEntries(['a','b','c'].map(name=>{let n=nodes[name];return [name,{checked:!!n.checked,indeterminate:!!n.indeterminate,activePseudo:n.matches(':active'),connected:n.isConnected}]}))});
window.apply=op=>{let n=nodes[op.target];switch(op.op){case 'attribute':n.setAttribute(op.name,op.value);break;case 'checked':n.checked=op.value;break;case 'focus':n.focus();break;case 'detach':n.remove();break;case 'style':n.style[op.name]=op.value;break;default:throw Error(op.op);}};
window.handler=HANDLER;window.didHandle=false;
for(let label of ['a','b','c','body'])for(let type of ['keydown','keyup','click','input','change','focus','focusin','blur','focusout'])nodes[label].addEventListener(type,event=>{
 if(event.target!==nodes[label])return;
 const keyboard=type==='keydown'||type==='keyup';
 const modifiers=((event.altKey?1:0)|(event.ctrlKey?2:0)|(event.metaKey?4:0)|(event.shiftKey?8:0));
 rows.push({type,target:nameOf(event.target),related:nameOf(event.relatedTarget),bubbles:event.bubbles,cancelable:event.cancelable,key:keyboard?event.key:null,keyCode:keyboard?event.keyCode:0,modifiers,...snapshot()});
 if(handler&&!didHandle&&handler.event===type&&handler.target===label){didHandle=true;if(handler.prevent)event.preventDefault();for(let op of handler.operations)apply(op);}
});
if(KIND!=='button')nodes.a.checked=true;nodes.a.focus();true
'''
def evaluate(client,text):
 r=client.command('Runtime.evaluate',dict(expression=text,returnByValue=True,awaitPromise=True));assert 'exceptionDetails' not in r,r;return r['result'].get('value')
def barrier(client):return evaluate(client,'new Promise(resolve=>requestAnimationFrame(()=>resolve(true)))')
report=dict(schema_version=1,owner_pid=os.getpid(),source=source,chrome_sha256=sha(chrome),harness_sha256=sha(ROOT/'tools/accountability/run_all_pixel_comparisons.py'),probe_sha256=sha(__file__),fixture_sha256=sha(OUT/'cases.json'),runs=[],all_commands_terminal=False,javascript_executed_by_openui=False,script_runs_only_in_separate_chromium=True,native_api_qualified=False,renderer_qualified=False,release_qualified=False,observed_event_types=['keydown','keyup','click','input','change','focus','focusin','blur','focusout'],keypress_api_not_measured=True,previous_diagnostic_correction='v3503 and v3504 are preserved. Each case now replaces body to remove accumulated root event listeners. Physical keyDown uses committed Space/Enter text; button checked expando is not assigned.')
def save():(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
save()
try:
 for repeat in [1,2]:
  records=[];client=None;process=None
  with tempfile.TemporaryDirectory(prefix='openui-keyboard3506-',dir='/dev/shm') as profile:
   try:
    command=[str(chrome),'--headless','--disable-gpu','--no-sandbox','--no-first-run','--no-default-browser-check','--remote-debugging-port=0','--user-data-dir='+profile,'--window-size=800,687','about:blank']
    process=subprocess.Popen(command,env=capture.chrome_environment(str(chrome.parent)),stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL,start_new_session=True);report['current_process']=dict(pid=process.pid,repeat=repeat);save()
    port=capture._wait_for_devtools_endpoint(profile,process);client=capture._CdpWebSocket(capture._page_websocket_url(port));client.command('Page.enable');client.command('Page.bringToFront');client.command('Emulation.setFocusEmulationEnabled',dict(enabled=True))
    for case in cases:
     evaluate(client,setup.replace('KIND',json.dumps(case['kind'])).replace('HANDLER',json.dumps(case['handler'])))
     for op in case['pre']:evaluate(client,'apply('+json.dumps(op)+');true')
     barrier(client);evaluate(client,'rows.length=0;didHandle=false;true');initial=evaluate(client,'snapshot()');steps=[]
     for op in case['steps']:
      if op['op']=='key':
       packet=dict(type='keyDown' if op['phase']=='down' else 'keyUp',key=op['key'],code='Space' if op['code']==32 else op['key'],windowsVirtualKeyCode=op['code'],nativeVirtualKeyCode=op['code'],modifiers=op['modifiers'])
       if op['phase']=='down' and op['code'] in [13,32]:packet.update(text='\r' if op['code']==13 else ' ',unmodifiedText='\r' if op['code']==13 else ' ')
       client.command('Input.dispatchKeyEvent',packet)
      else:evaluate(client,'apply('+json.dumps(op)+');true')
      synchronous=evaluate(client,'({rows:rows.slice(),state:snapshot(),errors:errors.slice()})');barrier(client);observed=evaluate(client,'({rows:rows.slice(),state:snapshot(),errors:errors.slice()})');steps.append(dict(operation=op,synchronous=synchronous,observed=observed))
     records.append(dict(scenario=case['name'],kind=case['kind'],initial=initial,steps=steps))
    report.pop('current_process',None)
   finally:
    capture._close_chromium_client(client);capture._stop_chromium_process(process)
    report.pop('current_process',None);save()
  path=OUT/('observations-'+str(repeat)+'.json');path.write_text(json.dumps(records,sort_keys=True,indent=2)+'\n');report['runs'].append(dict(repeat=repeat,scenarios=len(records),observations_sha256=sha(path)));save();print(json.dumps(report['runs'][-1]),flush=True)
 assert (OUT/'observations-1.json').read_bytes()==(OUT/'observations-2.json').read_bytes();report['repeated_observations_identical']=True;report['observed_exit_code']=0
except BaseException as error:report.update(observed_exit_code=1,failure=repr(error));raise
finally:
 report['source_after']=identity(ROOT);report['source_unchanged']=report['source_after']==source;report['all_commands_terminal']=True;assert report['source_unchanged'];assert sha(chrome)==report['chrome_sha256'];assert sha(OUT/'cases.json')==report['fixture_sha256'];save()
print(json.dumps(dict(terminal=True,exit=report['observed_exit_code'],scenarios=len(cases),output=str(OUT))),flush=True)
raise SystemExit(report['observed_exit_code'])
