import hashlib,json,os,signal,subprocess,sys,time
from pathlib import Path
root=Path('/dev/shm/openui-native-fieldset-consumer-c170aa7e');raw=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1');out=raw/'native-fieldset-retry-sizing-v1310';store=Path('/mnt/e/openui-v02-qualification-d174ea0b')/out.name
assert not out.exists() and not store.exists();store.mkdir();out.symlink_to(store,target_is_directory=True)
assert subprocess.run(['pgrep','-x','cargo'],capture_output=True).returncode==1
assert subprocess.run(['pgrep','-x','pixel_compare'],capture_output=True).returncode==1
sys.path.insert(0,str(root/'tools/qualification'));from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();buildpath=raw/'native-fieldset-retry-clean-v1310/build.json';build=json.loads(buildpath.read_bytes());source=repository_source_identity(root)
assert len(build['steps'])==10 and all(s['observed_exit_code']==0 for s in build['steps']) and source==build['source']==build['source_after'] and source['clean']
binary=buildpath.parent/'native_intrinsic_sizes';assert sha(binary)==next(s['binary_sha256'] for s in build['steps'] if s['name']=='intrinsic-build');oraclepath=raw/'native-intrinsic-neighbors-v1147/receipt.json';oracle=json.loads(oraclepath.read_bytes())
r={'schema_version':1,'source':source,'source_after':source,'diagnostic_only':True,'release_qualification':False,'promotion_allowed':False,'all_commands_terminal':False,'rss_limit_bytes':512*2**20,'wall_limit_seconds':60,'runs':[],'build_receipt_sha256':sha(buildpath),'oracle_receipt_sha256':sha(oraclepath),'probe_sha256':sha(Path(__file__))}
queue=[(0,200,scale) for scale in [1.0,1.25,1.5,2.0,3.0]];seen=set()
while queue:
 start,end,scale=queue.pop(0)
 if (start,end,scale) in seen:continue
 seen.add((start,end,scale));directory=out/str(scale);directory.mkdir();command=[str(binary),str(directory/'native'),str(scale),f'{start}:{end}'];log=directory/'native.log';peak=0;guard=None;started=time.monotonic()
 with log.open('xb') as f:
  p=subprocess.Popen(command,cwd=root,stdout=f,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   try:
    status=(Path('/proc')/str(p.pid)/'status').read_text();rss=int(next(l for l in status.splitlines() if l.startswith('VmRSS:')).split()[1])*1024;peak=max(peak,rss)
   except (OSError,StopIteration):pass
   if peak>r['rss_limit_bytes']:guard='rss-limit'
   elif time.monotonic()-started>r['wall_limit_seconds']:guard='wall-limit'
   if guard:
    os.killpg(p.pid,signal.SIGTERM);p.wait(timeout=5);break
   time.sleep(.025)
  p.wait()
 row={'scale':scale,'start':start,'end':end,'command':command,'observed_exit_code':p.returncode,'guard':guard,'peak_rss_bytes':peak,'wall_seconds':time.monotonic()-started,'log_sha256':sha(log),'last_log_lines':log.read_text().splitlines()[-4:]}
 if p.returncode==0:
  native=json.loads((directory/'native/geometry.json').read_bytes());assert len(native)==end-start;expected={s['id']:s for s in oracle['runs'][0][0]['query']['rows']};row['geometry_differences']=[{'native':s,'chromium':expected[s['id']]} for s in native if any(s[k]!=expected[s['id']][k] for k in ['width','height'])];row['geometry_exact_count']=len(native)-len(row['geometry_differences'])
 r['runs'].append(row);(out/'receipt.json').write_text(json.dumps(r,sort_keys=True,indent=2)+'\n');print(json.dumps({k:v for k,v in row.items() if k not in ['command','geometry_differences']}),flush=True)
r.update(all_commands_terminal=True,source_after=repository_source_identity(root));assert r['source_after']==source
(out/'receipt.json').write_text(json.dumps(r,sort_keys=True,indent=2)+'\n');raise SystemExit(int(any(s['observed_exit_code'] or s.get('geometry_exact_count')!=200 for s in r['runs'])))
