"""Preserve the narrow native sizing candidate without claiming runtime qualification."""
import gzip,hashlib,json,subprocess
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-intrinsic-snap-v1';INDEX=ROOT/'docs/renderer/generated/native-intrinsic-snap-v1.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();artifacts=[]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def keep(p,name,expected=None,compress=False):
 b=p.read_bytes();digest=hashlib.sha256(b).hexdigest()
 if expected:assert digest==expected
 assert p.read_bytes()==b
 q=OUT/name;assert not q.exists();q.write_bytes(gzip.compress(b,compresslevel=9,mtime=0) if compress else b)
 a=dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(q.read_bytes())==b;a.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(b))
 artifacts.append(a);return json.loads(b) if p.suffix=='.json' else None
def generated(name,b):
 p=OUT/name;assert not p.exists();p.write_bytes(b);artifacts.append(dict(path=str(p.relative_to(ROOT)),sha256=sha(p),bytes=len(b)))
s=keep(RAW/'native-intrinsic-snap-source-v1692.json','source.json','26ba1cae433ab7046d5ad0cc43baa02f8e755f58f1da3ba6c876aad8a02a81c4')
keep(Path('/tmp/openui-native-intrinsic-snap-source-v1692.py'),'source.py',s['probe_sha256'])
NATIVE=Path(s['root']);assert subprocess.check_output(['git','rev-parse','HEAD'],cwd=NATIVE,text=True).strip()==s['source'];assert not subprocess.check_output(['git','status','--porcelain'],cwd=NATIVE)
generated('public-native-baseline.patch',subprocess.check_output(['git','diff','--binary',s['parent'],s['baseline']],cwd=NATIVE))
generated('native-intrinsic-grid-ceiling.patch',subprocess.check_output(['git','diff','--binary',s['parent'],s['source']],cwd=NATIVE))
checks=keep(RAW/'native-intrinsic-snap-checks-v1693/receipt.json','source-checks.json');assert checks['all_commands_terminal'] and checks['source']==checks['source_after'] and checks['source']['commit']==s['source']
assert len(checks['checks'])==13 and all(r['observed_exit_code']==0 for r in checks['checks'])
keep(Path('/tmp/openui-native-intrinsic-snap-checks-v1693.py'),'source-checks.py',checks['probe_sha256'])
for r in checks['checks']:keep(RAW/'native-intrinsic-snap-checks-v1693'/(r['name']+'.log'),'source-'+r['name']+'.log.gz',r['log_sha256'],compress=True)
for kind in ['header','version']:
 p=Path(s['chromium_source_support'][kind+'_path']);keep(p,'chromium-support-'+p.name+'.gz',s['chromium_source_support'][kind+'_sha256'],compress=True)
failed=keep(RAW/'native-intrinsic-snap-preflight-v1694.json','unexecuted-preflight-failure.json');assert failed['python_syntax_error_before_execution'] and failed['source_root_never_created']
keep(Path('/tmp/openui-native-intrinsic-snap-prepare-v1694.py'),'failed-prepare.py',failed['probe_sha256'])
prepared=keep(RAW/'native-intrinsic-snap-prepared-v1696.json','native-runtime-prepared.json','ec153f5b8ba99d0148f8a72a40e72931be72400447b5942c876445b1447609dd')
assert prepared['native_owner_not_started'] and not (RAW/'native-intrinsic-snap-pipeline-v1697').exists()
keep(Path('/tmp/openui-native-intrinsic-snap-prepare-v1696.py'),'native-runtime-prepare.py',prepared['probe_sha256'])
for p,d in prepared['scripts'].items():keep(Path(p),Path(p).name,d)
glyph=keep(RAW/'native-glyph-raster-prepared-v1689.json','glyph-queue-unlaunched.json','8810d11078dfdc2e42f13ff7d95bed68d5acd7b6d3e3fb822371d2bd68f4f42e')
assert glyph['native_owner_not_started'] and not (RAW/'native-glyph-raster-pipeline-v1690').exists()
keep(Path('/tmp/openui-native-glyph-raster-prepare-v1689.py'),'glyph-prepare.py',glyph['probe_sha256'])
for p,d in glyph['scripts'].items():keep(Path(p),Path(p).name,d)
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,source=s['source'],baseline=s['baseline'],parent=s['parent'],source_read_only_checks_passed=13,
 production_functions_changed=1,production_has_no_font_size_family_fixture_or_test_id_condition=True,
 new_native_rust_callback_guard_scales=5,pinned_chromium_geometry_queries=10,
 old_native_geometry_short_by_css_pixels=1/64,chromium_source_support_not_binary_provenance=True,
 native_baseline_and_fixed_guards_not_executed=True,native_app_and_four_matrices_not_executed=True,
 own_source_hosted_run=37373688613,own_source_hosted_qualification_pending=True,
 selected_next_owner='native-intrinsic-snap-pipeline-v1697',selected_owner_not_started=True,
 glyph_owner_v1690_not_started=True,glyph_queue_requires_fresh_preparation_after_selected_owner=True,
 old_113_exports_and_30_layouts_preserved=True,source_unapplied=True,reference_bytes_unchanged=True,
 accepted_original_exact=21334,accepted_expanded_exact=22137,accepted_renderer_unchanged=True,
 pixel_tolerance=0,javascript_executed_by_openui=False,public_native_rust_apis_required=True,
 release_qualification=False,new_release_states_admitted=0,artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
for a in artifacts:
 assert sha(ROOT/a['path'])==a['sha256']
 if a.get('encoding')=='gzip':assert hashlib.sha256(gzip.decompress((ROOT/a['path']).read_bytes())).hexdigest()==a['decompressed_sha256']
print(json.dumps(dict(index=str(INDEX.relative_to(ROOT)),sha256=sha(INDEX),artifacts=len(artifacts),bytes=report['artifact_bytes'])),flush=True)
