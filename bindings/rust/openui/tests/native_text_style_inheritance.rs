use openui::prelude::*;
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
        label
            .set_font_family(FontFamilyList::single("Ahem"))
            .unwrap();
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
