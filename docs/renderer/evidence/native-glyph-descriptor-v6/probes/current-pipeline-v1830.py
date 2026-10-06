"""Exclusive ownership of all builds, native replays, renderer stages and gaps."""
import fcntl,hashlib,json,os,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1';NAME='native-glyph-current-pipeline-v1830'
OUT=RAW/NAME;STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/NAME
lock=open('/tmp/openui-native-cargo-raster-owner.lock','a');fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
priorpath=RAW/'native-rust-integrated-retry-pipeline-v1816/receipt.json';prior=json.loads(priorpath.read_bytes());assert prior['all_commands_terminal']
try:os.kill(prior['owner_pid'],0)
except ProcessLookupError:pass
else:raise AssertionError('prior whole owner live')
for p in ['cargo','pixel_compare']:assert subprocess.run(['pgrep','-x',p],capture_output=True).returncode==1
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(MAIN/'tools/qualification'));from renderer_source_identity import repository_source_identity
roots={l:Path('/dev/shm/openui-native-glyph-'+l+'-16187f4f-v1825') for l in ['baseline','candidate']};sources={l:repository_source_identity(p) for l,p in roots.items()}
assert sources['baseline']['clean'] and sources['baseline']['commit']=='db03c8facca7502d42f327bae4833acac7379946';assert sources['candidate']['clean'] and sources['candidate']['commit']=='c68d946c18ecd1bb6d2f3f84accecaa84cba3651'
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
checks={}
for l in roots:
 p=RAW/('native-glyph-current-'+l+'-checks-v1827/receipt.json');d=json.loads(p.read_bytes());assert d['source']==d['source_after']==sources[l] and d['all_commands_terminal'] and len(d['checks'])==15 and all(r['observed_exit_code']==0 for r in d['checks']);checks[l]=sha(p)
scripts=[Path('/tmp/openui-native-glyph-current-'+s+'-v1828.py') for s in ['build','consumer','matrices']]
report=dict(schema_version=1,source=sources['candidate'],source_after=sources['candidate'],baseline_source=sources['baseline'],owner_pid=os.getpid(),all_commands_terminal=False,state='prepared-verified',release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,whole_pipeline_lock_covers_all_stages_and_gaps=True,steps=[],probe_sha256=sha(Path(__file__)),immutable_probe_hashes={str(p):sha(p) for p in scripts},read_only_check_receipt_sha256=checks,prior_terminal_pipeline_sha256=sha(priorpath),native_reference_bytes_must_stay_unchanged=True)
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
stages=[]
for l in ['baseline','candidate']:
 stages.extend([(l+'-build',[sys.executable,str(scripts[0]),l],RAW/('native-glyph-current-'+l+'-build-v1828/build.json'),'build'),(l+'-native-fonts',[sys.executable,str(scripts[1]),l],RAW/('native-glyph-current-'+l+'-consumer-v1828/receipt.json'),'consumer')])
for suite in ['focused','primitive','full','expanded']:stages.append((suite,[sys.executable,str(scripts[2]),suite],RAW/('native-glyph-current-clean-'+suite+'-v1828')/(suite+'-summary.json'),'matrix'))
try:
 for name,command,terminalpath,kind in stages:
  assert all(repository_source_identity(roots[l])==sources[l] for l in roots)
  assert all(sha(p)==report['immutable_probe_hashes'][str(p)] for p in scripts)
  report['state']='running-'+name;save();log=OUT/(name+'.log')
  with log.open('xb') as stream:
   process=subprocess.Popen(command,cwd=roots['candidate'],env=dict(os.environ,OPENUI_NATIVE_WHOLE_OWNER=NAME,PYTHONDONTWRITEBYTECODE='1',TMPDIR='/dev/shm/oui1828'),stdout=stream,stderr=subprocess.STDOUT);report['current_process']=dict(pid=process.pid,program=Path(command[1]).name);save();process.wait()
  report.pop('current_process');row=dict(name=name,observed_exit_code=process.returncode,log_sha256=sha(log));report['steps'].append(row);report['source_after']=repository_source_identity(roots['candidate']);save();print(json.dumps({'stage':name,'actual_exit':process.returncode}),flush=True)
  assert terminalpath.exists(),'worker omitted terminal report';d=json.loads(terminalpath.read_bytes())
  if kind=='build':assert process.returncode==0 and d['all_commands_terminal']
  elif kind=='consumer':assert d['all_commands_terminal'] and not d.get('failure') and d['totals']['images']==400
  else:assert (RAW/('native-glyph-current-'+name+'-exit-v1828.json')).exists()
 report.update(state='complete-requires-exact-pixel-and-regression-review',observed_exit_code=int(any(r['observed_exit_code'] for r in report['steps'])))
except BaseException as error:
 report.update(state='stopped-on-build-or-harness-failure',failure=str(error),observed_exit_code=1);raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(roots['candidate']));save()
raise SystemExit(report['observed_exit_code'])
