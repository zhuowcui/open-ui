"""Inspect retained fragment geometry through the verified debug-only executable."""
import hashlib,json,os,re,subprocess,sys
from pathlib import Path
ROOT=Path('/dev/shm/openui-native-raster-fields-retry-e0dc491e')
RAW=Path('/home/nero/code/open-ui/out/renderer-evidence/native-viewport-scroll-v1')
OUT=RAW/'native-table-fragment-debug-v1596';STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists();STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(ROOT/'tools/qualification'))
from renderer_source_identity import repository_source_identity
sha=lambda p:hashlib.file_digest(p.open('rb'),'sha256').hexdigest()
build_path=RAW/'native-raster-fields-retry-clean-v1559/build.json';build=json.loads(build_path.read_bytes())
source=repository_source_identity(ROOT)
assert source==build['source']==build['source_after'] and source['clean'] and source['commit']=='e0dc491e61e17ce4407ff2dd30289e741690572b'
assert build['all_commands_terminal'] and all(r['observed_exit_code']==0 for r in build['steps'])
row=next(r for r in build['steps'] if r['name']=='pixel-build');binary=Path(row['binary'])
assert sha(binary)==row['binary_sha256']
identity=json.loads(subprocess.check_output([str(binary),'build-source-identity'],cwd=ROOT))
assert identity['source']==source
cli=ROOT/'bindings/rust/pixel-compare/src/main.rs';text=cli.read_text()
mode=text.split('Some("debug") => {',1)[1].split('Some("render") => {',1)[0]
assert 'doc.scene()' in mode and 'render_test' not in mode and 'render_to_png' not in mode
fixture=ROOT/'bindings/rust/pixel-compare/src/wpt/wpt_css_break.rs';fixtures=fixture.read_text()
builder=fixtures.split('fn css_break_table_repeated_section_variable_fragmentainer_size_003_crash(',1)[1].split('\nfn ',1)[0]
assert 'render_to_png' not in builder and 'SoftwareCompositor' not in builder
report=dict(schema_version=1,source=source,source_after=source,all_commands_terminal=False,
    release_qualification=False,promotion_allowed=False,javascript_executed_by_openui=False,
    cargo_commands_run=0,screenshots_generated=0,native_raster_commands_run=0,
    nonraster_geometry_diagnostic_only=True,mode='debug',binary_sha256=sha(binary),
    build_receipt_sha256=sha(build_path),cli_source_sha256=sha(cli),fixture_source_sha256=sha(fixture),
    probe_sha256=sha(Path(__file__)),runs=[])
prior_path=RAW/'native-table-fragment-debug-v1595/receipt.json';prior=json.loads(prior_path.read_bytes())
assert prior['all_commands_terminal'] and prior['observed_exit_code']==1 and prior['runs']==[]
report['prior_cli_failure']=dict(receipt_sha256=sha(prior_path),log_sha256=sha(prior_path.parent/'legacy-1.log'),reason='The executable uses --backend; --raster-backend belongs to the matrix wrapper. The first attempt stops before fixture layout.',actual_probe_exit=1)
save=lambda:(OUT/'receipt.json').write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');save()
try:
    for repeat in [1,2]:
        rows=[]
        for name,width,height,scale in [('legacy',800,600,1),('mobile',375,667,2),('desktop125',1280,720,1.25),('desktop150',1920,1080,1.5)]:
            command=[str(binary),'debug','wpt/css_break/table_repeated-section_variable-fragmentainer-size-003-crash','--viewport',f'{width}x{height}','--scale',str(scale),'--backend','cpu-skia']
            result=subprocess.run(command,cwd=ROOT,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'),capture_output=True,timeout=60)
            log=OUT/f'{name}-{repeat}.log';log.write_bytes(result.stdout+result.stderr)
            assert result.returncode==0 and b'Unknown test ID' not in result.stderr
            lines=result.stdout.decode().splitlines();fragments=[line for line in lines if 'frag: offset=' in line]
            assert fragments
            row=dict(profile=name,width=width,height=height,scale=scale,command=command,observed_exit_code=result.returncode,log=str(log),log_sha256=sha(log),fragment_count=len(fragments),column_box_count=sum('kind=ColumnBox' in line for line in fragments))
            rows.append(row)
        report['runs'].append(dict(repeat=repeat,rows=rows));save()
    assert all(a['log_sha256']==b['log_sha256'] for a,b in zip(report['runs'][0]['rows'],report['runs'][1]['rows']))
    report.update(independent_debug_runs_identical=True,observations=8,observed_exit_code=0)
except BaseException as error:
    report.update(observed_exit_code=1,failure=str(error));raise
finally:
    report.update(all_commands_terminal=True,source_after=repository_source_identity(ROOT))
    assert report['source_after']==source and sha(binary)==report['binary_sha256'];save()
print(json.dumps({'path':str(OUT/'receipt.json'),'sha256':sha(OUT/'receipt.json'),'profiles':[{'profile':r['profile'],'fragments':r['fragment_count'],'column_boxes':r['column_box_count']} for r in report['runs'][0]['rows']],'independent_debug_runs_identical':True,'screenshots':0}),flush=True)
