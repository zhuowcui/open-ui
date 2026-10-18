"""Prepare named authored-glyph baseline/fixed verification behind the text owner."""
import ast,hashlib,json,subprocess
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-glyph-guard-3b2e0d1f');BRANCH='agent/native-glyph-guard-v1657'
FIXED='3b2e0d1f60b90813859c4c24325c7b0061dea29c';BASELINE='347d901c8de7b5a7890031ac28e58b14cd061275'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
old=Path('/tmp/openui-native-text-loader-retry-pipeline-v1656.py');t=old.read_text();c=ast.literal_eval(ast.parse(t).body[0].value)
priors=c['prior_pipelines']+[c['name']];assert len(priors)==len(set(priors))==36
assert all(json.loads((RAW/name/'receipt.json').read_bytes())['all_commands_terminal'] for name in priors[:-1])
assert (ROOT/'tools/qualification/renderer_source_identity.py').is_file()
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==FIXED
probe=Path('/tmp/openui-native-glyph-guard-v1657.py');assert not probe.exists()
text='''"""Named baseline/fixed text precision guard; no renderer admission."""
import hashlib,json,os,re,shutil,signal,subprocess,sys,time
from pathlib import Path
ROOT=Path(__ROOT__);RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-glyph-guard-v1657';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
FIXED=__FIXED__;BASELINE=__BASELINE__;BRANCH=__BRANCH__;PRIORS=__PRIORS__
TEST='shaping::shape_result::tests::authored_lcd_origin_retains_shaped_advance_precision'
for name in PRIORS:assert json.loads((RAW/name/'receipt.json').read_bytes())['all_commands_terminal']
for program in ['cargo','pixel_compare']:assert subprocess.run(['pgrep','-x',program],capture_output=True).returncode==1
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
assert subprocess.check_output(['git','rev-parse',BRANCH],cwd=ROOT,text=True).strip()==FIXED
env=dict(os.environ,CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',CARGO_INCREMENTAL='0',RUST_MIN_STACK='4194304',CARGO_BUILD_JOBS='4',CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',PYTHONDONTWRITEBYTECODE='1')
base=['cargo','--config','.cargo/config.chromium.toml']
metadata=json.loads(subprocess.check_output(base+['metadata','--offline','--locked','--no-deps','--format-version','1'],cwd=ROOT/'bindings/rust',env=env,text=True))
members=set(metadata['workspace_members']);packages=[p['name'] for p in metadata['packages'] if p['id'] in members];assert len(packages)==18
clean=base+['clean']+[a for p in packages for a in ['-p',p]];tests=base+['test','--locked','-p','openui-text','--lib']
report=dict(schema_version=1,source=source,source_after=source,baseline=BASELINE,all_commands_terminal=False,baseline_regression_reproduced=False,state='running',steps=[],release_qualification=False,promotion_allowed=False,applied_to_umbrella=False,parent_has_83_exact_losses=True,javascript_executed_by_openui=False,pixel_tolerance=0,probe_sha256=sha(Path(__file__)))
receipt=OUT/'receipt.json';save=lambda:receipt.write_text(json.dumps(report,sort_keys=True,indent=2)+'\\n');save()
def run(name,command,commit):
 before=repository_source_identity(ROOT);assert before['clean'] and before['commit']==commit
 for program in ['cargo','pixel_compare']:assert subprocess.run(['pgrep','-x',program],capture_output=True).returncode==1
 log=OUT/(name+'.log');guard=False
 with log.open('xb') as stream:
  p=subprocess.Popen(command,cwd=ROOT/'bindings/rust',env=env,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True)
  while p.poll() is None:
   if shutil.disk_usage(MAIN_ROOT).free<512*2**20 or shutil.disk_usage(STORE).free<10*2**30:
    guard=True;os.killpg(p.pid,signal.SIGTERM);p.wait(timeout=20);break
   time.sleep(1)
  p.wait()
 after=repository_source_identity(ROOT);assert after==before
 content=log.read_bytes();row=dict(name=name,command=command,observed_exit_code=p.returncode,disk_guard_triggered=guard,source=before,source_after=after,log_sha256=sha(log))
 row['test_counts']=[list(map(int,v)) for v in re.findall(rb'test result: (?:ok|FAILED)\\. (\\d+) passed; (\\d+) failed; (\\d+) ignored;',content)]
 report['steps'].append(row);save();print(json.dumps({k:row[k] for k in ['name','observed_exit_code','disk_guard_triggered','test_counts']}),flush=True)
 return row,content
MAIN_ROOT=Path('/home/nero/code/open-ui')
try:
 try:
  subprocess.run(['git','switch','--detach',BASELINE],cwd=ROOT,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
  row,_=run('clean-baseline',clean,BASELINE);assert row['observed_exit_code']==0 and not row['disk_guard_triggered']
  row,content=run('baseline-authored-glyph-precision',tests+[TEST,'--','--exact'],BASELINE)
  report['baseline_regression_reproduced']=(row['observed_exit_code']==101 and not row['disk_guard_triggered'] and row['test_counts']==[[0,1,0]] and (TEST+' ... FAILED').encode() in content and b'authored glyph origin must retain shaped advance precision' in content);save()
 finally:
  subprocess.run(['git','switch',BRANCH],cwd=ROOT,check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);assert repository_source_identity(ROOT)==source
 assert report['baseline_regression_reproduced'],'named authored precision assertion was not reproduced'
 row,_=run('clean-fixed',clean,FIXED);assert row['observed_exit_code']==0 and not row['disk_guard_triggered']
 row,content=run('fixed-authored-glyph-precision',tests+[TEST,'--','--exact'],FIXED)
 assert row['observed_exit_code']==0 and not row['disk_guard_triggered'] and row['test_counts']==[[1,0,0]] and (TEST+' ... ok').encode() in content
 row,_=run('fixed-text-suite',tests,FIXED);assert row['observed_exit_code']==0 and not row['disk_guard_triggered'] and len(row['test_counts'])==1 and row['test_counts'][0][0]>0 and row['test_counts'][0][1:]==[0,0]
 report.update(state='complete',observed_exit_code=0)
except BaseException as error:
 report.update(state='failed',failure=str(error),observed_exit_code=1);raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT));assert report['source_after']==source;save()
'''
for key,value in [('__ROOT__',repr(str(ROOT))),('__FIXED__',repr(FIXED)),('__BASELINE__',repr(BASELINE)),('__BRANCH__',repr(BRANCH)),('__PRIORS__',repr(priors))]:text=text.replace(key,value)
ast.parse(text);probe.write_text(text)
c.update(root=str(ROOT),commit=FIXED,name='native-glyph-guard-pipeline-v1658',initial_state='awaiting-all-36-prior-whole-pipelines',prior_pipelines=priors,selections=['native-author-glyph-precision-checks-v1647/receipt.json','native-font-position-audit-v1643.json'],scripts=[probe.name],hard_stop_stages=['guards'],stages=[('guards',['/usr/bin/python3',str(probe)],'native-glyph-guard-v1657/receipt.json',True)])
body=t.split('\n',1)[1];start=body.index('report.update(owner_pid=');end=body.index('receipt = OUT',start)
body=body[:start]+"report.update(owner_pid=os.getpid(),baseline_commit="+repr(BASELINE)+",expected_stages=1,guard_only=True,parent_has_83_exact_losses=True,applied_to_umbrella=False,native_and_pixel_qualification_still_required=True)\n"+body[end:]
owner=Path('/tmp/openui-native-glyph-guard-pipeline-v1658.py');assert not owner.exists();owner.write_text('CONFIG = '+repr(c)+'\n'+body);ast.parse(owner.read_text())
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
report=dict(schema_version=1,root=str(ROOT),branch=BRANCH,source_commit=FIXED,baseline_commit=BASELINE,prior_whole_owners=36,prior_whole_pipelines=priors,guard_only=True,parent_has_83_exact_losses=True,release_qualification=False,native_and_pixel_gates_still_required=True,scripts={str(p):sha(p) for p in [probe,owner]},probe_sha256=sha(Path(__file__)))
p=RAW/'native-glyph-guard-prepared-v1657.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(prepared=True,owner=str(owner),source=FIXED,baseline=BASELINE,prior_owners=36,receipt_sha256=sha(p))),flush=True)
