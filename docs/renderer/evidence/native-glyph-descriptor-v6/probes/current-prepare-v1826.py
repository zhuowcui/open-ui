"""Resolve only obsolete guard imports and add a common public Rust consumer."""
from pathlib import Path
import subprocess,json,hashlib
root=Path('/home/nero/code/open-ui');base=Path('/dev/shm/openui-native-glyph-baseline-16187f4f-v1825');fixed=Path('/dev/shm/openui-native-glyph-candidate-16187f4f-v1825')
raw=root/'out/renderer-evidence/native-viewport-scroll-v1';receipt=raw/'native-glyph-current-source-v1826.json';assert not receipt.exists()
conflict='''<<<<<<< HEAD
    use super::{chromium_lcd_raster_x, chromium_lcd_strike_y_offset};
=======
>>>>>>> 2f53d5de (fix(text): preserve physical strike descriptors and configured glyph coverage)
'''
p=fixed/'bindings/rust/openui-text/src/shaping/shape_result.rs';before=p.read_bytes();s=before.decode();assert s.count(conflict)==1
conflictpath=raw/'native-glyph-current-preparation-conflict-v1826.txt';assert not conflictpath.exists();conflictpath.write_bytes(subprocess.check_output(['git','diff','--cc'],cwd=fixed))
p.write_text(s.replace(conflict,''));subprocess.run(['git','add',str(p)],cwd=fixed,check=True);subprocess.run(['git','-c','core.editor=true','cherry-pick','--continue'],cwd=fixed,stdout=subprocess.DEVNULL,check=True)
for path in ['bindings/rust/openui-text/src/shaping/shape_result.rs','bindings/rust/openui-paint/src/text_painter.rs']:
 old=subprocess.check_output(['git','show','fce42e086dee48f6e2d725b2b4f3bb8815e79e5c:'+path],cwd=root);assert (fixed/path).read_bytes()==old
source=(root/'bindings/rust/openui/examples/native_text_content.rs').read_text()
a='''    if args.next().is_some()
''';b='''    let raster = match args.next().as_deref() {
        None | Some("default") => RasterConfiguration::default(),
        Some("chromium-lcd") => RasterConfiguration::chromium_linux_lcd(),
        _ => return Err("raster must be default or chromium-lcd".into()),
    };
    if args.next().is_some()
''';assert source.count(a)==1;source=source.replace(a,b)
a='''    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        FontCollection::deterministic_test(),
    )?;''';b='''    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        FontCollection::deterministic_test(),
        EngineOptions { raster_configuration: raster },
    )?;
    assert_eq!(document.raster_configuration()?, raster);''';assert source.count(a)==1;source=source.replace(a,b)
a='''    root.click()?;
''';b='''    root.click()?;
    assert_eq!(document.raster_configuration()?, raster);
''';assert source.count(a)==1;source=source.replace(a,b)
source=source.replace('//! Shared native text replacement, Rust callbacks and owned geometry.','//! Native glyph coverage through explicit immutable options and Rust callbacks.')
report=dict(schema_version=1,preparation_conflict_resolved=True,conflict_limited_to_removed_private_guard_imports=True,preparation_first_actual_exit=1,conflict_bytes_sha256=hashlib.sha256(before).hexdigest(),conflict_diff_sha256=hashlib.sha256(conflictpath.read_bytes()).hexdigest(),sources=[],probe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),release_qualification=False,javascript_executed_by_openui=False,pixel_tolerance=0)
for work in [base,fixed]:
 target=work/'bindings/rust/openui/examples/native_glyph_coverage.rs';assert not target.exists();target.write_text(source)
 subprocess.run(['bash','-c','ulimit -s 262144; rustfmt --edition 2021 --config skip_children=true bindings/rust/openui/examples/native_glyph_coverage.rs bindings/rust/openui-text/src/shaping/shape_result.rs'],cwd=work,check=True)
 subprocess.run(['git','add','bindings/rust/openui/examples/native_glyph_coverage.rs','bindings/rust/openui-text/src/shaping/shape_result.rs'],cwd=work,check=True)
 subprocess.run(['git','commit','-q','-m','Exercise native glyph coverage through public Rust raster options'],cwd=work,check=True)
 assert subprocess.check_output(['git','status','--porcelain'],cwd=work)==b''
 report['sources'].append(dict(root=str(work),commit=subprocess.check_output(['git','rev-parse','HEAD'],cwd=work,text=True).strip()))
assert (base/'bindings/rust/openui/examples/native_glyph_coverage.rs').read_bytes()==(fixed/'bindings/rust/openui/examples/native_glyph_coverage.rs').read_bytes()
receipt.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(report['sources']))
