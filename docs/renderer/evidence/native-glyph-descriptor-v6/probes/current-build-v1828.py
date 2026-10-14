"""Clean source-pinned baseline/candidate builds under one whole pipeline owner."""
import hashlib,json,os,re,shutil,signal,subprocess,sys,time
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
assert os.environ.get('OPENUI_NATIVE_WHOLE_OWNER')=='native-glyph-current-pipeline-v1830'
label=sys.argv[1];assert label in ['baseline','candidate']
ROOT=Path('/dev/shm/openui-native-glyph-'+label+'-16187f4f-v1825');COMMITS={'baseline':'db03c8facca7502d42f327bae4833acac7379946','candidate':'c68d946c18ecd1bb6d2f3f84accecaa84cba3651'}
OUT=RAW/('native-glyph-current-'+label+'-build-v1828');STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name;assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==COMMITS[label]
for p in ['cargo','pixel_compare']:assert subprocess.run(['pgrep','-x',p],capture_output=True).returncode==1
TMP=Path('/mnt/e/openui-v02-qualification-d174ea0b/native-glyph-current-compiler-tmp-v1828');TMP.mkdir(exist_ok=True)
env=dict(os.environ,CARGO_TARGET_DIR='/mnt/e/openui-v02-cargo-c73754e2/target',CARGO_INCREMENTAL='0',RUST_MIN_STACK='4194304',CARGO_BUILD_JOBS='4',PYTHONDONTWRITEBYTECODE='1',TMPDIR=str(TMP),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0')
base=['cargo','--config','.cargo/config.chromium.toml'];metadata=json.loads(subprocess.check_output(base+['metadata','--offline','--locked','--no-deps','--format-version','1'],cwd=ROOT/'bindings/rust',env=env));members=set(metadata['workspace_members']);packages=[p['name'] for p in metadata['packages'] if p['id'] in members];assert len(packages)==18
TEST='shaping::shape_result::tests::fontations_preserves_physical_strike_descriptor_for_real_fonts'
steps=[('clean-workspace',base+['clean']+[a for package in packages for a in ['-p',package]],None),('physical-strike-guard',base+['test','--locked','-p','openui-text','--lib',TEST,'--','--exact'],None)]
if label=='candidate':steps.extend([('all-text-tests',base+['test','--locked','-p','openui-text','--lib'],None),('workspace',base+['test','--workspace','--locked','--features','openui-ffi/linux'],None)])
steps.append(('glyph-build',base+['build','--locked','-p','openui','--example','native_glyph_coverage'],'debug/examples/native_glyph_coverage'))
if label=='candidate':steps.extend([('ffi-build',base+['build','--locked','-p','openui-ffi','--features','linux'],'debug/libopenui_ffi.so'),('ffi-consumers',['python3',str(ROOT/'tools/ffi/verify_abi.py'),'--library',str(OUT/'libopenui_ffi.so')],None),('pixel-build',base+['build','--locked','-p','pixel-compare','--bin','pixel_compare'],'debug/pixel_compare')])
report=dict(schema_version=1,label=label,source=source,source_after=source,all_commands_terminal=False,release_qualification=False,steps=[],workspace_packages_cleaned=18,probe_sha256=sha(Path(__file__)),baseline_regression_reproduced=False,javascript_executed_by_openui=False,pixel_tolerance=0)
save=lambda:(OUT/'build.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
try:
 for name,command,artifact in steps:
  assert repository_source_identity(ROOT)==source
  log=OUT/(name+'.log');tracked=ROOT/'tests/pixel_text/openui_renders/basic_text_openui.png';backup=tracked.read_bytes() if name=='workspace' and tracked.exists() else None
  with log.open('xb') as stream:
   process=subprocess.Popen(command,cwd=ROOT/'bindings/rust',env=env,stdout=stream,stderr=subprocess.STDOUT,start_new_session=True);disk_guard=False
   while process.poll() is None:
    if shutil.disk_usage(MAIN).free<512*2**20 or shutil.disk_usage(STORE).free<10*2**30:
     disk_guard=True;os.killpg(process.pid,signal.SIGTERM);process.wait(timeout=20);break
    time.sleep(1)
   process.wait()
  if backup is not None and tracked.read_bytes()!=backup:
   (OUT/'generated-diagnostic-openui.png').write_bytes(tracked.read_bytes());tracked.write_bytes(backup)
  data=log.read_bytes();totals=[tuple(map(int,m)) for m in re.findall(rb'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;',data)]
  row=dict(name=name,observed_exit_code=process.returncode,disk_guard_triggered=disk_guard,log_sha256=sha(log),passed=sum(t[0] for t in totals),failed=sum(t[1] for t in totals),ignored=sum(t[2] for t in totals));report['steps'].append(row)
  report['source_after']=repository_source_identity(ROOT);assert report['source_after']==source and not disk_guard
  if label=='baseline' and name=='physical-strike-guard':
   reproduced=process.returncode==101 and row['failed']==1 and row['passed']==0 and (TEST+' ... FAILED').encode() in data and b'Fontations must preserve the physical strike descriptor' in data;report['baseline_regression_reproduced']=reproduced;assert reproduced
  else:assert process.returncode==0,'build/test stage failed'
  if artifact:
   p=OUT/Path(artifact).name;shutil.copy2(Path(env['CARGO_TARGET_DIR'])/artifact,p);row.update(binary=str(p),binary_sha256=sha(p))
   if name=='ffi-build':(OUT/'libopenui.so.0').symlink_to(p.name)
   if name=='pixel-build':
    ident=json.loads(subprocess.check_output([str(p),'build-source-identity'],cwd=ROOT,env=env));assert ident['source']==source;report['build_identity']=ident
  save();print(json.dumps({k:row[k] for k in ['name','observed_exit_code','passed','failed','ignored']}),flush=True)
 report['observed_exit_code']=0
except BaseException:
 report['observed_exit_code']=1;raise
finally:
 report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT));save()
