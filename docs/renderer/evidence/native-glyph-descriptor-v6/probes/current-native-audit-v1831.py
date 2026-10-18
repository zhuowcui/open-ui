"""Compare both complete native matrices without treating internal consistency as Chromium parity."""
import collections,hashlib,json
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1';OUT=RAW/'native-glyph-current-native-audit-v1831.json';assert not OUT.exists()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();paths={l:RAW/('native-glyph-current-'+l+'-consumer-v1828/receipt.json') for l in ['baseline','candidate']};data={l:json.loads(p.read_bytes()) for l,p in paths.items()}
for d in data.values():assert d['all_commands_terminal'] and not d.get('failure') and d['totals']['images']==400 and d['source']==d['source_after'] and d['source']['clean']
flatten=lambda d:{(r['raster'],r['family'],r['size'],r['scale'],i['state']):i for r in d['cases'] for i in r['images']}
a,b=map(flatten,[data['baseline'],data['candidate']]);assert a.keys()==b.keys() and len(a)==400
report=dict(schema_version=1,source=data['candidate']['source'],baseline_source=data['baseline']['source'],all_commands_terminal=True,release_qualification=False,pixel_tolerance=0,javascript_executed_by_openui=False,probe_sha256=sha(Path(__file__)),consumer_receipt_sha256={l:sha(p) for l,p in paths.items()},totals={l:d['totals'] for l,d in data.items()},groups={},changes=[],all_400_chromium_png_inputs_unchanged=True,formal_wpt_ownership_unchanged=True,new_release_states_admitted=0)
for raster in ['default','chromium-lcd']:
 keys=[k for k in b if k[0]==raster];g=dict(images=len(keys),prior_exact=0,exact=0,exact_gains=0,exact_losses=0,native_png_changes=0,prior_mismatched_pixels=0,mismatched_pixels=0,prior_geometry_exact=0,geometry_exact=0,phase_gains=0,phase_losses=0)
 for k in keys:
  old,row=a[k],b[k];assert row['chromium_png_sha256']==old['chromium_png_sha256'] and row['independent_reference_runs']==old['independent_reference_runs']
  oldexact=old['analysis']['mismatched_pixels']==0;exact=row['analysis']['mismatched_pixels']==0;g['prior_exact']+=oldexact;g['exact']+=exact;g['exact_gains']+=exact and not oldexact;g['exact_losses']+=oldexact and not exact;g['native_png_changes']+=old['native_png_sha256']!=row['native_png_sha256'];g['prior_mismatched_pixels']+=old['analysis']['mismatched_pixels'];g['mismatched_pixels']+=row['analysis']['mismatched_pixels']
  for p,q in zip(old['phases'],row['phases']):
   assert p['logical_phase_64ths']==q['logical_phase_64ths'] and p['chromium_bounds']==q['chromium_bounds'];g['prior_geometry_exact']+=p['geometry_exact'];g['geometry_exact']+=q['geometry_exact'];g['phase_gains']+=p['mismatched_pixels']!=0 and q['mismatched_pixels']==0;g['phase_losses']+=p['mismatched_pixels']==0 and q['mismatched_pixels']!=0
  if old['native_png_sha256']!=row['native_png_sha256'] or old['analysis']!=row['analysis']:
   report['changes'].append(dict(raster=k[0],family=k[1],size=k[2],scale=k[3],state=k[4],prior_analysis=old['analysis'],analysis=row['analysis'],prior_native_png_sha256=old['native_png_sha256'],native_png_sha256=row['native_png_sha256'],chromium_png_sha256=row['chromium_png_sha256'],phase_behavior=row['phases'],root_cause_reviewed=False))
 report['groups'][raster]=g
report['baseline_default_rows_identical_to_accepted_native_app']=sum(r['same_png'] and r['same_rgba'] for r in data['baseline']['default_prior_image_checks'])
assert report['baseline_default_rows_identical_to_accepted_native_app']==200
OUT.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'groups':report['groups'],'reference_inputs_unchanged':True,'baseline_default_rows_identical_to_accepted_native_app':200,'sha256':sha(OUT)}))
