"""Prepare source-identical glyph guards and a complete native Rust raster queue."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-glyph-raster-3b2e0d1f');BRANCH='agent/native-glyph-raster-v1689';FIXED='3b2e0d1f60b90813859c4c24325c7b0061dea29c'
assert not ROOT.exists();subprocess.run(['git','worktree','add','--quiet','-b',BRANCH,str(ROOT),FIXED],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
old_owner=Path('/tmp/openui-native-text-style-runtime-pipeline-v1678.py');text=old_owner.read_text();config=ast.literal_eval(ast.parse(text).body[0].value)
priors=config['prior_pipelines']+[config['name']];assert len(priors)==len(set(priors))==40
sys.path.insert(0,str(ROOT/'tools/qualification'));from renderer_source_identity import repository_source_identity
source=repository_source_identity(ROOT);assert source['clean'] and source['commit']==FIXED
p=RAW/'native-glyph-guard-v1666/receipt.json';guard=json.loads(p.read_bytes())
assert guard['all_commands_terminal'] and guard['source']==guard['source_after']==source and guard['baseline_regression_reproduced']
assert [r['observed_exit_code'] for r in guard['steps']]==[0,101,0,0,0]
for r in guard['steps']:assert sha(p.parent/(r['name']+'.log'))==r['log_sha256']
assert guard['steps'][-1]['test_counts']==[[342,0,0]]
shaping=(ROOT/'bindings/rust/openui-text/src/shaping/shape_result.rs').read_text()
for name in ['fontations_lcd_hints_at_physical_size_before_replay','authored_lcd_origin_retains_shaped_advance_precision']:assert 'fn '+name+'(' in shaping
files={};originals={str(old_owner):sha(old_owner)}
reuse=Path('/tmp/openui-native-glyph-raster-guard-reuse-v1689.py');assert not reuse.exists()
old=Path('/tmp/openui-native-text-style-runtime-guard-reuse-v1677.py');s=old.read_text();originals[str(old)]=sha(old)
s=s.replace(config['root'],str(ROOT)).replace('native-text-style-qualification-guards-v1675','native-glyph-guard-v1666')
s=s.replace(json.loads((RAW/'native-text-style-runtime-prepared-v1677.json').read_bytes())['guard_receipt_sha256'],sha(p))
files[reuse]=s
old=Path('/tmp/openui-native-text-style-runtime-build-v1677.py');s=old.read_text();originals[str(old)]=sha(old)
s=s.replace(config['root'],str(ROOT)).replace('native-text-style-runtime-clean-v1677','native-glyph-raster-clean-v1689')
s=s.replace('41b616c3be5224b074e9d06ba6aa963731a1b265',FIXED)
s=s.replace('for prior_name in '+repr(config['prior_pipelines'])+':','for prior_name in '+repr(priors)+':')
start=s.index('steps=[');end=s.index("assert source['commit']",start)
s=s[:start]+'''steps=[
 ('clean-workspace',base+['clean']+[arg for package in packages for arg in ['-p',package]],None),
 ('workspace',base+['test','--workspace','--locked','--features','openui-ffi/linux'],None),
 ('font-build',base+['build','--locked','-p','openui','--example','native_font_raster'],'debug/examples/native_font_raster'),
 ('pixel-build',base+['build','--locked','-p','pixel-compare','--bin','pixel_compare'],'debug/pixel_compare'),
]
assert len(steps)==4
'''+s[end:]
start=s.index("  if name=='workspace' and p.returncode==0:");end=s.index("  if p.returncode==0 and artifact:",start)
s=s[:start]+'''  if name=='workspace' and p.returncode==0:
   content=log.read_bytes()
   expected_names=['fontations_lcd_hints_at_physical_size_before_replay','authored_lcd_origin_retains_shaped_advance_precision']
   for test_name in expected_names:
    assert ('test shaping::shape_result::tests::'+test_name+' ... ok').encode() in content, 'source-declared text guard did not execute'
   entry['source_declared_text_guards_executed']=expected_names
'''+s[end:]
assert 'private text test absent' not in s and 'text_content-' not in s
build=Path('/tmp/openui-native-glyph-raster-build-v1689.py');assert not build.exists();files[build]=s
old=Path('/tmp/openui-native-raster-fields-retry-consumer-v1559.py');s=old.read_text();originals[str(old)]=sha(old)
s=s.replace('/dev/shm/openui-native-raster-fields-retry-e0dc491e',str(ROOT)).replace('native-raster-fields-retry-consumer-v1559','native-glyph-raster-consumer-v1689').replace('native-raster-fields-retry-clean-v1559','native-glyph-raster-clean-v1689')
s=s.replace('e0dc491e61e17ce4407ff2dd30289e741690572b',FIXED).replace("len(build['steps']) == 17","len(build['steps']) == 4")
start=s.index('binaries = {language:');end=s.index('binary_hashes =',start)
s=s[:start]+"binaries = {'rust': build_path.parent / 'native_font_raster'}\n"+s[end:]
s=s.replace("library = build_path.parent / 'libopenui_ffi.so'\nlibrary_sha = sha(library)\n",'')
s=s.replace("policies = dict(freetype='Freetype', fontations='Fontations')","policies = dict(fontations='Fontations')")
s=s.replace("prior_path = RAW / 'native-font-retry-consumer-v1311/receipt.json'","prior_path = RAW / 'native-raster-fields-retry-consumer-v1559/receipt.json'")
s=s.replace("    ffi_library_sha256=library_sha,\n",'').replace("    assert sha(library) == library_sha\n",'')
s=s.replace("dict(cases=200, images=1200, phase_states=76800,\n        images_exact=1200, phases_exact=76800, geometry_exact=76800, rust_pixels_equal=1200, rust_geometry_equal=1200)","dict(cases=100, images=200, phase_states=12800,\n        images_exact=200, phases_exact=12800, geometry_exact=12800, rust_pixels_equal=200, rust_geometry_equal=200)")
s=s.replace('all_commands_terminal=False, cases=[]','rust_self_comparisons_are_not_cross_language_parity=True, default_native_raster_not_qualified_by_explicit_preset=True, all_commands_terminal=False, cases=[]')
assert 'ffi_library_sha256' not in s and 'sha(library)' not in s and 'images=1200' not in s
s=s.replace("                    row['uses_unchanged_prior_chromium_images'] = reused","                    assert reused, 'immutable Chromium references must be reused'\n                    row['uses_unchanged_prior_chromium_images'] = reused")
consumer=Path('/tmp/openui-native-glyph-raster-consumer-v1689.py');assert not consumer.exists();files[consumer]=s
old=Path('/tmp/openui-native-text-style-runtime-matrices-v1677.py');s=old.read_text();originals[str(old)]=sha(old)
s=s.replace(config['root'],str(ROOT)).replace('native-text-style-runtime-','native-glyph-raster-').replace('v1677','v1689')
matrices=Path('/tmp/openui-native-glyph-raster-matrices-v1689.py');assert not matrices.exists();files[matrices]=s
config.update(root=str(ROOT),commit=FIXED,name='native-glyph-raster-pipeline-v1690',initial_state='awaiting-all-40-prior-whole-pipelines',prior_pipelines=priors,
 scripts=[p.name for p in files],selections=['native-glyph-guard-v1666/receipt.json','owner-interruption-witness-v1664.json','native-raster-fields-retry-consumer-v1559/receipt.json'],
 stages=[('guards',['/usr/bin/python3',str(reuse)],'native-glyph-guard-v1666/receipt.json',True),
 ('native-build',['/usr/bin/python3',str(build)],'native-glyph-raster-clean-v1689/build.json',True),
 ('native-application',['/usr/bin/python3',str(consumer)],'native-glyph-raster-consumer-v1689/receipt.json',True)]+[(suite,['/usr/bin/python3',str(matrices),suite],f'native-glyph-raster-clean-{suite}-v1689/{suite}-summary.json',False) for suite in ['focused','primitive','full','expanded']])
body=text.split('\n',1)[1].replace("baseline_commit='25322be849f1f0523a6e66291a567d0758931160'","baseline_commit='347d901c8de7b5a7890031ac28e58b14cd061275'")
body=body.replace("root_cause_owner='shared Engine native text replacement and C facade'","root_cause_owner='shared authored glyph origins before Skia LCD phase selection'")
body=body.replace('public_rust_c_cpp_callbacks_and_owned_bounds=True','public_rust_callbacks_and_owned_bounds=True, explicit_fontations_only=True, c_cpp_not_measured=True, parent_has_83_original_exact_losses=True')
owner=Path('/tmp/openui-native-glyph-raster-pipeline-v1690.py');assert not owner.exists();files[owner]='CONFIG = '+repr(config)+'\n'+body
for p,s in files.items():
 ast.parse(s);p.write_text(s)
 assert str(ROOT) in s
 assert '/dev/shm/openui-native-text-style-runtime-41b616c3' not in s
report=dict(schema_version=1,source=source,root=str(ROOT),branch=BRANCH,prior_whole_owners=40,
 actual_source_guards_reused=True,guard_receipt_sha256=sha(RAW/'native-glyph-guard-v1666/receipt.json'),all_guard_logs_rehashed=True,
 required_build_stages=4,required_native_images=200,required_geometry_states=12800,explicit_fontations_only=True,c_cpp_not_measured=True,
 default_native_raster_still_requires_qualification=True,rust_self_rows_not_parity_passes=True,required_matrices=4,parent_has_83_original_exact_losses=True,
 native_owner_not_started=True,source_unchanged=True,scripts={str(p):sha(p) for p in files},originals=originals,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,release_qualification=False,probe_sha256=sha(Path(__file__)))
p=RAW/'native-glyph-raster-prepared-v1689.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(prepared=True,source=FIXED,owner_not_started=True,scripts=len(files),receipt_sha256=sha(p))),flush=True)
