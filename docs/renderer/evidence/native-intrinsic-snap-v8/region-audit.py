"""Hash-verified offline analysis of every width trial regression; no render."""
import collections,hashlib,json,sys
from pathlib import Path
from PIL import Image
MAIN=Path('/home/nero/code/open-ui')
RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
CACHE=Path('/mnt/e/openui-v02-qualification-d174ea0b/renderer-cache')
OUT=RAW/'native-width-region-audit-v1774'
STORE=Path('/mnt/e/openui-v02-qualification-d174ea0b')/OUT.name
assert not OUT.exists() and not STORE.exists()
STORE.mkdir();OUT.symlink_to(STORE,target_is_directory=True)
sys.path.insert(0,str(MAIN/'tools/qualification'))
from residuals import analyze_image_difference,canonical_sha256
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
audit=RAW/'native-intrinsic-snap-matrix-audit-v1744.json'
assert sha(audit)=='ecd74304b5526912321fa95d701e4a244b3a2bde10bb2e98de5fce1dd5dbd9e3'
d=json.loads(audit.read_bytes()); changes=d['changed_original_comparisons'];assert len(changes)==27
old_path=RAW/'native-event-targets-clean-full-v1456/full-summary.json'
new_path=RAW/'native-intrinsic-snap-clean-full-v1716/full-summary.json'
old,new=json.loads(old_path.read_bytes()),json.loads(new_path.read_bytes())
assert sha(old_path)==d['suites']['full']['prior_summary_sha256'] and sha(new_path)==d['suites']['full']['summary_sha256']
flatten=lambda s:{(p['profile'],r['id']):r for p in s['profiles'] for r in p['tests']}
a,b=flatten(old),flatten(new)
assert len(a)==len(b)==22924
paths={};rows=[]
def verified_image(row,language):
 p=CACHE/row[language+'_png_cache_path'];assert sha(p)==row[language+'_png_sha256']
 with Image.open(p) as im:assert hashlib.sha256(im.convert('RGBA').tobytes()).hexdigest()==row[language+'_rgba_sha256']
 paths[str(p)]=sha(p);return p
for change in changes:
 key=(change['profile'],change['id']);prior,current=a[key],b[key]
 expected=verified_image(current,'chromium');before=verified_image(prior,'openui');after=verified_image(current,'openui')
 assert current['chromium_rgba_sha256']==prior['chromium_rgba_sha256']
 old_diff=analyze_image_difference(expected,before)
 new_diff=analyze_image_difference(expected,after)
 native_diff=analyze_image_difference(before,after)
 assert old_diff['mismatched_pixels']==change['prior_mismatched_pixels'] and new_diff['mismatched_pixels']==change['mismatched_pixels']
 assert new_diff['connected_region_count']==len(new_diff['connected_regions']) and native_diff['mismatched_pixels']>0
 rows.append(dict(profile=key[0],id=key[1],prior_vs_chromium=old_diff,trial_vs_chromium=new_diff,prior_vs_trial=native_diff,exact_loss=change['exact_loss'],exact_gain=change['exact_gain'],reviewed_root_cause=False,formal_owner_unchanged=True))
 counts=collections.Counter(r['profile'] for r in rows)
for p,h in paths.items():assert sha(Path(p))==h
ids=sorted({r['id'] for r in rows})
sweeps=[]
for test_id in ids:
 records=[dict(profile=k[0],prior_status=a[k]['status'],trial_status=b[k]['status'],prior_mismatched_pixels=a[k]['mismatched_pixels'],trial_mismatched_pixels=b[k]['mismatched_pixels'],native_pixels_changed=a[k]['openui_rgba_sha256']!=b[k]['openui_rgba_sha256'],chromium_fixed=a[k]['chromium_rgba_sha256']==b[k]['chromium_rgba_sha256']) for k in b if k[1]==test_id]
 assert len(records)==4 and all(r['chromium_fixed'] for r in records)
 sweeps.append(dict(id=test_id,profiles=records,reviewed_root_cause=False))
report=dict(schema_version=1,source=new['source'],prior_source=old['source'],all_commands_terminal=True,observed_exit_code=0,input_sha256={str(p):sha(p) for p in [audit,old_path,new_path]},image_sha256=paths,changed_rows=27,affected_tests=20,exact_losses=sum(r['exact_loss'] for r in rows),exact_gains=sum(r['exact_gain'] for r in rows),changes_by_profile=dict(counts),chromium_reference_bytes_unchanged=True,complete_four_profile_sweeps=20,rows=rows,sweeps=sweeps,cargo_commands_run=0,raster_commands_run=0,chromium_capture_commands_run=0,screenshots_generated=0,javascript_executed_by_openui=False,formal_residual_ownership_unchanged=True,minimized_reproducers_not_yet_qualified=True,reviewed_root_causes_not_inferred_from_images=True,release_qualification=False,promotion_allowed=False,new_release_states_admitted=0,pixel_tolerance=0,probe_sha256=sha(Path(__file__)))
p=OUT/'receipt.json';p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps({'receipt':str(p),'sha256':sha(p),'changed_rows':27,'affected_tests':20,'exact_losses':report['exact_losses'],'exact_gains':report['exact_gains'],'changes_by_profile':dict(counts),'images_hash_verified':len(paths),'reviewed_root_cause':False}),flush=True)
