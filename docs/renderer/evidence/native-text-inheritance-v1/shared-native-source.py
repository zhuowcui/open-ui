"""Add public native text/font regression and compose shared inheritance."""
import hashlib,json,subprocess
from pathlib import Path
MAIN=Path('/home/nero/code/open-ui');RAW=MAIN/'out/renderer-evidence/native-viewport-scroll-v1'
ROOT=Path('/dev/shm/openui-native-text-inherited-styles-90310e15')
BASE=Path('/dev/shm/openui-native-text-inheritance-baseline-90310e15')
PARENT='90310e15b86bed558a79c8eb085d2a72092eab69'
assert not BASE.exists();subprocess.run(['git','worktree','add','--quiet','-b','agent/native-text-inheritance-baseline-v1668',str(BASE),PARENT],cwd=MAIN,check=True)
relative='bindings/rust/openui/tests/native_text_style_inheritance.rs'
text='''use openui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn native_text_replacement_inherits_authored_fonts_through_rust_callbacks() {
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
        label.set_font_size(LengthValue::px(20.0)).unwrap();
        label.set_line_height(LineHeight::Number(1.0)).unwrap();
        label.set_color(Color::RED).unwrap();
        label.set_text("X").unwrap();
        root.append_child(&label).unwrap();
        let initial_style = label.computed_style().unwrap();
        let before = label.bounding_rect().unwrap().unwrap();
        assert_eq!(
            (before.x, before.y, before.width, before.height),
            (20.0, 20.0, 20.0, 20.0),
            "authored text must inherit its native container font: scale={scale}"
        );
        let count = Rc::new(Cell::new(0));
        let observed = Rc::clone(&count);
        let target = label.downgrade();
        root.on("click", move |_| {
            let label = target.upgrade().unwrap();
            label.set_font_size(LengthValue::px(24.0)).unwrap();
            label.set_color(Color::BLUE).unwrap();
            label.set_text("XX").unwrap();
            observed.set(observed.get() + 1);
        })
        .unwrap();
        root.click().unwrap();
        assert_eq!(count.get(), 1);
        let after = label.bounding_rect().unwrap().unwrap();
        assert_eq!(
            (after.x, after.y, after.width, after.height),
            (20.0, 20.0, 48.0, 24.0),
            "replacement text must inherit callback-updated native fonts: scale={scale}"
        );
        assert_eq!(label.text_content().unwrap(), "XX");
        assert_eq!(initial_style.font_size, 20.0);
        assert_eq!(initial_style.color, Color::RED);
        let weak = label.downgrade();
        drop(label);
        drop(root);
        drop(document);
        assert!(weak.upgrade().is_none());
    }
}
'''
for root in [BASE,ROOT]:
 p=root/relative;assert not p.exists();p.write_text(text)
subprocess.run(['bash','-c','ulimit -s 262144; rustfmt --edition 2021 --config skip_children=true "$1"','rustfmt',str(BASE/relative)],check=True)
(ROOT/relative).write_bytes((BASE/relative).read_bytes())
subprocess.run(['git','add','--',relative],cwd=BASE,check=True)
subprocess.run(['git','commit','--quiet','-m','Guard native text font inheritance through public Rust callbacks'],cwd=BASE,check=True)
baseline=subprocess.check_output(['git','rev-parse','HEAD'],cwd=BASE,text=True).strip()
subprocess.run(['python3','tools/style/generate_properties.py'],cwd=ROOT,check=True)
paths=['bindings/rust/openui-engine/src/lib.rs','bindings/rust/openui-engine/src/animation.rs','bindings/rust/openui-style/src/property.rs','bindings/rust/openui/examples/native_inherited_styles.rs','bindings/rust/openui/examples/native_relative_styles.rs','bindings/rust/openui/examples/native_static_position.rs',relative]
for path in paths:subprocess.run(['bash','-c','ulimit -s 262144; rustfmt --edition 2021 --config skip_children=true "$1"','rustfmt',str(ROOT/path)],check=True)
subprocess.run(['python3','tools/style/generate_properties.py','--check'],cwd=ROOT,check=True)
assert (ROOT/relative).read_bytes()==(BASE/relative).read_bytes()
subprocess.run(['git','add','--',*paths,'bindings/rust/openui-style/src/generated_properties.rs','tools/style/generate_properties.py'],cwd=ROOT,check=True)
subprocess.run(['git','diff','--cached','--check'],cwd=ROOT,check=True)
subprocess.run(['git','commit','--quiet','-m','Combine shared text replacement with native authored style inheritance'],cwd=ROOT,check=True)
fixed=subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip()
assert not subprocess.check_output(['git','status','--porcelain'],cwd=ROOT)
assert not subprocess.check_output(['git','diff',PARENT,fixed,'--','include','bindings/rust/openui-ffi','bindings/rust/openui-text','bindings/rust/openui-paint','bindings/rust/openui-compositor','.github'],cwd=ROOT)
report=dict(schema_version=1,parent=PARENT,existing_native_inheritance_source='0ccc37da4a0def755d5b7f25bb93e92b789787ce',
 baseline=baseline,source=fixed,root=str(ROOT),baseline_root=str(BASE),branch='agent/native-text-inherited-styles-v1668',
 generated_properties_regenerated=True,new_test='native_text_replacement_inherits_authored_fonts_through_rust_callbacks',
 public_rust_callback_scales=5,api_state_and_owned_snapshot_and_teardown_required=True,
 old_c_abi_and_renderer_bodies_unchanged=True,javascript_executed_by_openui=False,release_qualification=False,
 baseline_and_fixed_tests_not_yet_executed=True,probe_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
p=RAW/'native-text-inheritance-source-v1668.json';assert not p.exists();p.write_text(json.dumps(report,sort_keys=True,indent=2)+'\n')
print(json.dumps(report),flush=True)
