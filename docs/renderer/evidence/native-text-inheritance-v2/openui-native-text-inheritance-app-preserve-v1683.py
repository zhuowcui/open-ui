"""Publish terminal app evidence with genuine cross-language counts."""
import gzip,hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-text-inheritance-v2';INDEX=ROOT/'docs/renderer/generated/native-text-inheritance-v2.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir();artifacts=[]
sha=lambda b:hashlib.sha256(b).hexdigest()
def keep(path,name,compressed=False,expected=None):
 content=path.read_bytes();digest=sha(content)
 if expected is not None:assert digest==expected
 target=OUT/name;assert not target.exists();target.write_bytes(gzip.compress(content,compresslevel=9,mtime=0) if compressed else content)
 assert path.read_bytes()==content
 row=dict(path=str(target.relative_to(ROOT)),sha256=sha(target.read_bytes()),bytes=target.stat().st_size,source_path=str(path))
 if compressed:assert gzip.decompress(target.read_bytes())==content;row.update(encoding='gzip',decompressed_sha256=digest,decompressed_bytes=len(content))
 artifacts.append(row);return json.loads(content) if path.suffix=='.json' else None
p=RAW/'native-text-style-runtime-consumer-v1677/receipt.json';d=keep(p,'native-consumer-terminal.json.gz',True)
assert d['all_commands_terminal'] and d['source']==d['source_after'] and d['source']['commit']=='41b616c3be5224b074e9d06ba6aa963731a1b265'
assert d['observed_exit_code']==1 and d['totals']['images']==600 and d['totals']['images_exact']==0 and d['totals']['geometry_exact']==34560
images=[i for c in d['cases'] for i in c['images']];cross=[i for i in images if i['language']!='rust']
assert len(cross)==400 and all(i['rust_pixels_equal'] and i['rust_geometry_equal'] for i in cross)
assert len(d['cases'])==100 and all(c['native_repeats_identical'] and c['uses_unchanged_prior_chromium_images'] for c in d['cases'])
failures=[]
for c in d['cases']:
 for i in c['images']:
  for v in i['phases']:
   if not v['geometry_exact']:
    assert c['family']=='Ahem' and c['size'] in [12,16]
    assert all(v['native_bounds'][k]==v['chromium_bounds'][k] for k in ['x','y','height'])
    assert v['chromium_bounds']['width']-v['native_bounds']['width']==1/64
    failures.append(v)
assert len(failures)==3840
owner=keep(RAW/'native-text-style-runtime-pipeline-v1678/receipt.json','owner-at-observation.json')
app_step=next(r for r in owner['steps'] if r['name']=='native-application')
keep(RAW/'native-text-style-runtime-pipeline-v1678/native-application.log','native-application.log',expected=app_step['log_sha256'])
keep(Path('/tmp/openui-native-text-style-runtime-consumer-v1677.py'),'consumer-probe.py',expected=d['probe_sha256'])
keep(Path(__file__),Path(__file__).name)
report=dict(schema_version=1,source=d['source'],source_after=d['source_after'],all_native_app_commands_terminal=True,
 native_cases=100,native_images=600,chromium_exact_images=0,native_geometry_states=38400,chromium_exact_geometry_states=34560,
 actual_cross_language_images=400,actual_cross_language_images_matching_rust=400,actual_cross_language_geometry_matches_rust=True,rust_self_rows_excluded=200,
 all_native_repeat_runs_identical=True,all_chromium_reference_bytes_unchanged=True,
 geometry_differences=dict(count=3840,family='Ahem',sizes=[12,16],only_width_differs=True,native_width_short_by_css_pixels=1/64),
 native_inheritance_applied_only_on_private_source=True,default_native_raster_still_not_chromium_exact=True,
 app_actual_exit=1,owner='native-text-style-runtime-pipeline-v1678',owner_terminal_at_observation=owner['all_commands_terminal'],
 focused_primitive_original_expanded_still_required=True,accepted_original_exact=21334,accepted_expanded_exact=22137,
 pixel_tolerance=0,javascript_executed_by_openui=False,public_native_rust_apis_required=True,release_qualification=False,new_release_states_admitted=0,
 artifacts=artifacts,artifact_count=len(artifacts),artifact_bytes=sum(a['bytes'] for a in artifacts))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(dict(index=str(INDEX),sha256=sha(INDEX.read_bytes()),artifacts=len(artifacts),stored_bytes=report['artifact_bytes'],images_exact=0,geometry_exact=34560,actual_c_cpp_matches=400)),flush=True)
