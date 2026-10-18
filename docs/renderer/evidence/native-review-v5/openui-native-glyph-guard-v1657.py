"""Named baseline/fixed text precision guard; no renderer admission."""
import hashlib,json,os,re,shutil,signal,subprocess,sys,time
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-glyph-guard-3b2e0d1f');RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-glyph-guard-v1657';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
FIXED='3b2e0d1f60b90813859c4c24325c7b0061dea29c';BASELINE='347d901c8de7b5a7890031ac28e58b14cd061275';BRANCH='agent/native-glyph-guard-v1657';PRIORS=['cache-umbrella-pipeline-v1291', 'native-fieldset-retry-pipeline-v1310', 'native-font-retry-pipeline-v1311', 'native-c-raster-retry-pipeline-v1312', 'native-raster-fields-pipeline-v1337', 'native-default-strike-pipeline-v1338', 'cache-integration-pipeline-v1341', 'native-fieldset-capture-forensics-pipeline-v1346', 'native-style-integration-pipeline-v1354', 'native-intrinsic-constraints-pipeline-v1375', 'native-intrinsic-constraints-pipeline-v1369', 'native-raster-consumer-pipeline-v1398', 'umbrella-clean-workspace-pipeline-v1411', 'native-style-integration-retry-pipeline-v1419', 'native-intrinsic-constraints-retry-pipeline-v1420', 'native-raster-api-pipeline-v1425', 'native-image-occlusion-pipeline-v1437', 'native-image-coverage-pipeline-v1449', 'native-event-targets-pipeline-v1457', 'native-border-contrast-pipeline-v1462', 'native-intrinsic-cache-guard-pipeline-v1469', 'native-raster-fields-public-pipeline-v1470', 'native-image-imports-pipeline-v1471', 'native-rounded-border-pipeline-v1482', 'native-border-guard-pipeline-v1501', 'native-enum-values-pipeline-v1513', 'native-inline-replaced-pipeline-v1529', 'native-table-progress-pipeline-v1548', 'native-raster-fields-retry-pipeline-v1560', 'native-inline-fallback-pipeline-v1576', 'native-keywords-pipeline-v1588', 'native-table-source-pipeline-v1601', 'native-text-content-pipeline-v1621', 'native-text-content-viewport-pipeline-v1636', 'native-text-retry-pipeline-v1650', 'native-text-loader-retry-pipeline-v1656']
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
receipt=OUT/'receipt.json';save=lambda:receipt.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
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
 row['test_counts']=[list(map(int,v)) for v in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',content)]
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
