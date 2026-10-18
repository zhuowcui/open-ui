"""Own both native capture retries and all intervening gaps; reuse complete verified build."""
import fcntl, hashlib, json, os, subprocess, sys
from pathlib import Path
NAME='native-rust-integrated-retry-pipeline-v1816'
ROOT=Path('/dev/shm/openui-native-rust-integrated-retry-2d338d6c-v1814')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/NAME;STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/NAME
LOCK=open('/tmp/openui-native-cargo-raster-owner.lock','a');fcntl.flock(LOCK,fcntl.LOCK_EX|fcntl.LOCK_NB)
priorpath=RAW/'native-rust-integrated-pipeline-v1811/receipt.json';prior=json.loads(priorpath.read_bytes())
assert prior['all_commands_terminal']
try:os.kill(prior['owner_pid'],0)
except ProcessLookupError:pass
else:raise AssertionError('prior whole owner remains live')
for name in ['cargo','pixel_compare']:assert subprocess.run(['pgrep','-x',name],capture_output=True).returncode==1
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']=='2d338d6cf8334bae0d7893315d85633d2d586aad'
buildpath=RAW/'native-rust-integrated-clean-v1810/build.json';build=json.loads(buildpath.read_bytes())
assert build['source']==build['source_after']==source and build['all_commands_terminal']
assert len(build['steps'])==15 and all(r['observed_exit_code']==0 for r in build['steps'])
for row in build['steps']:
 if 'binary_sha256' in row:
  assert sha(Path(row['binary']))==row['binary_sha256']
scripts=[Path('/tmp/openui-native-rust-integrated-retry-'+s+'-consumer-v1814.py') for s in ['keywords','options']]
report=dict(schema_version=1,source=source,source_after=source,owner_pid=os.getpid(),all_commands_terminal=False,state='prepared-source-and-complete-build-verified',release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,whole_pipeline_lock_covers_all_stages_and_gaps=True,complete_clean_build_reused=True,build_receipt_sha256=sha(buildpath),prior_terminal_pipeline_sha256=sha(priorpath),probe_sha256=sha(Path(__file__)),immutable_probe_hashes={str(p):sha(p) for p in scripts},steps=[],short_chromium_temporary_storage='/dev/shm/oui1814',capture_visual_conditions_unchanged=True,full_pixel_censuses_and_configuration_field_behavior_unqualified=True)
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
for suffix,script in zip(['keywords','options'],scripts):
 assert repository_source_identity(ROOT)==source and all(sha(p)==report['immutable_probe_hashes'][str(p)] for p in scripts)
 name='native-'+suffix+'-chromium-images';report['state']='running-'+name;save()
 log=OUT/(name+'.log')
 with log.open('xb') as stream:
  process=subprocess.Popen([sys.executable,str(script)],cwd=ROOT,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1',OPENUI_NATIVE_WHOLE_OWNER=NAME,TMPDIR='/dev/shm/oui1814'),stdout=stream,stderr=subprocess.STDOUT)
  report['current_process']=dict(pid=process.pid,program=script.name);save();process.wait()
 report.pop('current_process');report['steps'].append(dict(name=name,observed_exit_code=process.returncode,log_sha256=sha(log)))
 report['source_after']=repository_source_identity(ROOT);assert report['source_after']==source
 receipt=RAW/('native-rust-integrated-retry-'+suffix+'-consumer-v1814')/'receipt.json'
 assert receipt.exists() and json.loads(receipt.read_bytes())['all_commands_terminal'];save()
 print(json.dumps({'stage':name,'actual_exit':process.returncode}),flush=True)
report.update(state='complete-requires-results-review',all_commands_terminal=True);save()
raise SystemExit(int(any(r['observed_exit_code'] for r in report['steps'])))
