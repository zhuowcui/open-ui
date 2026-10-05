use openui::prelude::*;
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
        label
            .set_font_family(FontFamilyList::single("Ahem"))
            .unwrap();
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
