"""Retain shaped fractions in native intrinsic sizing, with a measured Rust callback guard."""
import hashlib,json,subprocess,sys
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
PARENT='41b616c3be5224b074e9d06ba6aa963731a1b265'
ROOT=Path('/dev/shm/openui-native-intrinsic-snap-41b616c3');BASE=Path('/dev/shm/openui-native-intrinsic-snap-baseline-41b616c3')
for p,branch in [(ROOT,'agent/native-intrinsic-snap-v1692'),(BASE,'agent/native-intrinsic-snap-baseline-v1692')]:
 assert not p.exists();subprocess.run(['git','worktree','add','--quiet','-b',branch,str(p),PARENT],cwd=MAIN,check=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
consumer=RAW/'native-text-style-runtime-consumer-v1677/receipt.json';data=json.loads(consumer.read_bytes());assert data['all_commands_terminal'] and data['source']==data['source_after'] and data['source']['commit']==PARENT
observations=[]
for size,state,text,expected in [(12,'before','X',12.015625),(16,'after','XX',32.015625)]:
 for scale in [1.0,1.25,1.5,2.0,3.0]:
  case=next(c for c in data['cases'] if c['family']=='Ahem' and c['size']==size and c['scale']==scale)
  image=next(i for i in case['images'] if i['language']=='rust' and i['state']==state);phase=image['phases'][0]
  assert phase['chromium_bounds']==dict(x=20,y=20,width=expected,height=size)
  assert phase['native_bounds']==dict(x=20,y=20,width=expected-1/64,height=size)
  observations.append(dict(size=size,scale=scale,state=state,text=text,expected_bounds=phase['chromium_bounds'],prior_native_bounds=phase['native_bounds'],chromium_png_sha256=image['chromium_png_sha256']))
chromium=Path('/home/nero/chromium/src');header=chromium/'third_party/blink/renderer/platform/fonts/shaping/shape_result.h';version=chromium/'chrome/VERSION'
assert b'LayoutUnit SnappedWidth() const { return LayoutUnit::FromFloatCeil(width_); }' in header.read_bytes()
relative='bindings/rust/openui/tests/native_intrinsic_snap.rs'
test='''use openui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn native_intrinsic_text_width_retains_fraction_through_rust_callback() {
    // These natural widths are measured in the preserved pinned Chromium
    // font matrix. No width is authored, and no browser code runs in Open UI.
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let document = Document::with_font_collection(
            ViewportMetrics::from_logical_size(128.0, 96.0, scale).unwrap(),
            FontCollection::deterministic_test(),
        )
        .unwrap();
        let root = document.body();
        let label = Element::create(&document, "div").unwrap();
        label.set_position(Position::Absolute).unwrap();
        label.set_left(Length::px(20.0)).unwrap();
        label.set_top(Length::px(20.0)).unwrap();
        label.set_font_family(FontFamilyList::single("Ahem")).unwrap();
        label.set_font_size(LengthValue::px(12.0)).unwrap();
        label.set_line_height(LineHeight::Number(1.0)).unwrap();
        label.set_color(Color::BLACK).unwrap();
        label.set_text("X").unwrap();
        root.append_child(&label).unwrap();
        let before = label.bounding_rect().unwrap().unwrap();
        assert_eq!(
            (before.x, before.y, before.width, before.height),
            (20.0, 20.0, 12.015625, 12.0),
            "native intrinsic width must preserve Chromium's shaped fraction: scale={scale}"
        );
        let count = Rc::new(Cell::new(0));
        let observed = Rc::clone(&count);
        let target = label.downgrade();
        root.on("click", move |_| {
            let label = target.upgrade().unwrap();
            label.set_font_size(LengthValue::px(16.0)).unwrap();
            label.set_color(Color::BLUE).unwrap();
            label.set_text("XX").unwrap();
            observed.set(observed.get() + 1);
        })
        .unwrap();
        root.click().unwrap();
        assert_eq!(count.get(), 1);
        assert_eq!(label.text_content().unwrap(), "XX");
        let after = label.bounding_rect().unwrap().unwrap();
        assert_eq!(
            (after.x, after.y, after.width, after.height),
            (20.0, 20.0, 32.015625, 16.0),
            "callback text must retain its new intrinsic fraction: scale={scale}"
        );
        assert_eq!(before.width, 12.015625);
        assert_eq!(before.height, 12.0);
        let weak = label.downgrade();
        drop(label);
        drop(root);
        drop(document);
        assert!(weak.upgrade().is_none());
    }
}
'''
for r in [ROOT,BASE]:
 p=r/relative;assert not p.exists();p.write_text(test)
 subprocess.run(['bash','-c','ulimit -s 262144; rustfmt --edition 2021 --config skip_children=true "$1"','rustfmt',str(p)],check=True)
assert (ROOT/relative).read_bytes()==(BASE/relative).read_bytes()
subprocess.run(['git','add','--',relative],cwd=BASE,check=True);subprocess.run(['git','commit','--quiet','-m','Guard native intrinsic text fractions through Rust callbacks'],cwd=BASE,check=True)
baseline=subprocess.check_output(['git','rev-parse','HEAD'],cwd=BASE,text=True).strip()
path=ROOT/'bindings/rust/openui-layout/src/intrinsic_sizing.rs';s=path.read_text();start=s.index('/// Convert a shaped advance to Blink');end=s.index('\nfn intrinsic_close_rounding_excess',start)
s=s[:start]+'''/// Convert a shaped advance to Blink's 1/64px intrinsic-size grid.
///
/// ShapeResult::SnappedWidth ceil-converts the original shaped width. Even
/// a small positive fraction affects native bounds and must be preserved.
fn intrinsic_text_width(width: f32) -> LayoutUnit {
    LayoutUnit::from_f32_ceil(width)
}
'''+s[end:]
s=s.replace('fn intrinsic_text_width_normalizes_backend_roundoff_at_layout_unit_boundaries()', 'fn intrinsic_text_width_preserves_fraction_before_layout_grid_ceiling()')
before='''            intrinsic_text_width(208.000_396_729),
            LayoutUnit::from_i32(208)''';after='''            intrinsic_text_width(208.000_396_729),
            LayoutUnit::from_raw(208 * 64 + 1)'''
assert s.count(before)==1;s=s.replace(before,after)
path.write_text(s)
subprocess.run(['bash','-c','ulimit -s 262144; rustfmt --edition 2021 --config skip_children=true "$1"','rustfmt',str(path)],check=True)
subprocess.run(['git','add','--',relative,'bindings/rust/openui-layout/src/intrinsic_sizing.rs'],cwd=ROOT,check=True);subprocess.run(['git','diff','--cached','--check'],cwd=ROOT,check=True)
subprocess.run(['git','commit','--quiet','-m','Preserve shaped width fractions in native intrinsic sizing'],cwd=ROOT,check=True)
fixed=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
for r in [ROOT,BASE]:assert not subprocess.check_output(['git','status','--porcelain'],cwd=r)
paths=subprocess.check_output(['git','diff','--name-only',PARENT,fixed],cwd=ROOT,text=True).splitlines();assert paths==['bindings/rust/openui-layout/src/intrinsic_sizing.rs',relative]
report=dict(schema_version=1,parent=PARENT,source=fixed,baseline=baseline,root=str(ROOT),baseline_root=str(BASE),branch='agent/native-intrinsic-snap-v1692',
 consumer_receipt_sha256=sha(consumer),chromium_binary=data['chromium'],chromium_source_support=dict(header_path=str(header),header_sha256=sha(header),version_path=str(version),version_sha256=sha(version),source_patch_is_not_binary_provenance=True),
 native_callback_scales=5,measured_chromium_queries=observations,production_has_no_font_size_family_fixture_or_test_id_condition=True,
 production_function_changes=1,old_113_exports_and_30_layouts_preserved=True,reference_bytes_unchanged=True,public_native_rust_apis=True,javascript_executed_by_openui=False,
 baseline_and_fixed_tests_not_yet_executed=True,source_unapplied=True,release_qualification=False,new_release_states_admitted=0,probe_sha256=sha(Path(__file__)))
p=RAW/'native-intrinsic-snap-source-v1692.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n');print(json.dumps(dict(source=fixed,baseline=baseline,production_functions_changed=1,executed=False,receipt_sha256=sha(p))),flush=True)
