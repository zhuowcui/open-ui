"""Keep all existing image bytes used for rejected width trial forensics."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui')
RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-intrinsic-snap-v8'
INDEX=ROOT/'docs/renderer/generated/native-intrinsic-snap-v8.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
p=RAW/'native-width-region-audit-v1774/receipt.json';d=json.loads(p.read_bytes())
assert sha(p)=='fa7bb3ca94e4c0d83c0898081b3fe344a3b1885888c671b203f075f02c6f53c8'
assert d['all_commands_terminal'] and d['observed_exit_code']==0 and d['changed_rows']==27 and d['affected_tests']==20 and d['exact_losses']==21 and d['exact_gains']==0
assert d['chromium_reference_bytes_unchanged'] and len(d['image_sha256'])==57
artifacts=[]
def keep(p,name,expected,compress=False):
 assert sha(p)==expected;b=p.read_bytes();q=OUT/name;assert not q.exists();q.write_bytes(gzip.compress(b,mtime=0) if compress else b)
 row=dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p))
 if compress:assert gzip.decompress(q.read_bytes())==b;row.update(encoding='gzip',decompressed_sha256=expected,decompressed_bytes=len(b))
 artifacts.append(row)
keep(p,'regions-and-four-profile-sweeps.json.gz',sha(p),True)
keep(Path('/tmp/openui-native-width-region-audit-v1774.py'),'region-audit.py',d['probe_sha256'])
for path,digest in sorted(d['image_sha256'].items()):keep(Path(path),digest+'.png',digest)
keep(Path(__file__),'preserver.py',sha(Path(__file__)))
report=dict(schema_version=1,source=d['source'],prior_source=d['prior_source'],all_commands_terminal=True,all_57_existing_png_and_rgba_hashes_verified=True,
 changed_comparisons=27,affected_original_tests=20,exact_losses=21,exact_gains=0,all_20_tests_have_complete_four_profile_sweeps=True,
 all_changed_rows_have_bounds_connected_regions_and_channel_deltas=True,changes_by_profile=d['changes_by_profile'],
 chromium_reference_bytes_unchanged=True,read_only_offline_audit=True,cargo_commands_run=0,raster_commands_run=0,chromium_capture_commands_run=0,screenshots_generated=0,
 minimized_reproducers_and_reviewed_root_causes_still_required=True,formal_residual_ownership_unchanged=True,candidate_rejected=True,candidate_applied_to_umbrella=False,
 javascript_executed_by_openui=False,public_native_rust_apis_required=True,pixel_tolerance=0,release_qualification=False,new_release_states_admitted=0,
 artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'index':str(INDEX.relative_to(ROOT)),'sha256':sha(INDEX),'artifacts':len(artifacts),'bytes':report['artifact_bytes']}),flush=True)
