"""Preserve exact pinned Chromium sources without inferring residual ownership."""
import hashlib,json
from pathlib import Path
ROOT=Path('/home/nero/code/open-ui');RAW=ROOT/'out/renderer-evidence/native-viewport-scroll-v1'
OUT=ROOT/'docs/renderer/evidence/native-intrinsic-snap-v9';INDEX=ROOT/'docs/renderer/generated/native-intrinsic-snap-v9.json'
assert not OUT.exists() and not INDEX.exists();OUT.mkdir()
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest();rows=[]
def keep(p,name,expected=None):
 if expected:assert sha(p)==expected
 q=OUT/name;assert not q.exists();q.write_bytes(p.read_bytes());assert sha(q)==sha(p)
 rows.append(dict(path=str(q.relative_to(ROOT)),sha256=sha(q),bytes=q.stat().st_size,source_path=str(p)))
for name,stem,digest in [('native-width-pinned-source-v1782','source','cede49f6b07f5c20e2c44e7f812c67eaa64255316452126f9f36e7a2a005adac'),('native-width-pinned-metrics-v1783','metrics','7b2fe66b42e9135ce59a0cdb7ad695cbb7e67105997738e772f6aa3b6d664a4b')]:
 p=RAW/name/'receipt.json';assert sha(p)==digest;d=json.loads(p.read_bytes())
 assert d['all_commands_terminal'] and d['all_requested_files_fetched'] and d['chromium_tag']=='147.0.7727.50'
 keep(p,stem+'-receipt.json',digest)
 for f in d['files']:keep(Path(f['path']),Path(f['path']).name,f['sha256'])
 probe=Path('/tmp/openui-native-width-pinned-'+('source-v1782' if stem=='source' else 'metrics-v1783')+'.py');keep(probe,stem+'-fetcher.py',d['probe_sha256'])
p=Path('/home/nero/.cargo/git/checkouts/rust-skia-b4d79b6a888cdb4c/a31b86b/skia-bindings/skia/modules/skshaper/src/SkShaper_harfbuzz.cpp')
keep(p,'pinned-SkShaper_harfbuzz.cpp')
assert 'LayoutUnit SnappedWidth() const { return LayoutUnit::FromFloatCeil(width_); }' in (OUT/'shape_result.h').read_text()
assert 'return ClampTo<int>(value * kHbPosition1);' in (OUT/'skia_text_metrics.cc').read_text()
assert 'return SkScalarRoundToInt(value * kHbPosition1);' in (OUT/'pinned-SkShaper_harfbuzz.cpp').read_text()
keep(Path(__file__),'preserver.py')
report=dict(schema_version=1,chromium_binary_and_source_tag='147.0.7727.50',all_source_fetches_terminal=True,chromium_snapped_shape_width_uses_float_ceiling=True,chromium_glyph_advance_conversion_clamps_16_16_to_integer=True,pinned_skshaper_glyph_advance_conversion_rounds_16_16_to_integer=True,shared_intrinsic_fallback_uses_unshaped_skfont_measurement=True,source_facts_only=True,does_not_prove_runtime_residual_root_causes=True,formal_residual_ownership_unchanged=True,cargo_commands_run=0,raster_commands_run=0,screenshots_generated=0,javascript_executed_by_openui=False,release_qualification=False,pixel_tolerance=0,new_release_states_admitted=0,artifacts=rows,artifact_count=len(rows),artifact_bytes=sum(r['bytes'] for r in rows))
INDEX.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps({'index':str(INDEX.relative_to(ROOT)),'sha256':sha(INDEX),'artifacts':len(rows)}),flush=True)
